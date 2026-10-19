# Backlog

## Phase 2 – Design
- [x] Three concepts (A GNOME Native, B Workspaces, C Terminal-centric) as HTML mockups
- [x] Decision: A + palette/layout mode from C + templates from B (2026-10-18)
- [x] Combined concept A+ as a mockup (`mockups/combined.html`)
- [x] A+ approved: floating pane title, suggestions 1–6 adopted (2026-10-18)
- [x] Choose the project name: Parkett (2026-10-18; "Parquet" rejected because of Apache Parquet)
- [x] App icon and symbolic icon (`icon/`)
- [ ] Still check the name: trademark registers (DPMA/EUIPO), domains
- [x] Choose the app ID, icons under `data/icons/hicolor/` (de.linuxundich.Parkett)

## Phase 3 – Prototype
- [x] Language Rust, app ID de.linuxundich.Parkett (2026-10-18)
- [x] Window with libadwaita + VTE, asynchronous shell, process-exited bar with *Neu starten* (Restart)
- [x] Title/directory from termprops, colors/font following the system, zoom, copy/paste
- [x] Core `layout` with tests (split, close, size, equalize, zoom, neighbor, minimum size)
- [x] Test in a Fedora VM (Wayland, X11, Unicode, dark, resize, memory) – docs/PROTOTYPE.md
- [x] Start on the host (Arch, GNOME 51) – phase 5
- [x] Approval for phase 4 (2026-10-18, architecture proposals accepted)

## Phase 4 – MVP (docs/MVP.md)
- [x] Architecture: subclasses, GSettings + files, serde, Meson, GPL-3.0-or-later (accepted)
- [x] Step 1: tabs and splits, close with confirmation, shortcuts overview (tested in VM)
- [x] Step 2: Meson build, GSettings schema, LICENSE, Arch package (2026-10-18)
- [x] Step 3: profiles and color schemes (Ptyxis format, 14 schemes, import, tiles) (2026-10-18)
- [x] Step 4: preferences (General, Profiles, Keyboard Shortcuts) (2026-10-18)
- [ ] Test the profile fields environment/directory/login shell and deleting in the VM
- [x] Step 5: search, opening links, configurable shortcuts (2026-10-18)
- [x] Step 6: command palette, layout mode, templates with startup commands (2026-10-18)
- [x] Step 7: undo on close, context color per pane (2026-10-18)
- [x] Phase 5: quality assurance (docs/QA.md, 2026-10-19)
- [x] Terminology: *Reiter* (tab) throughout (2026-10-18)
- [x] Flatpak from the start or native first → native first (docs/PACKAGING.md)
- [x] Color scheme format: Ptyxis palette (step 3)

## Phase 6 – Stabilization and release
- [x] Code review (architecture, code quality) and findings fixed
- [x] Formatting (`rustfmt.toml`), Clippy without warnings, SPDX identifiers
- [x] CI script `build-aux/ci.sh` and VM smoke test `build-aux/vm/smoke.sh`
- [x] Docs: architecture, usage, README; examples in the usage guide as a test
- [x] Packaging options investigated (docs/PACKAGING.md)
- [x] Licenses checked (docs/LICENSES.md)
- [x] Release decision (2026-10-19): tag v0.1.0, local only for now; the only channel is the Arch repository on bender
- [x] Public repository on GitHub, documentation in English (2026-10-19)
- [ ] AUR and Fedora COPR

## QA audit 2026-10-19 (`qa/2026-10-19/QA-REPORT.md`)
- [x] QA-01 F2/F10 belong to the programs in the terminal again
- [x] QA-02 pane title covers text → design V2 (hidden, hover/focus change)
- [x] QA-03 no restart of closed tabs, release the toast; R-01 page query
- [x] QA-04 paste warning on every line break
- [x] QA-05/06 decode OSC 7, local machine only
- [x] QA-07 escape markup; QA-09 focus after menus
- [x] QA-10 palette One; R-03 quote the environment field; R-02 smoke test log
- [ ] QA-08 take over the environment of a second invocation (`FOO=1 parkett --tab`)
- [ ] QA-11 default on process exit (close on code 0? neutral bar) – decision
- [ ] QA-12 tooltips from the current shortcut assignment
- [ ] QA-13 header bar below 600 px (AdwBreakpoint)
- [ ] QA-14 blinking text only when focused
- [ ] QA-15 error dialog in a monospace font, toast without timeout
- [ ] QA-16 profile editor without focus in the name field
- [ ] QA-17 short tab label "program · folder" (decision)
- [ ] QA-18 screen reader names (overview, shortcut not in the name)
- [ ] QA-19 match count in search (optional)
- [ ] QA-20 cursor after line wrap: compare with Ptyxis
- [ ] R-04 `file:///` links from SSH sessions; R-05 to R-12 from the report
- [ ] *Hoher Kontrast* (High Contrast) light: sharpen colors below 3:1; `cargo audit` in CI
- [ ] Add palette, layout mode and templates to the main menu

## Development plan

Order is a proposal; each stage ends with a test in the VM and on the host, and a package.

### 0.2 – Gaps found in daily use
- [x] Font, cursor shape, blinking and bell per profile (2026-10-19)
- [x] Transparent background (opacity per profile) (2026-10-19)
- [x] Nautilus context menu via nautilus-python (2026-10-19)
- [x] Sixel switch, only active with a VTE that has Sixel (2026-10-19)
- [ ] Test Sixel with a VTE built with Sixel support (Arch and Fedora 44 build without it)
- [ ] High contrast: use the *Hoher Kontrast* color scheme automatically
- [ ] Selection color in dark mode (`set_color_highlight`)
- [ ] Dragging the divider with the mouse and dragging tabs into a new window: test and polish
- [ ] Cursor invisible after closing the tab overview – find the cause
- [ ] Sort imported schemes alphabetically right away
- [ ] Change the split layout of a template in the editor
- [ ] Ctrl+Enter in the palette: open the target in a new pane

- [x] Command line: `parkett -- command`, `--working-directory`, `--tab`, `--profile`, `--title` (2026-10-19)
- [ ] Cleanup from the review: L1/L2 weak references in preferences/drawing functions,
      L3 pause the per-second timer in background tabs, D2/D3 merge the foreground query and
      color conversion, D6 debug output (docs/QA.md, phase 6)

### 0.3 – Sessions
- [x] Save the session and restore it on startup (layout, directories, without processes)
      (2026-10-19)
- [ ] Send input to several panes at once (Tilix "synchronized input"), with
      clear marking
- [x] Notification when a long command finishes in a background tab (vte.sh termprops, 2026-10-19)

### 0.4 – Distribution
- [ ] English interface (gettext), then translations
- [ ] Debian/Ubuntu: Ubuntu 26.04 builds and runs unchanged (QA 2026-10-19); `tuxedo-os-forky` and a .deb still open

### Later
- [ ] Flatpak with a host agent (like Ptyxis), own repository – no Flathub
- [ ] Drop-down terminal (Quake style) – on Wayland only possible with help from the shell, check first
- [ ] Check Orca with speech output by hand; measure CPU under continuous output on the host

## Observations
- [ ] Search once reported no match for text that was on screen (after `clear`, with older output in the scrollback, search text replaced with Ctrl+A); not reproducible afterwards
