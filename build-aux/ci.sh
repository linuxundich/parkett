#!/usr/bin/env bash
# Checks before every commit/release, no window needed:
#   formatting, Clippy (warnings = errors), unit tests, cargo audit, Meson build and Meson tests
#   (.desktop, metainfo, GSettings schema, cargo test).
#   build-aux/ci.sh
set -euo pipefail
cd "$(dirname "$0")/.."
step() { printf '\n==> %s\n' "$*"; }

step "cargo fmt --check"
cargo fmt --all -- --check

step "cargo clippy (warnings are errors)"
cargo clippy --all-targets --quiet -- -D warnings

step "cargo test"
cargo test --quiet

# Known vulnerabilities in dependencies (RustSec); needs network for the database
step "cargo audit"
if cargo audit --version >/dev/null 2>&1; then
    cargo audit --quiet
else
    echo "skipped: cargo-audit missing (cargo install cargo-audit --locked)"
fi

step "meson (temporary build directory)"
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
meson setup "$dir" --prefix="$dir/install" >/dev/null
meson compile -C "$dir" >/dev/null
meson test -C "$dir" --print-errorlogs
meson install -C "$dir" --quiet >/dev/null
test -x "$dir/install/bin/parkett"
echo
echo "CI: all green"
