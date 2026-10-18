//! Parkett – Terminalemulator für GNOME mit Splits.
//! Stand: Phase-3-Prototyp (ein Fenster, ein Terminal).

mod colors;
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
";

fn main() -> glib::ExitCode {
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
        gtk::IconTheme::for_display(&display).add_search_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/icons"));
        gtk::Window::set_default_icon_name(APP_ID);
        setup_actions(app);
    });

    // Arbeitsverzeichnis des Aufrufers übernehmen (z. B. „parkett“ aus einem Ordner heraus)
    app.connect_command_line(|app, cmdline| {
        let cwd = cmdline.cwd().and_then(|p| p.to_str().map(String::from));
        window::build(app, cwd).present();
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
                window::build(&app, None).present();
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

    app.set_accels_for_action("app.new-window", &["<Ctrl><Shift>n"]);
}
