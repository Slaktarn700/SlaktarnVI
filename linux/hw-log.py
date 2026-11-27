#!/usr/bin/env python3
"""Logs GPU and CPU once a second while the game runs; read-only (sysfs, /proc).

    hw-log.py <out.tsv> [seconds to wait for the game]

Per line: time, GPU busy %, VRAM used and GTT used (MiB), GPU shader clock
(MHz), GPU edge/junction temperature, GPU power (W), CPU busy % (all cores),
CPU temperature (Tctl), the game's CPU % (100 = one core), the game's own
memory as the GPU driver counts it (/proc/<pid>/fdinfo: resident in VRAM, in
GTT, evicted from VRAM, requested in VRAM; MiB), and its busiest threads
(name:% ...). Ends when the game has gone (or never came).
"""
import glob
import os
import sys
import time

out = open(sys.argv[1], "w", buffering=1)
wait = float(sys.argv[2]) if len(sys.argv) > 2 else 120


def read(path, default=""):
    try:
        with open(path) as f:
            return f.read().strip()
    except OSError:
        return default


# The discrete card: the amdgpu device with the most VRAM.
cards = [c for c in glob.glob("/sys/class/drm/card[0-9]") if read(f"{c}/device/vendor") == "0x1002"]
card = max(cards, key=lambda c: int(read(f"{c}/device/mem_info_vram_total", "0") or 0))
dev = f"{card}/device"
gpu_hwmon = (glob.glob(f"{dev}/hwmon/hwmon*") or [""])[0]
cpu_hwmon = next((h for h in glob.glob("/sys/class/hwmon/hwmon*") if read(f"{h}/name") == "k10temp"), "")


def temp_by_label(hwmon, label):
    for f in glob.glob(f"{hwmon}/temp*_label"):
        if read(f) == label:
            return int(read(f.replace("_label", "_input"), "0")) / 1000
    return 0.0


def sclk():
    for line in read(f"{dev}/pp_dpm_sclk").splitlines():
        if line.endswith("*"):
            return line.split()[1]
    return "?"


def cpu_times():
    f = read("/proc/stat").splitlines()[0].split()[1:]
    v = list(map(int, f))
    idle = v[3] + v[4]
    return sum(v), idle


def game_pid():
    for p in os.listdir("/proc"):
        if p.isdigit():
            try:
                with open(f"/proc/{p}/cmdline", "rb") as f:
                    cmd = f.read()
            except OSError:
                continue
            if (b"valhalla.exe" in cmd or b"seger.exe" in cmd or b"laurel.exe" in cmd or b"victory.exe" in cmd) and b"run" in cmd and read(f"/proc/{p}/comm") == "main":
                return int(p)
    return None


def threads(pid):
    t = {}
    for task in glob.glob(f"/proc/{pid}/task/*"):
        stat = read(f"{task}/stat")
        if not stat:
            continue
        name = stat[stat.index("(") + 1 : stat.rindex(")")]
        rest = stat[stat.rindex(")") + 2 :].split()
        t[task] = (name, int(rest[11]) + int(rest[12]))
    return t


def drm_memory(pid):
    """The game's GPU memory by the driver's fdinfo, per DRM client once:
    (resident VRAM, resident GTT, evicted VRAM, requested VRAM) in MiB;
    '?' where the kernel does not tell."""
    clients = {}
    for fd in glob.glob(f"/proc/{pid}/fdinfo/*"):
        text = read(fd)
        if "drm-driver:" not in text:
            continue
        keys = {}
        for line in text.splitlines():
            k, _, v = line.partition(":")
            keys[k.strip()] = v.strip()
        clients[keys.get("drm-client-id", fd)] = keys

    def mib(*names):
        total, seen = 0, False
        for keys in clients.values():
            for name in names:
                if name in keys:
                    parts = keys[name].split()
                    unit = {"KiB": 1 << 10, "MiB": 1 << 20, "GiB": 1 << 30}.get(parts[1] if len(parts) > 1 else "", 1)
                    total += int(parts[0]) * unit
                    seen = True
                    break
        return str(total >> 20) if seen else "?"

    return (mib("drm-resident-vram", "drm-memory-vram"), mib("drm-resident-gtt", "drm-memory-gtt"),
            mib("amd-evicted-vram"), mib("amd-requested-vram"))


hz = os.sysconf("SC_CLK_TCK")
start = time.time()
pid = None
while pid is None and time.time() - start < wait:
    pid = game_pid()
    time.sleep(1)
if pid is None:
    sys.exit("no game process")
out.write("t\tgpu%\tvram_MiB\tgtt_MiB\tsclk\tgpu_edge\tgpu_junction\tgpu_W\tcpu%\tcpu_C\tgame%\tgame_vram\tgame_gtt\tevicted_vram\trequested_vram\tthreads\n")
prev_cpu = cpu_times()
prev_t = threads(pid)
prev_time = time.time()
while os.path.exists(f"/proc/{pid}"):
    time.sleep(1)
    now = time.time()
    dt = now - prev_time
    total, idle = cpu_times()
    cpu = 100 * (1 - (idle - prev_cpu[1]) / max(1, total - prev_cpu[0]))
    t = threads(pid)
    use = []
    for k, (name, ticks) in t.items():
        if k in prev_t:
            use.append((100 * (ticks - prev_t[k][1]) / hz / dt, name))
    use.sort(reverse=True)
    game = sum(u for u, _ in use)
    top = " ".join(f"{n}:{u:.0f}" for u, n in use[:8] if u >= 5)
    power = int(read(f"{gpu_hwmon}/power1_average", read(f"{gpu_hwmon}/power1_input", "0")) or 0) / 1e6
    out.write(
        f"{now - start:.0f}\t{read(f'{dev}/gpu_busy_percent', '?')}\t"
        f"{int(read(f'{dev}/mem_info_vram_used', '0')) >> 20}\t{int(read(f'{dev}/mem_info_gtt_used', '0')) >> 20}\t"
        f"{sclk()}\t{temp_by_label(gpu_hwmon, 'edge'):.0f}\t{temp_by_label(gpu_hwmon, 'junction'):.0f}\t{power:.0f}\t"
        f"{cpu:.0f}\t{temp_by_label(cpu_hwmon, 'Tctl'):.0f}\t{game:.0f}\t{chr(9).join(drm_memory(pid))}\t{top}\n"
    )
    prev_cpu, prev_t, prev_time = (total, idle), t, now
