// Pure Botania mana arithmetic (standalone crate lib, injected as
// `crate::game::botania`). No game access lives here: the endoflame's fuel burn
// and mana trickle, the spreader's burst and its loss over flight, the pool's
// capacity, and the flower-to-spreader link.
//
// Every number below is upstream's own, read from VazkiiMods/Botania at
// `release-1.20.1-456`:
//
// - `EndoflameBlockEntity`: `getMaxMana()` 300, `addMana(3)` every second tick
//   (`ticksExisted % 2 == 0`), `RANGE` 3, `FUEL_CAP` 32000, and
//   `burnTime = min(FUEL_CAP, smeltingBurnTime) / 2`.
// - `GeneratingFlowerBlockEntity`: `LINK_RANGE` 6 and `emptyManaIntoCollector`
//   (the flower pushes everything it holds into its bound spreader each tick).
// - `ManaSpreaderBlock.Variant.MANA`: `burstMana` 160, `manaCapacity` 1000,
//   `preLossTicks` 60, `lossPerTick` 4, `motionModifier` 1.
// - `ManaBurstEntity`: velocity `0.4 / 2` blocks a tick and, from flight tick
//   `ticksBeforeManaLoss` on, `manaLossPerTick` mana a tick.
// - `ManaPoolBlockEntity`: `MAX_MANA` 1,000,000.
//
// See `../../FINDINGS.md` for the file-and-line record.

/// `EndoflameBlockEntity.getMaxMana()`: the flower's own buffer.
pub const ENDOFLAME_MAX_MANA: i32 = 300;
/// `EndoflameBlockEntity.tickFlower`'s `addMana(3)`.
pub const ENDOFLAME_MANA_PER_ADD: i32 = 3;
/// The `ticksExisted % 2 == 0` gate on that add.
pub const ENDOFLAME_ADD_PERIOD: u32 = 2;
/// `EndoflameBlockEntity.FUEL_CAP`: the longest burn a single item may give.
pub const ENDOFLAME_FUEL_CAP: u32 = 32_000;
/// `EndoflameBlockEntity.RANGE`: the square the flower picks items up from.
pub const ENDOFLAME_RANGE: i32 = 3;
/// `GeneratingFlowerBlockEntity.LINK_RANGE`: how far a flower reaches a spreader.
pub const FLOWER_LINK_RANGE: i32 = 6;
/// `ManaSpreaderBlock.Variant.MANA.manaCapacity`.
pub const SPREADER_CAPACITY: i32 = 1000;
/// `ManaSpreaderBlock.Variant.MANA.burstMana`.
pub const SPREADER_BURST_MANA: i32 = 160;
/// `ManaSpreaderBlock.Variant.MANA.preLossTicks`: flight ticks at full mana.
pub const BURST_PRE_LOSS_TICKS: u32 = 60;
/// `ManaSpreaderBlock.Variant.MANA.lossPerTick`.
pub const BURST_LOSS_PER_TICK: i32 = 4;
/// `ManaBurstEntity.calculateBurstVelocity`'s `0.4F / 2D`: blocks a tick.
pub const BURST_SPEED_PER_TICK: f64 = 0.2;
/// `ManaPoolBlockEntity.MAX_MANA`.
pub const POOL_CAPACITY: i32 = 1_000_000;

/// The next dropped item the flower may burn, as the world reads it.
pub trait FuelSource {
    /// The item's smelting burn time in ticks, `None` when the item may not be
    /// burned at all (`EndoflameBlockEntity.getBurnTime`: an empty stack, a mana
    /// spreader item, or an item with no smelting recipe).
    fn peek(&self) -> Option<u32>;
    /// `EntityHelper.shrinkItem`: take one item out of the stack.
    fn take(&mut self);
}

/// What one tick of the flower did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EndoflameTick {
    /// Mana the flower added to its own buffer this tick (0 or 3).
    pub added: i32,
    /// Items taken out of the world this tick (0 or 1).
    pub taken: u32,
    /// The burn time the item it took this tick gives, in ticks.
    pub ignited: u32,
}

/// `EndoflameBlockEntity`'s saved state: its mana, the burn left, and the
/// `ticksExisted` its `% 2` gate reads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Endoflame {
    /// Mana in the flower (0..=`ENDOFLAME_MAX_MANA`).
    pub mana: i32,
    /// Burn ticks left; upstream's `burnTime`.
    pub burn_time: u32,
    /// `BlockEntity.ticksExisted`, saved so the parity survives a reload.
    pub age: u32,
}

impl Endoflame {
    /// An empty flower.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The mana the flower can still take.
    #[must_use]
    pub fn room(&self) -> i32 {
        ENDOFLAME_MAX_MANA.saturating_sub(self.mana).max(0)
    }

    /// One server tick of `tickFlower`: spend a burn tick, add 3 mana on the
    /// flower's even ticks while it burns, and take the next burnable item when
    /// the burn has run out and the flower has room.
    ///
    /// Upstream's order is kept exactly: the burn is decremented *first*, the
    /// mana is added only while `burnTime > 0` afterwards, and the fuel scan
    /// runs in the same tick the burn reaches zero, so a coal is picked up on
    /// the tick after the previous one ended.
    pub fn tick(&mut self, fuel: &mut impl FuelSource) -> EndoflameTick {
        let mut tick = EndoflameTick::default();
        let was_burning = self.burn_time > 0;
        if was_burning {
            self.burn_time -= 1;
        }
        if self.burn_time > 0 && self.age.is_multiple_of(ENDOFLAME_ADD_PERIOD) {
            let added = ENDOFLAME_MANA_PER_ADD.min(self.room());
            self.mana += added;
            tick.added = added;
        }
        if self.burn_time == 0
            && self.mana < ENDOFLAME_MAX_MANA
            && let Some(burn_time) = fuel.peek().filter(|burn| *burn > 0)
        {
            fuel.take();
            tick.taken = 1;
            self.burn_time = fuel_burn_ticks(burn_time);
            tick.ignited = self.burn_time;
        }
        self.age = self.age.wrapping_add(1);
        tick
    }

    /// `GeneratingFlowerBlockEntity.emptyManaIntoCollector`: hand everything the
    /// flower holds to its spreader, as far as the spreader has room.
    pub fn empty_into(&mut self, collector_mana: i32, collector_capacity: i32) -> i32 {
        let room = collector_capacity.saturating_sub(collector_mana).max(0);
        let moved = self.mana.clamp(0, room);
        self.mana -= moved;
        moved
    }
}

/// `EndoflameBlockEntity`: `min(FUEL_CAP, burnTime) / 2`.
#[must_use]
pub fn fuel_burn_ticks(smelting_burn_time: u32) -> u32 {
    smelting_burn_time.min(ENDOFLAME_FUEL_CAP) / 2
}

/// The `Variant.MANA` spreader's saved state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spreader {
    /// Mana in the spreader (0..=`SPREADER_CAPACITY`).
    pub mana: i32,
    /// `canShootBurst`: false while a burst of this spreader's is in flight.
    pub can_shoot: bool,
    /// Flight ticks left for the burst in flight.
    pub ticks_left: u32,
    /// The mana that burst started with.
    pub payload: i32,
    /// The mana that burst will hand its receiver when it lands, after its loss.
    pub arrival: i32,
}

impl Default for Spreader {
    fn default() -> Self {
        Self {
            mana: 0,
            can_shoot: true,
            ticks_left: 0,
            payload: 0,
            arrival: 0,
        }
    }
}

/// One spreader tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpreaderTick {
    /// Mana that arrived at the receiver this tick (`0` unless a burst landed).
    pub arrived: i32,
    /// Whether a burst left the spreader this tick.
    pub fired: bool,
}

impl Spreader {
    /// An empty spreader.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The mana the spreader can still take.
    #[must_use]
    pub fn room(&self) -> i32 {
        SPREADER_CAPACITY.saturating_sub(self.mana).max(0)
    }

    /// `ManaSpreaderBlockEntity.receiveMana`.
    pub fn receive(&mut self, mana: i32) -> i32 {
        let before = self.mana;
        self.mana = self.mana.saturating_add(mana).clamp(0, SPREADER_CAPACITY);
        self.mana - before
    }

    /// One tick of `commonTick` + `tryShootBurst`: land the burst in flight, or
    /// fire a new one at a receiver `distance` blocks away.
    ///
    /// A burst that lands gives its receiver `mana_at_arrival`; a burst cannot
    /// be fired while another is in flight (`canShootBurst`) and needs a whole
    /// `SPREADER_BURST_MANA` in the spreader and a receiver that is not full.
    pub fn tick(&mut self, receiver: Option<BurstTarget>) -> SpreaderTick {
        let mut tick = SpreaderTick::default();
        if !self.can_shoot {
            self.ticks_left = self.ticks_left.saturating_sub(1);
            if self.ticks_left == 0 {
                self.can_shoot = true;
                tick.arrived = self.arrival;
                self.arrival = 0;
            }
            return tick;
        }
        let Some(target) = receiver else {
            return tick;
        };
        if target.full || self.mana < SPREADER_BURST_MANA {
            return tick;
        }
        self.mana -= SPREADER_BURST_MANA;
        self.can_shoot = false;
        self.payload = SPREADER_BURST_MANA;
        self.ticks_left = travel_ticks(target.distance);
        self.arrival = mana_at_arrival(SPREADER_BURST_MANA, self.ticks_left);
        tick.fired = true;
        tick
    }
}

/// The spreader's receiver, as the world found it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurstTarget {
    /// Blocks from the spreader's centre to the receiver.
    pub distance: f64,
    /// `ManaReceiver.isFull`: a full pool takes no burst.
    pub full: bool,
    /// Where the receiver stands, for the glue that hands it the mana. The
    /// runtime never reads it: a pure test leaves it `None`.
    pub pos: Option<[i32; 3]>,
}

/// `ManaBurstEntity.calculateBurstVelocity` at `BURST_SPEED_PER_TICK`: the
/// flight ticks a burst needs for `distance` blocks (the burst moves a fixed
/// step each tick and is tested at its position, so the last partial step still
/// costs a tick).
#[must_use]
pub fn travel_ticks(distance: f64) -> u32 {
    if !distance.is_finite() || distance <= 0.0 {
        return 1;
    }
    let ticks = (distance / BURST_SPEED_PER_TICK).ceil();
    ticks.clamp(1.0, f64::from(u32::MAX)) as u32
}

/// `ManaBurstEntity.tick`'s loss: from flight tick `BURST_PRE_LOSS_TICKS` on,
/// `BURST_LOSS_PER_TICK` mana a tick, never below zero.
#[must_use]
pub fn mana_at_arrival(starting_mana: i32, flight_ticks: u32) -> i32 {
    let losing_ticks = if flight_ticks >= BURST_PRE_LOSS_TICKS {
        flight_ticks - BURST_PRE_LOSS_TICKS + 1
    } else {
        0
    };
    let lost = i32::try_from(losing_ticks).unwrap_or(i32::MAX) * BURST_LOSS_PER_TICK;
    starting_mana.saturating_sub(lost).max(0)
}

/// `ManaSpreaderBlock.setPlacedBy`: a spreader fires the way its placer looked
/// (place it facing the pool). Upstream stores that as a `rotationX` /
/// `rotationY` degree pair, and `ManaBurstEntity.calculateBurstVelocity` reads
/// the pair back into one of the six axes; `rotation_for_aim` is that pair and
/// `aim_direction` is the inverse.
#[must_use]
pub fn aim_direction(rotation_x: f32, rotation_y: f32) -> [i32; 3] {
    if rotation_y <= -45.0 {
        [0, -1, 0]
    } else if rotation_y >= 45.0 {
        [0, 1, 0]
    } else if (135.0..225.0).contains(&rotation_x) {
        [1, 0, 0]
    } else if (225.0..315.0).contains(&rotation_x) {
        [0, 0, -1]
    } else if (45.0..135.0).contains(&rotation_x) {
        [0, 0, 1]
    } else {
        [-1, 0, 0]
    }
}

/// The `rotationX` / `rotationY` upstream's `setPlacedBy` stores for an aim.
#[must_use]
pub fn rotation_for_aim(aim: [i32; 3]) -> (f32, f32) {
    match aim {
        [0, -1, 0] => (0.0, -90.0),
        [0, 1, 0] => (0.0, 90.0),
        [0, 0, -1] => (270.0, 0.0),
        [0, 0, 1] => (90.0, 0.0),
        [1, 0, 0] => (180.0, 0.0),
        _ => (0.0, 0.0),
    }
}

/// `Direction.orderedByNearest(placer)[0]`: the axis closest to the player's
/// look, from their yaw and pitch in degrees (the direction the spreader fires).
#[must_use]
pub fn aim_away_from(yaw: f32, pitch: f32) -> [i32; 3] {
    let yaw = yaw.to_radians();
    let pitch = pitch.to_radians();
    let look = [
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        yaw.cos() * pitch.cos(),
    ];
    let axes = [
        (look[0].abs(), [1, 0, 0]),
        (look[1].abs(), [0, 1, 0]),
        (look[2].abs(), [0, 0, 1]),
    ];
    let mut best = axes[0];
    for axis in axes {
        if axis.0 > best.0 {
            best = axis;
        }
    }
    let [x, y, z] = best.1;
    let sign = if look[0] * f32::from(x as i8)
        + look[1] * f32::from(y as i8)
        + look[2] * f32::from(z as i8)
        >= 0.0
    {
        1
    } else {
        -1
    };
    [x * sign, y * sign, z * sign]
}

/// A mana pool's saved state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pool {
    /// Mana held (0..=`POOL_CAPACITY`).
    pub mana: i32,
}

impl Pool {
    /// An empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The mana the pool can still take.
    #[must_use]
    pub fn room(&self) -> i32 {
        POOL_CAPACITY.saturating_sub(self.mana).max(0)
    }

    /// `ManaPoolBlockEntity.receiveMana`: clamped to the pool's capacity, a
    /// negative amount drains it. Answers what was actually taken.
    pub fn receive(&mut self, mana: i32) -> i32 {
        let before = self.mana;
        self.mana = self.mana.saturating_add(mana).clamp(0, POOL_CAPACITY);
        self.mana - before
    }

    /// `isFull`: a pool at its capacity takes no burst.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.mana >= POOL_CAPACITY
    }
}

/// `ManaNetwork.getClosestCollector`'s range test, squared so it stays integer:
/// a flower reaches a spreader within `FLOWER_LINK_RANGE` blocks of its centre.
#[must_use]
pub fn within_link_range(offset: [i32; 3]) -> bool {
    let squared: i32 = offset.iter().map(|d| d * d).sum();
    squared <= FLOWER_LINK_RANGE * FLOWER_LINK_RANGE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fuel source over a list of smelting burn times.
    struct FakeFuel {
        items: Vec<u32>,
    }

    impl FakeFuel {
        fn of(items: &[u32]) -> Self {
            Self {
                items: items.to_vec(),
            }
        }
    }

    impl FuelSource for FakeFuel {
        fn peek(&self) -> Option<u32> {
            self.items.first().copied()
        }

        fn take(&mut self) {
            if !self.items.is_empty() {
                self.items.remove(0);
            }
        }
    }

    /// Run a flower for `ticks` ticks, emptying it into a bottomless collector
    /// every tick, and answer the mana it made in all.
    fn run_flower(flower: &mut Endoflame, fuel: &mut FakeFuel, ticks: u32) -> i32 {
        let mut total = 0;
        for _ in 0..ticks {
            total += flower.tick(fuel).added;
            flower.empty_into(0, i32::MAX);
        }
        total
    }

    #[test]
    fn the_endoflame_constants_are_the_jars() {
        assert_eq!(ENDOFLAME_MAX_MANA, 300);
        assert_eq!(ENDOFLAME_MANA_PER_ADD, 3);
        assert_eq!(ENDOFLAME_FUEL_CAP, 32_000);
        assert_eq!(ENDOFLAME_RANGE, 3);
        assert_eq!(FLOWER_LINK_RANGE, 6);
        assert_eq!(SPREADER_CAPACITY, 1000);
        assert_eq!(SPREADER_BURST_MANA, 160);
        assert_eq!(BURST_PRE_LOSS_TICKS, 60);
        assert_eq!(BURST_LOSS_PER_TICK, 4);
        assert_eq!(POOL_CAPACITY, 1_000_000);
    }

    #[test]
    fn a_burning_flower_adds_three_mana_every_second_tick() {
        let mut flower = Endoflame {
            burn_time: 20,
            ..Endoflame::new()
        };
        let mut fuel = FakeFuel::of(&[]);
        // Age 0 (even) adds, age 1 (odd) does not: 3 mana every two ticks.
        assert_eq!(flower.tick(&mut fuel).added, 3);
        assert_eq!(flower.tick(&mut fuel).added, 0);
        assert_eq!(flower.tick(&mut fuel).added, 3);
        assert_eq!(flower.mana, 6);
    }

    #[test]
    fn one_coal_is_800_burn_ticks_and_about_1200_mana() {
        // A coal's smelting burn time is 1600 ticks; the endoflame halves it and
        // adds 3 mana on every second tick of the 800. 800 burn ticks at 3 mana
        // every other tick is 1200; the tick that ignites the coal also spends
        // an age slot without adding (`tickFlower` decrements before it adds and
        // ignites after), so an empty flower gets 399 adds and a burning one
        // 400. Both numbers are upstream's, not rounded.
        let mut burning = Endoflame {
            burn_time: 800,
            ..Endoflame::new()
        };
        let mut no_fuel = FakeFuel::of(&[]);
        assert_eq!(run_flower(&mut burning, &mut no_fuel, 900), 1200);

        let mut empty = Endoflame::new();
        let mut fuel = FakeFuel::of(&[1600]);
        let total = run_flower(&mut empty, &mut fuel, 900);
        assert_eq!(total, 1197);
        assert_eq!(empty.burn_time, 0);
        assert!(fuel.items.is_empty());
    }

    #[test]
    fn a_long_burn_is_capped_at_32000_smelting_ticks() {
        assert_eq!(fuel_burn_ticks(1600), 800);
        assert_eq!(fuel_burn_ticks(32_000), 16_000);
        assert_eq!(fuel_burn_ticks(40_000), 16_000);
        assert_eq!(fuel_burn_ticks(0), 0);
        assert_eq!(fuel_burn_ticks(1), 0);
    }

    #[test]
    fn a_flower_without_fuel_makes_no_mana() {
        let mut flower = Endoflame::new();
        let mut fuel = FakeFuel::of(&[]);
        let total = run_flower(&mut flower, &mut fuel, 200);
        assert_eq!(total, 0);
        assert_eq!(flower.mana, 0);
        assert_eq!(flower.burn_time, 0);
    }

    #[test]
    fn the_flower_takes_a_fuel_item_only_when_its_burn_ran_out() {
        let mut flower = Endoflame::new();
        let mut fuel = FakeFuel::of(&[200, 200]);
        let tick = flower.tick(&mut fuel);
        assert_eq!(tick.taken, 1);
        assert_eq!(tick.ignited, 100);
        assert_eq!(fuel.items.len(), 1);
        // While it burns, the next item stays on the ground.
        for _ in 0..50 {
            flower.tick(&mut fuel);
            flower.empty_into(0, i32::MAX);
        }
        assert_eq!(fuel.items.len(), 1);
        // ... and is taken the tick the burn reaches zero.
        for _ in 0..60 {
            flower.tick(&mut fuel);
            flower.empty_into(0, i32::MAX);
        }
        assert_eq!(fuel.items.len(), 0);
    }

    #[test]
    fn a_full_flower_takes_no_fuel() {
        let mut flower = Endoflame {
            mana: ENDOFLAME_MAX_MANA,
            ..Endoflame::new()
        };
        let mut fuel = FakeFuel::of(&[1600]);
        let tick = flower.tick(&mut fuel);
        assert_eq!(tick.taken, 0);
        assert_eq!(fuel.items.len(), 1);
        assert_eq!(flower.mana, ENDOFLAME_MAX_MANA);
    }

    #[test]
    fn the_flower_buffer_stops_at_300() {
        let mut flower = Endoflame {
            mana: 299,
            burn_time: 100,
            ..Endoflame::new()
        };
        let mut fuel = FakeFuel::of(&[]);
        assert_eq!(flower.tick(&mut fuel).added, 1);
        assert_eq!(flower.mana, ENDOFLAME_MAX_MANA);
        assert_eq!(flower.room(), 0);
    }

    #[test]
    fn the_flower_empties_into_its_spreader_only_as_far_as_its_room() {
        let mut flower = Endoflame {
            mana: 300,
            ..Endoflame::new()
        };
        assert_eq!(flower.empty_into(900, SPREADER_CAPACITY), 100);
        assert_eq!(flower.mana, 200);
        assert_eq!(flower.empty_into(0, SPREADER_CAPACITY), 200);
        assert_eq!(flower.mana, 0);
        assert_eq!(flower.empty_into(0, SPREADER_CAPACITY), 0);
    }

    #[test]
    fn the_spreader_takes_mana_up_to_1000() {
        let mut spreader = Spreader::new();
        assert_eq!(spreader.receive(600), 600);
        assert_eq!(spreader.receive(600), 400);
        assert_eq!(spreader.mana, SPREADER_CAPACITY);
        assert_eq!(spreader.room(), 0);
    }

    #[test]
    fn a_burst_carries_160_mana_and_blocks_the_next_until_it_lands() {
        let mut spreader = Spreader {
            mana: 320,
            ..Spreader::new()
        };
        let target = BurstTarget {
            distance: 2.0,
            full: false,
            pos: None,
        };
        let fired = spreader.tick(Some(target));
        assert!(fired.fired);
        assert_eq!(spreader.mana, 160);
        assert!(!spreader.can_shoot);
        // 2 blocks at 0.2 a tick is 10 ticks, and nothing is lost in 10.
        assert_eq!(spreader.ticks_left, 10);
        for _ in 0..9 {
            assert_eq!(spreader.tick(Some(target)).arrived, 0);
        }
        let landed = spreader.tick(Some(target));
        assert_eq!(landed.arrived, 160);
        assert!(spreader.can_shoot);
    }

    #[test]
    fn a_burst_past_the_pre_loss_ticks_loses_four_a_tick() {
        assert_eq!(mana_at_arrival(160, 10), 160);
        assert_eq!(mana_at_arrival(160, 59), 160);
        assert_eq!(mana_at_arrival(160, 60), 156);
        assert_eq!(mana_at_arrival(160, 65), 136);
        assert_eq!(mana_at_arrival(160, 100), 0);
        // 13 blocks is 65 ticks: 5 losing ticks after the 60th.
        assert_eq!(travel_ticks(13.0), 65);
        assert_eq!(mana_at_arrival(160, travel_ticks(13.0)), 136);
    }

    #[test]
    fn a_spreader_with_less_than_a_burst_fires_nothing() {
        let mut spreader = Spreader {
            mana: 159,
            ..Spreader::new()
        };
        let tick = spreader.tick(Some(BurstTarget {
            distance: 2.0,
            full: false,
            pos: None,
        }));
        assert!(!tick.fired);
        assert_eq!(spreader.mana, 159);
        assert!(spreader.can_shoot);
    }

    #[test]
    fn a_full_or_absent_receiver_takes_no_burst() {
        let mut spreader = Spreader {
            mana: 1000,
            ..Spreader::new()
        };
        assert!(!spreader.tick(None).fired);
        assert!(
            !spreader
                .tick(Some(BurstTarget {
                    distance: 2.0,
                    full: true,
                    pos: None,
                }))
                .fired
        );
        assert_eq!(spreader.mana, 1000);
    }

    #[test]
    fn travel_ticks_are_five_a_block() {
        assert_eq!(travel_ticks(0.0), 1);
        assert_eq!(travel_ticks(0.1), 1);
        assert_eq!(travel_ticks(0.2), 1);
        assert_eq!(travel_ticks(1.0), 5);
        assert_eq!(travel_ticks(2.0), 10);
        assert_eq!(travel_ticks(12.0), 60);
    }

    #[test]
    fn the_aim_is_the_axis_away_from_the_player() {
        // Looking down the +z axis: the spreader aims -z.
        assert_eq!(aim_away_from(0.0, 0.0), [0, 0, 1]);
        assert_eq!(aim_away_from(180.0, 0.0), [0, 0, -1]);
        assert_eq!(aim_away_from(90.0, 0.0), [-1, 0, 0]);
        assert_eq!(aim_away_from(-90.0, 0.0), [1, 0, 0]);
        // Minecraft's pitch: -90 looks up, 90 down.
        assert_eq!(aim_away_from(0.0, -89.0), [0, 1, 0]);
        assert_eq!(aim_away_from(0.0, 89.0), [0, -1, 0]);
    }

    #[test]
    fn the_rotation_pair_round_trips_through_the_aim() {
        for aim in [
            [1, 0, 0],
            [-1, 0, 0],
            [0, 1, 0],
            [0, -1, 0],
            [0, 0, 1],
            [0, 0, -1],
        ] {
            let (x, y) = rotation_for_aim(aim);
            assert_eq!(aim_direction(x, y), aim, "aim {aim:?}");
        }
        // Upstream's own numbers from `ManaSpreaderBlock.setPlacedBy`.
        assert_eq!(rotation_for_aim([0, 0, -1]), (270.0, 0.0));
        assert_eq!(rotation_for_aim([0, 0, 1]), (90.0, 0.0));
        assert_eq!(rotation_for_aim([1, 0, 0]), (180.0, 0.0));
        assert_eq!(rotation_for_aim([-1, 0, 0]), (0.0, 0.0));
    }

    #[test]
    fn a_pool_holds_a_million_and_never_more() {
        let mut pool = Pool::new();
        assert_eq!(pool.receive(160), 160);
        assert_eq!(pool.receive(POOL_CAPACITY), POOL_CAPACITY - 160);
        assert_eq!(pool.mana, POOL_CAPACITY);
        assert!(pool.is_full());
        assert_eq!(pool.room(), 0);
        assert_eq!(pool.receive(160), 0);
        assert_eq!(pool.receive(-60), -60);
        assert_eq!(pool.mana, POOL_CAPACITY - 60);
        assert!(!pool.is_full());
    }

    #[test]
    fn the_flower_reaches_six_blocks() {
        assert!(within_link_range([0, 0, 0]));
        assert!(within_link_range([6, 0, 0]));
        assert!(within_link_range([3, 4, 0]));
        assert!(!within_link_range([6, 1, 0]));
        assert!(!within_link_range([7, 0, 0]));
    }

    #[test]
    fn one_coal_reaches_the_pool_as_seven_bursts_and_a_half() {
        // The loop's arithmetic end to end: 1200 mana from one coal, 160 a burst
        // at 2 blocks, and the pool keeps every whole burst. The run is longer
        // than the coal so the last burst has landed by the end.
        let mut flower = Endoflame::new();
        let mut spreader = Spreader::new();
        let mut pool = Pool::new();
        let mut fuel = FakeFuel::of(&[1600]);
        let target = BurstTarget {
            distance: 2.0,
            full: false,
            pos: None,
        };
        for _ in 0..1400 {
            flower.tick(&mut fuel);
            let moved = flower.empty_into(spreader.mana, SPREADER_CAPACITY);
            spreader.receive(moved);
            let tick = spreader.tick(Some(target));
            if tick.arrived > 0 {
                pool.receive(tick.arrived);
            }
        }
        // The coal made 1197 mana (see `one_coal_is_800_burn_ticks_and_about_1200_mana`):
        // 7 bursts of 160 delivered (1120), the remaining 77 still in the flower
        // and the spreader, nothing in flight.
        assert_eq!(pool.mana, 1120);
        assert!(spreader.can_shoot);
        assert_eq!(spreader.mana + flower.mana, 77);
        assert_eq!(pool.mana + spreader.mana + flower.mana, 1197);
    }
}
