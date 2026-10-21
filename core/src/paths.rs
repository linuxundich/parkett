// SPDX-License-Identifier: GPL-3.0-or-later
//! Pfade in Profilen und Vorlagen: `~` auflösen und anzeigen, Unterverzeichnisse anhängen;
//! Dateien atomar und nur für den Benutzer lesbar schreiben.

use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

/// Schreibt atomar (temporäre Datei `tmp` + Umbenennen) mit Modus 0600 – unabhängig von der
/// umask, denn Profile können in Umgebungsvariablen Zugangsdaten tragen (R-12).
pub fn write_private(tmp: &Path, target: &Path, contents: &str) -> io::Result<()> {
    // Reste eines abgebrochenen Laufs hätten noch den alten Modus; `mode` gilt nur beim Anlegen
    let _ = std::fs::remove_file(tmp);
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(tmp)?;
    f.write_all(contents.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(tmp, target)
}

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

    #[test]
    fn private_file_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("parkett-test-private-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (tmp, target) = (dir.join(".a.tmp"), dir.join("a"));
        std::fs::write(&target, "alt").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o664)).unwrap();
        std::fs::write(&tmp, "Rest").unwrap();
        write_private(&tmp, &target, "neu").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "neu");
        assert_eq!(std::fs::metadata(&target).unwrap().permissions().mode() & 0o777, 0o600);
        assert!(!tmp.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
