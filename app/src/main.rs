//! Parkett – Terminalemulator für GNOME mit Splits.

mod pane;
mod prefs;
mod profiles;
mod settings;
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
.parkett-zoom-banner {
  padding: 4px 4px 4px 12px;
  border-radius: 10px;
  background: var(--popover-bg-color);
  box-shadow: 0 0 0 1px var(--border-color), 0 2px 8px rgb(0 0 6 / 18%);
}
";

fn main() -> glib::ExitCode {
    settings::init();
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_startup(|app| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(CSS);
        let display = gdk::Display::default().expect("Display");
        gtk::style_context_add_provider_for_display(&display, &provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        // Beim Start aus dem Quellbaum die Icons in data/ finden (installiert: hicolor)
        if option_env!("PARKETT_INSTALLED").is_none() {
            gtk::IconTheme::for_display(&display).add_search_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/icons"));
        }
        gtk::Window::set_default_icon_name(APP_ID);
        setup_actions(app);
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
    app.set_accels_for_action("app.new-window", &["<Ctrl><Shift>n"]);
    app.set_accels_for_action("app.shortcuts", &["<Ctrl>question"]);
    app.set_accels_for_action("app.preferences", &["<Ctrl>comma"]);
}

/// Übersicht aller Kürzel (AdwShortcutsDialog, libadwaita 1.8).
fn shortcuts_dialog() -> adw::ShortcutsDialog {
    let dialog = adw::ShortcutsDialog::new();
    let sections: &[(&str, &[(&str, &str)])] = &[
        ("Reiter", &[
            ("Neuer Reiter", "<Control><Shift>t"),
            ("Reiter schließen", "<Control><Shift>q"),
            ("Nächster / vorheriger Reiter", "<Control>Page_Down <Control>Page_Up"),
            ("Reiter 1–9", "<Control><Alt>1…9"),
            ("Reiter umbenennen", "F2"),
            ("Reiter-Übersicht", "<Control><Shift>o"),
            ("Neues Fenster", "<Control><Shift>n"),
        ]),
        ("Bereiche", &[
            ("Rechts teilen", "<Control><Alt>r"),
            ("Unten teilen", "<Control><Alt>d"),
            ("Automatisch teilen", "<Control><Alt>a"),
            ("Bereich schließen", "<Control><Shift>w"),
            ("Bereich maximieren", "<Control><Shift>x"),
            ("Größen angleichen", "<Control><Alt>equal"),
            ("Fokus wechseln", "<Alt>Left <Alt>Right <Alt>Up <Alt>Down"),
            ("Bereich 1–9", "<Alt>1…9"),
            ("Größe ändern", "<Shift><Alt>Left <Shift><Alt>Right <Shift><Alt>Up <Shift><Alt>Down"),
        ]),
        ("Terminal", &[
            ("Kopieren", "<Control><Shift>c"),
            ("Einfügen", "<Control><Shift>v"),
            ("Alles auswählen", "<Control><Shift>a"),
            ("Vergrößern / verkleinern", "<Control>plus <Control>minus"),
            ("Normale Größe", "<Control>0"),
        ]),
    ];
    for (title, items) in sections {
        let section = adw::ShortcutsSection::new(Some(title));
        for (label, accel) in items.iter() {
            section.add(adw::ShortcutsItem::new(label, accel));
        }
        dialog.add(section);
    }
    dialog
}
