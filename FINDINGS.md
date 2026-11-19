# FINDINGS — Botania 1.20.1-456, the mana loop

What the artefact says, before any Rust. Sources, in the order they were read:

1. The jar `ports/_shared/licence-cache/Botania-1.20.1-456-FORGE.jar` (9,516 entries,
   679 blockstates, 896 items, 2,100 recipes): blockstates, models, textures, loot
   tables, lang, recipes.
2. The upstream repository `VazkiiMods/Botania`, tag `release-1.20.1-456`
   (commit `361ddf3e2142047ff57e3c5c2dc6de617d7a0b55`), read for **behaviour only**
   (the port is a clean-room rewrite; no Java was copied or translated).

## The three blocks, from the jar

| Block | Blockstate | Model | Texture(s) | Loot |
|---|---|---|---|---|
| `botania:endoflame` | one variant, no properties | `shapes/cross` (two crossing quads) | `endoflame.png` | itself |
| `botania:mana_spreader` | one variant, no properties | `shapes/spreader` (a hollow wooden box, 2..14) | `mana_spreader_{back,side,core}.png`, `livingwood_log.png`, `stripped_livingwood_log.png` | itself |
| `botania:mana_pool` | one variant, no properties | `shapes/mana_pool` (a bowl, 0..8 high) | `mana_pool_{bottom,inside,side,top}.png` | itself |

- The **aim of a spreader is not a block state**: the block's state has no `facing`.
  `ManaSpreaderBlock.setPlacedBy` writes `rotationX` / `rotationY` floats into the block
  entity, and the burst's velocity is computed from that pair
  (`ManaBurstEntity.calculateBurstVelocity`).
- `assets/botania/models/item/mana_pool.json` carries an `overrides` entry on the
  `botania:full` item property pointing at `botania:block/mana_pool_full`. Neither the
  property nor the full model is in this slice.
- The **recipes are not carryable into this slice**: `mana_pool` is shaped from
  `botania:livingrock`, and `mana_spreader` from `#botania:petals` + `#botania:livingwood_logs`
  through a `botania:gog_alternation` wrapper (a Botania recipe type). None of those
  ingredients exists in slice 1, and `endoflame` has **no crafting recipe at all** — it is
  made in the Petal Apothecary, which is not in the slice.

## The numbers, from the source

`EndoflameBlockEntity`:

- `getMaxMana()` → **300**.
- `tickFlower()`: decrements `burnTime` first; then, while `burnTime > 0` and
  `ticksExisted % 2 == 0`, `addMana(3)` → **3 mana every second tick** (1.5 a tick).
- Fuel: scans `ItemEntity`s in a **7×7×7** box (`RANGE` 3, `offset(-3)..offset(4)`), takes
  the first whose smelting burn time is positive, that has **no crafting remainder** and is
  **not a mana spreader**, and sets `burnTime = min(32000, smeltingBurnTime) / 2`
  (`FUEL_CAP` 32000). One coal (1600 smelting ticks) → **800 burn ticks**.
- A coal therefore makes about **1200 mana** (800 burn ticks × 3 mana every other tick);
  the exact count is 1197 from an empty flower, because the igniting tick spends an age
  slot without adding. The flower's own buffer is 300, so it must hand mana on.

`GeneratingFlowerBlockEntity`:

- `LINK_RANGE` = **6**: the flower finds the closest collector (a spreader) within six
  blocks and, every tick, hands it everything it holds up to the collector's room
  (`emptyManaIntoCollector`).

`ManaSpreaderBlock.Variant.MANA` (the plain spreader): `burstMana` **160**,
`manaCapacity` **1000**, `preLossTicks` **60**, `lossPerTick` **4**, `motionModifier` 1.

`ManaSpreaderBlockEntity`:

- `tryShootBurst` only fires when `canShootBurst` is true, the spreader holds a whole
  burst, and the receiver is not full; it **subtracts the burst's mana from the spreader**
  and sets `canShootBurst` false.
- The spreader cannot fire again until its burst is gone (`setCanShoot(true)` from the
  burst's removal / pingback), so its cadence is the burst's flight time.
- The receiver is found by simulating a burst (`checkForReceiver`) and taking the first
  mana receiver it hits.

`ManaBurstEntity`:

- `calculateBurstVelocity`: `f = 0.4F`, and each component is scaled by `f / 2` →
  **0.2 blocks a tick** (five ticks a block).
- `tick()`: `ticksExisted++`, then if `ticksExisted >= minManaLoss` it loses
  `manaLossPerTick` (4) mana that tick. So a flight of `T ≥ 60` ticks loses
  `(T - 60 + 1) × 4` mana; a burst 2 blocks away (10 ticks) loses nothing, one 13 blocks
  away (65 ticks) arrives with `160 - 6×4 = 136`.

`ManaPoolBlockEntity`:

- `MAX_MANA` **1,000,000** (the diluted variant 10,000; not in this slice).
- `receiveMana` clamps to the capacity and accepts a negative amount as a drain.
- The pool's collision box is `box(0, 0, 0, 16, 8, 16)`; its outline is that box minus a
  cut-out above the rim.

## What this slice does not carry

The apothecary, runes, the lexicon, functional flowers, the other generating flowers,
lenses, the spreader's padding/scaffolding, the pool's infusion recipes and variants, the
wand/tablet readout, and mana on items. See `notes.md` for the gap list.
