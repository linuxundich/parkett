//! Search over the whole scrollback with PCRE2 (`glib::Regex`): regular expression, case, whole
//! word are the caller's choice of pattern and flag. Matches do not cross row ends (soft-wrapped
//! lines are matched per row).
use ghostty_vt::find::char_columns;
use gtk::glib;

/// A hit: screen row `y` (counted from the oldest line), columns `x0..x1` (`x1` exclusive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    pub y: u64,
    pub x0: u16,
    pub x1: u16,
}

pub fn compile(pattern: &str, case_sensitive: bool) -> Result<glib::Regex, String> {
    let mut flags = glib::RegexCompileFlags::OPTIMIZE;
    if !case_sensitive {
        flags |= glib::RegexCompileFlags::CASELESS;
    }
    match glib::Regex::new(pattern, flags, glib::RegexMatchFlags::empty()) {
        Ok(Some(re)) => Ok(re),
        Ok(None) => Err(String::from("invalid pattern")),
        Err(e) => Err(e.message().to_string()),
    }
}

pub fn find_all(lines: &[String], re: &glib::Regex) -> Vec<Match> {
    let mut out = Vec::new();
    for (y, line) in lines.iter().enumerate() {
        if line.is_empty() {
            continue;
        }
        let cols = char_columns(line);
        let gs = glib::GString::from(line.as_str());
        let mut pos = 0usize;
        while pos <= line.len() {
            let Ok(info) = re.match_full(gs.as_gstr(), pos as i32, glib::RegexMatchFlags::empty()) else { break };
            if !info.matches() {
                break;
            }
            let Some((s, e)) = info.fetch_pos(0) else { break };
            let (s, e) = (s as usize, e as usize);
            if e > s {
                let (cs, ce) = (line[..s].chars().count(), line[..e].chars().count());
                out.push(Match { y: y as u64, x0: cols[cs], x1: cols[ce] });
            }
            pos = if e > pos { e } else { pos + 1 };
            while pos < line.len() && !line.is_char_boundary(pos) {
                pos += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plain_case_regex_and_word() {
        let l = lines(&["Error: foo", "", "an error and ERROR", "terror"]);
        let m = find_all(&l, &compile("error", false).unwrap());
        assert_eq!(m.len(), 4);
        assert_eq!(m[0], Match { y: 0, x0: 0, x1: 5 });
        assert_eq!(m[2], Match { y: 2, x0: 13, x1: 18 });
        assert_eq!(find_all(&l, &compile("error", true).unwrap()).len(), 2);
        assert_eq!(find_all(&l, &compile(r"\berror\b", false).unwrap()).len(), 3);
        assert_eq!(find_all(&l, &compile(r"o+", true).unwrap()).len(), 4);
        assert!(compile("(", false).is_err());
    }

    #[test]
    fn columns_with_wide_characters_and_empty_matches() {
        let l = lines(&["漢字 ab", "x"]);
        let m = find_all(&l, &compile("ab", true).unwrap());
        assert_eq!(m, [Match { y: 0, x0: 5, x1: 7 }]);
        assert!(find_all(&l, &compile("z*", true).unwrap()).is_empty(), "empty matches are ignored");
    }
}
