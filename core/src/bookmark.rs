// SPDX-License-Identifier: GPL-3.0-or-later
//! Bookmarks for folders, commands and SSH targets (like Tilix): `~/.config/parkett/bookmarks.toml`,
//! editable by hand.
//!
//! ```toml
//! [[bookmark]]
//! name = "Projekte"
//! path = "~/05_Projekte"
//!
//! [[bookmark]]
//! name = "Logs"
//! command = "journalctl -f"
//!
//! [[bookmark]]
//! name = "bender"
//! host = "toff@bender"
//! ```

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    /// Folder; for a command or SSH the start directory (empty = like a new tab).
    /// `~` allowed (see `paths::expand_home`)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Command typed into the shell – the shell stays usable afterwards
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub command: String,
    /// SSH target with options, everything after `ssh` (e.g. `-p 2222 toff@bender`)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub host: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Folder,
    Command,
    Remote,
}

impl Bookmark {
    pub fn kind(&self) -> Kind {
        if !self.host.trim().is_empty() {
            Kind::Remote
        } else if !self.command.trim().is_empty() {
            Kind::Command
        } else {
            Kind::Folder
        }
    }

    /// What gets typed into the shell; `None` for a folder.
    pub fn shell_line(&self) -> Option<String> {
        match self.kind() {
            Kind::Folder => None,
            Kind::Command => Some(self.command.trim().to_string()),
            Kind::Remote => Some(format!("ssh {}", self.host.trim())),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default, rename = "bookmark")]
    bookmarks: Vec<Bookmark>,
}

/// Reads the file; if it is missing, there are no bookmarks.
pub fn parse(text: &str) -> Result<Vec<Bookmark>, toml::de::Error> {
    Ok(toml::from_str::<File>(text)?.bookmarks)
}

pub fn to_toml(list: &[Bookmark]) -> String {
    toml::to_string_pretty(&File { bookmarks: list.to_vec() }).expect("Lesezeichen lassen sich immer serialisieren")
}

/// Adds `path` (name = last folder, `~` for the home folder); a path appears
/// only once in the list. `false` if it was already there.
pub fn add(list: &mut Vec<Bookmark>, path: &str, home: &Path) -> bool {
    let shown = crate::paths::tilde(path, home);
    let same = |b: &Bookmark| crate::paths::expand_home(&b.path, home) == crate::paths::expand_home(&shown, home);
    if list.iter().filter(|b| b.kind() == Kind::Folder).any(same) {
        return false;
    }
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| shown.clone());
    list.push(Bookmark { name, path: shown, ..Default::default() });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_add() {
        let home = Path::new("/home/t");
        let mut list = Vec::new();
        assert!(add(&mut list, "/home/t/05_Projekte", home));
        assert!(add(&mut list, "/srv/www", home));
        assert!(!add(&mut list, "/home/t/05_Projekte", home), "doppelt");
        assert!(add(&mut list, "/home/t", home));
        assert_eq!(
            list[0],
            Bookmark { name: "05_Projekte".into(), path: "~/05_Projekte".into(), ..Default::default() }
        );
        assert_eq!(list[2].path, "~");
        let back = parse(&to_toml(&list)).unwrap();
        assert_eq!(back, list);
    }

    #[test]
    fn hand_written_file() {
        let list = parse("[[bookmark]]\nname = \"Web\"\npath = \"/srv/www\"\n").unwrap();
        assert_eq!(list, vec![Bookmark { name: "Web".into(), path: "/srv/www".into(), ..Default::default() }]);
        assert!(parse("").unwrap().is_empty());
        assert!(parse("[[bookmark]]\nname = 1\n").is_err());
    }

    #[test]
    fn commands_and_hosts() {
        let list = parse("[[bookmark]]\nname = \"Logs\"\ncommand = \"journalctl -f\"\n\n[[bookmark]]\nname = \"b\"\nhost = \"-p 2222 toff@bender\"\n").unwrap();
        assert_eq!(list[0].kind(), Kind::Command);
        assert_eq!(list[0].shell_line().as_deref(), Some("journalctl -f"));
        assert_eq!(list[1].kind(), Kind::Remote);
        assert_eq!(list[1].shell_line().as_deref(), Some("ssh -p 2222 toff@bender"));
        // empty fields do not end up in the file
        let text = to_toml(&list);
        assert!(!text.contains("path"), "{text}");
        assert_eq!(parse(&text).unwrap(), list);
        assert_eq!(Bookmark { name: "x".into(), path: "/".into(), ..Default::default() }.kind(), Kind::Folder);
    }
}
