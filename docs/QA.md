# Phase 5 – Quality assurance

Status: 2026-10-19, version 0.1.0 (commit see CHANGELOG). Test environments:

| Environment | System | Graphics |
|---|---|---|
| **VM** fedora44-wor (GNOME Boxes) | Fedora 44, GNOME 50.5, GTK 4.22.5, libadwaita 1.9.4, VTE 0.84.1, 8 cores, 5.8 GB | software rendering (llvmpipe) |
| **Host** smokki | Arch, GNOME 51.1, GTK 4.24.1, libadwaita 1.10.0, VTE 0.84.1, Mesa 26.2.5 | real GPU |

Legend: Pass = run and passed · Finding = run, with a finding · Fixed = finding fixed and
retested · Not run = not executed. Tools: `build-aux/qa/` (throughput, measuring),
`build-aux/vm/` (deploy, screenshot, click), QMP keys, AT-SPI via Python, D-Bus actions
(`org.gtk.Actions` on the window, e.g. `new-tab-profile`, `split-right`, `close-tab`).

## Results following the list in the briefing

### 1. Starting and ending shell processes

| ID | Test | Result |
|---|---|---|
| P1 | `exit 3` → bar „Exit-Code 3“ (Exit code 3), „Neu starten“ (Restart) | Pass (phase 3/4) |
| P2 | Kill the shell from outside with `kill -9` | Pass: „Prozess beendet · Signal 9“ (Process exited · Signal 9) |
| P3 | Profile with a nonexistent command | Pass: „gibtesnicht123 konnte nicht gestartet werden: …“ (gibtesnicht123 could not be started: …) |
| P4 | Profile with an unterminated quote | Pass: „… Text endete, bevor ein passendes Anführungszeichen …“ (… text ended before a matching quote …) |
| P5 | Profile `false` + „Neu starten“ | Pass: no loop: „kein automatischer Neustart, endete sofort“ (no automatic restart, exited immediately) |
| P6 | Profile with environment variable and start directory | Pass: `QA=ok`, `/usr/share` |
| P7 | Quit Parkett (SIGTERM) with 3 shells (host) | Pass: no orphaned shells |
| P8 | Close a tab with a running program | Pass: confirmation; process ended once the Undo period expired |

### 2. Several terminals in parallel

| ID | Test | VM | Host |
|---|---|---|---|
| M1 | Memory, 1 terminal | 201 MB RSS | 72 MB RSS |
| M1 | 24 terminals (8 tabs × 3 panes) | 307 MB (≈ 4.6 MB per terminal) | 123 MB |
| M1 | Idle CPU with 24 terminals | 0.53 % | 0.60 % |
| M2 | 3 rounds: open and close 21 terminals | Fixed: RSS hovers around ≈ 300 MB, no growth across rounds; back to 1 shell after 10 s | Fixed: exactly 3 shells after 12 s |

**Finding M2 (fixed):** Undo toasts were queued (8 s each); 7 closed tabs
kept their processes alive for up to a minute (15 shells still there after 14 s). Now a
new toast replaces the previous one, and the previous one is closed for good immediately. Undo thus applies
to whatever was closed last.

### 3. Nested splits

| ID | Test | Result |
|---|---|---|
| S1 | Repeatedly split right/down (maximized, 1280×800) | Pass: minimum size 20×5 kicks in („Zu wenig Platz zum Teilen“ (Not enough space to split)), no overlap |
| S2 | 3 panes, numbers, swapping, focus in layout mode | Pass (phase 4) |
| S3 | Floating title in narrow panes | Fixed: it covered the prompt; below 380 px only the buttons remain now |
| S4 | Setting „Bereichstitel: Als Zeile“ (Pane title: As a row) | Fixed: had been decided but was missing; now implemented and switchable live |

### 4. Window resizing

| ID | Test | Result |
|---|---|---|
| W1 | Maximize → restore with 5 panes | Pass: panes shrink proportionally, minimum size is kept, no overflow |
| W2 | Ratio 40.9 % at 960 and 1920 px | Pass (phase 4) |
| W3 | Remember window size/maximized | Pass (phase 4) |

### 5. Copy and paste and keyboard shortcuts

| ID | Test | Result |
|---|---|---|
| C1 | Double-click a word → Ctrl+Shift+C → Ctrl+Shift+V | Pass |
| C2 | Shift+Insert (primary selection) | Pass |
| C3 | Paste multi-line text | Fixed: the warning was missing (feature matrix P0); now a dialog with preview, can be turned off |
| K1 | All default shortcuts | Pass: checked individually in phase 4; customizable, conflicts refused |

Note on C3 (as of 0.1.0, changed since: Shift+Insert warns as well, see
`qa/2026-10-19/QA-REPORT.md`): the warning applies to Ctrl+Shift+V and the context menu. Shift+Insert and
middle-click still paste without asking (VTE-internal); modern shells protect against this with
"bracketed paste".

### 6. Unicode and character sets

| ID | Test | Result |
|---|---|---|
| U1 | Umlauts, Cyrillic, Greek, wide CJK, simple emoji, box-drawing characters, truecolor | Pass (phase 3) |
| U2 | Arabic/Hebrew (RTL), Devanagari, Thai, Hangul, math, Braille | Pass |
| U2 | Emoji sequences with ZWJ (woman technologist, U+1F469 U+200D U+1F4BB), skin tones, flags | Finding: they fall apart into single characters or letters – **a VTE limitation**, Ptyxis shows the same |

### 7. Long-running processes

| ID | Test | Result |
|---|---|---|
| D1 | Throughput: `seq` 3 million lines / 20 MB of colored text, maximized | Pass: VM: Parkett 2.56–2.60 s / 1.17–1.28 s, Ptyxis 2.53–2.63 s / 1.17–1.24 s · Host: Parkett 2.50 s / 0.91 s, GNOME Console 2.79 s / 0.90 s |
| D2 | Endurance run 15 min (VM), 4 panes with ≈ 1000 lines/s each | Pass: RSS 283 → 296 MB, flat from minute 10 (295–296 MB: scrollback of 10,000 lines per pane filled), 41 threads, CPU 36–60 % of one core (software rendering) |

The first throughput run in the VM with different window sizes had shown Parkett 13–17 %
slower; maximized, both are on par – the window size was the cause.

### 8. Faulty or exited shell processes

See P2–P5. In addition: tabs with a process error get a warning icon (Pass).

### 9. High CPU and memory load

See M1, M2, D2. Under continuous output, rendering in the VM (software) costs 40–60 % of one
core; not measured on the host with GPU.

### 10. Wayland and X11

| ID | Test | Result |
|---|---|---|
| X1 | Wayland (VM and host), start like GNOME Shell | Pass |
| X2 | X11 via XWayland (`GDK_BACKEND=x11`): splitting, layout mode, palette | Pass: no GTK warnings |

### 11. Accessibility and GNOME integration

| ID | Test | Result |
|---|---|---|
| A1 | AT-SPI: buttons named (VM 9, host 23) | Fixed: the New button was called „Weitere Möglichkeiten“ (More options) (AdwSplitButton), the search button read out the shortcut too; now „Neuer Reiter“ (New tab), „Suchen“ (Search) |
| A2 | Terminal text readable via AT-SPI (basis for Orca) | Pass: VM; host: 3 terminals, 8 tabs in the tree |
| A3 | Orca with speech output | Not run: cannot be checked audibly |
| G1 | Accent color live | Pass: focus line follows |
| G2 | System monospace font live | Pass |
| G3 | High contrast | Finding: the UI follows (libadwaita), the terminal color scheme stays; proposal: then switch to „Hoher Kontrast“ (High Contrast) automatically |
| G4 | Light/dark, .desktop, icon in the dock | Pass (phase 3/4) |

## Open items from QA

- ~~High contrast: use the color scheme „Hoher Kontrast“ automatically (proposal G3).~~ Done
  2026-10-22 (setting *Hohen Kontrast des Systems übernehmen*).
- Middle-click (primary selection) without paste warning (VTE-internal); Shift+Insert warns since
  the QA audit of 2026-10-19.
- Emoji sequences: VTE limitation, cannot be solved in Parkett.
- Check Orca with speech output by hand.
- Measure CPU under continuous output on the host.
- Host test without screenshots: the deskwright extension did not run in the host shell;
  checks were done via processes, log, AT-SPI and D-Bus.

## Feedback from Christoph after his own test (r13, 2026-10-19)

| Request | Implementation | Test (VM) |
|---|---|---|
| Split buttons like in Tilix | „Rechts teilen“ (Split right) and „Unten teilen“ (Split down) on the left of the header bar next to „Neuer Reiter“; custom symbolic icons (`data/icons/hicolor/scalable/actions/parkett-split-*-symbolic.svg`, Adwaita has no suitable ones); tooltip with the current shortcut; screen reader names | Pass: both buttons split the active pane (`kopfleiste-teilen-stil.png`) |
| Light/dark by hand | GSettings `style` (system/light/dark) → `AdwStyleManager::set_color_scheme`; style selector with three round buttons at the top of the main menu (modeled on GNOME Text Editor) and „Stil“ (Style) in Preferences › General | Pass: „Dunkel“ (Dark) on a light system, „Hell“ (Light) on a dark system; terminals follow |

## Phase 6 – Code review (2026-10-19)

Complete reading of all source files by a review agent, Clippy (default and
pedantic). Findings and implementation; test: Pass = reproduced in the VM · Code review = code checked only ·
Unit test = unit test.

| ID | Finding | Implementation | Test |
|---|---|---|---|
| K1 | After *Rückgängig* (Undo) to a new place, the focus and close handlers kept the old pane id: clicking did not set the focus, closing hit the wrong pane | Handlers read the id at call time via a weak reference to the pane | Pass: click into the restored pane, *Bereich schließen* (Close pane) ends exactly its shell |
| K2 | An unknown id on closing counted as "last pane" and closed the tab (with the last tab, the window) | `close_pane` ignores ids that are not in the layout | Code review |
| K8 | *Rückgängig* did not return to the old place after a focus change or dragging a divider | Compare only the split layout (`Layout::same_shape`) | Unit test |
| P1 | Crash on start when all profile files are broken | Default profile only in memory, under a free id; the broken file stays untouched | Unit test, Pass |
| P2 | Crash on non-ASCII in color values (`#aäbcd`), with a broken imported palette on every start | `Rgb::parse` rejects non-ASCII | Unit test, Pass |
| G4 | Broken profile/palette/template files only in the log | Toast „N Datei(en) nicht lesbar“ (N file(s) not readable) with *Details* | Pass |
| S1 | The paste warning only counted `\n`; text with `\r` went through without asking | `\r`, `\n` and `\r\n` count as line breaks | Pass |
| S2 | `file:` links from SSH sessions opened local files with the same path | only without host, `localhost` or the own hostname | Code review |
| S3 | A template's subdirectory could be absolute or contain „..“; `~` in captured commands was expanded when typed in | `paths::join_subdir` only allows relative paths; `~` is quoted | Unit test / Code review |
| S4 | Half-typed quotes in environment variables saved "no variables" | on error do not save, row turns red | Code review |
| K3 | Layout mode stayed active on the old tab after switching tabs | reset all tabs when ending | Pass |
| K4 | Layout mode swallowed keys in dialogs (confirmation, palette) | only intercept keys when a terminal has focus | Pass: input lands in the palette; in the terminal *R* still splits |
| K5/K6 | Automatic restart and rebuilds in background tabs grabbed the focus | focus only on clicking *Neu starten* (Restart) or when the tab is visible | Code review |
| K7/D1 | `~` resolved differently in two places (`~foo` wrong) | shared module `core/src/paths.rs` | Unit test |
| D5 | Every change to templates recolored all terminals | separate signal `templates-changed` | Code review |
| G1 | Double-clicking the header bar also opened the command palette | palette only after the double-click time has passed | Code review |
| G2 | *Alt+1…9* / *Ctrl+Alt+1…9* hard-wired, but active while recording custom shortcuts and without conflict check | disabled while recording, reported as taken | Code review |
| P3, D4, D7, D8 | `expect` in the template editor, dead code, fragile `"Aus"`, PCRE comment | fixed | Code review |

Not implemented, in the BACKLOG: L1/L2 (strong references in preferences and drawing functions,
no leak proven), L3 (per-second timer also in background tabs), D2/D3
(merge foreground process query and color conversion), D6 (debug output),
G3 (command line `-- befehl`, `--working-directory`).

Smoke test afterwards (`build-aux/vm/smoke.sh`): 9 of 9 checks passed. CI (`build-aux/ci.sh`):
formatting, Clippy without warnings, 45 tests, Meson tests green.

## Everyday features for 0.2 (2026-10-19, VM)

| Feature | Test | Result |
|---|---|---|
| Command line | `--help`, `--version`; `--gibtsnicht` and `-d /gibtsnicht` → message, exit 1; `--tab -d /usr/share -t Logs` from a second instance; `cd /tmp && parkett`; `--tab -- sh -c …` | Pass: tab „Logs“ in `/usr/share`, new window in `/tmp`, command instead of shell |
| Unknown profile | Create a profile TOML while Parkett is running, then `--profile qa-cursor` | Fixed: was not found at first; now re-read, Pass |
| Font, cursor | Profile with `Monospace Bold 18`, `underline`, `off`, Nord | Pass |
| Bell „markieren“ (mark) | `sleep 3; tput bel`, switch tabs | Pass: tab marked |
| Session | 2 windows, 4 tabs, titles, directories `~`, `/usr/share`, `/tmp`; SIGTERM; restart | Pass: everything back; Fixed: a command pane without `vte.cwd` would have ended up in `/` → fallback to `/proc/<pid>/cwd` |
| Window order | last focused window on top | Fixed: opening in reverse order did not help (Mutter focuses the first window) → open in saved order, Pass |
| Last tab closed | split, immediately *Reiter schließen* (Close tab) | Fixed: the old state was kept → session is deleted, restart is fresh, Pass |
| Window with tabs closed | split, immediately *Alt+F4* | Pass: 2 panes on restart |
| Notification | `sleep 12` / `sleep 11`, switch tabs | Pass: tab marked, `AddNotification` „Befehl beendet – sleep · nach 10 s“ (Command finished – sleep · after 10 s) (D-Bus), banner visible; `app.show-terminal` switches to the pane. Fixed: duration is now rounded |
| Smoke test | `build-aux/vm/smoke.sh` with the new check "session restored" | Pass: 10/10 |

Not checked: bell with sound (cannot be checked audibly), launching via `xdg-terminal-exec`
(installed neither in the VM nor on the host), host (Arch).

## Transparency and Sixel (2026-10-19, VM)

| Feature | Test | Result |
|---|---|---|
| Transparency | Profile `opacity = 0.7`, window in front of another window | Pass: terminal area shows through; Fixed: the header bar was transparent too (libadwaita uses the window background) → own background for the top bar |
| Divider with transparency | Feedback from Christoph (host): divider as a light line over a light window behind | Fixed: `--border-color` is semi-transparent → opaque color from window foreground and background; VM light and dark: divider pixels identical over Nautilus and desktop, Pass |
| Transparency split | *Unten teilen* (Split down) in the glass window | Pass; dimmed panes look somewhat more opaque (dimming paints the background color over them) |
| Sixel available? | `Vte.get_feature_flags()` on host and VM | Host (Arch, `vte4` 0.84.1) and VM (Fedora 44, `vte291-gtk4` 0.84.1): 7 = BIDI, ICU, SYSTEMD, **without Sixel** |
| Sixel switch | AT-SPI in the preferences | Pass: grayed out, subtitle „Die installierte VTE-Bibliothek ist ohne Sixel gebaut“ (The installed VTE library is built without Sixel) |
| Sixel output | – | Not run: not executable, no VTE with Sixel available |
| Smoke test, CI | `build-aux/vm/smoke.sh`, `build-aux/ci.sh` | Pass: 10/10, green |

## Nautilus (2026-10-19, VM Fedora 44, Nautilus 50.4, nautilus-python 4.1.0)

| Test | Result |
|---|---|
| Menu of the folder `/usr/share` shows *Parkett hier öffnen* (Open Parkett here); click while Parkett is not running | Pass: one shell in `/usr/share`, no session added |
| two folders selected, right-click → *In Parkett öffnen* (Open in Parkett) | Pass: new window with tabs `abrt` and `accountsservice`, first one selected; Fixed: at first planned as a chain of `--tab` calls – under Mutter the new window does not reliably get focus, so now one call with several `-d` |
| Child processes of Nautilus afterwards | Pass: none (no zombie) |
| Syntax of the extension | Pass: Meson test `compile-nautilus-extension` |

Not checked: Nautilus 51 on the host (the extension comes with the package; Nautilus is running there
with open windows and was not restarted for the test).
