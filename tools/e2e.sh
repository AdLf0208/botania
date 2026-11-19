#!/usr/bin/env bash
# In-game check for Botania slice 1: one generation -> transfer -> storage loop.
#
# The mod's code is compiled into a game the launcher builds from this folder, and
# the package's own `assets/`+`data/` are installed beside it, so GAME must be
# that build (see `ports/tools/launcher-game.sh build botania --port external/botania`).
# `GAME=<an unmodded launcher build>` runs the same scenario as the control: the
# first assertion (the mod's items register) must fail, which is what makes the
# positives non-vacuous.
#
# One headless creative launch, on a stone platform in the Overworld:
#   A. the three block items exist and the three blocks place with the jar's
#      capacities: endoflame 300, mana spreader 1000, mana pool 1,000,000;
#   B. an unfuelled flower makes no mana and the pool stays empty;
#   C. a dropped dirt is not fuel: it stays on the ground and nothing burns;
#   D. one coal ignites for 800 burn ticks (771-778 left after the first reads), the
#      flower fills, hands its mana to the spreader, and the pool grows in whole
#      160-mana bursts while the flower itself never holds more than 300;
#   E. a burst that lands 2 blocks away hands the pool exactly 160;
#   F. a burst that lands 13 blocks away arrives with 136 (160 - 6 x 4 after the
#      60th flight tick), the jar's loss curve;
#   G. a flower with a pool but no spreader in range burns its coal and delivers
#      nothing, so the pool's mana is the spreader's;
#   H. a spreader placed with the item keeps the axis the placer looked along
#      (`on_block_placed_by`), read back as aim [1, 0, 0] without /botania aim;
#   I. the pool's right-click readout answers and the click is claimed (the mana
#      is untouched), which is the wand's job upstream.
#
# SELFTEST=1 inverts step E's assertion (the near pool must NOT hold 160), which
# must fail: that proves the burst assertion is alive.
#
# Every mod command is typed with `ctl chat "/botania ..."`: `ctl command` runs the
# console path and bypasses the mod's command hook.
set -euo pipefail
trap 'echo "FAILED at line $LINENO: $BASH_COMMAND" >&2' ERR

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/external/botania
# The launcher's export for this mod (launcher-game.sh env prints the same paths).
EXPORT=${HORIZON_E2E_ROOT:-$HOME/.cache/horizon-e2e}/0b14f8a/botania
CTL=${CTL:-$EXPORT/horizon-ctl}
GAME=${GAME:-$EXPORT/horizon-client}
RES=${RES:-${HORIZON_RES:-$HOME/.cache/horizon-e2e/root/data/horizon-launcher/versions/nightly-2026-11-18-0b14f8a/.local/horizon-client/26.3}}
SOCK=${SOCK:-${E2E_TMP:-/tmp/opencode}/botania-e2e.sock}
SELFTEST=${SELFTEST:-0}
WORLD=BotaniaE2E

ENDOFLAME=botania:endoflame
SPREADER=botania:mana_spreader
POOL=botania:mana_pool

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/horizon-mod.toml" ] || { echo "missing package manifest: $PORT/horizon-mod.toml" >&2; exit 1; }
[ -f "$PORT/botania.hmod" ] || { echo "missing package: $PORT/botania.hmod" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/botania-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
say() { echo "== $* =="; }
count_item() { ctl inventory | jq -r --arg i "$1" '.counts[$i] // 0'; }
dropped_item() {
    ctl entities 12 item \
        | jq -r --arg i "$1" '[.entities[] | select(.kind == "item" and .item.item == $i) | .item.count] | add // 0'
}

# The `/botania info` readout for one block. `messages` is newest first, so the
# first line for this position is this query's answer, not an earlier one.
info() { # x y z
    ctl chat "/botania info $1 $2 $3" >/dev/null
    ctl wait 2 >/dev/null
    ctl messages 8 | jq -r '.chat[]' 2>/dev/null \
        | grep -m1 -E "^(Endoflame|Mana Spreader|Mana Pool) at \[$1, $2, $3\]" || true
}
mana_of() { info "$@" | sed -n 's/.*: \([0-9][0-9]*\)\/[0-9][0-9]* mana.*/\1/p'; }
cap_of() { info "$@" | sed -n 's/.*: [0-9][0-9]*\/\([0-9][0-9]*\) mana.*/\1/p'; }
burn_of() { info "$@" | sed -n 's/.*burn \(-*[0-9][0-9]*\) ticks.*/\1/p'; }
aim_of() { info "$@" | sed -n 's/.*aim \(\[[^]]*\]\).*/\1/p'; }

set_block() { # x y z block
    ctl setblock "$1" "$2" "$3" "$4" >/dev/null || fail "cannot set $4 at $1 $2 $3"
    ctl wait 3 >/dev/null
    local got
    got=$(ctl block "$1" "$2" "$3" | jq -r '.block // "unknown"')
    [ "$got" = "$4" ] || fail "the block at $1 $2 $3 is '$got', expected $4"
}
stand_near() { # x z
    ctl tp "$1.5" 101 "$(($2 + 2)).5" >/dev/null
    ctl wait 4 >/dev/null
}
drop_one() { # item
    ctl command "clear @s" >/dev/null
    ctl give "$1" 1 >/dev/null || fail "cannot give $1"
    ctl wait 2 >/dev/null
    ctl select "$1" >/dev/null || fail "cannot select $1"
    ctl wait 2 >/dev/null
    # `drop` throws the item along the view (vanilla's `Player.drop`): a
    # horizontal look sends it 3-4 blocks away, off the platform and out of the
    # flower's 3-block box. Looking straight up drops it at the player's feet.
    ctl look 0 -90 >/dev/null
    ctl wait 1 >/dev/null
    ctl drop >/dev/null || fail "cannot drop $1"
    ctl wait 2 >/dev/null
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" || true
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "E2E EXIT:$status"
}
trap cleanup EXIT

say "launch (headless, creative, the launcher build carries the package)"
"$CTL" --socket "$SOCK" launch --game "$GAME" --world "$WORLD" --seed 20261105 \
    --creative --headless --wait --no-pause --game-dir "$SCRATCH/world" --resources "$RES"
ctl wait-ready 300 >/dev/null
ctl command "gamerule spawn_mobs false" >/dev/null
ctl command "gamerule spawn_monsters false" >/dev/null
ctl command "time set 6000" >/dev/null
ctl command "kill @e[type=!minecraft:player]" >/dev/null
ctl wait 6 >/dev/null

say "an Overworld platform at y=100, away from the terrain"
PX=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1)
PZ=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
ctl command "fill $((PX-10)) 100 $((PZ-10)) $((PX+7)) 100 $((PZ+5)) minecraft:stone" >/dev/null
ctl command "fill $((PX-10)) 101 $((PZ-10)) $((PX+7)) 108 $((PZ+5)) minecraft:air" >/dev/null
ctl tp "$PX.5" 101 "$PZ.5" >/dev/null
ctl wait 8 >/dev/null
Y=101

# Lane A: the coal loop (flower -> spreader -> pool, 2 blocks each).
XA_F=$((PX - 6)); XA_S=$((PX - 4)); XA_P=$((PX - 2)); ZA=$((PZ + 1))
# Lane B: a deterministic 2-block burst.
XB_S=$((PX - 6)); XB_P=$((PX - 4)); ZB=$((PZ - 2))
# Lane C: a deterministic 13-block burst (the loss curve).
XC_S=$((PX - 8)); XC_P=$((PX + 5)); ZC=$((PZ - 5))
# Lane D: a flower and a pool with no spreader in range. It sits at +6/-9 because
# every spreader must be more than 6 blocks away (lane B's is 6 from the old spot,
# and the flower would have emptied into it).
XD_F=$((PX + 6)); XD_P=$((PX + 2)); ZD=$((PZ - 9))
# Lane E: a spreader placed with the item, while looking east.
XE_S=$((PX + 2)); XE_P=$((PX + 4)); ZE=$((PZ + 3))

say "A: the three block items register"
ctl command "clear @s" >/dev/null
for item in "$ENDOFLAME" "$SPREADER" "$POOL"; do
    ctl give "$item" 1 >/dev/null || true
done
ctl wait 3 >/dev/null
for item in "$ENDOFLAME" "$SPREADER" "$POOL"; do
    held=$(count_item "$item")
    echo "  $item in the inventory: $held"
    [ "$held" -ge 1 ] || fail "$item is not a known item: the package's registry did not load"
done

say "A: the three blocks place with the jar's capacities"
set_block "$XA_F" "$Y" "$ZA" "$ENDOFLAME"
set_block "$XA_S" "$Y" "$ZA" "$SPREADER"
set_block "$XA_P" "$Y" "$ZA" "$POOL"
ctl chat "/botania aim $XA_S $Y $ZA east" >/dev/null
ctl wait 3 >/dev/null
flower_line=$(info "$XA_F" "$Y" "$ZA")
spreader_line=$(info "$XA_S" "$Y" "$ZA")
pool_line=$(info "$XA_P" "$Y" "$ZA")
echo "  $flower_line"
echo "  $spreader_line"
echo "  $pool_line"
[ "$(cap_of "$XA_F" "$Y" "$ZA")" = 300 ] || fail "the endoflame's capacity is not 300: $flower_line"
[ "$(cap_of "$XA_S" "$Y" "$ZA")" = 1000 ] || fail "the spreader's capacity is not 1000: $spreader_line"
[ "$(cap_of "$XA_P" "$Y" "$ZA")" = 1000000 ] || fail "the pool's capacity is not 1,000,000: $pool_line"
[ "$(aim_of "$XA_S" "$Y" "$ZA")" = "[1, 0, 0]" ] || fail "the spreader's aim is not east: $spreader_line"

say "B: with no fuel the flower makes nothing and the pool stays empty"
ctl wait 100 >/dev/null
[ "$(mana_of "$XA_F" "$Y" "$ZA")" = 0 ] || fail "an unfuelled flower holds mana: $(info "$XA_F" "$Y" "$ZA")"
[ "$(burn_of "$XA_F" "$Y" "$ZA")" = 0 ] || fail "an unfuelled flower is burning: $(info "$XA_F" "$Y" "$ZA")"
[ "$(mana_of "$XA_P" "$Y" "$ZA")" = 0 ] || fail "the pool took mana with nothing burning: $(info "$XA_P" "$Y" "$ZA")"
echo "  100 ticks with no fuel: nothing anywhere"

say "C: a dropped dirt is not fuel"
stand_near "$XA_F" "$ZA"
drop_one minecraft:dirt
# Step out of pickup range at once: the drop lands inside the flower's box, and a
# creative player standing on it would collect it after its 40-tick pickup delay,
# which would hide whether the flower took it.
ctl tp "$PX.5" 101 "$PZ.5" >/dev/null
ctl wait 40 >/dev/null
[ "$(burn_of "$XA_F" "$Y" "$ZA")" = 0 ] || fail "the flower burned a dirt: $(info "$XA_F" "$Y" "$ZA")"
[ "$(mana_of "$XA_F" "$Y" "$ZA")" = 0 ] || fail "a dirt made mana: $(info "$XA_F" "$Y" "$ZA")"
[ "$(dropped_item minecraft:dirt)" -ge 1 ] || fail "the dirt disappeared from a flower that cannot burn it"
echo "  the dirt stayed on the ground and nothing burned"

say "D: one coal ignites for 800 ticks and the loop delivers whole 160-mana bursts"
stand_near "$XA_F" "$ZA"
drop_one minecraft:coal
ctl wait 20 >/dev/null
burn=$(burn_of "$XA_F" "$Y" "$ZA")
echo "  $(info "$XA_F" "$Y" "$ZA")"
[ "$burn" -gt 700 ] || fail "one coal ignited for $burn burn ticks, expected about 778"
pool=0
for _ in $(seq 1 40); do
    ctl wait 10 >/dev/null
    pool=$(mana_of "$XA_P" "$Y" "$ZA")
    [ "${pool:-0}" -gt 0 ] && break
done
echo "  the first burst landed: pool $pool mana"
[ "${pool:-0}" -gt 0 ] || fail "no burst reached the pool while the flower was burning"
[ $((pool % 160)) = 0 ] || fail "the pool holds $pool mana, not whole 160-mana bursts"
flower=$(mana_of "$XA_F" "$Y" "$ZA")
[ "$flower" = 0 ] || fail "the flower kept $flower mana instead of handing it to the spreader"

say "D: the pool keeps growing in whole bursts and the flower stays empty"
# A spreader fires one burst per ~107 ticks of the flower's trickle (160 mana at
# 3 per even tick), and nothing while a burst is in flight, so poll for the next
# landing instead of assuming a fixed interval.
later=$pool
for _ in $(seq 1 40); do
    ctl wait 10 >/dev/null
    later=$(mana_of "$XA_P" "$Y" "$ZA")
    [ "${later:-0}" -gt "$pool" ] && break
done
echo "  pool $pool -> $later"
[ "${later:-0}" -gt "$pool" ] || fail "the pool stopped growing at $later while the coal still burns"
[ $((later % 160)) = 0 ] || fail "the pool holds $later mana, not whole 160-mana bursts"
[ "$(mana_of "$XA_F" "$Y" "$ZA")" = 0 ] || fail "the flower holds mana after emptying: $(info "$XA_F" "$Y" "$ZA")"
spread=$(mana_of "$XA_S" "$Y" "$ZA")
[ "${spread:-0}" -lt 160 ] || fail "the spreader holds a whole burst but did not fire: $(info "$XA_S" "$Y" "$ZA")"

say "E: a burst that lands 2 blocks away hands the pool exactly 160"
set_block "$XB_S" "$Y" "$ZB" "$SPREADER"
set_block "$XB_P" "$Y" "$ZB" "$POOL"
ctl chat "/botania aim $XB_S $Y $ZB east" >/dev/null
ctl chat "/botania mana $XB_S $Y $ZB 160" >/dev/null
ctl wait 30 >/dev/null
near=$(mana_of "$XB_P" "$Y" "$ZB")
echo "  near pool: $(info "$XB_P" "$Y" "$ZB")"
if [ "$SELFTEST" = 1 ]; then
    say "selftest: expect the near pool to hold no burst (must fail)"
    [ "$near" = 160 ] && fail "SELFTEST: the near pool holds 160, so the burst assertion is alive"
    echo "SELFTEST unexpectedly saw $near mana"
    exit 0
fi
[ "$near" = 160 ] || fail "a 2-block burst handed the pool $near mana, expected 160"
[ "$(mana_of "$XB_S" "$Y" "$ZB")" = 0 ] || fail "the spreader kept mana after firing 160: $(info "$XB_S" "$Y" "$ZB")"

say "F: a burst that lands 13 blocks away arrives with 136"
set_block "$XC_S" "$Y" "$ZC" "$SPREADER"
set_block "$XC_P" "$Y" "$ZC" "$POOL"
ctl chat "/botania aim $XC_S $Y $ZC east" >/dev/null
ctl chat "/botania mana $XC_S $Y $ZC 160" >/dev/null
ctl wait 90 >/dev/null
far=$(mana_of "$XC_P" "$Y" "$ZC")
echo "  far pool: $(info "$XC_P" "$Y" "$ZC")"
[ "$far" = 136 ] || fail "a 13-block burst handed the pool $far mana, expected 136 (160 - 6 x 4)"

say "G: a flower with a pool but no spreader delivers nothing"
set_block "$XD_F" "$Y" "$ZD" "$ENDOFLAME"
set_block "$XD_P" "$Y" "$ZD" "$POOL"
stand_near "$XD_F" "$ZD"
drop_one minecraft:coal
# 300 mana at 3 per even tick needs ~200 ticks, and the flower stops taking fuel
# once full, so poll for the cap rather than assuming one interval.
gmana=0
for _ in $(seq 1 40); do
    ctl wait 10 >/dev/null
    gmana=$(mana_of "$XD_F" "$Y" "$ZD")
    [ "${gmana:-0}" = 300 ] && break
done
echo "  $(info "$XD_F" "$Y" "$ZD")"
echo "  $(info "$XD_P" "$Y" "$ZD")"
[ "$(burn_of "$XD_F" "$Y" "$ZD")" -gt 0 ] || fail "the no-spreader flower did not burn its coal"
[ "${gmana:-0}" = 300 ] || fail "the no-spreader flower did not fill: $(info "$XD_F" "$Y" "$ZD")"
[ "$(mana_of "$XD_P" "$Y" "$ZD")" = 0 ] || fail "the pool took mana with no spreader in range: $(info "$XD_P" "$Y" "$ZD")"

say "H: a placed spreader keeps the axis the placer looked along"
set_block "$XE_P" "$Y" "$ZE" "$POOL"
ctl tp "$((XE_S - 2)).5" 101 "$((ZE + 2)).5" >/dev/null
ctl look -90 0 >/dev/null
ctl wait 4 >/dev/null
ctl command "clear @s" >/dev/null
ctl give "$SPREADER" 1 >/dev/null
ctl wait 2 >/dev/null
ctl select "$SPREADER" >/dev/null
ctl wait 2 >/dev/null
ctl place "$XE_S" "$Y" "$ZE" --item "$SPREADER" >/dev/null || fail "cannot place the spreader item at $XE_S $Y $ZE"
ctl wait 4 >/dev/null
placed=$(ctl block "$XE_S" "$Y" "$ZE" | jq -r '.block // "unknown"')
[ "$placed" = "$SPREADER" ] || fail "the placed block at $XE_S $Y $ZE is '$placed'"
aim=$(aim_of "$XE_S" "$Y" "$ZE")
echo "  $(info "$XE_S" "$Y" "$ZE")"
[ "$aim" = "[1, 0, 0]" ] || fail "a spreader placed while looking east aims $aim, expected [1, 0, 0]"

say "I: the pool's readout answers a click and the click is claimed"
# The lane A pool is still filling from its coal, so a burst can land between the
# two reads; the lane B pool holds one burst and receives nothing more.
before=$(mana_of "$XB_P" "$Y" "$ZB")
ctl command "clear @s" >/dev/null
ctl tp "$((XB_P - 2)).5" 101 "$((ZB + 1)).5" >/dev/null
ctl look-at-block "$XB_P" "$Y" "$ZB" >/dev/null
ctl wait 4 >/dev/null
ctl interact "$XB_P" "$Y" "$ZB" >/dev/null || fail "the click on the pool failed"
ctl wait 4 >/dev/null
bar=$(ctl messages 6 | jq -r '.action_bar // .actionbar // ""' 2>/dev/null || true)
after=$(mana_of "$XB_P" "$Y" "$ZB")
echo "  readout: $(info "$XB_P" "$Y" "$ZB")"
echo "  action bar: $bar"
[ "$after" = "$before" ] || fail "the click changed the pool's mana: $before -> $after"
[ -n "$bar" ] || fail "the pool's click produced no action-bar line"
printf '%s' "$bar" | grep -q "Mana Pool: $after/1000000 mana" \
    || fail "the action bar does not carry the pool's readout: '$bar'"

echo "PASS: an unfuelled endoflame makes no mana and a non-fuel item is not burned, one coal ignites for 800 burn ticks, the flower hands its mana to the spreader and the pool grows in whole 160-mana bursts, a 2-block burst lands 160 and a 13-block burst arrives with 136, a pool with no spreader receives nothing, a placed spreader keeps the axis it was placed along, and the pool answers a claimed right-click with its readout"
