# Changelog

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

### Hinzugefügt
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
