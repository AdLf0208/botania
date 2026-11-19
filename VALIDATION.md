# VALIDATION — botania 0.2.0 (the SDK package)

Migrated 2026-11-18 to the flat SDK package at Horizon nightly
`0b14f8acfd97fa2c750e42795249dcc981178ca4` + horizon-mixins
`e59da9094c0c2fdda6c627fe9eff22dec6a906f0`. Every command below ran inside a
`ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` (the remote runner does not carry
`external/` trees). Logs: `ports/_shared/evidence/botania/sdk-*.log`.

## 1. The pure runtime, standalone

```
$ cargo test --offline
    test result: ok. 20 passed; 0 failed; 0 ignored
$ cargo fmt --all --check                       # clean
$ cargo clippy --all-targets --offline -- -D warnings
    Finished `dev` profile ... (no warnings)
```

`src/runtime.rs` is byte-identical to the retired `mixins/botania/runtime.rs`:
sha256 `8158dbaee0ce3b7628d8799d79d0b45d7dad1f796f081c796c47fee72dda4abe`.

## 2. The package

```
$ rm -f Cargo.lock
$ cargo horizon-mod build
SDK 0b14f8a (build 0b14f8acfd97fa2c750e42795249dcc981178ca4) in ./.horizon/sdk
Built ./target/horizon-mod/botania.hmod        # copied to the port root

$ rm -f Cargo.lock && cargo horizon-mod build  # second build
$ cmp <first> <second>                         # identical

$ cargo horizon-mod check botania.hmod
botania 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094c0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  40928 bytes, SHA-256 55a86bcfdab8b04a188664e42302db4e29c2a0bfe3882bb616eeef2e839b73b5, unsigned
CHECK_EXIT:0

$ cargo horizon-mod fits botania.hmod --build 0b14f8a
Botania:
0b14f8a  made for
FITS_EXIT:0
```

Delivered package: **`botania.hmod`**, **40,928 B**, **43 entries**, SHA-256
`55a86bcfdab8b04a188664e42302db4e29c2a0bfe3882bb616eeef2e839b73b5`. It carries
`horizon-mod.toml`, `Cargo.toml`, `src/{lib,runtime}.rs`, `assets/botania/**`,
`data/botania/**`, `README.md`, and **`LICENSE` + `LICENSE-NOTICE.md` inside the archive**.

**`rm -f Cargo.lock` before every build.** `cargo test`, `cargo clippy` and
`cargo horizon-mod check` all write a lock, and the packer takes the directory's file
list, so one left beside the sources is packed — and `verify-sdk-package.py` fails on
`no packed Cargo.lock`. The port has no registry dependency of its own, so removing it
changes nothing else: the two builds are byte-identical with it gone.

`python3 ports/tools/verify-sdk-package.py botania` → **1/1** (`no packed Cargo.lock ok`,
`runtime byte-identical 8158dbaee0ce`, 20 tests, 0 clippy warnings, check/fits green).

## 3. In-game verification

The launcher (`0.2.78`) builds a game from this folder at Nightly `0b14f8a` and the
package's content travels with it:

```
ports/tools/launcher-game.sh build botania --port external/botania --rebuild
eval "$(ports/tools/launcher-game.sh env botania)"
build-slot.sh run --job e2e-botania --port external/botania --step e2e -- \
    bash external/botania/tools/e2e.sh
```

| Run | Result | Log |
|---|---|---|
| modded game | `PASS` + `E2E EXIT:0` | `ports/_shared/evidence/botania-e2e/e2e-launcher.log` |
| same Nightly, no package (`GAME=$CONTROL_GAME`) | `FAIL: botania:endoflame is not a known item` + `EXIT:1` | `e2e-launcher-control.log` |
| `SELFTEST=1` (inverted burst assertion) | `FAIL: SELFTEST: the near pool holds 160, so the burst assertion is alive` + `EXIT:1` | `e2e-launcher-selftest.log` |

`check-e2e-logs.py botania-e2e --strict` → `logs=3 ok=3 missing-exit=0 bad-exit=0`.

The scenario: the three items and blocks register with the jar's capacities (300 / 1000 /
1,000,000); an unfuelled flower makes nothing; a dropped dirt is not fuel; one coal
ignites for 771-778 burn ticks and the pool grows in whole 160-mana bursts while the
flower itself stays empty; a 2-block burst lands exactly 160; a 13-block burst arrives
with 136 (the jar's loss curve); a flower with no spreader within 6 blocks fills to 300
and delivers nothing; a spreader placed with the item keeps the axis the placer looked
along; and a pool click answers with its readout on the action bar and is claimed.

The 0.1.0 evidence in `ports/_shared/evidence/botania/` stays as history for the same
behaviour under the retired `.hnmod` build path.

## 4. Campaign gates

```
$ python3 ports/tools/check-skills.py --notes external/botania --strict   # PASS
$ python3 ports/tools/check-e2e-logs.py botania-e2e --strict              # 3/3 ok
$ python3 ports/tools/verify-sdk-package.py botania                       # 1/1
```

`upstream_ready.py botania` still needs the orchestrator's ledger step.

## 5. Reproduce

```
cd external/botania
rm -f Cargo.lock
cargo test --offline
cargo fmt --all --check
cargo clippy --all-targets --offline -- -D warnings
rm -f Cargo.lock && cargo horizon-mod build && cp target/horizon-mod/botania.hmod .
rm -f Cargo.lock
cargo horizon-mod check botania.hmod
cargo horizon-mod fits botania.hmod --build 0b14f8a
```

The old `content/`, `mixins/`, `botania.hnmod` and their tools were retired with the format;
their final state is recorded in `notes.md §Migration to the SDK format`.
