//! Demo window: `cargo run -p ghostty-view --example demo`.
//! Test hooks: DEMO_INPUT (typed after 800 ms, `\n` = Enter), DEMO_SHOT=file.png (screenshot after
//! DEMO_WAIT ms, default 2500, then quit), DEMO_CMD (program instead of the login shell).
use ghostty_view::GhosttyView;
use gtk::prelude::*;
use gtk::{gdk, glib, graphene};
use parkett_core::pty::SpawnOptions;

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id("de.linuxundich.ParkettGhosttyDemo").build();
    app.connect_activate(|app| {
        let win = gtk::ApplicationWindow::builder().application(app).default_width(900).default_height(520).build();
        let view = GhosttyView::new();
        let scroller = gtk::ScrolledWindow::builder().child(&view).hscrollbar_policy(gtk::PolicyType::Never).build();
        win.set_child(Some(&scroller));
        win.set_title(Some("Parkett – libghostty demo"));
        let w = win.downgrade();
        view.connect_title_changed(move |v| {
            if let (Some(win), Some(t)) = (w.upgrade(), v.title()) {
                win.set_title(Some(&t));
            }
        });
        let a = app.downgrade();
        view.connect_exited(move |_, code| {
            eprintln!("exited: {code:?}");
            if let Some(a) = a.upgrade() {
                a.quit();
            }
        });
        win.present();
        let argv = match std::env::var("DEMO_CMD") {
            Ok(c) => vec!["sh".into(), "-c".into(), c],
            Err(_) => vec![std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())],
        };
        view.start(SpawnOptions { argv, cwd: std::env::var("HOME").ok(), ..Default::default() }).expect("spawn");
        view.grab_focus();

        if let Ok(input) = std::env::var("DEMO_INPUT") {
            let v = view.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                v.send_input(input.replace("\\n", "\r").as_bytes());
            });
        }
        // Ctrl+Shift+V path without the keyboard: DEMO_PASTE=text pastes it, asking nothing (auto-yes).
        if let Ok(text) = std::env::var("DEMO_PASTE") {
            let v = view.clone();
            view.connect_unsafe_paste({
                let v = v.clone();
                move |t| {
                    eprintln!("unsafe paste refused, confirming: {t:?}");
                    v.paste_text(t, true);
                }
            });
            glib::timeout_add_local_once(std::time::Duration::from_millis(1200), move || {
                v.paste_text(&text.replace("\\n", "\n"), false);
            });
        }
        view.set_custom_links(&[parkett_core::link::CustomLink {
            pattern: r"\bPARK-(\d+)\b".into(),
            target: "https://tracker.example/issue/$1".into(),
        }]);
        view.connect_link_activated(|l| eprintln!("link activated: {l:?}"));
        if let Ok(q) = std::env::var("DEMO_SEARCH") {
            let v = view.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(1800), move || {
                let n = v.search(Some(&q), false);
                eprintln!("search {q:?}: {n:?}");
                v.search_step(false);
                v.search_step(false);
                eprintln!("selected hit {:?} text {:?}", v.search_index(), v.selected_text());
            });
        }
        if let Ok(pos) = std::env::var("DEMO_LINK") {
            let v = view.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(2200), move || {
                for p in pos.split(';') {
                    let (x, y) = p.split_once(',').unwrap();
                    eprintln!("link at {p}: {:?}", v.link_at(x.parse().unwrap(), y.parse().unwrap()));
                }
            });
        }
        if std::env::var("DEMO_SELECT").is_ok() {
            let v = view.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(2000), move || {
                v.select_all();
                eprintln!("selected: {:?}", v.selected_text());
            });
        }
        if let Ok(path) = std::env::var("DEMO_SHOT") {
            let wait = std::env::var("DEMO_WAIT").ok().and_then(|v| v.parse().ok()).unwrap_or(2500);
            let (w, a, v, sc) = (win.clone(), app.clone(), view.clone(), scroller.clone());
            glib::timeout_add_local_once(std::time::Duration::from_millis(wait), move || {
                let paintable = gtk::WidgetPaintable::new(Some(&w));
                let snap = gtk::Snapshot::new();
                paintable.snapshot(&snap, w.width() as f64, w.height() as f64);
                if let (Some(node), Some(native)) = (snap.to_node(), w.native()) {
                    if let Some(r) = native.renderer() {
                        let tex = r.render_texture(
                            &node,
                            Some(&graphene::Rect::new(0.0, 0.0, w.width() as f32, w.height() as f32)),
                        );
                        let _ = tex.save_to_png(&path);
                    }
                }
                let adj = sc.vadjustment();
                eprintln!("scroll value={} upper={} page={}", adj.value(), adj.upper(), adj.page_size());
                eprintln!(
                    "grid {:?} title {:?} pwd {:?} sel {:?}",
                    v.grid_size(),
                    v.title(),
                    v.pwd(),
                    v.selected_text()
                );
                let _ = gdk::Display::default();
                a.quit();
            });
        }
    });
    app.run_with_args::<&str>(&[])
}
