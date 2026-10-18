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
  **Wartet auf Freigabe** von A+ (offen: Standard für Bereichstitel, neue Vorschläge 1–6).
- Phase 3 (technischer Prototyp) beginnt erst nach Freigabe des Designs.

Es gibt noch keinen Anwendungscode.

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
