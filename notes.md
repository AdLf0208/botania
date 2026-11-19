# botania — notes

**Class:** `content` · **Tier:** `deep` · **Role:** core · **Port:** external (Botania is not
in ATM10; its newest release is 1.20.1).
**Approach from `horizon/docs/modpacks.md`:** `deep` — mana as a fluid-like quantity with
generating flowers, spreaders and pools. This slice is **one generation → transfer →
storage loop**, with the jar's own numbers; the rest of the mod is a gap list below.
**Format:** the flat SDK package (**mod API 0.2**, one `botania.hmod`) since 2026-11-18; the
`.hnmod` container it was built in is retired (Horizon's mod-API merge #16941). Version
**0.2.0**.

## Licence

- **The Botania License** (<https://botaniamod.net/license.html>; verbatim copy at
  `ports/_shared/licence-texts/BOTANIA-LICENSE.txt`, shipped as [`LICENSE`](LICENSE)).
- **Rank 1** (copyleft-style, publishable under the same licence with the complete
  corresponding source). The jar's `mods.toml` declares `license="Botania License"`.
- Obligations, all satisfied in [`LICENSE-NOTICE.md`](LICENSE-NOTICE.md) **and inside the
  delivered `botania.hmod`**:
  1. **Attribution** — **Vazkii** is named as Botania's creator, and so are the jar's nine
     authors (**Vazkii, wiiv, williewillus, dylan4ever, Hubry, Alwinfy, artemisSystem,
     Falkory220, Wormbo**); the notice states that **this project is an alteration of
     Botania, not Botania itself**, and is not endorsed by its authors.
  2. **Copyleft** — the package is open source (its complete source travels in
     `src/lib.rs`, `src/runtime.rs`, `Cargo.toml`, `horizon-mod.toml`) and its `LICENSE`
     **is the Botania License itself**, so a derivative carries the clause forward.
  3. **Non-monetary** — nothing is charged for, no ad-wall, no URL shortener, no paywall
     (stated in the notice).
  4. **External clause** — no file read carries another licence marker, and **no Botania
     API code was used** (clean-room rewrite against Horizon's own hooks), which the notice
     states explicitly.

## Slice boundary (what is in, and what is deliberately out)

In: `botania:endoflame`, `botania:mana_spreader`, `botania:mana_pool` — blocks, items,
models, textures, lang, loot tables; the flower's fuel burn and mana trickle, the
flower→spreader link, the spreader's burst and its loss over flight, the pool's capacity and
its readout; a `/botania info|mana|aim` harness surface.

Out (gaps): the Petal Apothecary, runes, livingrock/livingwood/petals (so **no crafting
recipes**: the jar's recipes name ingredients this slice does not have, and `endoflame` has
no crafting recipe upstream at all — the apothecary makes it); the Lexica Botania and its
Patchouli pages; functional flowers; the other generating flowers; lenses; spreader
padding/scaffolding; mana on items (tablets, bands); the wand/tablet HUD; the pool's diluted,
creative and fabulous variants and its infusion recipes; comparator output for the pool;
particles and sounds; the spreader's client-side render of the moving core; block colours
(the spreader's faces carry `tintindex: 0`).

## Gaps (what a user notices)

1. **No crafting recipes for the three blocks.** The jar's recipes name
   `botania:livingrock`, `#botania:petals` and `#botania:livingwood_logs` (and the spreader's
   recipe is the `botania:gog_alternation` type), and `endoflame` has no crafting recipe at
   all — the Petal Apothecary makes it. Carrying them would be an ingredient that can never
   match, so slice 1 ships none and the blocks come from `/give` or creative.
2. **No GUI.** The pool has no screen upstream either; the readout is `/botania info` (and
   the action bar for a LAN guest, deviation 11).
3. **A burst is not drawn, and it is not an entity.** No travelling burst entity or
   particles (deviation 1), and the spreader's moving core is not rendered (deviation 9).
   **Migration gap:** the SDK has entity-type registration, but a mana-burst entity is not
   implemented — the burst's flight stays block-entity state (`ticksLeft`, `arrival`), so
   there is no object in the world for a client to see or for a third-party mod to
   intercept. A faithful burst entity is a later slice.
4. **No comparator output** for the pool (upstream's `getComparatorOutput`).
5. **No sounds** (`botania:endoflame`'s ignite, the spreader's fire).
6. **No mana network beyond the link**: a flower links to the **nearest** spreader within 6
   blocks (upstream's network does the same), but a spreader does not pull mana out of an
   **adjacent** pool, which upstream's `commonTick` does (`pool.receiveMana(-n)` into the
   spreader). Nothing in this loop needs it.
7. **Only the default spreader and pool variants**: elven/gaia/redstone spreaders, diluted,
   creative and fabulous pools, lenses, padding and scaffolding are out.
8. **Only the endoflame** of the generating flowers.
9. **Nothing else of the mod**: apothecary, runes, livingrock/livingwood/petals, the Lexica,
   functional flowers, mana on items, the wand/tablet HUD, pool infusion.

## Upstream files read

- Jar: `ports/_shared/licence-cache/Botania-1.20.1-456-FORGE.jar` — blockstates, models,
  shapes, textures, loot tables, lang, recipes for the three blocks (inventory in
  [`FINDINGS.md`](FINDINGS.md), carried-file list in [`CONTENT.md`](CONTENT.md)).
- Repository `VazkiiMods/Botania`, tag `release-1.20.1-456`
  (commit `361ddf3e2142047ff57e3c5c2dc6de617d7a0b55`), read for behaviour:
  `common/block/flower/generating/EndoflameBlockEntity.java`,
  `api/block_entity/GeneratingFlowerBlockEntity.java`,
  `common/block/block_entity/mana/ManaSpreaderBlockEntity.java`,
  `common/block/mana/ManaSpreaderBlock.java`,
  `common/entity/ManaBurstEntity.java`,
  `common/block/block_entity/mana/ManaPoolBlockEntity.java`,
  `common/block/mana/ManaPoolBlock.java`,
  `common/block/BotaniaFlowerBlocks.java`, `common/block/BotaniaBlocks.java`.
- The clone was deleted after the numbers were recorded; the jar stays as the licence and
  provenance evidence.

## The hooks (SDK mod API 0.2 at `0b14f8a`)

The three blocks are **block entities**, so their moments are **Parts** on the types this
package declares, not global hooks. The two remaining global hooks are the pool's click and
the harness commands; the spreader's aim is the spreader item's Part.

| Old `.hnmod` helper | New SDK method | What it does here | Status |
|---|---|---|---|
| `block_entity.rs` (hook) | `Registrar::block_entity_type(...).blocks(...).build()` ×3 in `Hooks::register` | declares one type per block; a declared type starts with an empty compound (the old `Some((id, Compound::new()))`) | kept |
| `block_entity_ticks.rs` (hook) | `BlockEntityPart::block_entity_ticks` on the flower and spreader Parts | sets `ticks = true` | kept |
| `block_entity_tick.rs` (hook) | `BlockEntityPart::block_entity_tick` on the flower and spreader Parts | burn + push (flower), fire + land (spreader) | kept |
| `place_block.rs` (hook) | `ItemPart::on_block_placed_by` on `botania:mana_spreader` | a placed spreader keeps the aim the placer looked along (`entity_rotation` → `rotation_for_aim`) | kept |
| `right_click_block.rs` (hook) | `Hooks::right_click_block` | a right click on a pool answers its mana on the action bar and claims the click | kept |
| `register_commands.rs` (hook) | `Hooks::register_commands` | `/botania info|mana|aim`, `mana`/`aim` at level 2 | kept |
| `perform_command.rs` (hook) | `Hooks::run_mod_command` | the same three commands, parsed arguments instead of a raw line | kept |
| `embedded_content.rs` (raw `content_root` injection) | — (the package format) | the content no longer needs the AE2 write-out-and-append path: `assets/` and `data/` are read from the same `.hmod` | format-provided |

Host calls (old → new):

| Old helper (`native_*` / `res.`) | New SDK call |
|---|---|
| `native_item_entities` + `native_set_item_entity` | `LevelRef::entities_in` → `Host::item_entity` → `ItemEntityRef::stack`; take with `ItemEntityRef::set_stack` (one less) or `EntityRef::remove(Discarded)` |
| `native_with_mod_block_entity` (read/write) | `Host::block_entity` → `BlockEntityRef::data`; write with `BlockEntityRef::merge_data` (loads and `setChanged`) |
| `native_block` | `LevelRef::block(host, pos)` + `BlockKey::id(host)` (hot loops compare the registered `BlockKey`) |
| `res.providers().burn_time(...)` | `Host::item_burn_time(ItemKey)` (`cooking_fuel` resolved by the engine) |
| `res.items().get(..).remainder` | `Host::item_crafting_remainder(ItemStack)` |
| `native_feedback` | `Commands::feedback(Text)` |
| `native_action_bar` | `PlayerRef::action_bar(host, Text)` |
| `native_dimension` | `LevelRef`; the block-entity tick finds its level with `LevelRef::in_place`, a command with `DimensionKey::id(host)` → `Host::level` |
| `world.modified.insert(..)` (`setChanged`) | `BlockEntityRef::merge_data` marks it; the tick's default `BlockEntityTickReply::Changed` saves its own payload |

## Deviations (deliberate, not defects)

1. **The burst is simulated, not an entity.** Horizon has no mana-burst entity for a mod at
   this pin (and the SDK's entity-type registration is not used yet), so the spreader keeps
   the burst's flight as state (`ticksLeft`, `arrival`): the mana leaves the spreader when it
   fires and reaches the pool `travel_ticks(distance)` later, with the same loss curve. A
   player sees no travelling burst.
2. **The receiver is the first pool along the aim**, up to 16 blocks, instead of a simulated
   burst that collides with the pool's shape. A pool hidden behind another block is still
   found by the scan; upstream's burst would hit the block in between.
3. **The aim is the axis the placer looked along.** Upstream stores `rotationX`/`rotationY`
   the same way and reads them back through `calculateBurstVelocity`; the port's
   `rotation_for_aim`/`aim_direction` are that mapping. The old hook was `place_block`
   (listen, after the write); the SDK's counterpart is the item Part's
   `on_block_placed_by` (after the final state and block-entity data are applied).
4. **`age` is saved** (upstream's `ticksExisted` is not, so a reload restarts the `% 2` phase
   and one mana tick can be gained or lost). Saving it makes the port's output reproducible.
5. **The pool's readout is a right click with an empty hand.** Upstream reads a pool with a
   wand or a mana tablet, which this slice does not have; the port answers with the numbers
   the wand HUD shows. The click is claimed.
6. **`/botania info|mana|aim` exist only for the harness** (precedent: `/powah info|set`).
7. **No comparator output** for the pool (upstream has one): a mod block's own comparator
   output needs a surface the slice does not need yet. Recorded as a gap.
8. **The pool's outline is its collision box** (the lower half); upstream's outline is that
   box minus a cut-out above the rim, which is not one box.
9. **`render_shape` is the model** for the spreader (upstream draws a moving core in its
   block entity): the slice has no client renderer.
10. **The flower's `burnTime` is capped at 32000 smelting ticks** exactly as upstream, and a
    fuel whose halved burn is 0 (a 1-tick fuel) ignites for zero ticks — upstream's own
    arithmetic, kept.
11. **The click readout reaches LAN guests, not the single player's window.** The SDK's
    `PlayerRef::action_bar` is documented as reaching the acting player too, so the old
    limitation may be gone; it is not re-measured without a game run (no e2e here). The
    owner's readout is `/botania info` either way.
12. **A dropped stack of one is removed with `EntityRef::remove(Discarded)`** instead of the
    old `setItem(air, 0)`: the SDK refuses an empty stack write, and vanilla discards the
    entity on its next tick anyway. A stack of more than one is written back one smaller.

## Upstream defects fixed

**None found.** The artefact is internally consistent for this slice: the three blockstates
declare no properties (the spreader's aim really is block-entity state, not a state
property), the loot tables name the blocks they belong to, and the numbers agree across
`EndoflameBlockEntity`, `GeneratingFlowerBlockEntity`, `ManaSpreaderBlock.Variant.MANA`,
`ManaBurstEntity` and `ManaPoolBlockEntity`. The one *carry* problem — the jar's recipes name
ingredients this slice does not have — is a scope boundary, not a defect: those recipes are
left out rather than carried as ingredients that can never match (see the gap list).

## Migration to the SDK format (2026-11-18)

Migrated from the retired `.hnmod` container (Horizon's mod-API merge #16941; PR #17992
cleans up the old files) to the flat SDK package at nightly
`0b14f8acfd97fa2c750e42795249dcc981178ca4` / mixins
`e59da9094c0c2fdda6c627fe9eff22dec6a906f0`. `runtime.rs` is **byte-identical** to the old
imported module (`sha256 8158dbae…7dda4abe`), with its 20 tests unchanged and green.

### File map

| Old `.hnmod` layout | New SDK package |
|---|---|
| `horizon-mixin.toml` (`[mod]`, `[compatibility]`, 2 `[[module]]`, 8 `[[injection]]`) | `horizon-mod.toml` (id/name/version/api/side/capabilities + `[native]` game/horizon/mixins) |
| `Cargo.toml` (lib = `mixins/botania/runtime.rs`) | `Cargo.toml` (`[lib] path = "src/lib.rs"`, depends on `horizon-mod-api` from `.horizon/sdk/`) |
| `mixins/botania/runtime.rs` (module + crate lib) | `src/runtime.rs` (byte-identical; `pub mod runtime;` at the crate root) |
| `mixins/botania/support.rs` (`botania_state` module) | `src/lib.rs` (one `impl Hooks` + the three Parts + the item Part) |
| 8 per-hook helper `.rs` files | methods of `impl Hooks` / the Parts; no per-hook files |
| `content/` (`assets/`, `data/`, its own `horizon-mod.toml`) | package root: `assets/`, `data/` (the content half of the same `.hmod`) |
| `mixins/botania/data/botania-content.hmod` + `embedded_content.rs` (AE2 write-out) | gone: content and code are one package |
| `horizon-native.json`, `compatibility.json`, `rust-toolchain.toml` | gone: `[native]` in `horizon-mod.toml` is the pin |
| `botania.hnmod` (15 entries, 29,139 B) | `botania.hmod` (44 entries, 43,264 B) |

### Manifest mapping

- identity: `id = "botania"`, `name`, `version = "0.2.0"` (the port's version, not
  upstream's), `license = "LicenseRef-Botania-License"` (the registry string is
  `Botania License`), `authors` = the jar's nine plus the port's author.
- `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]` — the package really
  declares blocks and block entity types.
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and `horizon-native.json`.
- The old `[[module]]` split (runtime → `crate::game::botania`, support →
  `crate::game::botania_state`) has no counterpart: the mod is its own crate, so the pure
  logic is `src/runtime.rs` and the glue is `src/lib.rs`.

### Build and verify

All in a `ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` (external ports are not
synced to the remote runner):

```
rm -f Cargo.lock                         # BEFORE EVERY BUILD — see below
cargo test --offline                     # 20 passed
cargo fmt --all --check                  # clean
cargo clippy --all-targets --offline -- -D warnings   # clean
rm -f Cargo.lock                         # cargo test wrote it again
cargo horizon-mod build                  # target/horizon-mod/botania.hmod
rm -f Cargo.lock
cargo horizon-mod build                  # byte-identical to the first
cargo horizon-mod check botania.hmod     # exit 0
cargo horizon-mod fits botania.hmod --build 0b14f8a   # exit 0
```

**The lock must be removed before each `cargo horizon-mod build`.** `cargo test`,
`cargo clippy` and `cargo horizon-mod check` all write a `Cargo.lock`, and the packer
takes the directory's file list, so a lock left beside the sources is packed into the
`.hmod` — which the registry gate refuses (`no packed Cargo.lock`). The port has no
registry dependency of its own (the only entry is the SDK path crate), so the lock is
not needed for the build; removing it changes nothing else. Measured on
`refinedtypes`; re-measured here: the two builds with the lock removed are byte-identical.

`check` output:

```
botania 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094c0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  40928 bytes, SHA-256 55a86bcfdab8b04a188664e42302db4e29c2a0bfe3882bb616eeef2e839b73b5, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`fits` output:

```
Botania:
0b14f8a  made for
```

The package also lists `license = "LicenseRef-Botania-License"`, and `LICENSE` +
`LICENSE-NOTICE.md` are the first two files after `Cargo.toml` in the archive.

### Traps a huge port's migration must know

1. **Read the SDK, not the old docs.** `level_block`, `level_in_place`,
   `level_entities_in` and `level_game_time` are **`LevelRef` handle methods**
   (`level.block(host,pos)`, `level.in_place(host)`, …), while `item_burn_time` and
   `item_crafting_remainder` are **`Host` methods**. `host.*` only has the latter.
2. **`BlockEntityPart: BlockPart`** — the `#[horizon_part]` macro derives the subscription
   table but does **not** implement the supertrait, so a block-entity Part needs an
   (empty) `#[horizon_part] impl BlockPart for P {}` beside its
   `impl BlockEntityPart for P`.
3. **A block-entity tick is not told its level.** Find it with
   `LevelRef::in_place(host)` over `host.levels().list_levels()`; a command uses
   `DimensionKey::id(host) → Host::level`. Do not guess "the overworld first".
4. **`DimensionKey` and `LevelKey` are different types.** `invocation.dimension` is a
   `DimensionKey` (`DimensionKey::id(host)`); `click.dimension` is a `LevelDimension`
   (`Dimension::key().id()`), which is free.
5. **`Compound` is a `BTreeMap<JavaString, Tag>`**; `get` needs a `&JavaString`, so a
   `&str` key is encoded to UTF-16 first (the old `horizon_world` `Compound` had the same
   shape). `Tag::Float`/`Double` hold **bits**.
6. **`rotation` returns `[f32; 2]`, not a tuple**, and the spreader's aim is written by the
   **item** Part (`on_block_placed_by`), because `entity_place` fires before the block
   entity exists.
7. **An `ItemEntity` accepts no empty stack**: shrink with `set_stack(count - 1)` and remove
   the entity (`RemovalReason::Discarded`) for the last item.
8. **Builds with `cargo horizon-mod` are their own thing.** The tool fetches `.horizon/sdk/`
   on any command; `build` writes `target/horizon-mod/<id>.hmod`, which must be **copied to
   the port root** (the deliverable, never `target/`). `check`/`fits` take the package file.
9. **The e2e drives the launcher build.** `tools/e2e.sh` runs a game the launcher built
   from this folder (`ports/tools/launcher-game.sh build botania --port external/botania`),
   with `GAME`/`CTL` from `launcher-game.sh env botania`. The old script drove a game built
   by `tools/horizon.py` with the retired bundle compiled in and could not run the SDK
   package; it is replaced, not supplemented. `GAME=$CONTROL_GAME` is the differential
   (the same Nightly without the package) and `SELFTEST=1` inverts the burst assertion.

## Registry entry (for the orchestrator; this job does not submit)

- id `botania`, version **`0.2.0`**, kind **native-mod**, file **`botania.hmod`**,
  **sha256 `55a86bcfdab8b04a188664e42302db4e29c2a0bfe3882bb616eeef2e839b73b5`** (40,928 B, 43 entries, no `Cargo.lock`),
  game `26.3`, horizon `0b14f8acfd97fa2c750e42795249dcc981178ca4`, mixins
  `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`.
- **licence: `Botania License`** (rank 1, copyleft-style: publishable under the same licence
  with the complete corresponding source — `Cargo.toml`'s SPDX form is
  `LicenseRef-Botania-License`, the registry string is `Botania License`).
- source: this package and its public repository (`LICENSE` = the Botania License verbatim,
  `LICENSE-NOTICE.md` = the attribution, the alteration statement and the four clauses).
- Former submission **#913** was closed in the format purge; this version supersedes it.

## Changelog for the resubmission

> Supersedes #913, closed because Horizon retired `.hnmod` (#17992). Changes: flat SDK
> package with `api = "0.2"` and `[native]` pinned to `0b14f8a`; hooks rewritten as
> `impl mod_api::Hooks` methods against typed hosts/events (7 hooks mapped, 1 recorded as a
> gap); `runtime.rs` and its tests unchanged. The package no longer carries a `Cargo.lock`
> (the registry gate refuses a packed one; the port has no registry dependency of its own),
> and it now ships an in-game leg (`PASS` on the launcher build, `FAIL` on the same Nightly
> without the package).

## Verification

Skills: superpowers/using-superpowers,verification-before-completion,systematic-debugging; BeautifulRust/rust-best-practices

Re-run 2026-11-20 for the packaging gate (the package was rebuilt without a packed
`Cargo.lock`; no source changed, so `runtime.rs` and its tests are the same bytes). The
2026-11-18 port job that produced that source used superpowers
using-superpowers/brainstorming/writing-plans/test-driven-development/systematic-debugging/
verification-before-completion and BeautifulRust rust-best-practices/rust-port.

- `cargo test --offline`: **20 passed**, 0 failed (`evidence/botania/sdk-test.log`).
- `cargo fmt --all --check`: clean (`sdk-fmt.log`).
- `cargo clippy --all-targets --offline -- -D warnings`: exit 0, no warnings
  (`sdk-clippy.log`).
- `rm -f Cargo.lock` then `cargo horizon-mod build`, twice: **40,928 B**, **43 entries**,
  both builds byte-identical (`cmp`), no `Cargo.lock` in the archive (`sdk-package.log`).
- `cargo horizon-mod check botania.hmod`: **exit 0**, native-1 at
  `0b14f8ac…/e59da909…` for 26.3, SHA-256 `55a86bcf…73b5` (`sdk-check.log`).
- `cargo horizon-mod fits botania.hmod --build 0b14f8a`: **exit 0**, "0b14f8a made for"
  (`sdk-fits.log`).
- `python3 ports/tools/verify-sdk-package.py botania`: **1/1**, `no packed Cargo.lock ok`,
  `runtime byte-identical 8158dbaee0ce`, 20 tests, 0 clippy warnings.
- `src/runtime.rs` sha256 `8158dbaee0ce3b7628d8799d79d0b45d7dad1f796f081c796c47fee72dda4abe`
  — identical to the retired `mixins/botania/runtime.rs`.
- The archive carries `LICENSE` and `LICENSE-NOTICE.md` (`sdk-package.log`).
- **In-game leg (launcher `0.2.78`, Nightly `0b14f8a`)**: `tools/e2e.sh` on the built game
  → `PASS` + `E2E EXIT:0` (`ports/_shared/evidence/botania-e2e/e2e-launcher.log`); the
  same scenario on the same Nightly **without** the package → `FAIL: botania:endoflame is
  not a known item` + `EXIT:1` (`e2e-launcher-control.log`); `SELFTEST=1` →
  `FAIL: SELFTEST: the near pool holds 160, so the burst assertion is alive` + `EXIT:1`
  (`e2e-launcher-selftest.log`). `check-e2e-logs.py botania-e2e --strict` → 3/3 ok.
  Assertions: the three items and blocks register with capacities 300/1000/1,000,000; an
  unfuelled flower makes nothing; a dirt is not fuel; one coal ignites for 771-778 burn
  ticks; the pool grows in whole 160-mana bursts while the flower stays empty; a 2-block
  burst lands exactly 160; a 13-block burst arrives with 136; a flower with no spreader in
  range fills to 300 and delivers nothing; a placed spreader keeps the placer's axis; a
  pool click answers with its readout on the action bar and is claimed.
- `python3 ports/tools/check-skills.py --notes external/botania` — run by the orchestrator
  before recording.
