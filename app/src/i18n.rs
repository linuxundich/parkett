// SPDX-License-Identifier: GPL-3.0-or-later
//! Set up translations; the helpers (`gettext`, `trf`, `ntrf`, `N_`) come from
//! `parkett_core::i18n`.

pub use parkett_core::i18n::*;

pub const DOMAIN: &str = "parkett";

/// Before GTK and before the first translated text: language from the environment, translations
/// from the installed location (Meson sets PARKETT_LOCALEDIR) or, for `cargo run`, from OUT_DIR.
pub fn init() {
    let dir = option_env!("PARKETT_LOCALEDIR").unwrap_or(concat!(env!("OUT_DIR"), "/locale"));
    let _ = gettextrs::bindtextdomain(DOMAIN, dir);
    // SAFETY: at the start of main(), no other threads yet
    let _ = unsafe { gettextrs::setlocale(gettextrs::LocaleCategory::LcAll, "") };
    let _ = gettextrs::bind_textdomain_codeset(DOMAIN, "UTF-8");
    let _ = gettextrs::textdomain(DOMAIN);
}
