#!/usr/bin/env bash
# Bildschirmfoto der Test-VM als PNG: build-aux/vm/shot.sh ziel.png
set -euo pipefail
out=${1:?Ziel-PNG angeben}
virsh -c qemu:///session screenshot fedora44-wor "$out.ppm" >/dev/null
magick "$out.ppm" "$out" && rm "$out.ppm" && echo "$out"
