#!/usr/bin/env bash
# Copies the source tree into the test VM (default fedora44-wor), builds and installs it there with
# Meson into ~/.local (binary, .desktop, metainfo, GSettings schema, icons) and starts Parkett
# the way GNOME Shell would: as systemd user unit app-gnome-<app-id>-<pid> on Wayland.
#   build-aux/vm/deploy.sh          build, install, start
#   NO_RUN=1 build-aux/vm/deploy.sh only build and install
#   PARKETT_ENV=VAR=value …         set one environment variable for the start
#   PARKETT_FEATURES=ghostty        build with the libghostty engine; needs libghostty-vt 1.0 in the VM at
#                                   ~/.local/ghostty-vt (copy it there; its .pc files get the VM's home)
# Required in the VM: rust, cargo, meson, gtk4-devel, libadwaita-devel, vte291-gtk4-devel.
set -euo pipefail
cd "$(dirname "$0")/../.."
# Other VM: VM_PORT=2222 (ubuntu26.04); default is fedora44-wor on 2223
SSHOPT=(-i "$HOME/.ssh/id_vmtest" -o IdentitiesOnly=yes -o BatchMode=yes -p "${VM_PORT:-2223}")
rsync -a --delete --exclude target --exclude _build --exclude _install --exclude .git \
  -e "ssh ${SSHOPT[*]}" ./ testt@localhost:parkett/
ssh "${SSHOPT[@]}" testt@localhost NO_RUN="${NO_RUN:-0}" PARKETT_ENV="${PARKETT_ENV:-}" PARKETT_FEATURES="${PARKETT_FEATURES:-}" bash -s <<'REMOTE'
set -euo pipefail
cd ~/parkett
# The VM clock can jump; ninja would consider outputs "from the future" up to date.
if [[ -d _build && -n "$(find _build -newermt '1 minute' -print -quit 2>/dev/null)" ]]; then
  echo "VM clock jumped: recreating _build"; rm -rf _build
fi
# A build with other features needs a fresh build directory (cargo does not see the change)
if [[ "$(cat _build/.features 2>/dev/null || true)" != "$PARKETT_FEATURES" ]]; then rm -rf _build; fi
if [[ -n "$PARKETT_FEATURES" ]]; then
  G=$HOME/.local/ghostty-vt
  sed -i "s|^prefix=.*|prefix=$G|" "$G"/lib/pkgconfig/*.pc
  export PARKETT_FEATURES PKG_CONFIG_PATH="$G/lib/pkgconfig" RUSTFLAGS="-C link-arg=-Wl,-rpath,$G/lib"
fi
[[ -d _build ]] || meson setup _build --prefix="$HOME/.local" --buildtype=release >/dev/null
meson install -C _build --quiet 2>&1 | grep -E "error|Finished" || true
printf %s "$PARKETT_FEATURES" > _build/.features
[[ "$NO_RUN" == 1 ]] && exit 0
export XDG_RUNTIME_DIR=/run/user/$(id -u)
pkill -x parkett || true; sleep 0.5
systemd-run --user -q --collect -u "app-gnome-de.linuxundich.Parkett-$$" ${PARKETT_ENV:+--setenv=$PARKETT_ENV} ~/.local/bin/parkett
sleep 1.5
A=$(pidof parkett); echo "running: pid $A, $(tr '\0' '\n' < /proc/$A/environ | grep -E '^WAYLAND_DISPLAY=')"
REMOTE
