// SPDX-License-Identifier: GPL-3.0-or-later
//! Eigene Links je Profil (wie Tilix „Custom Hyperlinks“): Ein regulärer Ausdruck macht Text in
//! der Ausgabe klickbar, das Ziel ist eine Adresse oder ein Befehl mit `$0` (ganzer Treffer) bis
//! `$9` (Gruppen). Die Ausgabe kann von jedem stammen (`cat datei`, SSH): Gruppen werden darum
//! in Adressen kodiert und in Befehlen gequotet, nie roh eingesetzt.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomLink {
    /// PCRE2 wie bei der Suche, z. B. `\bPARK-(\d+)\b`
    pub pattern: String,
    /// `https://tracker.example/issue/$1` oder `code --goto $1:$2`
    pub target: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// mit der Standardanwendung öffnen
    Open(String),
    /// Befehlszeile; Gruppen sind schon gequotet, die App zerlegt sie wie eine Shell
    Run(String),
}

impl CustomLink {
    /// Leere Einträge (halb ausgefüllt im Editor) gelten nicht.
    pub fn is_usable(&self) -> bool {
        !self.pattern.trim().is_empty() && !self.target.trim().is_empty()
    }
}

/// Hat `target` ein Schema wie `https:` oder `mailto:`? Dann ist es eine Adresse.
fn is_uri(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else { return false };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
}

/// Prozentkodierung für einen Teil einer Adresse; `/` bleibt (Pfade aus der Ausgabe).
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

/// In einfache Anführungszeichen wie in der Shell; `'` als `'\''`.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Setzt die Gruppen ein (`groups[0]` = ganzer Treffer; fehlende Gruppen werden leer).
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
        // Ausgabe mit Shell-Zeichen wird kein zweiter Befehl
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
