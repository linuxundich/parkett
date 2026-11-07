//! Selection, copy and paste on top of the library's own selection state: the terminal keeps the
//! active selection (and keeps it attached to the text while output scrolls), the widget only
//! decides where it starts and ends.
use crate::Terminal;
use ghostty_vt_sys as sys;
use std::ptr::{null, null_mut};

/// Outcome of [`Terminal::paste`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteResult {
    /// Written to the program (bracketed if the program asked for it).
    Pasted,
    /// Nothing to paste.
    Empty,
    /// The text could run commands (e.g. a newline outside bracketed paste); ask the user, then
    /// call again with `allow_unsafe`.
    Unsafe,
    Failed(i32),
}

impl Terminal {
    fn viewport_ref(&self, x: u16, y: u16) -> Option<sys::GhosttyGridRef> {
        let p = sys::GhosttyPoint {
            tag: sys::GhosttyPointTag_GHOSTTY_POINT_TAG_VIEWPORT,
            value: sys::GhosttyPointValue { coordinate: sys::GhosttyPointCoordinate { x, y: y as u32 } },
        };
        let mut r: sys::GhosttyGridRef = unsafe { std::mem::zeroed() };
        // SAFETY: live terminal, valid out pointer.
        (unsafe { sys::ghostty_terminal_grid_ref(self.raw(), p, &mut r) } == 0).then_some(r)
    }

    fn set_selection(&mut self, sel: Option<&sys::GhosttySelection>) {
        let v = sel.map_or(null(), |s| s as *const _ as *const core::ffi::c_void);
        // SAFETY: NULL clears; otherwise the struct lives for the call.
        unsafe { sys::ghostty_terminal_set(self.raw(), sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_SELECTION, v) };
    }

    /// Select from cell `a` to cell `b` (viewport coordinates, both included).
    pub fn select_range(&mut self, a: (u16, u16), b: (u16, u16), rectangle: bool) {
        let (Some(start), Some(end)) = (self.viewport_ref(a.0, a.1), self.viewport_ref(b.0, b.1)) else { return };
        let sel = sys::GhosttySelection { start, end, rectangle };
        self.set_selection(Some(&sel));
    }

    pub fn select_word(&mut self, x: u16, y: u16) -> bool {
        let Some(r) = self.viewport_ref(x, y) else { return false };
        let opts = sys::GhosttyTerminalSelectWordOptions {
            size: std::mem::size_of::<sys::GhosttyTerminalSelectWordOptions>(),
            ref_: r,
            boundary_codepoints: null(),
            boundary_codepoints_len: 0,
        };
        let mut sel: sys::GhosttySelection = unsafe { std::mem::zeroed() };
        // SAFETY: options and out pointer valid for the call.
        if unsafe { sys::ghostty_terminal_select_word(self.raw(), &opts, &mut sel) } != 0 {
            return false;
        }
        self.set_selection(Some(&sel));
        true
    }

    pub fn select_line(&mut self, x: u16, y: u16) -> bool {
        let Some(r) = self.viewport_ref(x, y) else { return false };
        let opts = sys::GhosttyTerminalSelectLineOptions {
            size: std::mem::size_of::<sys::GhosttyTerminalSelectLineOptions>(),
            ref_: r,
            whitespace: null(),
            whitespace_len: 0,
            semantic_prompt_boundary: false,
        };
        let mut sel: sys::GhosttySelection = unsafe { std::mem::zeroed() };
        // SAFETY: options and out pointer valid for the call.
        if unsafe { sys::ghostty_terminal_select_line(self.raw(), &opts, &mut sel) } != 0 {
            return false;
        }
        self.set_selection(Some(&sel));
        true
    }

    pub fn select_all(&mut self) -> bool {
        let mut sel: sys::GhosttySelection = unsafe { std::mem::zeroed() };
        // SAFETY: valid out pointer.
        if unsafe { sys::ghostty_terminal_select_all(self.raw(), &mut sel) } != 0 {
            return false;
        }
        self.set_selection(Some(&sel));
        true
    }

    pub fn clear_selection(&mut self) {
        self.set_selection(None);
    }

    fn current_selection(&self) -> Option<sys::GhosttySelection> {
        let mut sel: sys::GhosttySelection = unsafe { std::mem::zeroed() };
        // SAFETY: valid out pointer of the documented type.
        let r = unsafe {
            sys::ghostty_terminal_get(
                self.raw(),
                sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_SELECTION,
                &mut sel as *mut _ as *mut _,
            )
        };
        (r == 0).then_some(sel)
    }

    pub fn has_selection(&self) -> bool {
        self.current_selection().is_some()
    }

    /// The selected text, as plain text or as HTML (colors and bold kept). Soft-wrapped lines are
    /// joined and trailing blanks trimmed.
    pub fn selected_text(&self, html: bool) -> Option<String> {
        let sel = self.current_selection()?;
        let mut f = null_mut();
        let (mut p, mut n) = (null_mut(), 0usize);
        // SAFETY: handles live; every pointer handed over outlives its call; buffer freed below.
        unsafe {
            if sys::ghostty_formatter_terminal_new(null(), &mut f, self.raw()) != 0 {
                return None;
            }
            let emit = if html {
                sys::GhosttyFormatterFormat_GHOSTTY_FORMATTER_FORMAT_HTML
            } else {
                sys::GhosttyFormatterFormat_GHOSTTY_FORMATTER_FORMAT_PLAIN
            };
            let yes = true;
            sys::ghostty_formatter_set(
                f,
                sys::GhosttyFormatterOption_GHOSTTY_FORMATTER_OPT_EMIT,
                &emit as *const _ as *const _,
            );
            sys::ghostty_formatter_set(
                f,
                sys::GhosttyFormatterOption_GHOSTTY_FORMATTER_OPT_UNWRAP,
                &yes as *const _ as *const _,
            );
            sys::ghostty_formatter_set(
                f,
                sys::GhosttyFormatterOption_GHOSTTY_FORMATTER_OPT_SELECTION,
                &sel as *const _ as *const _,
            );
            let r = sys::ghostty_formatter_format_alloc(f, null(), &mut p, &mut n);
            let out = (r == 0 && !p.is_null())
                .then(|| String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned());
            if !p.is_null() {
                sys::ghostty_free(null(), p, n);
            }
            sys::ghostty_formatter_free(f);
            out.filter(|s| !s.is_empty())
        }
    }

    /// Paste text the way the program wants it (bracketed paste when enabled). Replies go through
    /// the callback set with `on_pty_write`.
    pub fn paste(&mut self, text: &str, allow_unsafe: bool) -> PasteResult {
        if text.is_empty() {
            return PasteResult::Empty;
        }
        unsafe extern "C" fn read(ud: *mut core::ffi::c_void, mime: sys::GhosttyString, w: sys::GhosttyWriter) -> bool {
            let text = &*(ud as *const &str);
            let m = std::slice::from_raw_parts(mime.ptr, mime.len);
            if m != b"text/plain" {
                return false;
            }
            match w.write {
                Some(write) => write(w.userdata, text.as_ptr(), text.len()),
                None => false,
            }
        }
        let mime = sys::GhosttyString { ptr: b"text/plain".as_ptr(), len: 10 };
        let mut t: &str = text;
        let paste = sys::GhosttyPaste {
            size: std::mem::size_of::<sys::GhosttyPaste>(),
            location: sys::GhosttyClipboardLocation_GHOSTTY_CLIPBOARD_LOCATION_STANDARD,
            source: sys::GhosttyPasteSource_GHOSTTY_PASTE_SOURCE_CLIPBOARD,
            mimes: &mime,
            mimes_len: 1,
            reader: sys::GhosttyMimeReader { read: Some(read), userdata: &mut t as *mut &str as *mut _ },
            allow_unsafe,
        };
        let mut pasted = false;
        // SAFETY: `paste`, `mime` and `t` outlive the call; the reader only reads `t`.
        let r = unsafe { sys::ghostty_terminal_paste(self.raw(), &paste, &mut pasted) };
        match r {
            0 if pasted => PasteResult::Pasted,
            0 => PasteResult::Empty,
            sys::GhosttyResult_GHOSTTY_REJECTED => PasteResult::Unsafe,
            e => PasteResult::Failed(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    fn term_with(text: &[u8]) -> Terminal {
        let mut t = Terminal::new(30, 4).unwrap();
        t.write(text);
        t
    }

    #[test]
    fn range_word_line_and_all() {
        let mut t = term_with(b"hello world foo\r\nsecond line");
        assert!(!t.has_selection());
        t.select_range((6, 0), (10, 0), false);
        assert_eq!(t.selected_text(false).as_deref(), Some("world"));
        assert!(t.select_word(1, 0));
        assert_eq!(t.selected_text(false).as_deref(), Some("hello"));
        assert!(t.select_line(3, 1));
        assert_eq!(t.selected_text(false).as_deref(), Some("second line"));
        assert!(t.select_all());
        assert_eq!(t.selected_text(false).as_deref(), Some("hello world foo\nsecond line"));
        t.clear_selection();
        assert!(!t.has_selection());
        assert_eq!(t.selected_text(false), None);
    }

    #[test]
    fn html_keeps_styling_and_rectangle_selects_a_block() {
        let mut t = term_with(b"\x1b[1;31mred\x1b[0m plain\r\nabcdef");
        t.select_range((0, 0), (2, 0), false);
        let h = t.selected_text(true).unwrap();
        assert!(h.contains("red") && h.contains('<'), "{h}");
        t.select_range((1, 0), (2, 1), true);
        assert_eq!(t.selected_text(false).as_deref(), Some("ed\nbc"));
    }

    #[test]
    fn paste_plain_bracketed_and_unsafe() {
        let sent = Rc::new(RefCell::new(Vec::new()));
        let s = sent.clone();
        let mut t = Terminal::new(30, 4).unwrap();
        t.on_pty_write(move |d| s.borrow_mut().extend_from_slice(d));
        assert_eq!(t.paste("ls -l", false), PasteResult::Pasted);
        assert_eq!(&sent.borrow()[..], b"ls -l");
        sent.borrow_mut().clear();
        assert_eq!(t.paste("rm x\n", false), PasteResult::Unsafe);
        assert!(sent.borrow().is_empty());
        assert_eq!(t.paste("rm x\n", true), PasteResult::Pasted);
        sent.borrow_mut().clear();
        t.write(b"\x1b[?2004h"); // bracketed paste on: newline is safe, text is framed
        assert_eq!(t.paste("a\nb", false), PasteResult::Pasted);
        assert_eq!(&sent.borrow()[..], b"\x1b[200~a\nb\x1b[201~");
        assert_eq!(t.paste("", false), PasteResult::Empty);
    }
}
