# QA audit Parkett – 2026-10-19

Status: commit `0faa590` (0.1.0 plus `[Unreleased]`: session, command line, transparency, Nautilus).
Checked on 2026-10-19, 08:48–09:25, in the VM `ubuntu26.04`. In addition, a review agent read
the complete source code (read-only).

**Nothing was changed in the source code, in the project configuration or in Christoph's data.**
Test data lived only with the VM user `testt` and has been removed again (see J).

Evidence is in `bilder/`, the design variants in [`designvarianten.html`](designvarianten.html).

Labels for findings:

- **reproduced:** triggered in the VM and documented
- **confirmed in code:** the cause is unambiguous in the source code, but was not triggered in the VM
- **suspected:** plausible, not yet confirmed

Preliminary remark: the assignment calls Parkett "the terminal application of the GNOME project". Parkett is,
however, Christoph's own project (app ID `de.linuxundich.Parkett`), not a GNOME project. The
GNOME HIG serve as a yardstick here, not as requirements of an upstream.

---

## A. Summary

For a 0.1, Parkett is remarkably robust:

- It builds on Ubuntu 26.04 without any adjustment. Rust 1.93, GTK 4.22.4, libadwaita 1.9.1 and VTE 0.84.0 meet the minimum versions.
- It starts in about 150 ms up to a running shell.
- It keeps up with Ptyxis in throughput.
- After 100 terminals opened and closed, it leaves no processes behind and has no memory leak.
- It reports broken files in an understandable way.

Shell, job control, colors, Unicode, search, links, session and confirmations on closing
work.

The biggest problems are at the boundaries between Parkett and the programs running inside it:

1. **Parkett takes keys away from programs** (QA-01, high). *F2* opens „Reiter umbenennen“ (Rename tab), *F10*
   the main menu – even when `htop`, `mc` or `nano` need these keys. In `htop`, this means
   neither *Setup* nor *Quit* works.
2. **The floating pane title covers the first line** (QA-02, high). In split tabs,
   the prompt, input and cursor of a fresh pane lie beneath the title. This is the
   default and affects every Ubuntu installation with a long prompt.
3. **A closed tab with the profile setting „Neu starten“ (Restart) restarts its program invisibly**
   (QA-03, high). With `ssh server`, the connection is re-established in the background. This only
   stops when you close something else.

On top of that come five medium security and functional bugs:

- The paste warning can be bypassed (QA-04).
- Directories with spaces or umlauts get lost when splitting (QA-05).
- A remote machine can set the start directory of local terminals via OSC 7 (QA-06).
- Startup commands of a template are invisible if they contain `&&` or `<` (QA-07).
- A second invocation of `parkett` loses its environment variables (QA-08).

Biggest potential for improvement in the design: **orientation in split tabs.**
Window title, tab and pane title show the same long text three times. As a result, the tabs
cannot be told apart. Three variants for this are in H.

---

## B. Inventory

### Test environment

| Item | Value |
|---|---|
| Host | Arch Linux, libvirt session VM, at times displayed in GNOME Boxes |
| Guest | Ubuntu 26.04.1 LTS, kernel 7.0.0-38, GNOME Shell 50.1, Wayland, German (de_DE.UTF-8), German keyboard |
| Libraries | GTK 4.22.4, libadwaita 1.9.1, VTE 0.84.0 (built without Sixel), Orca 50.2 present |
| Build | rustc/cargo 1.93.1, meson 1.10.1 (installed for the test), `meson install` into `~/.local`, release build 1 min 44 s |
| Start | like GNOME Shell: `systemd-run --user -u app-gnome-de.linuxundich.Parkett-…` |
| Graphics | **software rendering** (llvmpipe; `ZINK: failed to choose pdev` in the log), 8 vCPU, 5.3 GB RAM |
| Resolution | first 1280×800, from 09:08 on 1920×1033 (Boxes window opened) |
| Comparison | Ptyxis 50.1 (Ubuntu's default terminal) under the same conditions |
| Control | keys and mouse via QMP, D-Bus actions (`org.gtk.Actions`), AT-SPI via Python, SSH for measurements |

Deviations from the target platform to keep in mind while reading:

- **No GPU rendering:** CPU values for rendering are higher than on real hardware.
- **No German GTK 4 translations** (`gtk40.mo` missing, language pack not installed):
  key names from GTK appear in English ("Shift+Ctrl+T"). This is a gap in the VM,
  not a bug in Parkett. It does, however, make QA-12 visible.
- **Primary selection disabled:** Ubuntu sets `gtk-enable-primary-paste` to false. Pasting
  with the middle mouse button can therefore not be checked here.

### Architecture (short version)

| Part | Content |
|---|---|
| `core/` (without GTK) | split tree `layout`, profiles (TOML), palettes (Ptyxis format), templates, session (JSON), command line, root/SSH context, fuzzy search, paths – 53 unit tests, 2 doc tests |
| `app/` (GTK) | `window.rs` (AdwTabView/TabBar, actions, Undo), `tab.rs` (GtkPaned tree), `pane.rs` (overlay: title, focus line, dimming, search), `terminal.rs` (VTE, spawn, process exit, links, paste, notification), `prefs.rs` (AdwPreferencesDialog), `commands.rs` (palette), `shortcuts.rs`, `session.rs`, `profiles.rs` |
| Data | 17 GSettings keys, `~/.config/parkett/{profiles,templates}`, `~/.local/share/parkett/palettes`, `~/.local/state/parkett/session.json` |
| Processes | VTE `spawn_async` per pane, 1 s timer per terminal (`tcgetpgrp` + `/proc`), SIGHUP on closing, 8 s Undo period |
| Integration | .desktop with xdg-terminal-exec key, metainfo, icons, nautilus-python extension, D-Bus activation via GApplication |

### Features by area

| Area | Present |
|---|---|
| Functionality | tabs, splits right/down/automatic, maximize, equalize, focus by arrow and number, layout mode, command palette, templates with startup commands, profiles (command, login shell, directory, environment, process exit action), 14 color schemes light/dark, import Ptyxis/Tilix, search (regex, match case), links (regex, OSC 8), Undo, session, command line, notification when a command finishes, Nautilus menu |
| Usability | customizable shortcuts with conflict check, shortcuts overview, confirmation on closing, paste warning, error bar with *Neu starten* (Restart), toast for broken files |
| Design | libadwaita widgets, light/dark/system, accent color, root/SSH context color, dimming, opacity, font/cursor per profile |
| Engineering | core without GTK and testable, CI script (fmt, clippy -D warnings, tests), VM smoke test, atomic saving of profiles/templates/session |

Known limitations and technical debt (from BACKLOG and review):

- Sixel untested.
- L1–L3, D2, D3, D6 from the phase 6 review are open.
- Large functions: `Pane::new` ≈ 320 lines, profile editor ≈ 360 lines.
- `ProfileStore` and `TemplateStore` are nearly identical.
- The test count in the docs is outdated.

---

## C. Test results

Status: **Passed** · **Failed** · **Partial** · **Not tested** · **Not applicable**.

### Start, windows, sessions

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-01 | First start without configuration, shell runs | Passed | `01-erster-start.png` |
| T-02 | Start until the shell runs (4 runs) | Passed | 141 / 161 / 152 / 151 ms |
| T-03 | Close window without running programs: no confirmation, no orphaned shells | Passed | process list |
| T-04 | SIGTERM to Parkett with 7 shells: no orphaned shells | Passed | `kill -0` per PID |
| T-05 | Session: 2 tabs + split + color scheme back after restart | Passed | `session.json`, screenshot |
| T-06 | Session with directory `/tmp/a b/Ü` | Failed | QA-05 |
| T-07 | Maximize, Super+Arrow, shrink the window by keyboard | Partial | minimum width ≈ 550 px (QA-13) |
| T-08 | `exit` / Ctrl+D → bar „Prozess beendet“ (Process exited) with *Neu starten* | Passed (design QA-11) | `03-prozess-beendet-exit0.png` |
| T-09 | Profile „Neu starten“, close tab | Failed | QA-03 |
| T-10 | Broken profile and palette file | Passed (design QA-15) | `16-kaputte-dateien.png` |
| T-11 | Close a pane with `sleep` running → confirmation, Enter = *Abbrechen* (Cancel) | Passed | `24-schliessen-nachfrage.png` |
| T-12 | Second instance passes on the environment (`QA_ENV=gesetzt parkett --tab -- …`) | Failed | QA-08 |

### Terminal and shell

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-20 | ANSI 16/256/truecolor, bold, italic, underlined, curly, inverse, strikethrough, overline, blinking | Passed | `02-farben-unicode.png` |
| T-21 | Umlauts, Cyrillic, Greek, CJK, emoji, box drawing, math, RTL | Passed | same |
| T-22 | ZWJ emoji, flags | Partial | fall apart – VTE limitation (as in phase 5) |
| T-23 | Wrap a 600-character line | Passed | same |
| T-24 | `top`, `less` with search, `nano` (saving with umlauts), `htop` | Passed | screenshots, file content |
| T-25 | Ctrl+Z, `jobs`, `fg`, Ctrl+C (rc=130), Ctrl+L, Ctrl+W, Home/End | Passed | `04-jobsteuerung.png` |
| T-26 | Bash with bracketed paste: multi-line text is pasted highlighted, not executed | Passed | `11-einfuege-warnung.png` |
| T-27 | High load (`while :; do cat 200 000 Zeilen; done`): UI responds, splitting works, D-Bus response ≤ 10 ms | Passed | `05-hohe-last-teilen.png` |
| T-28 | Cursor after shrinking/enlarging with a wrapped prompt | Partial | `23-cursor-nach-umbruch.png`, QA-20 |

### Keyboard and input

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-30 | Ctrl+Shift+T, Ctrl+Page Up/Down, Alt+Arrow, Ctrl+Shift+X, Ctrl+Shift+P, Ctrl+Shift+L, Ctrl+?, Ctrl+, | Passed | `10-kuerzel.png` |
| T-31 | Ctrl+Alt+R/D (split), Ctrl+Shift+W, Ctrl+Shift+F | Passed | screenshots |
| T-32 | F2 in `htop` | Failed | QA-01, `08-f2-in-htop.png` |
| T-33 | F10 in `htop` | Failed | QA-01, `09-f10-in-htop.png` |
| T-34 | Conflicts with GNOME default shortcuts (gsettings of the VM) | Passed | no overlap |
| T-35 | Keyboard focus after closing the main menu with Esc | Failed | QA-09 |
| T-36 | Other keyboard layouts, input methods (IBus) | Not tested | German only |

### Clipboard

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-40 | Select by dragging, Ctrl+Shift+C, Ctrl+Shift+V | Passed | `wl-paste` |
| T-41 | Two-line text → warning with preview, focus on *Abbrechen* | Passed | `11-einfuege-warnung.png` |
| T-42 | Shift+Insert → same warning (QA.md still says: without) | Passed | screenshot |
| T-43 | Paste one line with a line break into `dash` | Failed | QA-04, `12-einfuegen-ohne-warnung-dash.png` |
| T-44 | Middle-click (primary selection) | Not applicable | disabled in Ubuntu |
| T-45 | Large amounts of text in the clipboard | Not tested | – |

### Search, links, context menu

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-50 | Incremental search, Enter = above, no match = red entry, Esc | Passed | `18-suche.png` |
| T-51 | OSC 8 link: tooltip shows the target | Passed | `19-links.png` |
| T-52 | URL: context menu *Link öffnen/kopieren* (Open/Copy link), Ctrl+click starts Firefox | Passed | process `firefox https://www.gnome.org` |
| T-53 | File paths as links | Not applicable | not planned (idea in G) |
| T-54 | `file:` link with a remote host | Not tested | phase 6 fix, code only |

### Preferences and appearance

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-60 | Preferences dialog: General, Profiles, Templates, Shortcuts | Passed | `20-einstellungen.png`, `21-profil.png` |
| T-61 | Changing the color scheme takes effect immediately, persists after restart | Passed | `standard.toml` |
| T-62 | System style light ↔ dark live | Passed | `26-dunkel-splits.png` |
| T-63 | Switch pane title „Als Zeile“ (As a row) live | Passed | `31-ist-zeile-dunkel.png` |
| T-64 | Text scaling 1.5 and high contrast | Partial | UI follows; color scheme stays (known, BACKLOG) |
| T-65 | Contrast of the 14 color schemes (calculated) | Partial | QA-10 |
| T-66 | Sixel | Not applicable | Ubuntu's VTE without Sixel, switch correctly grayed out |
| T-67 | Context color root/SSH | Not tested | sudo needs a password; tested in phase 4/5 |

### Error cases, stability, resources

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-70 | 5 rounds: open and close 20 terminals each | Passed | table in F |
| T-71 | Template with special characters in the startup command | Failed | QA-07 |
| T-72 | Split after `cd "/tmp/a b/Ü"` | Failed | QA-05 |
| T-73 | OSC 7 with a foreign hostname, then split | Failed | QA-06 |
| T-74 | GLib-CRITICAL on closing (review finding F2) | Passed | in scenario QA-03 no message ≤ warning in the app's journal |
| T-75 | Endurance run > 15 min | Not tested | phase 5 (D2) on Fedora |

### Accessibility

| ID | Test | Status | Evidence |
|---|---|---|---|
| T-80 | AT-SPI: buttons, tabs, terminals named | Partial | QA-18 |
| T-81 | Terminal text readable via AT-SPI | Partial | only the visible screen (84 characters), not the scrollback – VTE limitation |
| T-82 | Orca with speech output | Not tested | cannot be checked audibly |
| T-83 | Operation without a mouse (palette, shortcuts, F10, dialogs) | Partial | QA-09 |

---

## D. Bug list

By priority. Each entry names environment, steps and evidence. Unless stated otherwise, the environment for all of them is "VM
`ubuntu26.04`, see B".

### QA-01 – F2 and F10 do not reach terminal programs · high · reproduced

- **Category:** function / usability
- **Initial state:** default shortcuts, one terminal with focus.
- **Steps:**
  1. Start `htop`.
  2. Press F2.
  3. Esc, then press F10.
- **Expected:** htop opens *Setup* (F2) or quits (F10).
- **Actual:**
  - F2 opens „Reiter umbenennen“.
  - F10 opens Parkett's main menu.
  - htop keeps running after F10 (checked with `pgrep`).
- **Evidence:** `08-f2-in-htop.png`, `09-f10-in-htop.png`.
- **Impact:**
  - `htop` (F2 Setup, F10 Quit), `mc` (F2 menu, F10 Quit) and `nano` (F2 Exit) can only be used with restrictions. This hits admins every day.
  - To beginners it looks as if the program had hung.
- **Cause:**
  - F2: default shortcut of `win.rename-tab` (`app/src/shortcuts.rs:38`), as a shortcut in the capture phase before VTE.
  - F10: GTK opens the header bar's main menu with F10. This is the default behavior for a primary menu button.
- **Recommendation:**
  - *Reiter umbenennen* without a default shortcut (double-click and palette remain).
  - Intercept F10 while a terminal has focus (GNOME Terminal has a setting for this, off by default).
  - In general: no function key without a modifier as a default. Recording custom shortcuts already rejects plain function keys (`shortcuts.rs:195`), the defaults should too.
- **Regression test:** extend the VM smoke test with starting `htop` and F2/F10 via QMP; check that htop quits with F10.

### QA-02 – Floating pane title covers prompt, input and cursor · high · reproduced

- **Category:** usability / design
- **Initial state:** default „Bereichstitel: Schwebend“ (Pane title: Floating), Ubuntu prompt `testt@testt-Standard-PC-Q35-ICH9-2009:~$` (40 characters), pane wider than 380 px.
- **Steps:**
  1. *Rechts teilen* (Split right).
  2. Type `echo sichtbar` in the new pane.
- **Expected:** input and cursor are visible.
- **Actual:**
  - The title lies over columns ≈ 26–55 of the first line.
  - The input only shows through faintly.
  - `htop` also loses parts of the CPU bar.
- **Evidence:** `06-titel-verdeckt-prompt.png`, `07-titel-verdeckt-eingabe.png`, `08-f2-in-htop.png`.
- **Impact:**
  - In every freshly split pane you type blind at first.
  - All users with splits are affected; the default prompt of Ubuntu, Debian and Fedora is long.
- **Cause:** the title is a `GtkOverlay` above the VTE (`app/src/pane.rs:144–168`). Below 380 px width only the text is hidden (phase 5 fix), the buttons remain.
- **Recommendation:** design decision, see H (three variants). Possible right away without any rework: default to „Als Zeile“ (As a row).
- **Regression test:** screenshot after *Rechts teilen* with a 40-character prompt; the cursor position (VTE) must not lie beneath the title.

### QA-03 – Closed tab with profile setting „Neu starten“ restarts its program invisibly · high · reproduced

- **Category:** function (resources, possibly security)
- **Initial state:** profile `qa-neustart` with `command = "bash --norc -c \"…; exec sleep 1000\""` and `exit_action = "restart"`.
- **Steps:**
  1. `parkett --tab -p qa-neustart`.
  2. Close the tab (`close-tab`).
  3. Wait 13 s and 64 s.
  4. Look at Parkett's children.
- **Expected:** after the Undo period expires, the process is ended and there is no new one.
- **Actual:**
  - `sleep` (PID 9760) was ended.
  - At 09:06:03 a **new** `sleep 1000` (PID 9823) was running. It was still there at 09:06:59 and only disappeared when another tab was closed.
- **Evidence:** process lists in the log (timestamps 09:05:55, 09:06:09, 09:06:59).
- **Impact:**
  - A typical use of „Neu starten“ is `ssh server` or a log tail. After closing, Parkett invisibly re-establishes the connection.
  - The closed tab keeps the VTE buffer and scrollback in memory.
- **Cause:**
  - The expired Undo toast holds on to the `Tab` (`window.rs:408–420`).
  - Terminating via SIGHUP triggers `child-exited`. With `ExitAction::Restart` the handler restarts, because the shell ran longer than 2 s (`terminal.rs:509`).
  - `terminate()` does not remember that it was ended deliberately.
- **Recommendation:**
  - Set a flag in `terminate()` that suppresses restart and the bar.
  - Release the toast and the tab once the period has expired.
- **Regression test:**
  - Unit test for "deliberately ended → no restart".
  - In the smoke test, a profile with `restart`: close the tab, wait 10 s, check the process count.

### QA-04 – Paste warning lets one line with a line break through · medium · reproduced

- **Category:** security
- **Initial state:** „Vor mehrzeiligem Einfügen warnen“ (Warn before pasting multiple lines) on, shell without bracketed paste (`dash`, old servers, Python REPL).
- **Steps:**
  1. Triple-click the printed line `echo AUSGEFUEHRT-OHNE-RUECKFRAGE`. VTE includes the line break.
  2. Ctrl+Shift+C.
  3. Start `dash`.
  4. Ctrl+Shift+V.
- **Expected:** a warning, because the text contains a line break and would be executed immediately.
- **Actual:** no warning, the command runs immediately.
- **Evidence:** `12-einfuegen-ohne-warnung-dash.png`.
- **Impact:** classic "pastejacking": a web page puts `curl … | sh` followed by a line break into the clipboard, one click is enough. Bash 5.3 with bracketed paste is protected, `sh`, `dash` and server shells without this option are not.
- **Cause:** the trailing line break is cut off before the lines are counted (`terminal.rs:919–925`); but the whole text is pasted.
- **Recommendation:** treat every text with `\n` or `\r` as "will be executed". Alternatively, strip the trailing line break when pasting, like GNOME Console.
- **Regression test:** move the line counting to `core` and test it (`"a\n"` → warning).

### QA-05 – Directories with spaces or umlauts get lost · medium · reproduced

- **Category:** function
- **Steps:**
  1. `cd "/tmp/a b/Ü"`.
  2. *Unten teilen* (Split down).
  3. `pwd` in the new pane.
- **Expected:** `/tmp/a b/Ü`.
- **Actual:**
  - `/home/testt`.
  - `session.json` contains `"subdir": "tmp/a%20b/%C3%9C"`, so the session does not restore the directory either.
  - Control: `/usr/share` works.
- **Evidence:** `13-verzeichnis-leerzeichen.png`, excerpt from `session.json`.
- **Impact:** "new terminals in the current directory" fails silently for all paths with spaces or non-ASCII characters. Typical examples are `~/Schreibtisch/Neuer Ordner` or `Übersetzungen`.
- **Cause:** `Terminal::cwd()` takes `Uri::path()` of the percent-encoded OSC 7 URI (`terminal.rs:226–231`).
- **Recommendation:** `gio::File::for_uri(uri).path()` or `glib::Uri::unescape_string`. Fix together with QA-06.
- **Regression test:** unit test with `file:///tmp/a%20b/%C3%9C`.

### QA-06 – Foreign hostname in OSC 7 sets the local start directory · medium · reproduced

- **Category:** security / function
- **Steps:**
  1. `printf '\e]7;file://entfernter-server/etc\a'; PROMPT_COMMAND= bash --norc`. This corresponds to an SSH session to a server with `vte.sh`.
  2. *Unten teilen*.
  3. `pwd`.
- **Expected:**
  - The path of another machine is ignored.
  - Fallback: the real directory of the local process (`/proc/<pid>/cwd`) or the home directory.
- **Actual:** the new local pane starts in `/etc`.
- **Evidence:** `14-osc7-fremder-rechner.png`.
- **Impact:**
  - After `ssh server` and `cd /srv/app`, *split* starts locally in `/srv/app` (if it exists) or silently in the home directory.
  - Any output (`cat datei`) can set the start directory, for instance to a prepared Git repo whose hooks or `core.fsmonitor` a Git prompt runs.
  - Exploiting this takes several steps. But the mechanism is there.
- **Cause:** `terminal.rs:226–231` does not check the host. The check `is_local_file_uri` for links already exists.
- **Recommendation:** only accept URIs without a host or with the local hostname, otherwise `/proc/<pid>/cwd`.
- **Regression test:** unit test of the host check, VM step as above.

### QA-07 – Startup commands of a template invisible, *Öffnen* preselected · medium · reproduced

- **Category:** security / usability
- **Steps:**
  1. Create a template with `command = "make -j8 && ./start.sh 2>&1 <eingabe"`.
  2. Palette: `#QA`, Enter.
- **Expected:** the command appears under „Startbefehle ausführen“ (Run startup commands).
- **Actual:**
  - The subtitle is empty.
  - The switch is *on*, *Öffnen* (Open) has the focus.
- **Evidence:** `15-vorlage-startbefehl-unsichtbar.png`.
- **Impact:** you run commands you have not seen. Templates only come from your own configuration folder, so the risk is limited. With shared dotfiles or copied templates, however, it is real.
- **Cause:**
  - The subtitle of an `AdwActionRow` is Pango markup; `&` and `<` are not escaped (`window.rs:892`).
  - Same pattern: `window.rs:880` (directory), `prefs.rs:155` (scheme name), `prefs.rs:791`, toast title `window.rs:393` (tab title from OSC 0).
- **Recommendation:** `glib::markup_escape_text` wherever foreign text ends up in markup, or `use_markup(false)`.
- **Regression test:** open a template with `&&`; the subtitle must show the command.

### QA-08 – Second invocation loses its environment · medium · reproduced

- **Category:** function
- **Steps:**
  1. Parkett is running.
  2. `QA_ENV=gesetzt parkett --tab -- bash -c 'echo QA_ENV=${QA_ENV:-FEHLT}; exec bash'`.
- **Expected:** `QA_ENV=gesetzt`.
- **Actual:** `QA_ENV=FEHLT`.
- **Evidence:** `17-umgebung-zweite-instanz.png`.
- **Impact:**
  - `xdg-terminal-exec` from applications with their own environment, Python `venv`, `SSH_AUTH_SOCK` or `DISPLAY` changes do not reach the new terminal.
  - GNOME Terminal and Ptyxis take over the caller's environment.
- **Cause:** `ApplicationCommandLine::environ()` is not evaluated in `main.rs:128–197`.
- **Recommendation:** pass the invocation's environment on to the spawn of the new pane (only for terminals opened via the command line).
- **Regression test:** add the step above to the smoke test.

### QA-09 – Focus stays on the menu button after the main menu · medium · reproduced

- **Category:** usability / accessibility
- **Steps:**
  1. F10 (or click on the main menu button).
  2. Esc.
  3. Type `htop` and press Enter.
- **Expected:** input lands in the terminal.
- **Actual:**
  - AT-SPI reports `[toggle button] 'Hauptmenü' focused`.
  - The letters are lost, Enter opens the menu again.
  - `htop` did not run.
- **Evidence:** AT-SPI output, `27-fokus-bleibt-im-menue.png`.
- **Impact:** anyone who enters the menu by keyboard loses input afterwards. This is especially treacherous because Enter opens the menu again.
- **Cause (suspected):** GTK returns the focus to the button after a popover closes; Parkett does not put it on the terminal. No comparison with Ptyxis was made.
- **Recommendation:** on the popover's `closed`, put the focus on the active terminal. This also applies to the tab overview, New menu and search.
- **Regression test:** AT-SPI check "terminal has focus" after menu + Esc.

### QA-10 – Color scheme „One“ dark: text barely readable · medium · reproduced (calculated)

- **Category:** design / accessibility
- **Evidence:** `data/palettes/one.palette`, `[Dark]`: `Foreground=#5C6370` on `Background=#1E2127`, contrast **2.7:1** (WCAG AA requires 4.5:1). In One Dark, `#5C6370` is the color for comments; the usual foreground is `#ABB2BF`.
- **Further calculated results** (colors below 3:1 against the background):
  - In the light variants of Nord, Everforest, Catppuccin, GitHub, Tango and Solarized, 7–12 of the 16 colors are below 3:1. For white/light gray (7, 15) this is usual.
  - For green, yellow and cyan this is a real readability problem, e.g. Tango light `33`/`93`.
  - „Hoher Kontrast“ (High Contrast) light has 5 colors below 3:1.
- **Recommendation:**
  - Fix the foreground of One dark.
  - Adjust „Hoher Kontrast“ so that all colors except 0/7/8/15 reach at least 4.5:1.
  - Optionally, the contrast check as a unit test for all palettes.
- **Regression test:** test "foreground ≥ 4.5:1 in every palette".

### QA-11 – Normal exit (`exit`, Ctrl+D) appears as an error · low · reproduced

- **Category:** usability / design
- **Observation:**
  - After `exit` the pane stays, with a warning icon and a reddish bar „Prozess beendet“.
  - In the only tab, the window does not close.
- **Evidence:** `03-prozess-beendet-exit0.png`.
- **Assessment:** this is the deliberate default `exit_action = "hold"`. GNOME Console, Ptyxis and Tilix close on exit code 0. Beginners expect `exit` to close the window.
- **Recommendation:**
  - Default „Bereich schließen bei Erfolg“ (Close pane on success).
  - With code 0, a neutral bar without a warning icon.
  - Decision is up to Christoph.

### QA-12 – Shortcuts hard-coded in tooltips, mixed languages · low · reproduced

- **Observation:**
  - „Bereich maximieren (Strg+Umschalt+X)“ (Maximize pane (Ctrl+Shift+X)), „Bereich schließen …“ (Close pane …) and the search buttons are hard-coded (`pane.rs:148/153/191/196`). They do not follow custom shortcuts.
  - Next to them „Rechts teilen (Ctrl+Alt+R)“ from `accelerator_get_label`. Without a GTK translation this is English.
- **Recommendation:** generate all tooltips from `shortcuts::label()` and set them again on change.

### QA-13 – Minimum window width ≈ 550 px · low · reproduced

- **Observation:** by keyboard (Alt+F8) the window could not be made narrower than ≈ 550 px (measured from the screenshot). The header bar with 7 buttons and title determines the width.
- **Evidence:** `22-mindestbreite.png`.
- **Assessment:** the HIG recommend 360 px. For a terminal this matters when tiling on small screens (e.g. three columns on 1366 px).
- **Recommendation:** below ≈ 600 px move the split buttons into the New menu (`AdwBreakpoint`).

### QA-14 – Blinking text costs 7.8 % CPU when idle · low · reproduced (VM)

- **Measurement:**
  - 7 terminals, idle 20 s: 0.40–0.65 %.
  - Same state with visible SGR 5 text ("blink"): 7.8 %.
  - Unchanged with `cursor-blink=false`.
- **Assessment:** this is software rendering in the VM; much less on a real GPU (not measured). VTE can disable blinking text or restrict it to focus (`set_text_blink_mode`).
- **Recommendation:** `TextBlinkMode::Focused` as the default, like GNOME Terminal.

### QA-15 – Message „Nicht lesbare Dateien“ (Unreadable files) hard to read · low · reproduced

- **Observation:**
  - The TOML error with the `^` pointer is shown centered in a proportional font; the pointer is in the wrong place.
  - The toast disappears after a few seconds, and the notice cannot be called up again afterwards.
  - The palette message „Color0 in [Palette] fehlt“ (Color0 missing in [Palette]) names the wrong section (`[Light]`/`[Dark]` were missing).
- **Evidence:** `16-kaputte-dateien.png`.
- **Recommendation:**
  - Details left-aligned in a monospace font.
  - Toast without timeout.
  - Message „Abschnitt [Light] fehlt“ (Section [Light] missing).

### QA-16 – Profile editor opens with the name selected · low · reproduced

- **Observation:** when a profile is opened, the name entry has the focus and the name is selected. One keystroke overwrites it (and saves immediately).
- **Evidence:** `21-profil.png`.
- **Recommendation:** focus on the page, not in the entry.

### QA-17 – Identically labeled tabs and titles · low · reproduced

- **Observation:** window title, subtitle, tab and pane title show the same shell title. Four tabs look the same.
- **Evidence:** `26-dunkel-splits.png`.
- **Recommendation:** H, note "short tab label".

### QA-18 – Minor screen reader names · low · reproduced

- **Observation:**
  - The tab overview button is just called „4“ (the count); the description carries the meaning.
  - The New button has the description „Weitere Möglichkeiten“ (More options) (internal arrow part of the AdwSplitButton).
  - The pane buttons read the shortcut as part of the name („Bereich schließen (Strg+Umschalt+W)“).
- **Recommendation:**
  - Name „Reiter-Übersicht, 4 Reiter“ (Tab overview, 4 tabs).
  - Shortcut only in the tooltip.

### QA-19 – Search without match count · low · reproduced

- **Observation:** only the current match is highlighted; there is no "3 of 11" display. This is a limitation of the VTE API, which does not provide a list of matches.
- **Recommendation:** optionally count them ourselves (scrollback text via `text_range_format`). Weigh effort against benefit.

### QA-20 – Cursor in the wrong place after wrapping · low · still to be investigated

- **Observation:**
  - Window made narrow (prompt wraps), then maximized.
  - In the inactive panes the hollow cursor sits in the middle of the prompt (column 24) instead of after it.
- **Evidence:** `23-cursor-nach-umbruch.png`.
- **Assessment:** probably VTE rewrapping plus bash redrawing, so not Parkett. Not cross-checked against Ptyxis.

### Only confirmed in code (review), not triggered in the VM

| ID | Finding | Location | Severity |
|---|---|---|---|
| R-01 | `update_page` for tabs that are no longer in the TabView; `view.page()` returns NULL. **Not** triggered in scenario QA-03 (no CRITICAL); other paths: dragging a tab into another window, command finishing in a closed tab | `window.rs:429–486` | medium (suspected) |
| R-02 | Smoke test misses GLib-CRITICALs because `journalctl -o cat` does not show the prefix | `build-aux/vm/smoke.sh:46` | medium |
| R-03 | Environment variables field destroys values with spaces/quotes (`JAVA_OPTS=-Xmx1g -Dx=1` → two entries) | `prefs.rs:478/488` | medium |
| R-04 | `file:///…` links via OSC 8 from an SSH session open local files with the default application (tooltip shows the target) | `terminal.rs:894–901` | low–medium |
| R-05 | Undo after a normal exit (profile "close") brings back a dead pane without a bar | `terminal.rs:497`, `tab.rs:452` | low |
| R-06 | `GIO_LAUNCHED_DESKTOP_FILE(_PID)` reaches all shells (GNOME Terminal removes them) | `.desktop`, `main.rs` | low |
| R-07 | Dragging a divider does not save the session; the save timer restarts on every title change | `tab.rs:683`, `session.rs:64` | low |
| R-08 | `ratio = nan` in a template → `null` in `session.json` → session lost | `template.rs:106` | low |
| R-09 | The startup command is typed immediately; with an ssh profile with a password prompt, it ends up there | `terminal.rs:458` | low (suspected) |
| R-10 | `-e "htop -d 5"` as a single argument is not split; `--tab=x` silently accepted | `cli.rs:52` | low |
| R-11 | `ssh -vp 22 host` → context chip „22“ | `context.rs:45` | low |
| R-12 | Profiles (environment with tokens) with the default umask: 0644 on Arch, 0664 in the Ubuntu VM (umask 002) | `core` `*Store::save` | low |
| R-13 | Docs: QA.md says Shift+Insert does not warn – it does by now; test count outdated (53 + 2 + 1) | `docs/QA.md`, `ARCHITECTURE.md` | low |

---

## E. Usability and design assessment

### Usability

**Beginners:**

- Starting, typing, copying and pasting work as expected.
- With seven icons, the header bar is clear; the tooltips name the shortcuts.
- Weak:
  - `exit` closes nothing and looks like an error (QA-11).
  - F2 and F10 behave "wrongly" in programs (QA-01).
  - In a split pane you cannot see your own input (QA-02).
- The main menu contains only 5 entries. Palette, layout mode and templates are missing there and can only be found via shortcuts or the New menu.

**Experienced users with several tasks:**

- Splits, focus via Alt+Arrow, layout mode and palette are fast and consistent.
- Undo on closing is a real advantage over other terminals.
- Weak:
  - Identically labeled tabs (QA-17).
  - No hint which tab is which without opening the overview.

**Admins and developers:**

- Profiles, context color, templates with startup commands and the command line cover everyday work well.
- Weak:
  - Function keys (QA-01).
  - Environment with `parkett --tab` (QA-08).
  - Directories with spaces (QA-05).
  - Invisible startup commands (QA-07).
  - The environment field in the profile (R-03).

**Keyboard/mouse consistency:**

- Almost everything works both ways.
- Exceptions:
  - The focus stays in the header after menus (QA-09).
  - *Reiter umbenennen* has a function key, *Als Vorlage speichern* (Save as template) has no shortcut at all.

### Design

**Strengths:**

- Cleanly built on libadwaita: AdwTabBar, AdwPreferencesDialog, AdwAlertDialog with safe default, AdwToast.
- Light/dark and accent color follow live.
- Color scheme tiles with a light and a dark half are clear.
- The focus line (2 px accent) is a good, calm signal.

**Weaknesses:**

1. **Hierarchy of titles.** Three levels repeat the same 40-character text. This contradicts the desired "clear hierarchy instead of repetition".
2. **Floating elements above the content.** Besides the pane title, also the bar „Maximiert · 1 Bereich ausgeblendet“ (Maximized · 1 pane hidden) (`10-kuerzel.png`, top center). Both lie above terminal text.
3. **Dimming barely visible.** Inactive panes are only dimmed in the text. The measured background is `#2E3335` versus `#2E3436`. In Tango dark, the difference between active and inactive is mainly the line.
4. **Error bar for normal cases.** Warning icon and red tint even with exit code 0 (QA-11).
5. **Error dialog** centered with a proportional font (QA-15).
6. **Long combo values cut off** in the preferences („Verzeichnis des aktive…“ (Directory of the active…)). At a screen width of 1280 px this is noticeable.

Ubuntu-specific: the Yaru orange works as an accent; the focus line picks it up correctly.

---

## F. Security and performance

### Measurements (VM, software rendering)

| Measurement | Result | Conditions |
|---|---|---|
| Start until the shell runs | 141–161 ms (4 runs) | `parkett -- sh -c 'date …'`, warm, session restore on |
| Memory, 1 terminal | 219 MB RSS | first start, 1280×800 |
| Memory, 7–8 terminals | 289–345 MB RSS | 1920×1033, depending on scrollback |
| Idle CPU | 0.40–0.65 % | 7 terminals, 20 s |
| Idle CPU with blinking text | 7.8 % | QA-14 |
| CPU under continuous output | 88.6 % of one core | `cat` in a loop, llvmpipe |
| D-Bus response under continuous output | ≤ 10 ms | `org.gtk.Actions.List`, 3 measurements |
| Throughput `seq` 3 million lines | Parkett 1.67–1.79 s · Ptyxis 1.55–1.63 s | maximized, same VM, 3 runs each |
| Throughput 20 MB of colored text | Parkett 0.69–0.76 s · Ptyxis 0.80–0.94 s | same |
| Open/close 5 × 20 terminals | shells always back to 8; RSS 316 → 365 MB after round 1, then +0.2 MB per round; FDs 44 → 56 constant; 42 threads | `new-tab` + `split-right`, `close-tab`, 10 s pause |

Parkett's memory **cannot** be compared with Ptyxis here: Ptyxis had 1 terminal,
Parkett 8. The increase after the first round is probably the allocator and glyph cache, not a leak.
This is not proven. A measurement run over hours is missing.

### Security

**Checked and unremarkable:**

- Links only with allowed schemes; `file:` with a remote host is blocked (phase 6).
- The tooltip shows the real OSC 8 target.
- The Nautilus extension starts without a shell (argv), no injection.
- `session.json` stores no commands.
- Context chip from `/proc`, not from terminal output.
- No network access by Parkett itself. This was only checked in the code, not with a network capture.

**Findings:** QA-04, QA-06, QA-07 (reproduced), R-04, R-06, R-12 (code).

**Escape sequences for deception:**

- A foreign title (OSC 0) can imitate the SSH/root chip as text, but cannot remove it.
- It ends up unfiltered as markup in the Undo toast (QA-07, follow-up finding).
- OSC 7 can set the start directory (QA-06).
- Dependencies were **not checked** for known vulnerabilities (`cargo audit`); the tool is not installed.

**Explicitly:** the fact that no further vulnerabilities were found does not mean that Parkett is secure.
Not checked were fuzzing of the palette and TOML parsers, tests with malicious
escape sequences beyond VTE, and the D-Bus interface for abuse by other
processes of the session. The latter is session-internal and corresponds to other GTK apps.

---

## G. Improvement proposals

Rating: benefit / effort / risk / complexity, each high (H), medium (M), low (L).

### 1. Necessary bug fixes

| # | Measure | Finding | Benefit | Effort | Risk | Compl. |
|---|---|---|---|---|---|---|
| 1 | F2 without a default shortcut, pass F10 through when a terminal has focus | QA-01 | H | L | L | L |
| 2 | Do not restart deliberately ended terminals; release the expired toast | QA-03, R-01 | H | L | L | M |
| 3 | Paste warning on every line break | QA-04 | H | L | L | L |
| 4 | Decode the OSC 7 URI and check the host, otherwise `/proc/<pid>/cwd` | QA-05, QA-06 | H | L | L | L |
| 5 | Escape markup (templates, toasts, tooltips, scheme names) | QA-07 | M | L | L | L |
| 6 | Take over the command line's environment | QA-08 | M | M | L | M |
| 7 | Focus back to the terminal after popovers | QA-09 | M | L | L | L |
| 8 | One dark: foreground `#ABB2BF` | QA-10 | M | L | L | L |
| 9 | Environment field: quoting when displaying and parsing | R-03 | M | L | L | L |
| 10 | Smoke test: detect CRITICALs reliably, htop/function keys, restart profile | R-02 | M | L | L | L |

### 2. Worthwhile improvements

| # | Measure | Finding | Benefit | Effort | Risk | Compl. |
|---|---|---|---|---|---|---|
| 11 | Short tab label "program · folder" (with number) | QA-17 | H | M | L | M |
| 12 | Blinking text only with focus | QA-14 | M | L | L | L |
| 13 | Tooltips from the current bindings | QA-12 | L | L | L | L |
| 14 | Error dialog in a monospace font, toast without timeout | QA-15 | L | L | L | L |
| 15 | Profile editor: no focus in the name entry | QA-16 | L | L | L | L |
| 16 | Screen reader names (overview, shortcut not in the name) | QA-18 | M | L | L | L |
| 17 | Remove `GIO_LAUNCHED_DESKTOP_FILE*` before spawning | R-06 | L | L | L | L |
| 18 | Write profiles/templates with 0600 | R-12 | L | L | L | L |
| 19 | Save the session after dragging a divider, catch NaN | R-07, R-08 | L | L | L | L |
| 20 | Contrast test for all palettes; sharpen „Hoher Kontrast“ | QA-10 | M | L | L | L |
| 21 | `cargo audit` in `build-aux/ci.sh` | F | M | L | L | L |

### 3. Optional features and larger design changes (decision needed)

| # | Measure | Finding | Benefit | Effort | Risk | Compl. |
|---|---|---|---|---|---|---|
| 22 | New pane title: V1, V2 or V3 (see H) | QA-02 | H | L–H | L–M | L–H |
| 23 | Default on process exit: close on code 0, neutral bar | QA-11 | M | L | M (habit) | L |
| 24 | Adaptive header bar below 600 px (`AdwBreakpoint`) | QA-13 | L | M | L | M |
| 25 | Add palette, layout mode, templates to the main menu | E | M | L | L | L |
| 26 | File paths in the output as links (Ctrl+click opens in the file manager) | T-53 | M | M | M | M |
| 27 | Match count in the search | QA-19 | L | M | L | M |

---

## H. Design variants

In detail with images: **[`designvarianten.html`](designvarianten.html)**. They are based on real
screenshots from the VM:

- V1 is a real photo of the existing setting.
- For V2 and V3, the changes are drawn onto real photos. These are mockups, not running software.

| | V1 title row as default | V2 floating, evasive | V3 status bar, short tabs |
|---|---|---|---|
| Idea | the existing row per pane becomes the default | title invisible, appears 1.5 s after a focus change and when the mouse is at the top edge | no pane titles; one 20 px bar at the bottom for the active pane; tabs "No. program · folder" |
| Covers text | never | only on hover | never |
| Space | 1 row per pane | none | 1 row per window |
| Titles of inactive panes | always | on hover | not visible |
| Beginners / power users | very good / good | moderate / very good | good / very good |
| Engineering | default value only | hover zone, timer, cursor row | new bar in `window.rs`, tab titles from the 1 s timer |
| Effort | very small | medium | medium to large |

Independently of this: the short tab label (proposal 11) can be combined with any variant.
**None of the variants is implemented.**

---

## I. Prioritized action plan

**Short term (one version, e.g. 0.1.1), each measure small and independent:**

1. QA-01 function keys (proposal 1). Most noticeable, smallest intervention.
2. QA-03 restart after closing (2). Extend the smoke test first (10), so that R-01/R-02 are covered as well.
3. QA-05 + QA-06 OSC 7 (4). One place, one unit test.
4. QA-04 paste warning (3).
5. QA-07 markup (5) and QA-09 focus (7).
6. QA-10 palette One (8), R-03 environment field (9).

**Afterwards (0.2):**

7. QA-08 environment of the command line (6). Needs some rework in the spawn path.
8. Proposals 12–21, bundled as desired.

**With a design decision (approval first):**

9. Pane title V1/V2/V3 (22). Dependency: if V3 is chosen, proposal 11 is part of it. With V1/V2, 11 stands on its own.
10. Process exit default (23), adaptive header bar (24), main menu (25).

**Larger, later work:** path links (26), match count (27). Plus the open items from the
BACKLOG (synchronized input, translations, Debian package – building on Ubuntu 26.04 is proven by
this audit).

---

## J. Test coverage and open questions

**Not checked (and why):**

- **Orca with speech output:** cannot be checked audibly. AT-SPI structure checked.
- **Sixel:** Ubuntu's VTE without Sixel.
- **Context color root/SSH:** sudo needs a password; tested on Fedora in phase 4/5.
- **Nautilus extension on Ubuntu:** `python3-nautilus` installed, but not clicked.
- **X11 (XWayland), HiDPI/scaling 200 %, multiple monitors.**
- **Real GPU on Ubuntu:** all CPU values are software rendering.
- **Other keyboard layouts and input methods** (only German via QMP).
- **Endurance run over hours, memory in long sessions:** only 5 rounds of open/close; phase 5 has 15 min on Fedora.
- **Dragging dividers with the mouse, dragging a tab into a new window, notification when a command finishes, deleting a profile.**
- **Large clipboard contents, pasting via middle-click:** disabled in Ubuntu.
- **`cargo audit`, fuzzing the parsers, network capture.**

**Still missing before production use:**

- Regression tests for QA-01 to QA-09 in the smoke test or as unit tests.
- A run on real hardware with Ubuntu or Fedora.
- A manual Orca test.

**Open questions for Christoph:**

1. Which design variant for the pane titles (V1/V2/V3)? Short tab label yes/no?
2. Process exit default: keep "show bar" or "close on success"?
3. Should *Reiter umbenennen* get a different default shortcut (proposal: none) and F10 stay completely free?

**Changes to the VM `ubuntu26.04` made by this audit:**

- Installed: `rustc cargo meson libvte-2.91-gtk4-dev libadwaita-1-dev libgtk-4-dev pkg-config gir1.2-atspi-2.0 python3-nautilus xdotool vttest htop`.
- Test SSH key `id_vmtest` in `~testt/.ssh/authorized_keys` (SSH port 2222).
- Parkett in `~testt/.local`, source tree in `~testt/parkett`, test scripts in `~testt/qa`.
- Test profiles, palettes and templates deleted again.
- GSettings (color scheme, pane title, text scaling, high contrast) reset.
- Parkett is still running there.

---

## Implementation (2026-10-19, after approval)

Christoph approved items 1–6 from I and chose **design V2** for the pane titles.
Implemented and checked:

| Finding | Change | Test |
|---|---|---|
| QA-01 | *Reiter umbenennen* without a default shortcut; main menu no longer "primary" (F10) | Ubuntu: F2 in `htop` opens Setup, F10 quits `htop`; smoke test |
| QA-02 | Design V2: title hidden, visible with the pointer in the top strip (40 px) and for 1.5 s after a focus change, not when the cursor is in row 1; no click area while hidden | Ubuntu: after splitting, prompt and input are clear; Alt+Arrow shows the title briefly; hover shows it, leaving hides it; maximize button clickable |
| QA-03, R-01 | `terminate()` remembers the termination, no restart; the expired toast is released; a tab's window is determined at call time, page lookup without assumptions | smoke test (Ubuntu, Fedora): profile „Neu starten“, close tab, no process after 11 s |
| QA-04 | Check in `core::paste`, warns on every `\n`/`\r` | unit tests; Ubuntu: line with a line break in `dash` → dialog „Text mit Zeilenumbruch einfügen?“ (Paste text with line break?) |
| QA-05, QA-06 | `core::uri::local_path` (percent encoding, hostname); new terminals fall back to the shell's directory | unit tests; Ubuntu: `/tmp/a b/Ü` is taken over; after a foreign OSC 7, the new pane starts locally in `/tmp/a b/Ü` instead of `/etc` |
| QA-07 | Markup escaped (template dialog, tab tooltip, toasts as plain text, profile/template lists) | Ubuntu: template „QA <Vorlage> & Co“ with `make -j8 && … <eingabe` shown completely |
| QA-09 | Focus back into the terminal after the main menu, New menu and tab overview (not when a dialog is open) | Ubuntu: menu + Esc, overview + Esc, commands typed afterwards appear in `history` |
| QA-10 | One dark `#ABB2BF`; test "foreground ≥ 4.0:1" for all schemes | unit test |
| R-03 | Environment field shows quoted values (`core::profile::environment_line`) | unit test |
| R-02 | Smoke test evaluates the journal as JSON (every entry with `GLIB_DOMAIN`) | cross-checked with an artificial CRITICAL message: it is counted |

`build-aux/ci.sh` green (61 + 2 + 1 tests, Clippy without warnings). Smoke test green in
`ubuntu26.04` (15 checks) and `fedora44-wor` (13, without `htop`).
