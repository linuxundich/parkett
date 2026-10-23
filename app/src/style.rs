// SPDX-License-Identifier: GPL-3.0-or-later
//! Hell/Dunkel von Hand (Wunsch Christoph): GSettings `style` = system | light | dark,
//! angewendet über AdwStyleManager. Terminals folgen, weil sie auf `dark` des
//! StyleManagers hören.

use adw::prelude::*;

/// Stil aus den Einstellungen anwenden und Änderungen folgen. Einmal beim Start aufrufen.
pub fn init() {
    apply();
    crate::settings::settings().connect_changed(Some("style"), |_, _| apply());
}

fn apply() {
    let scheme = match crate::settings::settings().string("style").as_str() {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}

/// Drei runde Knöpfe System/Hell/Dunkel für das Hauptmenü (Muster: GNOME Text Editor).
pub fn selector() -> gtk::Widget {
    let settings = crate::settings::settings();
    let current = settings.string("style").to_string();
    let row =
        gtk::Box::builder().css_classes(["parkett-style-selector"]).halign(gtk::Align::Center).spacing(18).build();
    let mut group: Option<gtk::CheckButton> = None;
    for (value, label) in [
        ("system", crate::i18n::gettext("Follow system style")),
        ("light", crate::i18n::gettext("Light")),
        ("dark", crate::i18n::gettext("Dark")),
    ] {
        let b = gtk::CheckButton::builder()
            .css_classes([value])
            .tooltip_text(label.as_str())
            .active(current == value)
            .build();
        b.update_property(&[gtk::accessible::Property::Label(&label)]);
        if let Some(g) = &group {
            b.set_group(Some(g));
        } else {
            group = Some(b.clone());
        }
        b.connect_toggled(move |b| {
            if b.is_active() {
                let _ = crate::settings::settings().set_string("style", value);
            }
        });
        row.append(&b);
    }
    // Änderung von außen (Einstellungen, gsettings) im Menü nachziehen
    let weak = row.downgrade();
    let id = settings.connect_changed(Some("style"), move |s, _| {
        let Some(row) = weak.upgrade() else { return };
        let v = s.string("style");
        let mut child = row.first_child();
        while let Some(c) = child {
            if let Some(b) = c.downcast_ref::<gtk::CheckButton>() {
                if b.has_css_class(v.as_str()) && !b.is_active() {
                    b.set_active(true);
                }
            }
            child = c.next_sibling();
        }
    });
    let id = std::cell::RefCell::new(Some(id));
    row.connect_destroy(move |_| {
        if let Some(id) = id.borrow_mut().take() {
            crate::settings::settings().disconnect(id);
        }
    });
    row.upcast()
}
