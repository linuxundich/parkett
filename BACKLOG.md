# Backlog

## Phase 2 – Design
- [x] Drei Konzepte (A GNOME Native, B Arbeitsbereiche, C Terminalzentriert) als HTML-Mockups
- [x] Entscheidung: A + Palette/Layout-Modus aus C + Vorlagen aus B (2026-10-18)
- [x] Kombiniertes Konzept A+ als Mockup (`mockups/combined.html`)
- [x] Freigabe A+: Bereichstitel schwebend, Vorschläge 1–6 übernommen (2026-10-18)
- [x] Projektname festlegen: Parkett (2026-10-18; „Parquet“ verworfen wegen Apache Parquet)
- [x] App-Icon und symbolisches Icon (`icon/`)
- [ ] Name noch prüfen: Markenregister (DPMA/EUIPO), Domains
- [ ] App-ID festlegen, Icons danach unter `data/icons/hicolor/` umbenennen

## Phase 3 – Prototyp
- [x] Sprache Rust, App-ID de.linuxundich.Parkett (2026-10-18)
- [x] Fenster mit libadwaita + VTE, Shell asynchron, Prozessende-Leiste mit Neu starten
- [x] Titel/Verzeichnis aus Termprops, Farben/Schrift nach System, Zoom, Kopieren/Einfügen
- [x] Kern `layout` mit Tests (Split, Schließen, Größe, Angleichen, Zoom, Nachbar, Mindestgröße)
- [x] Test in Fedora-VM (Wayland, X11, Unicode, Dunkel, Resize, Speicher) – docs/PROTOTYP.md
- [ ] Start auf dem Host (Arch, GNOME 51)
- [x] Freigabe für Phase 4 (2026-10-18, Architekturvorschläge angenommen)

## Phase 4 – MVP (docs/MVP.md)
- [x] Architektur: Unterklassen, GSettings + Dateien, serde, Meson, GPL-3.0-or-later (angenommen)
- [x] Schritt 1: Tabs und Splits, Schließen mit Nachfrage, Kürzel-Übersicht (VM-getestet)
- [ ] Schritt 2: Meson-Build, GSettings-Schema, LICENSE
- [ ] Schritt 3: Basisprofile
- [ ] Schritt 4: Einstellungen (AdwPreferencesDialog)
- [ ] Schritt 5: Suche, Links öffnen
- [ ] Schritt 6: Befehlspalette, Layout-Modus, Vorlagen (A+)
- [ ] Schritt 7: Rückgängig beim Schließen, Kontextfarbe je Bereich
- [ ] Begriff „Tab“ oder „Reiter“ (libadwaita sagt „Reiter“) – Entscheidung Christoph
- [ ] Cursor nach Schließen der Tab-Übersicht unsichtbar bis zur Eingabe – Ursache suchen
- [ ] Tabs in neues Fenster ziehen und Trenner per Maus ziehen testen
- [ ] Flatpak von Anfang an (Host-Agent) oder zuerst nativ
- [ ] Farbschema-Format (Ptyxis-Palette prüfen)

## Gefunden beim Prototyp
- [ ] Auswahlfarbe im Dunkelmodus setzen (`set_color_highlight`), derzeit VTE-Standard
