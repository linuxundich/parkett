# Phase 1 – Analysis: Parkett, a Tilix successor for GTK4/libadwaita

As of: 2026-10-18. Based on the source code (shallow clones from 2026-10-18),
the locally installed libraries (gtk4 4.24.1, libadwaita 1.10.0, vte4 0.84.1,
Rust crates gtk4 0.11.5, libadwaita 0.9.2, vte4 0.10.0) as well as issues and release notes.

Legend: **F** = fact with evidence · **C** = conclusion · **P** = proposal.
Tilix paths are relative to `source/gx/tilix/` in the repo gnunn1/tilix (HEAD `46b6a7c`).

---

## 1. The situation in three sentences

- **F** Tilix is effectively frozen: the last release is 1.9.6 from 2023-11-08, since then almost only
  translations. The README has been looking for maintainers since 2019 (issue #1700) and promises "no new features".
  It is written in D with GtkD 3; GtkD only supports GTK3 and has been considered abandoned since 2021 (comment in #2013).
  No GTK4 port exists. PR #2282 (switch to the giD bindings) has been stalled since 2026-04.
- **F** Locally, the installed tilix 1.9.6 no longer starts (`libphobos2-ldc-shared.so.112` is missing);
  it has been removed from the official Arch repositories (#2264).
- **F/C** Among the GTK4/libadwaita terminals, only **Ghostty** has real splits – without
  a settings UI, without session restore on Linux, without drag and drop of splits on GTK.
  GNOME Console, Ptyxis, Black Box and elementary Terminal have no splits.
  **The niche "GNOME-native terminal with splits, profiles and a GUI" is unoccupied.**

---

## 2. Inventory report (the eight questions)

### 2.1 What makes Tilix special?

| Feature | Evidence |
|---|---|
| Tiling within a tab ("session") as a binary tree of GtkPaned, right/down/automatic along the longer side | `session.d:417, 507, 1491` |
| Split ratio is preserved when resizing (custom ratio mechanism) | `session.d:1631–1720` |
| Distribute evenly by double-clicking the separator | `session.d:200–260` |
| Maximize a terminal within the session (separate stack page) | `session.d:684–745` |
| Drag and drop of terminals by their title bar, dropping into four quadrants, also between sessions and windows, detaching into a new window | `terminal/terminal.d:3142–3450`, `session.d:544, 628` |
| Save/load a session as JSON (layout, profiles, directories, titles, optionally commands) | `session.d:784–1190` |
| Sidebar with thumbnails of the sessions (F12), tabs as an alternative | `sidebar.d:491`, `appwindow.d:1047` |
| Profiles with title templates (`${title} ${directory} ${process}` …), badges, custom links | `constants.d:122–187`, `terminal.d:2578, 3478` |
| Automatic profile switching by `user@host:/path` (via OSC 7) | `preferences.d:465–500` |
| Triggers (regex on output → 9 actions) – only with Fedora-patched VTE | `terminal.d:1589–1745` |
| Synchronized input per session, exception per terminal | `session.d:656`, `terminal.d:398` |
| Read-only, monitor silence/activity, password manager (libsecret), bookmarks | `terminal/password.d`, `bookmark/manager.d` |
| Quake mode – disabled on Wayland | `appwindow.d:1705–1730`, #1314 |
| CLI actions (`--action session-add-right`), `new-instance-mode` (e.g. split-right) | `application.d:679–700` |

**C** The core that users care about: **splits + sessions with layout + profiles**. Everything else is
extras, some of which depend on patched VTE or X11.

### 2.2 How do sessions, tabs, splits and workspaces work?

- **F** Hierarchy: `AppWindow` → `Notebook` (one page per session) → `Session : Stack` (pages
  "group" and "maximized") → binary tree of `TerminalPaned`, each child in a helper box
  (`session.d:432–438`). The session never sees the VTE directly, only the composite widget `Terminal`
  (title bar + overlay + ScrolledWindow + VTE, `terminal.d:199`).
- **F** A new terminal created by splitting inherits the profile and the current directory (`session.d:417`).
- **F** Directional focus is geometric (search in 20 px steps), keyboard resize 10 px per step
  (`session.d:1315, 1386`). No minimum size of its own – the VTE minimum applies (`:1622`).
- **F** Process start is synchronous (`vte.spawnSync`, `terminal.d:2896`); under Flatpak a separate path via
  `org.freedesktop.Flatpak.Development.HostCommand`.
- **F** "Workspaces" in the sense of projects do not exist; the closest things are saved
  session files and bookmarks.

### 2.3 Which interaction concepts still make sense?

**C**
- Splits within a tab using a binary tree with a preserved ratio.
- The keyboard shortcut set (Ctrl+Alt+R/D to split, Alt+Arrow for focus, Shift+Ctrl+X to maximize) – many
  users have it in their muscle memory. **P** Adopt the default bindings where they do not conflict
  with GNOME.
- Drag and drop with quadrant drop targets – Ghostty lacks this on GTK, it would be a unique selling point.
- Saving and opening session layouts.
- Profile switching by host/directory.
- Marking new output/finished processes.

### 2.4 What feels outdated?

**C** (from `prefeditor/*`, `appwindow.d:311–405`, `terminal.d:341–420`)
- GTK3 forms: Grid + ComboBox + CheckButton + FileChooserButton, profile editor as a notebook with
  seven tabs, a custom preferences window with two header bars.
- Options that require a restart (tabs instead of sidebar, transparency, overlay scrollbars).
- A full title bar per terminal with MenuButton, sync toggle, icons – costs height in every pane
  and repeats information.
- Sidebar with self-rendered thumbnails (detour via OffscreenWindow, `gx/gtk/cairo.d`);
  `AdwTabOverview` provides this out of the box today.
- Features that depend on Fedora VTE patches (triggers, badges, process-finished notice).
- Quake mode only on X11.

### 2.5 What can be implemented in a contemporary way with GTK4, libadwaita and VTE?

| Feature | Building block (F, since version) |
|---|---|
| Tabs, pinning, attention, dragging between windows | `AdwTabView`/`AdwTabBar` (1.0), `needs-attention`, `indicator-icon` (1.2), `create-window` |
| Tab overview with live preview and search | `AdwTabOverview` (1.3), `live-thumbnail`, `keyword` |
| Sidebar | `AdwOverlaySplitView` (1.4), `AdwSidebar` (1.9, prefix/suffix 1.10) |
| Adaptive layouts | `AdwBreakpoint` (1.4) |
| Preferences, dialogs, keyboard shortcut overview | `AdwPreferencesDialog`, `AdwAlertDialog` (1.5), `AdwShortcutsDialog` (1.8) |
| System monospace font, accent color | `AdwStyleManager:monospace-font-name` (1.7), `accent-color` (1.6) |
| Splits | nested `GtkPaned` with `shrink-*`/`resize-*` (4.0) |
| Drag and drop of panes | `GtkDragSource`/`GtkDropTarget` (4.0) |
| Spawn, process exit | `vte_terminal_spawn_async`, `child-exited`, `VTE_SPAWN_REQUIRE_SYSTEMD_SCOPE` (0.60) |
| Directory, title, container, command boundaries | **Termprops** (0.78): `vte.cwd`, `xterm.title`, `vte.container.*`, `vte.shell.precmd/preexec/postexec`, `vte.progress.*` (0.80) |
| Command-finished notice without a patch | `vte.shell.postexec` (termprop) or OSC 777 (`set_enable_legacy_osc777`, 0.78) |
| Search, URLs, OSC 8 | `search_set_regex` (PCRE2), `match_add_regex`, `check_hyperlink_at` |
| Context menu | `set_context_menu_model` (0.76) |
| Copy as HTML | `copy_clipboard_format(VTE_FORMAT_HTML)` |
| Accessibility | `set_enable_a11y` (0.78), GtkAccessibleText; #2614 closed 2025-09, follow-up bugs fixed up to 2026-10 |
| Scrollback | on disk, LZ4 + AES-256-GCM (`src/vtestream-file.h:84, 100`) |

### 2.6 What needs additional libraries or custom code?

| Need | Solution | Assessment |
|---|---|---|
| Split tree, directional focus, ratio, minimum size, distribute evenly | own module (pure, testable) | **C** core of the project, custom code anyway |
| Drag and drop with quadrant preview | custom code on top of GtkDragSource/DropTarget | medium |
| Save/restore sessions | own data format (serde/JSON), path `$XDG_STATE_HOME` | low |
| Flatpak: PTY and processes on the host | host agent following the Ptyxis pattern (D-Bus over socketpair, `CreatePty`, `HasForegroundProcess` …) | **C** high effort; without Flatpak (pacman/AUR) it is not needed |
| Detect the foreground process (confirmation on close) | `tcgetpgrp(pty_fd)` + `/proc` (as in `ptyxis-process-impl.c:283`) | low outside Flatpak |
| Containers (Podman/Toolbox/Distrobox) | like the Ptyxis agent | optional, later |
| Password manager | libsecret | optional, rather leave out |
| Quake/dropdown | GNOME: shell extension required; layer shell **not available** under Mutter (test: `Gtk4LayerShell.is_supported()` = False on GNOME Shell 51.1) | **C** not in the MVP |
| Ligatures, Kitty keyboard protocol, Kitty graphics | not available in VTE (#192, #2601 open) | **C** deliberately not – accept the VTE limit |

### 2.7 Adopt, change, optional, leave out

See the feature matrix in section 3.

### 2.8 Risks of a rewrite

| Risk | Type | Assessment / countermeasure |
|---|---|---|
| vte4-rs lags behind: some termprop functions and the signals `hyperlink-hover-uri-changed`, `setup-context-menu` are not bound (`vte4-0.10.0/src/auto/terminal.rs:1369–1688, 2638, 2755`) | technical | **C** low – `connect_local` or targeted FFI via vte4-sys; list the needs beforehand |
| GtkPaned trees with many panes: even distribution, minimum sizes, drift | technical | **C** medium – keep the ratio in the model; a custom `GtkLayoutManager` if needed |
| Focus/keyboard conflicts between VTE and app shortcuts (Ctrl+Shift+…), GNOME shortcuts | technical/UX | medium – shortcut editor, collision check |
| Flatpak sandbox: PTY, cwd, foreground process only with a host agent | technical | high if Flatpak is mandatory – **P** native first (pacman repo), Flatpak later |
| VTE accessibility under GTK4 is still young (#2901 open) | technical | medium – test with Orca, promise nothing |
| Memory with many terminals: scrollback per terminal | technical | low – VTE swaps to disk; default 10,000 lines |
| X11 increasingly deprecated in GTK 4.18+ (55 symbols `GDK_DEPRECATED_IN_4_18`) | technical | low – build nothing X11-specific |
| Feature set grows to Tilix-level complexity | design | high – strict MVP boundary, approvals per phase |
| Two navigation levels (tabs + splits + possibly workspaces) are overwhelming | design | medium – this is exactly what concepts A/B/C decide |
| Focus in splits not clearly recognizable | design/a11y | medium – never opacity alone; always an additional cue (line/border) |
| Showing context (root/ssh) per split instead of per window | design | new, no precedent – design carefully |
| Name/trademark (Tilix is the name of someone else's project) | organizational | own name needed – open |

---

## 3. Feature matrix (prioritized)

Priority: **P0** = MVP (indispensable) · **P1** = sensible extension after the MVP ·
**P2** = optional/convenience · **–** = deliberately not.
Origin: T = Tilix, C = Console, P = Ptyxis, B = Black Box, G = Ghostty, K = Kitty.

### Basic terminal features

| Feature | Prio | Implementation | Rationale |
|---|---|---|---|
| Shell in its own process, login shell selectable | P0 | `spawn_async`, systemd scope | basic feature; Tilix still starts synchronously |
| Complete emulation | P0 | VTE 0.84 | no convincing alternative under GTK4 (G uses its own rendering in Zig) |
| Copy/paste incl. primary selection, warning on multi-line/unsafe paste | P0 | VTE + own check | security (T had this) |
| Search in the terminal (regex, case sensitivity, wrap around) | P0 | `search_*` | |
| Zoom (Ctrl+Plus/Minus/0) | P0 | `font_scale` | |
| Font (follow system monospace), color schemes | P0 | `AdwStyleManager`, `set_colors` | |
| URLs, OSC 8, opening only on Ctrl+click / context menu | P0 | `match_add_regex`, `check_hyperlink_at`, `GtkUriLauncher` | safe handling of external actions |
| Configurable shortcuts | P1 | own editor | MVP with fixed, documented shortcuts |
| Copy as HTML | P2 | `VTE_FORMAT_HTML` | cheap |
| Sixel | P2 | `set_enable_sixel` | cheap, a toggle |
| Ligatures, Kitty protocols | – | VTE cannot do it | |

### Tabs and sessions

| Feature | Prio | Rationale |
|---|---|---|
| Multiple tabs (tab = session with a split tree) | P0 | T model |
| Rename (double-click, shortcut, palette) | P0 | |
| Status: running / new output / finished (exit code) | P0 | termprops `vte.shell.*`, `child-exited` |
| Finished process: pane stays with a bar "Restart / Close" (per profile: close/keep/restart) | P0 | T `exit-action` |
| Confirmation on close when a foreground process is running | P0 | P `prompt-on-close`; `tcgetpgrp` |
| New terminal inherits the directory (OSC 7 / `vte.cwd`) | P0 | |
| Profiles (command, directory, environment, font, colors, behavior on exit) | P0 (basic) / P1 (full) | |
| Tab overview | P1 | `AdwTabOverview`, almost free |
| Drag a tab into a new window | P1 | `create-window` |
| Restore sessions at startup (layout, cwd, profile, title – no processes) | P1 | reliably doable (cf. `ptyxis-session.c`) |
| Automatic profile switching by host/directory | P1 | T strength; requires vte.sh/OSC 7 |
| Context color for root/ssh/container – **per pane** | P1 | C/P/B color the window; with splits it has to be the pane (new) |
| Title templates with variables | P2 | T; a simple set is enough |
| Badges | – | rarely used, needs custom drawing |

### Split terminals

| Feature | Prio | Rationale / implementation |
|---|---|---|
| Split right/down, nested | P0 | binary tree |
| Split automatically (longer side) | P0 | T `addAutoOrientedTerminal` |
| Keyboard focus (direction, number, last used) | P0 | geometric as in T, with wrap-around as in G |
| Resize (mouse, keyboard), ratio is kept when the window is resized | P0 | ratio in the model |
| Close → sibling moves up | P0 | |
| Minimum size approx. 20×5 characters; no further splitting below that | P0 | own rule from `char-size-changed` |
| Maximize/zoom a pane with a visible "n hidden" hint | P0 | T/G; G shows an icon |
| Distribute evenly | P1 | T double-click, G `equalize_splits` |
| Swap/move a pane by drag and drop (quadrants) | P1 | T; G on GTK only "planned" |
| Undo for closing/restructuring | P1 | G (immutable tree) |
| Save/open layout templates | P1 | T session files, K sessions |
| Synchronized input | P2 | T; useful for servers, but risk of mistakes |
| Fixed layouts (tall/grid/stack) | P2 | K; as an "Arrange" command |
| Very many panes (> 6) | – as a goal | **C** nesting beyond 2–3 levels makes operation worse; a second tab is better |

### Advanced features

| Feature | Prio | Rationale |
|---|---|---|
| Starting several terminals in a defined layout (template + directory) | P1 | main benefit of "workspaces" |
| Project-based workspaces | P1/P2 | depending on concept B (central) or A/C (as a template) |
| Named directories/bookmarks | P2 | T bookmarks; the palette covers part of it |
| Search across multiple terminals | P2 | termprop/text API partly unbound in vte4-rs; limited benefit |
| Desktop notification on command completion/activity (only when the window/tab is inactive) | P1 | without a VTE patch via `vte.shell.postexec` |
| Command palette | P1 (A/B) / P0 (C) | K/G; replaces deep menus |
| Export/import of settings | P2 | one TOML/JSON file |
| Configuration file alongside the GUI | P2 | for dotfile users |
| Container integration | P2 | P agent; large effort |
| Quake/dropdown | P2 | only sensible with a GNOME Shell extension (experience from Gnomos/Pulsgeber available) |
| Triggers | – | complex, risk of mistakes and security risk; parts can be covered by termprops |
| Password manager | – | security risk, the system password manager is better |
| Transparency, background image | P2 / – | transparency is cheap; a background image is not clean without a VTE patch |
| Read-only | P2 | cheap (`set_input_enabled`) |

---

## 4. Choice of language

| Criterion | Rust (gtk4-rs 0.11, libadwaita 0.9.2, vte4 0.10) | C | Vala | Python/PyGObject |
|---|---|---|---|---|
| Memory safety | high (except deliberate FFI) | low | medium | high |
| VTE coverage | good; feature flags up to `v0_84`, termprops from `v0_78`; some functions/signals unbound | complete | VTE's VAPI | complete (GIR) |
| libadwaita 1.10 | bound (`v1_10`, incl. AdwSidebar) | native | VAPI | GIR |
| Error handling | `Result`, type system | manual `GError` | exceptions | runtime errors |
| Effort | high (subclassing, closures) | high (boilerplate) | low | low |
| Maintenance | gtk-rs active; vte4-rs last release 2026-03 | GNOME standard (Console, Ptyxis) | ecosystem shrinking | weaker startup time/performance |
| Testability of the layout logic | very good (pure module, `cargo test`) | medium | medium | good |
| Relation to your projects | Blocksatz, ScreenForge, Kulisse, Freisteller, Tastenspur (Rust + GTK4 + Flatpak) | Gnomos (C/Meson) | – | editors (Python) |
| Reference app with vte4-rs | Pods (marhkb/pods) | Console, Ptyxis | Black Box | – |

**Recommendation (C/P): Rust.** The split, session and profile logic is the actual custom-built part
and can be tested cleanly in Rust as a GTK-free module. The gaps in vte4-rs are small and
known. C would only make sense if we wanted to take code from Ptyxis (GPL-3.0) directly – the
host agent pattern can also be rebuilt without taking over code. According to the plan, the
language is only finally decided in phase 3.

---

## 5. Architecture recommendation (first draft)

```
app (GTK/Adw, presentation and input only)
 ├─ window      window, header bar/chrome, breakpoints, actions (GAction), shortcuts
 ├─ tabs        AdwTabView binding, overview, dragging between windows
 ├─ splitview   translates layout model ⇄ GtkPaned tree, DnD, focus display
 ├─ terminal    wrapper around vte4::Terminal: spawn, termprops → state, search, links, menu
 ├─ prefs       AdwPreferencesDialog, profile editor
 └─ palette     command palette (if chosen)
core (no GTK, fully testable with cargo test)
 ├─ layout      immutable binary tree: split/close/swap/resize/equalize/focus(dir), undo stack
 ├─ session     window → tabs → layout → terminal description; (de)serialization, restore
 ├─ profile     profiles, color schemes, selection rules (host/directory)
 └─ config      settings (GSettings or TOML – open), shortcut table
process
 ├─ spawn       shell/command, environment, systemd scope
 └─ host        later: host agent for Flatpak (Ptyxis pattern)
```

Guidelines:
- **Model before widget:** The split tree lives in `core::layout`; GTK widgets are generated from it
  and never read back the other way. This makes restoring, undo and tests simple (Ghostty pattern:
  `src/datastruct/split_tree.zig`).
- **Terminals survive restructuring:** When moving/zooming, the VTE widget is reparented, not
  recreated (Tilix does the same, `session.d:684–745`).
- **State from termprops**, not from deprecated signals (`current-directory-uri-changed` has been
  deprecated since 0.78).
- **External actions only explicitly:** links via Ctrl+click or menu, `GtkUriLauncher`/portal;
  no execution of text from the terminal.
- **Settings:** GSettings (customary on GNOME, Tilix/Ptyxis) vs. a TOML file (dotfiles,
  trivial export) – **decision open**, proposal: GSettings for app settings,
  profiles/templates as files under `$XDG_CONFIG_HOME`, sessions under `$XDG_STATE_HOME`.
- **Packaging:** native first (pacman repo `linuxundich`), Flatpak only with a host agent.
- **License:** open. Tilix is MPL-2.0, Ptyxis/Console GPL-3.0. Freely choosable without taking over code;
  GPL-3.0-or-later fits the GNOME role models.

---

## 6. Sources

- Tilix: github.com/gnunn1/tilix (README, NEWS, `source/gx/tilix/*`, schema `data/gsettings/com.gexperts.Tilix.gschema.xml`), issues #1314, #1601, #1700, #2013, #2264, #2282
- VTE: headers `/usr/include/vte-2.91-gtk4/vte/` (0.84.1), `src/vtestream-file.h`, issues #192, #2601, #2614, #2901, #2934, #2959 (gitlab.gnome.org/GNOME/vte)
- libadwaita: `/usr/share/gir-1.0/Adw-1.gir` (1.10.0), stylesheet from `libadwaita-1.so` (colors of the mockups)
- GTK: `/usr/include/gtk-4.0` (4.24.1), `gtkpaned.h`, `gdk/x11/*`
- vte4-rs 0.10.0, gtk4-rs 0.11.5, libadwaita-rs 0.9.2 (Cargo registry), Pods (github.com/marhkb/pods)
- Ptyxis: gitlab.gnome.org/chergert/ptyxis (`src/ptyxis-session.c`, `agent/ptyxis-process-impl.c`, `agent/org.gnome.Ptyxis.Agent.xml`, schema), tag 50.1
- GNOME Console 51.0 (local, `src/style.css`, schema), Black Box 0.15.2 (schema, `Window.vala`, `ThemeProvider.vala`)
- Ghostty 1.3.x (`src/datastruct/split_tree.zig`, `src/config/Config.zig`, release notes 1.3.0/1.3.1)
- Kitty 0.49.2 (`docs/layouts.rst`, `docs/actions.rst`), Foot 1.28.0, WezTerm, Konsole `ViewSplitter.h`
- Local layer shell test: GNOME Shell 51.1, `Gtk4LayerShell.is_supported()` → False
