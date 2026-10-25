// SPDX-License-Identifier: GPL-3.0-or-later
//! Access to the application's GSettings (schema: data/de.linuxundich.Parkett.gschema.xml).

use gtk::gio;
use gtk::prelude::*;

thread_local! {
    static SETTINGS: gio::Settings = gio::Settings::new(crate::APP_ID);
}

pub fn settings() -> gio::Settings {
    SETTINGS.with(|s| s.clone())
}

/// With `cargo run` the compiled schema lives in the build directory; when installed (Meson sets
/// PARKETT_INSTALLED at build time) in the default path. Must run before the first access.
pub fn init() {
    if option_env!("PARKETT_INSTALLED").is_none() {
        // SAFETY: called at the start of main(), before any threads exist
        unsafe { std::env::set_var("GSETTINGS_SCHEMA_DIR", concat!(env!("OUT_DIR"), "/schemas")) };
    }
}

pub fn confirm_close() -> bool {
    settings().boolean("confirm-close")
}

pub fn dim_unfocused() -> bool {
    settings().boolean("dim-unfocused")
}

/// Start new terminals in the directory of the active pane (otherwise in the home folder)?
pub fn inherit_directory() -> bool {
    settings().string("new-terminal-directory") == "current"
}

/// Tab label "1 htop · parkett" instead of the shell's title (QA-17).
pub fn short_tab_labels() -> bool {
    settings().boolean("short-tab-labels")
}

/// With GNOME's high contrast, the *High Contrast* scheme instead of the profile's scheme.
pub fn follow_high_contrast() -> bool {
    settings().boolean("follow-high-contrast")
}

/// Template for short tab names (`core::label::render`); empty = default.
pub fn tab_label_template() -> String {
    let t = settings().string("tab-label-template").trim().to_string();
    if t.is_empty() {
        parkett_core::label::DEFAULT_TEMPLATE.to_string()
    } else {
        t
    }
}

/// Template for the window title; empty = "{title}".
pub fn window_title_template() -> String {
    let t = settings().string("window-title-template").trim().to_string();
    if t.is_empty() {
        "{title}".to_string()
    } else {
        t
    }
}

pub fn context_colors() -> bool {
    settings().boolean("context-colors")
}

pub fn scrollback_lines() -> i64 {
    settings().int("scrollback-lines").into()
}
