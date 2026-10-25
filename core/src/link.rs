// SPDX-License-Identifier: GPL-3.0-or-later
//! Custom links per profile (like Tilix "Custom Hyperlinks"): a regular expression makes text in
//! the output clickable, the target is an address or a command with `$0` (whole match) to
//! `$9` (groups). The output can come from anyone (`cat file`, SSH), so groups are
//! encoded in addresses and quoted in commands, never inserted raw.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomLink {
    /// PCRE2 as in search, e.g. `\bPARK-(\d+)\b`
    pub pattern: String,
    /// `https://tracker.example/issue/$1` or `code --goto $1:$2`
    pub target: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// open with the default application
    Open(String),
    /// command line; groups are already quoted, the app splits it like a shell
    Run(String),
}

impl CustomLink {
    /// Empty entries (half filled in the editor) do not count.
    pub fn is_usable(&self) -> bool {
        !self.pattern.trim().is_empty() && !self.target.trim().is_empty()
    }
}

/// Does `target` have a scheme like `https:` or `mailto:`? Then it is an address.
fn is_uri(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else { return false };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
}

/// Percent-encoding for part of an address; `/` stays (paths from the output).
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// In single quotes as in the shell; `'` as `'\''`.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Fills in the groups (`groups[0]` = whole match; missing groups become empty).
pub fn expand(target: &str, groups: &[Option<&str>]) -> Action {
    let uri = is_uri(target.trim_start());
    let mut out = String::new();
    let mut chars = target.trim().chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            if let Some(d) = chars.peek().and_then(|d| d.to_digit(10)) {
                chars.next();
                let g = groups.get(d as usize).copied().flatten().unwrap_or("");
                out.push_str(&if uri { encode(g) } else { shell_quote(g) });
                continue;
            }
        }
        out.push(c);
    }
    if uri {
        Action::Open(out)
    } else {
        Action::Run(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_target_encodes_groups() {
        let a = expand("https://tracker.example/issue/$1", &[Some("PARK-12"), Some("12")]);
        assert_eq!(a, Action::Open("https://tracker.example/issue/12".into()));
        let a = expand("https://x.example/?q=$0", &[Some("a b&c")]);
        assert_eq!(a, Action::Open("https://x.example/?q=a%20b%26c".into()));
    }

    #[test]
    fn command_target_quotes_groups() {
        let a = expand("code --goto $1:$2", &[Some("x"), Some("src/main.rs"), Some("12")]);
        assert_eq!(a, Action::Run("code --goto 'src/main.rs':'12'".into()));
        // output with shell characters does not become a second command
        let a = expand("echo $0", &[Some("x'; rm -rf ~; '")]);
        assert_eq!(a, Action::Run(r"echo 'x'\''; rm -rf ~; '\'''".into()));
    }

    #[test]
    fn missing_groups_and_dollar() {
        assert_eq!(expand("open $3 $x", &[Some("a")]), Action::Run("open '' $x".into()));
        assert!(!CustomLink { pattern: " ".into(), target: "x".into() }.is_usable());
        assert!(!is_uri("code --goto a:b"));
        assert!(is_uri("mailto:a@b"));
    }
}
