#!/usr/bin/env bash
# Rauchtest in der Test-VM fedora44-wor: baut und startet Parkett (deploy.sh) und steuert es
# über die D-Bus-Aktionen des Fensters (org.gtk.Actions). Prüft Prozesse und Protokoll.
#   build-aux/vm/smoke.sh        Ausgabe PASS/FAIL je Prüfung, Exit-Code 1 bei Fehlern
set -euo pipefail
cd "$(dirname "$0")/../.."
SSH=(ssh -i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p 2223 testt@localhost)
fail=0
check() { if [[ "$2" == "$3" ]]; then echo "PASS  $1 ($2)"; else echo "FAIL  $1: erwartet $3, ist $2"; fail=1; fi; }
remote() { "${SSH[@]}" "export XDG_RUNTIME_DIR=/run/user/\$(id -u); $*"; }
act() { remote "gdbus call --session --dest de.linuxundich.Parkett --object-path /de/linuxundich/Parkett/window/1 --method org.gtk.Actions.Activate $1 \"[]\" \"{}\" >/dev/null"; sleep 0.5; }
shells() { remote 'pgrep -P $(pidof parkett) | wc -l'; }
# Zeiger aus dem Weg: liegt er auf einer Einblendung, hält libadwaita deren Ablauf an
pointer_away() { virsh -c qemu:///session qemu-monitor-command fedora44-wor '{"execute":"input-send-event","arguments":{"events":[{"type":"abs","data":{"axis":"x","value":500}},{"type":"abs","data":{"axis":"y","value":500}}]}}' >/dev/null; }

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
sleep 2   # Sitzung wird 1,5 s nach der letzten Änderung gespeichert
pids=$(remote 'pgrep -P $(pidof parkett) | tr "\n" " "')
remote 'kill -TERM $(pidof parkett)'
sleep 2
check "Parkett beendet" "$(remote 'pidof parkett >/dev/null && echo läuft || echo beendet')" beendet
check "keine verwaisten Shells" "$(remote "n=0; for p in $pids; do kill -0 \$p 2>/dev/null && n=\$((n+1)); done; echo \$n")" 0
remote 'systemd-run --user -q --collect -u app-gnome-de.linuxundich.Parkett-smoke ~/.local/bin/parkett'
sleep 2
check "Sitzung wiederhergestellt" "$(shells)" 3
remote 'kill -TERM $(pidof parkett)'
sleep 1
problems=$(remote "journalctl --user --since=@$since --no-pager -o cat | grep -cE 'panicked|Gtk-CRITICAL|Adwaita-CRITICAL|GLib-CRITICAL|Gtk-WARNING' || true")
check "keine Panics/CRITICAL/Gtk-Warnungen im Protokoll" "$problems" 0
exit $fail
