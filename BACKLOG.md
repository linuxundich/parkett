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

## Entwicklungsplan

Reihenfolge als Vorschlag; jede Stufe endet mit Test in VM und auf dem Host und einem Paket.

### 0.2 – Lücken aus dem Alltag
- [ ] Hoher Kontrast: Farbschema „Hoher Kontrast“ automatisch verwenden
- [ ] Auswahlfarbe im Dunkelmodus (`set_color_highlight`)
- [ ] Trenner per Maus ziehen und Reiter in neues Fenster ziehen: Test und Feinschliff
- [ ] Cursor nach Schließen der Reiter-Übersicht unsichtbar – Ursache
- [ ] Importierte Schemata sofort alphabetisch einsortieren
- [ ] Aufteilung einer Vorlage im Editor ändern
- [ ] Strg+Eingabe in der Palette: Ziel im neuen Bereich öffnen

- [ ] Befehlszeile: `parkett -- befehl`, `--working-directory` (xdg-terminal-exec, Nautilus)
- [ ] Aufräumen aus dem Review: L1/L2 schwache Referenzen in Einstellungen/Zeichenfunktionen,
      L3 Sekundentimer in Hintergrund-Reitern pausieren, D2/D3 Vordergrund-Abfrage und
      Farbumrechnung zusammenfassen, D6 Debug-Ausgabe (docs/QA.md, Phase 6)

### 0.3 – Sitzungen
- [ ] Sitzung beim Beenden sichern und beim Start wiederherstellen (Layout, Verzeichnisse,
      ohne Prozesse) – Tilix-Funktion „Sitzung speichern“
- [ ] Eingabe an mehrere Bereiche gleichzeitig senden (Tilix „Synchronisierte Eingabe“), mit
      deutlicher Kennzeichnung
- [ ] Benachrichtigung, wenn ein langer Befehl in einem Hintergrund-Reiter endet (OSC 777 / VTE)

### 0.4 – Verbreitung
- [ ] Englische Oberfläche (gettext), dann Übersetzungen
- [ ] Debian/Ubuntu: in den VMs `ubuntu26.04` und `tuxedo-os-forky` prüfen, ggf. libadwaita 1.7
- [ ] AUR und Fedora COPR aus dem öffentlichen Repo

### Später
- [ ] Flatpak mit Host-Agent (wie Ptyxis), eigenes Repo – kein Flathub
- [ ] Schnellterminal (Quake-Stil) – unter Wayland nur mit Hilfe der Shell möglich, erst prüfen
- [ ] Orca mit Sprachausgabe von Hand prüfen; CPU unter Dauerausgabe auf dem Host messen
