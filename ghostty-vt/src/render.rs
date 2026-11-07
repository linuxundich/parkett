//! Render state: an owned snapshot of what a frame needs (cells with text and resolved colors,
//! cursor, default colors). Copying instead of borrowing keeps the GTK side free of FFI lifetimes;
//! per-row dirty tracking can replace the full copy once drawing is measured (phase 3b).
use crate::Terminal;
use ghostty_vt_sys as sys;
use std::ptr::{null, null_mut};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<sys::GhosttyColorRgb> for Rgb {
    fn from(c: sys::GhosttyColorRgb) -> Self {
        Rgb { r: c.r, g: c.g, b: c.b }
    }
}

pub const BOLD: u16 = 1 << 0;
pub const ITALIC: u16 = 1 << 1;
pub const FAINT: u16 = 1 << 2;
pub const BLINK: u16 = 1 << 3;
pub const INVERSE: u16 = 1 << 4;
pub const INVISIBLE: u16 = 1 << 5;
pub const STRIKETHROUGH: u16 = 1 << 6;
pub const OVERLINE: u16 = 1 << 7;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    /// Grapheme cluster as UTF-8; empty for blank cells and for the tail of a wide character.
    pub text: String,
    /// `None`: use the frame's default colors.
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
    /// `BOLD`, `ITALIC`, … (bits 0–7 of the library's style flags).
    pub flags: u16,
    /// 0 none, then single, double, curly, dotted, dashed (SGR underline values).
    pub underline: u8,
    pub selected: bool,
    /// Occupies two columns (the next cell is its `spacer`).
    pub wide: bool,
    /// Right half of a wide character (or the spacer at a line end); draw nothing.
    pub spacer: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorShape {
    Bar,
    Block,
    Underline,
    BlockHollow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub x: u16,
    pub y: u16,
    pub shape: CursorShape,
    pub blinking: bool,
    pub wide_tail: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub cols: u16,
    pub rows: Vec<Vec<Cell>>,
    pub fg: Rgb,
    pub bg: Rgb,
    pub cursor_color: Option<Rgb>,
    /// `None` when hidden or scrolled out of the viewport.
    pub cursor: Option<Cursor>,
    pub palette: Vec<Rgb>,
}

impl Frame {
    /// Row as plain text with trailing blanks removed (for tests and accessibility later).
    pub fn row_text(&self, y: usize) -> String {
        let mut s: String = self.rows[y]
            .iter()
            .filter(|c| !c.spacer)
            .map(|c| if c.text.is_empty() { " " } else { c.text.as_str() })
            .collect();
        s.truncate(s.trim_end().len());
        s
    }
}

pub struct RenderState {
    raw: sys::GhosttyRenderState,
    rows: sys::GhosttyRenderStateRowIterator,
    cells: sys::GhosttyRenderStateRowCells,
}

fn get<T>(st: sys::GhosttyRenderState, what: sys::GhosttyRenderStateData, out: &mut T) -> sys::GhosttyResult {
    // SAFETY: `out` has the type the header documents for `what` (callers below).
    unsafe { sys::ghostty_render_state_get(st, what, out as *mut T as *mut _) }
}

impl RenderState {
    pub fn new() -> Result<Self, i32> {
        let (mut raw, mut rows, mut cells) = (null_mut(), null_mut(), null_mut());
        // SAFETY: out pointers are valid, NULL allocator = default.
        unsafe {
            for r in [
                sys::ghostty_render_state_new(null(), &mut raw),
                sys::ghostty_render_state_row_iterator_new(null(), &mut rows),
                sys::ghostty_render_state_row_cells_new(null(), &mut cells),
            ] {
                if r != 0 {
                    return Err(r);
                }
            }
        }
        Ok(Self { raw, rows, cells })
    }

    /// Capture the current screen of `term` as an owned `Frame` and mark it clean.
    pub fn frame(&mut self, term: &Terminal) -> Result<Frame, i32> {
        // SAFETY: both handles are live.
        let r = unsafe { sys::ghostty_render_state_update(self.raw, term.raw()) };
        if r != 0 {
            return Err(r);
        }
        let mut colors: sys::GhosttyRenderStateColors = unsafe { std::mem::zeroed() };
        colors.size = std::mem::size_of::<sys::GhosttyRenderStateColors>();
        let r = get(self.raw, sys::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_COLORS, &mut colors);
        if r != 0 {
            return Err(r);
        }
        let mut cur: sys::GhosttyRenderStateCursor = unsafe { std::mem::zeroed() };
        cur.size = std::mem::size_of::<sys::GhosttyRenderStateCursor>();
        let r = get(self.raw, sys::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_CURSOR, &mut cur);
        if r != 0 {
            return Err(r);
        }
        let cursor = (cur.visible && cur.viewport_has_value).then_some(Cursor {
            x: cur.viewport_x,
            y: cur.viewport_y,
            shape: match cur.visual_style {
                sys::GhosttyCursorStyle_GHOSTTY_CURSOR_STYLE_BAR => CursorShape::Bar,
                sys::GhosttyCursorStyle_GHOSTTY_CURSOR_STYLE_UNDERLINE => CursorShape::Underline,
                sys::GhosttyCursorStyle_GHOSTTY_CURSOR_STYLE_BLOCK_HOLLOW => CursorShape::BlockHollow,
                _ => CursorShape::Block,
            },
            blinking: cur.blinking,
            wide_tail: cur.wide_tail,
        });

        let mut frame = Frame {
            fg: colors.foreground.into(),
            bg: colors.background.into(),
            cursor_color: colors.cursor_has_value.then(|| colors.cursor.into()),
            palette: colors.palette.iter().map(|&c| c.into()).collect(),
            cursor,
            ..Default::default()
        };
        let mut cols = 0u16;
        get(self.raw, sys::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_COLS, &mut cols);
        frame.cols = cols;

        let r = get(self.raw, sys::GhosttyRenderStateData_GHOSTTY_RENDER_STATE_DATA_ROW_ITERATOR, &mut self.rows);
        if r != 0 {
            return Err(r);
        }
        // SAFETY: iterator and cells handles are live and re-pointed at the fresh update.
        unsafe {
            while sys::ghostty_render_state_row_iterator_next(self.rows) {
                let r = sys::ghostty_render_state_row_get(
                    self.rows,
                    sys::GhosttyRenderStateRowData_GHOSTTY_RENDER_STATE_ROW_DATA_CELLS,
                    &mut self.cells as *mut _ as *mut _,
                );
                if r != 0 {
                    return Err(r);
                }
                let mut row = Vec::with_capacity(cols as usize);
                while sys::ghostty_render_state_row_cells_next(self.cells) {
                    row.push(self.cell());
                }
                frame.rows.push(row);
            }
            sys::ghostty_render_state_clean(self.raw);
        }
        Ok(frame)
    }

    unsafe fn cell(&self) -> Cell {
        use sys::*;
        let c = self.cells;
        let mut cell = Cell::default();
        let mut raw: GhosttyCell = std::mem::zeroed();
        if ghostty_render_state_row_cells_get(
            c,
            GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_RAW,
            &mut raw as *mut _ as *mut _,
        ) == 0
        {
            let mut w: GhosttyCellWide = 0;
            ghostty_cell_get(raw, GhosttyCellData_GHOSTTY_CELL_DATA_WIDE, &mut w as *mut _ as *mut _);
            cell.wide = w == GhosttyCellWide_GHOSTTY_CELL_WIDE_WIDE;
            cell.spacer = w == GhosttyCellWide_GHOSTTY_CELL_WIDE_SPACER_TAIL
                || w == GhosttyCellWide_GHOSTTY_CELL_WIDE_SPACER_HEAD;
        }
        let mut len = 0u32;
        ghostty_render_state_row_cells_get(
            c,
            GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_LEN,
            &mut len as *mut _ as *mut _,
        );
        if len > 0 {
            let mut buf = [0u8; 64];
            let mut b = GhosttyBuffer { ptr: buf.as_mut_ptr(), cap: buf.len(), len: 0 };
            let r = ghostty_render_state_row_cells_get(
                c,
                GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_UTF8,
                &mut b as *mut _ as *mut _,
            );
            if r == 0 {
                cell.text = String::from_utf8_lossy(&buf[..b.len]).into_owned();
            } else {
                // Long cluster: ask for the size, then allocate.
                let mut big = vec![0u8; b.len.max(1)];
                let mut b2 = GhosttyBuffer { ptr: big.as_mut_ptr(), cap: big.len(), len: 0 };
                if ghostty_render_state_row_cells_get(
                    c,
                    GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_GRAPHEMES_UTF8,
                    &mut b2 as *mut _ as *mut _,
                ) == 0
                {
                    cell.text = String::from_utf8_lossy(&big[..b2.len]).into_owned();
                }
            }
        }
        let mut rgb = GhosttyColorRgb { r: 0, g: 0, b: 0 };
        if ghostty_render_state_row_cells_get(
            c,
            GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_FG_COLOR,
            &mut rgb as *mut _ as *mut _,
        ) == 0
        {
            cell.fg = Some(rgb.into());
        }
        if ghostty_render_state_row_cells_get(
            c,
            GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_BG_COLOR,
            &mut rgb as *mut _ as *mut _,
        ) == 0
        {
            cell.bg = Some(rgb.into());
        }
        let mut styled = false;
        ghostty_render_state_row_cells_get(
            c,
            GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_HAS_STYLING,
            &mut styled as *mut _ as *mut _,
        );
        if styled {
            let mut st: GhosttyStyle = std::mem::zeroed();
            ghostty_render_state_row_cells_get(
                c,
                GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_STYLE,
                &mut st as *mut _ as *mut _,
            );
            cell.flags = st.flags & 0xff;
            cell.underline = ((st.flags >> 8) & 0x7) as u8;
        }
        ghostty_render_state_row_cells_get(
            c,
            GhosttyRenderStateRowCellsData_GHOSTTY_RENDER_STATE_ROW_CELLS_DATA_SELECTED,
            &mut cell.selected as *mut _ as *mut _,
        );
        cell
    }
}

impl Drop for RenderState {
    fn drop(&mut self) {
        // SAFETY: handles came from the matching *_new calls and are freed once.
        unsafe {
            sys::ghostty_render_state_row_cells_free(self.cells);
            sys::ghostty_render_state_row_iterator_free(self.rows);
            sys::ghostty_render_state_free(self.raw);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styled_cells_cursor_and_wide_char() {
        let mut t = Terminal::new(20, 3).unwrap();
        t.write("a\x1b[1;32mgrün\x1b[0m \x1b[4mU\x1b[0m \x1b[48;2;1;2;3mB\x1b[0m 漢x".as_bytes());
        let mut rs = RenderState::new().unwrap();
        let f = rs.frame(&t).unwrap();
        assert_eq!((f.cols, f.rows.len()), (20, 3));
        assert_eq!(f.row_text(0), "agrün U B 漢x");
        let g = &f.rows[0][1];
        assert_eq!(g.text, "g");
        assert!(g.flags & BOLD != 0);
        assert!(g.fg.is_some(), "palette green is resolved");
        assert_eq!(f.rows[0][0].fg, None);
        assert_eq!(f.rows[0][6].underline, 1);
        assert_eq!(f.rows[0][8].bg, Some(Rgb { r: 1, g: 2, b: 3 }));
        // wide character: base cell has text, tail cell is empty
        assert_eq!(f.rows[0][10].text, "漢");
        assert!(f.rows[0][10].wide && f.rows[0][11].spacer);
        assert_eq!(f.rows[0][12].text, "x");
        let c = f.cursor.unwrap();
        assert_eq!((c.x, c.y), (13, 0));
        assert_eq!(f.palette.len(), 256);
    }

    #[test]
    fn selection_marks_cells() {
        let mut t = Terminal::new(20, 3).unwrap();
        t.write(b"hello world");
        t.select_range((2, 0), (4, 0), false);
        let mut rs = RenderState::new().unwrap();
        let f = rs.frame(&t).unwrap();
        let sel: Vec<bool> = f.rows[0].iter().take(7).map(|c| c.selected).collect();
        assert_eq!(sel, [false, false, true, true, true, false, false]);
    }

    #[test]
    fn select_all_marks_text_cells() {
        let mut t = Terminal::new(20, 3).unwrap();
        t.write(b"$ echo hi\r\n$ ");
        assert!(t.select_all());
        let mut rs = RenderState::new().unwrap();
        let f = rs.frame(&t).unwrap();
        let sel: Vec<bool> = f.rows[0].iter().take(10).map(|c| c.selected).collect();
        assert!(sel[..9].iter().all(|&b| b), "{sel:?}");
    }
}
