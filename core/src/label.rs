// SPDX-License-Identifier: GPL-3.0-or-later
//! Kurzes Reiter-Label „1 htop · parkett“ (QA-17): Nummer, laufendes Programm und letzter
//! Ordner statt des Shell-Titels, der in jedem Reiter fast gleich aussieht
//! („christoph@arch: ~/…“).

use std::path::Path;

/// Standardvorlage: ergibt „1 htop · parkett“ bzw. in einer SSH-Sitzung „3 ssh bender“.
pub const DEFAULT_TEMPLATE: &str = "{n} {program} · {dir}";

/// Werte für eine Titelvorlage. `number`: Platz des Reiters (1-basiert; passt zu
/// Strg+Alt+1…9, ab 10 leer), `program`: Vordergrundprogramm oder Shell, `dir`:
/// Arbeitsverzeichnis (`None` z. B. in einer SSH-Sitzung, deren Pfad hier nichts bedeutet).
#[derive(Default)]
pub struct Vars<'a> {
    pub number: Option<usize>,
    pub program: Option<&'a str>,
    pub dir: Option<&'a str>,
    pub host: Option<&'a str>,
    /// Titel, den die Shell setzt (OSC 0/2)
    pub title: Option<&'a str>,
    pub profile: Option<&'a str>,
}

/// Reiter-Name aus `template` mit `{n}`, `{program}`, `{dir}` (letzter Ordner), `{path}`
/// (ganzer Pfad mit `~`), `{host}`, `{title}`, `{profile}`. Leere Werte fallen samt ihrem
/// Trenner (` · `, ` - `, ` | `, `: ` …) weg; unbekannte Variablen bleiben stehen. Ganz
/// leer: „Shell“.
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
    // in Text und Variablen zerlegen
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
    // Zwischen zwei sichtbaren Werten steht der erste Text nach dem vorigen Wert; leere
    // Variablen dazwischen samt ihren Trennern entfallen. Ein Trenner am Ende entfällt,
    // abschließender Text („]“) bleibt.
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

/// Reiter-Name mit der Standardvorlage.
pub fn tab_label(number: Option<usize>, program: Option<&str>, dir: Option<&str>, home: &Path) -> String {
    render(DEFAULT_TEMPLATE, &Vars { number, program, dir, ..Vars::default() }, home)
}

/// `~` für den Persönlichen Ordner, `/` für die Wurzel, sonst der letzte Teil des Pfads.
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
        // Trenner verschwinden mit leeren Werten
        assert_eq!(render("{program} · {host} · {dir}", &v, home), "vim · parkett");
        let remote = Vars { number: Some(3), program: Some("ssh"), host: Some("bender"), ..Vars::default() };
        assert_eq!(render("{n} {program} | {host} | {dir}", &remote, home), "3 ssh | bender");
        assert_eq!(render("{unbekannt} {n}", &remote, home), "{unbekannt} 3");
        assert_eq!(render("", &remote, home), "Shell");
        assert_eq!(render("{dir}", &remote, home), "Shell", "leer ergibt Shell");
    }
}
