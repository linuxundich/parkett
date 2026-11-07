//! Safe wrapper around the parts of libghostty-vt that Parkett's optional backend needs.
//! Phase 1: terminal state fed with bytes, read back as text, title, working directory, cursor.
pub mod edit;
pub mod find;
pub mod input;
pub mod render;

pub use render::CursorShape;

use ghostty_vt_sys as sys;
use std::ptr::{null, null_mut};

/// A virtual terminal: parser plus screen state, no PTY and no drawing. Not `Send`/`Sync`.
type PtyWriteFn = Box<dyn FnMut(&[u8])>;
type OscFn = Box<dyn FnMut(&[u8])>;

/// What the shell reports with OSC 133 (semantic prompt marks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prompt {
    /// The prompt is being drawn: the previous command is over.
    PromptStart,
    /// The user can type.
    InputStart,
    /// The command was submitted and now produces output.
    OutputStart,
    /// The command ended (exit code if the shell sent one).
    CommandEnd(Option<i32>),
}

unsafe extern "C" fn unknown_trampoline(
    _t: sys::GhosttyTerminal,
    userdata: *mut core::ffi::c_void,
    seq: *const sys::GhosttyTerminalUnknownSequence,
) {
    // SAFETY: see `write_pty_trampoline`; `seq` is valid for the call and `tag` selects the union field
    let cb = &mut *(userdata as *mut Callbacks);
    let seq = &*seq;
    if seq.tag != sys::GhosttyTerminalUnknownSequenceTag_GHOSTTY_TERMINAL_UNKNOWN_SEQUENCE_OSC {
        return;
    }
    let c = seq.value.osc.content;
    if let (Some(f), false) = (cb.unknown_osc.as_mut(), c.ptr.is_null()) {
        f(std::slice::from_raw_parts(c.ptr, c.len));
    }
}

/// Rust side of the C callbacks; one heap block whose address is the library's userdata.
#[derive(Default)]
struct Callbacks {
    pty_write: Option<PtyWriteFn>,
    bell: Option<Box<dyn FnMut()>>,
    prompt: Option<Box<dyn FnMut(Prompt)>>,
    unknown_osc: Option<OscFn>,
}

pub struct Terminal {
    raw: sys::GhosttyTerminal,
    cb: Box<Callbacks>,
}

unsafe extern "C" fn write_pty_trampoline(
    _t: sys::GhosttyTerminal,
    userdata: *mut core::ffi::c_void,
    data: *const u8,
    len: usize,
) {
    // SAFETY: userdata is the `Callbacks` block owned by the terminal; the library calls this on
    // the thread that calls `write`.
    let cb = &mut *(userdata as *mut Callbacks);
    if let Some(f) = cb.pty_write.as_mut() {
        f(std::slice::from_raw_parts(data, len));
    }
}

unsafe extern "C" fn bell_trampoline(_t: sys::GhosttyTerminal, userdata: *mut core::ffi::c_void) {
    // SAFETY: see `write_pty_trampoline`
    let cb = &mut *(userdata as *mut Callbacks);
    if let Some(f) = cb.bell.as_mut() {
        f();
    }
}

unsafe extern "C" fn prompt_trampoline(
    _t: sys::GhosttyTerminal,
    userdata: *mut core::ffi::c_void,
    ev: *const sys::GhosttyTerminalSemanticPrompt,
) {
    // SAFETY: see `write_pty_trampoline`; `ev` is valid for the call
    let cb = &mut *(userdata as *mut Callbacks);
    let ev = &*ev;
    let p = match ev.kind {
        sys::GhosttySemanticPromptKind_GHOSTTY_SEMANTIC_PROMPT_PROMPT_START => Prompt::PromptStart,
        sys::GhosttySemanticPromptKind_GHOSTTY_SEMANTIC_PROMPT_INPUT_START => Prompt::InputStart,
        sys::GhosttySemanticPromptKind_GHOSTTY_SEMANTIC_PROMPT_OUTPUT_START => Prompt::OutputStart,
        sys::GhosttySemanticPromptKind_GHOSTTY_SEMANTIC_PROMPT_COMMAND_END => {
            Prompt::CommandEnd(ev.has_exit_code.then_some(ev.exit_code))
        }
        _ => return,
    };
    if let Some(f) = cb.prompt.as_mut() {
        f(p);
    }
}

fn ok(r: sys::GhosttyResult) -> Result<(), i32> {
    if r == 0 {
        Ok(())
    } else {
        Err(r)
    }
}

impl Terminal {
    pub(crate) fn raw(&self) -> sys::GhosttyTerminal {
        self.raw
    }

    pub fn new(cols: u16, rows: u16) -> Result<Self, i32> {
        let mut raw = null_mut();
        // SAFETY: out pointer is valid; NULL selects the default allocator.
        ok(unsafe { sys::ghostty_terminal_new(null(), &mut raw, cols, rows) })?;
        let mut t = Self { raw, cb: Box::default() };
        // One userdata for all callbacks; the block never moves (it is boxed).
        let ud = &mut *t.cb as *mut Callbacks as *mut core::ffi::c_void;
        t.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_USERDATA, ud);
        Ok(t)
    }

    fn set(&mut self, option: sys::GhosttyTerminalOption, value: *const core::ffi::c_void) {
        // SAFETY: live terminal; callers pass a pointer valid for the call (or a function pointer).
        unsafe { sys::ghostty_terminal_set(self.raw, option, value) };
    }

    /// Replies the terminal wants to send to the program (device attributes, cursor position
    /// reports, …). Without this, programs that ask never get an answer.
    pub fn on_pty_write(&mut self, f: impl FnMut(&[u8]) + 'static) {
        self.cb.pty_write = Some(Box::new(f));
        let cb: sys::GhosttyTerminalWritePtyFn = Some(write_pty_trampoline);
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_WRITE_PTY, cb.map_or(null(), |f| f as *const _));
    }

    /// The program rang the bell (BEL).
    pub fn on_bell(&mut self, f: impl FnMut() + 'static) {
        self.cb.bell = Some(Box::new(f));
        let cb: sys::GhosttyTerminalBellFn = Some(bell_trampoline);
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_BELL, cb.map_or(null(), |f| f as *const _));
    }

    /// The shell reports prompt and command boundaries (OSC 133; needs shell integration).
    pub fn on_prompt(&mut self, f: impl FnMut(Prompt) + 'static) {
        self.cb.prompt = Some(Box::new(f));
        let cb: sys::GhosttyTerminalSemanticPromptFn = Some(prompt_trampoline);
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_SEMANTIC_PROMPT, cb.map_or(null(), |f| f as *const _));
    }

    /// OSC sequences the library does not know (content without the leading `ESC ]`), e.g. VTE's
    /// `666;vte.shell.preexec!` from `vte.sh`.
    pub fn on_unknown_osc(&mut self, f: impl FnMut(&[u8]) + 'static) {
        self.cb.unknown_osc = Some(Box::new(f));
        let max: usize = 2048;
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_UNKNOWN_MAX_BYTES, &max as *const _ as *const _);
        let cb: sys::GhosttyTerminalUnknownSequenceFn = Some(unknown_trampoline);
        self.set(
            sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_UNKNOWN_SEQUENCE,
            cb.map_or(null(), |f| f as *const _),
        );
    }

    /// Default colors programs see (OSC 10/11 queries) and the 256-color palette.
    pub fn set_colors(&mut self, fg: [u8; 3], bg: [u8; 3], cursor: Option<[u8; 3]>, palette: &[[u8; 3]]) {
        let rgb = |c: [u8; 3]| sys::GhosttyColorRgb { r: c[0], g: c[1], b: c[2] };
        let (fg, bg) = (rgb(fg), rgb(bg));
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_COLOR_FOREGROUND, &fg as *const _ as *const _);
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_COLOR_BACKGROUND, &bg as *const _ as *const _);
        let cur = cursor.map(rgb);
        self.set(
            sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_COLOR_CURSOR,
            cur.as_ref().map_or(null(), |c| c as *const _ as *const _),
        );
        if !palette.is_empty() {
            // The profile gives the 16 ANSI colors; 16..255 (cube, grays) stay the library's.
            let mut pal = vec![sys::GhosttyColorRgb { r: 0, g: 0, b: 0 }; 256];
            // SAFETY: documented output type, 256 entries
            unsafe {
                sys::ghostty_terminal_get(
                    self.raw,
                    sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_COLOR_PALETTE_DEFAULT,
                    pal.as_mut_ptr() as *mut _,
                );
            }
            for (i, c) in palette.iter().take(256).enumerate() {
                pal[i] = rgb(*c);
            }
            self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_COLOR_PALETTE, pal.as_ptr() as *const _);
        }
    }

    /// Lines kept in the scrollback; `None` = unlimited.
    pub fn set_scrollback_lines(&mut self, lines: Option<usize>) {
        let v = lines.unwrap_or(usize::MAX);
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_LINES, &v as *const _ as *const _);
    }

    /// Cursor shape and blinking for programs that do not choose one (DECSCUSR resets to this).
    pub fn set_default_cursor(&mut self, style: render::CursorShape, blinking: bool) {
        let st = match style {
            render::CursorShape::Bar => sys::GhosttyCursorStyle_GHOSTTY_CURSOR_STYLE_BAR,
            render::CursorShape::Block => sys::GhosttyCursorStyle_GHOSTTY_CURSOR_STYLE_BLOCK,
            render::CursorShape::Underline => sys::GhosttyCursorStyle_GHOSTTY_CURSOR_STYLE_UNDERLINE,
            render::CursorShape::BlockHollow => sys::GhosttyCursorStyle_GHOSTTY_CURSOR_STYLE_BLOCK_HOLLOW,
        };
        self.set(sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_DEFAULT_CURSOR_STYLE, &st as *const _ as *const _);
        self.set(
            sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_DEFAULT_CURSOR_BLINK,
            &blinking as *const _ as *const _,
        );
    }

    /// Reset like VTE's `reset`; with `clear` the scrollback goes too.
    pub fn reset(&mut self, clear: bool) {
        self.write(b"\x1bc");
        if clear {
            self.write(b"\x1b[3J");
        }
    }

    /// Scroll the view by `rows` (negative = up into the scrollback).
    pub fn scroll(&mut self, rows: isize) {
        let b = sys::GhosttyTerminalScrollViewport {
            tag: sys::GhosttyTerminalScrollViewportTag_GHOSTTY_TERMINAL_SCROLL_VIEWPORT_DELTA,
            value: sys::GhosttyTerminalScrollViewportValue { delta: rows },
        };
        // SAFETY: live handle, plain value.
        unsafe {
            sys::ghostty_terminal_scroll_viewport(self.raw, b);
        }
    }

    /// Jump back to the live screen.
    pub fn scroll_to_bottom(&mut self) {
        let b = sys::GhosttyTerminalScrollViewport {
            tag: sys::GhosttyTerminalScrollViewportTag_GHOSTTY_TERMINAL_SCROLL_VIEWPORT_BOTTOM,
            value: sys::GhosttyTerminalScrollViewportValue { delta: 0 },
        };
        // SAFETY: live handle, plain value.
        unsafe {
            sys::ghostty_terminal_scroll_viewport(self.raw, b);
        }
    }

    /// Feed bytes as a program would write them to the PTY.
    pub fn write(&mut self, data: &[u8]) {
        // SAFETY: pointer and length describe `data`; the terminal handle is live.
        unsafe { sys::ghostty_terminal_vt_write(self.raw, data.as_ptr(), data.len()) }
    }

    /// Resize; the cell size in pixels is only used for size reports and graphics.
    pub fn resize(&mut self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> Result<(), i32> {
        ok(unsafe { sys::ghostty_terminal_resize(self.raw, cols, rows, cell_w, cell_h) })
    }

    fn get<T>(&self, what: sys::GhosttyTerminalData, out: &mut T) -> Result<(), i32> {
        // SAFETY: `out` has the type the header documents for `what` (callers below).
        ok(unsafe { sys::ghostty_terminal_get(self.raw, what, out as *mut T as *mut _) })
    }

    fn string(&self, what: sys::GhosttyTerminalData) -> Option<String> {
        let mut s = sys::GhosttyString { ptr: null(), len: 0 };
        self.get(what, &mut s).ok()?;
        if s.ptr.is_null() || s.len == 0 {
            return None;
        }
        // SAFETY: the library owns the bytes until the next write; we copy at once.
        let bytes = unsafe { std::slice::from_raw_parts(s.ptr, s.len) };
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// Title set by OSC 0/2.
    pub fn title(&self) -> Option<String> {
        self.string(sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_TITLE)
    }

    /// Working directory as the URI sent with OSC 7.
    pub fn pwd(&self) -> Option<String> {
        self.string(sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_PWD)
    }

    pub fn size(&self) -> (u16, u16) {
        let (mut c, mut r) = (0u16, 0u16);
        let _ = self.get(sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_COLS, &mut c);
        let _ = self.get(sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_ROWS, &mut r);
        (c, r)
    }

    /// Cursor position (column, row) on the active screen, 0-based.
    pub fn cursor(&self) -> (u16, u16) {
        let (mut x, mut y) = (0u16, 0u16);
        let _ = self.get(sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_CURSOR_X, &mut x);
        let _ = self.get(sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_CURSOR_Y, &mut y);
        (x, y)
    }

    /// Plain text of the screen (scrollback included), trailing blanks trimmed by the library.
    pub fn text(&self) -> String {
        let mut f = null_mut();
        let mut p = null_mut();
        let mut n = 0usize;
        // SAFETY: handles are live, out pointers valid; the buffer is freed below.
        unsafe {
            if sys::ghostty_formatter_terminal_new(null(), &mut f, self.raw) != 0 {
                return String::new();
            }
            let r = sys::ghostty_formatter_format_alloc(f, null(), &mut p, &mut n);
            let s = if r == 0 && !p.is_null() {
                String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned()
            } else {
                String::new()
            };
            if !p.is_null() {
                sys::ghostty_free(null(), p, n);
            }
            sys::ghostty_formatter_free(f);
            s
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // SAFETY: handle came from ghostty_terminal_new and is freed exactly once.
        unsafe { sys::ghostty_terminal_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_title_cwd_cursor() {
        let mut t = Terminal::new(40, 5).unwrap();
        t.write(b"\x1b]2;vim\x07\x1b]7;file://box/home/toff\x07ls\r\nfoo  bar\r\n$ ");
        assert_eq!(t.title().as_deref(), Some("vim"));
        assert!(t.pwd().unwrap().ends_with("/home/toff"));
        assert_eq!(t.text(), "ls\nfoo  bar\n$");
        assert_eq!(t.cursor(), (2, 2));
        assert_eq!(t.size(), (40, 5));
    }

    #[test]
    fn scrollback_and_utf8_split_across_writes() {
        let mut t = Terminal::new(10, 2).unwrap();
        for i in 0..5 {
            t.write(format!("line{i}\r\n").as_bytes());
        }
        // "ä" (0xC3 0xA4) split over two writes must still decode.
        t.write(&[0xC3]);
        t.write(&[0xA4]);
        let text = t.text();
        assert!(text.starts_with("line0\n"), "{text:?}");
        assert!(text.ends_with("ä"), "{text:?}");
    }

    #[test]
    fn replies_go_to_the_callback() {
        use std::{cell::RefCell, rc::Rc};
        let got = Rc::new(RefCell::new(Vec::new()));
        let g = got.clone();
        let mut t = Terminal::new(20, 3).unwrap();
        t.on_pty_write(move |d| g.borrow_mut().extend_from_slice(d));
        t.write(b"ab\x1b[6n"); // cursor position report
        assert_eq!(&got.borrow()[..], b"\x1b[1;3R");
    }

    #[test]
    fn bell_prompt_marks_colors_and_scrollback_limit() {
        use std::{cell::RefCell, rc::Rc};
        let events = Rc::new(RefCell::new(Vec::new()));
        let (e1, e2) = (events.clone(), events.clone());
        let mut t = Terminal::new(20, 3).unwrap();
        t.on_bell(move || e1.borrow_mut().push("bell".to_string()));
        t.on_prompt(move |p| e2.borrow_mut().push(format!("{p:?}")));
        t.write(b"\x07\x1b]133;A\x07\x1b]133;B\x07\x1b]133;C\x07\x1b]133;D;3\x07");
        assert_eq!(*events.borrow(), ["bell", "PromptStart", "InputStart", "OutputStart", "CommandEnd(Some(3))"]);
        // default colors are what OSC 11 reports
        let reply = Rc::new(RefCell::new(Vec::new()));
        let r = reply.clone();
        t.on_pty_write(move |d| r.borrow_mut().extend_from_slice(d));
        let pal = vec![[1u8, 2, 3]; 256];
        t.set_colors([10, 20, 30], [40, 50, 60], None, &pal);
        t.write(b"\x1b]11;?\x07");
        let got = String::from_utf8_lossy(&reply.borrow()).into_owned();
        assert!(got.contains("rgb:2828/3232/3c3c"), "{got:?}"); // 40, 50, 60 in hex
        t.set_scrollback_lines(Some(5));
        for i in 0..20000 {
            t.write(format!("line {i}\r\n").as_bytes());
        }
        assert!(t.scrollbar().total < 5000, "{:?}", t.scrollbar());
    }

    #[test]
    fn unknown_osc_reaches_the_host() {
        use std::{cell::RefCell, rc::Rc};
        let seen = Rc::new(RefCell::new(Vec::new()));
        let s2 = seen.clone();
        let mut t = Terminal::new(20, 3).unwrap();
        t.on_unknown_osc(move |c| s2.borrow_mut().push(String::from_utf8_lossy(c).into_owned()));
        t.write(b"\x1b]666;vte.shell.preexec!\x1b\\\x1b]666;vte.shell.postexec=3\x1b\\");
        assert_eq!(*seen.borrow(), ["666;vte.shell.preexec!", "666;vte.shell.postexec=3"]);
    }

    #[test]
    fn resize_reflows_wrapped_line() {
        let mut t = Terminal::new(10, 4).unwrap();
        t.write(b"0123456789abcdef");
        t.resize(20, 4, 8, 16).unwrap();
        assert_eq!(t.text(), "0123456789abcdef");
    }
}
