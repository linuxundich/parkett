// SPDX-License-Identifier: GPL-3.0-or-later
//! Übersetzungen (gettext, Textdomäne „parkett“, gesetzt von der App). Quelltexte sind
//! englisch, deutsche Texte stehen in po/de.po.
//!
//! - `gettext("…")` für feste Texte
//! - `trf("… {name} …", &[("name", wert)])` mit benannten Platzhaltern, damit eine
//!   Übersetzung die Reihenfolge ändern kann
//! - `ntrf("{n} pane", "{n} panes", n, &[])` für Einzahl/Mehrzahl
//! - `N_("…")` markiert Texte in Konstanten; übersetzt wird beim Anzeigen mit `gettext`
//!
//! Lange Texte in eine Zeile schreiben (`\n` statt Zeilenfortsetzung): po/extract_rust.py
//! erkennt nur Aufrufe mit einem einzelnen Literal.

pub use gettextrs::gettext;

/// Markiert einen Text für die Übersetzung, ohne ihn zu übersetzen (für Konstanten).
#[allow(non_snake_case)]
pub const fn N_(s: &'static str) -> &'static str {
    s
}

/// Übersetzt `msgid` und setzt die Werte für `{name}` ein.
pub fn trf(msgid: &str, args: &[(&str, &str)]) -> String {
    fill(gettext(msgid), args)
}

/// Wie `trf` mit Einzahl/Mehrzahl nach `n`; `{n}` wird mit `n` ersetzt.
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
        // ohne Textdomäne liefert gettext den Quelltext
        assert_eq!(trf("Bookmark “{name}” added", &[("name", "~/src")]), "Bookmark “~/src” added");
        assert_eq!(trf("{a} {b} {a}", &[("a", "1"), ("b", "2")]), "1 2 1");
        assert_eq!(ntrf("{n} pane", "{n} panes", 3, &[]), "3 panes");
        assert_eq!(ntrf("{n} pane", "{n} panes", 1, &[]), "1 pane");
    }
}
