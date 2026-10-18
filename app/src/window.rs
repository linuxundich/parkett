// SPDX-License-Identifier: GPL-3.0-or-later
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
        /// feste Kürzel Alt+1…9 / Strg+Alt+1…9
        pub fixed_shortcuts: std::cell::OnceCell<gtk::ShortcutController>,
        /// Layout-Modus aktiv; „M“ gedrückt, nächster Pfeil tauscht
        pub layout_mode: Cell<bool>,
        pub pending_swap: Cell<bool>,
        /// Einblendung „Rückgängig“ des zuletzt Geschlossenen; eine neue ersetzt sie
        pub undo_toast: std::cell::RefCell<Option<adw::Toast>>,
        pub split_buttons: OnceCell<(gtk::Button, gtk::Button)>,
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
        win.report_load_errors();
        win
    }

    /// Kaputte Profil-, Paletten- oder Vorlagendateien nennen statt sie still zu überspringen.
    fn report_load_errors(&self) {
        let errors = ProfileManager::get().errors();
        if errors.is_empty() {
            return;
        }
        let toast = adw::Toast::builder()
            .title(format!("{} Datei(en) nicht lesbar, siehe Details", errors.len()))
            .button_label("Details")
            .timeout(10)
            .build();
        let weak = self.downgrade();
        toast.connect_button_clicked(move |_| {
            let Some(w) = weak.upgrade() else { return };
            let d = adw::AlertDialog::new(Some("Nicht lesbare Dateien"), Some(&errors.join("\n\n")));
            d.add_response("ok", "OK");
            d.present(Some(&w));
        });
        self.imp().toasts.get().unwrap().add_toast(toast);
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
        let menu_ref2 = new_menu.clone();
        let handlers = std::cell::RefCell::new(vec![
            ProfileManager::get().connect_changed(move || fill_new_menu(&menu_ref)),
            ProfileManager::get().connect_templates_changed(move || fill_new_menu(&menu_ref2)),
        ]);
        self.connect_destroy(move |_| {
            for h in handlers.borrow_mut().drain(..) {
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
        // Stilwahl System/Hell/Dunkel oben im Menü (wie GNOME Text Editor, Console)
        let style_item = gio::MenuItem::new(None, None);
        style_item.set_attribute_value("custom", Some(&"style".to_variant()));
        let s0 = gio::Menu::new();
        s0.append_item(&style_item);
        main_menu.append_section(None, &s0);
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
        if let Some(popover) = menu_button.popover().and_downcast::<gtk::PopoverMenu>() {
            popover.add_child(&crate::style::selector(), "style");
        }

        let find_button = gtk::Button::builder()
            .icon_name("edit-find-symbolic")
            .tooltip_text("Suchen (Strg+Umschalt+F)")
            .action_name("win.find")
            .build();
        let overview_button = adw::TabButton::builder().view(&tab_view).action_name("win.overview").build();

        // Klick auf den Titel öffnet die Befehlspalette (Vorschlag 2)
        // Erst nach der Doppelklick-Zeit öffnen: Doppelklick auf die Kopfleiste maximiert nur
        let title_click = gtk::GestureClick::new();
        let pending: std::rc::Rc<std::cell::RefCell<Option<glib::SourceId>>> = Default::default();
        let weak = self.downgrade();
        title_click.connect_released(move |_, n_press, _, _| {
            if let Some(id) = pending.borrow_mut().take() {
                id.remove();
            }
            if n_press != 1 {
                return;
            }
            let delay = gtk::Settings::default().map_or(400, |s| s.gtk_double_click_time()).max(0) as u64;
            let weak = weak.clone();
            let p2 = pending.clone();
            let id = glib::timeout_add_local_once(std::time::Duration::from_millis(delay), move || {
                p2.borrow_mut().take();
                if let Some(w) = weak.upgrade() {
                    crate::commands::show(&w);
                }
            });
            *pending.borrow_mut() = Some(id);
        });
        title.add_controller(title_click);
        title.set_tooltip_text(Some("Befehlspalette (Strg+Umschalt+P)"));

        let header = adw::HeaderBar::builder().title_widget(&title).build();
        // AdwSplitButton gibt beiden Teilen den Tooltip des Pfeils als Namen (QA: Orca läse
        // „Weitere Möglichkeiten“ für den Neu-Knopf); daher ausdrücklich benennen.
        new_button.upcast_ref::<gtk::Widget>().update_property(&[gtk::accessible::Property::Label("Neuer Reiter")]);
        if let Some(main) = new_button.first_child() {
            main.update_property(&[gtk::accessible::Property::Label("Neuer Reiter")]);
        }
        find_button.update_property(&[gtk::accessible::Property::Label("Suchen")]);
        header.pack_start(&new_button);
        // Teilen-Knöpfe wie bei Tilix (Wunsch Christoph nach dem ersten eigenen Test)
        let split_right =
            gtk::Button::builder().icon_name("parkett-split-right-symbolic").action_name("win.split-right").build();
        let split_down =
            gtk::Button::builder().icon_name("parkett-split-down-symbolic").action_name("win.split-down").build();
        split_right.update_property(&[gtk::accessible::Property::Label("Rechts teilen")]);
        split_down.update_property(&[gtk::accessible::Property::Label("Unten teilen")]);
        header.pack_start(&split_right);
        header.pack_start(&split_down);
        imp.split_buttons.set((split_right, split_down)).ok();
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
            w.set_layout_mode(false);
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
                w.finish_close_page(page);
            } else {
                let (view2, page2) = (view.clone(), page.clone());
                let w2 = w.downgrade();
                w.confirm_close("Reiter schließen?", &procs, move |ok| match (ok, w2.upgrade()) {
                    (true, Some(w)) => w.finish_close_page(&page2),
                    _ => view2.close_page_finish(&page2, false),
                });
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
        self.attach_tab(Tab::new(cwd, profile))
    }

    /// Schließt einen Reiter endgültig, bietet aber 8 s lang „Rückgängig“ an (Vorschlag 4).
    /// Bis dahin laufen seine Prozesse weiter. Beim letzten Reiter schließt das Fenster.
    fn finish_close_page(&self, page: &adw::TabPage) {
        let view = self.tab_view();
        let tab = page.child().downcast::<Tab>().ok();
        let position = view.page_position(page);
        let title = tab.as_ref().map(|t| t.title()).unwrap_or_default();
        let last = view.n_pages() <= 1;
        view.close_page_finish(page, true);
        let Some(tab) = tab else { return };
        if last {
            tab.terminate_all();
            return;
        }
        let toast = adw::Toast::builder()
            .title(format!("Reiter „{title}“ geschlossen"))
            .button_label("Rückgängig")
            .timeout(8)
            .build();
        let undone = std::rc::Rc::new(std::cell::Cell::new(false));
        let weak = self.downgrade();
        let (t2, u2) = (tab.clone(), undone.clone());
        toast.connect_button_clicked(move |_| {
            let Some(w) = weak.upgrade() else { return };
            u2.set(true);
            let view = w.tab_view();
            let page = view.insert(&t2, position.min(view.n_pages()));
            view.set_selected_page(&page);
            w.update_page(&t2);
        });
        toast.connect_dismissed(move |_| {
            if !undone.get() {
                tab.terminate_all();
            }
        });
        self.show_undo_toast(toast);
    }

    /// Zeigt eine Rückgängig-Einblendung. Eine noch offene wird vorher geschlossen – sonst
    /// stünden die Einblendungen Schlange (je 8 s) und die Prozesse liefen so lange weiter
    /// (in der QA gefunden: 7 geschlossene Reiter, nach 14 s noch 15 Shells).
    fn show_undo_toast(&self, toast: adw::Toast) {
        if let Some(old) = self.imp().undo_toast.replace(Some(toast.clone())) {
            old.dismiss();
        }
        self.imp().toasts.get().unwrap().add_toast(toast);
    }

    fn attach_tab(&self, tab: Tab) -> Tab {
        let page = self.tab_view().append(&tab);
        let weak = self.downgrade();
        tab.connect_changed(move |tab| {
            if let Some(w) = weak.upgrade() {
                w.update_page(tab);
            }
        });
        // geschlossener Bereich: „Rückgängig“ anbieten, danach Prozess beenden
        let weak = self.downgrade();
        tab.connect_pane_closed(move |tab, serial| {
            let Some(w) = weak.upgrade() else { return };
            let toast =
                adw::Toast::builder().title("Bereich geschlossen").button_label("Rückgängig").timeout(8).build();
            let undone = std::rc::Rc::new(std::cell::Cell::new(false));
            let (t2, u2) = (tab.clone(), undone.clone());
            toast.connect_button_clicked(move |_| {
                u2.set(true);
                t2.restore_closed(serial);
            });
            let t3 = tab.clone();
            toast.connect_dismissed(move |_| {
                if !undone.get() {
                    t3.discard_closed(serial);
                }
            });
            w.show_undo_toast(toast);
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
                if self.imp().layout_mode.get() {
                    title.set_subtitle("Layout-Modus · Esc beendet");
                } else {
                    title.set_subtitle(&tab.cwd().map(|c| tilde(&c)).unwrap_or_default());
                }
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
        let dialog = adw::AlertDialog::new(
            Some("Reiter umbenennen"),
            Some("Leer lassen, um den Titel des Terminals zu verwenden."),
        );
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
        action("palette", crate::commands::show);
        action("layout-mode", |w| {
            let on = !w.imp().layout_mode.get();
            w.set_layout_mode(on);
        });
        action("save-template", |w| w.save_template());
        let a = gio::SimpleAction::new("open-template", Some(glib::VariantTy::STRING));
        let weak = self.downgrade();
        a.connect_activate(move |_, param| {
            if let (Some(w), Some(id)) = (weak.upgrade(), param.and_then(|p| p.get::<String>())) {
                w.open_template(&id);
            }
        });
        self.add_action(&a);
        // Tasten im Layout-Modus (Capture-Phase, vor Terminal und Kürzeln)
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.downgrade();
        keys.connect_key_pressed(move |_, key, _, state| match weak.upgrade() {
            // Nur wenn ein Terminal den Fokus hat – Dialoge (Nachfrage, Palette) bekommen ihre Tasten
            Some(w) if w.imp().layout_mode.get() && w.terminal_has_focus() => w.layout_key(key, state),
            _ => glib::Propagation::Proceed,
        });
        self.add_controller(keys);
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
            if pane.search_visible() {
                pane.hide_search()
            } else {
                pane.show_search()
            }
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
        self.add_controller(controller.clone());
        let _ = self.imp().fixed_shortcuts.set(controller);
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

    pub fn select_tab(&self, index: usize) {
        let view = self.tab_view();
        if (index as i32) < view.n_pages() {
            view.set_selected_page(&view.nth_page(index as i32));
        }
    }

    /// Führt eine Aktion aus der Kürzel-Liste aus; `term.*` im fokussierten Terminal.
    pub fn run_action(&self, action: &str) {
        if action.starts_with("term.") {
            if let Some(tab) = self.current_tab() {
                let _ = tab.focused_pane().terminal().vte().activate_action(action, None);
            }
        } else {
            let _ = WidgetExt::activate_action(self, action, None);
        }
    }

    pub fn set_layout_mode(&self, on: bool) {
        let imp = self.imp();
        if imp.layout_mode.get() == on {
            return;
        }
        imp.layout_mode.set(on);
        imp.pending_swap.set(false);
        if on {
            if let Some(tab) = self.current_tab() {
                tab.set_layout_mode(true);
            }
        } else {
            // alle, nicht nur den aktuellen: beim Reiterwechsel ist der alte schon nicht mehr aktuell
            for tab in self.tabs() {
                tab.set_layout_mode(false);
            }
        }
        self.update_title();
    }

    fn terminal_has_focus(&self) -> bool {
        GtkWindowExt::focus(self).is_some_and(|f| f.ancestor(vte::Terminal::static_type()).is_some())
    }

    /// Tasten im Layout-Modus. Alles andere wird verschluckt, damit nichts in der Shell landet.
    fn layout_key(&self, key: gtk::gdk::Key, state: gtk::gdk::ModifierType) -> glib::Propagation {
        use gtk::gdk::Key;
        let Some(tab) = self.current_tab() else { return glib::Propagation::Proceed };
        let imp = self.imp();
        let dir = match key {
            Key::Left | Key::KP_Left => Some(Direction::Left),
            Key::Right | Key::KP_Right => Some(Direction::Right),
            Key::Up | Key::KP_Up => Some(Direction::Up),
            Key::Down | Key::KP_Down => Some(Direction::Down),
            _ => None,
        };
        if let Some(d) = dir {
            if imp.pending_swap.replace(false) {
                tab.swap_direction(d);
            } else if state.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
                tab.resize(d);
            } else {
                tab.focus_direction(d);
            }
            return glib::Propagation::Stop;
        }
        match key.to_lower() {
            Key::Escape | Key::Return | Key::KP_Enter => self.set_layout_mode(false),
            Key::r => {
                if !tab.split(Some(Orientation::Horizontal)) {
                    self.toast("Zu wenig Platz zum Teilen");
                }
            }
            Key::d => {
                if !tab.split(Some(Orientation::Vertical)) {
                    self.toast("Zu wenig Platz zum Teilen");
                }
            }
            Key::a => {
                if !tab.split(None) {
                    self.toast("Zu wenig Platz zum Teilen");
                }
            }
            Key::z => tab.toggle_zoom(),
            Key::equal | Key::KP_Equal => tab.equalize(),
            Key::x => {
                let _ = WidgetExt::activate_action(self, "win.close-pane", None);
            }
            Key::m => imp.pending_swap.set(true),
            k => {
                if let Some(c) = k.to_unicode().and_then(|c| c.to_digit(10)).filter(|n| (1..=9).contains(n)) {
                    tab.focus_index(c as usize);
                }
            }
        }
        // Nummern und Zustand nach Umbauten auffrischen (neuer Bereich, Schließen …)
        if let Some(t) = self.current_tab() {
            t.set_layout_mode(imp.layout_mode.get());
        }
        glib::Propagation::Stop
    }

    /// Dialog „Vorlage öffnen“: Verzeichnis wählen, Startbefehle zeigen und ein-/ausschalten.
    pub fn open_template(&self, id: &str) {
        let Some(t) = ProfileManager::get().template(id) else { return };
        let home = glib::home_dir();
        let start = if t.directory.trim().is_empty() {
            self.current_tab().and_then(|tab| tab.cwd()).map(std::path::PathBuf::from).unwrap_or(home.clone())
        } else {
            parkett_core::paths::expand_home(t.directory.trim(), &home)
        };
        let dir = std::rc::Rc::new(std::cell::RefCell::new(start));
        let n = t.layout.panes().len();
        let dialog = adw::AlertDialog::new(
            Some(&format!("Vorlage „{}“ öffnen", t.name)),
            Some(&format!("Öffnet einen neuen Reiter mit {n} {}.", if n == 1 { "Bereich" } else { "Bereichen" })),
        );
        let list = gtk::ListBox::builder().css_classes(["boxed-list"]).selection_mode(gtk::SelectionMode::None).build();
        let dir_row =
            adw::ActionRow::builder().title("Verzeichnis").subtitle(tilde(&dir.borrow().to_string_lossy())).build();
        let pick = gtk::Button::builder()
            .icon_name("folder-open-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text("Verzeichnis wählen")
            .css_classes(["flat"])
            .build();
        dir_row.add_suffix(&pick);
        list.append(&dir_row);
        let commands = t.commands();
        let run = adw::SwitchRow::builder()
            .title("Startbefehle ausführen")
            .subtitle(if commands.is_empty() { "Keine Startbefehle".to_string() } else { commands.join(" · ") })
            .active(!commands.is_empty())
            .sensitive(!commands.is_empty())
            .build();
        list.append(&run);
        dialog.set_extra_child(Some(&list));
        dialog.add_responses(&[("cancel", "Abbrechen"), ("open", "Öffnen")]);
        dialog.set_response_appearance("open", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("open"));
        dialog.set_close_response("cancel");
        let d2 = dir.clone();
        let row2 = dir_row.clone();
        let weak = self.downgrade();
        pick.connect_clicked(move |_| {
            let Some(w) = weak.upgrade() else { return };
            let chooser = gtk::FileDialog::builder()
                .title("Verzeichnis für die Vorlage")
                .initial_folder(&gtk::gio::File::for_path(&*d2.borrow()))
                .modal(true)
                .build();
            let d3 = d2.clone();
            let row3 = row2.clone();
            chooser.select_folder(Some(&w), gtk::gio::Cancellable::NONE, move |res| {
                if let Some(path) = res.ok().and_then(|f| f.path()) {
                    row3.set_subtitle(&tilde(&path.to_string_lossy()));
                    *d3.borrow_mut() = path;
                }
            });
        });
        let weak = self.downgrade();
        dialog.connect_response(Some("open"), move |_, _| {
            let Some(w) = weak.upgrade() else { return };
            let dir = dir.borrow().clone();
            if !dir.is_dir() {
                w.toast(&format!("Verzeichnis „{}“ gibt es nicht", tilde(&dir.to_string_lossy())));
                return;
            }
            w.attach_tab(Tab::from_template(&t, &dir, run.is_active()));
        });
        dialog.present(Some(self));
    }

    /// Dialog „Reiter als Vorlage speichern“.
    fn save_template(&self) {
        let Some(tab) = self.current_tab() else { return };
        let suggestion = tab.custom_title().unwrap_or_else(|| {
            tab.cwd().and_then(|c| c.rsplit('/').next().map(String::from)).unwrap_or_else(|| "Vorlage".into())
        });
        let (preview, _) = tab.to_template(&suggestion);
        let n = preview.layout.panes().len();
        let commands = preview.commands();
        let body = format!(
            "Speichert Aufteilung und Profile der {n} {}{}. Das Verzeichnis wird beim Öffnen gefragt.",
            if n == 1 { "Bereichs" } else { "Bereiche" },
            if commands.is_empty() {
                String::new()
            } else {
                format!(" sowie die laufenden Befehle ({})", commands.join(", "))
            }
        );
        let entry = gtk::Entry::builder().text(&suggestion).activates_default(true).build();
        let dialog = adw::AlertDialog::new(Some("Reiter als Vorlage speichern"), Some(&body));
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", "Abbrechen"), ("save", "Speichern")]);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let weak = self.downgrade();
        let e2 = entry.clone();
        dialog.connect_response(Some("save"), move |_, _| {
            let Some(w) = weak.upgrade() else { return };
            let name = e2.text().trim().to_string();
            let name = if name.is_empty() { "Vorlage".to_string() } else { name };
            let (t, _) = tab.to_template(&name);
            match ProfileManager::get().save_template(&t) {
                Ok(t) => w.toast(&format!("Vorlage „{}“ gespeichert", t.name)),
                Err(e) => w.toast(&format!("Vorlage nicht gespeichert: {e}")),
            }
        });
        dialog.present(Some(self));
        entry.grab_focus();
    }

    /// Fensterkürzel aus/ein, z. B. während der Einstellungsdialog ein Kürzel aufnimmt.
    pub fn set_shortcuts_enabled(&self, enabled: bool) {
        let phase = if enabled { gtk::PropagationPhase::Capture } else { gtk::PropagationPhase::None };
        if let Some(c) = self.imp().shortcuts.borrow().as_ref() {
            c.set_propagation_phase(phase);
        }
        if let Some(c) = self.imp().fixed_shortcuts.get() {
            c.set_propagation_phase(phase);
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
        // Tooltips mit dem aktuellen Kürzel
        if let Some((right, down)) = imp.split_buttons.get() {
            let tip = |label: &str, action: &str| {
                let accels = crate::shortcuts::effective(action);
                if accels.is_empty() {
                    label.to_string()
                } else {
                    format!("{label} ({})", crate::shortcuts::label(&accels))
                }
            };
            right.set_tooltip_text(Some(&tip("Rechts teilen", "win.split-right")));
            down.set_tooltip_text(Some(&tip("Unten teilen", "win.split-down")));
        }
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
    let templates = gio::Menu::new();
    for t in manager.templates() {
        let item = gio::MenuItem::new(Some(&t.name), None);
        item.set_action_and_target_value(Some("win.open-template"), Some(&t.id.to_variant()));
        templates.append_item(&item);
    }
    templates.append(Some("Aktuellen Reiter als Vorlage speichern …"), Some("win.save-template"));
    menu.append_section(Some("Vorlagen"), &templates);
    let more = gio::Menu::new();
    more.append(Some("Neues Fenster"), Some("app.new-window"));
    more.append(Some("Profile bearbeiten …"), Some("app.preferences"));
    menu.append_section(None, &more);
}

fn shortcut(accel: &str, action: &str) -> gtk::Shortcut {
    gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(accel), Some(gtk::NamedAction::new(action)))
}

fn tilde(path: &str) -> String {
    parkett_core::paths::tilde(path, &glib::home_dir())
}
