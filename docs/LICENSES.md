# Licenses

Parkett is licensed under the **GPL-3.0-or-later** (`LICENSE`, Cargo, Meson, metainfo, About dialog).
Checked on 2026-10-19 with `cargo metadata` and `pacman -Qi`.

## Own parts

| Part | License | Note |
|---|---|---|
| Source code (`core/`, `app/`) | GPL-3.0-or-later | SPDX identifier at the top of each file |
| App icon `de.linuxundich.Parkett.svg`, symbolic icon | GPL-3.0-or-later | own work (`icon/gen_icon.py`) |
| Icons `parkett-split-right/down-symbolic.svg` | GPL-3.0-or-later | own work |
| Metainfo `*.metainfo.xml` | CC0-1.0 | as stated in the file (Flathub/AppStream requirement) |

## Third-party data

| Part | Origin | License |
|---|---|---|
| 14 color schemes `data/palettes/*.palette` | Ptyxis `src/palettes/` (commit `ecc97d39`) | GPL-3.0-or-later (Ptyxis); many originally from Gogh (MIT), noted at the end of each file |
| Mockups `mockups/shared/icons/` (documentation only, not in the package) | Adwaita icons | CC-BY-SA-3.0 / LGPL-3.0, noted in `icons.css` |

## Rust dependencies (77 crates)

| Count | License | Examples |
|---|---|---|
| 38 | MIT OR Apache-2.0 | serde, toml, futures, hashbrown, bitflags |
| 31 | MIT | gtk4, libadwaita, vte4, glib, gio, cairo-rs, pango |
| 4 | Apache-2.0 OR MIT | indexmap, autocfg |
| 1 | Unlicense OR MIT | memchr |
| 1 | (MIT OR Apache-2.0) AND Unicode-3.0 | unicode-ident |
| 1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | io-lifetimes |
| 1 | Apache-2.0 WITH LLVM-exception | target-lexicon (build time only) |

All are compatible with GPL-3.0 (Apache-2.0 is GPLv3-compatible, not GPLv2-compatible – irrelevant
for Parkett).

## System libraries (dynamically linked)

| Library | License |
|---|---|
| VTE (gtk4) 0.84 | LGPL-3.0-or-later |
| GTK 4.24 | LGPL-2.1-or-later |
| libadwaita 1.10 | LGPL-2.1-or-later |
| GLib, Pango | LGPL-2.1-or-later / LGPL-2.0-or-later |

## Implications for distribution

- Binary packages must offer the source code (GPL) – for the Arch package from the own repo
  this is covered by the Git clone; for a release on GitHub by the public repo.
- The color schemes from Ptyxis remain GPL-3.0-or-later; their origin is documented in
  `data/palettes/README.md`.
- No components with copyleft incompatibilities were found.
