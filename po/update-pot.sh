#!/usr/bin/env bash
# Erzeugt po/parkett.pot aus den Dateien in POTFILES.in und gleicht alle po/*.po damit ab.
# Braucht gettext und python3. Muster aus Tastenspur: Rust-Dateien über extract_rust.py
# (xgettext übersieht Texte in Makros), die übrigen über xgettext.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
mapfile -t files < <(grep -v '^#' po/POTFILES.in | grep -v '^$')
rust=(); other=()
for f in "${files[@]}"; do
  case "$f" in *.rs) rust+=("$f") ;; *) other+=("$f") ;; esac
done
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
python3 po/extract_rust.py "$tmp/rust.pot" "${rust[@]}"
xgettext --from-code=UTF-8 --add-comments=Translators -o "$tmp/other.pot" "${other[@]}"
msgcat --use-first "$tmp/other.pot" "$tmp/rust.pot" -o "$tmp/all.pot"
xgettext --from-code=UTF-8 --no-wrap --package-name=parkett \
  --msgid-bugs-address=https://github.com/linuxundich/parkett/issues \
  -o po/parkett.pot "$tmp/all.pot"
shopt -s nullglob
for po in po/*.po; do
  msgmerge --quiet --no-wrap --update --backup=none "$po" po/parkett.pot
done
echo "po/parkett.pot: $(grep -c '^msgid ' po/parkett.pot) Einträge"
