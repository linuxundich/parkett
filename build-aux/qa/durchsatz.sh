#!/bin/sh
# Läuft IM Terminal, das gemessen wird: schreibt 3 Mio. Zeilen bzw. eine farbige 20-MB-Datei
# ins Terminal und notiert die Dauer in ~/qa-durchsatz.txt (eine Zeile je Lauf).
#   sh durchsatz.sh NAME     NAME = z. B. parkett, ptyxis
name=${1:-unbekannt}
f=$HOME/qa-farbig.txt
[ -f "$f" ] || python3 -c '
import random
random.seed(1)
c=[31,32,33,34,35,36]
with open("'"$f"'","w") as o:
    while o.tell() < 20_000_000:
        o.write("".join("\033[%dm%s\033[0m " % (random.choice(c), "wort%d" % random.randint(0,9999)) for _ in range(12)) + "\n")
'
t0=$(date +%s.%N); seq 1 3000000; t1=$(date +%s.%N)
cat "$f"; t2=$(date +%s.%N)
awk -v n="$name" -v a="$t0" -v b="$t1" -v c="$t2" 'BEGIN { printf "%s seq3M=%.2fs farbig20MB=%.2fs\n", n, b-a, c-b }' >> "$HOME/qa-durchsatz.txt"
