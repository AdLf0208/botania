# Botania for Horizon — slice 1: endoflame → mana spreader → mana pool

A Rust port of [Botania](https://github.com/VazkiiMods/Botania) (1.20.1-456) to
[Horizon](https://github.com/horizon-rust/horizon), a rewrite of Minecraft: Java Edition in
Rust. Botania was created by **Vazkii**; the authors credited by the jar are Vazkii, wiiv,
williewillus, dylan4ever, Hubry, Alwinfy, artemisSystem, Falkory220 and Wormbo. **This
project is an alteration of Botania, not Botania itself**, and is not endorsed by its
authors. It is distributed under the **Botania License** — see [`LICENSE`](LICENSE) and
[`LICENSE-NOTICE.md`](LICENSE-NOTICE.md). It is a clean-room rewrite: no Java was copied,
and no Botania API code was used.

## What this slice does

One complete mana loop, with the jar's own numbers:

| Block | Behaviour |
|---|---|
| **Endoflame** (`botania:endoflame`) | Burns a **dropped** fuel item in a 7×7×7 box (no crafting remainder, not a spreader), for `min(32000, smelting burn time) / 2` ticks, and makes **3 mana every second tick** into a 300-mana buffer. It hands everything it holds to the nearest **mana spreader** within 6 blocks, every tick. |
| **Mana Spreader** (`botania:mana_spreader`) | Holds 1,000 mana. Fires a **160-mana burst** along the axis it was placed facing, once no burst of its own is in flight, at the first mana pool in its path. The burst flies at **0.2 blocks a tick** and loses **4 mana a tick from flight tick 60** — so a pool 2 blocks away receives all 160 and one 13 blocks away receives 136. |
| **Mana Pool** (`botania:mana_pool`) | Holds **1,000,000** mana, takes bursts up to its capacity, and answers a right click with its mana. |

The three blocks are **block entities** (Parts on their own declared types); the spreader's
aim is written by the spreader item's placement Part. The behaviour is the pure
[`src/runtime.rs`](src/runtime.rs) arithmetic plus the glue in [`src/lib.rs`](src/lib.rs).

## What it does not do (yet)

The Petal Apothecary, runes, livingrock/livingwood/petals, the Lexica Botania, functional
flowers, the other generating flowers, lenses, spreader padding/scaffolding, mana on items,
the wand/tablet HUD, pool variants and infusion. No crafting recipes yet: the ingredients
the jar's recipes name are not in this slice, so the three blocks are obtained with `/give`
(see [`notes.md`](notes.md)).

## Install

One file: **`botania.hmod`** — the SDK package (mod API 0.2). It carries both the content
(`assets/`, `data/`) and the code (`src/`, compiled by the launcher against the exact
`[native]` pins in `horizon-mod.toml`), so installing it gives the three blocks, their
models and textures, and their behaviour.

## Commands (harness controls; upstream has none)

```
/botania info <x> <y> <z>       the mana, capacity, burn/aim of the block there
/botania mana <x> <y> <z> <n>   set its mana (operator)
/botania aim <x> <y> <z> <axis> point a spreader north|south|east|west|up|down
```

## Build and test

```
cargo test --offline                       # the pure runtime (20 tests)
cargo horizon-mod build                    # botania.hmod in target/horizon-mod/
cargo horizon-mod check botania.hmod       # package check
cargo horizon-mod fits botania.hmod        # does it still fit another nightly?
```

The launcher builds the code against the pinned game (`horizon-launcher native-build .`).
See [`notes.md`](notes.md) for the hook map, the deviations and the gaps, and
[`FINDINGS.md`](FINDINGS.md) for where every number came from.
