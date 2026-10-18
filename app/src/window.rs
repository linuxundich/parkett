//! Hauptfenster (Konzept A+): Headerbar, AdwTabBar, je Tab ein Split-Baum, Tab-Übersicht.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use parkett_core::layout::{Direction, Orientation};

use crate::profiles::ProfileManager;
use crate::tab::Tab;

mod imp {
    use std::cell::{Cell, OnceCell};

    use super::*;

    #[derive(Default)]
    pub struct Window {
        pub tab_view: OnceCell<adw::TabView>,
        pub overview: OnceCell<adw::TabOverview>,
        pub title: OnceCell<adw::WindowTitle>,
        pub toasts: OnceCell<adw::ToastOverlay>,
        /// Nachfrage beim Schließen wurde bestätigt
        pub force_close: Cell<bool>,
        pub shortcuts: std::cell::RefCell<Option<gtk::ShortcutController>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "ParkettWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for Window {}
    impl WidgetImpl for Window {}
    impl WindowImpl for Window {
        fn close_request(&self) -> glib::Propagation {
            let obj = self.obj();
            if self.force_close.get() {
                obj.save_size();
                return glib::Propagation::Proceed;
            }
            let procs: Vec<String> = obj.tabs().iter().flat_map(|t| t.foreground_processes()).collect();
            if procs.is_empty() {
                obj.save_size();
                return glib::Propagation::Proceed;
            }
            let weak = obj.downgrade();
            obj.confirm_close("Fenster schließen?", &procs, move |ok| {
                if let (true, Some(w)) = (ok, weak.upgrade()) {
                    w.imp().force_close.set(true);
                    w.close();
                }
            });
            glib::Propagation::Stop
        }
    }
    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}


impl Window {
    pub fn new(app: &adw::Application, cwd: Option<String>) -> Self {
        let win: Window = glib::Object::builder()
            .property("application", app)
            .property("title", "Parkett")
            .property("icon-name", crate::APP_ID)
            .build();
        let s = crate::settings::settings();
        win.set_default_size(s.int("window-width"), s.int("window-height"));
        if s.boolean("window-maximized") {
            win.maximize();
        }
        win.build_ui();
        win.setup_actions();
        win.add_tab(cwd);
        win
    }

    fn tab_view(&self) -> &adw::TabView {
        self.imp().tab_view.get().unwrap()
    }

    pub fn tabs(&self) -> Vec<Tab> {
        let view = self.tab_view();
        (0..view.n_pages()).filter_map(|i| view.nth_page(i).child().downcast::<Tab>().ok()).collect()
    }

    fn current_tab(&self) -> Option<Tab> {
        self.tab_view().selected_page().and_then(|p| p.child().downcast::<Tab>().ok())
    }

    fn build_ui(&self) {
        let imp = self.imp();
        let tab_view = adw::TabView::new();
        let tab_bar = adw::TabBar::builder().view(&tab_view).build();
        let settings = crate::settings::settings();
        settings.bind("autohide-tab-bar", &tab_bar, "autohide").build();
        // Abblenden sofort umschalten, wenn die Einstellung sich ändert
        let weak = self.downgrade();
        let dim = settings.connect_changed(Some("dim-unfocused"), move |_, _| {
            if let Some(w) = weak.upgrade() {
                for tab in w.tabs() {
                    tab.update_states();
                }
            }
        });
        let dim = std::cell::RefCell::new(Some(dim));
        self.connect_destroy(move |_| {
            if let Some(id) = dim.borrow_mut().take() {
                crate::settings::settings().disconnect(id);
            }
        });

        let title = adw::WindowTitle::new("Parkett", "");

        // Menü am Neu-Knopf: ein Eintrag je Profil, folgt Änderungen an den Profilen
        let new_menu = gio::Menu::new();
        fill_new_menu(&new_menu);
        let menu_ref = new_menu.clone();
        let handler = ProfileManager::get().connect_changed(move || fill_new_menu(&menu_ref));
        let handler = std::cell::RefCell::new(Some(handler));
        self.connect_destroy(move |_| {
            if let Some(h) = handler.borrow_mut().take() {
                ProfileManager::get().disconnect(h);
            }
        });
        let new_button = adw::SplitButton::builder()
            .icon_name("tab-new-symbolic")
            .tooltip_text("Neuer Reiter (Strg+Umschalt+T)")
            .action_name("win.new-tab")
            .menu_model(&new_menu)
            .dropdown_tooltip("Weitere Möglichkeiten")
            .build();

        let main_menu = gio::Menu::new();
        let s1 = gio::Menu::new();
        s1.append(Some("Neues Fenster"), Some("app.new-window"));
        s1.append(Some("Reiter umbenennen …"), Some("win.rename-tab"));
        main_menu.append_section(None, &s1);
        let s2 = gio::Menu::new();
        s2.append(Some("Einstellungen"), Some("app.preferences"));
        s2.append(Some("Tastenkürzel"), Some("app.shortcuts"));
        s2.append(Some("Über Parkett"), Some("app.about"));
        main_menu.append_section(None, &s2);
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&main_menu)
            .primary(true)
            .tooltip_text("Hauptmenü")
            .build();

        let find_button = gtk::Button::builder().icon_name("edit-find-symbolic").tooltip_text("Suchen (Strg+Umschalt+F)").action_name("win.find").build();
        let overview_button = adw::TabButton::builder().view(&tab_view).action_name("win.overview").build();

        let header = adw::HeaderBar::builder().title_widget(&title).build();
        header.pack_start(&new_button);
        header.pack_end(&menu_button);
        header.pack_end(&overview_button);
        header.pack_end(&find_button);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.add_top_bar(&tab_bar);
        toolbar.set_content(Some(&tab_view));

        let overview = adw::TabOverview::builder()
            .view(&tab_view)
            .child(&toolbar)
            .enable_new_tab(true)
            .enable_search(true)
            .build();
        let weak = self.downgrade();
        overview.connect_create_tab(move |_| {
            let w = weak.upgrade().expect("Fenster lebt, solange die Übersicht lebt");
            let tab = w.add_tab(w.current_tab().and_then(|t| t.inherited_cwd()));
            w.tab_view().page(&tab)
        });

        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&overview));
        self.set_content(Some(&toasts));

        // Tab-Wechsel: Titel nachziehen, Terminal fokussieren
        let weak = self.downgrade();
        tab_view.connect_selected_page_notify(move |view| {
            let Some(w) = weak.upgrade() else { return };
            if let Some(page) = view.selected_page() {
                page.set_needs_attention(false);
                if let Ok(tab) = page.child().downcast::<Tab>() {
                    tab.focus_terminal();
                }
            }
            w.update_title();
        });

        // Schließen eines Tabs: bei laufendem Vordergrundprozess nachfragen
        let weak = self.downgrade();
        tab_view.connect_close_page(move |view, page| {
            let Some(w) = weak.upgrade() else { return glib::Propagation::Proceed };
            let procs = page.child().downcast::<Tab>().map(|t| t.foreground_processes()).unwrap_or_default();
            if procs.is_empty() {
                view.close_page_finish(page, true);
            } else {
                let (view2, page2) = (view.clone(), page.clone());
                w.confirm_close("Reiter schließen?", &procs, move |ok| view2.close_page_finish(&page2, ok));
            }
            glib::Propagation::Stop
        });

        // letzter Tab weg → Fenster zu
        let weak = self.downgrade();
        tab_view.connect_n_pages_notify(move |view| {
            if view.n_pages() == 0 {
                if let Some(w) = weak.upgrade() {
                    w.imp().force_close.set(true);
                    w.close();
                }
            }
        });

        imp.tab_view.set(tab_view).ok();
        imp.overview.set(overview).ok();
        imp.title.set(title).ok();
        imp.toasts.set(toasts).ok();
    }

    pub fn add_tab(&self, cwd: Option<String>) -> Tab {
        self.add_tab_with_profile(cwd, &ProfileManager::get().default_id())
    }

    pub fn add_tab_with_profile(&self, cwd: Option<String>, profile: &str) -> Tab {
        let tab = Tab::new(cwd, profile);
        let page = self.tab_view().append(&tab);
        let weak = self.downgrade();
        tab.connect_changed(move |tab| {
            if let Some(w) = weak.upgrade() {
                w.update_page(tab);
            }
        });
        let weak = self.downgrade();
        tab.connect_empty(move |tab| {
            if let Some(w) = weak.upgrade() {
                let page = w.tab_view().page(tab);
                w.tab_view().close_page(&page);
            }
        });
        self.update_page(&tab);
        self.tab_view().set_selected_page(&page);
        tab
    }

    fn update_page(&self, tab: &Tab) {
        let view = self.tab_view();
        let page = view.page(tab);
        page.set_title(&tab.title());
        page.set_tooltip(&tab.cwd().map(|c| tilde(&c)).unwrap_or_default());
        page.set_keyword(&tab.cwd().unwrap_or_default());
        if tab.has_failed_pane() {
            page.set_indicator_icon(Some(&gio::ThemedIcon::new("dialog-warning-symbolic")));
            page.set_indicator_tooltip("Ein Prozess ist mit Fehler beendet");
            if view.selected_page().as_ref() != Some(&page) {
                page.set_needs_attention(true);
            }
        } else {
            page.set_indicator_icon(gio::Icon::NONE);
        }
        if view.selected_page().as_ref() == Some(&page) {
            self.update_title();
        }
    }

    fn update_title(&self) {
        let title = self.imp().title.get().unwrap();
        match self.current_tab() {
            Some(tab) => {
                let t = tab.title();
                title.set_title(&t);
                title.set_subtitle(&tab.cwd().map(|c| tilde(&c)).unwrap_or_default());
                self.set_title(Some(&t));
            }
            None => title.set_title("Parkett"),
        }
    }

    /// Fenstergröße für das nächste Fenster merken (nur wenn nicht maximiert).
    fn save_size(&self) {
        let s = crate::settings::settings();
        let maximized = self.is_maximized();
        let _ = s.set_boolean("window-maximized", maximized);
        if !maximized {
            let (w, h) = self.default_size();
            let _ = s.set_int("window-width", w);
            let _ = s.set_int("window-height", h);
        }
    }

    fn toast(&self, text: &str) {
        self.imp().toasts.get().unwrap().add_toast(adw::Toast::new(text));
    }

    /// Nachfrage, wenn noch Programme laufen. `on_result(true)` bei „Schließen“, sonst `false`
    /// (auch Esc). Nur `response` auswerten: `closed` kann davor kommen (in der VM gesehen).
    fn confirm_close(&self, heading: &str, procs: &[String], on_result: impl Fn(bool) + 'static) {
        let mut names: Vec<&str> = procs.iter().map(String::as_str).collect();
        names.sort();
        names.dedup();
        let body = format!("Es läuft noch: {}. Beim Schließen werden die Programme beendet.", names.join(", "));
        let dialog = adw::AlertDialog::new(Some(heading), Some(&body));
        dialog.add_responses(&[("cancel", "Abbrechen"), ("close", "Schließen")]);
        dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.connect_response(None, move |_, response| on_result(response == "close"));
        dialog.present(Some(self));
    }

    fn rename_tab(&self) {
        let Some(tab) = self.current_tab() else { return };
        let entry = gtk::Entry::builder()
            .text(tab.custom_title().unwrap_or_default())
            .placeholder_text(tab.focused_pane().terminal().title().unwrap_or_default())
            .activates_default(true)
            .build();
        let dialog = adw::AlertDialog::new(Some("Reiter umbenennen"), Some("Leer lassen, um den Titel des Terminals zu verwenden."));
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", "Abbrechen"), ("rename", "Umbenennen")]);
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("rename"));
        dialog.set_close_response("cancel");
        let e2 = entry.clone();
        dialog.connect_response(Some("rename"), move |_, _| tab.set_custom_title(Some(e2.text().to_string())));
        dialog.present(Some(self));
        // AdwAlertDialog fokussiert sonst die Standard-Antwort, Tippen ginge ins Leere
        entry.grab_focus();
    }

    fn setup_actions(&self) {
        let action = |name: &str, f: fn(&Window)| {
            let a = gio::SimpleAction::new(name, None);
            let weak = self.downgrade();
            a.connect_activate(move |_, _| {
                if let Some(w) = weak.upgrade() {
                    f(&w);
                }
            });
            self.add_action(&a);
        };
        let on_tab = |name: &str, f: fn(&Window, &Tab)| {
            let a = gio::SimpleAction::new(name, None);
            let weak = self.downgrade();
            a.connect_activate(move |_, _| {
                if let Some(w) = weak.upgrade() {
                    if let Some(tab) = w.current_tab() {
                        f(&w, &tab);
                    }
                }
            });
            self.add_action(&a);
        };

        action("new-tab", |w| {
            let cwd = w.current_tab().and_then(|t| t.inherited_cwd());
            w.add_tab(cwd);
        });
        let a = gio::SimpleAction::new("new-tab-profile", Some(glib::VariantTy::STRING));
        let weak = self.downgrade();
        a.connect_activate(move |_, param| {
            let (Some(w), Some(id)) = (weak.upgrade(), param.and_then(|p| p.get::<String>())) else { return };
            let cwd = w.current_tab().and_then(|t| t.inherited_cwd());
            w.add_tab_with_profile(cwd, &id);
        });
        self.add_action(&a);
        action("close-tab", |w| {
            if let Some(page) = w.tab_view().selected_page() {
                w.tab_view().close_page(&page);
            }
        });
        action("next-tab", |w| {
            w.tab_view().select_next_page();
        });
        action("prev-tab", |w| {
            w.tab_view().select_previous_page();
        });
        action("rename-tab", |w| w.rename_tab());
        action("overview", |w| {
            let o = w.imp().overview.get().unwrap();
            o.set_open(!o.is_open());
        });

        fn split(w: &Window, tab: &Tab, o: Option<Orientation>) {
            if !tab.split(o) {
                w.toast("Zu wenig Platz zum Teilen");
            }
        }
        on_tab("split-right", |w, t| split(w, t, Some(Orientation::Horizontal)));
        on_tab("split-down", |w, t| split(w, t, Some(Orientation::Vertical)));
        on_tab("split-auto", |w, t| split(w, t, None));
        on_tab("toggle-zoom", |_, t| t.toggle_zoom());
        on_tab("equalize", |_, t| t.equalize());
        on_tab("close-pane", |w, t| {
            let pane = t.focused_pane();
            let id = pane.id();
            let fg = pane.terminal().foreground_process().filter(|_| crate::settings::confirm_close());
            match fg {
                None => t.close_pane(id),
                Some(p) => {
                    let tab = t.clone();
                    w.confirm_close("Bereich schließen?", &[p], move |ok| {
                        if ok {
                            tab.close_pane(id);
                        }
                    });
                }
            }
        });
        on_tab("find", |_, t| {
            let pane = t.focused_pane();
            if pane.search_visible() { pane.hide_search() } else { pane.show_search() }
        });
        on_tab("find-up", |_, t| t.focused_pane().search_step(true));
        on_tab("find-down", |_, t| t.focused_pane().search_step(false));
        on_tab("refresh-panes", |_, t| t.update_states());
        on_tab("focus-left", |_, t| t.focus_direction(Direction::Left));
        on_tab("focus-right", |_, t| t.focus_direction(Direction::Right));
        on_tab("focus-up", |_, t| t.focus_direction(Direction::Up));
        on_tab("focus-down", |_, t| t.focus_direction(Direction::Down));
        on_tab("resize-left", |_, t| t.resize(Direction::Left));
        on_tab("resize-right", |_, t| t.resize(Direction::Right));
        on_tab("resize-up", |_, t| t.resize(Direction::Up));
        on_tab("resize-down", |_, t| t.resize(Direction::Down));

        // Tab n: Strg+Alt+n, Bereich n: Alt+n (Tilix-Belegung)
        let controller = gtk::ShortcutController::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        for n in 1..=9u32 {
            let a = gio::SimpleAction::new(&format!("goto-tab-{n}"), None);
            let weak = self.downgrade();
            a.connect_activate(move |_, _| {
                if let Some(w) = weak.upgrade() {
                    let view = w.tab_view();
                    if (n as i32) <= view.n_pages() {
                        view.set_selected_page(&view.nth_page(n as i32 - 1));
                    }
                }
            });
            self.add_action(&a);
            let a = gio::SimpleAction::new(&format!("focus-pane-{n}"), None);
            let weak = self.downgrade();
            a.connect_activate(move |_, _| {
                if let Some(t) = weak.upgrade().and_then(|w| w.current_tab()) {
                    t.focus_index(n as usize);
                }
            });
            self.add_action(&a);
            controller.add_shortcut(shortcut(&format!("<Control><Alt>{n}"), &format!("win.goto-tab-{n}")));
            controller.add_shortcut(shortcut(&format!("<Alt>{n}"), &format!("win.focus-pane-{n}")));
        }
        self.add_controller(controller);
        // Änderbare Kürzel (Einstellungen › Tastenkürzel) in eigenem Controller, live neu gebaut.
        // Capture-Phase, damit z. B. Alt+Pfeil nicht in der Shell landet.
        self.rebuild_shortcuts();
        let weak = self.downgrade();
        let id = crate::settings::settings().connect_changed(Some("custom-shortcuts"), move |_, _| {
            if let Some(w) = weak.upgrade() {
                w.rebuild_shortcuts();
            }
        });
        let id = std::cell::RefCell::new(Some(id));
        self.connect_destroy(move |_| {
            if let Some(id) = id.borrow_mut().take() {
                crate::settings::settings().disconnect(id);
            }
        });
    }

    /// Fensterkürzel aus/ein, z. B. während der Einstellungsdialog ein Kürzel aufnimmt.
    pub fn set_shortcuts_enabled(&self, enabled: bool) {
        if let Some(c) = self.imp().shortcuts.borrow().as_ref() {
            c.set_propagation_phase(if enabled { gtk::PropagationPhase::Capture } else { gtk::PropagationPhase::None });
        }
    }

    fn rebuild_shortcuts(&self) {
        let imp = self.imp();
        if let Some(old) = imp.shortcuts.borrow_mut().take() {
            self.remove_controller(&old);
        }
        let c = crate::shortcuts::controller("win.");
        self.add_controller(c.clone());
        *imp.shortcuts.borrow_mut() = Some(c);
    }
}

fn fill_new_menu(menu: &gio::Menu) {
    menu.remove_all();
    let manager = ProfileManager::get();
    let default = manager.default_id();
    let profiles = gio::Menu::new();
    // Standardprofil zuerst, danach alphabetisch
    let mut list = manager.profiles();
    list.sort_by_key(|p| p.id != default);
    for p in list {
        let item = gio::MenuItem::new(Some(&p.name), None);
        item.set_action_and_target_value(Some("win.new-tab-profile"), Some(&p.id.to_variant()));
        profiles.append_item(&item);
    }
    menu.append_section(Some("Neuer Reiter mit Profil"), &profiles);
    let more = gio::Menu::new();
    more.append(Some("Neues Fenster"), Some("app.new-window"));
    more.append(Some("Profile bearbeiten …"), Some("app.preferences"));
    menu.append_section(None, &more);
}

fn shortcut(accel: &str, action: &str) -> gtk::Shortcut {
    gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(accel), Some(gtk::NamedAction::new(action)))
}

fn tilde(path: &str) -> String {
    let home = glib::home_dir();
    match home.to_str() {
        Some(h) if path == h => "~".into(),
        Some(h) if path.starts_with(&format!("{h}/")) => format!("~{}", &path[h.len()..]),
        _ => path.into(),
    }
}
