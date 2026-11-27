#!/bin/bash
# Starts the Valhalla launcher (by Slaktarn).
exec python3 -I "$(dirname "$(readlink -f "$0")")/launcher/valhalla.py" "$@"
