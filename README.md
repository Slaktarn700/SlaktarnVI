# SlaktarnVI

**Valhalla, by Slaktarn**: a fork of [VIctory](Marvisak/VIctory), a
compatibility layer that runs an Xbox Series title natively on a PC from files you supply,
here tuned for **Linux under Proton on an AMD Radeon RX 9070 XT**.

**This repository contains no game files, code or assets, and no binaries built from them.**
You need your own legally obtained copy of the title. This project is not affiliated with or
endorsed by the title's developer or publisher, nor by Microsoft or AMD.

## Where it stands

Tested on one machine only (see below), with the launcher's **S70** preset:
2560x1440 output, FSR 4 Quality, frame generation on, the Series X profile, no ray tracing.

| | Real frames (frame generation not counted) |
|---|---|
| Opening cutscene | 15-19 fps |
| Gameplay and later cutscenes | about 19-25 fps |
| Scene cuts (streaming a new place) | a few seconds at 3-8 fps |

Fixed: characters stretched to a point (vertices thrown off), faces and clothes in coloured
squares, textures picked by index left with old or empty bytes, the 3 fps heist (dynamic
descriptor slots running out), trees and grass popping from batch order, a hang of the test
suite. **Open**: the frame rate (goal: a steady 30 real fps), the scene-cut hitch, unordered
views of one compressed texture not mirrored, ray tracing (off until 30 fps).

The first run after a new build is slow for some seconds after loading: the driver builds
its pipeline cache for the changed executable. The second run is not.

## Tested system

| | |
|---|---|
| OS | CachyOS, kernel 7.3.5-2-cachyos |
| GPU | AMD Radeon RX 9070 XT (RDNA 4, gfx1201), 16 GB |
| GPU driver | **Mesa 26.3.1** (RADV, `vulkan-radeon 26.3.1-5`), Vulkan 1.4.363 |
| Firmware | `linux-firmware 20261019` |
| CPU | AMD Ryzen 7 9800X3D (8 cores) |
| RAM | 32 GB (30 GB usable); while it runs about 22 GB are in use in all |
| Proton | Proton Experimental `experimental-11.0-20261112`, with `linux/wineserver-view-tree.patch` |
| Rust | 1.98.1 (`rust-toolchain.toml`), target `x86_64-pc-windows-msvc`, `cargo-xwin 0.24.1` |
| Python | 3.14 with PySide6 6.12 (the launcher) |

Other AMD cards of RDNA 3 and 4 should work; FSR 4 needs RDNA 4. Nothing here was tested on
NVIDIA or Intel.

## What you need besides this repository

1. **Your own copy of the title** (its folder with the executable).
2. **Proton Experimental** from Steam. Recommended: rebuild its `wineserver` with
   `linux/wineserver-view-tree.patch` (finds memory views by address; without it streaming
   is much slower). Put that copy in `linux/proton-patched/` (it is used when it is there) or
   point `PROTON=` at it. Keep Steam's own copy untouched.
3. **Microsoft's shader compiler**, `d3dcompiler_47.dll` and `dxil.dll`, from the Windows
   SDK package on nuget.org (Wine's own compiler lacks what the runtime's shaders use), in
   `linux/keep/ms-d3d/`.
4. **For FSR and frame generation**: `amd_fidelityfx_upscaler_dx12.dll` and
   `amd_fidelityfx_framegeneration_dx12.dll` from AMD's FidelityFX SDK 2.4
   (`Kits/FidelityFX/signedbin` in GPUOpen-LibrariesAndSDKs/FidelityFX-SDK), in
   `linux/keep/fidelityfx/`.
5. **Rust** with rustup (the toolchain file picks 1.98.1) and `cargo install cargo-xwin`
   (it downloads the Windows SDK headers and libraries on first use; set
   `XWIN_ACCEPT_LICENSE=1` once you have read Microsoft's license).

## Build

```bash
cargo xwin build --locked --release -p victory --bins --target x86_64-pc-windows-msvc
```

The build with the diagnostics the scripts below log (frame lines, A/B phases, captures):

```bash
cargo xwin build --locked --profile diag -p victory --features diagnostics --bins --target x86_64-pc-windows-msvc
```

Tests run under Proton through `scripts/proton-run.sh` as the cargo runner (see
`docs/linux-wine.md`); one test, `switches::documented_switches_are_read`, fails under
Wine only.

## Run

Everything runs from `linux/`: copy the built `valhalla.exe` (from
`target/x86_64-pc-windows-msvc/diag/` or `.../release/`) to `linux/bin/valhalla.exe` (or set
`EXE=` to it), put the title in `linux/game/` or set `GAME_DIR=`, and set `GAME_EXE=` to the
file name of its executable. Caches and saves go to `linux/victory-data/`.

```bash
./linux/valhalla.sh
```

starts the launcher (presets Low, Medium, High, S70, Ultra; the S70 preset is what every
measurement above used). Without the launcher:

```bash
GAME_EXE=<the executable's name> ./linux/s70.sh
```

`linux/run-7900.sh` lists the switches it sets and the options for unattended runs (`SECS`,
`WALK`, `HOLD`, `LOG`). Every run writes a log and `<log>-hw.tsv` (GPU and CPU once a
second, `linux/hw-log.py`). `linux/ab-report.py` sums up an in-run A/B
(`VICTORY_AB=<seconds>,<switch>`).

Keyboard stand-in for the pad: W/A/S/D left stick, I/J/K/L right stick, Z and C the
triggers, Enter A, Backspace B, X and Y, Q and E the shoulders, Esc menu, Tab view; F3
shows or hides the info panel, F4 saves the next frame (with `VICTORY_CAPTURE`).

## Credits and license

GPL-3.0-or-later, as VIctory (see `LICENSE`). This fork builds on VIctory by Marvisak and
takes changes from the forks of fxceless, DexNom, myokex, avatarmanz, Belobug, ivall and
others, each noted where it is used in the code.
