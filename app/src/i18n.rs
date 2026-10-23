// SPDX-License-Identifier: GPL-3.0-or-later
//! Übersetzungen einrichten; die Helfer (`gettext`, `trf`, `ntrf`, `N_`) kommen aus
//! `parkett_core::i18n`.

pub use parkett_core::i18n::*;

pub const DOMAIN: &str = "parkett";

/// Vor GTK und vor dem ersten übersetzten Text: Sprache aus der Umgebung, Übersetzungen aus
/// dem installierten Ort (Meson setzt PARKETT_LOCALEDIR) oder für `cargo run` aus OUT_DIR.
pub fn init() {
    let dir = option_env!("PARKETT_LOCALEDIR").unwrap_or(concat!(env!("OUT_DIR"), "/locale"));
    let _ = gettextrs::bindtextdomain(DOMAIN, dir);
    // SAFETY: am Anfang von main(), noch ohne weitere Threads
    let _ = unsafe { gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "") };
    let _ = gettextrs::bind_textdomain_codeset(DOMAIN, "UTF-8");
    let _ = gettextrs::textdomain(DOMAIN);
}
