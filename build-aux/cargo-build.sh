#!/usr/bin/env bash
# Von Meson aufgerufen: baut Parkett mit cargo und kopiert das Programm an die Stelle,
# die Meson erwartet. Argumente: QUELLORDNER BUILDORDNER BUILDTYP AUSGABE
set -euo pipefail
source_root="$1"; build_root="$2"; buildtype="$3"; output="$4"

export CARGO_TARGET_DIR="$build_root/target"
# Installierter Build: Schema aus dem Standardpfad, nicht aus dem Build-Ordner
export PARKETT_INSTALLED=1

if [[ "$buildtype" == "debug" ]]; then
  profile=debug; flags=()
else
  profile=release; flags=(--release)
fi
cargo build "${flags[@]}" --manifest-path "$source_root/Cargo.toml" -p parkett
cp "$CARGO_TARGET_DIR/$profile/parkett" "$output"
