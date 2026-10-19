# Architektur

Stand 2026-10-19, Version 0.1.0. Entstanden aus `docs/PROTOTYP.md` (Phase 3) und den
Entscheidungen in Phase 4.

## Aufbau

```
parkett/
├── core/   parkett-core – Logik ohne GTK, vollständig mit `cargo test` prüfbar
└── app/    parkett – GTK4/libadwaita/VTE-Oberfläche
```

Abhängigkeiten: `core` nur serde, serde_json, toml. `app` zusätzlich gtk4 0.11 (`v4_18`),
libadwaita 0.9 (`v1_8`), vte4 0.10 (`v0_80`), libc. Keine weiteren Pakete
(Begründung in `docs/ANALYSE.md`).

### core/

| Modul | Aufgabe |
|---|---|
| `layout` | Split-Baum eines Reiters als **unveränderlicher** Binärbaum: teilen, schließen, tauschen, Verhältnis, Zoom, Nachbar in eine Richtung, Mindestgröße (`can_split`) |
| `palette` | Ptyxis-Palettenformat (hell/dunkel, root/SSH-Farben abgeleitet wie in Ptyxis), Import von Tilix-JSON |
| `profile` | Profile als TOML, `ProfileStore` (ein Verzeichnis, eine Datei je Profil) |
| `template` | Vorlagen (Layout + Profil/Startbefehl/Unterverzeichnis je Bereich), `TemplateStore`, `instantiate`, `from_layout` |
| `fuzzy` | Unscharfe Suche für die Befehlspalette |
| `context` | Kontext eines Bereichs (lokal, Remote(Host), Superuser) aus Prozessname, Befehlszeile, UID |
| `cli` | Befehlszeile auswerten (eigener Parser: alles nach `--` ist der Befehl) |
| `session` | Sitzung als JSON, Reiter im Vorlagenformat (Verzeichnis `/`, absolute Pfade als Unterverzeichnis, keine Befehle) |
| `paths` | `~` auflösen und anzeigen, Unterverzeichnisse von Vorlagen sicher anhängen |
| `uri` | `file:`-URIs (OSC 7, OSC 8): lokaler Rechner?, Pfad mit aufgelöster Prozent-Kodierung |
| `paste` | Einfüge-Prüfung: Zeilen und ob der Text einen Umbruch enthält (würde ausgeführt) |

### app/

| Modul | Aufgabe |
|---|---|
| `main` | Anwendung, CSS, App-Aktionen, Kürzel-Übersicht |
| `window` | `ParkettWindow` (AdwApplicationWindow): Kopfleiste mit Teilen-Knöpfen und Stilwahl, AdwTabBar/AdwTabOverview, Befehlspalette, Layout-Modus, Vorlagen-Dialoge, Rückgängig-Einblendungen |
| `tab` | `ParkettTab`: hält `Layout` und die Bereichs-Widgets; `rebuild()` erzeugt den GtkPaned-Baum aus dem Modell |
| `pane` | `ParkettPane`: Terminal plus schwebender Titel (ausgeblendet; Zeiger oben oder kurz nach Fokuswechsel) bzw. Titelzeile, Fokuslinie, Kontextlinie, Abblendung, Suchleiste |
| `terminal` | VTE-Widget, Start des Prozesses, Leiste „Prozess beendet“, Links, Suche, Einfüge-Warnung, Vordergrundprozess, Kontexterkennung |
| `profiles` | `ProfileManager` (GObject, Signal `changed`): Profile, Paletten, Vorlagen für die ganze App |
| `prefs` | Einstellungen: Allgemein, Profile (Farbschema-Kacheln mit Live-Vorschau), Vorlagen, Kürzel |
| `commands` | Befehlspalette (Präfixe `>` Aktionen, `@` Reiter/Bereiche, `#` Vorlagen) |
| `shortcuts` | Kürzel-Verzeichnis mit Standard- und eigener Belegung, Aufbau der Shortcut-Controller |
| `settings`, `style` | GSettings-Zugriff; Hell/Dunkel von Hand |
| `session` | Sitzung sichern (1,5 s nach Änderungen, beim Schließen des letzten Fensters) und beim Start öffnen |

## Regeln, die der Code einhält

1. **Das Modell ist maßgeblich.** `Layout` wird nie aus den Widgets zurückgelesen. Jede
   Änderung erzeugt ein neues `Layout`; danach hängt `tab.rebuild()` die vorhandenen
   `ParkettPane`s in einen neuen GtkPaned-Baum um. Terminals werden nie neu erzeugt, ihre
   Prozesse laufen weiter.
2. **Verhältnisse gegen die echte Ausdehnung** (`width`/`height` des GtkPaned), nicht gegen
   `max-position`; sonst verschieben sich Splits beim Neuaufbau (Fund aus Phase 4).
3. **Kürzel über Shortcut-Controller in der Capture-Phase**, nicht über App-Accels: Accels
   erreichen die Aktionsgruppen der Widgets nicht, und VTE würde Tasten wie Strg+= selbst
   verarbeiten. Ein Verzeichnis (`shortcuts::ENTRIES`) speist Controller, Einstellungsseite,
   Kürzel-Übersicht, Tooltips und Befehlspalette.
4. **Schwache Verweise in langlebigen Signalen.** Verbindungen zu `AdwStyleManager`,
   `ProfileManager` und GSettings halten Widgets nur schwach und werden bei `destroy`
   getrennt; sonst überleben geschlossene Bereiche samt Shell (Fund aus Phase 4).
5. **Prozesse ausdrücklich beenden.** Ein geschlossener Bereich schickt seiner Shell SIGHUP
   (die Shell reicht es an ihre Jobs weiter), sobald die Rückgängig-Frist abläuft oder eine neue Einblendung sie ersetzt.
6. **Zustand aus VTE-Termprops** (`xterm.title`, `vte.cwd`), nicht aus den veralteten Signalen.
7. **Kein Dateisystem-Zugriff im Kern außer den Stores**; alles andere (/proc, GSettings) liegt
   in `app`.

## Datenfluss

```
GSettings ─┐                      ┌─> ParkettWindow ─> ParkettTab ─> ParkettPane ─> Terminal (VTE)
TOML-Dateien ─> ProfileManager ───┤        │                │
Paletten ──┘     (Signal changed) └─> prefs│                └─ Layout (core, unveränderlich)
                                           └─ Rückgängig: Liste geschlossener Bereiche/Reiter
/proc (1 s) ─> Terminal::foreground ─> core::context ─> Kontextlinie und -farbe
```

## Gespeicherte Daten

| Ort | Inhalt |
|---|---|
| GSettings `de.linuxundich.Parkett` | Fenstergröße, Verhalten, Stil, Standardprofil, eigene Kürzel, Verlaufslänge |
| `~/.config/parkett/profiles/*.toml` | Profile |
| `~/.config/parkett/templates/*.toml` | Vorlagen |
| `~/.local/share/parkett/palettes/*.palette` | importierte Farbschemata |
| `~/.local/state/parkett/session.json` | letzte Sitzung |
| im Programm (`include_str!` aus `data/palettes/`) | mitgelieferte Farbschemata (14) |

## Tests

| Ebene | Wie | Wo |
|---|---|---|
| Kern | 61 Unit-Tests (u. a. Kontrast aller Farbschemata), dazu 2 Tests für die Beispiele in `docs/BEDIENUNG.md`; 1 Test in der App | `cargo test` |
| Daten | .desktop, Metainfo, Schema | `meson test` |
| Alles ohne Fenster | fmt, clippy, Tests, Meson-Build | `build-aux/ci.sh` |
| Oberfläche | Rauchtest in der VM über D-Bus-Aktionen und Tasten: Teilen, Reiter, Schließen, Fristablauf, F10 in `htop`, kein Neustart geschlossener Reiter, keine verwaisten Shells, Sitzung wiederhergestellt, Journal ohne GLib-Meldungen (Fedora 44 oder `VM_DOM=ubuntu26.04 VM_PORT=2222`) | `build-aux/vm/smoke.sh` |
| Leistung | Durchsatz, Speicher/CPU | `build-aux/qa/` |

Die Oberfläche hat keine Unit-Tests: GTK-Widgets brauchen eine Anzeige. Was sich ohne GTK
prüfen lässt, liegt deshalb in `core`.
