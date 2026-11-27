#!/usr/bin/env bash
# Where saves, caches and logs of a run go. The runtime refuses to put game-derived
# data inside a VIctory source tree (docs/asset-policy.md), so a linux/ folder that is
# still in a checkout keeps them in the user's data directory. A linux/ folder copied
# out of the tree keeps its own victory-data/ next to the scripts, as before.
#
#   . ./data-dir.sh
#   DATA_DIR=$(valhalla_data_dir "$PWD")
valhalla_data_dir() {
    local dir=$1 p
    p=$(cd "$dir" && pwd -P) || return 1
    while :; do
        if [ -f "$p/.victory-root" ]; then
            printf '%s\n' "${XDG_DATA_HOME:-$HOME/.local/share}/victory"
            return 0
        fi
        [ "$p" = "/" ] && break
        p=$(dirname "$p")
    done
    printf '%s\n' "$dir/victory-data"
}
