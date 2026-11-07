# Switching from VTE to libghostty – analysis (2026-10-29)

Question: can Parkett replace VTE with libghostty without regressions?

**Answer: no, not today.** libghostty is not a VTE replacement but a much lower layer. A switch
would mean writing a terminal widget of our own (rendering, fonts, input, selection,
accessibility, PTY), and for some features nothing is there at all. Decision: stay with VTE,
check again when Ghostty ships a ready-made widget or a stable, versioned libghostty.

Basis: Ghostty `main` at commit `108d503` (2026-10-29), Parkett 0.5.0, VTE 0.84.1 on Arch.

## What libghostty is today

| Part | State | Usable for Parkett? |
|---|---|---|
| `libghostty-vt` (`include/ghostty/vt.h`) | C/Zig library: VT parser, terminal state, scrollback, reflow, render *state*, key/mouse/focus encoding, search, selection, formatter (text/VT/HTML), OSC callbacks | yes, but only as a core; the header itself says "incomplete, work-in-progress API … definitely going to change", no version tag yet |
| Rendering, fonts, GTK widget | Ghostty's own renderer (OpenGL/Metal, font stack) lives in `src/renderer`, `src/font` and is **not** exported | no – Parkett would have to write its own renderer |
| Full embedder API (`include/ghostty.h`, "libghostty-internal") | only consumer is the macOS app; `ghostty_platform_e` knows only `MACOS` and `IOS`; undocumented | no Linux support |
| PTY and spawning | not part of libghostty-vt (only an `on_pty_write` callback for replies) | write our own (forkpty, environment, `setsid`, flatpak-spawn later) |
| Rust bindings `libghostty-rs` (third party, MIT) | safe wrappers for Terminal, RenderState, encoders; pinned to a Ghostty commit, pre-1.0 | yes, but needs Zig 0.17 at build time and fetches Ghostty source in `build.rs` |

Known GTK4 terminals on libghostty-vt (awesome-libghostty, e.g. *forgetty*) are young single-author
projects; none offers a reusable widget comparable to `VteTerminal`.

## What Parkett uses from VTE

`app/src/terminal.rs` (1861 lines) and `pane.rs` use about 70 VTE calls. Grouped, with what
libghostty-vt offers:

| Parkett feature | VTE API today | libghostty-vt | Effort / risk |
|---|---|---|---|
| Drawing text, colors, cursor, bold-is-bright, cell spacing, blink mode, transparency | `set_colors`, `set_color_*`, `set_font`, `set_cell_*_scale`, `set_cursor_*`, `set_text_blink_mode`, `set_bold_is_bright` | render state with dirty tracking only; drawing, glyph cache, Pango/HarfBuzz shaping, emoji, wide chars, font fallback, HiDPI are ours | **very high** – this is the bulk of VTE |
| Keyboard input incl. IME (dead keys, Compose, IBus) | built into the widget | key encoder (incl. Kitty protocol); `GtkIMContext` wiring is ours | high, regressions likely with input methods |
| Mouse: selection, double/triple click, copy on select, mouse reporting | built in, `connect_selection_changed` | selection + gesture helpers, mouse encoder | high |
| Scrolling, scrollbar (`ScrolledWindow` around VTE), scroll on output/keystroke | `vadjustment`, `GtkScrollable` | scrollback in the state; implementing `GtkScrollable` is ours | medium |
| Spawn, PTY, exit bar, foreground process (`tcgetpgrp`), echo check (`tcgetattr`) | `spawn_async`, `pty()`, `connect_child_exited` | nothing | medium (the logic after the fork stays) |
| Title, working directory, preexec time (vte.sh termprops) | `termprop_string`, `ref_termprop_uri`, `connect_termprop_changed` | OSC 0/2, OSC 7, OSC 133 callbacks | low–medium; **but** `vte.sh`/`vte.cwd` from distributions' `/etc/profile.d/vte.sh` would no longer apply – Ghostty's own shell integration or OSC 133 would have to be injected |
| Search with regex, whole word, wrap, match count | `search_set_regex`, `search_find_*`, `glib::Regex` | plain string search; regex would have to run over formatted text | medium, regex search regresses first |
| Links: URLs, file paths, custom links (regex), OSC 8 tooltip and Ctrl+click | `match_add_regex`, `check_match_at`, `check_hyperlink_at`, `hyperlink_hover_uri` | OSC 8 in the cell data; regex matching and hover are ours | medium |
| Copy as HTML, save output, copy text | `text_range_format(Html)`, `write_contents_sync` | formatter (text/VT/HTML) – good | low |
| Paste, paste special, file drop, read-only, synchronized input | `paste_text`, `feed_child`, `set_input_enabled`, `connect_commit` | paste encoding (bracketed paste) | low–medium |
| Bell, activity/silence monitor | `connect_bell`, `connect_contents_changed` | callbacks / own counters | low |
| Context menu | `set_context_menu_model` | ours (plain GTK) | low |
| **Accessibility** (screen readers; `build-aux/vm/atspi.py` reads terminal text) | VTE implements `GtkAccessibleText` | nothing; Ghostty's own GTK app has no accessible text either | **high, open-ended** – a regression for screen reader users and for our smoke tests |
| Sixel (decided: wait for distribution VTE) | `set_enable_sixel` | no Sixel; Kitty graphics instead | changes the feature, not a pure regression |

## Gains

- Speed and memory under heavy output (Ghostty's core is fast; VTE is slower with huge scrollback).
- Kitty keyboard protocol, Kitty graphics, synchronized output, modern OSCs (9;4 progress,
  OSC 52 read) – VTE supports only part of this.
- Snapshots of terminal state (`snapshot.h`) could later make session restore with content
  possible.

None of these is something Parkett users have asked for; VTE covers everyday use.

## Costs and risks

- Own renderer + font stack: the hardest part of a terminal; months of work and a long tail of
  bugs (CJK, emoji, ligatures, fallback, fractional scaling) that VTE solved years ago.
- Unstable API: every Ghostty commit may break the C signatures; no tagged version, so pinning a
  commit and following along by hand.
- Build and packaging: Zig 0.17 as build dependency (in Arch `extra`, not in Debian stable or
  Fedora in that version), static linking of a pinned Ghostty commit; distribution packagers dislike
  vendored source fetched in `build.rs`.
- Loss of accessibility and of the AT-SPI based smoke tests.
- Ecosystem: `vte.sh` integration, GNOME conventions (Ptyxis, Console share VTE fixes).

## Recommendation

1. **Do not switch now.** No regression-free path exists; the renderer and accessibility alone
   rule it out.
2. **Keep the seam small:** `terminal.rs` already wraps VTE in `Terminal`; keep new VTE calls inside
   it so a later backend switch stays local.
3. **Check again when** one of these happens: Ghostty tags libghostty with a stable API, a
   reusable GTK4 widget on libghostty appears (with accessible text), or Ghostty exports its
   renderer for Linux embedders. Optional small experiment until then: a headless spike with
   `libghostty-rs` that feeds recorded output into libghostty-vt and compares title/cwd/search
   results with VTE – no UI, one afternoon, tells us how the API feels from Rust.

Sources: Ghostty README and headers (`include/ghostty.h`, `include/ghostty/vt.h`,
`include/ghostty/vt/terminal.h`, `render.h`), `src/apprt/gtk/class/surface.zig`,
github.com/Uzaaft/awesome-libghostty, github.com/Uzaaft/libghostty-rs.

## Update 2026-11-07: libghostty-vt 1.0 and the decision to build a second backend

libghostty-vt 1.0.0 is out: stable C ABI, semantic versioning, versioned ELF symbols, own
signed source tarballs with bundled dependencies (offline build), shared library with SONAME
and pkg-config file. That removes the "unversioned API" blocker. It does **not** remove the
others: still no renderer, PTY or accessibility (1.0 is the VT core only).

Decision (Christoph, 2026-11-07): switch anyway, as a **second backend behind a switch**
(Cargo feature plus setting), VTE stays the default until parity.

Facts found on the machine:

- Arch `extra/ghostty 1.4.0` ships `libghostty-vt.so.0.1.0` and headers, pkg-config reports
  `0.1.0-dev`, header still says "work in progress". That is the **pre-1.0** API; bindings must
  be generated against the 1.0.0 headers (source tarball from ghostty-org/ghostty), not against
  this one. Zig 0.17 is installed but not needed with the system library.
- No `bindgen` installed (clang is); `libghostty-rs` is an alternative to own FFI.

Phases (each testable, each behind the switch):

1. FFI crate `ghostty-vt` (needed subset: terminal, render state, key/mouse encoder, formatter,
   search, selection) + headless tests (feed recorded output, compare title/cwd/text with VTE).
2. PTY and spawn in `core` without VTE (forkpty, env, `setsid`, resize, foreground process).
3. GTK4 widget `GhosttyView`: snapshot rendering with Pango (cells, colors, cursor, wide chars,
   scrollbar via `GtkScrollable`), input via `GtkIMContext`, key/mouse encoding.
4. Selection, clipboard, copy as HTML, links (URL, paths, custom regex, OSC 8), search.
5. Wire into `Terminal` (behind a trait), context menu, all profile settings, exit bar.
6. Accessibility (`GtkAccessibleText`), then parity audit against the smoke tests; only then
   discuss making it the default.

### Building libghostty-vt 1.0.0 locally (until distributions ship it)

```sh
B=https://release.files.ghostty.org/libghostty-vt/1.0.0
curl -LO $B/libghostty-vt-1.0.0.tar.gz -O $B/libghostty-vt-1.0.0-deps.tar.gz
tar xzf libghostty-vt-1.0.0.tar.gz && tar xzf libghostty-vt-1.0.0-deps.tar.gz
cd libghostty-vt-1.0.0 && ZIG_GLOBAL_CACHE_DIR=$(mktemp -d) zig build --prefix ~/.local/ghostty-vt \
  --system ../libghostty-vt-1.0.0-deps -Doptimize=ReleaseFast -Dcpu=baseline   # needs Zig 0.17
PKG_CONFIG_PATH=~/.local/ghostty-vt/lib/pkgconfig LD_LIBRARY_PATH=~/.local/ghostty-vt/lib \
  cargo test -p ghostty-vt-sys -p ghostty-vt
```

The library is installed in `~/.local/ghostty-vt` on Christoph's machine.

Phase 1 status: `ghostty-vt-sys` (raw bindgen output from the 1.0.0 headers, `build.rs` demands
`libghostty-vt >= 1.0.0` via pkg-config; excluded from `default-members` so normal builds and
`build-aux/ci.sh` do not need it). The hello test (text, OSC 2 title, OSC 7 cwd) passes.

Phase 1 also has the safe crate `ghostty-vt` (`Terminal`: write, resize, text, title, pwd, cursor,
size); 3 tests (reflow, scrollback, UTF-8 split across writes) pass.

Phase 2: `core/src/pty.rs` (`Pty::spawn` with `SpawnOptions`: argv, cwd, env additions or clean env,
size; `read`/`write_all`, `resize`, `foreground_pgid`, `tty_mode`, `try_wait`, `hangup`). The child is
started through `Command` + `pre_exec(setsid, TIOCSCTTY)`, not a raw fork. `TERM=xterm-256color` and
`COLORTERM=truecolor` are defaults (VTE sets the same). `ghostty-vt/tests/pty_roundtrip.rs` runs
PTY → terminal state headless. Not yet covered: spawning inside Flatpak (`flatpak-spawn`), the
exit-bar/restart logic, and the `vte.sh` shell integration (VTE injects `VTE_VERSION`; the new
backend needs OSC 7/133 from the shell itself).

Phase 3a: `ghostty-vt/src/render.rs`. `RenderState::frame(&Terminal) -> Frame` copies one screen: per
cell the grapheme as UTF-8, resolved fg/bg (`None` = frame default), flags (bold, italic, faint,
blink, inverse, invisible, strike, overline), underline kind, `selected`, `wide`/`spacer`; plus
default colors, 256 palette entries, cursor (shape, blinking, hidden when scrolled away). It is a full
copy per call and cleans the library's dirty state; per-row dirty tracking (`next_dirty`) and
row identity are for when drawing is measured. Next is 3b, the GTK widget.

Phase 3b: `ghostty-view` (GTK4 widget `GhosttyView`, `examples/demo.rs`).

- Drawing: per frame a `Frame` copy; background rectangles per cell, text in runs of equal style
  (one Pango layout per run, wide characters alone so fallback fonts cannot shift the line), block
  cursor by inverting the cell, hollow cursor without focus, bar/underline shapes.
- Input: `EventControllerKey` with an `IMMulticontext` (IME first), then `ghostty_vt::input` – the
  library's key encoder fed with the *physical* key (GDK code − 8 = evdev, table in `input.rs`),
  modifiers and the text of the key. Cursor-key mode and Kitty keyboard flags follow the terminal.
  Terminal replies (DA, cursor position report) go back through the `write_pty` callback.
- Wheel scrolls the scrollback by 3 rows; typing jumps back to the bottom.
- The PTY fd is watched with `glib_unix::fd_source_new` (glib 0.22 has no fd source itself).
- Verified headless: `dbus-run-session -- build-aux/ghostty-demo-headless.sh target/debug/examples/demo`
  with `DEMO_CMD`, `DEMO_INPUT`, `DEMO_SHOT=file.png`: colors, bold, underline, inverse, CJK, umlauts,
  `ls --color`, a shell with 40 lines scrolling, title via OSC 2, grid size 102x24 matched the window.
  Real key events (Ctrl, arrows, IME) are **not** tested yet; only the encoder is (unit test).

Not done in 3b: selection and clipboard, links, search, mouse reporting, paste, blinking text and
cursor, scrollbar, per-row dirty drawing, HiDPI/fractional-scaling check, font from settings.

Phase 4a: selection, clipboard, paste, mouse.

- `ghostty_vt::edit`: the library keeps the active selection (`OPT_SELECTION`); we set it from
  cells (`select_range`, rectangle with Alt), `select_word`, `select_line`, `select_all`, and read it
  back as plain text or HTML through the formatter (`selected_text`, soft wraps joined, trailing
  blanks trimmed). `Terminal::paste` uses `ghostty_terminal_paste`, so bracketed paste (mode 2004)
  and the "unsafe text" check come from the library (`PasteResult::Unsafe`).
- `ghostty_vt::input::MouseEncoder`: pixel position to cell report in the format the program chose
  (X10 … SGR). The widget reports when the program tracks the mouse and Shift is not held; with
  Shift the user selects as usual. Wheel becomes buttons 4/5 while tracking.
- Widget: click selects (double = word, triple = line), drag extends, releasing copies to the
  primary selection, middle click pastes it, Ctrl+Shift+C/V/A copy/paste/select all,
  `GhosttyView::copy(html)`, `paste_text`, `connect_unsafe_paste`.
- Fixes found by the headless screenshots: cell width is measured from ten drawn digits, background
  spans are merged (no hairlines between cells) and drawn before the text of the row.
- Note for comparisons: readline shows pasted text in reverse video, so a selected pasted line looks
  "unselected" – correct behavior, not a bug.
- Not tested: real pointer and key events through GDK (needs mutter RemoteDesktop, see the memory
  note on headless keys); the mouse encoder and selection are unit-tested, the paths in the widget
  are exercised only via the public API in the demo (`DEMO_SELECT`, `DEMO_PASTE`).

Phase 4b: links and search.

- `ghostty_vt::find`: `screen_lines()` (every row of scrollback + screen as text, via the
  formatter with a one-row selection), `char_columns()` (column of each character, wide = 2, with the
  library's own width function), `scrollbar()`, `select_screen_range`, `scroll_to_row`,
  `hyperlink_at` (OSC 8).
- `ghostty_view::links`: URL, mail and path patterns copied from the VTE version (PCRE2 through
  `glib::Regex`, so lookbehind and the profile's custom link patterns behave as before); custom
  links expand with `parkett_core::link::expand`. OSC 8 wins over detected text and shows its real
  target as a tooltip. Hovering underlines the link and switches the cursor; Ctrl+click calls
  `connect_link_activated(Link::{Uri, Path, Custom})` – scheme checks, the file manager and command
  launching stay with the host app, like `open_link`/`open_path` today.
- `ghostty_view::search`: `GhosttyView::search(pattern, case_sensitive)` (PCRE2; the host builds
  whole-word and plain patterns as `SearchQuery::pattern` does), `search_step(up)`, count, index;
  hits are highlighted (orange = current), selected and scrolled into view. The library's own
  search was not used because it is plain text, ASCII-case-insensitive only.
- Verified headless with `DEMO_SEARCH` and `DEMO_LINK`: 10 hits for `error E1\d`, step to the second hit
  scrolled to it and highlighted it; links at pixel positions returned URL (trailing comma dropped),
  `~/dir/a.rs:3` as path, `PARK-12` as custom link and the OSC 8 target.
- Limits: links and hits do not cross a soft wrap (per row); hits are computed when the search is
  set and not refreshed while output arrives; the hover underline and the pointer cursor are not
  tested with real pointer motion.

Phase 5: wiring into Parkett – plan and state.

5a (done, behavior-preserving): `Terminal::vte()` was used in 11 places outside `terminal.rs`. They
now call `Terminal::{focus_widget, owns_focus_widget, grab_focus, connect_focus_gained,
connect_cursor_moved, char_size, connect_char_size_changed, cursor_on_top_row, selected_text,
activate_action}`; tab compares terminals by `uid()`. `build-aux/ci.sh` exits 0.

What 5b has to cover inside `terminal.rs` (about 110 call sites, grouped by what a backend must
offer; `GhosttyView` has the first column, the right column is missing):

| Group | VTE calls today | GhosttyView today | Missing |
|---|---|---|---|
| Spawn, PTY, exit | `spawn_async`, `pty`, `connect_child_exited`, `feed_child` | `start`, `connect_exited`, `send_input` | pid for `/proc` cwd and foreground process (`Pty::foreground_pgid`), tty mode for the startup command, flatpak-spawn |
| Title, directory | `termprop_string`, `ref_termprop_uri`, `connect_termprop_changed` | `title`, `pwd`, `connect_title_changed` | `vte.shell.*` termprops: command start/end, preexec time → OSC 133 or own shell integration |
| Look | `set_colors`, `set_font`, cursor shape/blink, bold-is-bright, cell scale, scrollback, blink mode, sixel, highlight colors | `set_font` | palette/colors from the profile, cursor options, scrollback size, cell scale, opacity |
| Input | `connect_commit`, `set_input_enabled`, `paste_text` | key, paste | commit callback for synchronized input, read-only |
| Selection, copy | `copy_clipboard_format`, `text_range_format`, `write_contents_sync`, `connect_selection_changed` | selection, copy text/HTML | save output to file, selection-changed callback |
| Search, links | `search_*`, `match_add_regex`, `check_match_at`, hyperlink hover | search, links | wrap-around/count limit parity (`SEARCH_COUNT_MAX`), `connect` for hover tooltip |
| Menu, drop, misc | `set_context_menu_model`, drop target, bell, `connect_contents_changed`, `vadjustment`, scrollbar | – | context menu popup, bell, contents-changed (activity/silence monitor), vadjustment-like scroll API, scrollbar |

Approach (recommended): a private `Backend` enum in `app/src/backend.rs` (`Vte(vte::Terminal)` and, with
Cargo feature `ghostty`, `Ghostty(GhosttyView)`) with exactly the methods of the table; every
`self.0.vte.x()` in `terminal.rs` becomes `self.0.backend.x()`. The VTE arm keeps today's code
line for line, so the default path stays as tested; the Ghostty arm fills in column 3 step by
step (unsupported options log once and do nothing). Selection by `PARKETT_BACKEND=ghostty` (and later
a preferences switch) at startup, per window, not per pane.

5b (done, behavior-preserving): `app/src/backend.rs` is the only file that names VTE. `Backend`
(clone, weak handle, derefs to `gtk::Widget` for focus/controllers/actions/clipboard/drop) offers:
`apply(&Look)` (colors, font, cursor, spacing, scrolling, bell, sixel in one call), `spawn(&SpawnSpec)`,
`connect_child_exited/bell/commit/selection_changed/contents_changed/shell_event(ShellEvent::{Title,
Cwd, Preexec, Precmd})`, `feed`, `paste`, `foreground_pgid`, `tty_mode`, `title`, `cwd_uri`,
selection/copy (`copy(html)`), links (`add_match`, `match_at`, `hyperlink_at`), search (`set_search`,
`search_step`, `count_matches`), `char_size`, `cursor_on_top_row`, font scale, reset, `write_contents`,
context menu, read-only. `sixel_supported()` and `user_shell()` are free functions there.

Checked: `build-aux/ci.sh` exit 0; a private headless session started `parkett -e …` through the
new `spawn` (marker file written, `stty size` = 30 100). Not rerun: the VM smoke test
(`build-aux/vm/smoke.sh`), which is the real regression check for 5b – do it before a release.

For 5c: turn `Backend` into an enum (`Vte`, `Ghostty` behind the `ghostty` feature). The compiler
then lists every method that needs a Ghostty arm; the `Deref<Target = gtk::Widget>` stays (both
widgets are `gtk::Widget`s). Methods `GhosttyView` still lacks are in the table above.

5c (done): second backend and switch.

- `Backend` is now an enum: `Vte(vte::Terminal)` and, with the Cargo feature `ghostty`,
  `Ghostty(ghostty_view::GhosttyView)`. Every method matches on both; VTE arms are the code from 5b.
  `Backend::new` picks the engine: environment variable `PARKETT_BACKEND` (`ghostty`/`vte`), else the
  GSettings key `terminal-backend` (default `vte`; no preferences row, applies to terminals opened
  afterwards). Without the feature a request for `ghostty` logs a warning and uses VTE.
- Build and run: `cargo build -p parkett --features ghostty` needs libghostty-vt 1.0 on
  `PKG_CONFIG_PATH`/`LD_LIBRARY_PATH` (see the build recipe above); default builds and `ci.sh` do not.
- `GhosttyView` gained what the app needs: `Look` (colors incl. the 16 profile colors over the
  library's 256 palette, background opacity, selection color, font, line spacing, cursor, scrolling,
  bell, scrollback), bell/commit/selection/contents/cursor/char-size/shell-event signals (shell marks
  come from OSC 133, directory from OSC 7), process id, foreground process group, tty mode, exit status,
  host link patterns (`add_pattern`, `pattern_at`), OSC 8, match counting, read-only, font zoom, context
  menu, reset, padding like VTE's CSS, CSS name `parkett-terminal`.
- `ghostty-vt`: `on_bell`, `on_prompt` (OSC 133), `set_colors`, `set_scrollback_lines`,
  `set_default_cursor`, `reset`; one userdata block (`Callbacks`) for all C callbacks.
- Verified headless (private mutter session, debug-only hook `PARKETT_DEBUG_SHOT=file.png:ms`):
  Parkett starts with the libghostty engine, header bar and tab as usual, profile colors, OSC 2 title in
  the window title, cursor, padding; the same command under VTE looks the same apart from 1 px row
  height and bold weight. `ci.sh` and Clippy (with and without the feature) are green.

Not done in 5c (next, 5d and later):

- Scrollbar (the widget scrolls by wheel only; `scroller` is bypassed), cursor/text blinking,
  bold-is-bright, letter spacing, Sixel/Kitty images (the library has Kitty graphics, nothing draws it).
- Shell integration: VTE gets `vte.cwd`/`vte.shell.*` from distribution's `vte.sh`; the new engine needs
  OSC 7 and OSC 133 from the shell itself (Ghostty's shell-integration package, or a snippet Parkett
  injects) – without OSC 133 no *command finished* notification and no waiting for the first prompt
  (the startup command falls back to the quiet-output timer).
- Flatpak (`flatpak-spawn`), exit bar behavior after a restart (the old state is replaced; scrollback
  starts fresh), drag-and-drop file paste works through the shared drop target, copy on select
  uses the primary selection path of the widget plus `copy-on-select`.
- Real pointer and keyboard events (Ctrl+click, IME, dead keys) are untested; the VM smoke test
  (`build-aux/vm/smoke.sh`) has not been run, with either engine.
- Accessibility (`GtkAccessibleText`), HiDPI/fractional scaling, performance numbers against VTE.
- Preferences row and German translation of the new setting (`po/update-pot.sh`).

5d (done): scrollbar and shell integration.

- Scrollbar: `GhosttyView` implements `GtkScrollable` (properties `hadjustment`, `vadjustment`,
  `hscroll-policy`, `vscroll-policy`). Terminal puts it into the same `GtkScrolledWindow` as the VTE
  widget, so the overlay bar, its policy per profile (`scrollbar`) and dragging work the same. The
  adjustment mirrors scrollback + screen rows (value = first visible row) and is updated once per
  drawn frame. First attempt, a separate `GtkScrollbar` in an overlay, drew nothing under libadwaita
  although it was allocated (slider invisible even with a red background rule); do not retry it.
  `examples/demo.rs` prints the adjustment to check it.
- Shell integration: the Ghostty arm of `Backend::spawn` sets `VTE_VERSION` (from the linked VTE) like VTE
  does, so the distribution's `/etc/profile.d/vte.sh` switches on in login shells. It prints OSC 7
  (directory), OSC 133 A/B/D (prompt marks) and OSC 666 `vte.shell.preexec/precmd`; libghostty-vt
  reports 133 itself and hands unknown OSC to `Terminal::on_unknown_osc`, which the widget turns into
  `ShellEvent::Preexec/Precmd`. So the first-prompt hook, *command finished* notifications and the
  directory in the title work as with VTE. Checked headless: `bash -l` showed `user@host:/usr` and the
  directory in the subtitle after `cd /usr`. Shells without `vte.sh` (not bash/zsh, or no profile.d)
  stay without marks; Ghostty's own integration scripts would be the next step there.
- `ghostty-vt::on_unknown_osc` (limit 2048 bytes), a test for it.

Still open after 5d: letter spacing, Sixel/Kitty images; Flatpak spawn;
HiDPI/fractional scaling; Ctrl/arrows/IME/dead keys and pointer events through GDK; preferences
performance against VTE; soft-wrapped links/search;
`Pty`: environment `COLORTERM`/`TERM` come from `Pty` (VTE sets the same).

Preferences: *Behaviour › Terminal engine* (VTE / libghostty, experimental) appears only in builds with the
`ghostty` feature; it writes `terminal-backend`, which `PARKETT_BACKEND` still overrides. German texts are in
`po/de.po`.

VM smoke test (2026-11-07, Fedora 44): default VTE engine 15 of 17 checks pass, the 2 failures (F10/htop,
QA-02 screenshot) are identical on the 0.5.0 release. Not run for the libghostty engine.

VM smoke test of the libghostty engine (2026-11-07, Fedora 44): `PARKETT_FEATURES=ghostty
PARKETT_ENV=PARKETT_BACKEND=ghostty build-aux/vm/smoke.sh` after copying `~/.local/ghostty-vt` to the VM
(deploy.sh fixes the `.pc` prefix there and links with an rpath). Result 14/17: the two failures of the
VTE engine and of 0.5.0 plus QA-09, the accessibility gap (focused object is a panel, not a terminal).
The libghostty-vt build from Arch's glibc runs on Fedora 44 (glibc 2.43). Plain keys through real GDK events
work (`htop` typed by `virsh send-key`); Ctrl/arrows/IME/dead keys and pointer events are still untested.

Accessibility (done, tested in the VM, not with Orca's speech): `GhosttyView` has the role *Terminal*, the label
*Terminal* and implements `GtkAccessibleText`. `ghostty-view/src/a11y.rs` (pure, unit-tested) builds a
`Snapshot` of the visible rows from the last drawn frame: lines without trailing blanks joined by `\n`,
offsets in characters, caret from the cursor cell, selection from the selected cells, units
(character, word, line) for `contents_at`, cell columns for `extents`/`offset` (wide characters count
two columns). Each drawn frame is compared with the one before (`a11y::diff`: one remove/insert per changed
row, everything at once when more than 4 rows change, e.g. after scrolling) and GTK is told through
`update_contents`, `update_caret_position`, `update_selection_bound`. Text attributes (color, bold) and
`set_caret_position`/`set_selection` (GTK 4.22+) are not provided. VM smoke test of the libghostty engine:
15 of 17 (QA-09 now passes); `atspi.py texts` returns the pane text.

Blinking and bold-is-bright (done): `GhosttyView` runs one blink timer (half of GTK's `gtk-cursor-blink-time`) only
while the pane has the focus and the cursor blinks or a visible cell has SGR 5 – an idle or unfocused pane has
no timer (QA-14). Cursor: the profile forces it on or off, *System* follows the program (DECSCUSR) and GTK's
`gtk-cursor-blink`; the phase restarts at "shown" on typing and cursor movement. Blinking text is hidden in the
off phase. Bold-is-bright: `Cell::fg_index` carries the palette index, bold cells with index 0–7 take 8–15
(profile option `bold_is_bright`). Checked headless: blinking text visible/hidden/visible at 0.3/0.9/1.5 s,
cursor with `ESC[1 q` likewise; Parkett with `bold_is_bright = true` draws (246,97,81) instead of (192,28,40) for
bold red. Not checked: the system setting `gtk-cursor-blink = false` and the 10 s blink timeout of GTK.
