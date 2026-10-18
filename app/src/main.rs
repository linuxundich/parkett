// SPDX-License-Identifier: GPL-3.0-or-later
//! Parkett – Terminalemulator für GNOME mit Splits.

mod commands;
mod pane;
mod prefs;
mod profiles;
mod settings;
mod shortcuts;
mod style;
mod tab;
mod terminal;
mod window;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

pub const APP_ID: &str = "de.linuxundich.Parkett";
const VERSION: &str = env!("CARGO_PKG_VERSION");

const CSS: &str = "
vte-terminal { padding: 6px 8px; }
.parkett-exit-bar {
  padding: 6px 10px;
  background: color-mix(in srgb, var(--destructive-bg-color) 14%, var(--view-bg-color));
  box-shadow: inset 0 1px var(--border-color);
}
.parkett-focus-line { background: var(--accent-bg-color); }
.parkett-pill {
  padding: 2px 2px 2px 9px;
  border-radius: 8px;
  font-size: 0.86em;
  font-weight: 600;
  background: color-mix(in srgb, var(--headerbar-bg-color) 90%, transparent);
  box-shadow: 0 0 0 1px var(--border-color), 0 1px 3px rgb(0 0 6 / 10%);
}
.parkett-pill button { min-width: 22px; min-height: 22px; padding: 0; }
parkett-pane:not(.focused) .parkett-pill { opacity: 0.75; }
paned > separator { background: var(--border-color); }
.parkett-tile-preview { border-radius: 8px; box-shadow: 0 0 0 1px var(--border-color); }
.parkett-schemes > flowboxchild { border-radius: 12px; padding: 6px; }
.parkett-schemes > flowboxchild:selected { background: color-mix(in srgb, var(--accent-bg-color) 18%, transparent); box-shadow: inset 0 0 0 2px var(--accent-bg-color); }
.parkett-search {
  padding: 3px;
  border-radius: 10px;
  background: var(--popover-bg-color);
  box-shadow: 0 0 0 1px var(--border-color), 0 2px 8px rgb(0 0 6 / 18%);
}
.parkett-search button { min-width: 28px; min-height: 28px; padding: 0 4px; }
.parkett-badge {
  min-width: 64px; min-height: 64px;
  border-radius: 16px;
  font-size: 30px; font-weight: 800;
  background: color-mix(in srgb, var(--headerbar-bg-color) 92%, transparent);
  box-shadow: 0 0 0 1px var(--border-color), 0 2px 8px rgb(0 0 6 / 18%);
}
parkett-pane.focused .parkett-badge { background: var(--accent-bg-color); color: var(--accent-fg-color); }
.parkett-mode-bar > flowboxchild { padding: 0; }
.parkett-mode-bar {
  padding: 8px 14px;
  border-radius: 12px;
  background: var(--popover-bg-color);
  box-shadow: 0 0 0 1px var(--border-color), 0 2px 8px rgb(0 0 6 / 18%);
}
.parkett-mode-tag { font-weight: 800; padding: 2px 8px; border-radius: 6px; background: var(--accent-bg-color); color: var(--accent-fg-color); }
.parkett-key { font-size: 0.85em; font-weight: 600; padding: 1px 5px; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--border-color); }
.parkett-ptitle {
  padding: 1px 2px 1px 10px;
  font-size: 0.86em;
  background: var(--window-bg-color);
  box-shadow: inset 0 -1px var(--border-color);
}
.parkett-ptitle button { min-width: 22px; min-height: 22px; padding: 0; }
.parkett-ptitle.focused { background: color-mix(in srgb, var(--accent-bg-color) 14%, var(--window-bg-color)); font-weight: 600; }
/* Stilwahl im Hauptmenü (Muster: GNOME Text Editor) */
.parkett-style-selector { margin: 9px 0 3px; }
.parkett-style-selector checkbutton {
  padding: 1px; min-width: 44px; min-height: 44px;
  border-radius: 9999px; background-clip: content-box;
  box-shadow: inset 0 0 0 1px var(--border-color);
}
.parkett-style-selector checkbutton.system { background-image: linear-gradient(to bottom right, #ffffff 49.99%, #222226 50.01%); }
.parkett-style-selector checkbutton.light { background-color: #ffffff; }
.parkett-style-selector checkbutton.dark { background-color: #222226; }
.parkett-style-selector checkbutton:checked { box-shadow: inset 0 0 0 2px var(--accent-bg-color); }
.parkett-style-selector checkbutton radio {
  -gtk-icon-source: none; border: none; background: none; box-shadow: none;
  min-width: 12px; min-height: 12px; padding: 2px; transform: translate(27px, 14px);
}
.parkett-style-selector checkbutton radio:checked {
  -gtk-icon-source: -gtk-icontheme('object-select-symbolic');
  background-color: var(--accent-bg-color); color: var(--accent-fg-color); border-radius: 9999px;
}
.parkett-zoom-banner {
  padding: 4px 4px 4px 12px;
  border-radius: 10px;
  background: var(--popover-bg-color);
  box-shadow: 0 0 0 1px var(--border-color), 0 2px 8px rgb(0 0 6 / 18%);
}
";

fn main() -> glib::ExitCode {
    settings::init();
    let app =
        adw::Application::builder().application_id(APP_ID).flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE).build();

    app.connect_startup(|app| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(CSS);
        let display = gdk::Display::default().expect("Display");
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        // Beim Start aus dem Quellbaum die Icons in data/ finden (installiert: hicolor)
        if option_env!("PARKETT_INSTALLED").is_none() {
            gtk::IconTheme::for_display(&display)
                .add_search_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/icons"));
        }
        gtk::Window::set_default_icon_name(APP_ID);
        setup_actions(app);
        style::init();
    });

    // Arbeitsverzeichnis des Aufrufers übernehmen (z. B. „parkett“ aus einem Ordner heraus)
    app.connect_command_line(|app, cmdline| {
        let cwd = cmdline.cwd().and_then(|p| p.to_str().map(String::from));
        window::Window::new(app, cwd).present();
        glib::ExitCode::SUCCESS
    });

    app.run()
}

fn setup_actions(app: &adw::Application) {
    let new_window = gio::SimpleAction::new("new-window", None);
    new_window.connect_activate({
        let app = app.downgrade();
        move |_, _| {
            if let Some(app) = app.upgrade() {
                window::Window::new(&app, None).present();
            }
        }
    });
    app.add_action(&new_window);

    let about = gio::SimpleAction::new("about", None);
    about.connect_activate({
        let app = app.downgrade();
        move |_, _| {
            let Some(app) = app.upgrade() else { return };
            let dialog = adw::AboutDialog::builder()
                .application_name("Parkett")
                .application_icon(APP_ID)
                .developer_name("Christoph Langner")
                .version(VERSION)
                .comments("Terminal mit geteilten Bereichen für GNOME")
                .license_type(gtk::License::Gpl30)
                .build();
            dialog.present(app.active_window().as_ref());
        }
    });
    app.add_action(&about);

    let preferences = gio::SimpleAction::new("preferences", None);
    preferences.connect_activate({
        let app = app.downgrade();
        move |_, _| {
            let Some(app) = app.upgrade() else { return };
            prefs::show(app.active_window().as_ref());
        }
    });
    app.add_action(&preferences);

    let shortcuts = gio::SimpleAction::new("shortcuts", None);
    shortcuts.connect_activate({
        let app = app.downgrade();
        move |_, _| {
            let Some(app) = app.upgrade() else { return };
            shortcuts_dialog().present(app.active_window().as_ref());
        }
    });
    app.add_action(&shortcuts);

    // Nur App-Aktionen als globale Accels; Fenster- und Terminalkürzel hängen an ihren
    // Widgets (Capture-Phase), sonst würden sie doppelt oder gar nicht ausgelöst.
    shortcuts::apply_app_accels(app.upcast_ref());
    let weak = app.downgrade();
    settings::settings().connect_changed(Some("custom-shortcuts"), move |_, _| {
        if let Some(app) = weak.upgrade() {
            shortcuts::apply_app_accels(app.upcast_ref());
        }
    });
}

/// Übersicht aller Kürzel (AdwShortcutsDialog, libadwaita 1.8) mit der aktuellen Belegung.
fn shortcuts_dialog() -> adw::ShortcutsDialog {
    let dialog = adw::ShortcutsDialog::new();
    for section in shortcuts::SECTIONS {
        let sec = adw::ShortcutsSection::new(Some(section));
        for e in shortcuts::ENTRIES.iter().filter(|e| e.section == *section) {
            let accels = shortcuts::effective(e.action);
            if !accels.is_empty() {
                sec.add(adw::ShortcutsItem::new(e.label, &accels.join(" ")));
            }
        }
        match *section {
            "Reiter und Fenster" => sec.add(adw::ShortcutsItem::new("Reiter 1 … 9", "<Control><Alt>1")),
            "Bereiche" => sec.add(adw::ShortcutsItem::new("Bereich 1 … 9", "<Alt>1")),
            _ => {}
        }
        dialog.add(sec);
    }
    dialog
}
