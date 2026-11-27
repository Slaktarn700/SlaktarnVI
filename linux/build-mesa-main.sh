#!/usr/bin/env bash
# Builds RADV (Vulkan only, 64-bit) from Mesa's main branch into a prefix of its own;
# the system's Mesa is not touched. Use it for one run with MESA_PREFIX=<prefix>
# (run-7900.sh, s70.sh) or VK_DRIVER_FILES=<prefix>/share/vulkan/icd.d/radeon_icd.x86_64.json.
#   ./build-mesa-main.sh                 into ~/mesa-main/{src,build,prefix}
#   MESA_ROOT=/elsewhere ./build-mesa-main.sh
# Needs meson, ninja, glslang, python-mako, llvm, libdrm, wayland and X11 headers
# (Arch: the build dependencies of vulkan-radeon). A rerun updates src to the newest main.
set -euo pipefail
ROOT=${MESA_ROOT:-$HOME/mesa-main}
mkdir -p "$ROOT"
cd "$ROOT"
if [ -d src/.git ]; then
    git -C src fetch --depth 1 origin main && git -C src reset --hard FETCH_HEAD
else
    git clone --depth 1 https://gitlab.freedesktop.org/mesa/mesa.git src
fi
git -C src log -1 --format='%H %cI %s' > commit.txt
rm -rf build
meson setup build src --prefix="$ROOT/prefix" --buildtype=release -Db_ndebug=true \
    -Dvulkan-drivers=amd -Dgallium-drivers= -Dplatforms=x11,wayland \
    -Dvulkan-layers= -Dllvm=enabled -Dvideo-codecs= -Dglx=disabled -Degl=disabled -Dgbm=disabled \
    -Dopengl=false -Dgles1=disabled -Dgles2=disabled -Dtools=
nice -n 15 ninja -C build
rm -rf prefix
ninja -C build install
echo "built $(cut -c1-12 commit.txt): MESA_PREFIX=$ROOT/prefix"
