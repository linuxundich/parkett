// SPDX-License-Identifier: GPL-3.0-or-later
//! Parkett – Terminalemulator für GNOME mit Splits.

mod bookmarks;
mod commands;
mod pane;
mod prefs;
mod profiles;
mod session;
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
.parkett-exit-bar.neutral { background: color-mix(in srgb, var(--window-fg-color) 5%, var(--view-bg-color)); }
.parkett-focus-line { background: var(--accent-bg-color); }
.parkett-sync-line { background: var(--warning-bg-color); }
.parkett-drop-hint {
  background: color-mix(in srgb, var(--accent-bg-color) 28%, transparent);
  border: 2px solid var(--accent-bg-color);
  border-radius: 6px;
  margin: 4px;
}
.parkett-pill {
  padding: 2px 2px 2px 9px;
  border-radius: 8px;
  font-size: 0.86em;
  font-weight: 600;
  background: color-mix(in srgb, var(--headerbar-bg-color) 90%, transparent);
  box-shadow: 0 0 0 1px var(--border-color), 0 1px 3px rgb(0 0 6 / 10%);
}
.parkett-pill button { min-width: 22px; min-height: 22px; padding: 0; }
.parkett-pill { opacity: 0; transition: opacity 150ms ease-out; }
.parkett-pill.revealed { opacity: 1; }
parkett-pane:not(.focused) .parkett-pill.revealed { opacity: 0.85; }
paned > separator { background: var(--border-color); }
window.parkett-translucent { background-color: transparent; }
window.parkett-translucent toolbarview > .top-bar { background-color: var(--headerbar-bg-color); }
window.parkett-translucent paned > separator { background: color-mix(in srgb, var(--window-fg-color) 18%, var(--window-bg-color)); }
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
    // Gesetzt von GIO beim Start aus dem Menü; gilt für Parkett, nicht für die Shells darin
    // (würde sonst an jedes daraus gestartete Programm vererbt, wie gnome-terminal es vermeidet).
    // Hier ganz am Anfang, solange nur ein Thread läuft (R-06)
    for var in ["GIO_LAUNCHED_DESKTOP_FILE", "GIO_LAUNCHED_DESKTOP_FILE_PID"] {
        std::env::remove_var(var);
    }
    settings::init();
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE | gio::ApplicationFlags::SEND_ENVIRONMENT)
        .build();

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
        session::init();
    });

    // Befehlszeile; kommt bei einer zweiten Instanz von deren Aufruf (Argumente, Verzeichnis)
    app.connect_command_line(|app, cmdline| {
        let args: Vec<String> = cmdline.arguments().iter().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
        let mut cli = match parkett_core::cli::parse(&args) {
            Ok(cli) => cli,
            Err(e) => {
                cmdline.printerr_literal(&format!("parkett: {e}\n"));
                return glib::ExitCode::FAILURE;
            }
        };
        if cli.help {
            cmdline.print_literal(parkett_core::cli::HELP);
            return glib::ExitCode::SUCCESS;
        }
        if cli.version {
            cmdline.print_literal(&format!("Parkett {}\n", env!("CARGO_PKG_VERSION")));
            return glib::ExitCode::SUCCESS;
        }
        if let Some(line) = cli.command.as_ref().filter(|_| cli.command_needs_split).map(|c| c[0].clone()) {
            match glib::shell_parse_argv(&line) {
                Ok(v) => cli.command = Some(v.into_iter().map(|s| s.to_string_lossy().into_owned()).collect()),
                Err(e) => {
                    cmdline.printerr_literal(&format!("parkett: -e „{line}“: {}\n", e.message()));
                    return glib::ExitCode::FAILURE;
                }
            }
        }
        open(app, cmdline, cli)
    });

    let args: Vec<String> = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    app.run_with_args(&parkett_core::cli::protect_separator(args))
}

/// Fenster oder Reiter nach der Befehlszeile; ohne Vorgaben beim ersten Fenster die letzte
/// Sitzung.
fn open(app: &adw::Application, cmdline: &gio::ApplicationCommandLine, cli: parkett_core::cli::Cli) -> glib::ExitCode {
    let first = app.windows().is_empty();
    if first && cli.is_plain() && !cli.tab && session::restore(app) {
        return glib::ExitCode::SUCCESS;
    }
    // Verzeichnisse: Vorgaben (relativ zum Aufrufer, ~ aufgelöst), sonst das des Aufrufers
    let caller = cmdline.cwd();
    let mut dirs: Vec<Option<String>> = Vec::new();
    for d in &cli.working_directories {
        let p = parkett_core::paths::expand_home(d, &glib::home_dir());
        let p = match &caller {
            Some(base) if p.is_relative() => base.join(p),
            _ => p,
        };
        if !p.is_dir() {
            cmdline.printerr_literal(&format!("parkett: Verzeichnis „{}“ gibt es nicht\n", p.display()));
            return glib::ExitCode::FAILURE;
        }
        dirs.push(p.to_str().map(String::from));
    }
    if dirs.is_empty() {
        dirs.push(caller.and_then(|p| p.to_str().map(String::from)));
    }
    let manager = profiles::ProfileManager::get();
    let profile = match &cli.profile {
        Some(id) if manager.ensure_profile(id) => id.clone(),
        Some(id) => {
            cmdline.printerr_literal(&format!("parkett: Profil „{id}“ gibt es nicht, nehme das Standardprofil\n"));
            manager.default_id()
        }
        None => manager.default_id(),
    };
    // Aufruf an eine laufende Instanz: Umgebung des Aufrufers mitnehmen (QA-08). Beim ersten
    // Start ist sie die von Parkett selbst.
    // GApplication schickt sie nur mit SEND_ENVIRONMENT; leer hieße die Shell ohne PATH
    // zu starten, dann lieber die Umgebung von Parkett.
    let environment = cmdline
        .is_remote()
        .then(|| cmdline.environ().into_iter().map(|e| e.to_string_lossy().into_owned()).collect::<Vec<_>>())
        .filter(|env| env.iter().any(|e| e.starts_with("PATH=")));
    let launch = terminal::Launch { command: cli.command.clone(), environment };
    let target = if cli.tab { app.active_window().and_downcast::<window::Window>() } else { None };
    let win = target.unwrap_or_else(|| window::Window::empty(app));
    // ein Reiter je Verzeichnis (Nautilus: mehrere Ordner markiert), der erste bleibt gewählt
    let mut first_tab = None;
    for cwd in dirs {
        let tab = win.add_tab_with(cwd, &profile, launch.clone(), cli.title.clone());
        first_tab.get_or_insert(tab);
    }
    if let Some(tab) = first_tab {
        win.select(&tab);
    }
    win.present();
    glib::ExitCode::SUCCESS
}

fn setup_actions(app: &adw::Application) {
    // Klick auf „Befehl beendet“: Bereich zeigen
    let show = gio::SimpleAction::new("show-terminal", Some(glib::VariantTy::UINT64));
    show.connect_activate({
        let app = app.downgrade();
        move |_, param| {
            let (Some(app), Some(uid)) = (app.upgrade(), param.and_then(|p| p.get::<u64>())) else { return };
            for w in app.windows().into_iter().filter_map(|w| w.downcast::<window::Window>().ok()) {
                if w.show_terminal(uid) {
                    break;
                }
            }
        }
    });
    app.add_action(&show);

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
