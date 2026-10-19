# Parkett

Moderner Terminalemulator für GNOME auf Basis von GTK4, libadwaita und VTE, der die Stärken von
Tilix (Splits, Sitzungen mit Layout, Profile) übernimmt und die Oberfläche neu denkt. Der Name
steht für das Fischgrät-Muster aus Terminal-Bereichen, das die Splits bilden.

## Was es kann

Reiter und frei teilbare Bereiche (Knöpfe wie bei Tilix, Kürzel, Layout-Modus mit
Einzeltasten), Befehlspalette, Vorlagen mit Startbefehlen, Profile, 14 Farbschemata mit
heller und dunkler Fassung (Import von Ptyxis und Tilix), Kontextfarbe für root und SSH,
Rückgängig beim Schließen, Suche, Links, änderbare Kürzel, Hell/Dunkel von Hand,
Sitzung beim Start wiederherstellen, Befehlszeile (`parkett -d DIR -- befehl`), Schrift,
Cursor, Glocke und Deckkraft (transparenter Hintergrund) je Profil, Sixel-Grafiken (wo VTE sie kann), Benachrichtigung, wenn ein langer Befehl im Hintergrund endet, Kontextmenü in Nautilus
(mit `nautilus-python`).
Anleitung: [`docs/BEDIENUNG.md`](docs/BEDIENUNG.md).

## Dokumentation

| Datei | Inhalt |
|---|---|
| [`docs/BEDIENUNG.md`](docs/BEDIENUNG.md) | Bedienung, Kürzel, Profile, Vorlagen und Farbschemata als Datei |
| [`docs/ARCHITEKTUR.md`](docs/ARCHITEKTUR.md) | Module, Regeln, Datenfluss, Tests |
| [`docs/PAKETIERUNG.md`](docs/PAKETIERUNG.md) | Arch, Fedora, Debian, Flatpak – Stand und Empfehlung |
| [`docs/LIZENZEN.md`](docs/LIZENZEN.md) | Lizenzen aller Teile und Abhängigkeiten |
| [`docs/ANALYSE.md`](docs/ANALYSE.md) | Phase 1: Bestand Tilix, Funktionsmatrix, Sprachwahl |
| [`docs/KONZEPTE.md`](docs/KONZEPTE.md) | Phase 2: drei Konzepte, Entscheidung für A+ |
| [`docs/PROTOTYP.md`](docs/PROTOTYP.md) | Phase 3: Prototyp und Testprotokoll |
| [`docs/MVP.md`](docs/MVP.md) | Phase 4: Schritte 1–7 mit Tests |
| [`docs/QA.md`](docs/QA.md) | Phase 5: Qualitätssicherung in VM und auf dem Host |
| [`qa/2026-10-19/QA-BERICHT.md`](qa/2026-10-19/QA-BERICHT.md) | QA-Audit in Ubuntu 26.04 mit Befunden, Maßnahmenplan und [Designvarianten](qa/2026-10-19/designvarianten.html) |
| [`BACKLOG.md`](BACKLOG.md) | offene Punkte und Entwicklungsplan |

## Bauen und starten

Abhängigkeiten: Rust, GTK ≥ 4.18, libadwaita ≥ 1.8, VTE (gtk4) ≥ 0.80
(Arch: `gtk4 libadwaita vte4`; Fedora: `gtk4-devel libadwaita-devel vte291-gtk4-devel`).

```bash
cargo run -p parkett
```

Installieren mit Meson (GSettings-Schema, .desktop, Icons):

```bash
meson setup _build --prefix=$HOME/.local --buildtype=release
```

```bash
meson install -C _build
```

Arch: Paket `parkett` aus dem eigenen Repo `linuxundich` (bender, siehe `services/pacman-repo`) –
derzeit der einzige Verteilweg. Aktuelle Version: 0.2.0 (Tag `v0.2.0`).

Prüfen ohne Fenster (Formatierung, Clippy, Tests, Meson-Build und -Tests):

```bash
build-aux/ci.sh
```

Rauchtest in der VM (startet Parkett, steuert es über D-Bus, prüft Prozesse und Protokoll):

```bash
build-aux/vm/smoke.sh
```

Test in der VM `fedora44-wor` (GNOME Boxes, SSH-Port 2223; Ubuntu 26.04: `VM_DOM=ubuntu26.04 VM_PORT=2222`): `build-aux/vm/deploy.sh` baut dort,
installiert nach `~/.local` und startet wie GNOME Shell; `build-aux/vm/shot.sh datei.png` macht
ein Bildschirmfoto, `build-aux/vm/click.py X Y` klickt.

## Mockups ansehen

```bash
python3 -m http.server 8751 --bind 127.0.0.1 --directory mockups
```

Dann `http://127.0.0.1:8751/` öffnen (ES-Module brauchen einen Server, `file://` geht nicht).
Jede Konzeptseite hat Umschalter für Ansicht, Hell/Dunkel und Erläuterungen; der Zustand steht in
der URL (z. B. `concept-a.html#view=split&theme=dark&hints=0`).

Aufbau:

| Datei | Inhalt |
|---|---|
| `mockups/index.html` | Übersicht und Gegenüberstellung |
| `mockups/concept-{a,b,c}.html` | die drei Konzepte mit je fünf Ansichten |
| `mockups/combined.html` | Konzept A+ (gewählte Kombination) mit elf Ansichten und Titel-Schalter |
| `mockups/shared/base.css` | Farben aus libadwaita 1.10, Grundbausteine |
| `mockups/shared/content.js` | gemeinsamer Beispielinhalt, Split-Renderer, Flächenmessung |
| `mockups/shared/icons.css` | Adwaita-Symbolic-Icons (CC-BY-SA 3.0 / LGPL-3.0) als Masken |

## Icon

App-Icon und symbolisches Icon liegen unter `data/icons/hicolor/`; Quelle ist
[`icon/gen_icon.py`](icon/gen_icon.py) (Fischgrät-Parkett mit eingelassenem Terminal-Bereich,
dort ändern). Verworfene Entwürfe in `icon/entwuerfe/`.

## Lizenz

GPL-3.0-or-later, siehe [`LICENSE`](LICENSE) und [`docs/LIZENZEN.md`](docs/LIZENZEN.md).
