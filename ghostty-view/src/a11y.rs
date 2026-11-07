//! What a screen reader sees: the visible rows as text, the caret and the selection, with all
//! offsets in characters (`GtkAccessibleText`). Pure functions on the render frame, so they are
//! tested without a display; `imp.rs` wires them to the interface.
use ghostty_vt::render::Frame;

/// The visible screen as text lines plus the maps between offsets and cells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// One entry per row, trailing blanks removed.
    pub lines: Vec<String>,
    /// Character offset of the first character of every line (lines are joined by `\n`).
    starts: Vec<u32>,
    /// For every line: the cell column where each character starts (len == chars).
    cols: Vec<Vec<u16>>,
    caret: u32,
    selection: Option<(u32, u32)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granularity {
    Character,
    Word,
    /// Sentence, line and paragraph are all one screen row here.
    Line,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    Remove(u32, u32),
    Insert(u32, u32),
}

fn chars(s: &str) -> u32 {
    s.chars().count() as u32
}

impl Snapshot {
    pub fn from_frame(f: &Frame) -> Self {
        let mut s = Snapshot::default();
        let mut offset = 0u32;
        // (row, char index inside the row) of the selection ends
        let mut sel_first: Option<(usize, u32)> = None;
        let mut sel_last: Option<(usize, u32)> = None;
        for (y, row) in f.rows.iter().enumerate() {
            let (mut line, mut cols) = (String::new(), Vec::new());
            let mut per_cell: Vec<(u16, u32, bool)> = Vec::new(); // column, char index after this cell, selected
            for (x, c) in row.iter().enumerate() {
                if c.spacer {
                    continue;
                }
                let t = if c.text.is_empty() { " " } else { c.text.as_str() };
                for _ in t.chars() {
                    cols.push(x as u16);
                }
                line.push_str(t);
                per_cell.push((x as u16, line.chars().count() as u32, c.selected));
            }
            // trim trailing blanks (and the columns that go with them)
            let trimmed = line.trim_end().to_string();
            let n = trimmed.chars().count();
            cols.truncate(n);
            let n = n as u32;
            for (_, end, selected) in &per_cell {
                if *selected {
                    let start = end.saturating_sub(1).min(n);
                    sel_first.get_or_insert((y, start));
                    sel_last = Some((y, (*end).min(n)));
                }
            }
            if let Some(c) = f.cursor.filter(|c| c.y as usize == y) {
                let before = per_cell.iter().take_while(|(col, _, _)| *col < c.x).last().map_or(0, |(_, e, _)| *e);
                s.caret = offset + before.min(n);
            }
            s.starts.push(offset);
            offset += n + 1;
            s.lines.push(trimmed);
            s.cols.push(cols);
        }
        s.selection = match (sel_first, sel_last) {
            (Some((y0, c0)), Some((y1, c1))) => {
                let (a, b) = (s.starts[y0] + c0, s.starts[y1] + c1);
                (b > a).then_some((a, b))
            }
            _ => None,
        };
        s
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn char_count(&self) -> u32 {
        self.starts.last().map_or(0, |s| s + chars(self.lines.last().map_or("", |l| l.as_str())))
    }

    pub fn caret(&self) -> u32 {
        self.caret
    }

    pub fn selection(&self) -> Option<(u32, u32)> {
        self.selection
    }

    /// Characters `start..end` (clamped).
    pub fn contents(&self, start: u32, end: u32) -> String {
        self.text().chars().skip(start as usize).take(end.saturating_sub(start) as usize).collect()
    }

    fn row_of(&self, offset: u32) -> usize {
        self.starts.partition_point(|s| *s <= offset).saturating_sub(1)
    }

    /// The unit around `offset`: (start, end, text).
    pub fn contents_at(&self, offset: u32, g: Granularity) -> (u32, u32, String) {
        if self.lines.is_empty() {
            return (0, 0, String::new());
        }
        let total = self.char_count();
        let offset = offset.min(total);
        let row = self.row_of(offset);
        let (rs, line) = (self.starts[row], &self.lines[row]);
        let (start, end) = match g {
            Granularity::Character => (offset, (offset + 1).min(total)),
            Granularity::Line => (rs, rs + chars(line)),
            Granularity::Word => {
                let cs: Vec<char> = line.chars().collect();
                let i = ((offset - rs) as usize).min(cs.len());
                let word = |c: &char| c.is_alphanumeric() || *c == '_';
                let kind = |i: usize| cs.get(i).map(|c| (word(c), c.is_whitespace()));
                let k = kind(i);
                let (mut a, mut b) = (i, i);
                while a > 0 && kind(a - 1) == k {
                    a -= 1;
                }
                while b < cs.len() && kind(b) == k {
                    b += 1;
                }
                (rs + a as u32, rs + b as u32)
            }
        };
        (start, end, self.contents(start, end))
    }

    /// Row and cell column of a character offset.
    pub fn locate(&self, offset: u32) -> (usize, u16) {
        if self.lines.is_empty() {
            return (0, 0);
        }
        let row = self.row_of(offset);
        let i = (offset - self.starts[row].min(offset)) as usize;
        let col = self.cols[row].get(i).copied().unwrap_or_else(|| self.cols[row].last().map_or(0, |c| c + 1));
        (row, col)
    }

    /// Character offset of the cell (`row`, `col`), clamped to the text.
    pub fn offset_at(&self, row: usize, col: u16) -> u32 {
        let Some(cols) = self.cols.get(row) else { return self.char_count() };
        let i = cols.partition_point(|c| *c < col);
        let i = if cols.get(i).is_some_and(|c| *c == col) { i } else { i.saturating_sub(1) };
        self.starts[row] + (i as u32).min(chars(&self.lines[row]))
    }
}

/// Edits that turn `old` into `new` (one text change per changed row, or everything at once when
/// many rows changed, e.g. after scrolling). Offsets are valid when applied in order.
pub fn diff(old: &[String], new: &[String]) -> Vec<Edit> {
    let join_len = |l: &[String]| l.iter().map(|s| chars(s) + 1).sum::<u32>().saturating_sub(1);
    let changed: Vec<usize> = (0..old.len().max(new.len())).filter(|&i| old.get(i) != new.get(i)).collect();
    if changed.is_empty() {
        return Vec::new();
    }
    if old.len() != new.len() || changed.len() > 4 {
        let mut e = Vec::new();
        if join_len(old) > 0 {
            e.push(Edit::Remove(0, join_len(old)));
        }
        if join_len(new) > 0 {
            e.push(Edit::Insert(0, join_len(new)));
        }
        return e;
    }
    let mut edits = Vec::new();
    for i in changed {
        // everything above row i is already the new text, so its start is the same in both
        let start: u32 = new[..i].iter().map(|s| chars(s) + 1).sum();
        let (o, n) = (chars(&old[i]), chars(&new[i]));
        if o > 0 {
            edits.push(Edit::Remove(start, start + o));
        }
        if n > 0 {
            edits.push(Edit::Insert(start, start + n));
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use ghostty_vt::render::{Cell, Frame};
    use ghostty_vt::render::{Cursor, CursorShape};

    fn frame(rows: &[&str], cursor: Option<(u16, u16)>, selected: Option<(usize, usize, usize)>) -> Frame {
        let mut f = Frame::default();
        for (y, r) in rows.iter().enumerate() {
            let mut cells = Vec::new();
            for ch in r.chars() {
                let wide = ch as u32 > 0x2e80;
                cells.push(Cell {
                    text: if ch == ' ' { String::new() } else { ch.to_string() },
                    wide,
                    ..Default::default()
                });
                if wide {
                    cells.push(Cell { spacer: true, ..Default::default() });
                }
            }
            while cells.len() < 12 {
                cells.push(Cell::default());
            }
            if let Some((sy, x0, x1)) = selected {
                if sy == y {
                    for c in cells.iter_mut().take(x1 + 1).skip(x0) {
                        c.selected = true;
                    }
                }
            }
            f.rows.push(cells);
        }
        f.cursor = cursor.map(|(x, y)| Cursor { x, y, shape: CursorShape::Block, blinking: false, wide_tail: false });
        f
    }

    #[test]
    fn text_caret_and_selection() {
        let f = frame(&["$ ls", "foo bar", ""], Some((7, 1)), Some((1, 4, 6)));
        let s = Snapshot::from_frame(&f);
        assert_eq!(s.text(), "$ ls\nfoo bar\n");
        assert_eq!(s.char_count(), 13);
        assert_eq!(s.caret(), 5 + 7);
        assert_eq!(s.selection(), Some((9, 12)));
        assert_eq!(s.contents(9, 12), "bar");
        assert_eq!(s.contents(0, 4), "$ ls");
    }

    #[test]
    fn units_around_an_offset() {
        let s = Snapshot::from_frame(&frame(&["$ ls -l", "foo bar"], None, None));
        assert_eq!(s.contents_at(9, Granularity::Line), (8, 15, "foo bar".to_string()));
        assert_eq!(s.contents_at(10, Granularity::Word), (8, 11, "foo".to_string()));
        assert_eq!(s.contents_at(11, Granularity::Word), (11, 12, " ".to_string()));
        assert_eq!(s.contents_at(2, Granularity::Character), (2, 3, "l".to_string()));
        assert_eq!(s.contents_at(5, Granularity::Word), (5, 6, "-".to_string()));
    }

    #[test]
    fn offsets_and_cells_with_wide_characters() {
        let s = Snapshot::from_frame(&frame(&["a漢b"], None, None));
        assert_eq!(s.text(), "a漢b");
        assert_eq!(s.locate(2), (0, 3), "b starts in column 3");
        assert_eq!(s.offset_at(0, 3), 2);
        assert_eq!(s.offset_at(0, 1), 1);
        assert_eq!(s.offset_at(0, 2), 1, "second half of a wide character");
    }

    #[test]
    fn diff_edits() {
        let l = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(diff(&l(&["a", "b"]), &l(&["a", "b"])).is_empty());
        assert_eq!(diff(&l(&["ab", "cd"]), &l(&["ab", "xyz"])), [Edit::Remove(3, 5), Edit::Insert(3, 6)]);
        assert_eq!(diff(&l(&["a", ""]), &l(&["a", "b"])), [Edit::Insert(2, 3)]);
        let many = diff(&l(&["1", "2", "3", "4", "5", "6"]), &l(&["2", "3", "4", "5", "6", "7"]));
        assert_eq!(many, [Edit::Remove(0, 11), Edit::Insert(0, 11)]);
    }
}
