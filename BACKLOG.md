# Backlog

## Phase 2 – Design
- [x] Drei Konzepte (A GNOME Native, B Arbeitsbereiche, C Terminalzentriert) als HTML-Mockups
- [x] Entscheidung: A + Palette/Layout-Modus aus C + Vorlagen aus B (2026-10-18)
- [x] Kombiniertes Konzept A+ als Mockup (`mockups/combined.html`)
- [x] Freigabe A+: Bereichstitel schwebend, Vorschläge 1–6 übernommen (2026-10-18)
- [x] Projektname festlegen: Parkett (2026-10-18; „Parquet“ verworfen wegen Apache Parquet)
- [x] App-Icon und symbolisches Icon (`icon/`)
- [ ] Name noch prüfen: Markenregister (DPMA/EUIPO), Domains
- [x] App-ID festlegen, Icons unter `data/icons/hicolor/` (de.linuxundich.Parkett)

## Phase 3 – Prototyp
- [x] Sprache Rust, App-ID de.linuxundich.Parkett (2026-10-18)
- [x] Fenster mit libadwaita + VTE, Shell asynchron, Prozessende-Leiste mit Neu starten
- [x] Titel/Verzeichnis aus Termprops, Farben/Schrift nach System, Zoom, Kopieren/Einfügen
- [x] Kern `layout` mit Tests (Split, Schließen, Größe, Angleichen, Zoom, Nachbar, Mindestgröße)
- [x] Test in Fedora-VM (Wayland, X11, Unicode, Dunkel, Resize, Speicher) – docs/PROTOTYP.md
- [x] Start auf dem Host (Arch, GNOME 51) – Phase 5
- [x] Freigabe für Phase 4 (2026-10-18, Architekturvorschläge angenommen)

## Phase 4 – MVP (docs/MVP.md)
- [x] Architektur: Unterklassen, GSettings + Dateien, serde, Meson, GPL-3.0-or-later (angenommen)
- [x] Schritt 1: Tabs und Splits, Schließen mit Nachfrage, Kürzel-Übersicht (VM-getestet)
- [x] Schritt 2: Meson-Build, GSettings-Schema, LICENSE, Arch-Paket (2026-10-18)
- [x] Schritt 3: Profile und Farbschemata (Ptyxis-Format, 14 Schemata, Import, Kacheln) (2026-10-18)
- [x] Schritt 4: Einstellungen (Allgemein, Profile, Tastenkürzel) (2026-10-18)
- [ ] Profil-Felder Umgebung/Verzeichnis/Login-Shell und Löschen in der VM testen
- [x] Schritt 5: Suche, Links öffnen, änderbare Kürzel (2026-10-18)
- [x] Schritt 6: Befehlspalette, Layout-Modus, Vorlagen mit Startbefehlen (2026-10-18)
- [x] Schritt 7: Rückgängig beim Schließen, Kontextfarbe je Bereich (2026-10-18)
- [x] Phase 5: Qualitätssicherung (docs/QA.md, 2026-10-19)
- [x] Begriff: durchgehend „Reiter“ (2026-10-18)
- [x] Flatpak von Anfang an oder zuerst nativ → zuerst nativ (docs/PAKETIERUNG.md)
- [x] Farbschema-Format: Ptyxis-Palette (Schritt 3)

## Phase 6 – Stabilisierung und Veröffentlichung
- [x] Code-Review (Architektur, Codequalität) und Befunde behoben
- [x] Formatierung (`rustfmt.toml`), Clippy ohne Warnungen, SPDX-Kennungen
- [x] CI-Skript `build-aux/ci.sh` und VM-Rauchtest `build-aux/vm/smoke.sh`
- [x] Doku: Architektur, Bedienung, README; Beispiele in der Bedienung als Test
- [x] Paketierungswege untersucht (docs/PAKETIERUNG.md)
- [x] Lizenzen geprüft (docs/LIZENZEN.md)
- [x] Entscheidung Veröffentlichung (2026-10-19): Tag v0.1.0, vorerst nur lokal; einziger Weg ist das Arch-Repo auf bender
- [ ] Später: öffentliches Repo (GitHub, dann Englisch), AUR, COPR

## QA-Audit 2026-10-19 (`qa/2026-10-19/QA-BERICHT.md`)
- [x] QA-01 F2/F10 gehören wieder den Programmen im Terminal
- [x] QA-02 Bereichstitel verdeckt Text → Design V2 (ausgeblendet, Hover/Fokuswechsel)
- [x] QA-03 kein Neustart geschlossener Reiter, Einblendung loslassen; R-01 Seitenabfrage
- [x] QA-04 Einfüge-Warnung bei jedem Zeilenumbruch
- [x] QA-05/06 OSC 7 dekodieren, nur eigener Rechner
- [x] QA-07 Markup maskieren; QA-09 Fokus nach Menüs
- [x] QA-10 Palette One; R-03 Umgebungsfeld quoten; R-02 Rauchtest-Protokoll
- [ ] QA-08 Umgebung eines zweiten Aufrufs (`FOO=1 parkett --tab`) übernehmen
- [ ] QA-11 Prozessende-Standard (schließen bei Code 0? neutrale Leiste) – Entscheidung
- [ ] QA-12 Tooltips aus aktueller Kürzelbelegung
- [ ] QA-13 Kopfleiste unter 600 px (AdwBreakpoint)
- [ ] QA-14 blinkender Text nur bei Fokus
- [ ] QA-15 Fehlerdialog in Festbreitenschrift, Einblendung ohne Zeitablauf
- [ ] QA-16 Profil-Editor ohne Fokus im Namensfeld
- [ ] QA-17 kurze Reiter-Beschriftung „Programm · Ordner“ (Entscheidung)
- [ ] QA-18 Screenreader-Namen (Übersicht, Kürzel nicht im Namen)
- [ ] QA-19 Trefferzahl in der Suche (optional)
- [ ] QA-20 Cursor nach Umbruch: gegen Ptyxis prüfen
- [ ] R-04 `file:///`-Links aus SSH-Sitzungen; R-05 bis R-12 aus dem Bericht
- [ ] „Hoher Kontrast“ hell: Farben unter 3:1 nachschärfen; `cargo audit` in CI
- [ ] Hauptmenü um Palette, Layout-Modus, Vorlagen ergänzen

## Entwicklungsplan

Reihenfolge als Vorschlag; jede Stufe endet mit Test in VM und auf dem Host und einem Paket.

### 0.2 – Lücken aus dem Alltag
- [x] Schrift, Cursorform, Blinken und Glocke je Profil (2026-10-19)
- [x] Transparenter Hintergrund (Deckkraft je Profil) (2026-10-19)
- [x] Nautilus-Kontextmenü per nautilus-python (2026-10-19)
- [x] Sixel-Schalter, nur aktiv mit VTE mit Sixel (2026-10-19)
- [ ] Sixel mit einer VTE testen, die mit Sixel-Unterstützung gebaut ist (Arch und Fedora 44 bauen ohne)
- [ ] Hoher Kontrast: Farbschema „Hoher Kontrast“ automatisch verwenden
- [ ] Auswahlfarbe im Dunkelmodus (`set_color_highlight`)
- [ ] Trenner per Maus ziehen und Reiter in neues Fenster ziehen: Test und Feinschliff
- [ ] Cursor nach Schließen der Reiter-Übersicht unsichtbar – Ursache
- [ ] Importierte Schemata sofort alphabetisch einsortieren
- [ ] Aufteilung einer Vorlage im Editor ändern
- [ ] Strg+Eingabe in der Palette: Ziel im neuen Bereich öffnen

- [x] Befehlszeile: `parkett -- befehl`, `--working-directory`, `--tab`, `--profile`, `--title` (2026-10-19)
- [ ] Aufräumen aus dem Review: L1/L2 schwache Referenzen in Einstellungen/Zeichenfunktionen,
      L3 Sekundentimer in Hintergrund-Reitern pausieren, D2/D3 Vordergrund-Abfrage und
      Farbumrechnung zusammenfassen, D6 Debug-Ausgabe (docs/QA.md, Phase 6)

### 0.3 – Sitzungen
- [x] Sitzung sichern und beim Start wiederherstellen (Layout, Verzeichnisse, ohne Prozesse)
      (2026-10-19)
- [ ] Eingabe an mehrere Bereiche gleichzeitig senden (Tilix „Synchronisierte Eingabe“), mit
      deutlicher Kennzeichnung
- [x] Benachrichtigung, wenn ein langer Befehl in einem Hintergrund-Reiter endet (vte.sh-Termprops, 2026-10-19)

### 0.4 – Verbreitung
- [ ] Englische Oberfläche (gettext), dann Übersetzungen
- [ ] Debian/Ubuntu: Ubuntu 26.04 baut und läuft unverändert (QA 2026-10-19); `tuxedo-os-forky` und ein .deb offen
- [ ] AUR und Fedora COPR aus dem öffentlichen Repo

### Später
- [ ] Flatpak mit Host-Agent (wie Ptyxis), eigenes Repo – kein Flathub
- [ ] Schnellterminal (Quake-Stil) – unter Wayland nur mit Hilfe der Shell möglich, erst prüfen
- [ ] Orca mit Sprachausgabe von Hand prüfen; CPU unter Dauerausgabe auf dem Host messen
