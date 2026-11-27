#!/usr/bin/env python3
"""A/B phases of a run by switch combination (VICTORY_AB lines).

    ab-report.py <log> [first second] [last second] [least draws a frame, 3000]

Keeps phases in the world (over 3000 draws a frame) that ended between the
given seconds after the log's first timestamp (the walking window), and per
combination prints the phases' medians and their mean, frames weighted.
"""
import collections
import datetime
import re
import sys

log = sys.argv[1]
lo = float(sys.argv[2]) if len(sys.argv) > 2 else 0
hi = float(sys.argv[3]) if len(sys.argv) > 3 else 1e9
least = int(sys.argv[4]) if len(sys.argv) > 4 else 3000
start = None
by = collections.defaultdict(list)
for line in open(log, errors="replace"):
    m = re.match(r"(\d{4}-\d\d-\d\dT[\d:.]+)Z", line)
    if not m:
        continue
    t = datetime.datetime.fromisoformat(m.group(1))
    if start is None:
        start = t
    m2 = re.search(r"A/B phase (\d+) bits (\d+) on \[([^\]]*)\]: (\d+) frames, mean ([\d.]+) ms, median ([\d.]+), p90 ([\d.]+), (\d+) draws", line)
    if not m2:
        continue
    secs = (t - start).total_seconds()
    phase, bits, on, frames, mean, median, p90, draws = m2.groups()
    if int(draws) < least or not (lo <= secs <= hi):
        continue
    by[on or "(none)"].append((int(frames), float(mean), float(median), float(p90), secs))
for on, ph in sorted(by.items(), key=lambda kv: sum(p[2] * p[0] for p in kv[1]) / sum(p[0] for p in kv[1])):
    n = sum(p[0] for p in ph)
    med = sum(p[2] * p[0] for p in ph) / n
    p90 = sum(p[3] * p[0] for p in ph) / n
    print(f"{on:55s} {len(ph)} phases, {n} frames: median {med:.1f} ms ({1000 / med:.1f} fps), p90 {p90:.1f}; phases {[p[2] for p in ph]}")

# Each switch on against off, over all phases kept (the others mixed alike).
switches = sorted({s for on in by for s in on.split() if on != "(none)"})
if switches:
    print("\nper switch (frames-weighted median, on against off):")
    for s in switches:
        on = [p for k, ph in by.items() for p in ph if s in k.split()]
        off = [p for k, ph in by.items() for p in ph if s not in k.split()]
        w = lambda ps: sum(p[2] * p[0] for p in ps) / max(1, sum(p[0] for p in ps))
        if on and off:
            print(f"  {s:32s} on {w(on):.1f} ms ({len(on)} phases)  off {w(off):.1f} ms ({len(off)} phases)  on-off {w(on) - w(off):+.1f} ms")
