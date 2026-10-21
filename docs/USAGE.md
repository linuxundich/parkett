# Usage

The interface is currently available in German only; German labels are given below with their English meaning.

As of 0.2.0. All shortcuts can be changed or disabled under *Einstellungen › Kürzel*
(Preferences › Shortcuts); Ctrl+? shows the current bindings.

## Tabs and panes

- **Split:** the *Rechts teilen* (Split right) and *Unten teilen* (Split down) buttons on the left
  of the header bar, Ctrl+Alt+R, Ctrl+Alt+D or Ctrl+Alt+A (splits along the longer side). If there
  is not enough room (less than 20 × 5 characters per half), Parkett says so instead of splitting.
- **Move focus:** Alt+arrow; **move divider:** Shift+Alt+arrow;
  **equalize sizes:** Ctrl+Alt+=; **maximize pane:** Ctrl+Shift+X.
- **Layout mode** (Ctrl+Shift+L): panes show numbers. Digit = focus,
  arrows = focus, Shift+arrow = move divider, *M* followed by an arrow = swap with the
  neighbor, *R*/*D*/*A* = split, *Z* = maximize, *=* = equalize,
  *X* = close, Esc or Enter = done. The bar at the bottom lists all keys.
- **Close:** Ctrl+Shift+W (pane), Ctrl+Shift+Q (tab). If a program other than the shell is still
  running there, Parkett asks first. Afterwards, *Rückgängig* (Undo) is available for 8 seconds;
  the process keeps running until then. Closing something else finalizes the previous close.
- **Tab labels:** number, running program and folder, e.g. `1 zsh · parkett`, `2 htop · ~` or
  `3 ssh bender`; the number matches Ctrl+Alt+1…9. The full title set by the shell is in the
  window title and in the tab's tooltip. A tab you renamed keeps its name. Back to the shell
  title: *Einstellungen › Allgemein › Kurze Reiter-Namen* (Preferences › General › Short tab names).
- **Tab overview:** Ctrl+Shift+O; **rename:** main menu or command palette (no
  default shortcut: F2 and F10 stay with the programs in the terminal, e.g. `htop`, `mc`, `nano`).
- **Pane title** (in split tabs): floats at the top right, but is normally hidden so it does
  not cover a line. It appears while the pointer is at the top of the pane, and for 1.5 s after
  a focus change (not when the cursor is in the top line).
  Always visible: *Einstellungen › Allgemein › Bereichstitel: Als Zeile*
  (Preferences › General › Pane title: As a row).

## Command palette

Ctrl+Shift+P opens a list of all actions, tabs, panes, templates and profiles with
fuzzy search. Prefixes narrow it down: `>` actions only, `@` tabs and panes only,
`#` templates only. Each action shows its shortcut.

## Terminal

| What | Shortcut |
|---|---|
| Copy / Paste | Ctrl+Shift+C / Ctrl+Shift+V, Shift+Insert |
| Search, previous/next match | Ctrl+Shift+F, Ctrl+Shift+G, Ctrl+Shift+H |
| Font larger/smaller/normal | Ctrl++, Ctrl+-, Ctrl+0 |
| Open link | Ctrl+click; right-clicking a link offers *Öffnen* (Open) and *Kopieren* (Copy) |

Text containing a line break (even a single line with a break at the end) first shows a preview
when pasted via Ctrl+Shift+V, Shift+Insert or the context menu, because a shell without
"bracketed paste" would run it immediately (can be turned off under *Einstellungen › Allgemein*
(Preferences › General)).

**New terminals in the current directory** take the directory from the
shell integration (OSC 7). If an SSH session reports the path on the server, Parkett
uses the shell's local directory instead.

**Command finished:** if a command that ran for at least 10 seconds ends in a pane that is
not currently visible (another tab, or the window is in the background), the tab is
marked and a notification appears; clicking it shows the pane. This requires
VTE's shell integration (`vte.sh`, included for bash and zsh on Fedora and Arch);
can be turned off under *Einstellungen › Allgemein*.

**Sixel graphics:** programs such as `img2sixel` or `chafa -f sixel` display images directly in
the terminal if the installed VTE library is built with Sixel support. Arch (`vte4` 0.84.1) and Fedora 44 (`vte291-gtk4` 0.84.1) currently build it
without; in that case the switch under *Einstellungen › Allgemein* is grayed out and states the reason.

**Context color:** if `ssh`/`mosh` or a program running as root (`sudo`, `su`,
`doas`, `run0`, `pkexec`) is running in the pane, the background is tinted slightly and a colored line with
the host name or "root" appears. Can be turned off under *Einstellungen › Allgemein*.

## Command line

```text
parkett [OPTIONEN] [-- BEFEHL [ARGUMENTE …]]
  -d, --working-directory=DIR   Startverzeichnis; mehrfach = ein Reiter je Verzeichnis
  -e, -x, --                    Rest der Zeile statt der Shell ausführen
                                (-e mit nur einem Argument: wie in einer Shell zerlegt)
  -p, --profile=ID              Profil (Dateiname ohne .toml)
  -t, --title=TITEL             Titel des Reiters
      --tab                     als Reiter im aktiven Fenster statt neuem Fenster
```

In English: `-d` sets the start directory (given several times, one tab per directory); `-e`,
`-x` or `--` run the rest of the line instead of the shell (a single argument after `-e`, as in
`parkett -e "htop -d 5"`, is split like a shell would, as in GNOME Terminal); `-p` selects a profile (file name
without `.toml`); `-t` sets the tab title; `--tab` opens a tab in the active window instead of
a new window.

Examples: `parkett -d ~/src --tab` opens a tab in `~/src`, `parkett -t Logs --
journalctl -f` a window with `journalctl` running. If Parkett is already running, the
running instance handles the call. The `.desktop` file declares the options for
`xdg-terminal-exec` (`X-TerminalArgExec`, `X-TerminalArgDir`, `X-TerminalArgTitle`).

## Nautilus

With `nautilus-python` (Arch: package `nautilus-python`, Fedora: `nautilus-python`), Parkett
adds two entries to the Nautilus context menu: *In Parkett öffnen* (Open in Parkett) for selected folders
(several folders = one window with one tab each) and *Parkett hier öffnen* (Open Parkett here) in the menu of the
current folder or on empty space. After installation, quit Nautilus once
(`nautilus -q`) so it loads the extension. The built-in *In Konsole öffnen* (Open in Console, Ctrl+.)
is hard-wired to GNOME Console in Nautilus and cannot be redirected.

## Session

On startup, Parkett opens the windows and tabs of the last session: layout,
profiles, directories and custom tab titles. Programs do not keep running; you get
shells in the old directories. The session is saved continuously (shortly after every change),
so it also survives logging out or a crash. Closing the last tab
ends the session, and the next start begins fresh. With options on the command line
(`-d`, `--`, `-p`, `-t`), Parkett opens only what was requested. Can be turned off under
*Einstellungen › Allgemein*; the file is `~/.local/state/parkett/session.json`.

## Light and dark

At the top of the main menu: *Systemstil folgen* (Follow system style), *Hell* (Light), *Dunkel* (Dark) (also under *Einstellungen ›
Allgemein › Stil* (Preferences › General › Style)). Every color scheme has a light and a dark variant; the terminals
switch along.

## Profiles

*Einstellungen › Profile* (Preferences › Profiles): color scheme (tiles with preview, applies immediately to open
terminals), background opacity (30–100 %; below that the desktop shows through, the
header bar stays opaque), font (otherwise the system monospace font), cursor shape,
blinking, bell
(sound and mark tab, mark only, off), command instead of shell, login shell, start directory, environment variables, behavior
when the process exits (default: close the pane on exit code 0, as in GNOME Console and Ptyxis;
otherwise a bar with *Neu starten* (Restart) and *Schließen* (Close), red only for errors).
New tabs with a different profile: the arrow next to *Neuer Reiter* (New tab).

Stored in `~/.config/parkett/profiles/<id>.toml`, editable by hand:

```toml
name = "Server"
palette = "nord"
command = ""             # empty = shell
login_shell = true
directory = "~/srv"      # empty = setting "Neue Terminals öffnen in" (Open new terminals in)
environment = ["EDITOR=vim"]
exit_action = "hold"     # close (default) | hold | restart
font = "JetBrains Mono 11" # empty = system font
cursor_shape = "ibeam"   # block | ibeam | underline
cursor_blink = "system"  # system | on | off
bell = "mark"            # sound | mark | none
opacity = 0.9            # 0.3–1.0
```

## Color schemes

14 schemes are included. *Importieren …* (Import …) on the profile page imports Ptyxis palettes
(`.palette`) and Tilix schemes (`.json`); they end up in `~/.local/share/parkett/palettes/`.
The format is the same as Ptyxis (full example: `data/palettes/tango.palette`): section
`[Palette]` with `Name=`, then `[Light]` and `[Dark]`, each with `Foreground`, `Background`,
`Color0` to `Color15` and optionally `Cursor`, `SuperuserBackground` (panes running as root) and
`RemoteBackground` (SSH). Like Ptyxis, Parkett derives missing context colors from the palette.

If a Tilix scheme has only one variant, Parkett uses it for both light and dark and says so
during import.

## Templates

A template stores the layout of a tab with profile, start command and subdirectory
per pane. *Aktuellen Reiter als Vorlage speichern …* (Save current tab as template …; arrow next to *Neuer Reiter* or command palette)
picks up the running commands; *Einstellungen › Vorlagen* (Preferences › Templates) edits them. Open them via
the same menu or the command palette (`#`). Without a fixed directory, Parkett asks when opening.

Start commands are **typed** into the shell, not launched instead of the shell: after Ctrl+C
the pane remains usable as a shell.

`~/.config/parkett/templates/<id>.toml`:

```toml
name = "Webprojekt"
directory = ""            # empty = ask when opening

[layout]
type = "split"
direction = "horizontal"  # horizontal = side by side, vertical = stacked
ratio = 0.5

[layout.first]
type = "pane"
name = "Editor"
command = "nvim ."

[layout.second]
type = "split"
direction = "vertical"
ratio = 0.5

[layout.second.first]
type = "pane"
name = "Dev-Server"
command = "npm run dev"

[layout.second.second]
type = "pane"
profile = ""              # empty = default profile
subdir = "tests"
```

## Settings without a user interface

Everything under *Einstellungen › Allgemein* is stored in GSettings:

```bash
gsettings list-recursively de.linuxundich.Parkett
```
