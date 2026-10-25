// SPDX-License-Identifier: GPL-3.0-or-later
//! Bookmarks for folders, commands and SSH targets (file `~/.config/parkett/bookmarks.toml`, see `core::bookmark`).
//! Read fresh on every access: the file is small and may be edited by hand.

use gtk::glib;
use gtk::prelude::*;
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

/// Append a folder as a bookmark; `Ok(false)` if it already is one.
pub fn add(path: &str) -> Result<bool, String> {
    let mut list = load();
    if !bookmark::add(&mut list, path, &glib::home_dir()) {
        return Ok(false);
    }
    save(&list)?;
    Ok(true)
}

/// Write the whole list (preferences: rename, sort, delete).
pub fn save(list: &[Bookmark]) -> Result<(), String> {
    let f = file();
    if let Some(dir) = f.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&f, bookmark::to_toml(list)).map_err(|e| format!("{}: {e}", f.display()))
}

/// Absolute path of a bookmark (`~` resolved).
pub fn resolve(b: &Bookmark) -> String {
    parkett_core::paths::expand_home(&b.path, &glib::home_dir()).to_string_lossy().into_owned()
}

/// Action and target to open the bookmark; `place` is "tab", "pane" or "here"
/// (new tab, new pane, in the active pane).
pub fn action(b: &Bookmark, place: &str) -> (&'static str, glib::Variant) {
    let dir = if b.path.trim().is_empty() { String::new() } else { resolve(b) };
    match b.shell_line() {
        Some(line) => ("win.bookmark-command", (place.to_string(), dir, line).to_variant()),
        None => {
            let name = match place {
                "pane" => "win.bookmark-pane",
                "here" => "win.bookmark-cd",
                _ => "win.open-bookmark",
            };
            (name, dir.to_variant())
        }
    }
}

/// Icon per kind (palette, preferences).
pub fn icon(b: &Bookmark) -> &'static str {
    match b.kind() {
        bookmark::Kind::Folder => "folder-symbolic",
        bookmark::Kind::Command => "utilities-terminal-symbolic",
        bookmark::Kind::Remote => "network-server-symbolic",
    }
}

/// Second line: folder, command or `ssh …`.
pub fn detail(b: &Bookmark) -> String {
    b.shell_line().unwrap_or_else(|| b.path.clone())
}
