#!/bin/bash
# Throughput, idle CPU and memory of the two terminal engines (docs/LIBGHOSTTY.md).
#   cargo build --release -p parkett --features ghostty
#   dbus-run-session -- build-aux/ghostty-demo-headless.sh build-aux/bench/engines.sh vte|ghostty seq|color|idle
# Needs GSETTINGS_BACKEND=memory and GSETTINGS_SCHEMA_DIR=<target/release/build/parkett-*/out/schemas>, and
# libghostty-vt on LD_LIBRARY_PATH. Prints the time the shell needed to write the output into the
# terminal (the terminal has to keep up with it), CPU ticks and resident memory.
cd "$(dirname "$0")/../.."
ENGINE=$1; W=$2
T=$(mktemp -d)
case $W in
  seq)    CMD="date +%s.%N > $T/start; seq 1 3000000; date +%s.%N > $T/end; sleep 20" ;;
  color)  CMD="sh build-aux/bench/gen_color.sh $T" ;;
  idle)   CMD="date +%s.%N > $T/start; date +%s.%N > $T/end; sleep 40" ;;
esac
export PARKETT_BACKEND=$ENGINE
./target/release/parkett -e "sh -c '$CMD'" >/dev/null 2>&1 &
P=$!
for i in $(seq 200); do [ -s $T/end ] && break; sleep 0.2; done
sleep 1
cpu0=$(awk '{print $14+$15}' /proc/$P/stat)
if [ $W = idle ]; then sleep 15; fi
cpu1=$(awk '{print $14+$15}' /proc/$P/stat)
rss=$(awk '/VmRSS/{print $2}' /proc/$P/status)
dur=$(python3 -c "print(round(float(open('$T/end').read())-float(open('$T/start').read()),2))" 2>/dev/null)
tck=$(getconf CLK_TCK)
echo "engine=$ENGINE workload=$W duration=${dur}s idle_cpu_ticks_15s=$((cpu1-cpu0)) (tck=$tck) rss_kb=$rss total_cpu_ticks=$(awk '{print $14+$15}' /proc/$P/stat)"
kill $P 2>/dev/null; wait $P 2>/dev/null; rm -rf $T
