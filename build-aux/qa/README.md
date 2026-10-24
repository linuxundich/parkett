# QA helpers (phase 5)

| File | Purpose |
|---|---|
| `throughput.sh` | run inside the terminal being measured: 3 million lines of `seq` and 20 MB of colored text, duration written to `~/qa-throughput.txt` |
| `measure.sh` | from the host: RSS/threads/shells or CPU share of Parkett in the VM |

Results and log: `docs/QA.md`.
