// SPDX-License-Identifier: GPL-3.0-or-later
//! Translations (gettext, text domain "parkett", set by the app). Source strings are
//! English, German strings live in po/de.po.
//!
//! - `gettext("…")` for fixed strings
//! - `trf("… {name} …", &[("name", value)])` with named placeholders, so a
//!   translation can change the order
//! - `ntrf("{n} pane", "{n} panes", n, &[])` for singular/plural
//! - `N_("…")` marks strings in constants; they are translated with `gettext` when shown
//!
//! Write long strings on one line (`\n` instead of line continuation): po/extract_rust.py
//! only recognizes calls with a single literal.

pub use gettextrs::gettext;

/// Marks a string for translation without translating it (for constants).
#[allow(non_snake_case)]
pub const fn N_(s: &'static str) -> &'static str {
    s
}

/// Translates `msgid` and fills in the values for `{name}`.
pub fn trf(msgid: &str, args: &[(&str, &str)]) -> String {
    fill(gettext(msgid), args)
}

/// Like `trf` with singular/plural by `n`; `{n}` is replaced with `n`.
pub fn ntrf(singular: &str, plural: &str, n: u32, args: &[(&str, &str)]) -> String {
    fill(gettextrs::ngettext(singular, plural, n), args).replace("{n}", &n.to_string())
}

fn fill(mut text: String, args: &[(&str, &str)]) -> String {
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders() {
        // without a text domain gettext returns the source string
        assert_eq!(trf("Bookmark “{name}” added", &[("name", "~/src")]), "Bookmark “~/src” added");
        assert_eq!(trf("{a} {b} {a}", &[("a", "1"), ("b", "2")]), "1 2 1");
        assert_eq!(ntrf("{n} pane", "{n} panes", 3, &[]), "3 panes");
        assert_eq!(ntrf("{n} pane", "{n} panes", 1, &[]), "1 pane");
    }
}
