// SPDX-License-Identifier: GPL-3.0-or-later
//! Prüfung vor dem Einfügen: Würde der Text in einer Shell ohne „Bracketed Paste“ sofort
//! ausgeführt? Das ist bei jedem Zeilenumbruch so, auch bei einem einzigen am Ende.

/// Ergebnis der Prüfung: die Zeilen (ohne leere Zeile hinter dem letzten Umbruch) und ob
/// der Text einen Umbruch enthält.
pub struct Check<'a> {
    pub lines: Vec<&'a str>,
    pub executes: bool,
}

pub fn check(text: &str) -> Check<'_> {
    // \r führt ebenso aus wie \n (Enter); \r\n zählt als ein Umbruch
    let executes = text.contains(['\n', '\r']);
    let body = text.strip_suffix("\r\n").or_else(|| text.strip_suffix(['\n', '\r'])).unwrap_or(text);
    let lines = body.split("\r\n").flat_map(|l| l.split(['\n', '\r'])).collect();
    Check { lines, executes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_without_newline_is_harmless() {
        let c = check("ls -la");
        assert!(!c.executes);
        assert_eq!(c.lines, ["ls -la"]);
    }

    #[test]
    fn trailing_newline_executes() {
        // QA-04: eine Zeile mit Umbruch wurde ohne Rückfrage ausgeführt
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
