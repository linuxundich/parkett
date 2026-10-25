// SPDX-License-Identifier: GPL-3.0-or-later
//! Short tab label "1 htop · parkett" (QA-17): number, running program and last
//! folder instead of the shell title, which looks almost the same in every tab
//! ("christoph@arch: ~/…").

use std::path::Path;

/// Default template: gives "1 htop · parkett", or "3 ssh bender" in an SSH session.
pub const DEFAULT_TEMPLATE: &str = "{n} {program} · {dir}";

/// Values for a title template. `number`: position of the tab (1-based; matches
/// Ctrl+Alt+1…9, empty from 10 on), `program`: foreground program or shell, `dir`:
/// working directory (`None` e.g. in an SSH session, where the path means nothing here).
#[derive(Default)]
pub struct Vars<'a> {
    pub number: Option<usize>,
    pub program: Option<&'a str>,
    pub dir: Option<&'a str>,
    pub host: Option<&'a str>,
    /// title set by the shell (OSC 0/2)
    pub title: Option<&'a str>,
    pub profile: Option<&'a str>,
}

/// Tab name from `template` with `{n}`, `{program}`, `{dir}` (last folder), `{path}`
/// (full path with `~`), `{host}`, `{title}`, `{profile}`. Empty values are dropped together with
/// their separator (` · `, ` - `, ` | `, `: ` …); unknown variables stay as they are. Completely
/// empty: "Shell".
pub fn render(template: &str, v: &Vars, home: &Path) -> String {
    enum Piece {
        Text(String),
        Var(String),
    }
    let value = |name: &str| -> String {
        match name {
            "n" => v.number.filter(|n| (1..=9).contains(n)).map(|n| n.to_string()),
            "program" => v.program.map(String::from),
            "dir" => v.dir.map(|d| folder_name(d, home)),
            "path" => v.dir.map(|d| crate::paths::tilde(d, home)),
            "host" => v.host.map(String::from),
            "title" => v.title.map(String::from),
            "profile" => v.profile.map(String::from),
            other => Some(format!("{{{other}}}")),
        }
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
    };
    // split into text and variables
    let mut pieces = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else { break };
        if start > 0 {
            pieces.push(Piece::Text(rest[..start].to_string()));
        }
        pieces.push(Piece::Var(value(&rest[start + 1..start + len])));
        rest = &rest[start + len + 1..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Text(rest.to_string()));
    }
    let is_separator = |t: &str| {
        let t = t.trim();
        t.is_empty() || t.chars().all(|c| "·-–—|:/,•".contains(c))
    };
    // Between two visible values goes the first text after the previous value; empty
    // variables in between are dropped with their separators. A separator at the end is dropped,
    // closing text ("]") stays.
    let mut out = String::new();
    let mut gap: Option<String> = None;
    let mut seen_value = false;
    let mut var_after_gap = false;
    for p in &pieces {
        match p {
            Piece::Var(val) if val.is_empty() => var_after_gap = true,
            Piece::Var(val) => {
                if let Some(g) = gap.take() {
                    out.push_str(&g);
                }
                out.push_str(val);
                seen_value = true;
                var_after_gap = false;
            }
            Piece::Text(t) if !seen_value => {
                if !is_separator(t) {
                    out.push_str(t);
                }
            }
            Piece::Text(t) => {
                if gap.is_none() {
                    gap = Some(t.clone());
                    var_after_gap = false;
                }
            }
        }
    }
    if let Some(g) = gap {
        if !var_after_gap && !is_separator(&g) {
            out.push_str(&g);
        }
    }
    let out = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.is_empty() {
        crate::i18n::gettext("Shell")
    } else {
        out
    }
}

/// Tab name with the default template.
pub fn tab_label(number: Option<usize>, program: Option<&str>, dir: Option<&str>, home: &Path) -> String {
    render(DEFAULT_TEMPLATE, &Vars { number, program, dir, ..Vars::default() }, home)
}

/// `~` for the home folder, `/` for the root, otherwise the last part of the path.
fn folder_name(dir: &str, home: &Path) -> String {
    let d = dir.trim_end_matches('/');
    if d.is_empty() {
        return "/".into();
    }
    if home.to_str().is_some_and(|h| h.trim_end_matches('/') == d) {
        return "~".into();
    }
    d.rsplit('/').next().unwrap_or(d).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        let home = Path::new("/home/a");
        assert_eq!(tab_label(Some(1), Some("zsh"), Some("/home/a/src/parkett"), home), "1 zsh · parkett");
        assert_eq!(tab_label(Some(2), Some("htop"), Some("/home/a"), home), "2 htop · ~");
        assert_eq!(tab_label(Some(3), Some("bash"), Some("/"), home), "3 bash · /");
        assert_eq!(tab_label(Some(4), Some("ssh bender"), None, home), "4 ssh bender");
        assert_eq!(tab_label(Some(5), None, Some("/etc/"), home), "5 etc");
        assert_eq!(tab_label(None, None, None, home), "Shell");
        assert_eq!(tab_label(Some(10), Some("vim"), Some("/tmp"), home), "vim · tmp", "ohne Kürzel keine Nummer");
    }

    #[test]
    fn templates() {
        let home = Path::new("/home/a");
        let v = Vars {
            number: Some(2),
            program: Some("vim"),
            dir: Some("/home/a/src/parkett"),
            host: None,
            title: Some("a@arch: ~/src/parkett"),
            profile: Some("Standard"),
        };
        assert_eq!(render("{n}: {program} – {path}", &v, home), "2: vim – ~/src/parkett");
        assert_eq!(render("{title}", &v, home), "a@arch: ~/src/parkett");
        assert_eq!(render("{dir} [{profile}]", &v, home), "parkett [Standard]");
        // separators disappear with empty values
        assert_eq!(render("{program} · {host} · {dir}", &v, home), "vim · parkett");
        let remote = Vars { number: Some(3), program: Some("ssh"), host: Some("bender"), ..Vars::default() };
        assert_eq!(render("{n} {program} | {host} | {dir}", &remote, home), "3 ssh | bender");
        assert_eq!(render("{unbekannt} {n}", &remote, home), "{unbekannt} 3");
        assert_eq!(render("", &remote, home), "Shell");
        assert_eq!(render("{dir}", &remote, home), "Shell", "leer ergibt Shell");
    }
}
