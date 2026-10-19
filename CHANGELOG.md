# Changelog

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

### Hinzugefügt
- Sitzung: Fenster, Reiter, Aufteilung, Profile, Verzeichnisse und eigene Titel kommen
  beim Start wieder (ohne Programme); laufend gesichert, Schließen des letzten Reiters
  beendet die Sitzung; abschaltbar.
- Befehlszeile: `-d/--working-directory`, `-e/-x/--` (Befehl statt Shell), `-p/--profile`,
  `-t/--title`, `--tab`, `--help`, `--version`; Schlüssel für `xdg-terminal-exec` in der
  `.desktop`-Datei. Unbekannte Profile werden vorher neu von der Platte gelesen.
- Nautilus: „In Parkett öffnen“ (markierte Ordner, mehrere = Reiter) und „Parkett hier
  öffnen“ per nautilus-python-Erweiterung (`share/nautilus-python/extensions`).
- Befehlszeile: `-d` mehrfach = ein Reiter je Verzeichnis.
- Profil: Schrift, Cursorform, Blinken, Glocke (Ton und Reiter markieren, nur markieren, aus).
- Transparenter Hintergrund: Deckkraft je Profil (30–100 %), Kopfleiste und Trenner bleiben
  deckend (der Trenner war halbtransparent und zeigte als helle Linie das Fenster dahinter).
- Sixel-Grafiken (Einstellung, Standard an); wirkt nur mit einer VTE, die mit Sixel gebaut
  ist – Arch und Fedora 44 sind es derzeit nicht, dann ist der Schalter ausgegraut.
- Benachrichtigung, wenn ein Befehl (ab 10 s) in einem nicht sichtbaren Bereich endet;
  Reiter wird markiert, Klick zeigt den Bereich. Braucht `vte.sh`.

### Geändert
- Bereichstitel (Design V2 aus dem QA-Audit): schwebt weiter oben rechts, ist aber
  ausgeblendet und verdeckt keine Zeile mehr. Er erscheint, solange der Zeiger oben im
  Bereich steht, und 1,5 s nach einem Fokuswechsel, außer der Cursor steht in der obersten
  Zeile. Ausgeblendet nimmt er keine Klicks an.
- *Reiter umbenennen* hat kein Standardkürzel mehr (war F2); F10 öffnet nicht mehr das
  Hauptmenü. Beide Tasten gehören wieder den Programmen im Terminal (htop, mc, nano).
- Rauchtest: läuft auch in der Ubuntu-26.04-VM (`VM_DOM`, `VM_PORT`), prüft F10 in `htop`
  und Profile mit „Neu starten“ und erkennt GLib-Meldungen im Journal zuverlässig.

### Behoben (QA-Audit 2026-10-19, `qa/2026-10-19/QA-BERICHT.md`)
- Ein geschlossener Reiter mit Profil „Neu starten“ startete nach Ablauf der
  Rückgängig-Frist sein Programm unsichtbar neu (z. B. eine ssh-Verbindung); die
  abgelaufene Einblendung hielt den Reiter samt Verlauf im Speicher.
- Einfüge-Warnung: Eine einzelne Zeile mit Zeilenumbruch am Ende wurde ohne Rückfrage
  eingefügt und in Shells ohne „Bracketed Paste“ sofort ausgeführt.
- Neue Terminals übernahmen Verzeichnisse mit Leerzeichen oder Umlauten nicht (die Sitzung
  speicherte `%20`); ein Verzeichnis, das eine SSH-Sitzung per OSC 7 meldet, wurde als
  lokales Startverzeichnis genommen. Jetzt dekodiert und nur vom eigenen Rechner, sonst das
  Verzeichnis der Shell.
- Startbefehle mit `&&`, `<` oder `>` fehlten im Dialog „Vorlage öffnen“; Namen, Pfade und
  Fehlermeldungen in Einblendungen, Tooltips und Listen werden nicht mehr als Markup gelesen.
- Nach Hauptmenü, Neu-Menü oder Reiter-Übersicht bleibt der Fokus nicht mehr in der
  Kopfleiste; Getipptes landet wieder im Terminal.
- Farbschema „One“ dunkel: Vordergrund war die Kommentarfarbe (Kontrast 2,7:1), jetzt
  `#ABB2BF`; ein Test prüft den Kontrast aller Schemata.
- Umgebungsvariablen im Profil: Werte mit Leerzeichen oder Anführungszeichen werden
  gequotet angezeigt und beim Bearbeiten nicht mehr zerlegt.
- Ein geschlossener oder in ein anderes Fenster gezogener Reiter fragte seine alte
  Fensterseite ab (`TabView::page` ohne Seite).

## [0.1.0] - 2026-10-19

Erste Version: MVP (Phase 4), Qualitätssicherung (Phase 5) und Stabilisierung (Phase 6).
Nur lokal; Paket im eigenen Arch-Repo `linuxundich` auf bender.

### Phase 6
- Dokumentation: Bedienung (`docs/BEDIENUNG.md`, Beispiele für Profile und Vorlagen werden
  von `cargo test` geladen), Architektur, Paketierungswege, Lizenzen; README überarbeitet.
- `build-aux/ci.sh`: Formatierung, Clippy, Tests, Meson-Build und -Tests in einem Lauf.
- `build-aux/vm/smoke.sh`: Rauchtest in der VM über D-Bus (Teilen, Reiter, Schließen,
  Fristablauf, keine verwaisten Shells, Protokoll ohne Fehler).
- Entwicklungsplan für 0.2–0.4 im BACKLOG.

### Behoben (Phase 6, Code-Review)
- Nach „Rückgängig“ an neuer Stelle trafen Fokus und Schließen den falschen Bereich.
- Schließen eines schon geschlossenen Bereichs schloss den ganzen Reiter oder das Fenster.
- Absturz beim Start bei nur kaputten Profildateien oder Nicht-ASCII-Farbwerten in Paletten;
  nicht lesbare Dateien werden jetzt als Einblendung mit Details gemeldet.
- Einfüge-Warnung erkennt auch Zeilenumbrüche mit `\r`.
- `file:`-Links auf andere Rechner (OSC 8 aus SSH-Sitzungen) werden nicht mehr geöffnet.
- Unterverzeichnisse in Vorlagen nur noch relativ und ohne „..“.
- Layout-Modus: bleibt nicht mehr auf dem alten Reiter stehen und schluckt keine Tasten
  mehr in Dialogen und der Befehlspalette.
- Automatischer Neustart und Hintergrund-Reiter holen sich nicht mehr den Fokus.
- Doppelklick auf die Kopfleiste öffnet nicht mehr zusätzlich die Befehlspalette.
- Alt+1…9 / Strg+Alt+1…9 werden beim Aufnehmen eigener Kürzel als belegt gemeldet.
- Halb getippte Umgebungsvariablen löschen die gespeicherten nicht mehr.
- Änderungen an Vorlagen färben die Terminals nicht mehr neu ein.

### Hinzugefügt
- Knöpfe „Rechts teilen“ und „Unten teilen“ in der Kopfleiste (wie bei Tilix), eigene Symbole.
- Hell/Dunkel von Hand: Stilwahl System/Hell/Dunkel im Hauptmenü und in den Einstellungen.
- Einstellung „Bereichstitel bei geteilten Reitern: Als Zeile“ (Titelzeile über jedem Bereich).
- Warnung beim Einfügen von mehrzeiligem Text, mit Vorschau (abschaltbar).

### Behoben (Phase 5, QA)
- Rückgängig-Einblendungen standen Schlange und hielten geschlossene Prozesse bis zu einer
  Minute am Leben; eine neue Einblendung ersetzt jetzt die vorige.
- Schwebender Bereichstitel verdeckte in schmalen Bereichen den Prompt; dort jetzt nur Knöpfe.
- Screenreader-Namen: Neu-Knopf hieß „Weitere Möglichkeiten“, Suchknopf las das Kürzel mit.

### Hinzugefügt (Schritte 1–7)
- Kontextfarbe je Bereich: Bereiche mit SSH-Verbindung oder Root-Rechten werden getönt und
  mit Linie und Chip (Hostname bzw. „root“) gekennzeichnet; Farben aus dem Farbschema,
  abschaltbar.
- Rückgängig: geschlossene Bereiche und Reiter lassen sich 8 s lang zurückholen, ihre
  Programme laufen so lange weiter.
- Befehlspalette (Strg+Umschalt+P oder Klick auf den Fenstertitel) für Reiter, Bereiche,
  Aktionen, Vorlagen und Profile; Präfixe >, @, #.
- Layout-Modus (Strg+Umschalt+L): nummerierte Bereiche, Einzeltasten zum Teilen, Tauschen,
  Maximieren, Angleichen und Schließen.
- Vorlagen: aktuellen Reiter mit Aufteilung, Profilen und laufenden Befehlen speichern,
  in einem gewählten Verzeichnis öffnen (Startbefehle abschaltbar), Bearbeiten in den
  Einstellungen.
- Tastenkürzel änderbar (Einstellungen › Tastenkürzel), mit Konfliktprüfung und
  „Alle zurücksetzen“; Fenster, Terminals und Übersicht übernehmen Änderungen sofort.
- Suche im Terminal je Bereich (Strg+Umschalt+F; Groß/Klein, regulärer Ausdruck; Treffer
  darüber/darunter), Suchknopf in der Kopfleiste.
- Links: Strg+Klick öffnet Adressen und OSC-8-Links (nur http, https, ftp, mailto, file),
  Kontextmenü „Link öffnen/kopieren“, Tooltip mit dem wahren Ziel eines OSC-8-Links.
- Profile (TOML unter ~/.config/parkett/profiles): Farbschema, Befehl statt Shell,
  Login-Shell, Startverzeichnis, Umgebungsvariablen, Verhalten bei Prozessende.
- 14 Farbschemata mit heller und dunkler Fassung (aus Ptyxis), Wechsel live mit dem Systemstil.
- Import von Ptyxis-Paletten und Tilix-Farbschemata.
- Einstellungsdialog (Strg+,) mit Seiten „Allgemein“ und „Profile“, Farbschemata als Kacheln
  mit Live-Vorschau; „Neuer Reiter mit Profil“ im Menü am Neu-Knopf.
- Meson-Build mit .desktop, Metainfo, Icons und GSettings-Schema; Lizenztext GPL-3.0.
- Einstellungen per GSettings: Fenstergröße merken, Nachfrage beim Schließen, Abblenden,
  Reiterleiste ausblenden, Startverzeichnis neuer Terminals, Länge des Verlaufs.
- Arch-Paket im eigenen Repo `linuxundich`.

### Geändert
- Oberfläche sagt „Reiter“ statt „Tab“ (wie libadwaita).

### Hinzugefügt (Reiter und Bereiche)
- Tabs mit AdwTabView: Tab-Leiste (bei einem Tab ausgeblendet), Tab-Übersicht mit Suche,
  Tab umbenennen (F2), Warnsymbol bei Prozessende mit Fehler.
- Geteilte Bereiche: rechts/unten/automatisch teilen, Fokus und Größe per Tastatur,
  angleichen, maximieren mit Hinweis, Mindestgröße 20×5 Zellen, schwebender Titel,
  Fokuslinie und Abblenden inaktiver Bereiche.
- Nachfrage beim Schließen von Bereich, Tab oder Fenster, wenn noch ein Programm läuft.
- Tastenkürzel-Übersicht (Strg+?).
- 0.1.0 Prototyp (Phase 3): Fenster mit libadwaita und VTE, Shell-Start asynchron,
  Leiste „Prozess beendet“ mit Exit-Code, *Neu starten* und *Schließen*.
- Titel und Arbeitsverzeichnis in der Headerbar aus VTE-Termprops (`xterm.title`, `vte.cwd`).
- Terminalfarben hell/dunkel nach System, Schrift nach System-Monospace.
- Kopieren, Einfügen, Alles auswählen, Zoom per Kürzel und Kontextmenü; neues Fenster.
- `parkett-core`: Split-Layout als unveränderlicher Baum mit Tests.
- .desktop-Datei und Icons unter der App-ID `de.linuxundich.Parkett`.
