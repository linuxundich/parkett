# Backlog

Open work first, sorted by priority; finished work stays checked at the end. Sources: the
[QA audit of 2026-10-19](qa/2026-10-19/QA-REPORT.md) (IDs QA-xx and R-xx), the
[code review of phase 6](docs/QA.md) (IDs L, D, G) and notes from testing.

## Next release (0.3): fixes and small improvements

### From the QA audit
- [ ] QA-08 Take over the environment of a second invocation (`FOO=1 parkett --tab` reaches the
      running instance without `FOO`); needs rework in the spawn path
- [ ] R-04 `file:///` links (OSC 8) from an SSH session open local files with the same path;
      refuse `file:` links while the pane's context is remote
- [ ] R-05 Undo after a normal exit (profile *Bereich schließen* (Close pane)) brings back a dead
      pane without the exit bar
- [ ] R-06 Remove `GIO_LAUNCHED_DESKTOP_FILE` and `GIO_LAUNCHED_DESKTOP_FILE_PID` from the
      environment of new shells (GNOME Terminal does the same)
- [ ] R-07 Save the session after dragging a divider; the save timer restarts on every title change
- [ ] R-08 `ratio = nan` in a template ends up as `null` in `session.json` and the session is lost;
      clamp non-finite ratios on load
- [ ] R-09 A startup command is typed immediately; with an ssh profile it lands in the password
      prompt. Wait for the first prompt (vte.sh `precmd`) where available
- [ ] R-10 `-e "htop -d 5"` as one argument is not split; `--tab=x` is silently accepted
- [ ] R-11 `ssh -vp 22 host` shows the context chip „22“ instead of the host
- [ ] R-12 Write profiles and templates with mode 0600 (environment variables may hold tokens)
- [ ] QA-12 Tooltips from the current shortcut assignment (some are hard-coded)
- [ ] QA-14 Blinking text only while the window has focus (7.8 % CPU when idle in the VM)
- [ ] QA-15 Error dialog for unreadable files in a monospace font; toast without timeout
- [ ] QA-16 Profile editor opens without focus in the name field
- [ ] QA-18 Screen reader names: tab overview button, shortcut not part of the name
- [ ] QA-20 Cursor in the wrong place after line wrap: investigate, compare with Ptyxis
- [ ] Contrast test: sharpen the light variant of *Hoher Kontrast* (High Contrast) below 3:1
- [ ] `cargo audit` in `build-aux/ci.sh`
- [ ] Regression tests for QA-01 to QA-09 in the smoke test or as unit tests

### From the code review
- [ ] L1/L2 Weak references in the preferences and in the drawing functions of the pane
- [ ] L3 Pause the per-second foreground poll in background tabs and inactive windows
- [ ] D2 Merge the three foreground process queries in `terminal.rs`
- [ ] D3 One `Rgb` to Cairo/RGBA conversion in `core`
- [ ] D6 Debug output without `format!` on every divider move

### Small features
- [ ] High contrast: use the *Hoher Kontrast* color scheme automatically when GNOME's high
      contrast is on
- [ ] Selection color in dark mode (`set_color_highlight`), currently the VTE default
- [ ] Sort imported color schemes alphabetically right away
- [ ] Change the split layout of a template in the editor
- [ ] Ctrl+Enter in the command palette: open the target in a new pane
- [ ] Palette, layout mode and templates in the main menu (audit proposal 25)
- [ ] Match count in the search (QA-19)

## Decisions needed

- [ ] QA-11 Default when a process ends: keep the exit bar, or close the pane on exit code 0
      and show a neutral bar otherwise
- [ ] QA-17 Short tab label "program · folder" (with a number) instead of the shell title
- [ ] QA-13 Adaptive header bar below about 600 px window width (`AdwBreakpoint`)
- [ ] File paths in the output as links, Ctrl+click opens them in the file manager (audit T-53)
- [ ] Sixel: build and ship a VTE with Sixel support, or wait for the distributions

## Roadmap

### 0.3: Sessions and input
- [ ] Send input to several panes at once (Tilix "synchronized input"), clearly marked
- [ ] Drag and drop to move panes (Tilix-style drop zones)
- [ ] Automatic profile switching by host or directory
- [ ] Bookmarks for directories
- [ ] Title templates with variables

### 0.4: Distribution
- [ ] English interface (gettext), then translations; AppStream metadata and `.desktop` file in
      English with German translations
- [ ] AUR package and Fedora COPR from the public repository
- [ ] Debian package: Ubuntu 26.04 builds and runs unchanged (QA 2026-10-19); test
      `tuxedo-os-forky` and build a `.deb`
- [ ] Project page on linuxundich.de, then the "On the blog" section in the README

### Later
- [ ] Flatpak with a host agent (like Ptyxis), own repository, no Flathub
- [ ] Drop-down terminal (Quake style); on Wayland only possible with help from the shell
- [ ] Container integration (toolbox, distrobox) like Ptyxis

## Testing gaps

- [ ] New features of 0.2 on the host (Arch, GNOME 51): session, command line, notification,
      transparency; Nautilus 51 with the extension
- [ ] Sixel output with a VTE that has Sixel (Arch and Fedora 44 build without it)
- [ ] Launch through `xdg-terminal-exec` (installed neither in the VM nor on the host)
- [ ] Bell with sound
- [ ] Profile fields environment, directory, login shell and deleting a profile
- [ ] Dragging dividers with the mouse and dragging a tab into a new window
- [ ] Orca with speech output, by hand
- [ ] CPU under continuous output on the host with a real GPU
- [ ] X11 on Ubuntu, HiDPI with 200 % scaling, several monitors, other keyboard layouts
- [ ] Endurance run over hours

## Housekeeping

- [ ] Check the name: trademark registers (DPMA/EUIPO), domains
- [ ] R-13 Docs: `docs/QA.md` still says Shift+Insert does not warn (it does now); test counts in
      `docs/ARCHITECTURE.md` are outdated (now 61 + 2 + 1)
- [ ] German file names in the repository (`build-aux/qa/durchsatz.sh`, `messen.sh`,
      `icon/vorschau.png`, `icon/entwuerfe/`, screenshots under `docs/` and `qa/`)
- [ ] Code comments are German; translate when the code is touched anyway

## Observations

- [ ] Cursor invisible after closing the tab overview until the first key press; cause unknown
- [ ] Search once reported no match for text that was on screen (after `clear`, with older
      output in the scrollback, search text replaced with Ctrl+A); not reproducible afterwards

## Done

### Phase 2: Design
- [x] Three concepts (A GNOME Native, B Workspaces, C Terminal-centric) as HTML mockups
- [x] Decision: A + palette/layout mode from C + templates from B (2026-10-18)
- [x] Combined concept A+ as a mockup (`mockups/combined.html`)
- [x] A+ approved: floating pane title, suggestions 1–6 adopted (2026-10-18)
- [x] Project name: Parkett (2026-10-18; "Parquet" rejected because of Apache Parquet)
- [x] App icon and symbolic icon (`icon/`)
- [x] App ID de.linuxundich.Parkett, icons under `data/icons/hicolor/`

### Phase 3: Prototype
- [x] Language Rust (2026-10-18)
- [x] Window with libadwaita + VTE, asynchronous shell, process exit bar with *Neu starten* (Restart)
- [x] Title/directory from termprops, colors/font following the system, zoom, copy/paste
- [x] Core `layout` with tests (split, close, size, equalize, zoom, neighbor, minimum size)
- [x] Test in a Fedora VM (Wayland, X11, Unicode, dark, resize, memory) – docs/PROTOTYPE.md
- [x] Start on the host (Arch, GNOME 51) – phase 5
- [x] Approval for phase 4 (2026-10-18, architecture proposals accepted)

### Phase 4: MVP (docs/MVP.md)
- [x] Architecture: subclasses, GSettings + files, serde, Meson, GPL-3.0-or-later
- [x] Step 1: tabs and splits, close with confirmation, shortcuts overview
- [x] Step 2: Meson build, GSettings schema, LICENSE, Arch package (2026-10-18)
- [x] Step 3: profiles and color schemes (Ptyxis format, 14 schemes, import, tiles)
- [x] Step 4: preferences (General, Profiles, Keyboard Shortcuts)
- [x] Step 5: search, opening links, configurable shortcuts
- [x] Step 6: command palette, layout mode, templates with startup commands
- [x] Step 7: undo on close, context color per pane
- [x] Terminology: *Reiter* (tab) throughout
- [x] Native first, Flatpak later (docs/PACKAGING.md); color scheme format: Ptyxis palette

### Phase 5 and 6: Quality assurance, stabilization, release
- [x] Phase 5: quality assurance in the VM and on the host (docs/QA.md, 2026-10-19)
- [x] Code review (architecture, code quality) and findings fixed
- [x] Formatting (`rustfmt.toml`), Clippy without warnings, SPDX identifiers
- [x] CI script `build-aux/ci.sh` and VM smoke test `build-aux/vm/smoke.sh`
- [x] Docs: architecture, usage, README; examples in the usage guide as a test
- [x] Packaging options and licenses checked (docs/PACKAGING.md, docs/LICENSES.md)
- [x] Release 0.1.0 (local) and 0.2.0 (2026-10-19)
- [x] Public repository on GitHub, documentation in English (2026-10-19)

### QA audit 2026-10-19
- [x] QA-01 F2/F10 belong to the programs in the terminal again
- [x] QA-02 pane title covers text → design V2 (hidden, hover/focus change)
- [x] QA-03 no restart of closed tabs, release the toast; R-01 page query
- [x] QA-04 paste warning on every line break
- [x] QA-05/06 decode OSC 7, local machine only
- [x] QA-07 escape markup; QA-09 focus after menus
- [x] QA-10 palette One; R-03 quote the environment field; R-02 smoke test log

### 0.2: Everyday features
- [x] Command line: `parkett -- command`, `--working-directory` (several times = tabs), `--tab`,
      `--profile`, `--title`
- [x] Save the session and restore it on startup (layout, directories, without processes)
- [x] Notification when a long command finishes in a pane that is not visible
- [x] Font, cursor shape, blinking and bell per profile
- [x] Transparent background (opacity per profile)
- [x] Nautilus context menu via nautilus-python
- [x] Sixel switch, only active with a VTE that has Sixel
