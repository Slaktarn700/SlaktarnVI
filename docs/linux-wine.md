# Running this fork on Linux under Wine

Measured on one PC: Arch Linux (kernel 7.3), i9-13900KF, 32 GB RAM + 16 GB swap,
Radeon RX 7900 XTX (RADV, Mesa 26.3.1), Wine 11.22, Series X profile, 1080p window.
Scene: the opening prison cell, standing, 4,000-5,000 draws a frame. Runs of 5 minutes,
`scripts/play.ps1` settings (not `-Safe`), ray tracing off.

| Build | Before the changes below | After |
|---|---|---|
| diag | 3.4-3.5 fps (270-370 ms a frame) | 17.0-17.3 fps (about 56 ms) |
| release | - | **23.5 fps, 25.5 at the end of the run** (about 40 ms) |

For comparison, `fxceless/victory-fork` on the same machine: 6.0 fps (diag), 7.3 (release).
Memory: about 26 GB in use while playing, swap did not grow.

## What it needs

1. **A `wineserver` that keeps memory views in a tree**
   ([CachyOS/wine-cachyos@fd66dbc](https://github.com/CachyOS/wine-cachyos/commit/fd66dbca8eb0754839ca974e45b5ea446ca62d2f),
   see Marvisak/VIctory#834). Build `server/wineserver` from the source of the Wine you
   run, with that commit applied, and swap the binary. Without it every remap of title
   memory walks a list of tens of thousands of views: 0.4 fps instead of 6 on the other
   fork. With GE-Proton, build it from GE's own source (protocol 938 for GE-Proton11-14).
2. **vkd3d-proton with a dxil-spirv from 2026-11-25 or later** (master `71cae091`, the
   "Artifacts (Package)" build of its CI). With 3.2 (GE-Proton11-14) dxil-spirv recurses
   without end on five pipelines of the prologue (one graphics, four compute): the
   pipeline workers die of a stack overflow at the same address even with a 64 MiB
   stack, those pipelines are never made, and a character's face is not drawn. With master:
   no worker dies, nothing is left "being made".
3. **A low `VICTORY_PIPELINE_WAIT_MS`** (2 here). With workers dead or a pipeline still
   being made, the default 500 ms is spent in every frame (`recording per frame: ...
   500.1 ms waiting for pipelines`), which is all of the difference between 3.5 and 17 fps
   in the table above.
4. DXVK's `dxgi.dll` in the prefix (Wine's own fails), as for the other forks.

## Changes in this tree for it

- The pipeline workers get a 64 MiB stack (reserved, not committed). Under Wine the DXIL
  to SPIR-V conversion inside the driver call can pass the default 2 MiB.
- `VICTORY_LOG_THREADS=1` names the thread on each log line. That is how the five
  pipelines were found: the last `host pipeline shaders=[...]` line of each worker
  before its `has overflowed its stack`.
- `VICTORY_KEYS` (diagnostics builds) also gives a scripted run with no pad a host pad
  in slot 0: the account screen does not take the stand-in pad's A, so an unattended run
  without a pad stopped there.

## Notes

- Diagnostics builds are about 20-30 % slower than release under Wine, more than on
  Windows: `GetThreadTimes` goes through `wineserver`, and the submitting thread spent
  about a quarter of its time in `copystat::submission_ends` in a profile.
- The first run after a change of the translator or of vkd3d-proton stutters while the
  driver makes pipelines. Mesa's disk cache keeps them for the next run.
- `victory shaders <archives>` under Wine translates all 25,790 shaders in about 95 s on
  24 threads.

## A second machine: Ryzen 7 5800X (CPU bound)

Ryzen 7 5800X (8 cores, 32 MB L3, no 3D V-Cache), 32 GB, RX 9070 XT 16 GB, CachyOS kernel
7.4.0-rc1, Proton Experimental bleeding-edge (vkd3d-proton `71cae0`) with
`wineserver-view-tree.patch`, ray tracing off. diag build of `42b2226`, S70 settings
(2560x1440, FSR 4 Quality from 1706x960), frame generation off. Scene: the opening prison
cell, standing, about 4,840 draws a frame. Each row: the median of 60 s after 90 s in
the world.

| Run | fps | ms a frame | on the GPU |
|---|---|---|---|
| Mesa 26.2.6 (system) | 10.1 | 99 | 24 ms |
| Mesa main `aad91d03` (`build-mesa-main.sh`, `MESA_PREFIX=`) | 13.1 | 77 | 19 ms |
| Mesa main, 1920x1080 output (FSR from 1280x720) | 13.6 | 74 | 16.5 ms |
| Mesa main, transparent huge pages `always` | **14.4** | 69 | 15 ms |
| Mesa main, `VICTORY_CARD_STEPS_LEARN=0` | 15.5 | 65 | - |

- The CPU is the limit: the info panel shows CPU about 75 %, GPU about 37 %. A lower output
  size gains 4 %; most of a frame is copies (about 36 ms, 70 MB) and copies back (about
  24 ms, 35 MB).
- **Mesa older than 26.3 costs about 30 %** here: 26.2.6 against main, same build and settings.
- **Transparent huge pages** (`echo always | sudo tee
  /sys/kernel/mm/transparent_hugepage/enabled`, until reboot): +10 %. The title memory opened
  with `OpenExistingHeapFromAddress` becomes a userptr buffer in amdgpu, and the kernel walks
  its pages (`hmm_range_fault`, 174 MiB here) on every submission of vkd3d-proton's queue
  thread: about 700 times a second, 520 us each with 4 KiB pages (a third of a core), 66 us
  with 2 MiB pages (bpftrace on the kernel function, 10 s each).
- `VICTORY_CARD_STEPS_LEARN=0` is 18 % faster here, with the stretched characters described
  in `run-7900.sh`; it stays on.
- `VKD3D_CONFIG=dxr` against `dxr,descriptor_heap`: the same (13.1 and 13.2 fps).
- The **release** build of `42b2226` showed only a teal rectangle at the top left on black
  once in the world, in three runs on this machine (with and without lsfg-vk, Mesa 26.2.6
  and main), while its frame lines went on (14.3 fps at 1080p). Two more release runs
  stopped with an access violation (one at 2560x1440, one with FSR off). The diag build of
  the same commit, with the same settings, drew the world in all eight of its runs (five
  checked by screenshot). Cause not found yet.
