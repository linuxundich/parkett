# Phase 1 – Analyse: Parkett, ein Tilix-Nachfolger für GTK4/libadwaita

Stand: 2026-10-18. Grundlage sind die Quelltexte (flache Klone vom 2026-10-18),
die lokal installierten Bibliotheken (gtk4 4.24.1, libadwaita 1.10.0, vte4 0.84.1,
Rust-Crates gtk4 0.11.5, libadwaita 0.9.2, vte4 0.10.0) sowie Issues und Release-Notes.

Kennzeichnung: **F** = Fakt mit Beleg · **S** = Schlussfolgerung · **V** = Vorschlag.
Pfade bei Tilix sind relativ zu `source/gx/tilix/` im Repo gnunn1/tilix (HEAD `46b6a7c`).

---

## 1. Ausgangslage in drei Sätzen

- **F** Tilix ist faktisch eingefroren: letzter Release 1.9.6 vom 2023-11-08, seitdem fast nur
  Übersetzungen. Das README sucht seit 2019 Maintainer (Issue #1700) und verspricht „no new features“.
  Basis ist D mit GtkD 3; GtkD bietet nur GTK3 und gilt seit 2021 als verwaist (Kommentar in #2013).
  Ein GTK4-Port existiert nicht. PR #2282 (Umstieg auf giD-Bindings) stockt seit 2026-04.
- **F** Lokal startet das installierte tilix 1.9.6 nicht mehr (`libphobos2-ldc-shared.so.112` fehlt);
  in den offiziellen Arch-Repos ist es entfernt (#2264).
- **F/S** Unter den GTK4/libadwaita-Terminals hat nur **Ghostty** echte Splits – ohne
  Einstellungsoberfläche, ohne Sitzungswiederherstellung unter Linux, ohne Drag&Drop von Splits unter GTK.
  GNOME Console, Ptyxis, Black Box und elementary Terminal haben keine Splits.
  **Die Nische „GNOME-natives Terminal mit Splits, Profilen und GUI“ ist unbesetzt.**

---

## 2. Bestandsbericht (die acht Fragen)

### 2.1 Was macht Tilix besonders?

| Merkmal | Beleg |
|---|---|
| Kacheln in einem Tab („Session“) als Binärbaum aus GtkPaned, rechts/unten/automatisch nach längerer Seite | `session.d:417, 507, 1491` |
| Teilungsverhältnis bleibt beim Vergrößern erhalten (eigener Ratio-Mechanismus) | `session.d:1631–1720` |
| Gleichmäßig verteilen per Doppelklick auf den Trenner | `session.d:200–260` |
| Terminal maximieren innerhalb der Session (eigene Stack-Seite) | `session.d:684–745` |
| Drag&Drop von Terminals an der Titelleiste, Ablage in vier Quadranten, auch zwischen Sessions und Fenstern, Loslösen in neues Fenster | `terminal/terminal.d:3142–3450`, `session.d:544, 628` |
| Session als JSON speichern/laden (Layout, Profile, Verzeichnisse, Titel, optional Befehle) | `session.d:784–1190` |
| Seitenleiste mit Vorschaubildern der Sessions (F12), alternativ Tabs | `sidebar.d:491`, `appwindow.d:1047` |
| Profile mit Titel-Vorlagen (`${title} ${directory} ${process}` …), Badges, eigenen Links | `constants.d:122–187`, `terminal.d:2578, 3478` |
| Automatischer Profilwechsel nach `user@host:/pfad` (über OSC 7) | `preferences.d:465–500` |
| Trigger (Regex auf Ausgabe → 9 Aktionen) – nur mit Fedora-gepatchter VTE | `terminal.d:1589–1745` |
| Synchronisierte Eingabe je Session, Ausnahme je Terminal | `session.d:656`, `terminal.d:398` |
| Read-only, Stille/Aktivität überwachen, Passwort-Manager (libsecret), Lesezeichen | `terminal/password.d`, `bookmark/manager.d` |
| Quake-Modus – unter Wayland abgeschaltet | `appwindow.d:1705–1730`, #1314 |
| CLI-Aktionen (`--action session-add-right`), `new-instance-mode` (z. B. split-right) | `application.d:679–700` |

**S** Der Kern, um den es Nutzern geht: **Splits + Sessions mit Layout + Profile**. Alles andere sind
Beigaben, von denen einige an gepatchter VTE oder X11 hängen.

### 2.2 Wie funktionieren Sitzungen, Tabs, Splits und Arbeitsbereiche?

- **F** Hierarchie: `AppWindow` → `Notebook` (eine Seite je Session) → `Session : Stack` (Seiten
  „group“ und „maximized“) → Binärbaum aus `TerminalPaned`, jedes Kind in einer Hilfs-Box
  (`session.d:432–438`). Die Session sieht nie die VTE direkt, nur das Verbund-Widget `Terminal`
  (Titelleiste + Overlay + ScrolledWindow + VTE, `terminal.d:199`).
- **F** Neues Terminal beim Teilen erbt Profil und aktuelles Verzeichnis (`session.d:417`).
- **F** Fokus nach Richtung geometrisch (Suche in 20-px-Schritten), Resize per Tastatur 10 px/Schritt
  (`session.d:1315, 1386`). Keine eigene Mindestgröße – es gilt das Minimum der VTE (`:1622`).
- **F** Prozessstart synchron (`vte.spawnSync`, `terminal.d:2896`); unter Flatpak eigener Weg über
  `org.freedesktop.Flatpak.Development.HostCommand`.
- **F** „Arbeitsbereiche“ im Sinne von Projekten gibt es nicht; nächstliegend sind gespeicherte
  Session-Dateien und Lesezeichen.

### 2.3 Welche Bedienkonzepte sind weiterhin sinnvoll?

**S**
- Splits innerhalb eines Tabs mit Binärbaum und erhaltenem Verhältnis.
- Tastaturkürzel-Satz (Strg+Alt+R/D teilen, Alt+Pfeil Fokus, Umschalt+Strg+X maximieren) – viele
  Nutzer haben ihn im Muskelgedächtnis. **V** Standardbelegung übernehmen, wo sie nicht mit GNOME
  kollidiert.
- Drag&Drop mit Quadranten-Ablage – Ghostty fehlt das unter GTK, wäre ein Alleinstellungsmerkmal.
- Session-Layouts speichern und öffnen.
- Profilwechsel nach Host/Verzeichnis.
- Kennzeichnung neuer Ausgabe/beendeter Prozesse.

### 2.4 Was wirkt überholt?

**S** (aus `prefeditor/*`, `appwindow.d:311–405`, `terminal.d:341–420`)
- GTK3-Formulare: Grid + ComboBox + CheckButton + FileChooserButton, Profileditor als Notebook mit
  sieben Reitern, eigenes Einstellungsfenster mit zwei Headerbars.
- Optionen, die einen Neustart verlangen (Tabs statt Seitenleiste, Transparenz, Overlay-Scrollbars).
- Volle Titelleiste je Terminal mit MenuButton, Sync-Schalter, Symbolen – kostet in jedem Bereich
  Höhe und wiederholt Information.
- Seitenleiste mit selbst gerenderten Vorschaubildern (Umweg über OffscreenWindow, `gx/gtk/cairo.d`);
  `AdwTabOverview` liefert das heute fertig.
- Funktionen, die an Fedora-VTE-Patches hängen (Trigger, Badges, Prozessende-Hinweis).
- Quake-Modus nur unter X11.

### 2.5 Was lässt sich mit GTK4, libadwaita und VTE zeitgemäß umsetzen?

| Funktion | Baustein (F, seit Version) |
|---|---|
| Tabs, Anheften, Aufmerksamkeit, Drag zwischen Fenstern | `AdwTabView`/`AdwTabBar` (1.0), `needs-attention`, `indicator-icon` (1.2), `create-window` |
| Tab-Übersicht mit Live-Vorschau und Suche | `AdwTabOverview` (1.3), `live-thumbnail`, `keyword` |
| Seitenleiste | `AdwOverlaySplitView` (1.4), `AdwSidebar` (1.9, prefix/suffix 1.10) |
| Adaptive Layouts | `AdwBreakpoint` (1.4) |
| Einstellungen, Dialoge, Tastenkürzel-Übersicht | `AdwPreferencesDialog`, `AdwAlertDialog` (1.5), `AdwShortcutsDialog` (1.8) |
| System-Monospace-Schrift, Akzentfarbe | `AdwStyleManager:monospace-font-name` (1.7), `accent-color` (1.6) |
| Splits | verschachtelte `GtkPaned` mit `shrink-*`/`resize-*` (4.0) |
| Drag&Drop von Bereichen | `GtkDragSource`/`GtkDropTarget` (4.0) |
| Spawn, Prozessende | `vte_terminal_spawn_async`, `child-exited`, `VTE_SPAWN_REQUIRE_SYSTEMD_SCOPE` (0.60) |
| Verzeichnis, Titel, Container, Befehlsgrenzen | **Termprops** (0.78): `vte.cwd`, `xterm.title`, `vte.container.*`, `vte.shell.precmd/preexec/postexec`, `vte.progress.*` (0.80) |
| Befehlsende-Hinweis ohne Patch | `vte.shell.postexec` (Termprop) bzw. OSC 777 (`set_enable_legacy_osc777`, 0.78) |
| Suche, URLs, OSC 8 | `search_set_regex` (PCRE2), `match_add_regex`, `check_hyperlink_at` |
| Kontextmenü | `set_context_menu_model` (0.76) |
| Kopieren als HTML | `copy_clipboard_format(VTE_FORMAT_HTML)` |
| Barrierefreiheit | `set_enable_a11y` (0.78), GtkAccessibleText; #2614 geschlossen 2025-09, Folgefehler bis 2026-10 behoben |
| Scrollback | auf Platte, LZ4 + AES-256-GCM (`src/vtestream-file.h:84, 100`) |

### 2.6 Was braucht zusätzliche Bibliotheken oder eigenen Code?

| Bedarf | Lösung | Bewertung |
|---|---|---|
| Split-Baum, Fokus nach Richtung, Ratio, Mindestgröße, gleichmäßig verteilen | eigenes Modul (rein, testbar) | **S** Kern des Projekts, ohnehin eigener Code |
| Drag&Drop mit Quadranten-Vorschau | eigener Code auf GtkDragSource/DropTarget | mittel |
| Sitzung speichern/wiederherstellen | eigenes Datenformat (serde/JSON), Pfad `$XDG_STATE_HOME` | gering |
| Flatpak: PTY und Prozesse auf dem Host | Host-Agent nach Ptyxis-Muster (D-Bus über socketpair, `CreatePty`, `HasForegroundProcess` …) | **S** hoher Aufwand; ohne Flatpak (pacman/AUR) entfällt er |
| Vordergrundprozess erkennen (Nachfrage beim Schließen) | `tcgetpgrp(pty_fd)` + `/proc` (wie `ptyxis-process-impl.c:283`) | gering außerhalb Flatpak |
| Container (Podman/Toolbox/Distrobox) | wie Ptyxis-Agent | optional, später |
| Passwort-Manager | libsecret | optional, eher weglassen |
| Quake/Dropdown | GNOME: Shell-Erweiterung nötig; Layer-Shell unter Mutter **nicht verfügbar** (Test: `Gtk4LayerShell.is_supported()` = False auf GNOME Shell 51.1) | **S** nicht im MVP |
| Ligaturen, Kitty-Tastaturprotokoll, Kitty-Grafik | in VTE nicht vorhanden (#192, #2601 offen) | **S** bewusst nicht – VTE-Grenze akzeptieren |

### 2.7 Übernehmen, ändern, optional, weglassen

Siehe Funktionsmatrix in Abschnitt 3.

### 2.8 Risiken bei einem Neubau

| Risiko | Art | Einschätzung / Gegenmittel |
|---|---|---|
| vte4-rs hinkt hinterher: einige Termprop-Funktionen und die Signale `hyperlink-hover-uri-changed`, `setup-context-menu` sind nicht gebunden (`vte4-0.10.0/src/auto/terminal.rs:1369–1688, 2638, 2755`) | technisch | **S** gering – `connect_local` bzw. punktuell FFI über vte4-sys; Bedarf vorher auflisten |
| GtkPaned-Bäume bei vielen Bereichen: gleichmäßige Verteilung, Mindestgrößen, Drift | technisch | **S** mittel – Ratio im Modell halten; ab Bedarf eigener `GtkLayoutManager` |
| Fokus-/Tastaturkonflikte zwischen VTE und App-Kürzeln (Strg+Umschalt+…), GNOME-Kürzel | technisch/UX | mittel – Kürzel-Editor, Kollisionsprüfung |
| Flatpak-Sandbox: PTY, cwd, Vordergrundprozess nur mit Host-Agent | technisch | hoch, falls Flatpak Pflicht ist – **V** zuerst nativ (pacman-Repo), Flatpak später |
| Barrierefreiheit der VTE unter GTK4 noch jung (#2901 offen) | technisch | mittel – mit Orca testen, nichts versprechen |
| Speicher bei vielen Terminals: Scrollback je Terminal | technisch | gering – VTE lagert auf Platte aus; Standard 10 000 Zeilen |
| X11 in GTK 4.18+ zunehmend veraltet (55 Symbole `GDK_DEPRECATED_IN_4_18`) | technisch | gering – nichts X11-Spezifisches bauen |
| Funktionsumfang wächst zu Tilix-Komplexität | gestalterisch | hoch – strikte MVP-Grenze, Freigaben je Phase |
| Zwei Navigationsebenen (Tabs + Splits + ggf. Arbeitsbereiche) überfordern | gestalterisch | mittel – genau das entscheiden die Konzepte A/B/C |
| Fokus in Splits nicht eindeutig erkennbar | gestalterisch/a11y | mittel – nie nur Deckkraft; immer zusätzliches Merkmal (Linie/Rahmen) |
| Kontext (root/ssh) je Split statt je Fenster darstellen | gestalterisch | neu, kein Vorbild – sorgfältig gestalten |
| Name/Markenrecht (Tilix ist Name eines fremden Projekts) | organisatorisch | eigener Name nötig – offen |

---

## 3. Funktionsmatrix (priorisiert)

Priorität: **P0** = MVP (unverzichtbar) · **P1** = sinnvolle Erweiterung nach MVP ·
**P2** = optional/Komfort · **–** = bewusst nicht.
Herkunft: T = Tilix, C = Console, P = Ptyxis, B = Black Box, G = Ghostty, K = Kitty.

### Terminal-Grundfunktionen

| Funktion | Prio | Umsetzung | Begründung |
|---|---|---|---|
| Shell im eigenen Prozess, Login-Shell wählbar | P0 | `spawn_async`, systemd-Scope | Grundfunktion; Tilix startet noch synchron |
| Vollständige Emulation | P0 | VTE 0.84 | keine überzeugende Alternative unter GTK4 (G nutzt eigenes Rendering in Zig) |
| Kopieren/Einfügen inkl. Primary, Warnung bei mehrzeiligem/unsicherem Einfügen | P0 | VTE + eigene Prüfung | Sicherheit (T hatte das) |
| Suche im Terminal (Regex, Groß/Klein, Umlauf) | P0 | `search_*` | |
| Zoom (Strg+Plus/Minus/0) | P0 | `font_scale` | |
| Schrift (System-Monospace folgen), Farbschemata | P0 | `AdwStyleManager`, `set_colors` | |
| URLs, OSC 8, Öffnen nur nach Klick mit Strg / Kontextmenü | P0 | `match_add_regex`, `check_hyperlink_at`, `GtkUriLauncher` | sichere Verarbeitung externer Aktionen |
| Konfigurierbare Kürzel | P1 | eigener Editor | MVP mit festen, dokumentierten Kürzeln |
| Kopieren als HTML | P2 | `VTE_FORMAT_HTML` | billig |
| Sixel | P2 | `set_enable_sixel` | billig, Schalter |
| Ligaturen, Kitty-Protokolle | – | VTE kann es nicht | |

### Tabs und Sitzungen

| Funktion | Prio | Begründung |
|---|---|---|
| Mehrere Tabs (Tab = Sitzung mit Split-Baum) | P0 | T-Modell |
| Umbenennen (Doppelklick, Kürzel, Palette) | P0 | |
| Status: läuft / neue Ausgabe / beendet (Exit-Code) | P0 | Termprops `vte.shell.*`, `child-exited` |
| Beendeter Prozess: Bereich bleibt mit Leiste „Neu starten / Schließen“ (je Profil: schließen/halten/neu starten) | P0 | T `exit-action` |
| Nachfrage beim Schließen, wenn Vordergrundprozess läuft | P0 | P `prompt-on-close`; `tcgetpgrp` |
| Neues Terminal erbt Verzeichnis (OSC 7 / `vte.cwd`) | P0 | |
| Profile (Befehl, Verzeichnis, Umgebung, Schrift, Farben, Verhalten bei Ende) | P0 (Basis) / P1 (voll) | |
| Tab-Übersicht | P1 | `AdwTabOverview`, fast gratis |
| Tab per Drag in neues Fenster | P1 | `create-window` |
| Sitzungen beim Start wiederherstellen (Layout, cwd, Profil, Titel – keine Prozesse) | P1 | zuverlässig machbar (vgl. `ptyxis-session.c`) |
| Automatischer Profilwechsel nach Host/Verzeichnis | P1 | T-Stärke; setzt vte.sh/OSC 7 voraus |
| Kontextfarbe für root/ssh/Container – **je Bereich** | P1 | C/P/B färben das Fenster; bei Splits muss es der Bereich sein (neu) |
| Titel-Vorlagen mit Variablen | P2 | T; einfacher Satz reicht |
| Badges | – | wenig genutzt, braucht eigenes Zeichnen |

### Geteilte Terminals

| Funktion | Prio | Begründung / Umsetzung |
|---|---|---|
| Rechts/unten teilen, verschachtelt | P0 | Binärbaum |
| Automatisch teilen (längere Seite) | P0 | T `addAutoOrientedTerminal` |
| Fokus per Tastatur (Richtung, Nummer, zuletzt) | P0 | geometrisch wie T, mit Umlauf wie G |
| Größe ändern (Maus, Tastatur), Verhältnis bleibt beim Fenster-Resize | P0 | Ratio im Modell |
| Schließen → Geschwister rückt nach | P0 | |
| Mindestgröße ca. 20×5 Zeichen; darunter kein weiteres Teilen | P0 | eigene Regel aus `char-size-changed` |
| Bereich maximieren/zoomen mit sichtbarem Hinweis „n verborgen“ | P0 | T/G; G zeigt Symbol |
| Gleichmäßig verteilen | P1 | T Doppelklick, G `equalize_splits` |
| Bereich tauschen/verschieben per Drag&Drop (Quadranten) | P1 | T; G unter GTK nur „planned“ |
| Rückgängig für Schließen/Umbau | P1 | G (unveränderlicher Baum) |
| Layout-Vorlagen speichern/öffnen | P1 | T-Session-Dateien, K-Sessions |
| Synchronisierte Eingabe | P2 | T; nützlich für Server, aber Fehlbedienungsrisiko |
| Feste Layouts (tall/grid/stack) | P2 | K; als „Anordnen“-Befehl |
| Sehr viele Bereiche (> 6) | – als Ziel | **S** Verschachtelung über 2–3 Ebenen verschlechtert die Bedienung; besser zweiter Tab |

### Erweiterte Funktionen

| Funktion | Prio | Begründung |
|---|---|---|
| Start mehrerer Terminals in definiertem Layout (Vorlage + Verzeichnis) | P1 | Hauptnutzen von „Arbeitsbereichen“ |
| Projektbezogene Arbeitsbereiche | P1/P2 | je nach Konzept B (zentral) oder A/C (als Vorlage) |
| Benannte Verzeichnisse/Lesezeichen | P2 | T-Bookmarks; Palette deckt Teil ab |
| Suche über mehrere Terminals | P2 | Termprop-/Text-API in vte4-rs teils ungebunden; Nutzen begrenzt |
| Desktop-Benachrichtigung bei Befehlsende/Aktivität (nur wenn Fenster/Tab inaktiv) | P1 | ohne VTE-Patch via `vte.shell.postexec` |
| Befehlspalette | P1 (A/B) / P0 (C) | K/G; ersetzt tiefe Menüs |
| Export/Import der Einstellungen | P2 | eine TOML/JSON-Datei |
| Konfigurationsdatei neben GUI | P2 | für Dotfile-Nutzer |
| Container-Integration | P2 | P-Agent; großer Aufwand |
| Quake/Dropdown | P2 | nur mit GNOME-Shell-Erweiterung sinnvoll (Erfahrung aus Gnomos/Pulsgeber vorhanden) |
| Trigger | – | Komplex, Fehlbedienungs- und Sicherheitsrisiko; Teile über Termprops abdeckbar |
| Passwort-Manager | – | Sicherheitsrisiko, Passwortmanager des Systems besser |
| Transparenz, Hintergrundbild | P2 / – | Transparenz billig; Hintergrundbild ohne VTE-Patch nicht sauber |
| Read-only | P2 | billig (`set_input_enabled`) |

---

## 4. Sprachwahl

| Kriterium | Rust (gtk4-rs 0.11, libadwaita 0.9.2, vte4 0.10) | C | Vala | Python/PyGObject |
|---|---|---|---|---|
| Speichersicherheit | hoch (außer gezieltem FFI) | gering | mittel | hoch |
| VTE-Abdeckung | gut; Feature-Flags bis `v0_84`, Termprops ab `v0_78`; einige Funktionen/Signale ungebunden | vollständig | VAPI von VTE | vollständig (GIR) |
| libadwaita 1.10 | gebunden (`v1_10`, inkl. AdwSidebar) | nativ | VAPI | GIR |
| Fehlerbehandlung | `Result`, Typsystem | manuell `GError` | Exceptions | Laufzeitfehler |
| Aufwand | hoch (Subclassing, Closures) | hoch (Boilerplate) | niedrig | niedrig |
| Pflege | gtk-rs aktiv; vte4-rs letzter Release 2026-03 | GNOME-Standard (Console, Ptyxis) | Ökosystem schrumpft | Startzeit/Leistung schwächer |
| Testbarkeit der Layout-Logik | sehr gut (reines Modul, `cargo test`) | mittel | mittel | gut |
| Bezug zu deinen Projekten | Blocksatz, ScreenForge, Kulisse, Freisteller, Tastenspur (Rust + GTK4 + Flatpak) | Gnomos (C/Meson) | – | Editoren (Python) |
| Referenz-App mit vte4-rs | Pods (marhkb/pods) | Console, Ptyxis | Black Box | – |

**Empfehlung (S/V): Rust.** Die Split-, Sitzungs- und Profillogik ist der eigentliche Eigenbau-Anteil
und lässt sich in Rust als GTK-freies Modul sauber testen. Die Lücken in vte4-rs sind klein und
bekannt. C wäre nur sinnvoll, wenn wir Code aus Ptyxis (GPL-3.0) direkt übernehmen wollten – das
Muster des Host-Agenten lässt sich auch ohne Codeübernahme nachbauen. Endgültig festgelegt wird die
Sprache laut Plan erst in Phase 3.

---

## 5. Architektur-Empfehlung (erste Fassung)

```
app (GTK/Adw, nur Darstellung und Eingabe)
 ├─ window      Fenster, Headerbar/Chrome, Breakpoints, Aktionen (GAction), Kürzel
 ├─ tabs        AdwTabView-Anbindung, Übersicht, Drag zwischen Fenstern
 ├─ splitview   übersetzt Layout-Modell ⇄ GtkPaned-Baum, DnD, Fokus-Darstellung
 ├─ terminal    Wrapper um vte4::Terminal: Spawn, Termprops → Zustand, Suche, Links, Menü
 ├─ prefs       AdwPreferencesDialog, Profil-Editor
 └─ palette     Befehlspalette (falls gewählt)
core (ohne GTK, vollständig mit cargo test prüfbar)
 ├─ layout      unveränderlicher Binärbaum: split/close/swap/resize/equalize/focus(dir), Undo-Stapel
 ├─ session     Fenster → Tabs → Layout → Terminal-Beschreibung; (De-)Serialisierung, Wiederherstellung
 ├─ profile     Profile, Farbschemata, Auswahlregeln (Host/Verzeichnis)
 └─ config      Einstellungen (GSettings oder TOML – offen), Kürzel-Tabelle
process
 ├─ spawn       Shell/Befehl, Umgebung, systemd-Scope
 └─ host        später: Host-Agent für Flatpak (Ptyxis-Muster)
```

Leitlinien:
- **Modell vor Widget:** Der Split-Baum lebt im `core::layout`; GTK-Widgets werden daraus erzeugt
  und nie umgekehrt ausgelesen. So sind Wiederherstellung, Undo und Tests einfach (Muster Ghostty:
  `src/datastruct/split_tree.zig`).
- **Terminals überleben Umbauten:** Beim Verschieben/Zoomen wird das VTE-Widget umgehängt, nicht neu
  erzeugt (Tilix macht es genauso, `session.d:684–745`).
- **Zustand aus Termprops**, nicht aus veralteten Signalen (`current-directory-uri-changed` ist seit
  0.78 veraltet).
- **Externe Aktionen nur explizit:** Links per Strg+Klick oder Menü, `GtkUriLauncher`/Portal;
  keine Ausführung von Text aus dem Terminal.
- **Einstellungen:** GSettings (GNOME-üblich, Tilix/Ptyxis) vs. eine TOML-Datei (Dotfiles,
  Export trivial) – **Entscheidung offen**, Vorschlag: GSettings für App-Einstellungen,
  Profile/Vorlagen als Dateien unter `$XDG_CONFIG_HOME`, Sitzungen unter `$XDG_STATE_HOME`.
- **Paketierung:** zuerst nativ (pacman-Repo `linuxundich`), Flatpak erst mit Host-Agent.
- **Lizenz:** offen. Tilix ist MPL-2.0, Ptyxis/Console GPL-3.0. Ohne Codeübernahme frei wählbar;
  GPL-3.0-or-later passt zu den GNOME-Vorbildern.

---

## 6. Quellen

- Tilix: github.com/gnunn1/tilix (README, NEWS, `source/gx/tilix/*`, Schema `data/gsettings/com.gexperts.Tilix.gschema.xml`), Issues #1314, #1601, #1700, #2013, #2264, #2282
- VTE: Header `/usr/include/vte-2.91-gtk4/vte/` (0.84.1), `src/vtestream-file.h`, Issues #192, #2601, #2614, #2901, #2934, #2959 (gitlab.gnome.org/GNOME/vte)
- libadwaita: `/usr/share/gir-1.0/Adw-1.gir` (1.10.0), Stylesheet aus `libadwaita-1.so` (Farben der Mockups)
- GTK: `/usr/include/gtk-4.0` (4.24.1), `gtkpaned.h`, `gdk/x11/*`
- vte4-rs 0.10.0, gtk4-rs 0.11.5, libadwaita-rs 0.9.2 (Cargo-Registry), Pods (github.com/marhkb/pods)
- Ptyxis: gitlab.gnome.org/chergert/ptyxis (`src/ptyxis-session.c`, `agent/ptyxis-process-impl.c`, `agent/org.gnome.Ptyxis.Agent.xml`, Schema), Tag 50.1
- GNOME Console 51.0 (lokal, `src/style.css`, Schema), Black Box 0.15.2 (Schema, `Window.vala`, `ThemeProvider.vala`)
- Ghostty 1.3.x (`src/datastruct/split_tree.zig`, `src/config/Config.zig`, Release-Notes 1.3.0/1.3.1)
- Kitty 0.49.2 (`docs/layouts.rst`, `docs/actions.rst`), Foot 1.28.0, WezTerm, Konsole `ViewSplitter.h`
- Lokaler Test Layer-Shell: GNOME Shell 51.1, `Gtk4LayerShell.is_supported()` → False
