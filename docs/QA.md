# Phase 5 – Qualitätssicherung

Stand: 2026-10-19, Version 0.1.0 (Commit siehe CHANGELOG). Testumgebungen:

| Umgebung | System | Grafik |
|---|---|---|
| **VM** fedora44-wor (GNOME Boxes) | Fedora 44, GNOME 50.5, GTK 4.22.5, libadwaita 1.9.4, VTE 0.84.1, 8 Kerne, 5,8 GB | Software-Rendering (llvmpipe) |
| **Host** smokki | Arch, GNOME 51.1, GTK 4.24.1, libadwaita 1.10.0, VTE 0.84.1, Mesa 26.2.5 | echte GPU |

Kennzeichnung: ✅ ausgeführt und bestanden · ⚠️ ausgeführt, mit Befund · 🔧 Befund behoben und
nachgetestet · ⏭️ nicht ausgeführt. Werkzeuge: `build-aux/qa/` (Durchsatz, Messen),
`build-aux/vm/` (Deploy, Bildschirmfoto, Klick), QMP-Tasten, AT-SPI per Python, D-Bus-Aktionen
(`org.gtk.Actions` am Fenster, z. B. `new-tab-profile`, `split-right`, `close-tab`).

## Ergebnisse nach der Liste im Briefing

### 1. Starten und Beenden von Shell-Prozessen

| ID | Test | Ergebnis |
|---|---|---|
| P1 | `exit 3` → Leiste „Exit-Code 3“, Neu starten | ✅ (Phase 3/4) |
| P2 | Shell von außen mit `kill -9` beenden | ✅ „Prozess beendet · Signal 9“ |
| P3 | Profil mit nicht vorhandenem Befehl | ✅ „gibtesnicht123 konnte nicht gestartet werden: …“ |
| P4 | Profil mit unvollständigem Anführungszeichen | ✅ „… Text endete, bevor ein passendes Anführungszeichen …“ |
| P5 | Profil `false` + „Neu starten“ | ✅ keine Schleife: „kein automatischer Neustart, endete sofort“ |
| P6 | Profil mit Umgebungsvariable und Startverzeichnis | ✅ `QA=ok`, `/usr/share` |
| P7 | Parkett beenden (SIGTERM) mit 3 Shells (Host) | ✅ keine verwaisten Shells |
| P8 | Reiter mit laufendem Programm schließen | ✅ Nachfrage; nach Ablauf der Rückgängig-Frist Prozess beendet |

### 2. Mehrere parallele Terminals

| ID | Test | VM | Host |
|---|---|---|---|
| M1 | Speicher 1 Terminal | 201 MB RSS | 72 MB RSS |
| M1 | 24 Terminals (8 Reiter × 3 Bereiche) | 307 MB (≈ 4,6 MB je Terminal) | 123 MB |
| M1 | Leerlauf-CPU mit 24 Terminals | 0,53 % | 0,60 % |
| M2 | 3 Runden: 21 Terminals öffnen und schließen | 🔧 RSS pendelt bei ≈ 300 MB, kein Wachstum über die Runden; nach 10 s wieder 1 Shell | 🔧 nach 12 s genau 3 Shells |

**Befund M2 (behoben):** Rückgängig-Einblendungen standen Schlange (je 8 s); 7 geschlossene Reiter
hielten ihre Prozesse bis zu einer Minute am Leben (nach 14 s noch 15 Shells). Jetzt ersetzt eine
neue Einblendung die vorige, die vorige wird sofort endgültig geschlossen. Rückgängig gilt damit
für das zuletzt Geschlossene.

### 3. Verschachtelte Splits

| ID | Test | Ergebnis |
|---|---|---|
| S1 | Wiederholt rechts/unten teilen (maximiert, 1280×800) | ✅ Mindestgröße 20×5 greift („Zu wenig Platz zum Teilen“), keine Überlappung |
| S2 | 3 Bereiche, Nummern, Tauschen, Fokus im Layout-Modus | ✅ (Phase 4) |
| S3 | Schwebender Titel in schmalen Bereichen | 🔧 verdeckte den Prompt; jetzt unter 380 px nur noch die Knöpfe |
| S4 | Einstellung „Bereichstitel: Als Zeile“ | 🔧 war beschlossen, fehlte; jetzt umgesetzt und live umschaltbar |

### 4. Fenstergrößenänderungen

| ID | Test | Ergebnis |
|---|---|---|
| W1 | Maximieren → zurück mit 5 Bereichen | ✅ Bereiche schrumpfen anteilig, Mindestgröße bleibt, kein Überlauf |
| W2 | Verhältnis 40,9 % bei 960 und 1920 px | ✅ (Phase 4) |
| W3 | Fenstergröße/maximiert merken | ✅ (Phase 4) |

### 5. Copy & Paste und Tastaturkürzel

| ID | Test | Ergebnis |
|---|---|---|
| C1 | Wort doppelklicken → Strg+Umschalt+C → Strg+Umschalt+V | ✅ |
| C2 | Umschalt+Einfg (Primärauswahl) | ✅ |
| C3 | Mehrzeiligen Text einfügen | 🔧 Warnung fehlte (Funktionsmatrix P0); jetzt Dialog mit Vorschau, abschaltbar |
| K1 | Alle Standardkürzel | ✅ in Phase 4 einzeln geprüft; änderbar, Konflikte abgelehnt |

Hinweis C3: Die Warnung gilt für Strg+Umschalt+V und das Kontextmenü. Umschalt+Einfg und
Mittelklick fügen weiter ohne Rückfrage ein (VTE-intern); moderne Shells schützen dort mit
„Bracketed Paste“.

### 6. Unicode und Zeichensätze

| ID | Test | Ergebnis |
|---|---|---|
| U1 | Umlaute, Kyrillisch, Griechisch, CJK breit, Emoji einfach, Rahmenzeichen, Truecolor | ✅ (Phase 3) |
| U2 | Arabisch/Hebräisch (RTL), Devanagari, Thai, Hangul, Mathe, Braille | ✅ |
| U2 | Emoji-Sequenzen mit ZWJ (👩‍💻), Hauttöne, Flaggen | ⚠️ zerfallen in Einzelzeichen bzw. Buchstaben – **Grenze von VTE**, Ptyxis zeigt dasselbe |

### 7. Lange laufende Prozesse

| ID | Test | Ergebnis |
|---|---|---|
| D1 | Durchsatz: `seq` 3 Mio. Zeilen / 20 MB farbiger Text, maximiert | ✅ VM: Parkett 2,56–2,60 s / 1,17–1,28 s, Ptyxis 2,53–2,63 s / 1,17–1,24 s · Host: Parkett 2,50 s / 0,91 s, GNOME Console 2,79 s / 0,90 s |
| D2 | Dauerlauf 15 min, 4 Bereiche mit je ≈ 1000 Zeilen/s | siehe unten |

Erster Durchsatzlauf in der VM mit unterschiedlichen Fenstergrößen hatte Parkett 13–17 %
langsamer gezeigt; maximiert sind beide gleichauf – die Fenstergröße war die Ursache.

### 8. Fehlerhafte oder beendete Shell-Prozesse

Siehe P2–P5. Zusätzlich: Reiter mit Prozessfehler bekommen ein Warnsymbol (✅).

### 9. Hohe CPU- und Speicherauslastung

Siehe M1, M2, D2. Unter Dauerausgabe kostet das Rendering in der VM (Software) 40–60 % eines
Kerns; auf dem Host mit GPU nicht gemessen.

### 10. Wayland und X11

| ID | Test | Ergebnis |
|---|---|---|
| X1 | Wayland (VM und Host), Start wie GNOME Shell | ✅ |
| X2 | X11 über XWayland (`GDK_BACKEND=x11`): Teilen, Layout-Modus, Palette | ✅ keine GTK-Warnungen |

### 11. Barrierefreiheit und GNOME-Integration

| ID | Test | Ergebnis |
|---|---|---|
| A1 | AT-SPI: Knöpfe benannt (VM 9, Host 23) | 🔧 Neu-Knopf hieß „Weitere Möglichkeiten“ (AdwSplitButton), Suchknopf las das Kürzel mit; jetzt „Neuer Reiter“, „Suchen“ |
| A2 | Terminaltext über AT-SPI lesbar (Orca-Grundlage) | ✅ VM; Host: 3 Terminals, 8 Reiter im Baum |
| A3 | Orca mit Sprachausgabe | ⏭️ nicht hörbar prüfbar |
| G1 | Akzentfarbe live | ✅ Fokuslinie folgt |
| G2 | System-Monospace-Schrift live | ✅ |
| G3 | Hoher Kontrast | ⚠️ Oberfläche folgt (libadwaita), Terminal-Farbschema bleibt; Vorschlag: dann automatisch „Hoher Kontrast“ |
| G4 | Hell/Dunkel, .desktop, Icon im Dock | ✅ (Phase 3/4) |

## Offene Punkte aus der QA

- Hoher Kontrast: Farbschema „Hoher Kontrast“ automatisch verwenden (Vorschlag G3).
- Umschalt+Einfg/Mittelklick ohne Einfüge-Warnung (VTE-intern).
- Emoji-Sequenzen: VTE-Grenze, nicht in Parkett lösbar.
- Orca mit Sprachausgabe von Hand prüfen.
- CPU unter Dauerausgabe auf dem Host messen.
- Host-Test ohne Bildschirmfotos: Die deskwright-Erweiterung lief in der Host-Shell nicht;
  geprüft wurde über Prozesse, Protokoll, AT-SPI und D-Bus.
