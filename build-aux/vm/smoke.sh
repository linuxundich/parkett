#!/usr/bin/env bash
# Rauchtest in der Test-VM: baut und startet Parkett (deploy.sh) und steuert es über die
# D-Bus-Aktionen des Fensters (org.gtk.Actions) und Tasten (virsh send-key). Prüft Prozesse
# und Protokoll.
#   build-aux/vm/smoke.sh                                  fedora44-wor (SSH-Port 2223)
#   VM_DOM=ubuntu26.04 VM_PORT=2222 build-aux/vm/smoke.sh  Ubuntu 26.04
# Ausgabe PASS/FAIL je Prüfung, Exit-Code 1 bei Fehlern.
set -euo pipefail
cd "$(dirname "$0")/../.."
VM_DOM=${VM_DOM:-fedora44-wor}
export VM_PORT=${VM_PORT:-2223}
SSH=(ssh -i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p "$VM_PORT" testt@localhost)
fail=0
check() { if [[ "$2" == "$3" ]]; then echo "PASS  $1 ($2)"; else echo "FAIL  $1: erwartet $3, ist $2"; fail=1; fi; }
remote() { "${SSH[@]}" "export XDG_RUNTIME_DIR=/run/user/\$(id -u); $*"; }
act() { remote "gdbus call --session --dest de.linuxundich.Parkett --object-path /de/linuxundich/Parkett/window/1 --method org.gtk.Actions.Activate $1 \"[]\" \"{}\" >/dev/null"; sleep 0.5; }
shells() { remote 'pgrep -P $(pidof parkett) | wc -l'; }
keys() { for k in "$@"; do virsh -c qemu:///session send-key "$VM_DOM" --holdtime 50 $k >/dev/null; sleep 0.1; done; }
# Zeiger aus dem Weg: liegt er auf einer Einblendung, hält libadwaita deren Ablauf an
pointer_away() { virsh -c qemu:///session qemu-monitor-command "$VM_DOM" '{"execute":"input-send-event","arguments":{"events":[{"type":"abs","data":{"axis":"x","value":500}},{"type":"abs","data":{"axis":"y","value":500}}]}}' >/dev/null; }

since=$(remote 'date +%s')
# Sitzung: ohne gespeicherte Sitzung starten, Wiederherstellen am Ende prüfen
remote 'GSETTINGS_SCHEMA_DIR=$HOME/.local/share/glib-2.0/schemas gsettings set de.linuxundich.Parkett restore-session true 2>/dev/null; pkill -x parkett; rm -f /home/testt/.local/state/parkett/session.json' || true
build-aux/vm/deploy.sh >/dev/null
sleep 1
pointer_away
check "Start mit einer Shell" "$(shells)" 1
act split-right; act split-down
check "Teilen rechts/unten" "$(shells)" 3
act toggle-zoom; act toggle-zoom; act equalize; act layout-mode; act layout-mode
check "Zoom, Angleichen, Layout-Modus" "$(shells)" 3
act new-tab; act new-tab
check "Zwei neue Reiter" "$(shells)" 5
act close-tab
sleep 10
check "Reiter geschlossen, Rückgängig-Frist abgelaufen" "$(shells)" 4
act close-pane 2>/dev/null || true
sleep 10
check "Bereich geschlossen, Frist abgelaufen" "$(shells)" 3
# QA-01: F10 gehört dem Programm im Terminal (htop beendet sich), nicht dem Hauptmenü
if remote 'command -v htop' >/dev/null; then
  keys KEY_H KEY_T KEY_O KEY_P KEY_ENTER
  sleep 2
  check "htop gestartet" "$(remote 'pgrep -x htop >/dev/null && echo läuft || echo fehlt')" läuft
  keys KEY_F10
  sleep 1
  check "F10 beendet htop" "$(remote 'pgrep -x htop >/dev/null && echo läuft || echo beendet')" beendet
fi
# QA-03: Profil „Neu starten“ – geschlossener Reiter darf nach der Frist nichts neu starten
remote 'mkdir -p ~/.config/parkett/profiles; printf "name = \"Rauchtest\"\ncommand = \"sleep 1001\"\nexit_action = \"restart\"\n" > ~/.config/parkett/profiles/smoke-neustart.toml'
remote 'timeout 10 ~/.local/bin/parkett --tab -p smoke-neustart'
sleep 2
check "Reiter mit Neustart-Profil" "$(remote 'pgrep -fx "sleep 1001" | wc -l')" 1
act close-tab
sleep 11
check "kein Neustart nach Schließen" "$(remote 'pgrep -fx "sleep 1001" | wc -l')" 0
remote 'rm -f ~/.config/parkett/profiles/smoke-neustart.toml'
sleep 2   # Sitzung wird 1,5 s nach der letzten Änderung gespeichert
pids=$(remote 'pgrep -P $(pidof parkett) | tr "\n" " "')
remote 'kill -TERM $(pidof parkett)'
sleep 2
check "Parkett beendet" "$(remote 'pidof parkett >/dev/null && echo läuft || echo beendet')" beendet
check "keine verwaisten Shells" "$(remote "n=0; for p in $pids; do kill -0 \$p 2>/dev/null && n=\$((n+1)); done; echo \$n")" 0
remote 'systemd-run --user -q --collect -u app-gnome-de.linuxundich.Parkett-smoke ~/.local/bin/parkett'
sleep 2
check "Sitzung wiederhergestellt" "$(shells)" 3
# QA-07: Markup in Titeln bleibt Text (AT-SPI-Name des Reiters)
remote 'timeout 10 ~/.local/bin/parkett --tab -t "<b>A & B</b>"'
sleep 1
check "Reitertitel mit Markup wörtlich (QA-07)" \
  "$(remote 'python3 ~/parkett/build-aux/vm/atspi.py tabs' | grep -cx '[0-9]* <b>A & B</b>')" 1
# QA-09: nach dem Hauptmenü hat wieder das Terminal den Fokus
remote 'python3 ~/parkett/build-aux/vm/atspi.py press Hauptmenü toggle_button'
sleep 1
keys KEY_ESC
sleep 1
check "Fokus nach dem Hauptmenü im Terminal (QA-09)" "$(remote 'python3 ~/parkett/build-aux/vm/atspi.py focused')" "terminal: Terminal"
remote 'kill -TERM $(pidof parkett)'
sleep 1
# GLib schreibt unter systemd strukturiert ins Journal: Das Präfix „Gtk-CRITICAL“ fehlt dann
# in „-o cat“ (R-02). Daher jeden Eintrag mit GLIB_DOMAIN zählen (GLib, GTK, libadwaita,
# Parkett selbst); Debug-Meldungen sind ohne G_MESSAGES_DEBUG ohnehin aus. Mesa hat keine.
problems=$(remote "journalctl --user -u 'app-gnome-de.linuxundich.Parkett-*' --since=@$since --no-pager -o json" |
  python3 -c 'import json,sys; print(sum(1 for l in sys.stdin if (e:=json.loads(l)).get("GLIB_DOMAIN") or "panicked" in str(e.get("MESSAGE",""))))')
check "keine GLib-Meldungen oder Panics im Protokoll" "$problems" 0
exit $fail
