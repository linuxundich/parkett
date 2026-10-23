// SPDX-License-Identifier: GPL-3.0-or-later
//! Lesezeichen für Ordner (Datei `~/.config/parkett/bookmarks.toml`, siehe `core::bookmark`).
//! Bei jedem Zugriff frisch gelesen: Die Datei ist klein und darf von Hand bearbeitet werden.

use gtk::glib;
use parkett_core::bookmark::{self, Bookmark};

fn file() -> std::path::PathBuf {
    glib::user_config_dir().join("parkett").join("bookmarks.toml")
}

pub fn load() -> Vec<Bookmark> {
    let Ok(text) = std::fs::read_to_string(file()) else { return Vec::new() };
    bookmark::parse(&text).unwrap_or_else(|e| {
        glib::g_warning!("parkett", "{}: {e}", file().display());
        Vec::new()
    })
}

/// Ordner als Lesezeichen anhängen; `Ok(false)`, wenn er schon eines ist.
pub fn add(path: &str) -> Result<bool, String> {
    let mut list = load();
    if !bookmark::add(&mut list, path, &glib::home_dir()) {
        return Ok(false);
    }
    let f = file();
    if let Some(dir) = f.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&f, bookmark::to_toml(&list)).map_err(|e| format!("{}: {e}", f.display()))?;
    Ok(true)
}

/// Absoluter Pfad eines Lesezeichens (`~` aufgelöst).
pub fn resolve(b: &Bookmark) -> String {
    parkett_core::paths::expand_home(&b.path, &glib::home_dir()).to_string_lossy().into_owned()
}
