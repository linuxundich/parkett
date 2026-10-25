// SPDX-License-Identifier: GPL-3.0-or-later
//! Check before pasting: would the text run immediately in a shell without "bracketed paste"?
//! That is the case for every line break, even a single one at the end.

/// Result of the check: the lines (without the empty line after the last break) and whether
/// the text contains a line break.
pub struct Check<'a> {
    pub lines: Vec<&'a str>,
    pub executes: bool,
}

pub fn check(text: &str) -> Check<'_> {
    // \r executes just like \n (Enter); \r\n counts as one break
    let executes = text.contains(['\n', '\r']);
    let body = text.strip_suffix("\r\n").or_else(|| text.strip_suffix(['\n', '\r'])).unwrap_or(text);
    let lines = body.split("\r\n").flat_map(|l| l.split(['\n', '\r'])).collect();
    Check { lines, executes }
}

/// Advanced paste (Tilix "Advanced Paste"): replace tabs with `tab_width` spaces
/// (e.g. for Python in an editor, where a tab would trigger completion) and turn
/// Windows line endings (`\r\n`, also lone `\r`) into `\n`.
pub fn transform(text: &str, tab_width: Option<usize>, unix_newlines: bool) -> String {
    let mut out = if unix_newlines { text.replace("\r\n", "\n").replace('\r', "\n") } else { text.to_string() };
    if let Some(n) = tab_width {
        out = out.replace('\t', &" ".repeat(n));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_tabs_and_newlines() {
        assert_eq!(transform("a\tb\r\nc\rd", Some(4), true), "a    b\nc\nd");
        assert_eq!(transform("a\tb\r\n", None, false), "a\tb\r\n");
        assert_eq!(transform("\t", Some(2), false), "  ");
    }

    #[test]
    fn single_line_without_newline_is_harmless() {
        let c = check("ls -la");
        assert!(!c.executes);
        assert_eq!(c.lines, ["ls -la"]);
    }

    #[test]
    fn trailing_newline_executes() {
        // QA-04: a single line with a line break was executed without asking
        for text in ["curl x | sh\n", "curl x | sh\r", "curl x | sh\r\n"] {
            let c = check(text);
            assert!(c.executes, "{text:?}");
            assert_eq!(c.lines, ["curl x | sh"], "{text:?}");
        }
    }

    #[test]
    fn multiple_lines() {
        let c = check("echo eins\r\necho zwei\recho drei\n\n");
        assert!(c.executes);
        assert_eq!(c.lines, ["echo eins", "echo zwei", "echo drei", ""]);
    }
}
