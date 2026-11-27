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
