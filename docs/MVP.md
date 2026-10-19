# Phase 4 – Usable MVP

Approved on 2026-10-18 together with the architecture proposals from `PROTOTYPE.md` (subclasses,
Blueprint for dialogs, GSettings + files, serde, Meson, GPL-3.0-or-later, check the Ptyxis palette).

## Steps

| # | Step | Status |
|---|---|---|
| 1 | **Tabs and splits** (this document, section below) | done, tested in VM |
| 2 | Meson build, GSettings schema, LICENSE, Arch package | done, tested in VM |
| 3 | Profiles and color schemes (Ptyxis format, import, tiles with live preview) | done, tested in VM |
| 4 | Preferences (AdwPreferencesDialog): General, Profiles, Keyboard Shortcuts | done |
| 5 | Search in the terminal, opening links (Ctrl+click), customizable keyboard shortcuts | done, tested in VM |
| 6 | Command palette, layout mode, templates with startup commands (A+) | done, tested in VM |
| 7 | Undo for closing, context color per pane (proposals 1, 4) | done, tested in VM |

## Step 1: Tabs and splits

### Implemented

- **Subclasses** `ParkettWindow` (AdwApplicationWindow), `ParkettTab` and `ParkettPane`
  (AdwBin). The tab holds the layout model from `parkett-core` and builds the GtkPaned tree from it;
  when the layout is rebuilt, panes are only re-parented, terminals keep running.
- **Tabs:** AdwTabView, AdwTabBar (hidden with a single tab), AdwTabOverview with search
  (title and directory), AdwTabButton with count, split button „Neuer Tab“ (New tab) with menu.
  New tabs and new panes take over the directory of the focused pane (OSC 7).
  Rename tab (F2, menu); an empty name means the terminal's title.
- **Splits:** right, down, automatic (longer side); focus by direction; pane 1–9;
  resizing by keyboard (3 % per step) and mouse; equalize; maximize with the notice
  „Maximiert · n Bereiche ausgeblendet · Alle zeigen“ (Maximized · n panes hidden · Show all) and „(1/n)“ in the title (proposal 3).
  Minimum size 20×5 cells; splitting below that is refused with a toast.
- **Appearance (A+):** floating title per pane only in split tabs, focus line in
  accent color, inactive panes dimmed.
- **Context menu** extended with split, maximize, equalize, close; right-clicking
  focuses the pane first.
- **Close with confirmation** when a foreground process is running (tcgetpgrp like Ptyxis) –
  for pane, tab and window. The last pane closes the tab, the last tab closes the window.
- **Process exit:** the bar stays; the tab gets a warning icon and "needs attention" when
  a pane exits with an error and the tab is in the background.
- **Keyboard shortcuts overview** (AdwShortcutsDialog, Ctrl+?, menu). The minimum libadwaita version
  is now 1.8 because of this.

### Shortcuts

| Action | Shortcut |
|---|---|
| New tab / close tab | Ctrl+Shift+T / Ctrl+Shift+Q |
| Switch tab | Ctrl+Page Down / Ctrl+Page Up, Ctrl+Alt+1…9 |
| Rename tab / overview | F2 / Ctrl+Shift+O |
| Split right / down / automatically | Ctrl+Alt+R / D / A |
| Close / maximize / equalize pane | Ctrl+Shift+W / Ctrl+Shift+X / Ctrl+Alt+= |
| Focus / size | Alt+Arrow / Shift+Alt+Arrow, pane n: Alt+1…9 |

Window and terminal shortcuts are attached as `GtkShortcutController` (capture phase) to the window
or terminal respectively, not as app accels (see PROTOTYPE.md, bug found).

### Tests in the VM fedora44-wor (GNOME 50.5, Wayland, 1920×1033)

| Test | Result |
|---|---|
| `cargo test` (13 core tests incl. ratio by path, unzoom, 1 app test), `cargo clippy` | green, no findings |
| Split right, then split down: 50 % / 50 % | ok **after fix** (35 % at first) – see findings 1, 2 |
| Alt+Left focuses the left pane; Shift+Alt+Right ×3 → 59 % | ok (`mvp-splits.png`) |
| Ctrl+Shift+X: maximizes with notice and „(1/3)“, input arrives; again → back | ok (`mvp-maximiert.png`) |
| Ctrl+Alt+= equalizes | ok |
| Window width 960 → maximized 1920 → back: ratio stays 40.9 % | ok (log `PARKETT_DEBUG_LAYOUT`) |
| New tab, tab bar appears, Ctrl+Page Up switches | ok |
| F2 → „Server“ → Enter renames the tab | ok **after fix** (finding 3) |
| Tab overview with previews of both tabs | ok (`mvp-uebersicht.png`) |
| `sleep 300`, Ctrl+Shift+W → confirmation names „sleep“; confirming ends shell and sleep | ok **after fix** (finding 4) (`mvp-nachfrage.png`) |
| Close tab with `sleep`: Esc cancels, confirming ends everything | ok **after fix** (finding 5) |
| Close window with `sleep` running → confirmation | ok |
| Right-click in the inactive pane: focus changes, menu with shortcuts; „Unten teilen“ (Split down) acts there | ok |
| `exit` → bar, Ctrl+Shift+W closes the exited pane without confirmation | ok |

### Found and fixed

1. Divider sync calculated with `max-position` instead of the total length (GTK subtracts the
   minimum size of the second child there) → left half only 35 %.
2. Collapsed GtkRevealer (bar „Prozess beendet“ (Process exited)) still requested ~300 px width →
   panes could not get narrower. Now invisible while collapsed.
3. AdwAlertDialog focuses the default response; the rename entry received
   no keyboard input → the entry is now focused.
4. **Processes kept running after closing:** closures held strong references to the VTE widget
   (among others on the global StyleManager), so it was never freed. Now weak references,
   disconnecting on destroy, plus an explicit SIGHUP to the shell.
5. AdwAlertDialog emits `closed` before `response` → the confirmation arrived too late, the tab stayed
   open. Now only `response` is evaluated.

### Open / observed

- After closing the tab overview, the cursor was invisible until the next input (four
  captures 0.35 s apart). Cause still unclear (VTE blink logic after focus change?).
- The selection color in dark mode is the VTE default.
- **Terminology:** decided on 2026-10-18 – consistently „Reiter“ (tab) like libadwaita (UI,
  shortcuts overview, schema texts). Code and older docs keep saying "Tab".
- Not tested: dragging tabs into a new window, dragging dividers with the mouse (only checked via
  keyboard and window size), host start, Orca.
- Drag and drop of panes (P1) and Undo (proposal 4) come later.

## Step 2: Meson, GSettings, LICENSE, Arch package

### Implemented

- **Term „Reiter“** in all UI texts (Christoph's decision).
- **Meson** as in Kulisse: `meson.build` calls `build-aux/cargo-build.sh` (sets
  `PARKETT_INSTALLED`), installs the program, `.desktop` (Exec with full path from `bindir`),
  metainfo, GSettings schema and icons; `gnome.post_install` compiles the schema and caches.
  `meson test`: desktop-file-validate, appstreamcli, glib-compile-schemas --strict, cargo test.
- **`cargo run`** still works: `app/build.rs` compiles the schema to `$OUT_DIR/schemas`,
  `settings::init()` then sets `GSETTINGS_SCHEMA_DIR` (only without `PARKETT_INSTALLED`).
- **GSettings** `de.linuxundich.Parkett` with keys that already take effect:
  `window-width/-height/-maximized`, `confirm-close`, `dim-unfocused` (live),
  `autohide-tab-bar` (live, via binding), `new-terminal-directory` (current/home),
  `scrollback-lines` (live, -1 = unlimited). Profiles and templates come as files.
- **LICENSE** GPL-3.0 (full text).
- **VM script** `build-aux/vm/deploy.sh` now builds and installs with Meson into `~/.local`.
- **Arch package** `parkett` in the pacman repo (`services/pacman-repo/pkgbuilds/parkett`).

### Tests (VM fedora44-wor)

| Test | Result |
|---|---|
| `meson setup/compile/test/install` locally (prefix `_install`) | 4/4 tests ok, all files in the right place |
| `meson install` into `~/.local` in the VM, start | ok |
| `gsettings list-recursively de.linuxundich.Parkett` in the VM | all 8 keys with default values |
| `dim-unfocused false` / `autohide-tab-bar false` at runtime | take effect immediately (no dimming, tab bar visible with a single tab) |
| Maximize window, close the last tab → `window-maximized true`, restart maximized | ok **after fix**: closing via the last tab did not save at first |

Not tested: `new-terminal-directory home`, `scrollback-lines` at runtime, saved
window width after resizing with the mouse.

## Step 3: Profiles and color schemes

Christoph's decision (2026-10-18): Ptyxis palette format with light/dark pairs and
root/SSH colors, a curated selection of 10–15, import of Ptyxis and Tilix schemes,
selection as tiles with live preview.

### Implemented

- **Core `palette`:** parser for Ptyxis palettes (`[Palette]`, `[Light]`, `[Dark]`; older
  files with colors only in `[Palette]` apply to both variants), derivation of missing
  root/SSH backgrounds following Ptyxis's rules, import of Tilix JSON, writing back in
  Ptyxis format. 9 tests, among them: all built-in schemes have a real light and dark variant,
  the round trip read → write → read is lossless.
- **14 built-in schemes** (`data/palettes`, origin in `data/palettes/README.md`): GNOME,
  Hoher Kontrast (High Contrast), Tango, Solarized, Selenized, Nord, Dracula, Catppuccin, Gruvbox, Everforest,
  One, GitHub, Papercolor, VS Code – all from Ptyxis (GPL-3.0-or-later), colors unchanged.
- **Core `profile`:** profile as a TOML file (`~/.config/parkett/profiles/<id>.toml`) with name,
  color scheme, command instead of shell, login shell, start directory, environment variables, behavior
  on process exit (bar / close on success / restart). Unknown fields do no harm,
  broken files are reported instead of deleted. 3 tests.
- **App:** colors per pane from the palette of its profile, variant according to the system style, live on
  changes of style, scheme or profile. Dimming in the scheme's background color.
  New panes inherit the profile of the active pane; „Neuer Reiter mit Profil“ (New tab with profile) in the menu of the
  New button (default profile first). Restart guard: if a process exits within 2 s,
  it is not restarted automatically. Tabs without an OSC title are named after the profile's command.
- **Preferences** (Ctrl+, and menu): AdwPreferencesDialog with „Allgemein“ (General) (all
  GSettings keys) and „Profile“ (Profiles) (list, new profile, editor as subpage). Color schemes
  as tiles (light on the left, dark on the right), selection applies immediately; „Importieren …“ (Import …) with file filter.
  New GSettings key `default-profile`.

### Tests (VM fedora44-wor)

| Test | Result |
|---|---|
| `cargo test`: 24 core tests + 1 app test, `cargo clippy` | green, no findings |
| First start creates `standard.toml` with scheme „gnome“ | ok |
| Preferences: page „Allgemein“, page „Profile“, editor with tiles | ok (`mvp-farbschemata.png`) |
| Click tile „Tango“ → both running panes Tango immediately, profile saved | ok |
| Dark mode → dark Tango variant, dimming matches | ok (`mvp-tango-hell-dunkel.png`) |
| Import Tilix `monokai.json` → `monokai-dark.palette`, notice „nur eine Fassung“ (only one variant), profile uses it | ok (`mvp-import.png`) – tile selection followed **only after fix** |
| Import Ptyxis palette with only `[Palette]` (Afterglow) → tile selected | ok |
| „+“ creates a new profile, renaming via the entry is saved | ok |
| Profile with command `sleep 3` + „Neu starten“ (Restart): new process every 3 s | ok |
| „Bereich schließen (bei Erfolg)“ (Close pane (on success)): tab closes after `sleep 3` ends, no processes left | ok |
| Menu „Neuer Reiter mit Profil“ opens a tab with the chosen profile, title „sleep 3“ | ok (**after fix**: at first „Standard (Standard)“ and „Shell“) |

Not tested: environment variables, fixed start directory, login shell, deleting a profile,
error message for a broken import file, restart guard (< 2 s). Command and behavior on exit
were set in the TOML file for the test, not via the entry fields.

### Open / observed

- A profile's id is derived from its first name („neues-profil“) and stays the same when
  renaming – invisible in the UI, but visible in file names.
- Imported schemes appear at the end instead of alphabetically until the next start.
- Context color per pane (root/SSH) is present in the palettes but only applied with
  step 7.

## Step 5: Search, links, keyboard shortcuts

### Implemented

- **Search per pane** (Ctrl+Shift+F, button in the header bar, context menu): bar at the top
  right of the pane with search entry, „Aa“ (match case), „.*“ (regular expression), match
  above/below, close. Searches from the end (newest output first); Enter = match
  above, Shift+Enter = below, Ctrl+Shift+G/H also without an open bar. No
  match or invalid expression: entry turns red, error as tooltip. Selected text becomes the
  search term. VTE search with PCRE2 (flags like Ptyxis), text is escaped without „.*“.
- **Links:** addresses (http, https, ftp, file, mailto) are detected by regex, trailing punctuation
  is not part of them; OSC 8 links come from VTE. **Ctrl+click** opens via `GtkUriLauncher`, only with an
  allowed scheme (no javascript:, data: …). Context menu „Link öffnen“ (Open link) / „Link kopieren“ (Copy link),
  only visible when a link is under the pointer. **OSC 8 links show their real target as a
  tooltip** (the visible text can be misleading); vte4-rs does not bind the signal, hence
  `connect_local`.
- **Customizable keyboard shortcuts:** central list `shortcuts.rs` (32 actions with default bindings),
  custom bindings in GSettings `custom-shortcuts` (`a{ss}`). Page „Tastenkürzel“ (Keyboard shortcuts) in the
  preferences: click a row, press the combination; Esc cancels, Backspace disables,
  „Standard wiederherstellen“ (Restore default), „Alle zurücksetzen“ (Reset all). Conflicts are refused with a reference to
  the action already bound; without Ctrl/Alt/Super only function keys are allowed. During recording,
  window and app shortcuts are disabled. Windows, terminals (incl. the context menu display),
  app accels and the shortcuts overview follow changes immediately.

### Tests (VM fedora44-wor)

| Test | Result |
|---|---|
| Search „apfel“: first finds „APFEL“ (bottom), Enter → „apfel“ above; „apfelbanane“ → red | ok (`mvp-suche.png`) |
| Ctrl+click on `https://linuxundich.de/artikel/test,` opens Firefox with the address without the comma | ok |
| Pointer over OSC 8 text „Hier klicken“ (Click here) → tooltip `https://example.org/wahres-ziel` | ok (`mvp-osc8-tooltip.png`) |
| Right-click on `mailto:…` → „Link öffnen/kopieren“; copy → `wl-paste` = address | ok (`mvp-link-menue.png`) |
| Right-click on `javascript:alert(1)` → no link entries | ok |
| Shortcuts page lists all actions | ok (`mvp-kuerzel-seite.png`) |
| „Neuer Reiter“ (New tab) → Ctrl+Shift+X → notice „schon Bereich maximieren“ (already Maximize pane), nothing triggered | ok (`mvp-kuerzel-konflikt.png`) |
| Ctrl+Alt+E recorded → GSettings `{'win.new-tab': '<Control><Alt>e'}`; Ctrl+Shift+T has no effect, Ctrl+Alt+E opens a tab (live) | ok |
| After recording, Ctrl+, works again; row shows „Eigene Belegung“ (Custom binding); „Alle zurücksetzen“ → `{}` | ok |
| Shortcuts overview (Ctrl+?) shows the current bindings, no GTK warnings | ok |

### Found and fixed

- **Crash on start:** `glib::VariantDict` requires `a{sv}`, the key is `a{ss}` →
  reading and writing as `HashMap<String, String>`.
- Invalid shortcut texts in the overview („Strg+Klick“, „1…9“) replaced.

Not tested: regex and match-case toggles of the search, changing a terminal shortcut
(same mechanism as window shortcuts), `file://` links.

## Step 6: Command palette, layout mode, templates

### Implemented

- **Core:** `fuzzy` (all words must occur, word starts and text start count more;
  prefixes `>` actions, `@` tabs/panes, `#` templates), `Layout::swap` and
  `Layout::from_node`, `template` (template as a tree with profile, startup command, subdirectory per
  pane; TOML under `~/.config/parkett/templates/`). 35 core tests.
- **Command palette** (Ctrl+Shift+P or **click on the window title**, proposal 2):
  AdwDialog with search entry and list in the groups tabs/panes, templates, actions (all
  shortcut actions with their shortcut, „Neuer Reiter mit Profil …“, „Als Vorlage speichern“ (Save as template)).
  Arrow keys move the selection, Enter runs, Esc closes.
- **Layout mode** (Ctrl+Shift+L): large numbers on the panes (focused one in
  accent color), help bar at the bottom (wraps in narrow windows), notice in the window title.
  Arrows focus, Shift+Arrow size, R/D/A split, M+Arrow swap, Z maximize,
  = equalize, X close (with confirmation), 1–9 jump, Esc/Enter ends; other keys
  are swallowed. Switching tabs ends the mode.
- **Templates:** „Aktuellen Reiter als Vorlage speichern …“ (Save current tab as template …) (New menu, palette) takes over
  the split layout, ratios, profiles, running foreground commands (from `/proc/<pgid>/cmdline`,
  only necessary quotes) and subdirectories relative to the first pane. „Vorlage
  öffnen“ (Open template) asks for the directory (folder chooser), shows all startup commands and lets you
  disable them (proposal 6); startup commands are typed into the shell after start
  (proposal 5). Templates appear in the New menu and in the palette. Preferences page
  „Vorlagen“ (Templates) with editor: name, fixed directory, preview of the split layout, per pane name,
  profile, startup command, subdirectory; delete.

### Tests (VM fedora44-wor)

| Test | Result |
|---|---|
| `cargo test` (35 core + 1 app), `cargo clippy` | green, no findings |
| Palette „teil“ → three split actions with shortcuts; arrow down + Enter → „Unten teilen“ | ok (`mvp-palette.png`) |
| Click on the window title opens the palette; „@“ → tab and three panes | ok |
| Layout mode: numbers, title notice; M+Left swaps `sleep 600` to the left; „1“ focuses; Esc → input in pane 1 | ok |
| Help bar complete in a 960 px window | ok **after fix** (cut off at first) (`mvp-layout-modus.png`) |
| Save as template (palette): TOML with split layout, ratios, `command = "sleep 600"` | ok |
| Open template from the New menu: dialog with directory and startup command, new tab with the same split layout, `sleep 600` runs in the third pane | ok (`mvp-vorlage-oeffnen.png`) |
| Directory without subdirectory | ok **after fix** (at first `~/` instead of `~`) |
| Preferences › Templates: list and editor with preview | ok (`mvp-vorlage-editor.png`) |

Not tested: folder chooser in the open dialog, editing via the editor (saving while
typing), prefix `#`, R/D/A/Z/=/X in layout mode (same functions as the shortcuts).
The preferences page „Tastenkürzel“ is now called „Kürzel“ (Shortcuts) (it no longer fit with four pages).

### Open / observed

- The split layout of a template cannot be changed in the editor; it comes from the tab.
- Ctrl+Enter in the palette („im neuen Bereich“ (in a new pane), mockup) is not implemented.

## Step 7: Undo and context color

### Implemented

- **Context color per pane** (proposal 1): each pane queries its foreground process once per second
  (`tcgetpgrp` + `/proc/<pgid>/{comm,cmdline,status}`). ssh, mosh,
  autossh, et, telnet → **remote** (hostname from the command line, ssh options with a value
  are skipped, `user@host` → `host`); effective UID 0 or sudo/su/doas/run0/pkexec →
  **root** (also covers the root shell after `sudo -i`). Display: background tinted 35 % towards
  `RemoteBackground`/`SuperuserBackground` of the color scheme, 3 px line at the top in
  the scheme's blue or red, chip with hostname or „root“ in the floating title (then also visible with
  only one pane). Can be turned off: Preferences › General › „Kontextfarbe für root
  und SSH“ (Context color for root and SSH) (GSettings `context-colors`). Detection in the core (`context.rs`, 2 tests).
- **Undo** (proposal 4): closed panes and tabs are kept for 8 s including their processes;
  toast „Bereich geschlossen“ (Pane closed) or „Reiter „…“ geschlossen“ (Tab "…" closed) with „Rückgängig“ (Undo).
  Pane: back to its old place if the layout has not changed since, otherwise
  next to the active pane. Tab: back to its old position. When the time runs out, the shell
  is terminated (SIGHUP). The confirmation for running programs still comes first. With the last
  tab, the window closes as before (without Undo).

### Tests (VM fedora44-wor)

| Test | Result |
|---|---|
| `cargo test` (37 core + 1 app), `cargo clippy` | green, no findings |
| `ssh localhost` (waiting for host key) → pane tinted blue, line, chip „localhost“ | ok |
| `sudo -i` → pane tinted red, line, chip „root“ | ok (`mvp-kontextfarbe-hell.png`) |
| Dark mode: context colors from the dark variant | ok (`mvp-kontextfarbe-dunkel.png`) |
| Close pane → toast, shell keeps running; „Rückgängig“ → pane back | ok |
| Let the toast expire → shell of the closed pane terminated | ok |
| Close tab with `sleep 600` (confirmation accepted) → toast, `sleep` runs; „Rückgängig“ → tab back with `sleep` running | ok (`mvp-rueckgaengig.png`) |
| Close tab and let it expire → `sleep` and shell terminated | ok |

Observed: libadwaita pauses the timeout while the pointer is on the toast
(intended behavior of AdwToast); only afterwards is it closed for good.

Not tested: toggling „Kontextfarbe“ (Context color) at runtime, mosh/telnet, restoring a
pane after an intermediate layout change (place next to the active pane).

## MVP status

All seven steps of phase 4 are implemented and checked in the VM. Next phase according to the
briefing: **Phase 5 – Quality assurance** (systematic tests: processes, many terminals,
Unicode, long-running jobs, load, Wayland/X11, accessibility, host test on Arch).
