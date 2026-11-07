#!/usr/bin/env bash
# Called by Meson: builds Parkett with cargo and copies the binary to the place
# Meson expects. Arguments: SOURCE_ROOT BUILD_ROOT BUILDTYPE OUTPUT LOCALEDIR
set -euo pipefail
source_root="$1"; build_root="$2"; buildtype="$3"; output="$4"
# Translations from the installed location
export PARKETT_LOCALEDIR="$5"

export CARGO_TARGET_DIR="$build_root/target"
# Installed build: schema from the standard path, not from the build directory
export PARKETT_INSTALLED=1

if [[ "$buildtype" == "debug" ]]; then
  profile=debug; flags=()
else
  profile=release; flags=(--release)
fi
# PARKETT_FEATURES=ghostty builds the experimental libghostty engine (docs/LIBGHOSTTY.md)
features=()
[[ -n "${PARKETT_FEATURES:-}" ]] && features=(--features "$PARKETT_FEATURES")
cargo build "${flags[@]}" "${features[@]}" --manifest-path "$source_root/Cargo.toml" -p parkett
cp "$CARGO_TARGET_DIR/$profile/parkett" "$output"
