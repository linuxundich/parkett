#!/usr/bin/env bash
# Smoke test in the test VM: builds and starts Parkett (deploy.sh) and drives it via the
# window's D-Bus actions (org.gtk.Actions) and keys (virsh send-key). Checks processes
# and the log.
#   build-aux/vm/smoke.sh                                  fedora44-wor (SSH-Port 2223)
#   VM_DOM=ubuntu26.04 VM_PORT=2222 build-aux/vm/smoke.sh  Ubuntu 26.04
# Prints PASS/FAIL per check, exit code 1 on failures.
set -euo pipefail
cd "$(dirname "$0")/../.."
VM_DOM=${VM_DOM:-fedora44-wor}
export VM_PORT=${VM_PORT:-2223}
SSH=(ssh -i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p "$VM_PORT" testt@localhost)
fail=0
check() { if [[ "$2" == "$3" ]]; then echo "PASS  $1 ($2)"; else echo "FAIL  $1: expected $3, got $2"; fail=1; fi; }
remote() { "${SSH[@]}" "export XDG_RUNTIME_DIR=/run/user/\$(id -u); $*"; }
act() { remote "gdbus call --session --dest de.linuxundich.Parkett --object-path /de/linuxundich/Parkett/window/1 --method org.gtk.Actions.Activate $1 \"[]\" \"{}\" >/dev/null"; sleep 0.5; }
shells() { remote 'pgrep -P $(pidof parkett) | wc -l'; }
keys() { for k in "$@"; do virsh -c qemu:///session send-key "$VM_DOM" --holdtime 50 $k >/dev/null; sleep 0.1; done; }
# Move the pointer away: while it rests on a toast, libadwaita pauses its timeout
pointer_away() { virsh -c qemu:///session qemu-monitor-command "$VM_DOM" '{"execute":"input-send-event","arguments":{"events":[{"type":"abs","data":{"axis":"x","value":500}},{"type":"abs","data":{"axis":"y","value":500}}]}}' >/dev/null; }

since=$(remote 'date +%s')
# Session: start without a saved session, check restoring at the end
remote 'GSETTINGS_SCHEMA_DIR=$HOME/.local/share/glib-2.0/schemas gsettings set de.linuxundich.Parkett restore-session true 2>/dev/null; pkill -x parkett; rm -f /home/testt/.local/state/parkett/session.json' || true
build-aux/vm/deploy.sh >/dev/null
sleep 1
pointer_away
check "Start with one shell" "$(shells)" 1
act split-right; act split-down
check "Split right/down" "$(shells)" 3
act toggle-zoom; act toggle-zoom; act equalize; act layout-mode; act layout-mode
check "Zoom, equalize, layout mode" "$(shells)" 3
act new-tab; act new-tab
check "Two new tabs" "$(shells)" 5
act close-tab
sleep 10
check "Tab closed, undo period expired" "$(shells)" 4
act close-pane 2>/dev/null || true
sleep 10
check "Pane closed, period expired" "$(shells)" 3
# QA-01: F10 belongs to the program in the terminal (htop quits), not to the main menu
if remote 'command -v htop' >/dev/null; then
  keys KEY_H KEY_T KEY_O KEY_P KEY_ENTER
  sleep 2
  check "htop started" "$(remote 'pgrep -x htop >/dev/null && echo running || echo missing')" running
  keys KEY_F10
  sleep 1
  check "F10 quits htop" "$(remote 'pgrep -x htop >/dev/null && echo running || echo stopped')" stopped
fi
# QA-03: profile "Neu starten" (restart) – a closed tab must not restart anything after the period
remote 'mkdir -p ~/.config/parkett/profiles; printf "name = \"Rauchtest\"\ncommand = \"sleep 1001\"\nexit_action = \"restart\"\n" > ~/.config/parkett/profiles/smoke-neustart.toml'
remote 'timeout 10 ~/.local/bin/parkett --tab -p smoke-neustart'
sleep 2
check "Tab with restart profile" "$(remote 'pgrep -fx "sleep 1001" | wc -l')" 1
act close-tab
sleep 11
check "no restart after closing" "$(remote 'pgrep -fx "sleep 1001" | wc -l')" 0
remote 'rm -f ~/.config/parkett/profiles/smoke-neustart.toml'
sleep 2   # session is saved 1.5 s after the last change
pids=$(remote 'pgrep -P $(pidof parkett) | tr "\n" " "')
remote 'kill -TERM $(pidof parkett)'
sleep 2
check "Parkett stopped" "$(remote 'pidof parkett >/dev/null && echo running || echo stopped')" stopped
check "no orphaned shells" "$(remote "n=0; for p in $pids; do kill -0 \$p 2>/dev/null && n=\$((n+1)); done; echo \$n")" 0
remote 'systemd-run --user -q --collect -u app-gnome-de.linuxundich.Parkett-smoke ~/.local/bin/parkett'
sleep 2
check "Session restored" "$(shells)" 3
# QA-07: markup in titles stays text (AT-SPI name of the tab)
remote 'timeout 10 ~/.local/bin/parkett --tab -t "<b>A & B</b>"'
sleep 1
check "Tab title with markup kept literally (QA-07)" \
  "$(remote 'python3 ~/parkett/build-aux/vm/atspi.py tabs' | grep -cx '[0-9]* <b>A & B</b>')" 1
# QA-09: after the main menu the terminal has the focus again
remote 'python3 ~/parkett/build-aux/vm/atspi.py press Hauptmenü toggle_button'
sleep 1
keys KEY_ESC
sleep 1
check "Focus in the terminal after the main menu (QA-09)" "$(remote 'python3 ~/parkett/build-aux/vm/atspi.py focused')" "terminal: Terminal"
# QA-02: the floating title is hidden until the pointer is at the top of the pane. Checks
# the top right corner of the right pane by screenshot: plain without title, not plain with
# title. Assumes the window geometry after deploy.sh (960×640, centered on 1280×800, with
# tab bar).
act split-right
pointer_away
sleep 2.5   # title gone again after the focus change (1.5 s)
spread() { virsh -c qemu:///session screenshot "$VM_DOM" /tmp/parkett-smoke.ppm >/dev/null
  magick /tmp/parkett-smoke.ppm -crop 150x26+960+190 -colorspace gray -format '%[fx:standard_deviation]' info:; }
hidden=$(spread)
virsh -c qemu:///session qemu-monitor-command "$VM_DOM" '{"execute":"input-send-event","arguments":{"events":[{"type":"abs","data":{"axis":"x","value":25600}},{"type":"abs","data":{"axis":"y","value":8200}}]}}' >/dev/null
sleep 1
shown=$(spread)
rm -f /tmp/parkett-smoke.ppm
check "Pane title hides nothing, appears at the top (QA-02)" \
  "$(awk -v h="$hidden" -v s="$shown" 'BEGIN { print (h < 0.02 && s > 0.1) ? "ok" : "hidden " h ", shown " s }')" ok
remote 'kill -TERM $(pidof parkett)'
sleep 1
# Under systemd GLib writes structured entries to the journal: the prefix "Gtk-CRITICAL" is then
# missing in "-o cat" (R-02). So count every entry with GLIB_DOMAIN (GLib, GTK, libadwaita,
# Parkett itself); debug messages are off anyway without G_MESSAGES_DEBUG. Mesa sets none.
problems=$(remote "journalctl --user -u 'app-gnome-de.linuxundich.Parkett-*' --since=@$since --no-pager -o json" |
  python3 -c 'import json,sys; print(sum(1 for l in sys.stdin if (e:=json.loads(l)).get("GLIB_DOMAIN") or "panicked" in str(e.get("MESSAGE",""))))')
check "no GLib messages or panics in the log" "$problems" 0
exit $fail
