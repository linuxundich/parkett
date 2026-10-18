#!/usr/bin/env bash
# Kopiert den Quellbaum in die Test-VM fedora44-wor, baut und installiert dort mit Meson
# nach ~/.local (Programm, .desktop, Metainfo, GSettings-Schema, Icons) und startet Parkett
# so, wie GNOME Shell es täte: als systemd-User-Unit app-gnome-<app-id>-<pid> mit Wayland.
#   build-aux/vm/deploy.sh          bauen, installieren, starten
#   NO_RUN=1 build-aux/vm/deploy.sh nur bauen und installieren
#   PARKETT_ENV=VAR=wert …          eine Umgebungsvariable für den Start setzen
# Voraussetzung in der VM: rust, cargo, meson, gtk4-devel, libadwaita-devel, vte291-gtk4-devel.
set -euo pipefail
cd "$(dirname "$0")/../.."
SSHOPT=(-i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p 2223)
rsync -a --delete --exclude target --exclude _build --exclude _install --exclude .git \
  -e "ssh ${SSHOPT[*]}" ./ testt@localhost:parkett/
ssh "${SSHOPT[@]}" testt@localhost NO_RUN="${NO_RUN:-0}" PARKETT_ENV="${PARKETT_ENV:-}" bash -s <<'REMOTE'
set -euo pipefail
cd ~/parkett
# Die VM-Uhr kann springen; ninja hielte Ausgaben „aus der Zukunft“ für aktuell.
if [[ -d _build && -n "$(find _build -newermt '1 minute' -print -quit 2>/dev/null)" ]]; then
  echo "VM-Uhr gesprungen: _build wird neu angelegt"; rm -rf _build
fi
[[ -d _build ]] || meson setup _build --prefix="$HOME/.local" --buildtype=release >/dev/null
meson install -C _build --quiet 2>&1 | grep -E "error|Finished" || true
[[ "$NO_RUN" == 1 ]] && exit 0
export XDG_RUNTIME_DIR=/run/user/$(id -u)
pkill -x parkett || true; sleep 0.5
systemd-run --user -q --collect -u "app-gnome-de.linuxundich.Parkett-$$" ${PARKETT_ENV:+--setenv=$PARKETT_ENV} ~/.local/bin/parkett
sleep 1.5
A=$(pidof parkett); echo "läuft: pid $A, $(tr '\0' '\n' < /proc/$A/environ | grep -E '^WAYLAND_DISPLAY=')"
REMOTE
