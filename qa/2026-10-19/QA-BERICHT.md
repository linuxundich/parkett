# QA-Audit Parkett – 19.10.2026

Stand: Commit `0faa590` (0.1.0 plus `[Unreleased]`: Sitzung, Befehlszeile, Transparenz, Nautilus).
Geprüft am 19.10.2026, 08:48–09:25 Uhr, in der VM `ubuntu26.04`. Dazu kam ein vollständiges Lesen
des Quelltexts durch einen Review-Agenten (nur lesend).

**Am Quellcode, an der Konfiguration des Projekts und an Christophs Daten wurde nichts geändert.**
Testdaten lagen nur beim VM-Nutzer `testt` und sind wieder entfernt (siehe J).

Belege liegen in `bilder/`, die Designvarianten in [`designvarianten.html`](designvarianten.html).

Kennzeichnung der Befunde:

- **reproduziert:** in der VM ausgelöst und belegt
- **Code bestätigt:** Ursache im Quelltext eindeutig, aber nicht in der VM ausgelöst
- **vermutet:** plausibel, noch nicht bestätigt

Vorbemerkung: Der Auftrag nennt Parkett „die Terminal-Anwendung des GNOME-Projekts“. Parkett ist
aber Christophs eigenes Projekt (App-ID `de.linuxundich.Parkett`), kein GNOME-Projekt. Die
GNOME-HIG dienen hier als Maßstab, nicht als Vorgabe eines Upstreams.

---

## A. Zusammenfassung

Parkett ist für eine 0.1 erstaunlich robust:

- Es baut auf Ubuntu 26.04 ohne jede Anpassung. Rust 1.93, GTK 4.22.4, libadwaita 1.9.1 und VTE 0.84.0 erfüllen die Mindestversionen.
- Es startet in rund 150 ms bis zur laufenden Shell.
- Es hält mit Ptyxis beim Durchsatz mit.
- Nach 100 geöffneten und geschlossenen Terminals hinterlässt es keine Prozesse und hat kein Speicherleck.
- Es meldet kaputte Dateien verständlich.

Shell, Jobsteuerung, Farben, Unicode, Suche, Links, Sitzung und Nachfragen beim Schließen
funktionieren.

Die größten Probleme liegen an den Rändern zwischen Parkett und den Programmen darin:

1. **Parkett nimmt Programmen Tasten weg** (QA-01, hoch). *F2* öffnet „Reiter umbenennen“, *F10*
   das Hauptmenü – auch wenn `htop`, `mc` oder `nano` diese Tasten brauchen. In `htop` geht damit
   weder *Setup* noch *Beenden*.
2. **Der schwebende Bereichstitel verdeckt die erste Zeile** (QA-02, hoch). Bei geteilten Reitern
   liegen Prompt, Eingabe und Cursor eines frischen Bereichs unter dem Titel. Das ist der
   Standard und betrifft jede Ubuntu-Installation mit langem Prompt.
3. **Ein geschlossener Reiter mit Profil „Neu starten“ startet sein Programm unsichtbar neu**
   (QA-03, hoch). Bei `ssh server` baut sich die Verbindung im Hintergrund wieder auf. Das hört erst
   auf, wenn man wieder etwas schließt.

Dazu kommen fünf mittlere Sicherheits- und Funktionsfehler:

- Die Einfüge-Warnung lässt sich umgehen (QA-04).
- Verzeichnisse mit Leerzeichen oder Umlauten gehen beim Teilen verloren (QA-05).
- Ein fremder Rechner kann per OSC 7 das Startverzeichnis lokaler Terminals setzen (QA-06).
- Startbefehle einer Vorlage sind unsichtbar, wenn sie `&&` oder `<` enthalten (QA-07).
- Ein zweiter Aufruf von `parkett` verliert seine Umgebungsvariablen (QA-08).

Größtes Verbesserungspotenzial bei der Gestaltung: **Orientierung in geteilten Reitern.**
Fenstertitel, Reiter und Bereichstitel zeigen dreimal denselben langen Text. Die Reiter sind
dadurch nicht zu unterscheiden. Drei Varianten dazu stehen in H.

---

## B. Bestandsaufnahme

### Testumgebung

| Punkt | Wert |
|---|---|
| Host | Arch Linux, libvirt-Session-VM, zeitweise in GNOME Boxes angezeigt |
| Gast | Ubuntu 26.04.1 LTS, Kernel 7.0.0-38, GNOME Shell 50.1, Wayland, Deutsch (de_DE.UTF-8), Tastatur DE |
| Bibliotheken | GTK 4.22.4, libadwaita 1.9.1, VTE 0.84.0 (ohne Sixel gebaut), Orca 50.2 vorhanden |
| Bauen | rustc/cargo 1.93.1, meson 1.10.1 (für den Test nachinstalliert), `meson install` nach `~/.local`, Release-Build 1 min 44 s |
| Start | wie GNOME Shell: `systemd-run --user -u app-gnome-de.linuxundich.Parkett-…` |
| Grafik | **Softwarerendering** (llvmpipe; `ZINK: failed to choose pdev` im Protokoll), 8 vCPU, 5,3 GB RAM |
| Auflösung | erst 1280×800, ab 09:08 1920×1033 (Boxes-Fenster geöffnet) |
| Vergleich | Ptyxis 50.1 (Ubuntu-Standardterminal) unter gleichen Bedingungen |
| Steuerung | Tasten und Maus per QMP, D-Bus-Aktionen (`org.gtk.Actions`), AT-SPI per Python, SSH für Messungen |

Abweichungen von der Zielplattform, die man beim Lesen beachten muss:

- **Kein GPU-Rendering:** CPU-Werte für die Darstellung sind höher als auf echter Hardware.
- **Keine deutschen GTK-4-Übersetzungen** (`gtk40.mo` fehlt, Sprachpaket nicht installiert):
  Tastenbezeichnungen aus GTK erscheinen englisch („Shift+Ctrl+T“). Das ist eine Lücke der VM,
  kein Fehler von Parkett. Es macht aber QA-12 sichtbar.
- **Primärauswahl abgeschaltet:** Ubuntu setzt `gtk-enable-primary-paste` auf false. Einfügen
  per Mittelklick ist hier daher nicht prüfbar.

### Architektur (Kurzfassung)

| Teil | Inhalt |
|---|---|
| `core/` (ohne GTK) | Split-Baum `layout`, Profile (TOML), Paletten (Ptyxis-Format), Vorlagen, Sitzung (JSON), Befehlszeile, Kontext root/SSH, unscharfe Suche, Pfade – 53 Unit-Tests, 2 Doku-Tests |
| `app/` (GTK) | `window.rs` (AdwTabView/TabBar, Aktionen, Rückgängig), `tab.rs` (GtkPaned-Baum), `pane.rs` (Overlay: Titel, Fokuslinie, Abblendung, Suche), `terminal.rs` (VTE, Spawn, Prozessende, Links, Einfügen, Benachrichtigung), `prefs.rs` (AdwPreferencesDialog), `commands.rs` (Palette), `shortcuts.rs`, `session.rs`, `profiles.rs` |
| Daten | 17 GSettings-Schlüssel, `~/.config/parkett/{profiles,templates}`, `~/.local/share/parkett/palettes`, `~/.local/state/parkett/session.json` |
| Prozesse | VTE `spawn_async` je Bereich, 1-s-Timer je Terminal (`tcgetpgrp` + `/proc`), SIGHUP beim Schließen, 8 s Rückgängig-Frist |
| Integration | .desktop mit xdg-terminal-exec-Schlüssel, Metainfo, Icons, nautilus-python-Erweiterung, D-Bus-Aktivierung über GApplication |

### Funktionen nach Bereichen

| Bereich | Vorhanden |
|---|---|
| Funktionalität | Reiter, Splits rechts/unten/automatisch, Maximieren, Angleichen, Fokus per Pfeil und Nummer, Layout-Modus, Befehlspalette, Vorlagen mit Startbefehlen, Profile (Befehl, Login-Shell, Verzeichnis, Umgebung, Prozessende-Aktion), 14 Farbschemata hell/dunkel, Import Ptyxis/Tilix, Suche (Regex, Groß/klein), Links (Regex, OSC 8), Rückgängig, Sitzung, Befehlszeile, Benachrichtigung bei Befehlsende, Nautilus-Menü |
| Usability | änderbare Kürzel mit Konfliktprüfung, Kürzel-Übersicht, Nachfrage beim Schließen, Einfüge-Warnung, Fehlerleiste mit *Neu starten*, Einblendung für kaputte Dateien |
| Design | libadwaita-Widgets, Hell/Dunkel/System, Akzentfarbe, Kontextfarbe root/SSH, Abblendung, Deckkraft, Schrift/Cursor je Profil |
| Technik | Kern ohne GTK und testbar, CI-Skript (fmt, clippy -D warnings, Tests), VM-Rauchtest, atomares Speichern von Profilen/Vorlagen/Sitzung |

Bekannte Grenzen und technische Schulden (aus BACKLOG und Review):

- Sixel ungetestet.
- L1–L3, D2, D3, D6 aus dem Phase-6-Review sind offen.
- Große Funktionen: `Pane::new` ≈ 320 Zeilen, Profil-Editor ≈ 360 Zeilen.
- `ProfileStore` und `TemplateStore` sind fast gleich.
- Die Testanzahl in der Doku ist veraltet.

---

## C. Testergebnisse

Status: **Bestanden** · **Fehlgeschlagen** · **Teilweise** · **Nicht getestet** · **Nicht anwendbar**.

### Start, Fenster, Sitzungen

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-01 | Erster Start ohne Konfiguration, Shell läuft | Bestanden | `01-erster-start.png` |
| T-02 | Start bis Shell läuft (4 Läufe) | Bestanden | 141 / 161 / 152 / 151 ms |
| T-03 | Fenster schließen ohne laufende Programme: keine Nachfrage, keine verwaisten Shells | Bestanden | Prozessliste |
| T-04 | SIGTERM an Parkett mit 7 Shells: keine verwaisten Shells | Bestanden | `kill -0` je PID |
| T-05 | Sitzung: 2 Reiter + Split + Farbschema nach Neustart wieder da | Bestanden | `session.json`, Bildschirmfoto |
| T-06 | Sitzung mit Verzeichnis `/tmp/a b/Ü` | Fehlgeschlagen | QA-05 |
| T-07 | Maximieren, Super+Pfeil, Fenster per Tastatur verkleinern | Teilweise | Mindestbreite ≈ 550 px (QA-13) |
| T-08 | `exit` / Strg+D → Leiste „Prozess beendet“ mit *Neu starten* | Bestanden (Gestaltung QA-11) | `03-prozess-beendet-exit0.png` |
| T-09 | Profil „Neu starten“, Reiter schließen | Fehlgeschlagen | QA-03 |
| T-10 | Kaputte Profil- und Palettendatei | Bestanden (Gestaltung QA-15) | `16-kaputte-dateien.png` |
| T-11 | Bereich mit laufendem `sleep` schließen → Nachfrage, Enter = *Abbrechen* | Bestanden | `24-schliessen-nachfrage.png` |
| T-12 | Zweite Instanz übergibt Umgebung (`QA_ENV=gesetzt parkett --tab -- …`) | Fehlgeschlagen | QA-08 |

### Terminal und Shell

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-20 | ANSI 16/256/Truecolor, fett, kursiv, unterstrichen, wellig, invers, durchgestrichen, überstrichen, blinkend | Bestanden | `02-farben-unicode.png` |
| T-21 | Umlaute, Kyrillisch, Griechisch, CJK, Emoji, Rahmen, Mathe, RTL | Bestanden | dto. |
| T-22 | ZWJ-Emoji, Flaggen | Teilweise | zerfallen – VTE-Grenze (wie Phase 5) |
| T-23 | 600 Zeichen lange Zeile umbrechen | Bestanden | dto. |
| T-24 | `top`, `less` mit Suche, `nano` (Speichern mit Umlauten), `htop` | Bestanden | Bildschirmfotos, Dateiinhalt |
| T-25 | Strg+Z, `jobs`, `fg`, Strg+C (rc=130), Strg+L, Strg+W, Pos1/Ende | Bestanden | `04-jobsteuerung.png` |
| T-26 | Bash mit Bracketed Paste: Mehrzeiliges wird markiert eingefügt, nicht ausgeführt | Bestanden | `11-einfuege-warnung.png` |
| T-27 | Hohe Last (`while :; do cat 200 000 Zeilen; done`): UI reagiert, Teilen geht, D-Bus-Antwort ≤ 10 ms | Bestanden | `05-hohe-last-teilen.png` |
| T-28 | Cursor nach Verkleinern/Vergrößern mit umbrochenem Prompt | Teilweise | `23-cursor-nach-umbruch.png`, QA-20 |

### Tastatur und Eingabe

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-30 | Strg+Umschalt+T, Strg+Bild↑/↓, Alt+Pfeil, Strg+Umschalt+X, Strg+Umschalt+P, Strg+Umschalt+L, Strg+?, Strg+, | Bestanden | `10-kuerzel.png` |
| T-31 | Strg+Alt+R/D (Teilen), Strg+Umschalt+W, Strg+Umschalt+F | Bestanden | Bildschirmfotos |
| T-32 | F2 in `htop` | Fehlgeschlagen | QA-01, `08-f2-in-htop.png` |
| T-33 | F10 in `htop` | Fehlgeschlagen | QA-01, `09-f10-in-htop.png` |
| T-34 | Konflikte mit GNOME-Standardkürzeln (gsettings der VM) | Bestanden | keine Überschneidung |
| T-35 | Tastaturfokus nach Schließen des Hauptmenüs mit Esc | Fehlgeschlagen | QA-09 |
| T-36 | Andere Tastaturbelegungen, Eingabemethoden (IBus) | Nicht getestet | nur DE |

### Zwischenablage

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-40 | Markieren per Ziehen, Strg+Umschalt+C, Strg+Umschalt+V | Bestanden | `wl-paste` |
| T-41 | Zweizeiliger Text → Warnung mit Vorschau, Fokus auf *Abbrechen* | Bestanden | `11-einfuege-warnung.png` |
| T-42 | Umschalt+Einfg → gleiche Warnung (QA.md sagt noch: ohne) | Bestanden | Bildschirmfoto |
| T-43 | Eine Zeile mit Zeilenumbruch in `dash` einfügen | Fehlgeschlagen | QA-04, `12-einfuegen-ohne-warnung-dash.png` |
| T-44 | Mittelklick (Primärauswahl) | Nicht anwendbar | in Ubuntu abgeschaltet |
| T-45 | Große Textmengen in der Zwischenablage | Nicht getestet | – |

### Suche, Links, Kontextmenü

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-50 | Suche inkrementell, Enter = darüber, kein Treffer = rotes Feld, Esc | Bestanden | `18-suche.png` |
| T-51 | OSC-8-Link: Tooltip zeigt Ziel | Bestanden | `19-links.png` |
| T-52 | URL: Kontextmenü *Link öffnen/kopieren*, Strg+Klick startet Firefox | Bestanden | Prozess `firefox https://www.gnome.org` |
| T-53 | Dateipfade als Links | Nicht anwendbar | nicht vorgesehen (Idee in G) |
| T-54 | `file:`-Link mit fremdem Rechner | Nicht getestet | Phase-6-Fix, nur Code |

### Einstellungen und Darstellung

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-60 | Einstellungsdialog: Allgemein, Profile, Vorlagen, Kürzel | Bestanden | `20-einstellungen.png`, `21-profil.png` |
| T-61 | Farbschema wechseln wirkt sofort, bleibt nach Neustart | Bestanden | `standard.toml` |
| T-62 | Systemstil hell ↔ dunkel live | Bestanden | `26-dunkel-splits.png` |
| T-63 | Bereichstitel „Als Zeile“ live umschalten | Bestanden | `31-ist-zeile-dunkel.png` |
| T-64 | Textskalierung 1,5 und hoher Kontrast | Teilweise | Oberfläche folgt; Farbschema bleibt (bekannt, BACKLOG) |
| T-65 | Kontrast der 14 Farbschemata (berechnet) | Teilweise | QA-10 |
| T-66 | Sixel | Nicht anwendbar | Ubuntu-VTE ohne Sixel, Schalter korrekt ausgegraut |
| T-67 | Kontextfarbe root/SSH | Nicht getestet | sudo braucht Passwort; Phase 4/5 getestet |

### Fehlerfälle, Stabilität, Ressourcen

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-70 | 5 Runden: je 20 Terminals öffnen und schließen | Bestanden | Tabelle in F |
| T-71 | Vorlage mit Sonderzeichen im Startbefehl | Fehlgeschlagen | QA-07 |
| T-72 | Teilen nach `cd "/tmp/a b/Ü"` | Fehlgeschlagen | QA-05 |
| T-73 | OSC 7 mit fremdem Rechnernamen, dann teilen | Fehlgeschlagen | QA-06 |
| T-74 | GLib-CRITICAL beim Schließen (Review-Fund F2) | Bestanden | im Szenario QA-03 keine Meldung ≤ Warnung im Journal der App |
| T-75 | Dauerlauf > 15 min | Nicht getestet | Phase 5 (D2) in Fedora |

### Barrierefreiheit

| ID | Test | Status | Nachweis |
|---|---|---|---|
| T-80 | AT-SPI: Knöpfe, Reiter, Terminals benannt | Teilweise | QA-18 |
| T-81 | Terminaltext per AT-SPI lesbar | Teilweise | nur sichtbarer Bildschirm (84 Zeichen), nicht der Verlauf – VTE-Grenze |
| T-82 | Orca mit Sprachausgabe | Nicht getestet | nicht hörbar prüfbar |
| T-83 | Bedienung ohne Maus (Palette, Kürzel, F10, Dialoge) | Teilweise | QA-09 |

---

## D. Fehlerliste

Nach Priorität. Jeder Eintrag nennt Umgebung, Schritte und Belege. Die Umgebung ist bei allen „VM
`ubuntu26.04`, siehe B“, sofern nicht anders angegeben.

### QA-01 – F2 und F10 erreichen Terminalprogramme nicht · hoch · reproduziert

- **Kategorie:** Funktion / Usability
- **Ausgangszustand:** Standardkürzel, ein Terminal mit Fokus.
- **Schritte:**
  1. `htop` starten.
  2. F2 drücken.
  3. Esc, dann F10 drücken.
- **Erwartet:** htop öffnet *Setup* (F2) bzw. beendet sich (F10).
- **Tatsächlich:**
  - F2 öffnet „Reiter umbenennen“.
  - F10 öffnet das Hauptmenü von Parkett.
  - htop läuft nach F10 weiter (`pgrep` geprüft).
- **Nachweis:** `08-f2-in-htop.png`, `09-f10-in-htop.png`.
- **Auswirkung:**
  - `htop` (F2 Setup, F10 Beenden), `mc` (F2 Menü, F10 Beenden) und `nano` (F2 Beenden) sind nur eingeschränkt bedienbar. Das trifft Admins täglich.
  - Für Einsteiger wirkt es, als hinge das Programm.
- **Ursache:**
  - F2: Standardkürzel von `win.rename-tab` (`app/src/shortcuts.rs:38`), als Shortcut in der Capture-Phase vor VTE.
  - F10: GTK öffnet mit F10 das Hauptmenü der Kopfleiste. Das ist das Standardverhalten für einen primären Menüknopf.
- **Empfehlung:**
  - *Reiter umbenennen* ohne Standardkürzel (Doppelklick und Palette bleiben).
  - F10 abfangen, solange ein Terminal Fokus hat (GNOME Terminal hat dafür eine Einstellung, Standard aus).
  - Allgemein: keine Funktionstaste ohne Modifikator als Standard. Die Aufnahme eigener Kürzel lehnt reine F-Tasten schon ab (`shortcuts.rs:195`), der Standard sollte es auch.
- **Regressionstest:** VM-Rauchtest um `htop`-Start und F2/F10 per QMP erweitern; prüfen, dass htop sich mit F10 beendet.

### QA-02 – Schwebender Bereichstitel verdeckt Prompt, Eingabe und Cursor · hoch · reproduziert

- **Kategorie:** Usability / Design
- **Ausgangszustand:** Standard „Bereichstitel: Schwebend“, Ubuntu-Prompt `testt@testt-Standard-PC-Q35-ICH9-2009:~$` (40 Zeichen), Bereich breiter als 380 px.
- **Schritte:**
  1. *Rechts teilen*.
  2. Im neuen Bereich `echo sichtbar` tippen.
- **Erwartet:** Eingabe und Cursor sind sichtbar.
- **Tatsächlich:**
  - Der Titel liegt über Spalte ≈ 26–55 der ersten Zeile.
  - Die Eingabe scheint nur blass durch.
  - Auch `htop` verliert Teile der CPU-Leiste.
- **Nachweis:** `06-titel-verdeckt-prompt.png`, `07-titel-verdeckt-eingabe.png`, `08-f2-in-htop.png`.
- **Auswirkung:**
  - In jedem frisch geteilten Bereich tippt man zunächst blind.
  - Betroffen sind alle Nutzer mit Splits; der Standard-Prompt von Ubuntu, Debian und Fedora ist lang.
- **Ursache:** Der Titel ist ein `GtkOverlay` über dem VTE (`app/src/pane.rs:144–168`). Unter 380 px Breite wird nur der Text ausgeblendet (Phase-5-Fix), die Knöpfe bleiben.
- **Empfehlung:** Gestaltungsentscheidung, siehe H (drei Varianten). Ganz ohne Umbau sofort möglich: Standard auf „Als Zeile“.
- **Regressionstest:** Bildschirmfoto nach *Rechts teilen* mit 40-Zeichen-Prompt; Cursorposition (VTE) darf nicht unter dem Titel liegen.

### QA-03 – Geschlossener Reiter mit Profil „Neu starten“ startet sein Programm unsichtbar neu · hoch · reproduziert

- **Kategorie:** Funktion (Ressourcen, ggf. Sicherheit)
- **Ausgangszustand:** Profil `qa-neustart` mit `command = "bash --norc -c \"…; exec sleep 1000\""` und `exit_action = "restart"`.
- **Schritte:**
  1. `parkett --tab -p qa-neustart`.
  2. Reiter schließen (`close-tab`).
  3. 13 s und 64 s warten.
  4. Kinder von Parkett ansehen.
- **Erwartet:** Nach Ablauf der Rückgängig-Frist ist der Prozess beendet, und es gibt keinen neuen.
- **Tatsächlich:**
  - `sleep` (PID 9760) wurde beendet.
  - Um 09:06:03 lief ein **neuer** `sleep 1000` (PID 9823). Er war um 09:06:59 noch da und verschwand erst, als ein anderer Reiter geschlossen wurde.
- **Nachweis:** Prozesslisten im Protokoll (Zeitstempel 09:05:55, 09:06:09, 09:06:59).
- **Auswirkung:**
  - Typischer Einsatz von „Neu starten“ ist `ssh server` oder ein Log-Tail. Nach dem Schließen baut Parkett die Verbindung unsichtbar neu auf.
  - Der geschlossene Reiter hält VTE-Puffer und Verlauf im Speicher.
- **Ursache:**
  - Die abgelaufene Rückgängig-Einblendung hält den `Tab` fest (`window.rs:408–420`).
  - Das Beenden per SIGHUP löst `child-exited` aus. Der Handler startet bei `ExitAction::Restart` neu, weil die Shell länger als 2 s lief (`terminal.rs:509`).
  - `terminate()` merkt sich nicht, dass bewusst beendet wurde.
- **Empfehlung:**
  - In `terminate()` ein Flag setzen, das Neustart und Leiste unterdrückt.
  - Nach Ablauf der Frist die Einblendung und den Reiter freigeben.
- **Regressionstest:**
  - Unit-Test für „bewusst beendet → kein Neustart“.
  - Im Rauchtest Profil mit `restart`: Reiter schließen, 10 s warten, Prozessanzahl prüfen.

### QA-04 – Einfüge-Warnung lässt eine Zeile mit Zeilenumbruch durch · mittel · reproduziert

- **Kategorie:** Sicherheit
- **Ausgangszustand:** „Vor mehrzeiligem Einfügen warnen“ an, Shell ohne Bracketed Paste (`dash`, alte Server, Python-REPL).
- **Schritte:**
  1. Ausgegebene Zeile `echo AUSGEFUEHRT-OHNE-RUECKFRAGE` dreifach anklicken. VTE übernimmt den Zeilenumbruch.
  2. Strg+Umschalt+C.
  3. `dash` starten.
  4. Strg+Umschalt+V.
- **Erwartet:** Warnung, weil der Text einen Zeilenumbruch enthält und sofort ausgeführt würde.
- **Tatsächlich:** keine Warnung, der Befehl läuft sofort.
- **Nachweis:** `12-einfuegen-ohne-warnung-dash.png`.
- **Auswirkung:** Klassisches „Pastejacking“: Eine Webseite legt `curl … | sh⏎` in die Zwischenablage, ein Klick genügt. Bash 5.3 mit Bracketed Paste ist geschützt, `sh`, `dash` und Server-Shells ohne diese Option nicht.
- **Ursache:** Vor dem Zählen der Zeilen wird der Umbruch am Ende abgeschnitten (`terminal.rs:919–925`); eingefügt wird aber der ganze Text.
- **Empfehlung:** Jeden Text mit `\n` oder `\r` als „wird ausgeführt“ behandeln. Alternativ den abschließenden Umbruch beim Einfügen entfernen, wie GNOME Console.
- **Regressionstest:** Zeilenzählung nach `core` verschieben und testen (`"a\n"` → Warnung).

### QA-05 – Verzeichnisse mit Leerzeichen oder Umlauten gehen verloren · mittel · reproduziert

- **Kategorie:** Funktion
- **Schritte:**
  1. `cd "/tmp/a b/Ü"`.
  2. *Unten teilen*.
  3. Im neuen Bereich `pwd`.
- **Erwartet:** `/tmp/a b/Ü`.
- **Tatsächlich:**
  - `/home/testt`.
  - In `session.json` steht `"subdir": "tmp/a%20b/%C3%9C"`, die Sitzung stellt das Verzeichnis also auch nicht wieder her.
  - Kontrolle: `/usr/share` funktioniert.
- **Nachweis:** `13-verzeichnis-leerzeichen.png`, Auszug `session.json`.
- **Auswirkung:** „Neue Terminals im aktuellen Verzeichnis“ versagt still bei allen Pfaden mit Leerzeichen oder Nicht-ASCII-Zeichen. Typisch sind `~/Schreibtisch/Neuer Ordner` oder `Übersetzungen`.
- **Ursache:** `Terminal::cwd()` nimmt `Uri::path()` der prozentkodierten OSC-7-URI (`terminal.rs:226–231`).
- **Empfehlung:** `gio::File::for_uri(uri).path()` bzw. `glib::Uri::unescape_string`. Mit QA-06 zusammen beheben.
- **Regressionstest:** Unit-Test mit `file:///tmp/a%20b/%C3%9C`.

### QA-06 – Fremder Rechnername in OSC 7 setzt das lokale Startverzeichnis · mittel · reproduziert

- **Kategorie:** Sicherheit / Funktion
- **Schritte:**
  1. `printf '\e]7;file://entfernter-server/etc\a'; PROMPT_COMMAND= bash --norc`. Das entspricht einer SSH-Sitzung auf einen Server mit `vte.sh`.
  2. *Unten teilen*.
  3. `pwd`.
- **Erwartet:**
  - Der Pfad eines anderen Rechners wird ignoriert.
  - Ersatz: das echte Verzeichnis des lokalen Prozesses (`/proc/<pid>/cwd`) oder das Home-Verzeichnis.
- **Tatsächlich:** Der neue lokale Bereich startet in `/etc`.
- **Nachweis:** `14-osc7-fremder-rechner.png`.
- **Auswirkung:**
  - Nach `ssh server` und `cd /srv/app` startet *Teilen* lokal in `/srv/app` (falls vorhanden) oder still im Home-Verzeichnis.
  - Jede Ausgabe (`cat datei`) kann das Startverzeichnis setzen, etwa auf ein vorbereitetes Git-Repo, dessen Hooks oder `core.fsmonitor` ein Git-Prompt ausführt.
  - Die Ausnutzung braucht mehrere Schritte. Der Mechanismus ist aber da.
- **Ursache:** `terminal.rs:226–231` prüft den Host nicht. Die Prüfung `is_local_file_uri` für Links gibt es schon.
- **Empfehlung:** Nur URIs ohne Host oder mit lokalem Hostnamen übernehmen, sonst `/proc/<pid>/cwd`.
- **Regressionstest:** Unit-Test der Host-Prüfung, VM-Schritt wie oben.

### QA-07 – Startbefehle einer Vorlage unsichtbar, *Öffnen* vorausgewählt · mittel · reproduziert

- **Kategorie:** Sicherheit / Usability
- **Schritte:**
  1. Vorlage mit `command = "make -j8 && ./start.sh 2>&1 <eingabe"` anlegen.
  2. Palette: `#QA`, Eingabe.
- **Erwartet:** Unter „Startbefehle ausführen“ steht der Befehl.
- **Tatsächlich:**
  - Der Untertitel ist leer.
  - Der Schalter steht auf *an*, *Öffnen* hat den Fokus.
- **Nachweis:** `15-vorlage-startbefehl-unsichtbar.png`.
- **Auswirkung:** Man führt Befehle aus, die man nicht gesehen hat. Vorlagen kommen nur aus dem eigenen Konfigurationsordner, das Risiko ist daher begrenzt. Bei geteilten Dotfiles oder kopierten Vorlagen ist es aber real.
- **Ursache:**
  - Der Untertitel einer `AdwActionRow` ist Pango-Markup; `&` und `<` werden nicht maskiert (`window.rs:892`).
  - Gleiches Muster: `window.rs:880` (Verzeichnis), `prefs.rs:155` (Schemaname), `prefs.rs:791`, Toast-Titel `window.rs:393` (Reitertitel aus OSC 0).
- **Empfehlung:** `glib::markup_escape_text` überall, wo fremder Text in Markup landet, oder `use_markup(false)`.
- **Regressionstest:** Vorlage mit `&&` öffnen; Untertitel muss den Befehl zeigen.

### QA-08 – Zweiter Aufruf verliert seine Umgebung · mittel · reproduziert

- **Kategorie:** Funktion
- **Schritte:**
  1. Parkett läuft.
  2. `QA_ENV=gesetzt parkett --tab -- bash -c 'echo QA_ENV=${QA_ENV:-FEHLT}; exec bash'`.
- **Erwartet:** `QA_ENV=gesetzt`.
- **Tatsächlich:** `QA_ENV=FEHLT`.
- **Nachweis:** `17-umgebung-zweite-instanz.png`.
- **Auswirkung:**
  - `xdg-terminal-exec` aus Anwendungen mit eigener Umgebung, Python-`venv`, `SSH_AUTH_SOCK` oder `DISPLAY`-Wechsel landen nicht im neuen Terminal.
  - GNOME Terminal und Ptyxis übernehmen die Umgebung des Aufrufers.
- **Ursache:** `ApplicationCommandLine::environ()` wird in `main.rs:128–197` nicht ausgewertet.
- **Empfehlung:** Umgebung des Aufrufs an den Spawn des neuen Bereichs durchreichen (nur für per Befehlszeile geöffnete Terminals).
- **Regressionstest:** Schritt oben in den Rauchtest.

### QA-09 – Fokus bleibt nach dem Hauptmenü auf dem Menüknopf · mittel · reproduziert

- **Kategorie:** Usability / Barrierefreiheit
- **Schritte:**
  1. F10 (oder Klick auf ☰).
  2. Esc.
  3. `htop⏎` tippen.
- **Erwartet:** Eingabe landet im Terminal.
- **Tatsächlich:**
  - AT-SPI meldet `[toggle button] 'Hauptmenü' focused`.
  - Die Buchstaben gehen verloren, Enter öffnet das Menü erneut.
  - `htop` lief nicht.
- **Nachweis:** AT-SPI-Ausgabe, `27-fokus-bleibt-im-menue.png`.
- **Auswirkung:** Wer per Tastatur ins Menü geht, verliert danach Eingaben. Das ist besonders tückisch, weil Enter das Menü wieder öffnet.
- **Ursache (vermutet):** GTK gibt den Fokus nach dem Schließen eines Popovers an den Knopf zurück; Parkett setzt ihn nicht aufs Terminal. Vergleich mit Ptyxis nicht gemacht.
- **Empfehlung:** Bei `closed` des Popovers den Fokus auf das aktive Terminal legen. Das gilt auch für Reiter-Übersicht, Neu-Menü und Suche.
- **Regressionstest:** AT-SPI-Prüfung „Terminal hat Fokus“ nach Menü + Esc.

### QA-10 – Farbschema „One“ dunkel: Text kaum lesbar · mittel · reproduziert (berechnet)

- **Kategorie:** Design / Barrierefreiheit
- **Nachweis:** `data/palettes/one.palette`, `[Dark]`: `Foreground=#5C6370` auf `Background=#1E2127`, Kontrast **2,7:1** (WCAG AA verlangt 4,5:1). `#5C6370` ist in One Dark die Farbe für Kommentare; der übliche Vordergrund ist `#ABB2BF`.
- **Weitere Rechenergebnisse** (Farben unter 3:1 zum Hintergrund):
  - In den hellen Fassungen liegen bei Nord, Everforest, Catppuccin, GitHub, Tango und Solarized 7–12 der 16 Farben unter 3:1. Bei Weiß/Hellgrau (7, 15) ist das üblich.
  - Bei Grün, Gelb und Cyan ist das ein echtes Lesbarkeitsproblem, z. B. Tango hell `33`/`93`.
  - „Hoher Kontrast“ hell hat 5 Farben unter 3:1.
- **Empfehlung:**
  - Vordergrund von One dunkel korrigieren.
  - „Hoher Kontrast“ so anpassen, dass alle Farben außer 0/7/8/15 mindestens 4,5:1 erreichen.
  - Optional die Kontrastprüfung als Unit-Test für alle Paletten.
- **Regressionstest:** Test „Vordergrund ≥ 4,5:1 in jeder Palette“.

### QA-11 – Normales Ende (`exit`, Strg+D) erscheint als Fehler · niedrig · reproduziert

- **Kategorie:** Usability / Design
- **Beobachtung:**
  - Nach `exit` bleibt der Bereich stehen, mit Warnsymbol und rötlicher Leiste „Prozess beendet“.
  - Im einzigen Reiter schließt sich das Fenster nicht.
- **Nachweis:** `03-prozess-beendet-exit0.png`.
- **Einordnung:** Das ist der bewusste Standard `exit_action = "hold"`. GNOME Console, Ptyxis und Tilix schließen bei Exit-Code 0. Einsteiger erwarten, dass `exit` das Fenster schließt.
- **Empfehlung:**
  - Standard „Bereich schließen bei Erfolg“.
  - Bei Code 0 eine neutrale Leiste ohne Warnsymbol.
  - Entscheidung bei Christoph.

### QA-12 – Kürzel in Tooltips fest eingetragen, gemischte Sprache · niedrig · reproduziert

- **Beobachtung:**
  - „Bereich maximieren (Strg+Umschalt+X)“, „Bereich schließen …“ und die Suchknöpfe stehen fest im Code (`pane.rs:148/153/191/196`). Sie folgen eigenen Kürzeln nicht.
  - Daneben „Rechts teilen (Ctrl+Alt+R)“ aus `accelerator_get_label`. Ohne GTK-Übersetzung ist das englisch.
- **Empfehlung:** Alle Tooltips aus `shortcuts::label()` erzeugen und bei Änderung neu setzen.

### QA-13 – Mindestbreite des Fensters ≈ 550 px · niedrig · reproduziert

- **Beobachtung:** Per Tastatur (Alt+F8) ließ sich das Fenster nicht schmaler als ≈ 550 px ziehen (aus dem Bildschirmfoto gemessen). Die Kopfleiste mit 7 Knöpfen und Titel bestimmt die Breite.
- **Nachweis:** `22-mindestbreite.png`.
- **Einordnung:** Die HIG empfehlen 360 px. Für ein Terminal ist das relevant beim Kacheln auf kleinen Bildschirmen (z. B. drei Spalten auf 1366 px).
- **Empfehlung:** Unter ≈ 600 px Teilen-Knöpfe ins Neu-Menü verschieben (`AdwBreakpoint`).

### QA-14 – Blinkender Text kostet im Leerlauf 7,8 % CPU · niedrig · reproduziert (VM)

- **Messung:**
  - 7 Terminals, Leerlauf 20 s: 0,40–0,65 %.
  - Gleicher Zustand mit sichtbarem SGR-5-Text („blink“): 7,8 %.
  - Mit `cursor-blink=false` unverändert.
- **Einordnung:** Das ist Softwarerendering in der VM; auf echter GPU deutlich weniger (nicht gemessen). VTE kann blinkenden Text abschalten oder auf den Fokus beschränken (`set_text_blink_mode`).
- **Empfehlung:** `TextBlinkMode::Focused` als Standard, wie GNOME Terminal.

### QA-15 – Meldung „Nicht lesbare Dateien“ schwer lesbar · niedrig · reproduziert

- **Beobachtung:**
  - Der TOML-Fehler mit Zeiger `^` wird zentriert in Proportionalschrift gezeigt; der Zeiger steht an der falschen Stelle.
  - Die Einblendung verschwindet nach wenigen Sekunden, der Hinweis lässt sich danach nicht mehr aufrufen.
  - Die Palettenmeldung „Color0 in [Palette] fehlt“ nennt den falschen Abschnitt (gefehlt haben `[Light]`/`[Dark]`).
- **Nachweis:** `16-kaputte-dateien.png`.
- **Empfehlung:**
  - Details links ausgerichtet in Festbreitenschrift.
  - Einblendung ohne Zeitablauf.
  - Meldung „Abschnitt [Light] fehlt“.

### QA-16 – Profil-Editor öffnet mit markiertem Namen · niedrig · reproduziert

- **Beobachtung:** Beim Öffnen eines Profils hat das Namensfeld den Fokus, der Name ist markiert. Ein Tastendruck überschreibt ihn (und speichert sofort).
- **Nachweis:** `21-profil.png`.
- **Empfehlung:** Fokus auf die Seite, nicht ins Feld.

### QA-17 – Gleich lautende Reiter und Titel · niedrig · reproduziert

- **Beobachtung:** Fenstertitel, Untertitel, Reiter und Bereichstitel zeigen denselben Shell-Titel. Vier Reiter sehen gleich aus.
- **Nachweis:** `26-dunkel-splits.png`.
- **Empfehlung:** H, Hinweis „Kurze Reiter-Beschriftung“.

### QA-18 – Kleinere Screenreader-Namen · niedrig · reproduziert

- **Beobachtung:**
  - Der Reiter-Übersicht-Knopf heißt nur „4“ (Anzahl), die Beschreibung trägt den Sinn.
  - Der Neu-Knopf hat als Beschreibung „Weitere Möglichkeiten“ (interner Pfeilteil des AdwSplitButton).
  - Die Bereichsknöpfe lesen das Kürzel im Namen mit („Bereich schließen (Strg+Umschalt+W)“).
- **Empfehlung:**
  - Name „Reiter-Übersicht, 4 Reiter“.
  - Kürzel nur in den Tooltip.

### QA-19 – Suche ohne Trefferzahl · niedrig · reproduziert

- **Beobachtung:** Nur der aktuelle Treffer ist markiert; es gibt keine Anzeige „3 von 11“. Das ist eine Grenze der VTE-API, die keine Trefferliste liefert.
- **Empfehlung:** Optional selbst zählen (Text des Verlaufs per `text_range_format`). Aufwand und Nutzen abwägen.

### QA-20 – Cursor nach Umbruch an falscher Stelle · niedrig · noch zu untersuchen

- **Beobachtung:**
  - Fenster schmal gezogen (Prompt bricht um), dann maximiert.
  - In den nicht aktiven Bereichen steht der hohle Cursor mitten im Prompt (Spalte 24) statt dahinter.
- **Nachweis:** `23-cursor-nach-umbruch.png`.
- **Einordnung:** Vermutlich VTE-Umbruch plus bash-Neuzeichnen, also nicht Parkett. Nicht gegen Ptyxis gegengeprüft.

### Nur im Code bestätigt (Review), nicht in der VM ausgelöst

| ID | Fund | Ort | Schwere |
|---|---|---|---|
| R-01 | `update_page` für Reiter, die nicht mehr in der TabView sind; `view.page()` liefert NULL. Im Szenario QA-03 **nicht** ausgelöst (keine CRITICAL); weitere Pfade: Reiter in anderes Fenster ziehen, Befehlsende im geschlossenen Reiter | `window.rs:429–486` | mittel (vermutet) |
| R-02 | Rauchtest übersieht GLib-CRITICALs, weil `journalctl -o cat` das Präfix nicht zeigt | `build-aux/vm/smoke.sh:46` | mittel |
| R-03 | Umgebungsvariablen-Feld zerstört Werte mit Leerzeichen/Anführungszeichen (`JAVA_OPTS=-Xmx1g -Dx=1` → zwei Einträge) | `prefs.rs:478/488` | mittel |
| R-04 | `file:///…`-Links per OSC 8 aus einer SSH-Sitzung öffnen lokale Dateien mit dem Standardprogramm (Tooltip zeigt Ziel) | `terminal.rs:894–901` | niedrig–mittel |
| R-05 | Rückgängig nach normalem Ende (Profil „schließen“) bringt einen toten Bereich ohne Leiste | `terminal.rs:497`, `tab.rs:452` | niedrig |
| R-06 | `GIO_LAUNCHED_DESKTOP_FILE(_PID)` erreicht alle Shells (GNOME Terminal entfernt sie) | `.desktop`, `main.rs` | niedrig |
| R-07 | Trenner ziehen speichert die Sitzung nicht; Speicher-Timer startet bei jeder Titeländerung neu | `tab.rs:683`, `session.rs:64` | niedrig |
| R-08 | `ratio = nan` in einer Vorlage → `null` in `session.json` → Sitzung verloren | `template.rs:106` | niedrig |
| R-09 | Startbefehl wird sofort getippt; bei ssh-Profil mit Passwortabfrage landet er dort | `terminal.rs:458` | niedrig (vermutet) |
| R-10 | `-e "htop -d 5"` als ein Argument wird nicht zerlegt; `--tab=x` still akzeptiert | `cli.rs:52` | niedrig |
| R-11 | `ssh -vp 22 host` → Kontext-Chip „22“ | `context.rs:45` | niedrig |
| R-12 | Profile (Umgebung mit Tokens) mit Standard-umask: 0644 auf Arch, 0664 in der Ubuntu-VM (umask 002) | `core` `*Store::save` | niedrig |
| R-13 | Doku: QA.md sagt, Umschalt+Einfg warne nicht – tut es inzwischen; Testanzahl veraltet (53 + 2 + 1) | `docs/QA.md`, `ARCHITEKTUR.md` | niedrig |

---

## E. Usability- und Designbewertung

### Bedienbarkeit

**Einsteiger:**

- Start, Tippen, Kopieren und Einfügen funktionieren wie erwartet.
- Die Kopfleiste ist mit sieben Symbolen übersichtlich; die Tooltips nennen Kürzel.
- Schwach:
  - `exit` schließt nichts und sieht nach Fehler aus (QA-11).
  - F2 und F10 verhalten sich in Programmen „falsch“ (QA-01).
  - Im geteilten Bereich sieht man die eigene Eingabe nicht (QA-02).
- Das Hauptmenü enthält nur 5 Einträge. Palette, Layout-Modus und Vorlagen fehlen dort und sind nur über Kürzel oder Neu-Menü zu finden.

**Erfahrene Nutzer mit mehreren Aufgaben:**

- Splits, Fokus per Alt+Pfeil, Layout-Modus und Palette sind schnell und konsistent.
- Rückgängig beim Schließen ist ein echter Vorteil gegenüber anderen Terminals.
- Schwach:
  - Gleich lautende Reiter (QA-17).
  - Kein Hinweis, welcher Reiter welcher ist, ohne die Übersicht zu öffnen.

**Admins und Entwickler:**

- Profile, Kontextfarbe, Vorlagen mit Startbefehlen und Befehlszeile decken den Alltag gut ab.
- Schwach:
  - F-Tasten (QA-01).
  - Umgebung bei `parkett --tab` (QA-08).
  - Verzeichnisse mit Leerzeichen (QA-05).
  - Unsichtbare Startbefehle (QA-07).
  - Das Umgebungsfeld im Profil (R-03).

**Konsistenz Tastatur/Maus:**

- Fast alles geht auf beiden Wegen.
- Ausnahmen:
  - Der Fokus bleibt nach Menüs im Kopf (QA-09).
  - *Reiter umbenennen* hat eine F-Taste, *Als Vorlage speichern* gar kein Kürzel.

### Gestaltung

**Stärken:**

- Sauber auf libadwaita gebaut: AdwTabBar, AdwPreferencesDialog, AdwAlertDialog mit sicherer Vorauswahl, AdwToast.
- Hell/Dunkel und Akzentfarbe folgen live.
- Farbschema-Kacheln mit heller und dunkler Hälfte sind klar.
- Die Fokuslinie (2 px Akzent) ist ein gutes, ruhiges Signal.

**Schwächen:**

1. **Hierarchie der Titel.** Drei Ebenen wiederholen denselben 40-Zeichen-Text. Das widerspricht dem gewünschten „klare Hierarchie statt Wiederholung“.
2. **Schwebende Elemente über dem Inhalt.** Neben dem Bereichstitel auch die Leiste „Maximiert · 1 Bereich ausgeblendet“ (`10-kuerzel.png`, Mitte oben). Beide liegen über Terminaltext.
3. **Abblendung kaum sichtbar.** Inaktive Bereiche werden nur im Text gedämpft. Gemessen ist der Hintergrund `#2E3335` gegen `#2E3436`. In Tango dunkel ist der Unterschied zwischen aktiv und inaktiv vor allem die Linie.
4. **Fehlerleiste für Normalfälle.** Warnsymbol und Rotton auch bei Exit-Code 0 (QA-11).
5. **Fehlerdialog** zentriert mit Proportionalschrift (QA-15).
6. **Lange Kombi-Werte abgeschnitten** in den Einstellungen („Verzeichnis des aktive…“). Bei 1280 px Bildschirmbreite fällt das auf.

Ubuntu-spezifisch: Das Yaru-Orange als Akzent passt; die Fokuslinie übernimmt es korrekt.

---

## F. Sicherheit und Performance

### Messwerte (VM, Softwarerendering)

| Messung | Ergebnis | Bedingungen |
|---|---|---|
| Start bis Shell läuft | 141–161 ms (4 Läufe) | `parkett -- sh -c 'date …'`, warm, Sitzungswiederherstellung an |
| Speicher 1 Terminal | 219 MB RSS | erster Start, 1280×800 |
| Speicher 7–8 Terminals | 289–345 MB RSS | 1920×1033, je nach Verlauf |
| Leerlauf-CPU | 0,40–0,65 % | 7 Terminals, 20 s |
| Leerlauf-CPU mit blinkendem Text | 7,8 % | QA-14 |
| CPU unter Dauerausgabe | 88,6 % eines Kerns | `cat` in Schleife, llvmpipe |
| D-Bus-Antwort unter Dauerausgabe | ≤ 10 ms | `org.gtk.Actions.List`, 3 Messungen |
| Durchsatz `seq` 3 Mio. Zeilen | Parkett 1,67–1,79 s · Ptyxis 1,55–1,63 s | maximiert, gleiche VM, je 3 Läufe |
| Durchsatz 20 MB farbiger Text | Parkett 0,69–0,76 s · Ptyxis 0,80–0,94 s | dto. |
| Öffnen/Schließen 5 × 20 Terminals | Shells immer zurück auf 8; RSS 316 → 365 MB nach Runde 1, danach +0,2 MB je Runde; FDs 44 → 56 konstant; Threads 42 | `new-tab` + `split-right`, `close-tab`, 10 s Pause |

Speicher Parkett gegen Ptyxis lässt sich hier **nicht** vergleichen: Ptyxis hatte 1 Terminal,
Parkett 8. Die Zunahme nach der ersten Runde ist vermutlich Allokator und Glyphen-Cache, kein Leck.
Bewiesen ist das nicht. Ein Messlauf über Stunden fehlt.

### Sicherheit

**Geprüft und unauffällig:**

- Links nur mit erlaubten Schemata; `file:` mit fremdem Rechner wird blockiert (Phase 6).
- Der Tooltip zeigt das echte OSC-8-Ziel.
- Die Nautilus-Erweiterung startet ohne Shell (argv), keine Injektion.
- `session.json` speichert keine Befehle.
- Kontext-Chip aus `/proc`, nicht aus Terminalausgabe.
- Keine Netzwerkzugriffe von Parkett selbst. Geprüft ist das nur im Code, nicht mit Netzmitschnitt.

**Befunde:** QA-04, QA-06, QA-07 (reproduziert), R-04, R-06, R-12 (Code).

**Escape-Sequenzen zur Täuschung:**

- Ein fremder Titel (OSC 0) kann den SSH/root-Chip als Text nachahmen, aber nicht entfernen.
- Er landet ungefiltert als Markup in der Rückgängig-Einblendung (QA-07, Folgefund).
- OSC 7 kann das Startverzeichnis setzen (QA-06).
- Abhängigkeiten auf bekannte Schwachstellen (`cargo audit`) sind **nicht geprüft**; das Werkzeug ist nicht installiert.

**Ausdrücklich:** Dass keine weiteren Lücken gefunden wurden, heißt nicht, dass Parkett sicher ist.
Nicht geprüft wurden Fuzzing der Paletten- und TOML-Parser, Tests mit bösartigen
Escape-Sequenzen über VTE hinaus und die D-Bus-Schnittstelle auf Missbrauch durch andere
Prozesse der Sitzung. Letztere ist sitzungsintern und entspricht anderen GTK-Apps.

---

## G. Verbesserungsvorschläge

Bewertung: Nutzen / Aufwand / Risiko / Komplexität, jeweils hoch (H), mittel (M), niedrig (N).

### 1. Notwendige Fehlerkorrekturen

| # | Maßnahme | Befund | Nutzen | Aufwand | Risiko | Kompl. |
|---|---|---|---|---|---|---|
| 1 | F2 ohne Standardkürzel, F10 bei Terminal-Fokus durchreichen | QA-01 | H | N | N | N |
| 2 | Bewusst beendete Terminals nicht neu starten; abgelaufene Einblendung freigeben | QA-03, R-01 | H | N | N | M |
| 3 | Einfüge-Warnung bei jedem Umbruch | QA-04 | H | N | N | N |
| 4 | OSC-7-URI dekodieren und Host prüfen, sonst `/proc/<pid>/cwd` | QA-05, QA-06 | H | N | N | N |
| 5 | Markup maskieren (Vorlagen, Toasts, Tooltips, Schemanamen) | QA-07 | M | N | N | N |
| 6 | Umgebung der Befehlszeile übernehmen | QA-08 | M | M | N | M |
| 7 | Fokus nach Popovers zurück aufs Terminal | QA-09 | M | N | N | N |
| 8 | One dunkel: Vordergrund `#ABB2BF` | QA-10 | M | N | N | N |
| 9 | Umgebungsfeld: Quoting beim Anzeigen und Parsen | R-03 | M | N | N | N |
| 10 | Rauchtest: CRITICALs zuverlässig erkennen, htop/F-Tasten, Neustart-Profil | R-02 | M | N | N | N |

### 2. Sinnvolle Verbesserungen

| # | Maßnahme | Befund | Nutzen | Aufwand | Risiko | Kompl. |
|---|---|---|---|---|---|---|
| 11 | Kurze Reiter-Beschriftung „Programm · Ordner“ (mit Nummer) | QA-17 | H | M | N | M |
| 12 | Blinkender Text nur bei Fokus | QA-14 | M | N | N | N |
| 13 | Tooltips aus aktueller Belegung | QA-12 | N | N | N | N |
| 14 | Fehlerdialog in Festbreitenschrift, Einblendung ohne Zeitablauf | QA-15 | N | N | N | N |
| 15 | Profil-Editor: kein Fokus im Namensfeld | QA-16 | N | N | N | N |
| 16 | Screenreader-Namen (Übersicht, Kürzel nicht im Namen) | QA-18 | M | N | N | N |
| 17 | `GIO_LAUNCHED_DESKTOP_FILE*` vor dem Spawn entfernen | R-06 | N | N | N | N |
| 18 | Profile/Vorlagen mit 0600 schreiben | R-12 | N | N | N | N |
| 19 | Sitzung nach Trennerziehen speichern, NaN abfangen | R-07, R-08 | N | N | N | N |
| 20 | Kontrast-Test für alle Paletten; „Hoher Kontrast“ nachschärfen | QA-10 | M | N | N | N |
| 21 | `cargo audit` in `build-aux/ci.sh` | F | M | N | N | N |

### 3. Optionale Funktionen und größere Designänderungen (Entscheidung nötig)

| # | Maßnahme | Befund | Nutzen | Aufwand | Risiko | Kompl. |
|---|---|---|---|---|---|---|
| 22 | Bereichstitel neu: V1, V2 oder V3 (siehe H) | QA-02 | H | N–H | N–M | N–H |
| 23 | Standard bei Prozessende: schließen bei Code 0, neutrale Leiste | QA-11 | M | N | M (Gewohnheit) | N |
| 24 | Adaptive Kopfleiste unter 600 px (`AdwBreakpoint`) | QA-13 | N | M | N | M |
| 25 | Hauptmenü um Palette, Layout-Modus, Vorlagen ergänzen | E | M | N | N | N |
| 26 | Dateipfade in der Ausgabe als Links (Strg+Klick öffnet im Dateimanager) | T-53 | M | M | M | M |
| 27 | Trefferzahl in der Suche | QA-19 | N | M | N | M |

---

## H. Designvarianten

Ausführlich mit Bildern: **[`designvarianten.html`](designvarianten.html)**. Grundlage sind echte
Bildschirmfotos aus der VM:

- V1 ist ein echtes Foto der vorhandenen Einstellung.
- Bei V2 und V3 sind die Änderungen auf echte Fotos gezeichnet. Das sind Mockups, keine laufende Software.

| | V1 Titelzeile als Standard | V2 Schwebend, ausweichend | V3 Statusleiste, kurze Reiter |
|---|---|---|---|
| Idee | vorhandene Zeile je Bereich wird Standard | Titel unsichtbar, erscheint 1,5 s nach Fokuswechsel und bei Maus am oberen Rand | keine Bereichstitel; eine 20-px-Leiste unten für den aktiven Bereich; Reiter „Nr Programm · Ordner“ |
| Verdeckt Text | nie | nur bei Hover | nie |
| Platz | 1 Zeile je Bereich | keiner | 1 Zeile je Fenster |
| Titel inaktiver Bereiche | immer | bei Hover | nicht sichtbar |
| Einsteiger / Vielnutzer | sehr gut / gut | mäßig / sehr gut | gut / sehr gut |
| Technik | nur Standardwert | Hover-Zone, Timer, Cursorzeile | neue Leiste in `window.rs`, Reitertitel aus 1-s-Timer |
| Aufwand | sehr klein | mittel | mittel bis groß |

Unabhängig davon: kurze Reiter-Beschriftung (Vorschlag 11) mit jeder Variante kombinierbar.
**Keine Variante ist umgesetzt.**

---

## I. Priorisierter Maßnahmenplan

**Kurzfristig (eine Version, z. B. 0.1.1), jede Maßnahme klein und unabhängig:**

1. QA-01 F-Tasten (Vorschlag 1). Am stärksten spürbar, kleinster Eingriff.
2. QA-03 Neustart nach Schließen (2). Vorher Rauchtest erweitern (10), damit R-01/R-02 mit abgedeckt sind.
3. QA-05 + QA-06 OSC 7 (4). Eine Stelle, ein Unit-Test.
4. QA-04 Einfüge-Warnung (3).
5. QA-07 Markup (5) und QA-09 Fokus (7).
6. QA-10 Palette One (8), R-03 Umgebungsfeld (9).

**Danach (0.2):**

7. QA-08 Umgebung der Befehlszeile (6). Braucht etwas Umbau im Spawn-Pfad.
8. Vorschläge 12–21 nach Belieben gebündelt.

**Mit Designentscheidung (vorher Freigabe):**

9. Bereichstitel V1/V2/V3 (22). Abhängigkeit: Wird V3 gewählt, ist Vorschlag 11 Teil davon. Bei V1/V2 ist 11 eigenständig.
10. Prozessende-Standard (23), adaptive Kopfleiste (24), Hauptmenü (25).

**Größere, spätere Arbeiten:** Pfad-Links (26), Trefferzahl (27). Dazu die offenen Punkte aus dem
BACKLOG (Synchronisierte Eingabe, Übersetzungen, Debian-Paket – das Bauen auf Ubuntu 26.04 ist mit
diesem Audit belegt).

---

## J. Testabdeckung und offene Fragen

**Nicht geprüft (und warum):**

- **Orca mit Sprachausgabe:** nicht hörbar prüfbar. AT-SPI-Struktur geprüft.
- **Sixel:** Ubuntu-VTE ohne Sixel.
- **Kontextfarbe root/SSH:** sudo braucht ein Passwort; in Phase 4/5 auf Fedora getestet.
- **Nautilus-Erweiterung auf Ubuntu:** `python3-nautilus` installiert, aber nicht geklickt.
- **X11 (XWayland), HiDPI/Skalierung 200 %, mehrere Monitore.**
- **Echte GPU auf Ubuntu:** alle CPU-Werte sind Softwarerendering.
- **Andere Tastaturbelegungen und Eingabemethoden** (nur DE per QMP).
- **Dauerlauf über Stunden, Speicher bei langen Sitzungen:** nur 5 Runden Öffnen/Schließen; Phase 5 hat 15 min in Fedora.
- **Trenner per Maus ziehen, Reiter in neues Fenster ziehen, Benachrichtigung bei Befehlsende, Profil löschen.**
- **Große Zwischenablage-Inhalte, Einfügen per Mittelklick:** in Ubuntu abgeschaltet.
- **`cargo audit`, Fuzzing der Parser, Netzmitschnitt.**

**Vor produktivem Einsatz noch fehlend:**

- Regressionstests für QA-01 bis QA-09 im Rauchtest oder als Unit-Test.
- Ein Lauf auf echter Hardware mit Ubuntu oder Fedora.
- Ein Orca-Test von Hand.

**Offene Fragen an Christoph:**

1. Welche Designvariante für die Bereichstitel (V1/V2/V3)? Kurze Reiter-Beschriftung ja/nein?
2. Prozessende-Standard: weiter „Leiste anzeigen“ oder „schließen bei Erfolg“?
3. Soll *Reiter umbenennen* ein anderes Standardkürzel bekommen (Vorschlag: keins) und F10 ganz frei bleiben?

**Änderungen an der VM `ubuntu26.04` durch dieses Audit:**

- Nachinstalliert: `rustc cargo meson libvte-2.91-gtk4-dev libadwaita-1-dev libgtk-4-dev pkg-config gir1.2-atspi-2.0 python3-nautilus xdotool vttest htop`.
- Test-SSH-Schlüssel `id_vmtest` in `~testt/.ssh/authorized_keys` (SSH-Port 2222).
- Parkett in `~testt/.local`, Quellbaum in `~testt/parkett`, Testskripte in `~testt/qa`.
- Testprofile, -paletten und -vorlagen wieder gelöscht.
- GSettings (Farbschema, Bereichstitel, Textskalierung, hoher Kontrast) zurückgesetzt.
- Parkett läuft dort noch.

---

## Umsetzung (19.10.2026, nach Freigabe)

Christoph hat die Punkte 1–6 aus I freigegeben und für die Bereichstitel **Design V2**
gewählt. Umgesetzt und geprüft:

| Befund | Änderung | Test |
|---|---|---|
| QA-01 | *Reiter umbenennen* ohne Standardkürzel; Hauptmenü nicht mehr „primary“ (F10) | Ubuntu: F2 in `htop` öffnet Setup, F10 beendet `htop`; Rauchtest |
| QA-02 | Design V2: Titel ausgeblendet, sichtbar bei Zeiger im oberen Streifen (40 px) und 1,5 s nach Fokuswechsel, nicht bei Cursor in Zeile 1; ausgeblendet ohne Klickfläche | Ubuntu: nach Teilen Prompt und Eingabe frei; Alt+Pfeil zeigt Titel kurz; Hover zeigt, Verlassen blendet aus; Maximieren-Knopf klickbar |
| QA-03, R-01 | `terminate()` merkt sich das Beenden, kein Neustart; abgelaufene Einblendung wird losgelassen; Fenster eines Reiters wird beim Aufruf bestimmt, Seitenabfrage ohne Annahme | Rauchtest (Ubuntu, Fedora): Profil „Neu starten“, Reiter schließen, nach 11 s kein Prozess |
| QA-04 | Prüfung in `core::paste`, warnt bei jedem `\n`/`\r` | Unit-Tests; Ubuntu: Zeile mit Umbruch in `dash` → Dialog „Text mit Zeilenumbruch einfügen?“ |
| QA-05, QA-06 | `core::uri::local_path` (Prozent-Kodierung, Rechnername); neue Terminals fallen auf das Verzeichnis der Shell zurück | Unit-Tests; Ubuntu: `/tmp/a b/Ü` wird übernommen; nach fremdem OSC 7 startet der neue Bereich lokal in `/tmp/a b/Ü` statt `/etc` |
| QA-07 | Markup maskiert (Vorlagen-Dialog, Reiter-Tooltip, Toasts als reiner Text, Profil-/Vorlagenlisten) | Ubuntu: Vorlage „QA <Vorlage> & Co“ mit `make -j8 && … <eingabe` vollständig angezeigt |
| QA-09 | Fokus zurück ins Terminal nach Hauptmenü, Neu-Menü und Reiter-Übersicht (nicht, wenn ein Dialog offen ist) | Ubuntu: Menü + Esc, Übersicht + Esc, danach getippte Befehle in `history` |
| QA-10 | One dunkel `#ABB2BF`; Test „Vordergrund ≥ 4,0:1“ für alle Schemata | Unit-Test |
| R-03 | Umgebungsfeld zeigt gequotete Werte (`core::profile::environment_line`) | Unit-Test |
| R-02 | Rauchtest wertet das Journal als JSON aus (jeder Eintrag mit `GLIB_DOMAIN`) | gegengeprüft mit einer künstlichen CRITICAL-Meldung: wird gezählt |

`build-aux/ci.sh` grün (61 + 2 + 1 Tests, Clippy ohne Warnungen). Rauchtest grün in
`ubuntu26.04` (15 Prüfungen) und `fedora44-wor` (13, ohne `htop`).

