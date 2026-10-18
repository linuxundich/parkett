# Lizenzen

Parkett steht unter der **GPL-3.0-or-later** (`LICENSE`, Cargo, Meson, Metainfo, Über-Dialog).
Geprüft am 2026-10-19 mit `cargo metadata` und `pacman -Qi`.

## Eigene Teile

| Teil | Lizenz | Hinweis |
|---|---|---|
| Quellcode (`core/`, `app/`) | GPL-3.0-or-later | SPDX-Kennung am Dateianfang |
| App-Icon `de.linuxundich.Parkett.svg`, symbolisches Icon | GPL-3.0-or-later | eigene Arbeit (`icon/gen_icon.py`) |
| Symbole `parkett-split-right/down-symbolic.svg` | GPL-3.0-or-later | eigene Arbeit |
| Metainfo `*.metainfo.xml` | CC0-1.0 | wie in der Datei angegeben (Flathub/AppStream-Vorgabe) |

## Übernommene Daten

| Teil | Herkunft | Lizenz |
|---|---|---|
| 14 Farbschemata `data/palettes/*.palette` | Ptyxis `src/palettes/` (Commit `ecc97d39`) | GPL-3.0-or-later (Ptyxis); viele ursprünglich aus Gogh (MIT), Hinweis am Dateiende |
| Mockups `mockups/shared/icons/` (nur Doku, nicht im Paket) | Adwaita-Icons | CC-BY-SA-3.0 / LGPL-3.0, vermerkt in `icons.css` |

## Rust-Abhängigkeiten (77 Pakete)

| Anzahl | Lizenz | Beispiele |
|---|---|---|
| 38 | MIT OR Apache-2.0 | serde, toml, futures, hashbrown, bitflags |
| 31 | MIT | gtk4, libadwaita, vte4, glib, gio, cairo-rs, pango |
| 4 | Apache-2.0 OR MIT | indexmap, autocfg |
| 1 | Unlicense OR MIT | memchr |
| 1 | (MIT OR Apache-2.0) AND Unicode-3.0 | unicode-ident |
| 1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | io-lifetimes |
| 1 | Apache-2.0 WITH LLVM-exception | target-lexicon (nur beim Bauen) |

Alle sind mit GPL-3.0 vereinbar (Apache-2.0 ist GPLv3-kompatibel, nicht GPLv2-kompatibel – für
Parkett ohne Belang).

## Systembibliotheken (dynamisch gelinkt)

| Bibliothek | Lizenz |
|---|---|
| VTE (gtk4) 0.84 | LGPL-3.0-or-later |
| GTK 4.24 | LGPL-2.1-or-later |
| libadwaita 1.10 | LGPL-2.1-or-later |
| GLib, Pango | LGPL-2.1-or-later / LGPL-2.0-or-later |

## Folgen für die Weitergabe

- Binärpakete müssen den Quelltext anbieten (GPL) – beim Arch-Paket aus dem eigenen Repo
  erfüllt durch den Git-Klon; bei einer Veröffentlichung auf GitHub durch das öffentliche Repo.
- Die Farbschemata aus Ptyxis bleiben GPL-3.0-or-later; die Herkunft steht in
  `data/palettes/README.md`.
- Keine Bestandteile mit Copyleft-Unverträglichkeit gefunden.
