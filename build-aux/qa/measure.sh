#!/usr/bin/env bash
# From the host: memory (RSS), threads, child processes and CPU time of Parkett in the VM.
#   build-aux/qa/measure.sh            measure once
#   build-aux/qa/measure.sh cpu 30     CPU share over 30 s
set -euo pipefail
SSH=(ssh -i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p 2223 testt@localhost)
mode=${1:-rss}
"${SSH[@]}" MODE="$mode" SECS="${2:-30}" bash -s <<'REMOTE'
A=$(pidof parkett) || { echo "parkett is not running"; exit 1; }
if [[ $MODE == cpu ]]; then
  t() { awk '{print $14+$15}' /proc/$A/stat; }
  hz=$(getconf CLK_TCK); a=$(t); sleep "$SECS"; b=$(t)
  awk -v a="$a" -v b="$b" -v hz="$hz" -v s="$SECS" 'BEGIN { printf "CPU %.2f %% over %d s\n", 100*(b-a)/hz/s, s }'
else
  rss=$(awk '/VmRSS/ {print $2}' /proc/$A/status); thr=$(awk '/Threads/ {print $2}' /proc/$A/status)
  echo "RSS $((rss/1024)) MB · Threads $thr · Shells $(pgrep -P "$A" | wc -l)"
fi
REMOTE
