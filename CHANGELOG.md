# Changelog

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/).

## [Unreleased]

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
