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
| D2 | Dauerlauf 15 min (VM), 4 Bereiche mit je ≈ 1000 Zeilen/s | ✅ RSS 283 → 296 MB, ab Minute 10 flach (295–296 MB: Verlauf von 10 000 Zeilen je Bereich gefüllt), Threads 41, CPU 36–60 % eines Kerns (Software-Rendering) |

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

## Rückmeldung Christoph nach eigenem Test (r13, 2026-10-19)

| Wunsch | Umsetzung | Test (VM) |
|---|---|---|
| Knöpfe zum Teilen wie bei Tilix | „Rechts teilen“ und „Unten teilen“ links in der Kopfleiste neben „Neuer Reiter“; eigene symbolische Symbole (`data/icons/hicolor/scalable/actions/parkett-split-*-symbolic.svg`, Adwaita hat keine passenden); Tooltip mit aktuellem Kürzel; Screenreader-Namen | ✅ beide Knöpfe teilen den aktiven Bereich (`kopfleiste-teilen-stil.png`) |
| Hell/Dunkel von Hand | GSettings `style` (system/light/dark) → `AdwStyleManager::set_color_scheme`; Stilwahl mit drei runden Knöpfen oben im Hauptmenü (Muster GNOME Text Editor) und „Stil“ in Einstellungen › Allgemein | ✅ „Dunkel“ bei hellem System, „Hell“ bei dunklem System; Terminals folgen |

## Phase 6 – Code-Review (2026-10-19)

Vollständiges Lesen aller Quelldateien durch einen Review-Agenten, Clippy (Standard und
pedantic). Funde und Umsetzung; Test: ✅ in der VM nachgestellt · 🔍 nur Code geprüft ·
🧪 Unit-Test.

| ID | Fund | Umsetzung | Test |
|---|---|---|---|
| K1 | Nach *Rückgängig* an neuer Stelle behielten Fokus- und Schließen-Handler die alte Bereichs-Id: Klick setzte den Fokus nicht, Schließen traf den falschen Bereich | Handler lesen die Id beim Aufruf über eine schwache Referenz auf den Bereich | ✅ Klick in den zurückgeholten Bereich, *Bereich schließen* beendet genau dessen Shell |
| K2 | Unbekannte Id beim Schließen galt als „letzter Bereich“ und schloss den Reiter (beim letzten Reiter das Fenster) | `close_pane` ignoriert Ids, die nicht im Layout sind | 🔍 |
| K8 | *Rückgängig* kehrte nach Fokuswechsel oder Trennerziehen nicht an den alten Platz zurück | Vergleich nur der Aufteilung (`Layout::same_shape`) | 🧪 |
| P1 | Absturz beim Start, wenn alle Profildateien kaputt sind | Standardprofil nur im Speicher, unter freier Id; kaputte Datei bleibt unangetastet | 🧪 ✅ |
| P2 | Absturz an Nicht-ASCII in Farbwerten (`#aäbcd`), bei kaputter importierter Palette bei jedem Start | `Rgb::parse` lehnt Nicht-ASCII ab | 🧪 ✅ |
| G4 | Kaputte Profil-/Paletten-/Vorlagendateien nur im Protokoll | Einblendung „N Datei(en) nicht lesbar“ mit *Details* | ✅ |
| S1 | Einfüge-Warnung zählte nur `\n`; Text mit `\r` lief ohne Rückfrage durch | `\r`, `\n` und `\r\n` zählen als Umbruch | ✅ |
| S2 | `file:`-Links aus SSH-Sitzungen öffneten lokale Dateien gleichen Pfads | nur ohne Host, `localhost` oder eigener Rechnername | 🔍 |
| S3 | Unterverzeichnis einer Vorlage konnte absolut sein oder „..“ enthalten; `~` in übernommenen Befehlen wurde beim Eintippen aufgelöst | `paths::join_subdir` lässt nur relative Pfade zu; `~` wird gequotet | 🧪 / 🔍 |
| S4 | Halb getippte Anführungszeichen in Umgebungsvariablen speicherten „keine Variablen“ | bei Fehler nicht speichern, Zeile rot | 🔍 |
| K3 | Layout-Modus blieb nach Reiterwechsel auf dem alten Reiter stehen | beim Beenden alle Reiter zurücksetzen | ✅ |
| K4 | Layout-Modus schluckte Tasten in Dialogen (Nachfrage, Palette) | Tasten nur abfangen, wenn ein Terminal den Fokus hat | ✅ Eingabe landet in der Palette; im Terminal teilt *R* weiter |
| K5/K6 | Automatischer Neustart und Umbauten in Hintergrund-Reitern holten sich den Fokus | Fokus nur bei Klick auf *Neu starten* bzw. bei sichtbarem Reiter | 🔍 |
| K7/D1 | `~` an zwei Stellen unterschiedlich aufgelöst (`~foo` falsch) | gemeinsames Modul `core/src/paths.rs` | 🧪 |
| D5 | Jede Änderung an Vorlagen färbte alle Terminals neu ein | eigenes Signal `templates-changed` | 🔍 |
| G1 | Doppelklick auf die Kopfleiste öffnete zusätzlich die Befehlspalette | Palette erst nach Ablauf der Doppelklick-Zeit | 🔍 |
| G2 | *Alt+1…9* / *Strg+Alt+1…9* fest belegt, aber beim Aufnehmen eigener Kürzel aktiv und ohne Konfliktprüfung | beim Aufnehmen abgeschaltet, als belegt gemeldet | 🔍 |
| P3, D4, D7, D8 | `expect` im Vorlagen-Editor, toter Code, fragiles `"Aus"`, PCRE-Kommentar | behoben | 🔍 |

Nicht umgesetzt, im BACKLOG: L1/L2 (starke Referenzen in Einstellungen und Zeichenfunktionen,
kein Leck nachgewiesen), L3 (Sekundentimer auch in Hintergrund-Reitern), D2/D3
(Vordergrundprozess-Abfrage und Farbumrechnung zusammenfassen), D6 (Debug-Ausgabe),
G3 (Befehlszeile `-- befehl`, `--working-directory`).

Rauchtest danach (`build-aux/vm/smoke.sh`): 9 von 9 Prüfungen bestanden. CI (`build-aux/ci.sh`):
Formatierung, Clippy ohne Warnungen, 45 Tests, Meson-Tests grün.

## Alltagsfunktionen für 0.2 (2026-10-19, VM)

| Funktion | Test | Ergebnis |
|---|---|---|
| Befehlszeile | `--help`, `--version`; `--gibtsnicht` und `-d /gibtsnicht` → Meldung, Exit 1; `--tab -d /usr/share -t Logs` aus zweiter Instanz; `cd /tmp && parkett`; `--tab -- sh -c …` | ✅ Reiter „Logs“ in `/usr/share`, neues Fenster in `/tmp`, Befehl statt Shell |
| Unbekanntes Profil | Profil-TOML anlegen, während Parkett läuft, dann `--profile qa-cursor` | 🔧 wurde zuerst nicht gefunden; jetzt neu eingelesen ✅ |
| Schrift, Cursor | Profil mit `Monospace Bold 18`, `underline`, `off`, Nord | ✅ |
| Glocke „markieren“ | `sleep 3; tput bel`, Reiter wechseln | ✅ Reiter markiert |
| Sitzung | 2 Fenster, 4 Reiter, Titel, Verzeichnisse `~`, `/usr/share`, `/tmp`; SIGTERM; Neustart | ✅ alles zurück; 🔧 Befehlsbereich ohne `vte.cwd` wäre in `/` gelandet → Rückfall auf `/proc/<pid>/cwd` |
| Fensterreihenfolge | zuletzt fokussiertes Fenster oben | 🔧 umgekehrtes Öffnen half nicht (Mutter fokussiert das erste Fenster) → in gespeicherter Reihenfolge öffnen ✅ |
| Letzter Reiter geschlossen | teilen, sofort *Reiter schließen* | 🔧 alter Stand blieb stehen → Sitzung wird gelöscht, Neustart frisch ✅ |
| Fenster mit Reitern geschlossen | teilen, sofort *Alt+F4* | ✅ 2 Bereiche beim Neustart |
| Benachrichtigung | `sleep 12` / `sleep 11`, Reiter wechseln | ✅ Reiter markiert, `AddNotification` „Befehl beendet – sleep · nach 10 s“ (D-Bus), Banner sichtbar; `app.show-terminal` wechselt zum Bereich. 🔧 Dauer jetzt gerundet |
| Rauchtest | `build-aux/vm/smoke.sh` mit neuer Prüfung „Sitzung wiederhergestellt“ | ✅ 10/10 |

Nicht geprüft: Glocke mit Ton (nicht hörbar prüfbar), Aufruf über `xdg-terminal-exec`
(weder in der VM noch auf dem Host installiert), Host (Arch).

## Transparenz und Sixel (2026-10-19, VM)

| Funktion | Test | Ergebnis |
|---|---|---|
| Transparenz | Profil `opacity = 0.7`, Fenster vor anderem Fenster | ✅ Terminalfläche scheint durch; 🔧 Kopfleiste war mit durchsichtig (libadwaita nutzt den Fensterhintergrund) → eigener Hintergrund für die obere Leiste |
| Trenner bei Transparenz | Rückmeldung Christoph (Host): Trenner als helle Linie über hellem Fenster dahinter | 🔧 `--border-color` ist halbtransparent → deckende Farbe aus Fenster-Vorder- und Hintergrund; VM hell und dunkel: Trennerpixel gleich über Nautilus und Desktop ✅ |
| Transparenz geteilt | *Unten teilen* im Glas-Fenster | ✅; abgeblendete Bereiche wirken etwas deckender (Abblendung malt die Hintergrundfarbe darüber) |
| Sixel verfügbar? | `Vte.get_feature_flags()` auf Host und VM | Host (Arch, `vte4` 0.84.1) und VM (Fedora 44, `vte291-gtk4` 0.84.1): 7 = BIDI, ICU, SYSTEMD, **ohne Sixel** |
| Sixel-Schalter | AT-SPI in den Einstellungen | ✅ ausgegraut, Untertitel „Die installierte VTE-Bibliothek ist ohne Sixel gebaut“ |
| Sixel-Ausgabe | – | ⏭️ nicht ausführbar: keine VTE mit Sixel vorhanden |
| Rauchtest, CI | `build-aux/vm/smoke.sh`, `build-aux/ci.sh` | ✅ 10/10, grün |

## Nautilus (2026-10-19, VM Fedora 44, Nautilus 50.4, nautilus-python 4.1.0)

| Test | Ergebnis |
|---|---|
| Menü des Ordners `/usr/share` zeigt *Parkett hier öffnen*; Klick bei nicht laufendem Parkett | ✅ eine Shell in `/usr/share`, keine Sitzung dazu |
| zwei Ordner markiert, Rechtsklick → *In Parkett öffnen* | ✅ neues Fenster mit Reitern `abrt` und `accountsservice`, erster gewählt; 🔧 zuerst als Kette von `--tab`-Aufrufen geplant – das neue Fenster bekommt unter Mutter nicht sicher den Fokus, daher jetzt ein Aufruf mit mehreren `-d` |
| Kindprozesse von Nautilus danach | ✅ keine (kein Zombie) |
| Syntax der Erweiterung | ✅ Meson-Test `compile-nautilus-extension` |

Nicht geprüft: Nautilus 51 auf dem Host (Erweiterung kommt mit dem Paket; Nautilus läuft dort
mit offenen Fenstern und wurde für den Test nicht neu gestartet).
