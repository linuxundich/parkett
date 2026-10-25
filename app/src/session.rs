// SPDX-License-Identifier: GPL-3.0-or-later
//! Save the session and restore it at startup (setting `restore-session`).
//!
//! Saved continuously, shortly after every change to tabs, layout or
//! directories – so the session also survives logging out or a crash. When the last
//! window closes, its state is written immediately; after that (no windows left) nothing more.
//! Closing the last tab ends the session: the next start begins fresh.

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

/// Watch the setting: turned off → delete the saved session.
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
    // No window left (app is quitting): keep the last written state
    if list.is_empty() {
        return;
    }
    if let Err(e) = (Session { version: VERSION, windows: list }).save(&path()) {
        glib::g_warning!("parkett", "session not saved: {e}");
    }
}

/// Save after a change, batched (1.5 s after the first change not yet saved).
/// A running timer is not restarted: a program that keeps setting its title
/// would otherwise postpone saving forever (R-07).
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

/// A window closes. If there are others, write their state. If it is the last one: with
/// tabs (window button, logout) write its state; without tabs (the last tab
/// was closed) it was a deliberate end – then the next start begins fresh.
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

/// Open the last session. `false` if there is none or it is turned off.
pub fn restore(app: &adw::Application) -> bool {
    if !enabled() {
        return false;
    }
    let Some(session) = Session::load(&path()) else { return false };
    // app.windows() returns the most recently focused window first. Open it first too:
    // Mutter gives focus to the first window; later present() calls without an activation token
    // do not bring their window to the front (seen in the VM)
    for w in &session.windows {
        let win = Window::empty(app);
        win.restore_session(w);
        win.present();
    }
    true
}
