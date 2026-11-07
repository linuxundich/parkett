//! Detecting clickable text in a row: URLs, mail addresses, file paths and the profile's custom
//! links (PCRE2 through `glib::Regex`, the same dialect VTE used). OSC 8 hyperlinks are handled
//! by the widget with the terminal's own data; they win over detected text.
use ghostty_vt::render::Cell;
use gtk::glib;
use parkett_core::link::{self, Action, CustomLink};

/// Addresses without OSC 8; trailing punctuation does not belong to them.
pub const URL_PATTERNS: &[&str] = &[
    r#"\b(?:https?|ftp|file)://[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}]"#,
    r#"\bmailto:[^\s<>"'`@]+@[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}]"#,
    // File paths: /…, ~/…, ./…, ../…, also with :line:column; not in the middle of words,
    // addresses (`https://`) or dates (`10/23`)
    r#"(?<![\w/~.:@-])(?:~|\.\.?)?/[\w.+@%~/-]*[\w+@%~/-](?::\d+){0,2}"#,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    /// An address (OSC 8 or detected): open it with the default application.
    Uri(String),
    /// A detected file path, as written in the output.
    Path(String),
    /// A custom link of the profile with its groups already filled in.
    Custom(Action),
}

/// A link and the cells it covers in one row (`x1` exclusive).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub y: u16,
    pub x0: u16,
    pub x1: u16,
    pub link: Link,
}

/// Row text with the column of every cell, so match offsets become columns.
pub struct RowText {
    pub text: String,
    /// (byte offset in `text`, column, width in columns) per drawn cell.
    cells: Vec<(usize, u16, u16)>,
}

impl RowText {
    pub fn new(row: &[Cell]) -> Self {
        let mut text = String::new();
        let mut cells = Vec::with_capacity(row.len());
        for (x, c) in row.iter().enumerate() {
            if c.spacer {
                continue;
            }
            cells.push((text.len(), x as u16, if c.wide { 2 } else { 1 }));
            text.push_str(if c.text.is_empty() { " " } else { &c.text });
        }
        Self { text, cells }
    }

    fn col_at(&self, byte: usize) -> u16 {
        let i = self.cells.partition_point(|c| c.0 < byte);
        match self.cells.get(i) {
            Some(c) => c.1,
            None => self.cells.last().map_or(0, |c| c.1 + c.2),
        }
    }

    /// Columns covered by bytes `start..end`.
    fn cols(&self, start: usize, end: usize) -> (u16, u16) {
        (self.col_at(start), self.col_at(end))
    }
}

pub struct Detector {
    builtin: Vec<(glib::Regex, bool)>, // regex, is_path
    custom: Vec<(glib::Regex, CustomLink)>,
    /// Patterns registered by the host (tag, regex): the host decides what a match means.
    patterns: Vec<(i32, glib::Regex)>,
    next_tag: i32,
    use_builtin: bool,
}

fn compile(pattern: &str) -> Option<glib::Regex> {
    match glib::Regex::new(pattern, glib::RegexCompileFlags::OPTIMIZE, glib::RegexMatchFlags::empty()) {
        Ok(Some(re)) => Some(re),
        Ok(None) => None,
        Err(e) => {
            glib::g_warning!("parkett", "Link pattern “{pattern}”: {e}");
            None
        }
    }
}

impl Default for Detector {
    fn default() -> Self {
        let builtin = URL_PATTERNS.iter().enumerate().filter_map(|(i, p)| Some((compile(p)?, i == 2))).collect();
        Self { builtin, custom: Vec::new(), patterns: Vec::new(), next_tag: 1, use_builtin: true }
    }
}

impl Detector {
    /// Detect URLs, mail addresses and paths by itself (default) or leave that to patterns the
    /// host registers with `add_pattern`.
    pub fn set_builtin(&mut self, on: bool) {
        self.use_builtin = on;
    }

    pub fn add_pattern(&mut self, pattern: &str) -> Result<i32, String> {
        let re = glib::Regex::new(pattern, glib::RegexCompileFlags::OPTIMIZE, glib::RegexMatchFlags::empty())
            .map_err(|e| e.message().to_string())?
            .ok_or_else(|| String::from("invalid pattern"))?;
        let tag = self.next_tag;
        self.next_tag += 1;
        self.patterns.push((tag, re));
        Ok(tag)
    }

    pub fn remove_pattern(&mut self, tag: i32) {
        self.patterns.retain(|(t, _)| *t != tag);
    }

    /// Host pattern under `col`: the matched text, its tag and the columns it covers.
    pub fn find_pattern(&self, row: &RowText, col: u16) -> Option<(String, i32, u16, u16)> {
        for (tag, re) in &self.patterns {
            if let Some((x0, x1, groups)) = find_at(re, row, col) {
                return Some((groups[0].clone().unwrap_or_default(), *tag, x0, x1));
            }
        }
        None
    }

    pub fn set_custom(&mut self, links: &[CustomLink]) {
        self.custom =
            links.iter().filter(|l| l.is_usable()).filter_map(|l| Some((compile(&l.pattern)?, l.clone()))).collect();
    }

    /// The link under column `col` of `row` (custom links first, then addresses, then paths).
    pub fn find(&self, y: u16, row: &RowText, col: u16) -> Option<Span> {
        for (re, cl) in &self.custom {
            if let Some((x0, x1, groups)) = find_at(re, row, col) {
                let g: Vec<Option<&str>> = groups.iter().map(|s| s.as_deref()).collect();
                return Some(Span { y, x0, x1, link: Link::Custom(link::expand(&cl.target, &g)) });
            }
        }
        for (re, is_path) in self.builtin.iter().filter(|_| self.use_builtin) {
            if let Some((x0, x1, groups)) = find_at(re, row, col) {
                let text = groups[0].clone().unwrap_or_default();
                let link = if *is_path { Link::Path(text) } else { Link::Uri(text) };
                return Some(Span { y, x0, x1, link });
            }
        }
        None
    }
}

/// First match of `re` in the row that covers `col`: columns and groups 0..=9.
fn find_at(re: &glib::Regex, row: &RowText, col: u16) -> Option<(u16, u16, Vec<Option<String>>)> {
    let mut pos = 0;
    while pos <= row.text.len() {
        let gs = glib::GString::from(row.text.as_str());
        let Ok(info) = re.match_full(gs.as_gstr(), pos as i32, glib::RegexMatchFlags::empty()) else {
            return None;
        };
        if !info.matches() {
            return None;
        }
        let (s, e) = info.fetch_pos(0)?;
        let (x0, x1) = row.cols(s as usize, e as usize);
        if (x0..x1).contains(&col) {
            let groups = (0..10).map(|i| info.fetch(i).map(|g| g.to_string())).collect();
            return Some((x0, x1, groups));
        }
        if e as usize <= pos && s as usize >= row.text.len() {
            return None;
        }
        pos = (e as usize).max(pos + 1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str) -> Vec<Cell> {
        text.chars().map(|c| Cell { text: c.to_string(), ..Default::default() }).collect()
    }

    fn find(det: &Detector, text: &str, col: u16) -> Option<Span> {
        det.find(0, &RowText::new(&row(text)), col)
    }

    #[test]
    fn urls_mail_and_paths() {
        let det = Detector::default();
        let t = "see https://example.org/a?b=1, or mail:x and ~/dir/file.rs:12:3 now";
        let s = find(&det, t, 8).unwrap();
        assert_eq!(s.link, Link::Uri("https://example.org/a?b=1".into()));
        assert_eq!((s.x0, s.x1), (4, 29), "trailing comma excluded");
        assert_eq!(find(&det, t, 2), None);
        let p = find(&det, t, 45).unwrap();
        assert_eq!(p.link, Link::Path("~/dir/file.rs:12:3".into()));
        assert_eq!(find(&det, "date 10/23 ok", 6), None, "not a path");
        assert!(find(&det, "mailto:a@b.de", 3).is_some());
    }

    #[test]
    fn custom_links_with_groups_and_wide_cells() {
        let mut det = Detector::default();
        det.set_custom(&[CustomLink { pattern: r"\bPARK-(\d+)\b".into(), target: "https://t.example/$1".into() }]);
        // a wide character before the match shifts the columns by one
        let mut r = row("漢 PARK-42");
        r[0].wide = true;
        r.insert(1, Cell { spacer: true, ..Default::default() });
        let s = det.find(0, &RowText::new(&r), 5).unwrap();
        assert_eq!(s.link, Link::Custom(Action::Open("https://t.example/42".into())));
        assert_eq!((s.x0, s.x1), (3, 10));
    }
}
