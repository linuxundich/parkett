# Parkett

Moderner Terminalemulator für GNOME auf Basis von GTK4, libadwaita und VTE, der die Stärken von
Tilix (Splits, Sitzungen mit Layout, Profile) übernimmt und die Oberfläche neu denkt. Der Name
steht für das Fischgrät-Muster aus Terminal-Bereichen, das die Splits bilden.

## Stand

- **Phase 1 – Analyse:** fertig → [`docs/ANALYSE.md`](docs/ANALYSE.md)
  (Bestandsbericht, Funktionsmatrix, Sprachwahl, Architektur-Vorschlag, Risiken)
- **Phase 2 – Design:** drei Konzepte als HTML-Mockups → [`docs/KONZEPTE.md`](docs/KONZEPTE.md),
  [`mockups/`](mockups/). Entschieden: A als Grundgerüst plus Befehlspalette und Layout-Modus aus C
  und Vorlagen mit Startbefehlen aus B → `mockups/combined.html` („Konzept A+“).
  A+ ist freigegeben (Bereichstitel schwebend, Vorschläge 1–6 übernommen).
- **Phase 3 – Prototyp:** Rust, App-ID `de.linuxundich.Parkett`; ein Fenster mit einem Terminal,
  Split-Layout als getesteter Kern ohne GTK → [`docs/PROTOTYP.md`](docs/PROTOTYP.md)
  (Architektur, Testprotokoll, offene Entscheidungen).
- **Phase 4 – MVP:** freigegeben; Schritt 1 Tabs und Splits fertig und in der VM getestet →
  [`docs/MVP.md`](docs/MVP.md). Schritte 2 (Meson, GSettings, Arch-Paket), 3 (Profile, 14 Farbschemata hell/dunkel, Import),
  4/5 (Einstellungen, Suche, Links, änderbare Kürzel) und 6 (Befehlspalette, Layout-Modus,
  Vorlagen) fertig. Weiter: Rückgängig beim Schließen, Kontextfarbe je Bereich.

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

Arch: Paket `parkett` aus dem eigenen Repo `linuxundich` (bender, siehe `services/pacman-repo`).

```bash
cargo test
```

Test in der VM `fedora44-wor` (GNOME Boxes, SSH-Port 2223): `build-aux/vm/deploy.sh` baut dort,
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

App-Icon und symbolisches Icon liegen in [`icon/`](icon/): `parkett.svg` (Fischgrät-Parkett mit
eingelassenem Terminal-Bereich, erzeugt von `icon/gen_icon.py` – dort ändern) und
`parkett-symbolic.svg` (16 px). Vorschau in `icon/vorschau.png`, verworfene Entwürfe in
`icon/entwuerfe/`. Sobald die App-ID feststeht, wandern beide unter `data/icons/hicolor/`.
