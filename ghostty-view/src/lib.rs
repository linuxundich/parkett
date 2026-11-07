//! `GhosttyView`: a GTK4 terminal widget drawn from libghostty-vt's render state, with the child
//! on a `parkett_core::pty::Pty`. Phase 3b of the libghostty backend (docs/LIBGHOSTTY.md): drawing,
//! keyboard (incl. IME), wheel scrolling, resize. Selection, links, search and mouse reporting
//! follow in phase 4.
pub mod a11y;
mod imp;
pub mod links;
pub mod search;

pub use links::Link;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use parkett_core::profile::{CursorBlink, CursorShape};
use parkett_core::pty::SpawnOptions;

/// Everything the host decides about appearance and scrolling, in one piece.
#[derive(Clone)]
pub struct Look {
    pub foreground: gdk::RGBA,
    /// Alpha is the background opacity.
    pub background: gdk::RGBA,
    pub palette: Vec<gdk::RGBA>,
    pub cursor: Option<gdk::RGBA>,
    pub cursor_foreground: Option<gdk::RGBA>,
    /// Selected cells get this background.
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
    /// Lines of scrollback, -1 = unlimited.
    pub scrollback: i64,
}

/// What the shell or program reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellEvent {
    Title,
    Cwd,
    /// A command starts (OSC 133 `C`).
    Preexec,
    /// The prompt is back (OSC 133 `A`).
    Precmd,
}

glib::wrapper! {
    pub struct GhosttyView(ObjectSubclass<imp::GhosttyView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable, gtk::AccessibleText;
}

impl Default for GhosttyView {
    fn default() -> Self {
        Self::new()
    }
}

impl GhosttyView {
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Start the program. The PTY is created at the widget's current size (80x24 before the first
    /// allocation) and resized as the widget is.
    pub fn start(&self, opts: SpawnOptions) -> std::io::Result<()> {
        self.imp().start(opts)
    }

    /// Font as a Pango description string, e.g. `"Monospace 11"`.
    pub fn set_font(&self, font: &str) {
        self.imp().set_font(font);
    }

    pub fn title(&self) -> Option<String> {
        self.imp().with_state(|s| s.term.title()).flatten()
    }

    /// Working directory from OSC 7 (a `file://` URI).
    pub fn pwd(&self) -> Option<String> {
        self.imp().with_state(|s| s.term.pwd()).flatten()
    }

    /// Visible text of the whole screen including scrollback (tests, later accessibility).
    pub fn text(&self) -> String {
        self.imp().with_state(|s| s.term.text()).unwrap_or_default()
    }

    /// Grid size in cells.
    pub fn grid_size(&self) -> (u16, u16) {
        self.imp().with_state(|s| s.term.size()).unwrap_or((0, 0))
    }

    /// Type bytes as if entered on the keyboard (startup commands, paste).
    pub fn send_input(&self, data: &[u8]) {
        self.imp().send(data);
    }

    pub fn has_selection(&self) -> bool {
        self.imp().has_selection()
    }

    pub fn select_all(&self) {
        self.imp().select_all();
    }

    pub fn clear_selection(&self) {
        self.imp().clear_selection();
    }

    pub fn selected_text(&self) -> Option<String> {
        self.imp().selected_text(false)
    }

    /// Copy the selection to the clipboard; with `html` also as `text/html` (colors, bold).
    pub fn copy(&self, html: bool) {
        self.imp().copy(html);
    }

    /// Paste the clipboard (Ctrl+Shift+V does the same).
    pub fn paste(&self) {
        self.imp().paste_clipboard();
    }

    /// Paste `text`; returns what happened. `Unsafe` text is reported to the
    /// [`connect_unsafe_paste`](Self::connect_unsafe_paste) listeners and not sent.
    pub fn paste_text(&self, text: &str, allow_unsafe: bool) -> ghostty_vt::edit::PasteResult {
        self.imp().paste_text(text, allow_unsafe)
    }

    /// Text that could run commands was refused; ask the user and call `paste_text(.., true)`.
    pub fn connect_unsafe_paste(&self, f: impl Fn(&str) + 'static) {
        self.imp().unsafe_paste_cbs.borrow_mut().push(Box::new(f));
    }

    /// Custom links of the profile (PCRE2 pattern, target with `$0`…`$9`).
    pub fn set_custom_links(&self, links: &[parkett_core::link::CustomLink]) {
        self.imp().set_custom_links(links);
    }

    /// The link under a pixel position of the widget (for a context menu).
    pub fn link_at(&self, x: f64, y: f64) -> Option<Link> {
        let (cw, ch) = self.imp().cell_size();
        self.imp().link_at_cell((x / cw).max(0.0) as u16, (y / ch).max(0.0) as u16).map(|s| s.link)
    }

    /// Ctrl+click on a link; the host opens it (scheme checks, file manager, commands).
    pub fn connect_link_activated(&self, f: impl Fn(&Link) + 'static) {
        self.imp().link_cbs.borrow_mut().push(Box::new(f));
    }

    /// Search the scrollback. `pattern` is PCRE2 (the caller adds `\b…\b` and escaping for whole
    /// words and plain text); `None` or empty clears the highlights. Returns the number of hits or the
    /// error of a bad pattern. Hits do not cross row ends.
    pub fn search(&self, pattern: Option<&str>, case_sensitive: bool) -> Result<usize, String> {
        self.imp().search_set(pattern, case_sensitive)
    }

    pub fn search_count(&self) -> usize {
        self.imp().search_count()
    }

    /// 0-based index of the selected hit, if one is selected.
    pub fn search_index(&self) -> Option<usize> {
        self.imp().search_index()
    }

    /// Select the next or previous hit (wraps around) and scroll it into view.
    pub fn search_step(&self, up: bool) -> bool {
        self.imp().search_step(up)
    }

    pub fn connect_title_changed(&self, f: impl Fn(&Self) + 'static) {
        self.imp().title_cbs.borrow_mut().push(Box::new(f_wrap(self, f)));
    }

    /// The program ended; argument is its exit code (`None` when killed by a signal).
    pub fn connect_exited(&self, f: impl Fn(&Self, Option<i32>) + 'static) {
        use std::os::unix::process::ExitStatusExt;
        self.connect_exited_status(move |v, raw| f(v, std::process::ExitStatus::from_raw(raw).code()));
    }

    /// The program ended; argument is the raw wait status (as `waitpid` reports it).
    pub fn connect_exited_status(&self, f: impl Fn(&Self, i32) + 'static) {
        let weak = self.downgrade();
        self.imp().exit_cbs.borrow_mut().push(Box::new(move |status| {
            if let Some(v) = weak.upgrade() {
                f(&v, status);
            }
        }));
    }

    /// Appearance and scrolling (call again to change them).
    pub fn set_look(&self, look: Look) {
        self.imp().set_look(look);
    }

    /// Scroll position: value is the first visible row of scrollback + screen, upper the total
    /// number of rows. The widget is `GtkScrollable`; inside a `GtkScrolledWindow` this is its bar.
    pub fn scroll_position(&self) -> gtk::Adjustment {
        self.imp().adjustment()
    }

    pub fn set_scrollback_lines(&self, lines: i64) {
        self.imp().set_scrollback_lines(lines);
    }

    pub fn font_scale(&self) -> f64 {
        self.imp().font_scale()
    }

    pub fn set_font_scale(&self, scale: f64) {
        self.imp().set_font_scale(scale);
    }

    /// Read only: keys and paste do not reach the program.
    pub fn set_input_enabled(&self, on: bool) {
        self.imp().set_input_enabled(on);
    }

    pub fn is_input_enabled(&self) -> bool {
        self.imp().is_input_enabled()
    }

    /// Menu shown on right-click.
    pub fn set_context_menu(&self, model: &impl IsA<gtk::gio::MenuModel>) {
        self.imp().set_context_menu(model.upcast_ref());
    }

    /// pid of the shell on the host when it was started through `flatpak-spawn --host` (reported by the shell).
    pub fn host_pid(&self) -> Option<u32> {
        self.imp().host_pid()
    }

    pub fn child_pid(&self) -> Option<u32> {
        self.imp().child_pid()
    }

    /// Process group in the foreground of the terminal.
    pub fn foreground_pgid(&self) -> Option<i32> {
        self.imp().foreground_pgid()
    }

    pub fn tty_mode(&self) -> Option<parkett_core::startup::TtyMode> {
        self.imp().tty_mode()
    }

    /// SIGHUP to the program's process group.
    pub fn hangup(&self) {
        self.imp().hterminate();
    }

    pub fn reset(&self, clear: bool) {
        self.imp().reset(clear);
    }

    pub fn cursor_on_top_row(&self) -> bool {
        self.imp().cursor_on_top_row()
    }

    /// Size of one character cell in pixels.
    pub fn char_size(&self) -> (i64, i64) {
        let (w, h) = self.imp().cell_size();
        (w.round() as i64, h.round() as i64)
    }

    /// Detect links with these host patterns instead of the built-in ones (`set_builtin_links`).
    pub fn add_pattern(&self, pattern: &str) -> Result<i32, String> {
        self.imp().add_pattern(pattern)
    }

    pub fn remove_pattern(&self, tag: i32) {
        self.imp().remove_pattern(tag);
    }

    /// Use the built-in URL, mail and path detection (on by default).
    pub fn set_builtin_links(&self, on: bool) {
        self.imp().set_builtin_links(on);
    }

    /// Host pattern at a pixel position: matched text and the tag from `add_pattern`.
    pub fn pattern_at(&self, x: f64, y: f64) -> (Option<String>, i32) {
        self.imp().pattern_at(x, y)
    }

    /// OSC 8 link at a pixel position.
    pub fn hyperlink_at(&self, x: f64, y: f64) -> Option<String> {
        self.imp().hyperlink_at_point(x, y)
    }

    /// Number of matches of `re` in the scrollback, at most `max`.
    pub fn count_matches(&self, re: &glib::Regex, max: usize) -> usize {
        self.imp().count_matches(re, max)
    }

    pub fn connect_bell(&self, f: impl Fn() + 'static) {
        self.imp().bell_cbs.borrow_mut().push(Box::new(f));
    }

    /// Text the user typed or pasted (what went to the program; not `send_input`).
    pub fn connect_commit(&self, f: impl Fn(&str) + 'static) {
        self.imp().commit_cbs.borrow_mut().push(Box::new(f));
    }

    pub fn connect_selection_changed(&self, f: impl Fn() + 'static) {
        self.imp().selection_cbs.borrow_mut().push(Box::new(f));
    }

    pub fn connect_contents_changed(&self, f: impl Fn() + 'static) {
        self.imp().contents_cbs.borrow_mut().push(Box::new(f));
    }

    pub fn connect_cursor_moved(&self, f: impl Fn() + 'static) {
        self.imp().cursor_cbs.borrow_mut().push(Box::new(f));
    }

    pub fn connect_char_size_changed(&self, f: impl Fn() + 'static) {
        self.imp().char_cbs.borrow_mut().push(Box::new(f));
    }

    /// Title, directory (OSC 7) and shell marks (OSC 133) reported by the program.
    pub fn connect_shell_event(&self, f: impl Fn(ShellEvent) + 'static) {
        self.imp().shell_cbs.borrow_mut().push(Box::new(f));
    }
}

fn f_wrap(v: &GhosttyView, f: impl Fn(&GhosttyView) + 'static) -> impl Fn() + 'static {
    let weak = v.downgrade();
    move || {
        if let Some(v) = weak.upgrade() {
            f(&v);
        }
    }
}
