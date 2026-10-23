// SPDX-License-Identifier: GPL-3.0-or-later
//! Lesezeichen für Ordner: `~/.config/parkett/bookmarks.toml`, von Hand bearbeitbar.
//!
//! ```toml
//! [[bookmark]]
//! name = "Projekte"
//! path = "~/05_Projekte"
//! ```

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    /// `~` erlaubt (siehe `paths::expand_home`)
    pub path: String,
}

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default, rename = "bookmark")]
    bookmarks: Vec<Bookmark>,
}

/// Liest die Datei; fehlt sie, gibt es keine Lesezeichen.
pub fn parse(text: &str) -> Result<Vec<Bookmark>, toml::de::Error> {
    Ok(toml::from_str::<File>(text)?.bookmarks)
}

pub fn to_toml(list: &[Bookmark]) -> String {
    toml::to_string_pretty(&File { bookmarks: list.to_vec() }).expect("Lesezeichen lassen sich immer serialisieren")
}

/// Fügt `path` hinzu (Name = letzter Ordner, `~` für den Persönlichen Ordner); ein Pfad steht
/// nur einmal in der Liste. `false`, wenn er schon da war.
pub fn add(list: &mut Vec<Bookmark>, path: &str, home: &Path) -> bool {
    let shown = crate::paths::tilde(path, home);
    let same = |b: &Bookmark| crate::paths::expand_home(&b.path, home) == crate::paths::expand_home(&shown, home);
    if list.iter().any(same) {
        return false;
    }
    let name = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| shown.clone());
    list.push(Bookmark { name, path: shown });
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
        assert_eq!(list[0], Bookmark { name: "05_Projekte".into(), path: "~/05_Projekte".into() });
        assert_eq!(list[2].path, "~");
        let back = parse(&to_toml(&list)).unwrap();
        assert_eq!(back, list);
    }

    #[test]
    fn hand_written_file() {
        let list = parse("[[bookmark]]\nname = \"Web\"\npath = \"/srv/www\"\n").unwrap();
        assert_eq!(list, vec![Bookmark { name: "Web".into(), path: "/srv/www".into() }]);
        assert!(parse("").unwrap().is_empty());
        assert!(parse("[[bookmark]]\nname = 1\n").is_err());
    }
}
