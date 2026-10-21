#!/usr/bin/env bash
# Prüfungen vor jedem Commit/Release, ohne Fenster:
#   Formatierung, Clippy (Warnungen = Fehler), Unit-Tests, cargo audit, Meson-Build und Meson-Tests
#   (.desktop, Metainfo, GSettings-Schema, cargo test).
#   build-aux/ci.sh
set -euo pipefail
cd "$(dirname "$0")/.."
step() { printf '\n==> %s\n' "$*"; }

step "cargo fmt --check"
cargo fmt --all -- --check

step "cargo clippy (Warnungen sind Fehler)"
cargo clippy --all-targets --quiet -- -D warnings

step "cargo test"
cargo test --quiet

# Bekannte Sicherheitslücken in Abhängigkeiten (RustSec); braucht Netz für die Datenbank
step "cargo audit"
if cargo audit --version >/dev/null 2>&1; then
    cargo audit --quiet
else
    echo "übersprungen: cargo-audit fehlt (cargo install cargo-audit --locked)"
fi

step "meson (temporärer Build-Ordner)"
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
meson setup "$dir" --prefix="$dir/install" >/dev/null
meson compile -C "$dir" >/dev/null
meson test -C "$dir" --print-errorlogs
meson install -C "$dir" --quiet >/dev/null
test -x "$dir/install/bin/parkett"
echo
echo "CI: alles grün"
