// SPDX-License-Identifier: GPL-3.0-or-later
//! Pfade in Profilen und Vorlagen: `~` auflösen und anzeigen, Unterverzeichnisse anhängen.

use std::path::{Component, Path, PathBuf};

/// `~` und `~/…` gegen `home` auflösen; `~name` bleibt (fremder Benutzer, nicht unterstützt).
pub fn expand_home(dir: &str, home: &Path) -> PathBuf {
    match dir.strip_prefix('~') {
        Some("") => home.to_path_buf(),
        Some(rest) if rest.starts_with('/') => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(dir),
    }
}

/// Umkehrung für die Anzeige: `$HOME/x` → `~/x`.
pub fn tilde(path: &str, home: &Path) -> String {
    match home.to_str() {
        Some(h) if path == h => "~".into(),
        Some(h) if path.starts_with(&format!("{h}/")) => format!("~{}", &path[h.len()..]),
        _ => path.into(),
    }
}

/// Unterverzeichnis einer Vorlage an `base` hängen. Nur relative Pfade ohne „..“ – sonst
/// könnte eine Vorlage ihre Bereiche an beliebigen Orten starten; dann bleibt es bei `base`.
pub fn join_subdir(base: &Path, sub: &str) -> PathBuf {
    let sub = sub.trim();
    let p = Path::new(sub);
    if sub.is_empty() || !p.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)) {
        return base.to_path_buf();
    }
    base.join(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand() {
        let home = Path::new("/home/a");
        assert_eq!(expand_home("~", home), PathBuf::from("/home/a"));
        assert_eq!(expand_home("~/src", home), PathBuf::from("/home/a/src"));
        assert_eq!(expand_home("~foo", home), PathBuf::from("~foo"));
        assert_eq!(expand_home("/tmp", home), PathBuf::from("/tmp"));
    }

    #[test]
    fn tilde_roundtrip() {
        let home = Path::new("/home/a");
        assert_eq!(tilde("/home/a", home), "~");
        assert_eq!(tilde("/home/a/src", home), "~/src");
        assert_eq!(tilde("/home/ab", home), "/home/ab");
    }

    #[test]
    fn subdir_stays_inside() {
        let base = Path::new("/p");
        assert_eq!(join_subdir(base, ""), PathBuf::from("/p"));
        assert_eq!(join_subdir(base, "web/src"), PathBuf::from("/p/web/src"));
        assert_eq!(join_subdir(base, "/etc"), PathBuf::from("/p"));
        assert_eq!(join_subdir(base, "../x"), PathBuf::from("/p"));
        assert_eq!(join_subdir(base, "a/../../x"), PathBuf::from("/p"));
    }
}
