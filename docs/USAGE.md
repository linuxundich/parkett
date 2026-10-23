# Usage

The interface is available in English and German (it follows the system language). Labels below
are given in German with their English meaning; the English interface uses the English terms.

As of 0.4.0. All shortcuts can be changed or disabled under *Einstellungen › Kürzel*
(Preferences › Shortcuts); Ctrl+? shows the current bindings.

## Tabs and panes

- **Split:** the *Rechts teilen* (Split right) and *Unten teilen* (Split down) buttons on the left
  of the header bar, Ctrl+Alt+R, Ctrl+Alt+D or Ctrl+Alt+A (splits along the longer side). If there
  is not enough room (less than 20 × 5 characters per half), Parkett says so instead of splitting.
- **Move focus:** Alt+arrow; **move divider:** Shift+Alt+arrow;
  **equalize sizes:** Ctrl+Alt+= or a double-click on a divider; **maximize pane:** Ctrl+Shift+X.
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
  The label follows a template below that switch, default `{n} {program} · {dir}`. Placeholders:
  `{n}` number, `{program}` program (over SSH „ssh host“), `{dir}` last folder, `{path}` full
  path with `~`, `{host}` host of an SSH session, `{title}` the shell's title, `{profile}`
  profile. Empty values drop out together with their separator (` · `, ` | `, ` - `, `: `).
  A profile can have its own template (profile editor, *Reiter*; `tab_label` in the TOML file),
  which wins over the setting. The window title has its own template (*Vorlage für den
  Fenstertitel*, default `{title}`) with the same placeholders, e.g. `{n}: {title}`.
- **Moving panes:** drag a pane's title (floating title: move the pointer to the top of the pane
  first; or the title row) onto another pane of the same tab. Near an edge the pane is placed on
  that side, half the target's size; in the middle both panes swap. The zone shows while dragging.
  Dropped on a tab in the tab bar (also in another Parkett window), the pane moves to that tab,
  next to its active pane; the program keeps running.
- **Input to all panes** (Ctrl+Alt+I, also in the context menu and the command palette): what you
  type or paste in one pane of the tab goes to all its panes, like Tilix's "synchronize input".
  Receiving panes get an orange line at the top, the tab an icon (clicking it turns the mode
  off). *Dieser Bereich empfängt gemeinsame Eingabe* (This pane receives shared input) in the
  context menu leaves a single pane out. The mode is not saved in the session.
- **Narrow windows** (below 600 px, down to 360 px): the split and search buttons leave the header
  bar; splitting stays in the menu of the arrow next to *Neuer Reiter*, both stay on their
  shortcuts.
- **New window from a tab:** drag the tab out of the tab bar and drop it outside the window;
  dropping it on another Parkett window's tab bar moves it there.
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
`#` templates only, `~` bookmarks only. Each action shows its shortcut. Enter runs the selected
entry; Ctrl+Enter on a profile or bookmark opens it in a new pane of the current tab instead,
Alt+Enter on a bookmark uses the active pane (`cd` for a folder). The palette, the layout mode, the
templates and the bookmarks are also in the main menu.

**Bookmarks:** *Aktuellen Ordner als Lesezeichen* (Current folder as bookmark; palette or main
menu) stores the active pane's folder. Enter on a bookmark opens a new tab there. Besides
folders, a bookmark can be a command or an SSH host (like Tilix): it is typed into the shell of
the new tab, the new pane or the active pane, so the shell stays usable after the command or
the connection ends. They live in `~/.config/parkett/bookmarks.toml`; *Einstellungen ›
Lesezeichen* (Preferences › Bookmarks) renames, sorts and removes them and adds folders,
commands and SSH hosts (the *+* button), or edit the file by hand:

```toml
[[bookmark]]
name = "Projekte"
path = "~/05_Projekte"

[[bookmark]]
name = "Logs"
command = "journalctl -f"
path = "/var/log"        # optional: start folder

[[bookmark]]
name = "bender"
host = "-p 2222 toff@bender"   # everything after "ssh"
```

## Terminal

| What | Shortcut |
|---|---|
| Copy / Paste | Ctrl+Shift+C / Ctrl+Shift+V, Shift+Insert |
| Paste special | Ctrl+Shift+Alt+V: edit the text first, convert tabs to spaces, Windows line endings (CRLF) to LF |
| Copy as HTML | context menu (keeps colors and bold, e.g. for an e-mail or a document) |
| Search, previous/next match | Ctrl+Shift+F, Ctrl+Shift+G, Ctrl+Shift+H |
| Font larger/smaller/normal | Ctrl++, Ctrl+-, Ctrl+0 |
| Open link | Ctrl+click; right-clicking a link offers *Öffnen* (Open) and *Kopieren* (Copy) |
| Insert file paths | drag files from the file manager onto the terminal (quoted, not executed) |
| Save output, reset, read only, monitor | context menu › *Terminal*: *Ausgabe speichern …* (Save output…), *Zurücksetzen* (Reset), *Zurücksetzen und leeren* (Reset and clear), *Schreibgeschützt* (Read only: keys and pasting no longer reach the program, a lock shows at the bottom left), *Aktivität überwachen* (Monitor activity: notifies when the pane, while not in view, gets new output) and *Stille überwachen* (Monitor silence: notifies after 10 s without output) |
| Open a path from the output | Ctrl+click on `/…`, `~/…`, `./…` or `../…` (also `file.rs:42:7`): folders open in the file manager, files show selected in their folder; not over SSH |

The search bar shows how many matches the whole scrollback holds (up to „999+ Treffer“). The
count is taken when the search text or its options change, not while new output arrives.
Options: *Aa* case sensitive, *W* whole words only, *.\** regular expression.

**Copy on select** (*Einstellungen › Allgemein*, off by default): selected text goes to the
clipboard right away, as in Tilix; the primary selection (middle click) works either way.

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
      --split-right             aktiven Bereich des aktiven Fensters rechts teilen
      --split-down              aktiven Bereich des aktiven Fensters unten teilen
```

In English: `-d` sets the start directory (given several times, one tab per directory); `-e`,
`-x` or `--` run the rest of the line instead of the shell (a single argument after `-e`, as in
`parkett -e "htop -d 5"`, is split like a shell would, as in GNOME Terminal); `-p` selects a profile (file name
without `.toml`); `-t` sets the tab title; `--tab` opens a tab in the active window instead of
a new window; `--split-right` and `--split-down` split the active pane of the active window
instead (with `-d`, `-p` and a command as for a tab; without a window Parkett opens one).

Examples: `parkett -d ~/src --tab` opens a tab in `~/src`, `parkett -t Logs --
journalctl -f` a window with `journalctl` running. If Parkett is already running, the
running instance handles the call; the new shell then gets the caller's environment (for
example an activated Python venv), except variables of the caller's own terminal, and display
and session variables from Parkett. The `.desktop` file declares the options for
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

Selected text keeps its color and gets a background in the GNOME accent color (*Einstellungen ›
Darstellung* in GNOME), like text selection in other GTK apps; before, VTE drew the selection
with swapped colors, a bright block in dark mode.

**High contrast:** while *Hoher Kontrast* (High Contrast) is on in GNOME's accessibility settings,
all panes use the color scheme *Hoher Kontrast* instead of the scheme of their profile. The
profile keeps its scheme. Can be turned off under *Einstellungen › Allgemein* (*Hohen Kontrast des
Systems übernehmen*).

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
scrollbar = true         # thin overlay scrollbar at the right edge
bold_is_bright = false   # draw bold text in the bright colors 8–15
line_spacing = 1.2       # 1.0–2.0, also letter_spacing
scroll_on_output = false # jump to the end on new output
scroll_on_keystroke = true
hosts = ["bender", "*.tuxedo.de"]  # use automatically for these SSH hosts
directories = ["~/kunde"]          # … and for these local folders (also below)

[[links]]                # custom link, see below
pattern = '\bPARK-(\d+)\b'
target = "https://tracker.example/issue/$1"
```

**Custom links** (profile editor, *Eigene Links*; like Tilix's custom hyperlinks): text matching
a regular expression (PCRE2, as in the search) becomes a link for Ctrl+click and the context
menu. The target is an address (with a scheme such as `https:`) or a command, run in the pane's
folder; `$0` is the whole match, `$1`…`$9` the groups. Output can come from anyone (`cat`, SSH),
so the groups are percent-encoded in addresses and quoted in commands, never inserted raw.
Example for an editor: pattern `([\w./-]+\.rs):(\d+)`, target `code --goto $1:$2`.

**Automatic profile:** under *Automatisch verwenden* (Use automatically) a profile lists hosts
and folders (`*` and `?` as wildcards). While a pane's SSH connection goes to such a host, or
its local folder lies in such a folder, the pane shows that profile's color scheme, font,
opacity and cursor; afterwards it returns to its own profile. Bell and exit behavior stay with
the pane's own profile, and the program keeps running. Host rules win over folder rules; the
tab tooltip names the automatic profile.

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
In the template editor, the buttons next to *Bereich N* (Pane N) split a pane to the right or
below or remove it, and *Angleichen* (Equalize) centers all dividers.

Start commands are **typed** into the shell, not launched instead of the shell: after Ctrl+C
the pane remains usable as a shell. They are typed once the shell is ready: at the first prompt with VTE's
shell integration, otherwise when the output has been quiet for a moment. While the terminal
waits for a password (ssh, sudo), Parkett waits too.

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
