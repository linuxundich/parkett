// SPDX-License-Identifier: GPL-3.0-or-later
//! The one place that talks to the terminal widget (VTE today). `terminal.rs` and the panes only
//! use this interface, so a second implementation (libghostty, see docs/LIBGHOSTTY.md) can sit
//! beside it. Methods take and return plain types; the only thing the VTE arm exposes besides
//! them is the `gtk::Widget` it derefs to (focus, controllers, actions, clipboard, drop targets).

use std::os::fd::AsRawFd;

use gtk::glib::translate::{from_glib_full, ToGlibPtr};
use gtk::prelude::*;
use gtk::{gdk, gio, glib, pango};
use parkett_core::profile::{CursorBlink, CursorShape};
use vte::prelude::*;

/// PCRE2 flags like Ptyxis: UTF, NO_UTF_CHECK, UCP and MULTILINE (required by VTE).
pub const PCRE2_FLAGS: u32 = 0x0008_0000 | 0x4000_0000 | 0x0002_0000 | 0x0000_0400;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;

/// Rows per block when counting matches (bottom to top).
const SEARCH_COUNT_ROWS: libc::c_long = 20_000;

/// Everything `apply_colors` decides, in one piece.
pub struct Look<'a> {
    pub foreground: gdk::RGBA,
    pub background: gdk::RGBA,
    pub palette: &'a [gdk::RGBA],
    pub cursor: Option<gdk::RGBA>,
    pub cursor_foreground: Option<gdk::RGBA>,
    pub highlight: gdk::RGBA,
    pub font: String,
    pub bold_is_bright: bool,
    pub line_spacing: f64,
    pub letter_spacing: f64,
    pub scroll_on_output: bool,
    pub scroll_on_keystroke: bool,
    pub cursor_shape: CursorShape,
    pub cursor_blink: CursorBlink,
    pub audible_bell: bool,
    pub sixel: bool,
}

/// What the shell tells the terminal (VTE termprops).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellEvent {
    Title,
    Cwd,
    /// A command starts (shell integration).
    Preexec,
    /// The prompt is back.
    Precmd,
}

/// What to start.
pub struct SpawnSpec<'a> {
    pub argv: &'a [&'a str],
    pub dir: Option<&'a str>,
    pub env: &'a [&'a str],
    /// Do not inherit Parkett's environment (call to a running instance, QA-08).
    pub clean_env: bool,
}

#[derive(Clone)]
pub enum Backend {
    Vte(vte::Terminal),
    #[cfg(feature = "ghostty")]
    Ghostty(ghostty_view::GhosttyView),
}

pub enum WeakBackend {
    Vte(glib::WeakRef<vte::Terminal>),
    #[cfg(feature = "ghostty")]
    Ghostty(glib::WeakRef<ghostty_view::GhosttyView>),
}

impl WeakBackend {
    pub fn upgrade(&self) -> Option<Backend> {
        match self {
            WeakBackend::Vte(w) => w.upgrade().map(Backend::Vte),
            #[cfg(feature = "ghostty")]
            WeakBackend::Ghostty(w) => w.upgrade().map(Backend::Ghostty),
        }
    }
}

impl std::ops::Deref for Backend {
    type Target = gtk::Widget;
    fn deref(&self) -> &gtk::Widget {
        match self {
            Backend::Vte(v) => v.upcast_ref(),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.upcast_ref(),
        }
    }
}

/// Which engine new terminals use: `PARKETT_BACKEND`, else the `terminal-backend` setting.
fn wants_ghostty() -> bool {
    let name = std::env::var("PARKETT_BACKEND")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| crate::settings::settings().string("terminal-backend").to_string());
    name == "ghostty"
}

impl Backend {
    pub fn new(scrollback: i64) -> Self {
        #[cfg(feature = "ghostty")]
        if wants_ghostty() {
            let g = ghostty_view::GhosttyView::new();
            // The patterns arrive through `add_match` like they do for VTE.
            g.set_builtin_links(false);
            g.set_scrollback_lines(scrollback);
            return Backend::Ghostty(g);
        }
        #[cfg(not(feature = "ghostty"))]
        if wants_ghostty() {
            glib::g_warning!("parkett", "libghostty backend requested, but this build has no `ghostty` feature");
        }
        let vte = vte::Terminal::builder()
            .hexpand(true)
            .vexpand(true)
            .allow_hyperlink(true)
            .pointer_autohide(true)
            .bold_is_bright(false)
            .enable_a11y(true)
            .build();
        vte.set_size(100, 30);
        // Setter instead of builder: only it accepts -1 (unlimited)
        vte.set_scrollback_lines(scrollback);
        Backend::Vte(vte)
    }

    pub fn downgrade(&self) -> WeakBackend {
        match self {
            Backend::Vte(v) => WeakBackend::Vte(v.downgrade()),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => WeakBackend::Ghostty(g.downgrade()),
        }
    }

    /// Is `w` inside a terminal widget (or one itself)?
    pub fn is_terminal_widget(w: &gtk::Widget) -> bool {
        #[cfg(feature = "ghostty")]
        if w.ancestor(ghostty_view::GhosttyView::static_type()).is_some() {
            return true;
        }
        w.ancestor(vte::Terminal::static_type()).is_some()
    }

    // ---- settings -----------------------------------------------------------------------

    pub fn set_scrollback_lines(&self, lines: i64) {
        match self {
            Backend::Vte(v) => v.set_scrollback_lines(lines),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.set_scrollback_lines(lines),
        }
    }

    pub fn apply(&self, l: &Look) {
        match self {
            Backend::Vte(v) => {
                let refs: Vec<&gdk::RGBA> = l.palette.iter().collect();
                v.set_colors(Some(&l.foreground), Some(&l.background), &refs);
                v.set_color_cursor(l.cursor.as_ref());
                v.set_color_cursor_foreground(l.cursor_foreground.as_ref());
                // Selection in the accent color instead of inverted
                v.set_color_highlight(Some(&l.highlight));
                v.set_font(Some(&pango::FontDescription::from_string(&l.font)));
                v.set_bold_is_bright(l.bold_is_bright);
                v.set_cell_height_scale(l.line_spacing);
                v.set_cell_width_scale(l.letter_spacing);
                v.set_scroll_on_output(l.scroll_on_output);
                v.set_scroll_on_keystroke(l.scroll_on_keystroke);
                v.set_cursor_shape(match l.cursor_shape {
                    CursorShape::Block => vte::CursorShape::Block,
                    CursorShape::Ibeam => vte::CursorShape::Ibeam,
                    CursorShape::Underline => vte::CursorShape::Underline,
                });
                // blinking text (SGR 5) only in the focused pane, like gnome-terminal: in all
                // panes it cost 7.8 % CPU while idle in the VM (QA-14)
                v.set_text_blink_mode(vte::TextBlinkMode::Focused);
                v.set_cursor_blink_mode(match l.cursor_blink {
                    CursorBlink::System => vte::CursorBlinkMode::System,
                    CursorBlink::On => vte::CursorBlinkMode::On,
                    CursorBlink::Off => vte::CursorBlinkMode::Off,
                });
                v.set_audible_bell(l.audible_bell);
                v.set_enable_sixel(l.sixel);
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => {
                // bold-is-bright, letter spacing and Sixel have no counterpart yet
                g.set_look(ghostty_view::Look {
                    foreground: l.foreground,
                    background: l.background,
                    palette: l.palette.to_vec(),
                    cursor: l.cursor,
                    cursor_foreground: l.cursor_foreground,
                    highlight: l.highlight,
                    font: l.font.clone(),
                    bold_is_bright: l.bold_is_bright,
                    line_spacing: l.line_spacing,
                    letter_spacing: l.letter_spacing,
                    scroll_on_output: l.scroll_on_output,
                    scroll_on_keystroke: l.scroll_on_keystroke,
                    cursor_shape: l.cursor_shape,
                    cursor_blink: l.cursor_blink,
                    audible_bell: l.audible_bell,
                    scrollback: crate::settings::scrollback_lines(),
                });
            }
        }
    }

    pub fn font_scale(&self) -> f64 {
        match self {
            Backend::Vte(v) => v.font_scale(),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.font_scale(),
        }
    }

    pub fn set_font_scale(&self, scale: f64) {
        match self {
            Backend::Vte(v) => v.set_font_scale(scale),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.set_font_scale(scale),
        }
    }

    pub fn set_context_menu(&self, menu: &gio::Menu) {
        match self {
            Backend::Vte(v) => v.set_context_menu_model(Some(menu)),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.set_context_menu(menu),
        }
    }

    pub fn set_input_enabled(&self, on: bool) {
        match self {
            Backend::Vte(v) => v.set_input_enabled(on),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.set_input_enabled(on),
        }
    }

    pub fn is_input_enabled(&self) -> bool {
        match self {
            Backend::Vte(v) => v.is_input_enabled(),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.is_input_enabled(),
        }
    }

    // ---- geometry -----------------------------------------------------------------------

    /// Size of one character cell in pixels.
    pub fn char_size(&self) -> (i64, i64) {
        match self {
            Backend::Vte(v) => (v.char_width(), v.char_height()),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.char_size(),
        }
    }

    pub fn connect_char_size_changed(&self, f: impl Fn() + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_char_size_changed(move |_, _, _| f());
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_char_size_changed(f),
        }
    }

    pub fn connect_cursor_moved(&self, f: impl Fn() + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_cursor_moved(move |_| f());
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_cursor_moved(f),
        }
    }

    /// Is the cursor in the top visible row?
    pub fn cursor_on_top_row(&self) -> bool {
        match self {
            Backend::Vte(v) => {
                let top = v.vadjustment().map_or(0.0, |a| a.value());
                (v.cursor_position().1 as f64 - top) < 1.0
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.cursor_on_top_row(),
        }
    }

    // ---- process ------------------------------------------------------------------------

    /// Start the program. `done` is called with the process id (VTE: when it is running;
    /// libghostty backend: at once).
    pub fn spawn(&self, spec: &SpawnSpec, done: impl FnOnce(Result<glib::Pid, glib::Error>) + 'static) {
        match self {
            Backend::Vte(v) => {
                let flags = if spec.clean_env {
                    glib::SpawnFlags::SEARCH_PATH
                        | glib::SpawnFlags::from_bits_retain(vte::ffi::VTE_SPAWN_NO_PARENT_ENVV as u32)
                } else {
                    glib::SpawnFlags::SEARCH_PATH
                };
                v.spawn_async(
                    vte::PtyFlags::DEFAULT,
                    spec.dir,
                    spec.argv,
                    spec.env,
                    flags,
                    || {},
                    -1,
                    gio::Cancellable::NONE,
                    done,
                );
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => {
                // `vte.sh` from the distribution (OSC 7 directory, prompt and command marks) only
                // switches on when VTE_VERSION is set; the sequences it prints are standard OSC 7/133
                // plus OSC 666 marks that the widget understands.
                let mut env: Vec<String> = spec.env.iter().map(|s| s.to_string()).collect();
                if !env.iter().any(|e| e.starts_with("VTE_VERSION=")) {
                    // SAFETY: plain version queries
                    let (ma, mi, mc) = unsafe {
                        (
                            vte::ffi::vte_get_major_version(),
                            vte::ffi::vte_get_minor_version(),
                            vte::ffi::vte_get_micro_version(),
                        )
                    };
                    env.push(format!("VTE_VERSION={}", ma * 10000 + mi * 100 + mc));
                }
                let opts = parkett_core::pty::SpawnOptions {
                    argv: spec.argv.iter().map(|s| s.to_string()).collect(),
                    cwd: spec.dir.map(String::from),
                    env,
                    clean_env: spec.clean_env,
                    cols: 0,
                    rows: 0,
                    // Inside Flatpak the shell has to run on the host (docs/LIBGHOSTTY.md)
                    host: parkett_core::pty::in_flatpak(),
                };
                match g.start(opts) {
                    Ok(()) => done(Ok(glib::Pid(g.child_pid().unwrap_or(0) as i32))),
                    Err(e) => done(Err(glib::Error::new(glib::FileError::Failed, &e.to_string()))),
                }
            }
        }
    }

    /// The child ended; argument is the wait status.
    pub fn connect_child_exited(&self, f: impl Fn(i32) + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_child_exited(move |_, status| f(status));
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_exited_status(move |_, status| f(status)),
        }
    }

    /// Type text to the program (not subject to read-only).
    pub fn feed(&self, data: &[u8]) {
        match self {
            Backend::Vte(v) => v.feed_child(data),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.send_input(data),
        }
    }

    /// Paste as the program wants it (bracketed paste); dropped while read-only.
    pub fn paste(&self, text: &str) {
        match self {
            Backend::Vte(v) => v.paste_text(text),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => {
                // Parkett asks about multi-line text itself (`paste_with_check`)
                g.paste_text(text, true);
            }
        }
    }

    /// Process group in the foreground of the PTY.
    pub fn foreground_pgid(&self) -> Option<i32> {
        match self {
            Backend::Vte(v) => {
                let fd = v.pty()?.fd().as_raw_fd();
                // SAFETY: valid descriptor of the PTY
                let pgid = unsafe { libc::tcgetpgrp(fd) };
                (pgid > 0).then_some(pgid)
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.foreground_pgid(),
        }
    }

    /// termios of the PTY: line-wise input and echo (detects a password prompt).
    pub fn tty_mode(&self) -> Option<parkett_core::startup::TtyMode> {
        match self {
            Backend::Vte(v) => {
                let fd = v.pty()?.fd().as_raw_fd();
                let mut t: libc::termios = unsafe { std::mem::zeroed() };
                // SAFETY: valid descriptor of the PTY, `t` is a complete termios
                if unsafe { libc::tcgetattr(fd, &mut t) } != 0 {
                    return None;
                }
                Some(parkett_core::startup::TtyMode {
                    canonical: t.c_lflag & libc::ICANON != 0,
                    echo: t.c_lflag & libc::ECHO != 0,
                })
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.tty_mode(),
        }
    }

    // ---- state the shell reports --------------------------------------------------------

    /// Window title the shell set via OSC 0/2.
    pub fn title(&self) -> Option<String> {
        match self {
            Backend::Vte(v) => v.termprop_string("xterm.title").0.map(String::from),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.title(),
        }
    }

    /// Working directory from OSC 7 (VTE: `vte.cwd`, requires vte.sh in the shell).
    /// vte4-rs does not bind `ref_termprop_uri`; hence the one FFI call.
    pub fn cwd_uri(&self) -> Option<glib::Uri> {
        match self {
            Backend::Vte(v) => unsafe {
                from_glib_full(vte::ffi::vte_terminal_ref_termprop_uri(v.to_glib_none().0, c"vte.cwd".as_ptr()))
            },
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.pwd().and_then(|u| glib::Uri::parse(&u, glib::UriFlags::NONE).ok()),
        }
    }

    pub fn connect_shell_event(&self, f: impl Fn(ShellEvent) + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_termprop_changed(None, move |_, name| {
                    let ev = match name {
                        "xterm.title" => ShellEvent::Title,
                        "vte.cwd" => ShellEvent::Cwd,
                        // Shell integration (vte.sh): command starts or prompt returns
                        "vte.shell.preexec" => ShellEvent::Preexec,
                        "vte.shell.precmd" => ShellEvent::Precmd,
                        _ => return,
                    };
                    f(ev);
                });
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_shell_event(move |e| {
                f(match e {
                    ghostty_view::ShellEvent::Title => ShellEvent::Title,
                    ghostty_view::ShellEvent::Cwd => ShellEvent::Cwd,
                    ghostty_view::ShellEvent::Preexec => ShellEvent::Preexec,
                    ghostty_view::ShellEvent::Precmd => ShellEvent::Precmd,
                })
            }),
        }
    }

    pub fn connect_bell(&self, f: impl Fn() + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_bell(move |_| f());
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_bell(f),
        }
    }

    /// Text the user types or pastes (VTE `commit`).
    pub fn connect_commit(&self, f: impl Fn(&str) + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_commit(move |_, text, _| f(text));
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_commit(f),
        }
    }

    pub fn connect_selection_changed(&self, f: impl Fn() + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_selection_changed(move |_| f());
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_selection_changed(f),
        }
    }

    pub fn connect_contents_changed(&self, f: impl Fn() + 'static) {
        match self {
            Backend::Vte(v) => {
                v.connect_contents_changed(move |_| f());
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.connect_contents_changed(f),
        }
    }

    // ---- selection and clipboard ----------------------------------------------------------

    pub fn has_selection(&self) -> bool {
        match self {
            Backend::Vte(v) => v.has_selection(),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.has_selection(),
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        match self {
            Backend::Vte(v) => v.text_selected(vte::Format::Text).map(String::from),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.selected_text(),
        }
    }

    pub fn select_all(&self) {
        match self {
            Backend::Vte(v) => v.select_all(),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.select_all(),
        }
    }

    pub fn copy(&self, html: bool) {
        match self {
            Backend::Vte(v) => v.copy_clipboard_format(if html { vte::Format::Html } else { vte::Format::Text }),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.copy(html),
        }
    }

    /// Reset the terminal; `clear` also discards the scrollback.
    pub fn reset(&self, clear: bool) {
        match self {
            Backend::Vte(v) => v.reset(true, clear),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.reset(clear),
        }
    }

    /// Scrollback and visible text as plain text.
    pub fn write_contents(&self, stream: &gio::OutputStream) -> Result<(), glib::Error> {
        match self {
            Backend::Vte(v) => v.write_contents_sync(stream, vte::WriteFlags::Default, gio::Cancellable::NONE),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => stream.write_all(g.text().as_bytes(), gio::Cancellable::NONE).map(|_| ()),
        }
    }

    // ---- links ----------------------------------------------------------------------------

    /// Make text matching the PCRE2 pattern clickable; the tag identifies it in `match_at`.
    pub fn add_match(&self, pattern: &str) -> Result<i32, String> {
        match self {
            Backend::Vte(v) => {
                let re = vte::Regex::for_match(pattern, PCRE2_FLAGS).map_err(|e| e.message().to_string())?;
                let tag = v.match_add_regex(&re, 0);
                v.match_set_cursor_name(tag, "pointer");
                Ok(tag)
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.add_pattern(pattern),
        }
    }

    pub fn remove_match(&self, tag: i32) {
        match self {
            Backend::Vte(v) => v.match_remove(tag),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.remove_pattern(tag),
        }
    }

    /// Matched text and tag at a widget position.
    pub fn match_at(&self, x: f64, y: f64) -> (Option<String>, i32) {
        match self {
            Backend::Vte(v) => {
                let (text, tag) = v.check_match_at(x, y);
                (text.map(String::from), tag)
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.pattern_at(x, y),
        }
    }

    /// OSC 8 link at a widget position.
    pub fn hyperlink_at(&self, x: f64, y: f64) -> Option<String> {
        match self {
            Backend::Vte(v) => v.check_hyperlink_at(x, y).map(String::from),
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.hyperlink_at(x, y),
        }
    }

    /// OSC 8 links show their real target as a tooltip – the visible text can deceive.
    pub fn show_hyperlink_tooltips(&self) {
        match self {
            Backend::Vte(v) => {
                v.connect_hyperlink_hover_uri_notify(|vte| vte.set_tooltip_text(vte.hyperlink_hover_uri().as_deref()));
            }
            // the widget does it itself
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(_) => {}
        }
    }

    // ---- search ---------------------------------------------------------------------------

    /// Sets the search pattern (PCRE2); `None` clears it. Error text for an invalid pattern.
    pub fn set_search(&self, pattern: Option<&str>, case_sensitive: bool) -> Result<(), String> {
        match self {
            Backend::Vte(v) => {
                let Some(pattern) = pattern else {
                    v.search_set_regex(None, 0);
                    v.unselect_all();
                    return Ok(());
                };
                let flags = PCRE2_FLAGS | if case_sensitive { 0 } else { PCRE2_CASELESS };
                let re = vte::Regex::for_search(pattern, flags).map_err(|e| e.message().to_string())?;
                v.search_set_regex(Some(&re), 0);
                v.search_set_wrap_around(true);
                Ok(())
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => {
                if pattern.is_none() {
                    g.clear_selection();
                }
                g.search(pattern, case_sensitive).map(|_| ())
            }
        }
    }

    /// Next match upward (older output) or downward. `false` = nothing found.
    pub fn search_step(&self, up: bool) -> bool {
        match self {
            Backend::Vte(v) => {
                if up {
                    v.search_find_previous()
                } else {
                    v.search_find_next()
                }
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => g.search_step(up),
        }
    }

    /// Matches of `re` in the whole scrollback, at most `max`. VTE does not count itself; the
    /// text is read from the bottom in blocks (fetching all of it cost about 0.27 s per key with
    /// 300,000 lines). A match across a block boundary can be missed – the number is an overview.
    pub fn count_matches(&self, re: &glib::Regex, max: usize) -> Option<usize> {
        match self {
            Backend::Vte(vte) => {
                let adj = vte.vadjustment()?;
                let (first, last) = (adj.lower() as libc::c_long, adj.upper() as libc::c_long);
                let mut n = 0;
                let mut end = last;
                while end > first && n < max {
                    // rows start..end, without end (it belongs to the block below)
                    let start = (end - SEARCH_COUNT_ROWS).max(first);
                    let (content, _) = vte.text_range_format(vte::Format::Text, start, 0, end - 1, vte.column_count());
                    if let Some(content) = content {
                        if let Ok(info) = re.match_(content.as_gstr(), glib::RegexMatchFlags::empty()) {
                            while info.matches() && n < max {
                                n += 1;
                                if !info.next().unwrap_or(false) {
                                    break;
                                }
                            }
                        }
                    }
                    end = start;
                }
                Some(n)
            }
            #[cfg(feature = "ghostty")]
            Backend::Ghostty(g) => Some(g.count_matches(re, max)),
        }
    }
}

/// Is the installed VTE built with Sixel? (Arch and Fedora 44 are not.)
pub fn sixel_supported() -> bool {
    // vte4-rs does not bind vte_get_feature_flags; plain query without arguments
    let flags = vte::FeatureFlags::from_bits_truncate(unsafe { vte::ffi::vte_get_feature_flags() });
    flags.contains(vte::FeatureFlags::FLAG_SIXEL)
}

/// The user's login shell according to VTE (passwd).
pub fn user_shell() -> Option<String> {
    unsafe { from_glib_full::<_, Option<glib::GString>>(vte::ffi::vte_get_user_shell()) }.map(String::from)
}
