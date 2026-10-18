#!/usr/bin/env bash
# Kopiert den Quellbaum in die Test-VM fedora44-wor, baut dort (release), installiert
# nach ~/.local (Binary, .desktop, Icons) und startet Parkett so, wie GNOME Shell es
# täte: als systemd-User-Unit app-gnome-<app-id>-<pid>.scope mit WAYLAND_DISPLAY.
#   build-aux/vm/deploy.sh          bauen, installieren, starten
#   NO_RUN=1 build-aux/vm/deploy.sh nur bauen und installieren
# Voraussetzung in der VM: rust, cargo, gtk4-devel, libadwaita-devel, vte291-gtk4-devel.
set -euo pipefail
cd "$(dirname "$0")/../.."
SSHOPT=(-i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p 2223)
rsync -a --delete --exclude target --exclude .git -e "ssh ${SSHOPT[*]}" ./ testt@localhost:parkett/
ssh "${SSHOPT[@]}" testt@localhost NO_RUN="${NO_RUN:-0}" bash -s <<'REMOTE'
set -euo pipefail
cd ~/parkett
cargo build --release 2>&1 | tail -1
install -Dm755 target/release/parkett ~/.local/bin/parkett
install -Dm644 data/de.linuxundich.Parkett.desktop ~/.local/share/applications/de.linuxundich.Parkett.desktop
sed -i "s|^Exec=parkett|Exec=$HOME/.local/bin/parkett|" ~/.local/share/applications/de.linuxundich.Parkett.desktop
mkdir -p ~/.local/share/icons && cp -r data/icons/hicolor ~/.local/share/icons/
update-desktop-database ~/.local/share/applications 2>/dev/null || true
[[ "$NO_RUN" == 1 ]] && exit 0
export XDG_RUNTIME_DIR=/run/user/$(id -u)
pkill -x parkett || true; sleep 0.5
systemd-run --user -q --collect -u "app-gnome-de.linuxundich.Parkett-$$" ~/.local/bin/parkett
sleep 1.5
A=$(pidof parkett); echo "läuft: pid $A, $(tr '\0' '\n' < /proc/$A/environ | grep -E '^WAYLAND_DISPLAY=')"
REMOTE
