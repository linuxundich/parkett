// SPDX-License-Identifier: GPL-3.0-or-later
//! Kurzes Reiter-Label „1 htop · parkett“ (QA-17): Nummer, laufendes Programm und letzter
//! Ordner statt des Shell-Titels, der in jedem Reiter fast gleich aussieht
//! („christoph@arch: ~/…“).

use std::path::Path;

/// `number`: Platz des Reiters (1-basiert; passt zu Strg+Alt+1…9, ab 10 ohne Nummer),
/// `program`: Vordergrundprogramm oder Shell, `dir`: Arbeitsverzeichnis (`None` z. B. in
/// einer SSH-Sitzung, deren Pfad hier nichts bedeutet).
pub fn tab_label(number: Option<usize>, program: Option<&str>, dir: Option<&str>, home: &Path) -> String {
    let folder = dir.map(|d| folder_name(d, home));
    let body = match (program.filter(|p| !p.is_empty()), folder) {
        (Some(p), Some(f)) => format!("{p} · {f}"),
        (Some(p), None) => p.to_string(),
        (None, Some(f)) => f,
        (None, None) => "Shell".to_string(),
    };
    match number.filter(|n| (1..=9).contains(n)) {
        Some(n) => format!("{n} {body}"),
        None => body,
    }
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
}
