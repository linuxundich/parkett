# Backlog

Open work first, sorted by priority; finished work stays checked at the end. Sources: the
[QA audit of 2026-10-19](qa/2026-10-19/QA-REPORT.md) (IDs QA-xx and R-xx), the
[code review of phase 6](docs/QA.md) (IDs L, D, G) and notes from testing.

## Next release (0.5): fixes and small improvements

### From the QA audit
- [x] Regression test for QA-02 (pane title hidden): screenshot comparison in the smoke test

### From the code review

### Small features
- [x] Change the split layout of a template in the editor

## Decisions needed

- [x] QA-13 Adaptive header bar below about 600 px window width (`AdwBreakpoint`; down to 360 px)
- [x] File paths in the output as links, Ctrl+click opens them in the file manager (audit T-53)
- [x] Sixel: wait for the distributions to build VTE with Sixel (decided 2026-10-23); no own
      VTE build

## Roadmap

### 0.4: Sessions and input (released 2026-10-23)
- [x] Send input to several panes at once (Tilix "synchronized input"), clearly marked
      (per tab, panes can opt out; decided 2026-10-23)
- [x] Drag and drop to move panes (drop zones: edges place beside, middle swaps; same tab only)
- [x] Automatic profile switching by host or directory (appearance only, returns afterwards)
- [x] Bookmarks for directories (palette `~`, main menu, `bookmarks.toml`)
- [x] Title templates with variables (setting `tab-label-template`)
- [x] Move panes into another tab by dropping them on the tab bar (also between windows)
- [x] Bookmarks page in the preferences (rename, sort, delete, add a folder)
- [x] Title template per profile (`tab_label`)

### 0.5: Distribution
- [x] English interface (gettext), then translations; AppStream metadata and `.desktop` file in
      English with German translations
- [ ] AUR package and Fedora COPR from the public repository
- [ ] Debian package: Ubuntu 26.04 builds and runs unchanged (QA 2026-10-19); test
      `tuxedo-os-forky` and build a `.deb`
- [ ] Project page on linuxundich.de, then the "On the blog" section in the README

### Tilix parity (comparison 2026-10-23)
- [x] Drag a tab into a new window (`create-window`)
- [x] Scrollbar per profile (overlay, `ScrolledWindow` like Ptyxis)
- [x] Reset / reset and clear
- [x] Drop files onto the terminal (quoted paths)
- [x] Save output to a file
- [x] Monitor activity and silence per pane
- [ ] Copy on select (setting)
- [ ] Read-only terminal (`set_input_enabled`)
- [ ] Copy as HTML
- [ ] Advanced paste: edit before pasting, convert tabs to spaces, CRLF
- [ ] Bookmarks for commands and SSH targets (Tilix has folder, path, command, remote)
- [ ] Command line: `--split-right`/`--split-down` for a call to a running instance
- [ ] Custom links per profile (regex → command, e.g. `#123` → issue in the browser)
- [ ] Double-click on a divider equalizes
- [ ] Text options per profile: bold is bright, cell spacing, scroll on output/keystroke
- [ ] Search: whole words only
- [ ] Window title template
- Deliberately not (ANALYSIS.md): triggers, badges, password manager, background image,
  sidebar with thumbnails (tab overview instead)

### Later
- [ ] Flatpak with a host agent (like Ptyxis), own repository, no Flathub
- [ ] Drop-down terminal (Quake style); on Wayland only possible with help from the shell
- [ ] Container integration (toolbox, distrobox) like Ptyxis

## Testing gaps

- [ ] Still open from the VM run (2026-10-22/23): context color right after switching tabs;
      renaming a tab through the dialog (custom title via `-t` passed); Orca with speech output
- [x] Host run 2026-10-23 (Arch, GNOME 51.1, GTK 4.24.1, libadwaita 1.10.0, VTE 0.84.1) in a
      private headless session (own D-Bus, `mutter --headless`, XDG dirs and GSettings keyfile in
      a temp dir – the desktop instance and `~/.config/parkett` untouched), build from `main`:
      session restore (2 tabs, split, directory `/tmp` kept), command line (`-d` twice = window
      with two tabs, `--tab -t Logs --`, unknown profile falls back with a message, `--tab=x`
      refused, `--help`), notification after `sleep 11` in a hidden tab and its click target,
      profile fields (login shell `bash -l`, directory, environment with an invalid entry
      filtered), Nautilus 51: nautilus-python loads, the extension (unchanged since 0.2)
      returns *In Parkett öffnen* for folders, nothing for a file, *Parkett hier öffnen* in the
      background, and opens one window with a tab per folder
- [x] VM run 2026-10-22/23 (Fedora 44, GNOME 50), all passed or fixed:
      R-04 (`file:///` OSC 8 link inside `ssh localhost` refused with dialog, locally it opens
      Nautilus), R-05 (Undo after `exit` brings the pane back with the exit bar, *Neu starten*
      works), R-06 (no `GIO_LAUNCHED_*` in the shell after `gtk-launch`), R-07 (divider at 27 %
      survives a restart), R-10 (`-e "htop -d 5"`), QA-11 (`-- false` red bar with exit code 1,
      `-- true` neutral bar with `hold`, profile without `exit_action` closes the window),
      QA-12 (changed shortcut in header and pane button tooltips), QA-14 (idle CPU 0.8 % with
      blinking text in 7 tabs, was 7.8 %), L3 (background tab label updates after ≈ 5 s),
      QA-16 (existing profile: name not selected), QA-17 (labels with `htop`, after `cd`,
      reordering, custom title, switch off/on), QA-18 names via AT-SPI, import through the
      dialog (sorted), selection in light mode with blue/orange/green accent, match count with
      300 000 lines. The lost first key in an earlier run was the test's own Esc (Meta+e in bash)
- [ ] On the host by hand: deleting a profile in the preferences
- [x] On the host by hand (Christoph, 2026-10-23): transparency, the Nautilus context menu
      (right click on a folder), several monitors
- [ ] Sixel output with a VTE that has Sixel (Arch and Fedora 44 build without it)
- [ ] Launch through `xdg-terminal-exec` (installed neither in the VM nor on the host)
- [ ] Bell with sound
- [ ] Dragging dividers with the mouse and dragging a tab into a new window
- [ ] Orca with speech output, by hand
- [ ] CPU under continuous output on the host with a real GPU
- [ ] X11 on Ubuntu, HiDPI with 200 % scaling, other keyboard layouts
- [ ] Endurance run over hours

## Housekeeping

- [ ] Check the name: trademark registers (DPMA/EUIPO), domains
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

### 0.3 fixes from the QA audit
- [x] R-04 refuse `file:` links while the pane's context is remote
- [x] R-06 `GIO_LAUNCHED_DESKTOP_FILE(_PID)` removed from the environment of new shells
- [x] R-07 save the session after dragging a divider; save timer no longer restarts
- [x] R-08 non-finite ratios in templates become 0.5
- [x] R-11 grouped ssh options (`ssh -vp 22 host`) in the context chip
- [x] R-12 profiles, templates and session written with mode 0600
- [x] R-13 docs: Shift+Insert warning in `docs/QA.md`, test counts in `docs/ARCHITECTURE.md`
- [x] R-05 Undo after an automatic close on exit brings the pane back with the exit bar
- [x] R-10 `-e "htop -d 5"` is split like a shell; flags with a value (`--tab=x`) are refused
- [x] QA-11 default: close the pane on exit code 0; neutral bar for a normal end (decided 2026-10-21)
- [x] QA-17 short tab label „1 htop · parkett“, switch under *Allgemein* (decided 2026-10-21)
- [x] QA-14 blinking text only in the focused pane (`TextBlinkMode::Focused`)
- [x] L3 foreground poll every 5 s instead of every second in background tabs and inactive windows
- [x] QA-12 tooltips built from the current shortcuts when shown (header bar, pane, search)
- [x] QA-16 existing profile opens without the name selected (new profile: name selected)
- [x] QA-18 screen reader names without shortcut; overview button „Reiter-Übersicht, 4 Reiter“
- [x] `cargo audit` in `build-aux/ci.sh` (no findings on 2026-10-21)
- [x] Unit tests for the default shortcuts (QA-01: nothing without Ctrl/Alt/Super, no duplicates)

### 0.3 fixes, second round (2026-10-22)
- [x] QA-15 error dialog: monospace, path on its own line, *Kopieren*, toast without timeout;
      palette message names the missing `[Light]`/`[Dark]`
- [x] *Hoher Kontrast* sharpened: colors 1–6 / 9–14 at 7:1 or 4.5:1 (unit test)
- [x] D3 one `Rgb` ⇄ float conversion in `core` (`Rgb::to_f64`, `Rgb::from_f64`)
- [x] D6 layout debug output as a macro, environment read once
- [x] `parkett -- cmd` on first start: GLib removed the `--` (found in the VM; `cli::protect_separator`)

### After 0.3.0 (2026-10-23)
- [x] L1/L2 handlers of the preferences released on destroy (`release_on_destroy`), drawing
      functions with `WeakTerminal`
- [x] D2 one foreground read (`read_foreground`) for context, label, notification and template
- [x] Ctrl+Enter in the command palette: profile in a new pane (`win.split-profile`)
- [x] Palette, layout mode and templates in the main menu (audit proposal 25)
- [x] Smoke test: QA-07 (markup in a tab title) and QA-09 (focus after the main menu) via
      `build-aux/vm/atspi.py`; 16 checks
- [x] QA-20 closed: not reproducible with VTE 0.84.1 (Fedora 44); after narrowing, the
      prompt shows longer in Parkett and Ptyxis alike (VTE rewrap plus bash redraw), the cursor
      is right after maximizing

### Spawn path (2026-10-23)
- [x] QA-08 environment of a call to a running instance (`SEND_ENVIRONMENT`,
      `VTE_SPAWN_NO_PARENT_ENVV`, filter in `core::environment`; session variables from Parkett)
- [x] R-09 start commands wait for the first prompt or quiet output, never into a password
      prompt (`core::startup`; VM: ssh with password waits, local and ssh with key typed)

### VM test run 2026-10-22/23: fixes
- [x] A link whose default app fails to start shows a toast instead of doing nothing
- [x] QA-18: main part of *Neuer Reiter* had the description „Weitere Möglichkeiten“ (libadwaita
      `described-by`); now „Neuen Reiter öffnen“
- [x] Match count reads the scrollback from the bottom in blocks and stops at 999: with 300 000
      lines a common term cost 82 CPU ticks for three keys, now about 22

### 0.3 small features
- [x] *Hoher Kontrast* scheme automatically while GNOME's high contrast is on (switch in the
      preferences)
- [x] Selection in the accent color (`set_color_highlight`), light and dark
- [x] Imported color schemes sorted right away
- [x] Match count in the search (QA-19), counted with GRegex over the whole scrollback

### 0.2: Everyday features
- [x] Command line: `parkett -- command`, `--working-directory` (several times = tabs), `--tab`,
      `--profile`, `--title`
- [x] Save the session and restore it on startup (layout, directories, without processes)
- [x] Notification when a long command finishes in a pane that is not visible
- [x] Font, cursor shape, blinking and bell per profile
- [x] Transparent background (opacity per profile)
- [x] Nautilus context menu via nautilus-python
- [x] Sixel switch, only active with a VTE that has Sixel
