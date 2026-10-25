// SPDX-License-Identifier: GPL-3.0-or-later
//! Paths in profiles and templates: resolve and display `~`, append subdirectories;
//! write files atomically and readable only by the user.

use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

/// Writes atomically (temporary file `tmp` + rename) with mode 0600 – regardless of the
/// umask, because profiles can carry credentials in environment variables (R-12).
pub fn write_private(tmp: &Path, target: &Path, contents: &str) -> io::Result<()> {
    // leftovers of an aborted run would still have the old mode; `mode` only applies on creation
    let _ = std::fs::remove_file(tmp);
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(tmp)?;
    f.write_all(contents.as_bytes())?;
    f.sync_all()?;
    std::fs::rename(tmp, target)
}

/// Resolve `~` and `~/…` against `home`; `~name` stays (other user, not supported).
pub fn expand_home(dir: &str, home: &Path) -> PathBuf {
    match dir.strip_prefix('~') {
        Some("") => home.to_path_buf(),
        Some(rest) if rest.starts_with('/') => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(dir),
    }
}

/// Inverse for display: `$HOME/x` → `~/x`.
pub fn tilde(path: &str, home: &Path) -> String {
    match home.to_str() {
        Some(h) if path == h => "~".into(),
        Some(h) if path.starts_with(&format!("{h}/")) => format!("~{}", &path[h.len()..]),
        _ => path.into(),
    }
}

/// Path from the output (Ctrl+click): a trailing `:line` or `:line:column` is dropped,
/// `~` becomes the home folder, `./` and `../` are relative to `cwd`. Without a known
/// directory, relative paths have no target.
pub fn output_path(text: &str, cwd: Option<&str>, home: &Path) -> Option<PathBuf> {
    let mut t = text.trim();
    // "src/main.rs:42:7" from compiler messages
    for _ in 0..2 {
        if let Some((head, tail)) = t.rsplit_once(':') {
            if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
                t = head;
            }
        }
    }
    if t.is_empty() {
        return None;
    }
    let path = if t == "~" || t.starts_with("~/") {
        expand_home(t, home)
    } else if t.starts_with('/') {
        PathBuf::from(t)
    } else if t.starts_with("./") || t.starts_with("../") {
        Path::new(cwd?).join(t)
    } else {
        return None;
    };
    // resolve ".." and "." without asking the file system
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    Some(out)
}

/// Append a template's subdirectory to `base`. Only relative paths without ".." – otherwise
/// a template could start its panes anywhere; in that case it stays at `base`.
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

    #[test]
    fn paths_from_output() {
        let home = Path::new("/home/a");
        let p = |t: &str, cwd: Option<&str>| output_path(t, cwd, home).map(|p| p.to_string_lossy().into_owned());
        assert_eq!(p("/etc/fstab", None).as_deref(), Some("/etc/fstab"));
        assert_eq!(p("~/src/parkett", None).as_deref(), Some("/home/a/src/parkett"));
        assert_eq!(p("~", None).as_deref(), Some("/home/a"));
        assert_eq!(
            p("./app/src/main.rs:42:7", Some("/home/a/parkett")).as_deref(),
            Some("/home/a/parkett/app/src/main.rs")
        );
        assert_eq!(p("../core/lib.rs:3", Some("/home/a/parkett/app")).as_deref(), Some("/home/a/parkett/core/lib.rs"));
        assert_eq!(p("./x", None), None, "relativ ohne Verzeichnis");
        assert_eq!(p("src/main.rs", Some("/tmp")), None, "ohne ./ kein Pfad");
        assert_eq!(p("/var/log/../tmp/", None).as_deref(), Some("/var/tmp"));
    }
}
