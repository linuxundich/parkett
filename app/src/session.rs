// SPDX-License-Identifier: GPL-3.0-or-later
//! Sitzung sichern und beim Start wiederherstellen (Einstellung `restore-session`).
//!
//! Gespeichert wird laufend, kurz nach jeder Änderung an Reitern, Aufteilung oder
//! Verzeichnissen – so übersteht die Sitzung auch Abmelden oder Absturz. Schließt das letzte
//! Fenster, wird sein Stand sofort geschrieben; danach (keine Fenster mehr) nichts mehr.
//! Wer den letzten Reiter schließt, beendet die Sitzung: Der nächste Start beginnt frisch.

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib;
use parkett_core::session::{Session, SessionWindow, VERSION};

use crate::window::Window;

thread_local! {
    static PENDING: RefCell<Option<glib::SourceId>> = const { RefCell::new(None) };
}

fn path() -> PathBuf {
    glib::user_state_dir().join("parkett").join("session.json")
}

fn enabled() -> bool {
    crate::settings::settings().boolean("restore-session")
}

/// Einstellung beobachten: ausgeschaltet → gespeicherte Sitzung löschen.
pub fn init() {
    crate::settings::settings().connect_changed(Some("restore-session"), |s, _| {
        if !s.boolean("restore-session") {
            let _ = std::fs::remove_file(path());
        }
    });
}

fn windows(app: &gtk::Application) -> Vec<Window> {
    app.windows().into_iter().filter_map(|w| w.downcast::<Window>().ok()).collect()
}

fn write(app: &gtk::Application, skip: Option<&Window>) {
    if !enabled() {
        return;
    }
    let list: Vec<SessionWindow> = windows(app)
        .iter()
        .filter(|w| Some(*w) != skip)
        .map(|w| w.to_session())
        .filter(|w| !w.tabs.is_empty())
        .collect();
    // Kein Fenster mehr (App beendet sich): den zuletzt geschriebenen Stand behalten
    if list.is_empty() {
        return;
    }
    if let Err(e) = (Session { version: VERSION, windows: list }).save(&path()) {
        glib::g_warning!("parkett", "session not saved: {e}");
    }
}

/// Nach einer Änderung speichern, gebündelt (1,5 s nach der ersten noch nicht gespeicherten
/// Änderung). Ein laufender Zeitgeber wird nicht neu gestartet: Ein Programm, das laufend
/// seinen Titel setzt, würde das Speichern sonst immer weiter aufschieben (R-07).
pub fn schedule_save() {
    if !enabled() {
        return;
    }
    PENDING.with(|p| {
        if p.borrow().is_some() {
            return;
        }
        let id = glib::timeout_add_local_once(Duration::from_millis(1500), || {
            PENDING.with(|p| p.borrow_mut().take());
            if let Some(app) = gtk::gio::Application::default().and_downcast::<gtk::Application>() {
                write(&app, None);
            }
        });
        *p.borrow_mut() = Some(id);
    });
}

/// Ein Fenster schließt. Gibt es weitere, deren Stand schreiben. Ist es das letzte: mit
/// Reitern (Fensterknopf, Abmelden) seinen Stand schreiben; ohne Reiter (der letzte Reiter
/// wurde geschlossen) war das ein bewusstes Ende – dann beginnt der nächste Start frisch.
pub fn window_closing(window: &Window) {
    let Some(app) = window.application() else { return };
    let others = windows(&app).iter().any(|w| w != window);
    if others {
        write(&app, Some(window));
    } else if window.tabs().is_empty() {
        PENDING.with(|p| {
            if let Some(id) = p.borrow_mut().take() {
                id.remove();
            }
        });
        let _ = std::fs::remove_file(path());
    } else {
        write(&app, None);
    }
}

/// Letzte Sitzung öffnen. `false`, wenn es keine gibt oder sie ausgeschaltet ist.
pub fn restore(app: &adw::Application) -> bool {
    if !enabled() {
        return false;
    }
    let Some(session) = Session::load(&path()) else { return false };
    // app.windows() liefert das zuletzt fokussierte Fenster zuerst. Es auch zuerst öffnen:
    // Mutter gibt dem ersten Fenster den Fokus, spätere present() ohne Aktivierungs-Token
    // holen ihr Fenster nicht nach vorn (in der VM gesehen)
    for w in &session.windows {
        let win = Window::empty(app);
        win.restore_session(w);
        win.present();
    }
    true
}
