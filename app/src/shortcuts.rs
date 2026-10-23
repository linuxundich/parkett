// SPDX-License-Identifier: GPL-3.0-or-later
//! Alle änderbaren Tastenkürzel an einer Stelle: Standardbelegung, eigene Belegung aus
//! GSettings (`custom-shortcuts`, Aktion → Kürzel, leer = aus) und Aufbau der
//! Shortcut-Controller für Fenster (`win.*`) und Terminal (`term.*`).

use gtk::prelude::*;
use std::collections::HashMap;

use gtk::gdk;
use gtk::glib::translate::IntoGlib;

use crate::i18n::{gettext, N_};

pub struct Entry {
    /// Voller Aktionsname, z. B. „win.split-right“
    pub action: &'static str,
    pub section: &'static str,
    pub label: &'static str,
    /// Standardkürzel (GtkAccelerator-Syntax); mehrere = gleichwertige Alternativen
    pub defaults: &'static [&'static str],
}

const fn e(
    action: &'static str,
    section: &'static str,
    label: &'static str,
    defaults: &'static [&'static str],
) -> Entry {
    Entry { action, section, label, defaults }
}

pub const SECTIONS: &[&str] = &[N_("Tabs and windows"), N_("Panes"), N_("Terminal")];

/// Belegung nach Tilix, wo sie nicht mit GNOME kollidiert (CONCEPTS.md).
pub const ENTRIES: &[Entry] = &[
    e("win.new-tab", N_("Tabs and windows"), N_("New tab"), &["<Control><Shift>t"]),
    e("win.close-tab", N_("Tabs and windows"), N_("Close tab"), &["<Control><Shift>q"]),
    e("win.next-tab", N_("Tabs and windows"), N_("Next tab"), &["<Control>Page_Down"]),
    e("win.prev-tab", N_("Tabs and windows"), N_("Previous tab"), &["<Control>Page_Up"]),
    // ohne Standard: F2 brauchen htop, mc und nano selbst (QA-01)
    e("win.rename-tab", N_("Tabs and windows"), N_("Rename tab"), &[]),
    e("win.overview", N_("Tabs and windows"), N_("Tab overview"), &["<Control><Shift>o"]),
    e("app.new-window", N_("Tabs and windows"), N_("New window"), &["<Control><Shift>n"]),
    e("win.palette", N_("Tabs and windows"), N_("Command palette"), &["<Control><Shift>p"]),
    e("win.save-template", N_("Tabs and windows"), N_("Save tab as template"), &[]),
    e("win.add-bookmark", N_("Tabs and windows"), N_("Bookmark current folder"), &[]),
    e("app.preferences", N_("Tabs and windows"), N_("Preferences"), &["<Control>comma"]),
    e("app.shortcuts", N_("Tabs and windows"), N_("Keyboard shortcuts overview"), &["<Control>question"]),
    e("win.layout-mode", N_("Panes"), N_("Layout mode"), &["<Control><Shift>l"]),
    e("win.split-right", N_("Panes"), N_("Split right"), &["<Control><Alt>r"]),
    e("win.split-down", N_("Panes"), N_("Split down"), &["<Control><Alt>d"]),
    e("win.split-auto", N_("Panes"), N_("Split automatically"), &["<Control><Alt>a"]),
    e("win.close-pane", N_("Panes"), N_("Close pane"), &["<Control><Shift>w"]),
    e("win.toggle-sync", N_("Panes"), N_("Input to all panes"), &["<Control><Alt>i"]),
    e("win.sync-pane", N_("Panes"), N_("Pane receives shared input"), &[]),
    e("win.toggle-zoom", N_("Panes"), N_("Maximize pane"), &["<Control><Shift>x"]),
    e("win.equalize", N_("Panes"), N_("Equalize sizes"), &["<Control><Alt>equal"]),
    e("win.focus-left", N_("Panes"), N_("Focus left"), &["<Alt>Left"]),
    e("win.focus-right", N_("Panes"), N_("Focus right"), &["<Alt>Right"]),
    e("win.focus-up", N_("Panes"), N_("Focus up"), &["<Alt>Up"]),
    e("win.focus-down", N_("Panes"), N_("Focus down"), &["<Alt>Down"]),
    e("win.resize-left", N_("Panes"), N_("Move divider left"), &["<Shift><Alt>Left"]),
    e("win.resize-right", N_("Panes"), N_("Move divider right"), &["<Shift><Alt>Right"]),
    e("win.resize-up", N_("Panes"), N_("Move divider up"), &["<Shift><Alt>Up"]),
    e("win.resize-down", N_("Panes"), N_("Move divider down"), &["<Shift><Alt>Down"]),
    e("win.find", N_("Terminal"), N_("Search"), &["<Control><Shift>f"]),
    e("win.find-up", N_("Terminal"), N_("Previous match"), &["<Control><Shift>g"]),
    e("win.find-down", N_("Terminal"), N_("Next match"), &["<Control><Shift>h"]),
    e("term.copy", N_("Terminal"), N_("Copy"), &["<Control><Shift>c"]),
    e("term.paste", N_("Terminal"), N_("Paste"), &["<Control><Shift>v", "<Shift>Insert"]),
    e("term.select-all", N_("Terminal"), N_("Select all"), &["<Control><Shift>a"]),
    e("term.save-output", N_("Terminal"), N_("Save output…"), &[]),
    e("term.reset", N_("Terminal"), N_("Reset"), &[]),
    e("term.reset-clear", N_("Terminal"), N_("Reset and clear"), &[]),
    e(
        "term.zoom-in",
        N_("Terminal"),
        N_("Increase font size"),
        &["<Control>plus", "<Control>equal", "<Control>KP_Add"],
    ),
    e("term.zoom-out", N_("Terminal"), N_("Decrease font size"), &["<Control>minus", "<Control>KP_Subtract"]),
    e("term.zoom-reset", N_("Terminal"), N_("Normal font size"), &["<Control>0", "<Control>KP_0"]),
];

pub fn entry(action: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.action == action)
}

/// Eigene Belegungen aus GSettings (`a{ss}`; `glib::VariantDict` taugt nicht, das ist `a{sv}`).
fn custom() -> HashMap<String, String> {
    crate::settings::settings().value("custom-shortcuts").get::<HashMap<String, String>>().unwrap_or_default()
}

/// Wirksame Kürzel einer Aktion: eigene Belegung (eine, leer = aus) oder die Standards.
pub fn effective(action: &str) -> Vec<String> {
    match custom().get(action) {
        Some(s) if s.is_empty() => Vec::new(),
        Some(s) => vec![s.clone()],
        None => entry(action).map(|e| e.defaults.iter().map(|s| s.to_string()).collect()).unwrap_or_default(),
    }
}

pub fn is_custom(action: &str) -> bool {
    custom().contains_key(action)
}

/// Eigene Belegung setzen (`Some("")` = aus) oder auf Standard zurück (`None`).
pub fn set(action: &str, accel: Option<&str>) {
    let mut map = custom();
    match accel {
        Some(a) => {
            map.insert(action.to_string(), a.to_string());
        }
        None => {
            map.remove(action);
        }
    }
    let _ = crate::settings::settings().set_value("custom-shortcuts", &map.to_variant());
}

pub fn reset_all() {
    crate::settings::settings().reset("custom-shortcuts");
}

/// Welche andere Aktion hat dieses Kürzel schon? (Vergleich über geparste Tasten)
pub fn conflict(action: &str, accel: &str) -> Option<&'static Entry> {
    let wanted = gtk::accelerator_parse(accel)?;
    ENTRIES
        .iter()
        .filter(|e| e.action != action)
        .find(|e| effective(e.action).iter().any(|a| gtk::accelerator_parse(a).is_some_and(|k| k == wanted)))
}

/// Fest belegt und nicht änderbar: Alt+1…9 (Bereich), Strg+Alt+1…9 (Reiter). Liefert einen
/// mit `N_` markierten Text; beim Anzeigen mit `gettext` übersetzen.
pub fn reserved(accel: &str) -> Option<&'static str> {
    let (key, mods) = gtk::accelerator_parse(accel)?;
    let digit = key.to_unicode().and_then(|c| c.to_digit(10)).is_some_and(|d| (1..=9).contains(&d));
    let alt = gtk::gdk::ModifierType::ALT_MASK;
    let ctrl_alt = alt | gtk::gdk::ModifierType::CONTROL_MASK;
    match mods {
        m if digit && m == alt => Some(N_("Select pane 1–9")),
        m if digit && m == ctrl_alt => Some(N_("Select tab 1–9")),
        _ => None,
    }
}

/// Lesbare Form, z. B. „Strg+Umschalt+F“; leer = „Aus“.
pub fn label(accels: &[String]) -> String {
    if accels.is_empty() {
        return gettext("Off");
    }
    accels
        .iter()
        .filter_map(|a| gtk::accelerator_parse(a).map(|(k, m)| gtk::accelerator_get_label(k, m).to_string()))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// Tooltip „Bezeichnung (Kürzel)“ mit der aktuellen Belegung; `fixed`: Tasten, die nicht
/// änderbar sind (z. B. „Eingabe“ in der Suche), vor den Kürzeln der Aktion. `label` und
/// `fixed` sind englische Quelltexte (`N_`); übersetzt wird hier beim Zeigen.
pub fn tooltip(label: &str, action: &str, fixed: &[&str]) -> String {
    let accels = effective(action);
    let label = gettext(label);
    let mut keys: Vec<String> = fixed.iter().map(|s| gettext(*s)).collect();
    if !accels.is_empty() {
        keys.push(self::label(&accels));
    }
    if keys.is_empty() {
        label
    } else {
        format!("{label} ({})", keys.join(", "))
    }
}

/// Tooltip, der beim Zeigen aus der aktuellen Belegung entsteht – folgt so geänderten
/// Kürzeln auch in schon offenen Bereichen (QA-12). Der Name für Screenreader ist nur die
/// Bezeichnung, ohne Kürzel (QA-18).
pub fn live_tooltip(widget: &impl IsA<gtk::Widget>, label: &str, action: &'static str, fixed: &'static [&'static str]) {
    widget.upcast_ref::<gtk::Widget>().update_property(&[gtk::accessible::Property::Label(&gettext(label))]);
    tooltip_only(widget, label, action, fixed);
}

/// Wie `live_tooltip`, aber ohne den Namen für Screenreader zu setzen (Widgets, die ihren
/// eigenen Namen behalten sollen, z. B. der Fenstertitel).
pub fn tooltip_only(widget: &impl IsA<gtk::Widget>, label: &str, action: &'static str, fixed: &'static [&'static str]) {
    let label = label.to_string();
    widget.set_has_tooltip(true);
    widget.connect_query_tooltip(move |_, _, _, _, tip| {
        tip.set_text(Some(&tooltip(&label, action, fixed)));
        true
    });
}

/// Shortcut-Controller (Capture-Phase, vor VTE) für alle Aktionen mit Präfix `prefix`.
pub fn controller(prefix: &str) -> gtk::ShortcutController {
    let c = gtk::ShortcutController::new();
    c.set_propagation_phase(gtk::PropagationPhase::Capture);
    for e in ENTRIES.iter().filter(|e| e.action.starts_with(prefix)) {
        let accels = effective(e.action);
        if accels.is_empty() {
            continue;
        }
        let trigger = gtk::ShortcutTrigger::parse_string(&accels.join("|"));
        c.add_shortcut(gtk::Shortcut::new(trigger, Some(gtk::NamedAction::new(e.action))));
    }
    c
}

/// App-Aktionen als globale Accels setzen (erscheinen so auch in Menüs).
pub fn apply_app_accels(app: &gtk::Application) {
    for e in ENTRIES.iter().filter(|e| e.action.starts_with("app.")) {
        let accels = effective(e.action);
        let refs: Vec<&str> = accels.iter().map(String::as_str).collect();
        app.set_accels_for_action(e.action, &refs);
    }
}

/// Kürzel aus einem Tastendruck; `None` für reine Modifikatortasten.
/// Ohne Strg/Alt/Super sind nur F-Tasten erlaubt, sonst ginge Tippen im Terminal verloren.
/// Der Fehlertext ist mit `N_` markiert; beim Anzeigen mit `gettext` übersetzen.
pub fn from_key(key: gdk::Key, state: gdk::ModifierType) -> Result<Option<String>, &'static str> {
    use gdk::Key;
    if matches!(
        key,
        Key::Shift_L
            | Key::Shift_R
            | Key::Control_L
            | Key::Control_R
            | Key::Alt_L
            | Key::Alt_R
            | Key::Super_L
            | Key::Super_R
            | Key::Meta_L
            | Key::Meta_R
            | Key::ISO_Level3_Shift
            | Key::Caps_Lock
    ) {
        return Ok(None);
    }
    let mods = state & gtk::accelerator_get_default_mod_mask();
    let strong =
        mods.intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK | gdk::ModifierType::SUPER_MASK);
    let fkey = (Key::F1.into_glib()..=Key::F12.into_glib()).contains(&key.to_lower().into_glib());
    if !strong && !fkey {
        return Err(N_("Please combine with Ctrl, Alt or Super (only F keys work alone)"));
    }
    Ok(Some(gtk::accelerator_name(key.to_lower(), mods).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// „<Control><Shift>t“ → ({"control", "shift"}, "t"), ohne GTK (`accelerator_parse` will
    /// ein initialisiertes GTK und damit ein Display).
    fn split(accel: &str) -> (Vec<String>, String) {
        let mut mods = Vec::new();
        let mut rest = accel;
        while let Some(r) = rest.strip_prefix('<') {
            let (m, r) = r.split_once('>').expect("„>“ fehlt");
            mods.push(m.to_ascii_lowercase());
            rest = r;
        }
        mods.sort();
        (mods, rest.to_ascii_lowercase())
    }

    /// Programme im Terminal brauchen F-Tasten und alles ohne Strg/Alt selbst (QA-01: F2 in
    /// htop, mc, nano). Einzige Ausnahme: Umschalt+Einfg zum Einfügen, wie in jedem Terminal.
    #[test]
    fn defaults_leave_keys_to_programs() {
        for e in ENTRIES {
            for &a in e.defaults {
                let (mods, key) = split(a);
                let strong = mods.iter().any(|m| ["control", "alt", "super"].contains(&m.as_str()));
                assert!(strong || a == "<Shift>Insert", "{}: „{a}“ gehört den Programmen im Terminal", e.action);
                assert!(!key.is_empty(), "{}: „{a}“ ohne Taste", e.action);
                // Alt+1…9 und Strg+Alt+1…9 sind fest belegt (`reserved`)
                let digit = key.len() == 1 && key.chars().all(|c| ('1'..='9').contains(&c));
                assert!(!(digit && mods.contains(&"alt".to_string())), "{}: „{a}“ ist fest belegt", e.action);
            }
        }
    }

    #[test]
    fn defaults_are_unique() {
        let mut seen: Vec<((Vec<String>, String), &str)> = Vec::new();
        for e in ENTRIES {
            assert_eq!(ENTRIES.iter().filter(|o| o.action == e.action).count(), 1, "{} doppelt", e.action);
            assert!(SECTIONS.contains(&e.section), "{}: Abschnitt „{}“", e.action, e.section);
            for &a in e.defaults {
                let k = split(a);
                if let Some((_, other)) = seen.iter().find(|(k2, _)| *k2 == k) {
                    panic!("„{a}“ für {} und {other}", e.action);
                }
                seen.push((k, e.action));
            }
        }
    }
}
