#!/usr/bin/env bash
# Screenshot of the test VM as PNG: build-aux/vm/shot.sh target.png
set -euo pipefail
out=${1:?specify target PNG}
virsh -c qemu:///session screenshot fedora44-wor "$out.ppm" >/dev/null
magick "$out.ppm" "$out" && rm "$out.ppm" && echo "$out"
