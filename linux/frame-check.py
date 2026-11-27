#!/usr/bin/env python3
"""Flags captured frames that look broken: mostly black, blown out, or with
many magenta / pure green pixels (uninitialised or wrongly read data).
    frame-check.py <capture folder> [more folders...]"""
import sys, glob, os
from PIL import Image

def check(path):
    im = Image.open(path).convert("RGB")
    w, h = im.size
    small = im.resize((w // 4, h // 4))
    px = list(small.getdata())
    n = len(px)
    lum = [0.2126 * r + 0.7152 * g + 0.0722 * b for r, g, b in px]
    black = sum(1 for l in lum if l < 6) / n
    white = sum(1 for l in lum if l > 250) / n
    magenta = sum(1 for r, g, b in px if r > 150 and b > 150 and g < 60) / n
    green = sum(1 for r, g, b in px if g > 200 and r < 40 and b < 40) / n
    mean = sum(lum) / n
    flags = []
    if black > 0.85: flags.append(f"black {black:.0%}")
    if white > 0.30: flags.append(f"blown out {white:.0%}")
    if magenta > 0.01: flags.append(f"magenta {magenta:.1%}")
    if green > 0.01: flags.append(f"green {green:.1%}")
    return mean, black, flags

for folder in sys.argv[1:]:
    for path in sorted(glob.glob(os.path.join(folder, "frame-*.png")), key=lambda p: int(os.path.basename(p)[6:-4]) if os.path.basename(p)[6:-4].isdigit() else 0):
        mean, black, flags = check(path)
        print(f"{path}: mean {mean:5.1f}, black {black:4.0%} {'  <-- ' + ', '.join(flags) if flags else ''}")
