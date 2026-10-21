// SPDX-License-Identifier: GPL-3.0-or-later
//! Zugriff auf die GSettings der Anwendung (Schema: data/de.linuxundich.Parkett.gschema.xml).

use gtk::gio;
use gtk::prelude::*;

thread_local! {
    static SETTINGS: gio::Settings = gio::Settings::new(crate::APP_ID);
}

pub fn settings() -> gio::Settings {
    SETTINGS.with(|s| s.clone())
}

/// Bei `cargo run` liegt das kompilierte Schema im Build-Ordner; installiert (Meson setzt
/// PARKETT_INSTALLED beim Bauen) im Standardpfad. Muss vor dem ersten Zugriff laufen.
pub fn init() {
    if option_env!("PARKETT_INSTALLED").is_none() {
        // SAFETY: wird am Anfang von main() aufgerufen, bevor Threads existieren
        unsafe { std::env::set_var("GSETTINGS_SCHEMA_DIR", concat!(env!("OUT_DIR"), "/schemas")) };
    }
}

pub fn confirm_close() -> bool {
    settings().boolean("confirm-close")
}

pub fn dim_unfocused() -> bool {
    settings().boolean("dim-unfocused")
}

/// Neue Terminals im Verzeichnis des aktiven Bereichs starten (sonst im Persönlichen Ordner)?
pub fn inherit_directory() -> bool {
    settings().string("new-terminal-directory") == "current"
}

/// Reiter-Label „1 htop · parkett“ statt des Titels der Shell (QA-17).
pub fn short_tab_labels() -> bool {
    settings().boolean("short-tab-labels")
}

pub fn context_colors() -> bool {
    settings().boolean("context-colors")
}

pub fn scrollback_lines() -> i64 {
    settings().int("scrollback-lines").into()
}
