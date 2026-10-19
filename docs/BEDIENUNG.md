# Bedienung

Stand 0.2.0. Alle Kürzel lassen sich unter *Einstellungen › Kürzel* ändern oder abschalten;
*Strg+?* zeigt die aktuelle Belegung.

## Reiter und Bereiche

- **Teilen:** Knöpfe *Rechts teilen* und *Unten teilen* links in der Kopfleiste, *Strg+Alt+R*,
  *Strg+Alt+D* oder *Strg+Alt+A* (teilt an der längeren Seite). Ist zu wenig Platz
  (unter 20 × 5 Zeichen je Hälfte), meldet Parkett das statt zu teilen.
- **Fokus wechseln:** *Alt+Pfeil*; **Trenner verschieben:** *Umschalt+Alt+Pfeil*;
  **Größen angleichen:** *Strg+Alt+=*; **Bereich maximieren:** *Strg+Umschalt+X*.
- **Layout-Modus** (*Strg+Umschalt+L*): Bereiche zeigen Nummern. Ziffer = Fokus,
  Pfeile = Fokus, *Umschalt+Pfeil* = Trenner verschieben, *M* und dann Pfeil = mit dem
  Nachbarn tauschen, *R*/*D*/*A* = teilen, *Z* = maximieren, *=* = angleichen,
  *X* = schließen, *Esc* oder *Eingabe* = fertig. Die Leiste unten nennt alle Tasten.
- **Schließen:** *Strg+Umschalt+W* (Bereich), *Strg+Umschalt+Q* (Reiter). Läuft dort noch ein
  Programm außer der Shell, fragt Parkett nach. Danach bleibt 8 Sekunden lang
  *Rückgängig*; der Prozess läuft so lange weiter. Ein weiteres Schließen beendet das vorige
  endgültig.
- **Reiter-Übersicht:** *Strg+Umschalt+O*; **umbenennen:** Hauptmenü oder Befehlspalette (ohne
  Standardkürzel: F2 und F10 bleiben bei den Programmen im Terminal, z. B. `htop`, `mc`, `nano`).
- **Bereichstitel** (bei geteilten Reitern): schwebt oben rechts, ist aber normalerweise
  ausgeblendet, damit er keine Zeile verdeckt. Er erscheint, solange der Zeiger oben im Bereich
  steht, und 1,5 s nach einem Fokuswechsel (nicht, wenn der Cursor in der obersten Zeile steht).
  Dauerhaft sichtbar: *Einstellungen › Allgemein › Bereichstitel: Als Zeile*.

## Befehlspalette

*Strg+Umschalt+P* öffnet eine Liste aller Aktionen, Reiter, Bereiche, Vorlagen und Profile mit
unscharfer Suche. Präfixe grenzen ein: `>` nur Aktionen, `@` nur Reiter und Bereiche,
`#` nur Vorlagen. Jede Aktion zeigt ihr Kürzel.

## Terminal

| Was | Kürzel |
|---|---|
| Kopieren / Einfügen | *Strg+Umschalt+C* / *Strg+Umschalt+V*, *Umschalt+Einfg* |
| Suchen, Treffer darüber/darunter | *Strg+Umschalt+F*, *Strg+Umschalt+G*, *Strg+Umschalt+H* |
| Schrift größer/kleiner/normal | *Strg++*, *Strg+-*, *Strg+0* |
| Link öffnen | *Strg+Klick*; Rechtsklick auf einen Link bietet *Öffnen* und *Kopieren* |

Text mit Zeilenumbruch (auch eine einzelne Zeile mit Umbruch am Ende) zeigt beim Einfügen über
*Strg+Umschalt+V*, *Umschalt+Einfg* oder das Kontextmenü erst eine Vorschau, weil eine Shell
ohne „Bracketed Paste“ ihn sofort ausführen würde (abschaltbar unter *Einstellungen › Allgemein*).

**Neue Terminals im aktuellen Verzeichnis** übernehmen das Verzeichnis aus der
Shell-Integration (OSC 7). Meldet eine SSH-Sitzung den Pfad auf dem Server, nimmt Parkett
stattdessen das lokale Verzeichnis der Shell.

**Befehl fertig:** Endet ein Befehl, der mindestens 10 Sekunden lief, in einem Bereich, der
gerade nicht zu sehen ist (anderer Reiter oder Fenster im Hintergrund), wird der Reiter
markiert und eine Benachrichtigung erscheint; ein Klick darauf zeigt den Bereich. Das braucht
die Shell-Integration von VTE (`vte.sh`, bei Fedora und Arch für bash und zsh dabei);
abschaltbar unter *Einstellungen › Allgemein*.

**Sixel-Grafiken:** Programme wie `img2sixel` oder `chafa -f sixel` zeigen Bilder direkt im
Terminal, wenn die installierte VTE-Bibliothek mit Sixel-Unterstützung gebaut ist. Arch (`vte4` 0.84.1) und Fedora 44 (`vte291-gtk4` 0.84.1) bauen sie derzeit
ohne; dann ist der Schalter unter *Einstellungen › Allgemein* ausgegraut und nennt den Grund.

**Kontextfarbe:** Läuft im Bereich `ssh`/`mosh` oder ein Programm als root (`sudo`, `su`,
`doas`, `run0`, `pkexec`), färbt sich der Hintergrund leicht und eine farbige Linie mit
Rechnername bzw. „root“ erscheint. Abschaltbar unter *Einstellungen › Allgemein*.

## Befehlszeile

```text
parkett [OPTIONEN] [-- BEFEHL [ARGUMENTE …]]
  -d, --working-directory=DIR   Startverzeichnis; mehrfach = ein Reiter je Verzeichnis
  -e, -x, --                    Rest der Zeile statt der Shell ausführen
  -p, --profile=ID              Profil (Dateiname ohne .toml)
  -t, --title=TITEL             Titel des Reiters
      --tab                     als Reiter im aktiven Fenster statt neuem Fenster
```

Beispiele: `parkett -d ~/src --tab` öffnet einen Reiter in `~/src`, `parkett -t Logs --
journalctl -f` ein Fenster mit laufendem `journalctl`. Läuft Parkett schon, übernimmt die
laufende Instanz den Aufruf. Die `.desktop`-Datei nennt die Optionen für
`xdg-terminal-exec` (`X-TerminalArgExec`, `X-TerminalArgDir`, `X-TerminalArgTitle`).

## Nautilus

Mit `nautilus-python` (Arch: Paket `nautilus-python`, Fedora: `nautilus-python`) bringt
Parkett zwei Einträge ins Kontextmenü von Nautilus: *In Parkett öffnen* für markierte Ordner
(mehrere Ordner = ein Fenster mit je einem Reiter) und *Parkett hier öffnen* im Menü des
aktuellen Ordners bzw. auf freier Fläche. Nach der Installation einmal Nautilus beenden
(`nautilus -q`), damit es die Erweiterung lädt. Das eingebaute *In Konsole öffnen* (Strg+.)
ist in Nautilus fest mit GNOME Console verbunden und lässt sich nicht umbiegen.

## Sitzung

Parkett öffnet beim Start die Fenster und Reiter der letzten Sitzung: Aufteilung,
Profile, Verzeichnisse und eigene Reitertitel. Programme laufen nicht weiter; es kommen
Shells in den alten Verzeichnissen. Gespeichert wird laufend (kurz nach jeder Änderung),
die Sitzung übersteht also auch Abmelden oder Absturz. Wer den letzten Reiter schließt,
beendet die Sitzung, der nächste Start beginnt frisch. Mit Vorgaben auf der Befehlszeile
(`-d`, `--`, `-p`, `-t`) öffnet Parkett nur das Verlangte. Abschaltbar unter
*Einstellungen › Allgemein*; die Datei liegt in `~/.local/state/parkett/session.json`.

## Hell und Dunkel

Oben im Hauptmenü: *Systemstil folgen*, *Hell*, *Dunkel* (auch unter *Einstellungen ›
Allgemein › Stil*). Jedes Farbschema hat eine helle und eine dunkle Fassung; die Terminals
wechseln mit.

## Profile

*Einstellungen › Profile*: Farbschema (Kacheln mit Vorschau, wirkt sofort auf offene
Terminals), Deckkraft des Hintergrunds (30–100 %; darunter scheint der Desktop durch, die
Kopfleiste bleibt deckend), Schrift (sonst die Festbreitenschrift des Systems), Cursorform,
Blinken, Glocke
(Ton und Reiter markieren, nur markieren, aus), Befehl statt Shell, Login-Shell, Startverzeichnis, Umgebungsvariablen, Verhalten
beim Prozessende. Neue Reiter mit anderem Profil: Pfeil neben *Neuer Reiter*.

Gespeichert in `~/.config/parkett/profiles/<id>.toml`, von Hand bearbeitbar:

```toml
name = "Server"
palette = "nord"
command = ""             # leer = Shell
login_shell = true
directory = "~/srv"      # leer = Einstellung „Neue Terminals öffnen in“
environment = ["EDITOR=vim"]
exit_action = "hold"     # hold | close | restart
font = "JetBrains Mono 11" # leer = Systemschrift
cursor_shape = "ibeam"   # block | ibeam | underline
cursor_blink = "system"  # system | on | off
bell = "mark"            # sound | mark | none
opacity = 0.9            # 0.3–1.0
```

## Farbschemata

14 Schemata sind dabei. *Importieren …* auf der Profilseite übernimmt Ptyxis-Paletten
(`.palette`) und Tilix-Schemata (`.json`); sie landen in `~/.local/share/parkett/palettes/`.
Format wie bei Ptyxis (vollständiges Beispiel: `data/palettes/tango.palette`): Abschnitt
`[Palette]` mit `Name=`, dann `[Light]` und `[Dark]` mit je `Foreground`, `Background`,
`Color0` bis `Color15` und optional `Cursor`, `SuperuserBackground` (Bereiche mit root) und
`RemoteBackground` (SSH). Fehlende Kontextfarben leitet Parkett wie Ptyxis aus der Palette ab.

Hat ein Tilix-Schema nur eine Fassung, nutzt Parkett sie für Hell und Dunkel und sagt das
beim Import.

## Vorlagen

Eine Vorlage speichert das Layout eines Reiters mit Profil, Startbefehl und Unterverzeichnis
je Bereich. *Aktuellen Reiter als Vorlage speichern …* (Pfeil neben *Neuer Reiter* oder Befehlspalette)
übernimmt die laufenden Befehle; *Einstellungen › Vorlagen* bearbeitet sie. Öffnen über
dasselbe Menü oder die Befehlspalette (`#`). Ohne festes Verzeichnis fragt Parkett beim Öffnen.

Startbefehle werden in die Shell **getippt**, nicht statt der Shell gestartet: Nach *Strg+C*
bleibt der Bereich als Shell nutzbar.

`~/.config/parkett/templates/<id>.toml`:

```toml
name = "Webprojekt"
directory = ""            # leer = beim Öffnen fragen

[layout]
type = "split"
direction = "horizontal"  # horizontal = nebeneinander, vertical = übereinander
ratio = 0.5

[layout.first]
type = "pane"
name = "Editor"
command = "nvim ."

[layout.second]
type = "split"
direction = "vertical"
ratio = 0.5

[layout.second.first]
type = "pane"
name = "Dev-Server"
command = "npm run dev"

[layout.second.second]
type = "pane"
profile = ""              # leer = Standardprofil
subdir = "tests"
```

## Einstellungen ohne Oberfläche

Alles unter *Einstellungen › Allgemein* steht in GSettings:

```bash
gsettings list-recursively de.linuxundich.Parkett
```
