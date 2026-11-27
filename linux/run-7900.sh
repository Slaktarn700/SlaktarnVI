#!/bin/bash
# Run the game with the RX 7900 XTX fork (victory7900/, branch linux) under
# Proton, from your saves in victory-data/, with the settings of the fork's own
# scripts/play.ps1.
#   ./run-7900.sh               the fork's play settings   (log: 7900.log)
#   SAFE=1 ./run-7900.sh        without the newest, faster ways (play.ps1 -Safe)
#   RT=1 ./run-7900.sh          the title's ray-traced indirect light (about a third slower)
#   SECS=480 ./run-7900.sh      stop after 480 s (unattended tests)
#   LOG=x.log                   another log file
#   CONSOLE=series-s            run as that console (lighter settings, lower resolution)
#   WALK=150-300                walk (5 s forward, 5 s back, repeated) from 150 s to 300 s
#   HOLD="500-560:C 520-524:A"  keys held over those seconds (C: right trigger, Z: left)
#   VICTORY_PANEL=0             without the on-screen panel (fps and frame parts)
# Proton: VICTORY_PIPELINE_WAIT_MS=30 (the fork waits up to 500 ms a frame for
# pipelines; under Proton a few are never made, and every frame waited all 500).
# VICTORY_PIPELINE_BOOK=1 writes down the pipelines the title creates, for
#   ./run-victory.sh warm --used  (builds them ahead, between runs).
# Unplug the controller for the first 40 s so the intro is skipped.
cd "$(dirname "$0")"
. ./data-dir.sh
DATA_DIR=$(valhalla_data_dir "$PWD")
# SKIP_INTRO=0: no Enter presses in the first 40 s (they skip the intro).
keys=""
if [ "${SKIP_INTRO:-1}" = 1 ]; then
    keys=$(for t in $(seq 8 2 40) ${EXTRA_KEYS:-}; do printf '%s:Enter,' "$t"; done); keys=${keys%,}
fi
# WALK=<from>-<to>: walk 5 s forward (W), 5 s back (S), over and over, all
# that time: the character stays where it is (unattended tests while walking).
if [ -n "${WALK:-}" ]; then
    from=${WALK%-*}; to=${WALK#*-}
    for ((t = from; t < to; t += 10)); do keys+=",$t-$((t + 5)):W,$((t + 5))-$((t + 10)):S"; done
    keys=${keys#,}
fi
# HOLD="<from>-<to>:<key> ...": keys held over those seconds.
for h in ${HOLD:-}; do keys+=",$h"; done
keys=${keys#,}
# scripts/play.ps1's settings.
extra=(VICTORY_TEXTURE_NO_WAIT=1 VICTORY_RELAY=2 VICTORY_WIDE_BACK=1 VICTORY_CARD_REHASH_RUNS=1
       VICTORY_JOIN_COPIES=1 VICTORY_INDIRECT_WINDOW=1 VICTORY_RELAY_COPY=1 VICTORY_CARD_STEPS=2
       VICTORY_INDIRECT_BATCH=2 VICTORY_MAP_THREADS=1 VICTORY_TITLE_PRIORITIES=0 VICTORY_CARD_AWAKE=1
       VICTORY_SHOW_AFTER=1 VICTORY_AMBIENT="${VICTORY_AMBIENT:-0.8}" VICTORY_D3D12_DRED="${VICTORY_D3D12_DRED:-light}"
       VICTORY_PORT_DESCRIPTORS=1 VICTORY_PORT_CONSTANTS=1 VICTORY_PORT_PRIVATE=1
       VICTORY_FRAME_INTERVAL_MS="${VICTORY_FRAME_INTERVAL_MS:-16.667}" VICTORY_PORT_CBV=1
       VICTORY_OUTPUT="${VICTORY_OUTPUT:-2560x1440}")
if [ "${SAFE:-0}" = 1 ]; then
    extra+=(VICTORY_UNDER_WIDE=0 VICTORY_LAND_ECHO=0)
else
    # Renderer port 6 (VICTORY_PORT_GUARD=1, the guards' lock shared by faults and refreshes)
    # stays off: A/B p90 -1.4 to -5 ms, but in a 15-minute walk with VICTORY_REFRESH_CHECK=1
    # 61 blocks of a copy in the card's memory stayed stale three refreshes in a row while
    # the title streamed (refreshes leave blocks they cannot claim). STATUS.md, 2026-11-27.
    # avatarmanz#434 (card steps learn the places the title writes after sending, and
    # submissions reading them wait: against stretched characters): on. Off, a third of
    # the frames of the boat cutscene had a character's face, hair or clothes stretched to
    # a point (run ab2, 17 of 52 screen shots); on, none of 40 (run cs1). It costs about
    # 6 ms a frame there (19.5 against 22.2 fps): =0 for the speed without it.
    extra+=(VICTORY_THIN=1 VICTORY_JOIN_PARTS=8 VICTORY_UPLOAD_AHEAD=1 VICTORY_CARD_STEPS_LEARN="${VICTORY_CARD_STEPS_LEARN:-1}")
fi
[ "${RT:-0}" = 1 ] && extra+=(VICTORY_BVH_BUILDER=1)
# PATCHED=1 (default): our copy of Proton Experimental whose wineserver finds
# views by address (keep/wineserver-view-tree.patch); PATCHED=0: Steam's own.
if [ "${PATCHED:-1}" = 1 ] && [ -x "$PWD/proton-patched/files/bin/wineserver" ]; then
    export PROTON="${PROTON:-$PWD/proton-patched}"
fi
# GAME_DIR: the game's folder (its own files; default game/ beside this script).
gamedir="${GAME_DIR:-$PWD/game}"
run=(./run-victory.sh run "$gamedir/${GAME_EXE:?set GAME_EXE to the file name of the game executable}" --execute)
# CONSOLE=series-s: the title believes it runs on that console (its lighter settings).
[ -n "${CONSOLE:-}" ] && run+=(--console "$CONSOLE")
# The program: bin/valhalla.exe in a package, else this tree's build.
exe="$PWD/bin/valhalla.exe"
[ -f "$exe" ] || exe="$PWD/target-7900/x86_64-pc-windows-msvc/diag/valhalla.exe"
# GPU and CPU once a second beside the game's log: <log>-hw.tsv (hw-log.py,
# read-only; ends with the game).
# NO_HW_LOG=1: without it.
hwlog="${LOG:-7900.log}"
[ "${NO_HW_LOG:-0}" = 1 ] || python3 -I "$PWD/hw-log.py" "${hwlog%.log}-hw.tsv" 180 >/dev/null 2>&1 &
[ -n "${SECS:-}" ] && run=(timeout "$SECS" "${run[@]}")
env "${extra[@]}" \
VICTORY_KEYS="$keys" \
VICTORY_PANEL="${VICTORY_PANEL:-1}" \
VICTORY_PIPELINE_WAIT_MS="${VICTORY_PIPELINE_WAIT_MS:-30}" \
VICTORY_PIPELINE_BOOK="${VICTORY_PIPELINE_BOOK:-1}" \
VICTORY_CACHE_DIR="${VICTORY_CACHE_DIR:-$DATA_DIR/cache-7900}" \
VICTORY_EXE="${EXE:-$exe}" \
VICTORY_GAME_DIR="Z:${gamedir//\//\\}" \
    "${run[@]}" 2>&1 | sed -u 's/\x1b\[[0-9;]*m//g' > "${LOG:-7900.log}"
echo "exit $?" >> "${LOG:-7900.log}"
