#!/usr/bin/env bash
# Runs a Windows exe (victory.exe by default) under GE-Proton's Wine, with
# vkd3d-proton (D3D12) and DXVK (DXGI) from the Proton build.
#
#   ./run-victory.sh config
#   ./run-victory.sh run "$PWD/game/<title>.exe" --execute
#   VICTORY_EXE=/path/to/other.exe ./run-victory.sh ...     (used as cargo runner)
#
# Linux paths in arguments are fine: Wine maps / to Z:\.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
. "$HERE/data-dir.sh"
DATA_DIR=$(valhalla_data_dir "$HERE")
# Proton Experimental: its vkd3d-proton (dxil-spirv) builds every pipeline of
# the title; GE-Proton11-11's emits invalid SPIR-V for some, which crashes RADV.
PROTON="${PROTON:-$HOME/.steam/steam/steamapps/common/Proton - Experimental}"
PREFIX="${PREFIX:-$HERE/proton-prefix-exp}"

# First run: let Proton build the prefix (installs vkd3d-proton / DXVK DLLs).
if [[ ! -f "$PREFIX/pfx/system.reg" ]]; then
    mkdir -p "$PREFIX"
    STEAM_COMPAT_DATA_PATH="$PREFIX" \
    STEAM_COMPAT_CLIENT_INSTALL_PATH="${STEAM_COMPAT_CLIENT_INSTALL_PATH:-$HOME/.local/share/Steam}" \
    SteamAppId=0 SteamGameId=0 \
        python3 "$PROTON/proton" run cmd /c exit >/dev/null 2>&1 || true
fi

export WINEPREFIX="$PREFIX/pfx"
export WINEDLLOVERRIDES="d3d12,d3d12core,dxgi,d3d11,d3d10core=n,b;${WINEDLLOVERRIDES:-}"
export WINEDEBUG="${WINEDEBUG:--all}"
export WINEFSYNC=1
# Keep vkd3d-proton's pipeline cache out of the working directory.
export VKD3D_SHADER_CACHE_PATH="${VKD3D_SHADER_CACHE_PATH:-$DATA_DIR/vkd3d-cache}"
mkdir -p "$VKD3D_SHADER_CACHE_PATH"
# Where victory keeps saves / shader cache unless victory.toml says otherwise.
export VICTORY_DATA_DIR="${VICTORY_DATA_DIR:-$DATA_DIR/data}"
export VICTORY_CACHE_DIR="${VICTORY_CACHE_DIR:-$DATA_DIR/cache}"
mkdir -p "$VICTORY_DATA_DIR" "$VICTORY_CACHE_DIR"

# Microsoft's own shader compiler (keep/ms-d3d, from the Windows SDK package on
# nuget.org): Wine's lacks [RootSignature], ByteAddressBuffer methods and more,
# which the runtime's own shaders use. Copied next to the exe that runs.
MSD3D="${MSD3D:-$HERE/keep/ms-d3d}"
with_ms_d3d() {
    local dir; dir="$(dirname "$1")"
    for f in d3dcompiler_47.dll dxil.dll; do
        [[ -f "$dir/$f" ]] || cp "$MSD3D/$f" "$dir/$f" 2>/dev/null || true
    done
}
export WINEDLLOVERRIDES="d3dcompiler_47,dxil=n,b;${WINEDLLOVERRIDES:-}"
if [[ -n "${VICTORY_EXE:-}" ]]; then
    with_ms_d3d "$VICTORY_EXE"
    exec "$PROTON/files/bin/wine" "$VICTORY_EXE" "$@"
elif [[ "${1:-}" == *.exe && -f "${1:-}" ]]; then
    with_ms_d3d "$1"
    exec "$PROTON/files/bin/wine" "$@"          # cargo runner: first arg is the exe
else
    exec "$PROTON/files/bin/wine" "$HERE/VIctory/target/x86_64-pc-windows-msvc/release/victory.exe" "$@"
fi
