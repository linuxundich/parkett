# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added
- Short tab labels: number, running program and folder (`1 zsh · parkett`, `3 ssh bender`)
  instead of the shell title, which looked the same in every tab; the full title stays in the
  window title and the tooltip. Can be turned off (*Einstellungen › Allgemein › Kurze
  Reiter-Namen*).

### Changed
- Blinking text (SGR 5) only blinks in the focused pane, like GNOME Terminal; idle CPU in the VM
  was 7.8 % with blinking text in several panes.
- Panes in background tabs and inactive windows check their foreground process every 5 s instead
  of every second (context color, tab label), and immediately when they come into view.
- Tooltips show the current shortcut, also after changing it and in panes that are already open;
  screen readers get the name without the shortcut, the tab overview button says how many tabs
  are open.
- Opening an existing profile no longer selects its name, so a key press does not overwrite it.
- New profiles close the pane when its process ends with exit code 0 (`exit`, Ctrl+D), like GNOME
  Console, Ptyxis and Tilix. The bar „Prozess beendet“ only uses the warning icon and red tint for
  errors. Existing profiles keep their setting (*Wenn der Prozess endet*).
- Published on GitHub: README as a product page with screenshots, documentation translated to
  English (`docs/USAGE.md`, `ARCHITECTURE.md`, `PACKAGING.md`, `LICENSES.md`, design and QA
  documents), build instructions in `docs/BUILDING.md`.
- Profiles, templates and the session are written with mode 0600, regardless of the umask
  (environment variables in profiles may hold tokens).
- `build-aux/ci.sh` runs `cargo audit` when it is installed; unit tests keep the default shortcuts
  away from keys that programs in the terminal need (QA-01).
- The session is saved after dragging a divider, and no longer waits until a program stops
  changing its title.

### Fixed
- `file:` links are not opened while a remote connection runs in the pane, even without a host
  name (`file:///…`); before, an OSC 8 link from an SSH session opened a local file with the same
  path.
- `GIO_LAUNCHED_DESKTOP_FILE` and `GIO_LAUNCHED_DESKTOP_FILE_PID` no longer reach the shells.
- A template with `ratio = nan` (or `inf`) gets an even split instead of breaking the saved
  session.
- Undo after a pane closed itself on exit (profile *Bereich schließen*) brings it back with the
  exit bar (*Neu starten*, *Schließen*) instead of a dead terminal without a hint.
- `parkett -e "htop -d 5"`: a single argument after `-e` is split like a shell would (as in GNOME
  Terminal); flags given a value (`--tab=x`) are refused instead of silently accepted.
- Context chip for grouped ssh options: `ssh -vp 22 host` shows „host“ instead of „22“.

## [0.2.0] - 2026-10-19

Everyday features: session, command line, Nautilus, transparency, font/cursor/bell,
notifications; fixes from the QA audit on Ubuntu 26.04. First public release, source only;
see `docs/BUILDING.md`.

### Added
- Session: windows, tabs, split layout, profiles, directories and custom titles come
  back on startup (without programs); saved continuously, closing the last tab
  ends the session; can be turned off.
- Command line: `-d/--working-directory`, `-e/-x/--` (command instead of shell), `-p/--profile`,
  `-t/--title`, `--tab`, `--help`, `--version`; key for `xdg-terminal-exec` in the
  `.desktop` file. Unknown profiles are re-read from disk first.
- Nautilus: *In Parkett öffnen* (Open in Parkett; selected folders, several = tabs) and
  *Parkett hier öffnen* (Open Parkett here) via a nautilus-python extension
  (`share/nautilus-python/extensions`).
- Command line: `-d` given several times = one tab per directory.
- Profile: font, cursor shape, blinking, bell (sound and mark tab, mark only, off).
- Transparent background: opacity per profile (30–100 %), header bar and divider stay
  opaque (the divider was semi-transparent and showed the window behind it as a bright line).
- Sixel graphics (setting, on by default); only takes effect with a VTE built with Sixel
  support – Arch and Fedora 44 currently are not, in which case the switch is greyed out.
- Notification when a command (running 10 s or longer) finishes in a pane that is not visible;
  the tab is marked, clicking shows the pane. Requires `vte.sh`.

### Changed
- Pane title (design V2 from the QA audit): still floats at the top right, but is now
  hidden and no longer covers a line. It appears while the pointer is at the top of the
  pane, and for 1.5 s after a focus change, unless the cursor is in the top line.
  While hidden it does not accept clicks.
- Maximize button in the pane title: its own, smaller icon the size of the close button
  (`view-fullscreen` filled the whole 16 px grid and looked twice as large).
- *Reiter umbenennen* (Rename tab) no longer has a default shortcut (was F2); F10 no longer
  opens the main menu. Both keys belong to the programs in the terminal again (htop, mc, nano).
- Smoke test: also runs in the Ubuntu 26.04 VM (`VM_DOM`, `VM_PORT`), checks F10 in `htop`
  and profiles with *Neu starten* (Restart), and reliably detects GLib messages in the journal.

### Fixed (QA audit 2026-10-19, `qa/2026-10-19/QA-REPORT.md`)
- A closed tab with the profile setting *Neu starten* restarted its program invisibly after
  the undo period expired (for example an ssh connection); the expired toast kept the tab
  and its scrollback in memory.
- Paste warning: a single line with a trailing line break was pasted without asking
  and executed immediately in shells without bracketed paste.
- New terminals did not pick up directories containing spaces or umlauts (the session
  stored `%20`); a directory reported by an SSH session via OSC 7 was taken as the local
  starting directory. Now decoded and only accepted from the local machine, otherwise the
  shell's directory is used.
- Startup commands containing `&&`, `<` or `>` were missing in the *Vorlage öffnen*
  (Open template) dialog; names, paths and error messages in toasts, tooltips and lists are
  no longer parsed as markup.
- After the main menu, the new menu or the tab overview, focus no longer stays in the
  header bar; typed text goes to the terminal again.
- Color scheme "One" dark: the foreground was the comment color (contrast 2.7:1), now
  `#ABB2BF`; a test checks the contrast of all schemes.
- Environment variables in the profile: values with spaces or quotes are shown quoted
  and are no longer split up when editing.
- A tab that was closed or dragged into another window queried its old window page
  (`TabView::page` without a page).

## [0.1.0] - 2026-10-19

First version: MVP (phase 4), quality assurance (phase 5) and stabilization (phase 6).
Local only; package in the own Arch repository `linuxundich` on bender.

### Phase 6
- Documentation: usage (`docs/USAGE.md`, the example profiles and templates are loaded
  by `cargo test`), architecture, packaging options, licenses; README revised.
- `build-aux/ci.sh`: formatting, Clippy, tests, Meson build and Meson tests in one run.
- `build-aux/vm/smoke.sh`: smoke test in the VM over D-Bus (splitting, tabs, closing,
  undo period expiry, no orphaned shells, log without errors).
- Development plan for 0.2–0.4 in the BACKLOG.

### Fixed (phase 6, code review)
- After *Rückgängig* (Undo) at a new position, focus and close hit the wrong pane.
- Closing a pane that was already closed closed the whole tab or the window.
- Crash on startup when there were only broken profile files or non-ASCII color values in palettes;
  unreadable files are now reported in a toast with details.
- The paste warning also detects line breaks with `\r`.
- `file:` links to other machines (OSC 8 from SSH sessions) are no longer opened.
- Subdirectories in templates must now be relative and must not contain "..".
- Layout mode: no longer stays on the old tab and no longer swallows keys
  in dialogs and the command palette.
- Automatic restart and background tabs no longer grab focus.
- Double-clicking the header bar no longer also opens the command palette.
- Alt+1…9 / Ctrl+Alt+1…9 are reported as taken when recording custom shortcuts.
- Half-typed environment variables no longer delete the saved ones.
- Changes to templates no longer recolor the terminals.

### Added
- *Rechts teilen* (Split right) and *Unten teilen* (Split down) buttons in the header bar
  (as in Tilix), with their own icons.
- Manual light/dark: style choice *System*/*Hell*/*Dunkel* (System/Light/Dark) in the main menu
  and in the preferences.
- Setting *Bereichstitel bei geteilten Reitern: Als Zeile* (Pane title in split tabs: as a row;
  a title row above each pane).
- Warning when pasting multi-line text, with preview (can be turned off).

### Fixed (phase 5, QA)
- Undo toasts queued up and kept closed processes alive for up to a minute;
  a new toast now replaces the previous one.
- The floating pane title covered the prompt in narrow panes; there it now shows only buttons.
- Screen reader names: the new button was called *Weitere Möglichkeiten* (More options),
  the search button read out its shortcut too.

### Added (steps 1–7)
- Context color per pane: panes with an SSH connection or root privileges are tinted and
  marked with a line and a chip (host name or "root"); colors come from the color scheme,
  can be turned off.
- Undo: closed panes and tabs can be brought back for 8 s; their programs keep running
  during that time.
- Command palette (Ctrl+Shift+P or click on the window title) for tabs, panes,
  actions, templates and profiles; prefixes >, @, #.
- Layout mode (Ctrl+Shift+L): numbered panes, single keys to split, swap,
  maximize, equalize and close.
- Templates: save the current tab with its split layout, profiles and running commands,
  open it in a chosen directory (startup commands can be turned off), edit in the
  preferences.
- Configurable keyboard shortcuts (*Einstellungen › Tastenkürzel*, Preferences › Keyboard
  Shortcuts), with conflict check and *Alle zurücksetzen* (Reset all); windows, terminals and
  the overview apply changes immediately.
- Search in the terminal per pane (Ctrl+Shift+F; match case, regular expression; matches
  above/below), search button in the header bar.
- Links: Ctrl+click opens addresses and OSC 8 links (only http, https, ftp, mailto, file),
  context menu *Link öffnen/kopieren* (Open/copy link), tooltip with the real target of an OSC 8 link.
- Profiles (TOML under ~/.config/parkett/profiles): color scheme, command instead of shell,
  login shell, starting directory, environment variables, behavior when the process exits.
- 14 color schemes with light and dark variants (from Ptyxis), switching live with the system style.
- Import of Ptyxis palettes and Tilix color schemes.
- Preferences dialog (Ctrl+,) with the pages *Allgemein* (General) and *Profile* (Profiles), color
  schemes as tiles with live preview; *Neuer Reiter mit Profil* (New tab with profile) in the menu
  on the new button.
- Meson build with .desktop file, metainfo, icons and GSettings schema; license text GPL-3.0.
- Settings via GSettings: remember window size, confirm on close, dimming,
  hide the tab bar, starting directory of new terminals, scrollback length.
- Arch package in the own repository `linuxundich`.

### Changed
- The interface says *Reiter* instead of *Tab* (as libadwaita does).

### Added (tabs and panes)
- Tabs with AdwTabView: tab bar (hidden when there is only one tab), tab overview with search,
  rename tab (F2), warning icon when the process exits with an error.
- Split panes: split right/down/automatically, focus and size via keyboard,
  equalize, maximize with a hint, minimum size 20×5 cells, floating title,
  focus line and dimming of inactive panes.
- Confirmation when closing a pane, tab or window while a program is still running.
- Keyboard shortcuts overview (Ctrl+?).
- 0.1.0 prototype (phase 3): window with libadwaita and VTE, asynchronous shell start,
  *Prozess beendet* (Process exited) bar with exit code, *Neu starten* and *Schließen* (Close).
- Title and working directory in the header bar from VTE termprops (`xterm.title`, `vte.cwd`).
- Terminal colors light/dark following the system, font following the system monospace font.
- Copy, paste, select all, zoom via shortcut and context menu; new window.
- `parkett-core`: split layout as an immutable tree with tests.
- .desktop file and icons under the app ID `de.linuxundich.Parkett`.
