//! Hauptfenster des Prototyps: Headerbar + ein Terminal.
//! Tabs (AdwTabView) und Splits (aus parkett_core::layout) folgen in Phase 4.

use adw::prelude::*;
use gtk::{gio, glib};
use vte::prelude::*;

use crate::terminal::Terminal;

pub fn build(app: &adw::Application, cwd: Option<String>) -> adw::ApplicationWindow {
    let title = adw::WindowTitle::new("Parkett", "");

    let new_window = gtk::Button::builder()
        .icon_name("window-new-symbolic")
        .tooltip_text("Neues Fenster (Strg+Umschalt+N)")
        .action_name("app.new-window")
        .build();

    let menu = gio::Menu::new();
    menu.append(Some("Neues Fenster"), Some("app.new-window"));
    let about = gio::Menu::new();
    about.append(Some("Über Parkett"), Some("app.about"));
    menu.append_section(None, &about);
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .primary(true)
        .tooltip_text("Hauptmenü")
        .build();

    let header = adw::HeaderBar::builder().title_widget(&title).build();
    header.pack_start(&new_window);
    header.pack_end(&menu_button);

    let terminal = Terminal::new(cwd);

    let view = adw::ToolbarView::new();
    view.add_top_bar(&header);
    view.set_content(Some(terminal.widget()));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Parkett")
        .icon_name(crate::APP_ID)
        .content(&view)
        .build();

    // Fenster- und Untertitel folgen dem Terminal (Titel per OSC, Verzeichnis per OSC 7)
    let update = {
        let terminal = terminal.clone();
        let title = title.clone();
        let window = window.downgrade();
        move || {
            let t = terminal.title().unwrap_or_else(|| "Parkett".into());
            let cwd = terminal.cwd().map(|p| tilde(&p)).unwrap_or_default();
            title.set_title(&t);
            title.set_subtitle(&cwd);
            if let Some(w) = window.upgrade() {
                w.set_title(Some(&t));
            }
        }
    };
    terminal.connect_state_changed(update.clone());
    update();

    let weak = window.downgrade();
    terminal.connect_close(move || {
        if let Some(w) = weak.upgrade() {
            w.close();
        }
    });

    // Mindestgröße des Fensters aus der Zellgröße (MIN_COLUMNS×MIN_ROWS)
    let vte = terminal.vte().clone();
    let set_min = move |v: &vte::Terminal| {
        let (cw, ch) = (v.char_width() as i32, v.char_height() as i32);
        v.set_size_request(cw * crate::terminal::MIN_COLUMNS as i32, ch * crate::terminal::MIN_ROWS as i32);
    };
    set_min(&vte);
    vte.connect_char_size_changed(move |v, _, _| set_min(v));

    glib::idle_add_local_once({
        let vte = terminal.vte().clone();
        move || {
            vte.grab_focus();
        }
    });
    window
}

fn tilde(path: &str) -> String {
    let home = glib::home_dir();
    match home.to_str() {
        Some(h) if path == h => "~".into(),
        Some(h) if path.starts_with(&format!("{h}/")) => format!("~{}", &path[h.len()..]),
        _ => path.into(),
    }
}
