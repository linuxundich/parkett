#!/usr/bin/env bash
# Vom Host aus: Speicher (RSS), Threads, Kindprozesse und CPU-Zeit von Parkett in der VM.
#   build-aux/qa/messen.sh            einmal messen
#   build-aux/qa/messen.sh cpu 30     CPU-Anteil über 30 s
set -euo pipefail
SSH=(ssh -i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p 2223 testt@localhost)
mode=${1:-rss}
"${SSH[@]}" MODE="$mode" SECS="${2:-30}" bash -s <<'REMOTE'
A=$(pidof parkett) || { echo "parkett läuft nicht"; exit 1; }
if [[ $MODE == cpu ]]; then
  t() { awk '{print $14+$15}' /proc/$A/stat; }
  hz=$(getconf CLK_TCK); a=$(t); sleep "$SECS"; b=$(t)
  awk -v a="$a" -v b="$b" -v hz="$hz" -v s="$SECS" 'BEGIN { printf "CPU %.2f %% über %d s\n", 100*(b-a)/hz/s, s }'
else
  rss=$(awk '/VmRSS/ {print $2}' /proc/$A/status); thr=$(awk '/Threads/ {print $2}' /proc/$A/status)
  echo "RSS $((rss/1024)) MB · Threads $thr · Shells $(pgrep -P "$A" | wc -l)"
fi
REMOTE
