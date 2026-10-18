# Phase 3 – Technischer Prototyp

Stand: 2026-10-18, Version 0.1.0 (nicht veröffentlicht).

## Festgelegt (Entscheidungen Christoph, 2026-10-18)

| Frage | Entscheidung |
|---|---|
| Designkonzept | A+ (A + Befehlspalette/Layout-Modus aus C + Vorlagen mit Startbefehlen aus B), siehe KONZEPTE.md |
| Bereichstitel bei Splits | **schwebend** als Standard, „als Zeile“ als Einstellung |
| Vorschläge 1–6 aus KONZEPTE.md | alle übernommen (Umsetzung ab Phase 4) |
| Sprache | **Rust** |
| App-ID | **de.linuxundich.Parkett** |

## Bibliotheken

| Crate | Version | Feature | Wozu |
|---|---|---|---|
| gtk4 | 0.11 | `v4_18` | Oberfläche |
| libadwaita | 0.9 | `v1_7` (ab Phase 4 `v1_8`) | Fenster, Headerbar, About; `StyleManager::monospace_font_name` (1.7) |
| vte4 | 0.10 | `v0_80` | Terminal; Termprops (0.78), Kontextmenü-Modell (0.76) |

Keine weiteren Abhängigkeiten. Mindestversionen damit: GTK 4.18, libadwaita 1.7, VTE 0.80
(Fedora 44 und Arch erfüllen das; Debian/Ubuntu-LTS noch prüfen).

## Modulstruktur

```
core/  parkett-core   ohne GTK, cargo test
  layout.rs           unveränderlicher Split-Baum: split, split_auto, close, resize,
                      equalize, toggle_zoom, geometry, neighbour (Fokus nach Richtung),
                      can_split (Mindestgröße) – 12 Tests
app/   parkett        GTK
  main.rs             AdwApplication, CSS, App-Aktionen (new-window, about)
  window.rs           Fenster: Headerbar + ein Terminal; Titel/Untertitel aus dem Terminal
  terminal.rs         VTE-Wrapper: Spawn, Prozessende-Leiste, Termprops, Aktionen + Kürzel,
                      Kontextmenü, Farben/Schrift nach AdwStyleManager
  colors.rs           Farbschema der Mockups, hell/dunkel
data/                 .desktop, Icons (hicolor, nach App-ID benannt)
build-aux/vm/         deploy.sh, shot.sh, click.py für die Test-VM
```

Umgesetzte Architekturregeln:
- **Zustand aus Termprops:** Titel aus `xterm.title`, Verzeichnis aus `vte.cwd`. vte4-rs bindet
  `ref_termprop_uri` nicht – dafür ein einzelner FFI-Aufruf (`terminal.rs: cwd()`).
- **Asynchroner Start** über `spawn_async` (Tilix startet synchron); Fehler beim Start landen in
  derselben Leiste wie das Prozessende.
- **Kürzel je Terminal** über einen `GtkShortcutController` in der Capture-Phase am VTE-Widget.
  App-weite Accels werden am Fenster aufgelöst und erreichen Aktionen, die am Terminal hängen,
  nicht (in der VM gefunden, siehe unten). Das gilt auch für alle späteren Bereichsaktionen.
- **Mindestgröße** 20×5 Zellen über `set_size_request` aus der Zellgröße.
- Start übernimmt das Arbeitsverzeichnis des Aufrufers (`HANDLES_COMMAND_LINE`); zweiter Aufruf
  öffnet ein neues Fenster in derselben Instanz.

## Tests

### Ausgeführt und bestanden

| Test | Wo | Ergebnis |
|---|---|---|
| `cargo test` (12 Layout-Tests, 1 Exit-Status-Test) | Host (Arch) und VM | grün |
| `cargo clippy --all-targets` | Host | keine Befunde |
| Build release | VM Fedora 44 (GNOME 50.5, GTK 4.22.5, libadwaita 1.9.4, VTE 0.84.1) | ok, 1 min 24 s frisch |
| Start als `app-gnome-…`-Unit, Wayland (`WAYLAND_DISPLAY=wayland-0`), Bash als Kindprozess | VM | ok |
| Titel per OSC und Verzeichnis per OSC 7 in der Headerbar (`/usr/share`, `~`) | VM | ok |
| Eingabe, `ls --color`, Unicode (Umlaute, Kyrillisch, Griechisch, CJK doppelt breit, Emoji, Kombizeichen, Rahmen), fett/kursiv/unterstrichen/Truecolor/invers | VM | ok, siehe `prototyp-unicode.png` |
| Hell → Dunkel live (`color-scheme prefer-dark`): Fenster und Terminalfarben wechseln | VM | ok |
| Größenänderung (Kacheln auf halbe Breite): Inhalt bricht um | VM | ok |
| Zoom Strg+= / Strg+0 | VM | ok **nach Korrektur** (erst tippte Strg+= „=“) |
| Alles auswählen + Kopieren (Strg+Umschalt+A/C), gegengeprüft mit `wl-paste` | VM | ok |
| `exit 3` → Leiste „Prozess beendet · Exit-Code 3“, *Neu starten* per Klick startet neue Shell | VM | ok, siehe `prototyp-prozessende.png` |
| Drei Fenster in einer Instanz (Strg+Umschalt+N), je eine Shell | VM | ok |
| Speicher (RSS, VM mit Software-Rendering): 1 Fenster 227 MB, 3 Fenster 248 MB; Ptyxis 1 Fenster 221 MB | VM | Grundlast liegt im GTK-Rendering, nicht in Parkett |
| Icon und Name im Dock über `.desktop` + App-ID | VM | ok (beim allerersten Start direkt nach der Installation noch generisch) |
| X11-Backend (`GDK_BACKEND=x11` über XWayland) | VM | startet und läuft; Mesa warnt nur über fehlendes DRI3 der VM |

### Nicht ausgeführt

- Start auf dem Host (Arch, GNOME 51.1) – nur gebaut und Unit-Tests; Fenster nicht geöffnet,
  um deinen Desktop nicht zu stören. Die kopflose GNOME-Sitzung (deskwright) war nicht nutzbar.
- Kontextmenü per Rechtsklick, Einfügen (Strg+Umschalt+V), Links (OSC 8).
- Barrierefreiheit mit Orca, HiDPI/fraktionale Skalierung, zweiter Monitor.
- Langlauf, viele Terminals, hohe Ausgaberate (`yes`, `cat` großer Dateien).
- Flatpak (bewusst später, braucht Host-Agent).

### Gefunden und behoben

- Terminal-Kürzel als App-Accels wirkungslos → Shortcut-Controller am Terminal (siehe oben).

### Nebenwirkung in der VM

`dnf install vte291-gtk4-devel wl-clipboard` hat eine von GNOME Software vorgemerkte
Offline-Aktualisierung verworfen (dnf-Meldung „Pending offline transaction has been invalidated“).
GNOME Software plant sie beim nächsten Lauf neu.

## Offene Architekturentscheidungen (vor/in Phase 4)

| Frage | Optionen | Vorschlag |
|---|---|---|
| Widgets als GObject-Unterklassen | jetzt einfache Rust-Strukturen mit `Rc` | ab Phase 4 `TerminalPane`, `SplitView`, `Window` als Unterklassen (Template/Blueprint?) |
| Blueprint für UI-Dateien | Blueprint wie Kulisse, oder Code | Blueprint für Dialoge/Einstellungen, Code für Split-Baum |
| Einstellungen | GSettings vs. Dateien | GSettings für App-Optionen, Profile/Vorlagen als Dateien (`$XDG_CONFIG_HOME/parkett/`) |
| Serialisierung (Sitzungen, Vorlagen) | serde + JSON/TOML | serde_json für Sitzungen, TOML für Vorlagen (von Hand editierbar) |
| Build/Installation | nur Cargo, Meson wie Kulisse | Meson ab Phase 4 (Schema, Icons, .desktop, Übersetzungen) |
| Lizenz | GPL-3.0-or-later (Cargo, About) | so lassen, LICENSE-Datei ergänzen |
| Startbefehle der Vorlagen | in Shell tippen (`feed_child`) | entschieden mit Vorschlag 5 |
| Farbschema-Format | eigenes, Tilix-JSON, Ptyxis-Palette | Ptyxis-Palette (enthält Kontextfarben für root/ssh) – prüfen |
