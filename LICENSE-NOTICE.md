# Licence notice — Botania for Horizon

This project is a **port of Botania** to [Horizon](https://github.com/horizon-rust/horizon),
a Rust rewrite of Minecraft: Java Edition. It is a **clean-room Rust rewrite**: no
Java source was copied or translated, and **no Botania API code** (`src/main/java/vazkii/botania/api`)
was used.

## What this is ported from

- **Botania**, created by **Vazkii**, version **1.20.1-456** for Minecraft Java 1.20.1 (Forge).
- Upstream: <https://github.com/VazkiiMods/Botania>, tag `release-1.20.1-456`.
- The jar this port was written against: `Botania-1.20.1-456-FORGE.jar`
  (9,516 entries; SHA-256 `3cf540357c9cb5e4…`, recorded in [`SOURCE.md`](SOURCE.md)).
- The jar's own `authors` list, credited here in full: **Vazkii, wiiv, williewillus,
  dylan4ever, Hubry, Alwinfy, artemisSystem, Falkory220, Wormbo**.

## The licence, and the four obligations it puts on this port

Botania is distributed under **the Botania License**
(<https://botaniamod.net/license.html>). The complete text travels with this package as
[`LICENSE`](LICENSE), copied verbatim from
`ports/_shared/licence-texts/BOTANIA-LICENSE.txt` in the campaign tree. The jar's own
`mods.toml` declares `license="Botania License"`.

The licence grants the right to **Use, Share and Adapt** the mod, so a port is permitted.
This port is an **adaptation**, and it satisfies every clause that applies to one:

1. **Attribution.** Vazkii is credited as the creator of Botania here, in
   [`README.md`](README.md) and in `horizon-mod.toml`'s `authors`; the port's own files
   say in their headers that the behaviour is read from Botania and that the Rust is a
   rewrite. **This project is an alteration of Botania, not Botania itself**, and it is
   not endorsed by Vazkii or the Botania authors. A link back is at
   <https://botaniamod.net/>.
2. **Copyleft.** This project is **open source**: its complete corresponding source
   travels in this package (`src/lib.rs`, `src/runtime.rs`, `Cargo.toml`,
   `horizon-mod.toml`) and in its public repository, visible to everyone, and may be
   redistributed and modified. The package's licence is the Botania License itself — the
   same text, with the same clause — so a derivative of this port carries the same
   obligation forward.
3. **Non-monetary.** Nothing here is charged for, and nothing is put behind a paywall or
   an ad-wall: no payment, donation, advertisement, URL shortener or gated download is
   part of this distribution, now or later.
4. **External clause.** Files read from Botania that are marked under another licence
   would have been obeyed; none of the files this slice uses (blockstates, models,
   textures, loot tables, lang names, and the Java behaviour read for analysis) carries an
   alternative licence marker. **No Botania API code was copied**: the port is a rewrite
   against Horizon's own named hooks, so the API clause ("if Botania API code is used, it
   must be included verbatim") does not apply — nothing was used.

## What travels with the package

- [`LICENSE`](LICENSE) — the Botania License, verbatim.
- This notice, shipped **inside `botania.hmod`** as `LICENSE-NOTICE.md` beside `LICENSE`.
- [`SOURCE.md`](SOURCE.md) — the jar, its SHA-256 and the upstream tag every carried asset
  came from; [`CONTENT.md`](CONTENT.md) lists each carried file and each 26.3 re-spelling.
