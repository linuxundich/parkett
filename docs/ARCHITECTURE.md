# Architecture

As of 2026-10-19, version 0.2.0. Derived from `docs/PROTOTYPE.md` (phase 3) and the
decisions made in phase 4.

## Structure

```
parkett/
├── core/   parkett-core – logic without GTK, fully testable with `cargo test`
└── app/    parkett – GTK4/libadwaita/VTE user interface
```

Dependencies: `core` only serde, serde_json, toml. `app` additionally gtk4 0.11 (`v4_18`),
libadwaita 0.9 (`v1_8`), vte4 0.10 (`v0_80`), libc. No other crates
(rationale in `docs/ANALYSIS.md`).

### core/

| Module | Purpose |
|---|---|
| `layout` | Split tree of a tab as an **immutable** binary tree: split, close, swap, ratio, zoom, neighbor in a direction, minimum size (`can_split`) |
| `palette` | Ptyxis palette format (light/dark, root/SSH colors derived as in Ptyxis), import of Tilix JSON |
| `profile` | Profiles as TOML, `ProfileStore` (one directory, one file per profile) |
| `template` | Templates (layout + profile/start command/subdirectory per pane), `TemplateStore`, `instantiate`, `from_layout` |
| `fuzzy` | Fuzzy search for the command palette |
| `context` | Context of a pane (local, Remote(Host), Superuser) from process name, command line, UID |
| `cli` | Command-line parsing (own parser: everything after `--` is the command) |
| `session` | Session as JSON, tabs in the template format (directory `/`, absolute paths as subdirectory, no commands) |
| `paths` | Expand and display `~`, safely append template subdirectories |
| `uri` | `file:` URIs (OSC 7, OSC 8): local host?, path with percent-encoding decoded |
| `paste` | Paste check: lines and whether the text contains a line break (would be executed) |

### app/

| Module | Purpose |
|---|---|
| `main` | Application, CSS, app actions, shortcuts overview |
| `window` | `ParkettWindow` (AdwApplicationWindow): header bar with split buttons and style selector, AdwTabBar/AdwTabOverview, command palette, layout mode, template dialogs, Undo toasts |
| `tab` | `ParkettTab`: holds the `Layout` and the pane widgets; `rebuild()` creates the GtkPaned tree from the model |
| `pane` | `ParkettPane`: terminal plus floating title (hidden; shown when the pointer is at the top or briefly after a focus change) or title row, focus line, context line, dimming, search bar |
| `terminal` | VTE widget, process spawning, "process exited" bar, links, search, paste warning, foreground process, context detection |
| `profiles` | `ProfileManager` (GObject, signal `changed`): profiles, palettes, templates for the whole app |
| `prefs` | Preferences: General, Profiles (color scheme tiles with live preview), Templates, Shortcuts |
| `commands` | Command palette (prefixes `>` actions, `@` tabs/panes, `#` templates) |
| `shortcuts` | Shortcut registry with default and custom bindings, construction of the shortcut controllers |
| `settings`, `style` | GSettings access; manual light/dark |
| `session` | Save the session (1.5 s after changes, when the last window is closed) and restore it on startup |

## Rules the code follows

1. **The model is authoritative.** `Layout` is never read back from the widgets. Every
   change produces a new `Layout`; then `tab.rebuild()` re-parents the existing
   `ParkettPane`s into a new GtkPaned tree. Terminals are never recreated, their
   processes keep running.
2. **Ratios against the real extent** (`width`/`height` of the GtkPaned), not against
   `max-position`; otherwise splits shift during a rebuild (finding from phase 4).
3. **Shortcuts via shortcut controllers in the capture phase**, not via app accels: accels
   do not reach the widgets' action groups, and VTE would handle keys like Ctrl+= itself.
   One registry (`shortcuts::ENTRIES`) feeds the controllers, preferences page,
   shortcuts overview, tooltips and command palette.
4. **Weak references in long-lived signals.** Connections to `AdwStyleManager`,
   `ProfileManager` and GSettings hold widgets only weakly and are disconnected on `destroy`;
   otherwise closed panes survive together with their shell (finding from phase 4).
5. **Terminate processes explicitly.** A closed pane sends SIGHUP to its shell
   (the shell passes it on to its jobs) as soon as the Undo period expires or a new toast replaces it.
6. **State from VTE termprops** (`xterm.title`, `vte.cwd`), not from the deprecated signals.
7. **No file system access in the core except the stores**; everything else (/proc, GSettings) lives
   in `app`.

## Data flow

```
GSettings ─┐                      ┌─> ParkettWindow ─> ParkettTab ─> ParkettPane ─> Terminal (VTE)
TOML files ──> ProfileManager ────┤        │                │
Palettes ──┘     (signal changed) └─> prefs│                └─ Layout (core, immutable)
                                           └─ Undo: list of closed panes/tabs
/proc (1 s) ─> Terminal::foreground ─> core::context ─> context line and color
```

## Stored data

| Location | Content |
|---|---|
| GSettings `de.linuxundich.Parkett` | Window size, behavior, style, default profile, custom shortcuts, scrollback length |
| `~/.config/parkett/profiles/*.toml` | Profiles |
| `~/.config/parkett/templates/*.toml` | Templates |
| `~/.local/share/parkett/palettes/*.palette` | Imported color schemes |
| `~/.local/state/parkett/session.json` | Last session |
| in the binary (`include_str!` from `data/palettes/`) | Bundled color schemes (14) |

## Tests

| Level | How | Where |
|---|---|---|
| Core | 65 unit tests (including contrast of all color schemes), plus 2 tests for the examples in `docs/USAGE.md`; 3 tests in the app (exit status, default shortcuts) | `cargo test` |
| Data | .desktop, metainfo, schema | `meson test` |
| Everything without a window | fmt, clippy, tests, Meson build | `build-aux/ci.sh` |
| User interface | Smoke test in the VM via D-Bus actions and keys: split, tabs, close, period expiry, F10 in `htop`, no restart of closed tabs, no orphaned shells, session restored, journal without GLib messages (Fedora 44 or `VM_DOM=ubuntu26.04 VM_PORT=2222`) | `build-aux/vm/smoke.sh` |
| Performance | Throughput, memory/CPU | `build-aux/qa/` |

The user interface has no unit tests: GTK widgets need a display. Whatever can be checked
without GTK therefore lives in `core`.
