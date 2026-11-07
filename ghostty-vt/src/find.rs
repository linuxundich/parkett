//! Reading text back for search and links: rows as text, their cell columns, OSC 8 targets, and
//! selecting or scrolling to a place in the whole scrollback ("screen" rows count from the oldest
//! line; the viewport shows rows `offset..offset + rows`).
use crate::Terminal;
use ghostty_vt_sys as sys;
use std::ptr::{null, null_mut};

/// Scroll position: total rows (scrollback + screen), first visible row, visible rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scrollbar {
    pub total: u64,
    pub offset: u64,
    pub len: u64,
}

/// Start column of every character of `text` plus the column after the last one
/// (`result.len() == chars + 1`); wide characters take two columns, combining marks none.
pub fn char_columns(text: &str) -> Vec<u16> {
    let mut cols = Vec::with_capacity(text.chars().count() + 1);
    let mut x = 0u16;
    for c in text.chars() {
        cols.push(x);
        // SAFETY: pure function of the codepoint.
        x = x.saturating_add(unsafe { sys::ghostty_unicode_codepoint_width(c as u32) } as u16);
    }
    cols.push(x);
    cols
}

impl Terminal {
    pub fn scrollbar(&self) -> Scrollbar {
        let mut s = sys::GhosttyTerminalScrollbar { total: 0, offset: 0, len: 0 };
        // SAFETY: documented output type.
        unsafe {
            sys::ghostty_terminal_get(
                self.raw(),
                sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_SCROLLBAR,
                &mut s as *mut _ as *mut _,
            );
        }
        Scrollbar { total: s.total, offset: s.offset, len: s.len }
    }

    fn screen_ref(&self, x: u16, y: u64) -> Option<sys::GhosttyGridRef> {
        let p = sys::GhosttyPoint {
            tag: sys::GhosttyPointTag_GHOSTTY_POINT_TAG_SCREEN,
            value: sys::GhosttyPointValue { coordinate: sys::GhosttyPointCoordinate { x, y: y as u32 } },
        };
        let mut r: sys::GhosttyGridRef = unsafe { std::mem::zeroed() };
        // SAFETY: live terminal, valid out pointer.
        (unsafe { sys::ghostty_terminal_grid_ref(self.raw(), p, &mut r) } == 0).then_some(r)
    }

    /// Plain text of every row of the active screen including scrollback, one entry per row (soft
    /// wraps are *not* joined, trailing blanks trimmed). Empty rows give empty strings.
    pub fn screen_lines(&self) -> Vec<String> {
        let total = self.scrollbar().total;
        let (cols, _) = self.size();
        let mut f = null_mut();
        let mut out = Vec::with_capacity(total as usize);
        // SAFETY: handles live; the selection struct outlives each call; buffers freed after use.
        unsafe {
            if sys::ghostty_formatter_terminal_new(null(), &mut f, self.raw()) != 0 {
                return out;
            }
            let emit = sys::GhosttyFormatterFormat_GHOSTTY_FORMATTER_FORMAT_PLAIN;
            sys::ghostty_formatter_set(
                f,
                sys::GhosttyFormatterOption_GHOSTTY_FORMATTER_OPT_EMIT,
                &emit as *const _ as *const _,
            );
            for y in 0..total {
                let (Some(start), Some(end)) = (self.screen_ref(0, y), self.screen_ref(cols.saturating_sub(1), y))
                else {
                    out.push(String::new());
                    continue;
                };
                let sel = sys::GhosttySelection { start, end, rectangle: false };
                sys::ghostty_formatter_set(
                    f,
                    sys::GhosttyFormatterOption_GHOSTTY_FORMATTER_OPT_SELECTION,
                    &sel as *const _ as *const _,
                );
                let (mut p, mut n) = (null_mut(), 0usize);
                let r = sys::ghostty_formatter_format_alloc(f, null(), &mut p, &mut n);
                let line = if r == 0 && !p.is_null() {
                    String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).trim_end_matches('\n').to_string()
                } else {
                    String::new()
                };
                if !p.is_null() {
                    sys::ghostty_free(null(), p, n);
                }
                out.push(line);
            }
            sys::ghostty_formatter_free(f);
        }
        out
    }

    /// Select from screen cell `a` to `b` (both included, `y` in screen rows).
    pub fn select_screen_range(&mut self, a: (u16, u64), b: (u16, u64)) {
        let (Some(start), Some(end)) = (self.screen_ref(a.0, a.1), self.screen_ref(b.0, b.1)) else { return };
        let sel = sys::GhosttySelection { start, end, rectangle: false };
        // SAFETY: struct valid for the call.
        unsafe {
            sys::ghostty_terminal_set(
                self.raw(),
                sys::GhosttyTerminalOption_GHOSTTY_TERMINAL_OPT_SELECTION,
                &sel as *const _ as *const _,
            );
        }
    }

    /// Make screen row `y` the first visible row.
    pub fn scroll_to_row(&mut self, y: u64) {
        let b = sys::GhosttyTerminalScrollViewport {
            tag: sys::GhosttyTerminalScrollViewportTag_GHOSTTY_TERMINAL_SCROLL_VIEWPORT_ROW,
            value: sys::GhosttyTerminalScrollViewportValue { row: y as usize },
        };
        // SAFETY: live handle, plain value.
        unsafe {
            sys::ghostty_terminal_scroll_viewport(self.raw(), b);
        }
    }

    /// OSC 8 target of the cell at viewport position (`x`, `y`).
    pub fn hyperlink_at(&self, x: u16, y: u16) -> Option<String> {
        let p = sys::GhosttyPoint {
            tag: sys::GhosttyPointTag_GHOSTTY_POINT_TAG_VIEWPORT,
            value: sys::GhosttyPointValue { coordinate: sys::GhosttyPointCoordinate { x, y: y as u32 } },
        };
        let mut r: sys::GhosttyGridRef = unsafe { std::mem::zeroed() };
        let mut buf = [0u8; 4096];
        let mut n = 0usize;
        // SAFETY: live terminal; buffer and out pointers valid for the calls.
        unsafe {
            if sys::ghostty_terminal_grid_ref(self.raw(), p, &mut r) != 0 {
                return None;
            }
            if sys::ghostty_grid_ref_hyperlink_uri(&r, buf.as_mut_ptr(), buf.len(), &mut n) != 0 || n == 0 {
                return None;
            }
        }
        Some(String::from_utf8_lossy(&buf[..n.min(buf.len())]).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_columns_and_scrolling() {
        let mut t = Terminal::new(10, 3).unwrap();
        for i in 0..6 {
            t.write(format!("row{i}\r\n").as_bytes());
        }
        t.write("漢x".as_bytes());
        let sb = t.scrollbar();
        assert_eq!((sb.total, sb.len), (7, 3), "{sb:?}"); // 6 lines + the current one, 3 visible
        let lines = t.screen_lines();
        assert_eq!(lines.len() as u64, sb.total);
        assert_eq!(lines[0], "row0");
        assert_eq!(lines.last().unwrap(), "漢x");
        assert_eq!(char_columns("a漢x"), [0, 1, 3, 4]);
        t.scroll_to_row(0);
        assert_eq!(t.scrollbar().offset, 0);
        t.select_screen_range((0, 1), (3, 1));
        assert_eq!(t.selected_text(false).as_deref(), Some("row1"));
    }

    #[test]
    fn hyperlinks_by_osc8() {
        let mut t = Terminal::new(30, 2).unwrap();
        t.write(b"x \x1b]8;;https://example.org/a\x07link\x1b]8;;\x07 y");
        assert_eq!(t.hyperlink_at(3, 0).as_deref(), Some("https://example.org/a"));
        assert_eq!(t.hyperlink_at(0, 0), None);
        assert_eq!(t.hyperlink_at(10, 0), None);
    }
}
