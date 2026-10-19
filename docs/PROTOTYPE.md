# Phase 3 – Technical prototype

As of: 2026-10-18, version 0.1.0 (not published).

## Settled (decisions by Christoph, 2026-10-18)

| Question | Decision |
|---|---|
| Design concept | A+ (A + command palette/layout mode from C + templates with start commands from B), see CONCEPTS.md |
| Pane title with splits | **floating** as the default, "as a row" as a setting |
| Proposals 1–6 from CONCEPTS.md | all adopted (implementation from phase 4 on) |
| Language | **Rust** |
| App ID | **de.linuxundich.Parkett** |

## Libraries

| Crate | Version | Feature | Purpose |
|---|---|---|---|
| gtk4 | 0.11 | `v4_18` | user interface |
| libadwaita | 0.9 | `v1_7` (`v1_8` from phase 4 on) | window, header bar, About; `StyleManager::monospace_font_name` (1.7) |
| vte4 | 0.10 | `v0_80` | terminal; termprops (0.78), context menu model (0.76) |

No further dependencies. Resulting minimum versions: GTK 4.18, libadwaita 1.7, VTE 0.80
(Fedora 44 and Arch meet these; Debian/Ubuntu LTS still to be checked).

## Module structure

```
core/  parkett-core   no GTK, cargo test
  layout.rs           immutable split tree: split, split_auto, close, resize,
                      equalize, toggle_zoom, geometry, neighbour (directional focus),
                      can_split (minimum size) – 12 tests
app/   parkett        GTK
  main.rs             AdwApplication, CSS, app actions (new-window, about)
  window.rs           window: header bar + one terminal; title/subtitle from the terminal
  terminal.rs         VTE wrapper: spawn, process-exit bar, termprops, actions + shortcuts,
                      context menu, colors/font following AdwStyleManager
  colors.rs           color scheme of the mockups, light/dark
data/                 .desktop, icons (hicolor, named after the app ID)
build-aux/vm/         deploy.sh, shot.sh, click.py for the test VM
```

Architecture rules implemented:
- **State from termprops:** title from `xterm.title`, directory from `vte.cwd`. vte4-rs does not bind
  `ref_termprop_uri` – a single FFI call is used for it instead (`terminal.rs: cwd()`).
- **Asynchronous start** via `spawn_async` (Tilix starts synchronously); errors during startup end up in
  the same bar as the process exit.
- **Shortcuts per terminal** via a `GtkShortcutController` in the capture phase on the VTE widget.
  App-wide accels are resolved on the window and do not reach actions attached to the terminal
  (found in the VM, see below). This also applies to all later pane actions.
- **Minimum size** of 20×5 cells via `set_size_request` derived from the cell size.
- Startup takes over the caller's working directory (`HANDLES_COMMAND_LINE`); a second invocation
  opens a new window in the same instance.

## Tests

### Run and passed

| Test | Where | Result |
|---|---|---|
| `cargo test` (12 layout tests, 1 exit status test) | host (Arch) and VM | green |
| `cargo clippy --all-targets` | host | no findings |
| Release build | VM Fedora 44 (GNOME 50.5, GTK 4.22.5, libadwaita 1.9.4, VTE 0.84.1) | ok, 1 min 24 s from scratch |
| Start as an `app-gnome-…` unit, Wayland (`WAYLAND_DISPLAY=wayland-0`), Bash as child process | VM | ok |
| Title via OSC and directory via OSC 7 in the header bar (`/usr/share`, `~`) | VM | ok |
| Input, `ls --color`, Unicode (umlauts, Cyrillic, Greek, double-width CJK, emoji, combining characters, box drawing), bold/italic/underline/truecolor/inverse | VM | ok, see `prototyp-unicode.png` |
| Light → dark live (`color-scheme prefer-dark`): window and terminal colors switch | VM | ok |
| Resizing (tiling to half width): content rewraps | VM | ok |
| Zoom Ctrl+= / Ctrl+0 | VM | ok **after a fix** (at first Ctrl+= typed "=") |
| Select all + copy (Ctrl+Shift+A/C), cross-checked with `wl-paste` | VM | ok |
| `exit 3` → bar *Prozess beendet · Exit-Code 3* (Process exited · exit code 3), clicking *Neu starten* (Restart) starts a new shell | VM | ok, see `prototyp-prozessende.png` |
| Three windows in one instance (Ctrl+Shift+N), one shell each | VM | ok |
| Memory (RSS, VM with software rendering): 1 window 227 MB, 3 windows 248 MB; Ptyxis 1 window 221 MB | VM | the base load lies in GTK rendering, not in Parkett |
| Icon and name in the dock via `.desktop` + app ID | VM | ok (on the very first start right after installation still generic) |
| X11 backend (`GDK_BACKEND=x11` via XWayland) | VM | starts and runs; Mesa only warns about the VM's missing DRI3 |

### Not run

- Start on the host (Arch, GNOME 51.1) – only built and unit-tested; window not opened
  so as not to disturb your desktop. The headless GNOME session (deskwright) was not usable.
- Context menu via right-click, paste (Ctrl+Shift+V), links (OSC 8).
- Accessibility with Orca, HiDPI/fractional scaling, second monitor.
- Long-running use, many terminals, high output rate (`yes`, `cat` of large files).
- Flatpak (deliberately later, needs a host agent).

### Found and fixed

- Terminal shortcuts as app accels had no effect → shortcut controller on the terminal (see above).

### Side effect in the VM

`dnf install vte291-gtk4-devel wl-clipboard` discarded an offline update queued by GNOME Software
(dnf message "Pending offline transaction has been invalidated").
GNOME Software schedules it again on its next run.

## Open architecture decisions (before/in phase 4)

| Question | Options | Proposal |
|---|---|---|
| Widgets as GObject subclasses | currently simple Rust structs with `Rc` | from phase 4 on `TerminalPane`, `SplitView`, `Window` as subclasses (template/Blueprint?) |
| Blueprint for UI files | Blueprint as in Kulisse, or code | Blueprint for dialogs/preferences, code for the split tree |
| Settings | GSettings vs. files | GSettings for app options, profiles/templates as files (`$XDG_CONFIG_HOME/parkett/`) |
| Serialization (sessions, templates) | serde + JSON/TOML | serde_json for sessions, TOML for templates (editable by hand) |
| Build/installation | Cargo only, Meson as in Kulisse | Meson from phase 4 on (schema, icons, .desktop, translations) |
| License | GPL-3.0-or-later (Cargo, About) | keep it, add a LICENSE file |
| Start commands of templates | type into the shell (`feed_child`) | decided with proposal 5 |
| Color scheme format | own, Tilix JSON, Ptyxis palette | Ptyxis palette (contains context colors for root/ssh) – to be checked |
