//! Botania for Horizon — slice 1: the mana loop as one `impl Hooks`.
//!
//! Ported from Botania 1.20.1-456 (VazkiiMods/Botania, the Botania License); see
//! `FINDINGS.md` for the behaviour read and `notes.md` for the hook map and gaps.
//!
//! The three blocks are **block entities**, so each one is a [`BlockEntityPart`]
//! attached to its own declared type, not a global hook: the endoflame burns a
//! dropped fuel item into its 300-mana buffer and hands it to the nearest mana
//! spreader within six blocks; the spreader fires a 160-mana burst along the axis
//! it was placed facing and lands it on the first pool in its path; the pool only
//! receives. The spreader's aim is written on placement by the spreader item's
//! [`ItemPart`]. The `/botania` commands and the pool's click readout are the
//! port's harness surface (upstream reads a pool with a wand).
//!
//! The numbers, and every rule that decides mana, live in [`runtime`], which has
//! no `mod_api` type and is unit-tested by `cargo test` alone.
pub mod runtime;

use std::sync::OnceLock;

use mod_api::{
    self, BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockKey, BlockPart,
    Command, CommandArgumentValue, Compound, Flow, Hooks, Host, HostSide, ItemEntityRef, ItemKey,
    ItemPart, ItemStack, LevelRef, OnBlockPlacedByEvent, RegisterCommandsEvent, Registrar,
    RegistrationContext, RemovalReason, RightClickBlockEvent, RunModCommandEvent, ServerHost, Tag,
    Text,
};
use runtime::{
    BurstTarget, Endoflame, FuelSource, Pool, SPREADER_CAPACITY, Spreader, aim_away_from,
    aim_direction, rotation_for_aim, within_link_range,
};

/// The three blocks and their items (`BotaniaFlowerBlocks.ENDOFLAME`,
/// `BotaniaBlocks.MANA_SPREADER`, `BotaniaBlocks.MANA_POOL`).
const ENDOFLAME: &str = "botania:endoflame";
const MANA_SPREADER: &str = "botania:mana_spreader";
const MANA_POOL: &str = "botania:mana_pool";

/// The block entity types this mod declares, one per block, in its own namespace.
const ENDOFLAME_TYPE: &str = "endoflame";
const SPREADER_TYPE: &str = "mana_spreader";
const POOL_TYPE: &str = "mana_pool";

/// Upstream's saved keys (`saveAdditional` / `writeToPacketNBT`).
const KEY_MANA: &str = "mana";
const KEY_BURN_TIME: &str = "burnTime";
const KEY_AGE: &str = "age";
const KEY_ROTATION_X: &str = "rotationX";
const KEY_ROTATION_Y: &str = "rotationY";
const KEY_CAN_SHOOT: &str = "canShootBurst";
const KEY_TICKS_LEFT: &str = "ticksLeft";
const KEY_PAYLOAD: &str = "payload";
const KEY_ARRIVAL: &str = "arrival";

/// How far a spreader looks for its receiver along its aim (upstream's burst
/// simulates until it hits something; sixteen blocks is past every burst's
/// useful range at 0.2 blocks a tick before its mana is gone).
const RECEIVER_SCAN: i32 = 16;

/// The registration keys, so a hot tick compares block identity instead of
/// allocating a string per block read. Registration runs before any hook.
static ENDOFLAME_KEY: OnceLock<BlockKey> = OnceLock::new();
static SPREADER_KEY: OnceLock<BlockKey> = OnceLock::new();
static POOL_KEY: OnceLock<BlockKey> = OnceLock::new();
static SPREADER_ITEM: OnceLock<ItemKey> = OnceLock::new();

// ---------------------------------------------------------------------------
// The block entities (Parts, one per type)
// ---------------------------------------------------------------------------

/// `EndoflameBlockEntity.tickFlower` plus `emptyManaIntoCollector`.
struct EndoflamePart;

#[mod_api::horizon_part]
impl BlockPart for EndoflamePart {}

#[mod_api::horizon_part]
impl BlockEntityPart for EndoflamePart {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = level_in_place(host) else {
            *event.reply = mod_api::BlockEntityTickReply::Unchanged;
            return;
        };
        tick_endoflame(host, level, event.pos, &mut event.entity.nbt);
    }
}

/// `ManaSpreaderBlockEntity.commonTick` + `tryShootBurst`: land the burst in
/// flight on the receiver it was fired at, or fire a new one.
struct SpreaderPart;

#[mod_api::horizon_part]
impl BlockPart for SpreaderPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for SpreaderPart {
    fn block_entity_ticks(&self, _: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = level_in_place(host) else {
            *event.reply = mod_api::BlockEntityTickReply::Unchanged;
            return;
        };
        tick_spreader(host, level, event.pos, &mut event.entity.nbt);
    }
}

// The pool has no server tick: it only receives bursts and answers a click.
// It is still a block entity, so its type is declared and it saves its mana.

// ---------------------------------------------------------------------------
// The item part: a placed spreader keeps the aim the placer looked along
// ---------------------------------------------------------------------------

/// `ManaSpreaderBlock.setPlacedBy`: the spreader fires the way its placer looked,
/// saved as upstream's `rotationX` / `rotationY` degree pair.
struct SpreaderPlacementPart;

#[mod_api::horizon_part]
impl ItemPart for SpreaderPlacementPart {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        if event.state.block().id(host) != MANA_SPREADER {
            return;
        }
        let Some(placer) = event.placer else {
            return;
        };
        let Some(level) = placer.level(host).or_else(|| level_in_place(host)) else {
            return;
        };
        let [yaw, pitch] = placer.entity().rotation(host).unwrap_or([0.0, 0.0]);
        let (rotation_x, rotation_y) = rotation_for_aim(aim_away_from(yaw, pitch));
        let mut data = Compound::new();
        write_float(&mut data, KEY_ROTATION_X, rotation_x);
        write_float(&mut data, KEY_ROTATION_Y, rotation_y);
        merge_data(host, level, event.pos, data);
    }
}

// ---------------------------------------------------------------------------
// The one impl Hooks
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the three block entity types (each held by its own block) and the
    /// spreader item's placement part. The types are what the old `block_entity`
    /// hook answered: a declared type starts with an empty compound.
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let _ = ENDOFLAME_KEY.set(r.block_key(ENDOFLAME));
        let _ = SPREADER_KEY.set(r.block_key(MANA_SPREADER));
        let _ = POOL_KEY.set(r.block_key(MANA_POOL));
        let _ = SPREADER_ITEM.set(r.item_key(MANA_SPREADER));

        let endoflame = r
            .block_entity_type(ENDOFLAME_TYPE)
            .blocks(&[ENDOFLAME])
            .build();
        let _ = r.block_entity_part(&endoflame, EndoflamePart);

        let spreader = r
            .block_entity_type(SPREADER_TYPE)
            .blocks(&[MANA_SPREADER])
            .build();
        let _ = r.block_entity_part(&spreader, SpreaderPart);

        let _ = r.block_entity_type(POOL_TYPE).blocks(&[MANA_POOL]).build();

        let spreader_item = r.item_key(MANA_SPREADER);
        r.item_part(spreader_item, SpreaderPlacementPart);
    }

    /// A right click on a mana pool shows its mana. Upstream reads a pool with a
    /// wand or a mana tablet; this slice has neither, so the pool answers an
    /// empty hand with the numbers the wand HUD shows, and claims the click.
    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        let pos = event.click.pos;
        let Some(level) = host.level(event.click.dimension.key().id()) else {
            return Flow::Continue;
        };
        let Some(pool) = pool_at(host, level, pos) else {
            return Flow::Continue;
        };
        if let Some(player) = host.player(event.click.player) {
            let line = Text::translate(
                "botania.mana_pool.readout",
                vec![
                    Text::literal(pool.mana.to_string()),
                    Text::literal(runtime::POOL_CAPACITY.to_string()),
                ],
            );
            let _ = player.action_bar(host, line);
        }
        Flow::Cancel
    }

    /// `/botania info|mana|aim` (`native_register_commands` in the old bundle).
    /// Upstream has no commands; these exist so the harness can read and set a
    /// block without a lexicon (precedent: `/powah info|set`).
    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("botania info <pos:block_pos>"));
        event.commands.push(Command::op(
            "botania mana <pos:block_pos> <amount:integer>",
            2,
        ));
        event
            .commands
            .push(Command::op("botania aim <pos:block_pos> <axis:word>", 2));
    }

    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let invocation = event.command;
        let Some(CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos") else {
            return Flow::Continue;
        };
        let pos = *pos;
        match invocation.literals.get(1).map(String::as_str) {
            Some("info") => {
                let Some(level) = level_of(host, invocation) else {
                    feedback(host, format!("{pos:?} is not a mod block entity."));
                    return Flow::Cancel;
                };
                let line = readout(host, level, pos);
                feedback(host, line);
                Flow::Cancel
            }
            Some("mana") => {
                let Some(CommandArgumentValue::Integer(amount)) = invocation.argument("amount")
                else {
                    feedback(host, "usage: /botania mana <x> <y> <z> <amount>".to_owned());
                    return Flow::Cancel;
                };
                let Some(level) = level_of(host, invocation) else {
                    feedback(host, format!("{pos:?} is not a Botania mana block."));
                    return Flow::Cancel;
                };
                let Some(id) = block_id(host, level, pos) else {
                    feedback(host, format!("{pos:?} is not a Botania mana block."));
                    return Flow::Cancel;
                };
                let capacity = match id.as_str() {
                    ENDOFLAME => runtime::ENDOFLAME_MAX_MANA,
                    MANA_SPREADER => SPREADER_CAPACITY,
                    MANA_POOL => runtime::POOL_CAPACITY,
                    _ => {
                        feedback(host, format!("{pos:?} is not a Botania mana block."));
                        return Flow::Cancel;
                    }
                };
                let mut data = Compound::new();
                write_int(&mut data, KEY_MANA, (*amount).clamp(0, capacity));
                let changed = merge_data(host, level, pos, data);
                feedback(
                    host,
                    if changed {
                        format!("{id} at {pos:?}: set to {amount} mana.")
                    } else {
                        format!("{pos:?} is not a Botania mana block.")
                    },
                );
                Flow::Cancel
            }
            Some("aim") => {
                let Some(CommandArgumentValue::String(axis)) = invocation.argument("axis") else {
                    feedback(
                        host,
                        "usage: /botania aim <x> <y> <z> <north|south|east|west|up|down>"
                            .to_owned(),
                    );
                    return Flow::Cancel;
                };
                let Some(aim) = axis_from_name(axis) else {
                    feedback(
                        host,
                        "usage: /botania aim <x> <y> <z> <north|south|east|west|up|down>"
                            .to_owned(),
                    );
                    return Flow::Cancel;
                };
                let Some(level) = level_of(host, invocation) else {
                    feedback(host, format!("{pos:?} is not a mana spreader."));
                    return Flow::Cancel;
                };
                if !is_block(&*host, level, pos, &SPREADER_KEY) {
                    feedback(host, format!("{pos:?} is not a mana spreader."));
                    return Flow::Cancel;
                }
                let (rotation_x, rotation_y) = rotation_for_aim(aim);
                let mut data = Compound::new();
                write_float(&mut data, KEY_ROTATION_X, rotation_x);
                write_float(&mut data, KEY_ROTATION_Y, rotation_y);
                let changed = merge_data(host, level, pos, data);
                feedback(
                    host,
                    if changed {
                        format!("Mana Spreader at {pos:?}: aim set to {aim:?}.")
                    } else {
                        format!("{pos:?} is not a mana spreader.")
                    },
                );
                Flow::Cancel
            }
            _ => Flow::Continue,
        }
    }
}

// ---------------------------------------------------------------------------
// The block entities' ticks
// ---------------------------------------------------------------------------

fn tick_endoflame(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], nbt: &mut Compound) {
    let mut flower = Endoflame {
        mana: read_int(nbt, KEY_MANA).clamp(0, runtime::ENDOFLAME_MAX_MANA),
        burn_time: read_int(nbt, KEY_BURN_TIME).max(0) as u32,
        age: read_int(nbt, KEY_AGE).max(0) as u32,
    };
    // A flower with no room takes no fuel, so it does not even look for any.
    let mut fuel = if flower.room() > 0 {
        dropped_fuel(host, level, pos)
    } else {
        DroppedFuel::none()
    };
    flower.tick(&mut fuel);
    if let Some(item) = fuel.taken {
        consume_one(host, item);
    }
    // `emptyManaIntoCollector`: the nearest spreader within `LINK_RANGE`.
    if flower.mana > 0
        && let Some(collector) = nearest_spreader(host, level, pos)
        && let Some(mut target) = spreader_at(host, level, collector)
        && target.room() > 0
    {
        let moved = flower.empty_into(target.mana, SPREADER_CAPACITY);
        target.receive(moved);
        let mut data = Compound::new();
        write_int(&mut data, KEY_MANA, target.mana);
        merge_data(host, level, collector, data);
    }
    write_int(nbt, KEY_MANA, flower.mana);
    write_int(nbt, KEY_BURN_TIME, flower.burn_time as i32);
    write_int(nbt, KEY_AGE, flower.age as i32);
}

fn tick_spreader(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], nbt: &mut Compound) {
    let mut spreader = Spreader {
        mana: read_int(nbt, KEY_MANA).clamp(0, SPREADER_CAPACITY),
        can_shoot: read_bool(nbt, KEY_CAN_SHOOT, true),
        ticks_left: read_int(nbt, KEY_TICKS_LEFT).max(0) as u32,
        payload: read_int(nbt, KEY_PAYLOAD),
        arrival: read_int(nbt, KEY_ARRIVAL),
    };
    let aim = aim_direction(
        read_float(nbt, KEY_ROTATION_X),
        read_float(nbt, KEY_ROTATION_Y),
    );
    // The receiver this tick: a pool along the aim, as `checkForReceiver`'s
    // simulated burst finds one. A spreader with no pool in its path fires
    // nothing.
    let receiver = find_receiver(host, level, pos, aim);
    let tick = spreader.tick(receiver);
    if tick.arrived > 0
        && let Some(target) = receiver
        && let Some(pool_pos) = target.pos
        && let Some(mut pool) = pool_at(host, level, pool_pos)
        && pool.room() > 0
    {
        pool.receive(tick.arrived);
        let mut data = Compound::new();
        write_int(&mut data, KEY_MANA, pool.mana);
        merge_data(host, level, pool_pos, data);
    }
    write_int(nbt, KEY_MANA, spreader.mana);
    write_bool(nbt, KEY_CAN_SHOOT, spreader.can_shoot);
    write_int(nbt, KEY_TICKS_LEFT, spreader.ticks_left as i32);
    write_int(nbt, KEY_PAYLOAD, spreader.payload);
    write_int(nbt, KEY_ARRIVAL, spreader.arrival);
}

/// The spreader's receiver: the first mana pool along its aim, with the
/// distance the burst flies and whether the pool is full.
fn find_receiver(
    host: &dyn Host,
    level: LevelRef,
    pos: [i32; 3],
    aim: [i32; 3],
) -> Option<BurstTarget> {
    for step in 1..=RECEIVER_SCAN {
        let at = [
            pos[0] + aim[0] * step,
            pos[1] + aim[1] * step,
            pos[2] + aim[2] * step,
        ];
        if !is_block(host, level, at, &POOL_KEY) {
            continue;
        }
        let pool = pool_at(host, level, at)?;
        return Some(BurstTarget {
            distance: f64::from(step),
            full: pool.is_full(),
            pos: Some(at),
        });
    }
    None
}

/// The nearest mana spreader within `LINK_RANGE` of `pos` (upstream's
/// `ManaNetwork.getClosestCollector`), by squared distance.
fn nearest_spreader(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<[i32; 3]> {
    let mut best: Option<(i32, [i32; 3])> = None;
    for dx in -runtime::FLOWER_LINK_RANGE..=runtime::FLOWER_LINK_RANGE {
        for dy in -runtime::FLOWER_LINK_RANGE..=runtime::FLOWER_LINK_RANGE {
            for dz in -runtime::FLOWER_LINK_RANGE..=runtime::FLOWER_LINK_RANGE {
                if !within_link_range([dx, dy, dz]) {
                    continue;
                }
                let at = [pos[0] + dx, pos[1] + dy, pos[2] + dz];
                if !is_block(host, level, at, &SPREADER_KEY) {
                    continue;
                }
                let squared = dx * dx + dy * dy + dz * dz;
                if best.is_none_or(|(best, _)| squared < best) {
                    best = Some((squared, at));
                }
            }
        }
    }
    best.map(|(_, at)| at)
}

/// A neighbouring spreader's state, as its own tick would read it.
fn spreader_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<Spreader> {
    if !is_block(host, level, pos, &SPREADER_KEY) {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    let nbt = entity.data(host)?;
    Some(Spreader {
        mana: read_int(&nbt, KEY_MANA).clamp(0, SPREADER_CAPACITY),
        ..Spreader::new()
    })
}

fn pool_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<Pool> {
    if !is_block(host, level, pos, &POOL_KEY) {
        return None;
    }
    let entity = host.block_entity(level, pos)?;
    let nbt = entity.data(host)?;
    Some(Pool {
        mana: read_int(&nbt, KEY_MANA).clamp(0, runtime::POOL_CAPACITY),
    })
}

// ---------------------------------------------------------------------------
// The dropped fuel
// ---------------------------------------------------------------------------

/// The dropped items the flower may burn, as `EndoflameBlockEntity` reads them:
/// the first item in its 7x7x7 box whose smelting burn time is positive, that
/// leaves no crafting remainder and is not a mana spreader.
struct DroppedFuel {
    burn_time: Option<u32>,
    item: Option<ItemEntityRef>,
    taken: Option<ItemEntityRef>,
}

impl DroppedFuel {
    /// Nothing to burn: a flower with a full buffer does not scan for items.
    fn none() -> Self {
        Self {
            burn_time: None,
            item: None,
            taken: None,
        }
    }
}

impl FuelSource for DroppedFuel {
    fn peek(&self) -> Option<u32> {
        self.burn_time
    }

    fn take(&mut self) {
        self.taken = self.item;
    }
}

/// Pick the first burnable dropped item in the flower's range.
fn dropped_fuel(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> DroppedFuel {
    let range = runtime::ENDOFLAME_RANGE;
    let min = [
        f64::from(pos[0] - range),
        f64::from(pos[1] - range),
        f64::from(pos[2] - range),
    ];
    let max = [
        f64::from(pos[0] + range + 1),
        f64::from(pos[1] + range + 1),
        f64::from(pos[2] + range + 1),
    ];
    let Some(entities) = level.entities_in(host, min, max) else {
        return DroppedFuel::none();
    };
    for entity in entities {
        let Some(item) = host.item_entity(entity) else {
            continue;
        };
        let Some(stack) = item.stack(host) else {
            continue;
        };
        let Some(burn_time) = burn_time_of(host, &stack) else {
            continue;
        };
        return DroppedFuel {
            burn_time: Some(burn_time),
            item: Some(item),
            taken: None,
        };
    }
    DroppedFuel::none()
}

/// `EndoflameBlockEntity.getBurnTime`: an item with a smelting burn time, no
/// crafting remainder and no mana spreader block.
fn burn_time_of(host: &dyn Host, stack: &ItemStack) -> Option<u32> {
    if stack.count() == 0 || Some(stack.item()) == SPREADER_ITEM.get().copied() {
        return None;
    }
    if host.item_crafting_remainder(stack).is_some() {
        return None;
    }
    let burn = host.item_burn_time(stack.item())?;
    (burn > 0).then(|| u32::try_from(burn).unwrap_or(0))
}

/// `EntityHelper.shrinkItem`: take one item out of the dropped stack; an empty
/// stack is removed (`ItemEntity.discard`) rather than written as a zero count.
fn consume_one(host: &mut dyn ServerHost, item: ItemEntityRef) {
    let Some(stack) = item.stack(host) else {
        return;
    };
    let count = stack.count();
    if count > 1 {
        let mut left = stack;
        if left.set_count(count - 1).is_ok() {
            let _ = item.set_stack(host, left);
        }
    } else {
        let _ = item.entity().remove(host, RemovalReason::Discarded);
    }
}

// ---------------------------------------------------------------------------
// Readout, commands and level helpers
// ---------------------------------------------------------------------------

/// A block's state, as `/botania info` prints it.
fn readout(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> String {
    let Some(nbt) = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(host))
    else {
        return format!("{pos:?} is not a mod block entity.");
    };
    let time = level.game_time(host).unwrap_or(0);
    match block_id(host, level, pos).as_deref() {
        Some(ENDOFLAME) => format!(
            "Endoflame at {pos:?}: {}/{} mana, burn {} ticks, t={time}",
            read_int(&nbt, KEY_MANA).clamp(0, runtime::ENDOFLAME_MAX_MANA),
            runtime::ENDOFLAME_MAX_MANA,
            read_int(&nbt, KEY_BURN_TIME),
        ),
        Some(MANA_SPREADER) => {
            let aim = aim_direction(
                read_float(&nbt, KEY_ROTATION_X),
                read_float(&nbt, KEY_ROTATION_Y),
            );
            format!(
                "Mana Spreader at {pos:?}: {}/{} mana, aim {aim:?}, burst {} left, in flight {}, t={time}",
                read_int(&nbt, KEY_MANA).clamp(0, SPREADER_CAPACITY),
                SPREADER_CAPACITY,
                read_bool(&nbt, KEY_CAN_SHOOT, true),
                read_int(&nbt, KEY_TICKS_LEFT),
            )
        }
        Some(MANA_POOL) => format!(
            "Mana Pool at {pos:?}: {}/{} mana, t={time}",
            read_int(&nbt, KEY_MANA).clamp(0, runtime::POOL_CAPACITY),
            runtime::POOL_CAPACITY,
        ),
        Some(other) => format!("{pos:?} is a {other} block entity."),
        None => format!("{pos:?} is not a mod block entity."),
    }
}

/// `/botania aim`'s six axis words.
fn axis_from_name(name: &str) -> Option<[i32; 3]> {
    match name {
        "north" => Some([0, 0, -1]),
        "south" => Some([0, 0, 1]),
        "east" => Some([1, 0, 0]),
        "west" => Some([-1, 0, 0]),
        "up" => Some([0, 1, 0]),
        "down" => Some([0, -1, 0]),
        _ => None,
    }
}

/// The level a command executes in, from the dimension its invocation named.
fn level_of(host: &dyn ServerHost, invocation: &mod_api::CommandInvocation) -> Option<LevelRef> {
    invocation.dimension.id(host).and_then(|id| host.level(&id))
}

/// The level whose block entities the server ticks now. A block-entity tick is
/// not told its level; the level in place is the one being ticked.
fn level_in_place(host: &dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

/// Whether `pos` holds the block registered under `key`.
fn is_block(host: &dyn Host, level: LevelRef, pos: [i32; 3], key: &OnceLock<BlockKey>) -> bool {
    let Some(found) = level.block(host, pos) else {
        return false;
    };
    key.get().is_some_and(|wanted| *wanted == found)
}

fn block_id(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    level.block(host, pos).map(|key| key.id(host))
}

/// Merge `data` into the block entity at `pos` (`BlockDataAccessor.setData`:
/// load, `setChanged`, and a client update). Answers whether it changed.
fn merge_data(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], data: Compound) -> bool {
    host.block_entity(level, pos)
        .is_some_and(|entity| entity.merge_data(host, data).unwrap_or(false))
}

fn feedback(host: &mut dyn ServerHost, line: String) {
    let _ = host.commands().feedback(Text::literal(line));
}

// ---------------------------------------------------------------------------
// Saved-compound reads
// ---------------------------------------------------------------------------

/// `key`'s tag, with Java's UTF-16 key spelling.
fn tag<'a>(nbt: &'a Compound, key: &str) -> Option<&'a Tag> {
    let units: Vec<u16> = key.encode_utf16().collect();
    nbt.iter()
        .find(|(name, _)| name.0 == units)
        .map(|(_, tag)| tag)
}

fn read_int(nbt: &Compound, key: &str) -> i32 {
    match tag(nbt, key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

fn write_int(nbt: &mut Compound, key: &str, value: i32) {
    nbt.insert(key.into(), Tag::Int(value));
}

/// `Tag::Float` holds the value's bits, as Java's NBT float does.
fn read_float(nbt: &Compound, key: &str) -> f32 {
    match tag(nbt, key) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 0.0,
    }
}

fn write_float(nbt: &mut Compound, key: &str, value: f32) {
    nbt.insert(key.into(), Tag::Float(value.to_bits()));
}

fn read_bool(nbt: &Compound, key: &str, default: bool) -> bool {
    match tag(nbt, key) {
        Some(Tag::Byte(value)) => *value != 0,
        _ => default,
    }
}

fn write_bool(nbt: &mut Compound, key: &str, value: bool) {
    nbt.insert(key.into(), Tag::Byte(i8::from(value)));
}
