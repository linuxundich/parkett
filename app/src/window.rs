// SPDX-License-Identifier: GPL-3.0-or-later
//! Main window (concept A+): header bar, AdwTabBar, one split tree per tab, tab overview.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use parkett_core::layout::{Direction, Orientation};

use crate::i18n::{gettext, ntrf, trf, N_};
use crate::profiles::ProfileManager;
use crate::tab::Tab;
use crate::terminal::Launch;

mod imp {
    use std::cell::{Cell, OnceCell};

    use super::*;

    #[derive(Default)]
    pub struct Window {
        pub tab_view: OnceCell<adw::TabView>,
        pub overview: OnceCell<adw::TabOverview>,
        pub title: OnceCell<adw::WindowTitle>,
        pub toasts: OnceCell<adw::ToastOverlay>,
        /// confirmation on close was accepted
        pub force_close: Cell<bool>,
        pub shortcuts: std::cell::RefCell<Option<gtk::ShortcutController>>,
        /// fixed shortcuts Alt+1…9 / Ctrl+Alt+1…9
        pub fixed_shortcuts: std::cell::OnceCell<gtk::ShortcutController>,
        /// layout mode active; "M" pressed, next arrow swaps
        pub layout_mode: Cell<bool>,
        pub pending_swap: Cell<bool>,
        /// "Undo" toast of the last closed item; a new one replaces it
        pub undo_toast: std::cell::RefCell<Option<adw::Toast>>,
        pub new_button: OnceCell<adw::SplitButton>,
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
                crate::session::window_closing(&obj);
                return glib::Propagation::Proceed;
            }
            let procs: Vec<String> = obj.tabs().iter().flat_map(|t| t.foreground_processes()).collect();
            if procs.is_empty() {
                obj.save_size();
                crate::session::window_closing(&obj);
                return glib::Propagation::Proceed;
            }
            let weak = obj.downgrade();
            obj.confirm_close(&gettext("Close window?"), &procs, move |ok| {
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
        let win = Window::empty(app);
        win.add_tab(cwd);
        win
    }

    /// Window without tabs; the caller adds some right away (command line, session).
    pub fn empty(app: &adw::Application) -> Self {
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
        win.report_load_errors();
        win
    }

    /// State for the session: all tabs plus the selected tab.
    pub fn to_session(&self) -> parkett_core::session::SessionWindow {
        let view = self.tab_view();
        let selected = view.selected_page().map(|p| view.page_position(&p).max(0) as usize).unwrap_or(0);
        parkett_core::session::SessionWindow { selected, tabs: self.tabs().iter().map(Tab::to_session).collect() }
    }

    /// Open tabs from a saved session.
    pub fn restore_session(&self, w: &parkett_core::session::SessionWindow) {
        for t in &w.tabs {
            self.attach_tab(Tab::from_template(t, std::path::Path::new("/"), false));
        }
        let view = self.tab_view();
        if (w.selected as i32) < view.n_pages() {
            view.set_selected_page(&view.nth_page(w.selected as i32));
        }
    }

    /// Tab according to the command line: profile, command instead of shell, title.
    pub fn add_tab_with(&self, cwd: Option<String>, profile: &str, launch: Launch, title: Option<String>) -> Tab {
        let tab = Tab::with_launch(cwd, profile, launch);
        tab.set_custom_title(title);
        self.attach_tab(tab)
    }

    /// Name broken profile, palette or template files instead of silently skipping them.
    fn report_load_errors(&self) {
        let errors = ProfileManager::get().errors();
        if errors.is_empty() {
            return;
        }
        // No timeout: otherwise the notice cannot be brought up again later (QA-15)
        let toast = adw::Toast::builder()
            .title(ntrf(
                "{n} file could not be read, see details",
                "{n} files could not be read, see details",
                errors.len() as u32,
                &[],
            ))
            .button_label(gettext("Details"))
            .timeout(0)
            .build();
        let weak = self.downgrade();
        toast.connect_button_clicked(move |_| {
            let Some(w) = weak.upgrade() else { return };
            let d = adw::AlertDialog::new(
                Some(&gettext("Unreadable files")),
                Some(&gettext("Parkett skipped these files and left them unchanged.")),
            );
            // Monospace and left-aligned: TOML errors point at the spot with "^" (QA-15).
            // Not selectable: a selectable label would get the focus, be fully selected and scrolled to
            // the right; for passing it on there is "Copy".
            let home = glib::home_dir();
            let text = errors
                .iter()
                .map(|e| match e.trim_end().split_once('\n') {
                    Some((path, rest)) => format!("{}\n{rest}", parkett_core::paths::tilde(path, &home)),
                    None => e.trim_end().to_string(),
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            let label = gtk::Label::builder().label(&text).xalign(0.0).css_classes(["monospace"]).build();
            label.set_margin_start(12);
            label.set_margin_end(12);
            label.set_margin_top(12);
            label.set_margin_bottom(12);
            let scroll = gtk::ScrolledWindow::builder()
                .child(&label)
                .propagate_natural_height(true)
                .max_content_height(360)
                .css_classes(["card"])
                .build();
            d.set_extra_child(Some(&scroll));
            d.add_response("copy", &gettext("Copy"));
            d.add_response("ok", &gettext("OK"));
            d.set_default_response(Some("ok"));
            d.connect_response(Some("copy"), move |d, _| d.clipboard().set_text(&text));
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

    pub fn current_tab(&self) -> Option<Tab> {
        self.tab_view().selected_page().and_then(|p| p.child().downcast::<Tab>().ok())
    }

    fn build_ui(&self) {
        let imp = self.imp();
        let tab_view = adw::TabView::new();
        let tab_bar = adw::TabBar::builder().view(&tab_view).build();
        // Drag a pane onto a tab in the bar: move it there (also from another
        // window); the process keeps running
        tab_bar.setup_extra_drop_target(gtk::gdk::DragAction::MOVE, &[String::static_type()]);
        // Without our own answer the bar reports an action that does not fit MOVE
        // (gdk_drop_status: assertion), and the drop never ends (VM 2026-10-23)
        tab_bar.set_extra_drag_preload(true);
        tab_bar.connect_extra_drag_value(|_, _, value| match value.and_then(|v| v.get::<String>().ok()) {
            Some(key) if crate::pane::parse_drag_key(&key).is_some() => gtk::gdk::DragAction::MOVE,
            _ => gtk::gdk::DragAction::empty(),
        });
        let weak = self.downgrade();
        tab_bar.connect_extra_drag_drop(move |_, page, value| {
            let (Some(w), Ok(target), Ok(key)) =
                (weak.upgrade(), page.child().downcast::<Tab>(), value.get::<String>())
            else {
                return false;
            };
            w.move_pane_to(&key, &target)
        });
        let settings = crate::settings::settings();
        settings.bind("autohide-tab-bar", &tab_bar, "autohide").build();
        // Switch dimming right away when the setting changes
        let weak = self.downgrade();
        let dim = settings.connect_changed(Some("dim-unfocused"), move |_, _| {
            if let Some(w) = weak.upgrade() {
                for tab in w.tabs() {
                    tab.update_states();
                }
            }
        });
        let weak = self.downgrade();
        let labels = settings.connect_changed(None, move |_, key| {
            if !matches!(key, "short-tab-labels" | "tab-label-template" | "window-title-template") {
                return;
            }
            if let Some(w) = weak.upgrade() {
                w.update_labels();
                w.update_title();
            }
        });
        let handlers = std::cell::RefCell::new(vec![dim, labels]);
        self.connect_destroy(move |_| {
            for id in handlers.borrow_mut().drain(..) {
                crate::settings::settings().disconnect(id);
            }
        });

        let title = adw::WindowTitle::new("Parkett", "");

        // Menu on the New button: one entry per profile, follows changes to the profiles
        let new_menu = gio::Menu::new();
        fill_new_menu(&new_menu);
        let menu_ref = new_menu.clone();
        let menu_ref2 = new_menu.clone();
        // Templates in the main menu too (audit proposal 25)
        let template_menu = gio::Menu::new();
        fill_template_menu(&template_menu);
        let menu_ref3 = template_menu.clone();
        let handlers = std::cell::RefCell::new(vec![
            ProfileManager::get().connect_changed(move || fill_new_menu(&menu_ref)),
            ProfileManager::get().connect_templates_changed(move || {
                fill_new_menu(&menu_ref2);
                fill_template_menu(&menu_ref3);
            }),
        ]);
        self.connect_destroy(move |_| {
            for h in handlers.borrow_mut().drain(..) {
                ProfileManager::get().disconnect(h);
            }
        });
        let new_button = adw::SplitButton::builder()
            .icon_name("tab-new-symbolic")
            .action_name("win.new-tab")
            .menu_model(&new_menu)
            .dropdown_tooltip(gettext("More options"))
            .build();

        let main_menu = gio::Menu::new();
        // Style selector System/Light/Dark at the top of the menu (like GNOME Text Editor, Console)
        let style_item = gio::MenuItem::new(None, None);
        style_item.set_attribute_value("custom", Some(&"style".to_variant()));
        let s0 = gio::Menu::new();
        s0.append_item(&style_item);
        main_menu.append_section(None, &s0);
        let s1 = gio::Menu::new();
        s1.append(Some(&gettext("New window")), Some("app.new-window"));
        s1.append(Some(&gettext("Rename tab…")), Some("win.rename-tab"));
        main_menu.append_section(None, &s1);
        // Make the palette, layout mode and templates discoverable, not only via shortcuts and
        // the arrow on the New button (audit proposal 25)
        let s_tools = gio::Menu::new();
        s_tools.append(Some(&gettext("Command palette")), Some("win.palette"));
        s_tools.append(Some(&gettext("Layout mode")), Some("win.layout-mode"));
        s_tools.append_submenu(Some(&gettext("Templates")), &template_menu);
        let bookmark_menu = gio::Menu::new();
        fill_bookmark_menu(&bookmark_menu);
        s_tools.append_submenu(Some(&gettext("Bookmarks")), &bookmark_menu);
        main_menu.append_section(None, &s_tools);
        let s2 = gio::Menu::new();
        s2.append(Some(&gettext("Preferences")), Some("app.preferences"));
        s2.append(Some(&gettext("Keyboard shortcuts")), Some("app.shortcuts"));
        s2.append(Some(&gettext("About Parkett")), Some("app.about"));
        main_menu.append_section(None, &s2);
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .menu_model(&main_menu)
            // not "primary": otherwise F10 opens the menu, and htop/mc could no longer be
            // quit with F10 (QA-01; GNOME Terminal turns F10 off as well)
            .tooltip_text(gettext("Main menu"))
            .build();
        // Bookmarks fresh on every opening: the file may be edited by hand
        menu_button.connect_active_notify(move |b| {
            if b.is_active() {
                fill_bookmark_menu(&bookmark_menu);
            }
        });
        if let Some(popover) = menu_button.popover().and_downcast::<gtk::PopoverMenu>() {
            popover.add_child(&crate::style::selector(), "style");
            self.refocus_after(&popover);
        }
        if let Some(popover) = new_button.popover() {
            self.refocus_after(&popover);
        }

        let find_button = gtk::Button::builder().icon_name("edit-find-symbolic").action_name("win.find").build();
        crate::shortcuts::live_tooltip(&find_button, N_("Search"), "win.find", &[]);
        let overview_button = adw::TabButton::builder().view(&tab_view).action_name("win.overview").build();
        crate::shortcuts::live_tooltip(&overview_button, N_("Tab overview"), "win.overview", &[]);
        // Name with count instead of just the number the button shows (QA-18)
        let button = overview_button.clone();
        let name_overview = move |view: &adw::TabView| {
            let label = ntrf("Tab overview, {n} tab", "Tab overview, {n} tabs", view.n_pages() as u32, &[]);
            button.upcast_ref::<gtk::Widget>().update_property(&[gtk::accessible::Property::Label(&label)]);
        };
        name_overview(&tab_view);
        tab_view.connect_n_pages_notify(name_overview);

        // Click on the title opens the command palette (proposal 2)
        // Open only after the double-click time: a double-click on the header bar only maximizes
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
        crate::shortcuts::tooltip_only(&title, N_("Command palette"), "win.palette", &[]);

        let header = adw::HeaderBar::builder().title_widget(&title).build();
        // AdwSplitButton gives both parts the arrow's tooltip as name (QA: Orca would read
        // "More options" for the New button); so name it explicitly.
        new_button
            .upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Label(&gettext("New tab"))]);
        // The description as well: otherwise it also comes from the arrow, Orca would read "New tab,
        // More options" (QA-18, AT-SPI in the VM 2026-10-22)
        // libadwaita sets it again on every tooltip change, so repeat it afterwards.
        let name_main = |b: &adw::SplitButton| {
            if let Some(main) = b.first_child() {
                // libadwaita points to the whole button via described-by; otherwise the description
                // is computed from that, not from the property
                main.reset_relation(gtk::AccessibleRelation::DescribedBy);
                main.update_property(&[
                    gtk::accessible::Property::Label(&gettext("New tab")),
                    gtk::accessible::Property::Description(&gettext("Open a new tab")),
                ]);
            }
        };
        name_main(&new_button);
        new_button.connect_notify_local(None, move |b, pspec| {
            if pspec.name().starts_with("tooltip") || pspec.name() == "dropdown-tooltip" {
                name_main(b);
            }
        });
        header.pack_start(&new_button);
        // Split buttons like in Tilix (Christoph's wish after his first own test)
        let split_right =
            gtk::Button::builder().icon_name("parkett-split-right-symbolic").action_name("win.split-right").build();
        let split_down =
            gtk::Button::builder().icon_name("parkett-split-down-symbolic").action_name("win.split-down").build();
        crate::shortcuts::live_tooltip(&split_right, N_("Split right"), "win.split-right", &[]);
        crate::shortcuts::live_tooltip(&split_down, N_("Split down"), "win.split-down", &[]);
        header.pack_start(&split_right);
        header.pack_start(&split_down);
        // Narrow window (tiling on small screens): hide split buttons and search,
        // otherwise the header bar sets the minimum width (≈ 550 px, QA-13). Splitting stays in the
        // menu on the New button and via shortcut, search via shortcut and context menu.
        let narrow =
            adw::Breakpoint::new(adw::BreakpointCondition::parse("max-width: 600sp").expect("valid condition"));
        for w in [split_right.upcast_ref::<glib::Object>(), split_down.upcast_ref(), find_button.upcast_ref()] {
            narrow.add_setter(w, "visible", Some(&false.to_value()));
        }
        self.add_breakpoint(narrow);
        // Breakpoints require a fixed minimum size; 360 px as in the GNOME HIG
        self.set_size_request(360, 294);
        imp.new_button.set(new_button.clone()).ok();
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
            let w = weak.upgrade().expect("window lives as long as the overview");
            let tab = w.add_tab(w.current_tab().and_then(|t| t.inherited_cwd()));
            w.tab_view().page(&tab)
        });

        // Overview closed (Esc or selection): focus into the terminal of the selected tab
        let weak = self.downgrade();
        overview.connect_open_notify(move |o| {
            if let (false, Some(w)) = (o.is_open(), weak.upgrade()) {
                if let Some(tab) = w.current_tab() {
                    tab.focus_terminal();
                }
            }
        });

        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&overview));
        self.set_content(Some(&toasts));

        // Tab switch: update the title, focus the terminal
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
            w.update_sync_actions();
        });

        // Drag a tab out of the window and drop it beside: new window for it. The
        // tab's handlers look up their window when called (R-01), processes keep running.
        let weak = self.downgrade();
        tab_view.connect_create_window(move |_| {
            let app = weak.upgrade()?.application().and_downcast::<adw::Application>()?;
            let w = Window::empty(&app);
            w.present();
            Some(w.tab_view().clone())
        });

        // Click on the shared input icon turns it off for this tab
        let weak = self.downgrade();
        tab_view.connect_indicator_activated(move |_, page| {
            let (Some(w), Ok(tab)) = (weak.upgrade(), page.child().downcast::<Tab>()) else { return };
            if tab.sync() {
                tab.set_sync(false);
                w.update_sync_actions();
                w.toast(&gettext("Shared input off"));
            }
        });

        // Closing a tab: ask if a foreground process is running
        let weak = self.downgrade();
        tab_view.connect_close_page(move |view, page| {
            let Some(w) = weak.upgrade() else { return glib::Propagation::Proceed };
            let procs = page.child().downcast::<Tab>().map(|t| t.foreground_processes()).unwrap_or_default();
            if procs.is_empty() {
                w.finish_close_page(page);
            } else {
                let (view2, page2) = (view.clone(), page.clone());
                let w2 = w.downgrade();
                w.confirm_close(&gettext("Close tab?"), &procs, move |ok| match (ok, w2.upgrade()) {
                    (true, Some(w)) => w.finish_close_page(&page2),
                    _ => view2.close_page_finish(&page2, false),
                });
            }
            glib::Propagation::Stop
        });

        // last tab gone → close window; otherwise the numbers in the labels are wrong
        let weak = self.downgrade();
        tab_view.connect_n_pages_notify(move |view| {
            let Some(w) = weak.upgrade() else { return };
            if view.n_pages() == 0 {
                w.imp().force_close.set(true);
                w.close();
            } else {
                w.update_labels();
            }
        });
        let weak = self.downgrade();
        tab_view.connect_page_reordered(move |_, _, _| {
            if let Some(w) = weak.upgrade() {
                w.update_labels();
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

    /// Closes a tab for good, but offers "Undo" for 8 s (proposal 4).
    /// Until then its processes keep running. For the last tab the window closes.
    fn finish_close_page(&self, page: &adw::TabPage) {
        crate::session::schedule_save();
        let view = self.tab_view();
        let tab = page.child().downcast::<Tab>().ok();
        // empty because its last pane was dragged into another tab: nothing
        // to restore, so without "Undo"
        if tab.as_ref().is_some_and(|t| t.pane_count() == 0) {
            view.close_page_finish(page, true);
            return;
        }
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
            .title(trf("Tab “{name}” closed", &[("name", &glib::markup_escape_text(&title))]))
            .button_label(gettext("Undo"))
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

    /// Shows an undo toast. One still open is closed first – otherwise
    /// the toasts would queue up (8 s each) and the processes would keep running that long
    /// (found in QA: 7 closed tabs, still 15 shells after 14 s).
    fn show_undo_toast(&self, toast: adw::Toast) {
        if let Some(old) = self.imp().undo_toast.replace(Some(toast.clone())) {
            old.dismiss();
        }
        // Let go after expiry: through its handlers the toast holds the closed
        // tab including its VTE buffer (QA-03).
        let weak = self.downgrade();
        toast.connect_dismissed(move |t| {
            let Some(w) = weak.upgrade() else { return };
            let mut slot = w.imp().undo_toast.borrow_mut();
            if slot.as_ref() == Some(t) {
                slot.take();
            }
        });
        self.imp().toasts.get().unwrap().add_toast(toast);
    }

    /// After closing a menu the focus would otherwise stay on its button: typing would get
    /// lost, and Enter would open the menu again (QA-09). If a menu item opened a dialog,
    /// that one keeps the focus.
    fn refocus_after(&self, popover: &impl IsA<gtk::Popover>) {
        let weak = self.downgrade();
        popover.connect_closed(move |_| {
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                let Some(w) = weak.upgrade() else { return };
                let in_header = gtk::prelude::GtkWindowExt::focus(&w)
                    .is_none_or(|f| f.ancestor(adw::HeaderBar::static_type()).is_some());
                if in_header {
                    if let Some(tab) = w.current_tab() {
                        tab.focus_terminal();
                    }
                }
            });
        });
    }

    /// Page of a tab if it is (still) in this window. `TabView::page` assumes that;
    /// but a closed tab within the undo period or one dragged into another window
    /// still reports in (R-01).
    fn page_of(&self, tab: &Tab) -> Option<adw::TabPage> {
        let view = self.tab_view();
        (0..view.n_pages()).map(|i| view.nth_page(i)).find(|p| p.child() == *tab.upcast_ref::<gtk::Widget>())
    }

    fn attach_tab(&self, tab: Tab) -> Tab {
        let page = self.tab_view().append(&tab);
        // Determine the window only when called: a tab can be dragged into another window,
        // and a closed one (undo period) has none at all (R-01).
        let window_of = |tab: &Tab| tab.root().and_downcast::<Window>();
        tab.connect_changed(move |tab| {
            if let Some(w) = window_of(tab) {
                w.update_page(tab);
                // Focus change in the tab: update the "receives shared input" check mark
                if w.current_tab().as_ref() == Some(tab) {
                    w.sync_action_states();
                }
            }
        });
        // closed pane: offer "Undo", then end the process
        tab.connect_pane_closed(move |tab, serial| {
            let toast =
                adw::Toast::builder().title(gettext("Pane closed")).button_label(gettext("Undo")).timeout(8).build();
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
            match window_of(tab) {
                Some(w) => w.show_undo_toast(toast),
                None => tab.discard_closed(serial),
            }
        });
        tab.connect_empty(move |tab| {
            if let Some(w) = window_of(tab) {
                if let Some(page) = w.page_of(tab) {
                    w.tab_view().close_page(&page);
                }
            }
        });
        self.update_page(&tab);
        self.tab_view().set_selected_page(&page);
        tab
    }

    /// Window background transparent as soon as it contains a terminal with opacity below 100 %
    /// – otherwise only the window color would shine through instead of the desktop.
    fn update_translucency(&self) {
        let any = self.tabs().iter().any(Tab::is_translucent);
        if any != self.has_css_class("parkett-translucent") {
            if any {
                self.add_css_class("parkett-translucent");
            } else {
                self.remove_css_class("parkett-translucent");
            }
        }
    }

    fn update_page(&self, tab: &Tab) {
        crate::session::schedule_save();
        self.update_translucency();
        let view = self.tab_view();
        let Some(page) = self.page_of(tab) else { return };
        page.set_title(&self.page_label(tab, &page));
        // full shell title and directory when the label is short
        let cwd = tab.cwd().map(|c| tilde(&c)).unwrap_or_default();
        let mut tooltip = if crate::settings::short_tab_labels() { format!("{}\n{cwd}", tab.title()) } else { cwd };
        if let Some(name) = tab.focused_pane().terminal().automatic_profile() {
            tooltip.push('\n');
            tooltip.push_str(&trf("Appearance: profile “{name}” (automatic)", &[("name", &name)]));
        }
        page.set_tooltip(&glib::markup_escape_text(tooltip.trim()));
        page.set_keyword(&tab.cwd().unwrap_or_default());
        if tab.take_attention() && view.selected_page().as_ref() != Some(&page) {
            page.set_needs_attention(true);
        }
        if tab.has_failed_pane() {
            page.set_indicator_icon(Some(&gio::ThemedIcon::new("dialog-warning-symbolic")));
            page.set_indicator_tooltip(&gettext("A process exited with an error"));
            page.set_indicator_activatable(false);
            if view.selected_page().as_ref() != Some(&page) {
                page.set_needs_attention(true);
            }
        } else if tab.sync() {
            // shared input: icon on the tab, clicking it turns it off
            page.set_indicator_icon(Some(&gio::ThemedIcon::new("input-keyboard-symbolic")));
            page.set_indicator_tooltip(&gettext("Shared input on – click to turn it off"));
            page.set_indicator_activatable(true);
        } else {
            page.set_indicator_icon(gio::Icon::NONE);
        }
        if view.selected_page().as_ref() == Some(&page) {
            self.update_title();
        }
    }

    /// Label in the tab bar: short "1 htop · parkett" (QA-17) or the shell's title.
    fn page_label(&self, tab: &Tab, page: &adw::TabPage) -> String {
        if crate::settings::short_tab_labels() {
            tab.short_label(Some(self.tab_view().page_position(page) as usize + 1))
        } else {
            tab.title()
        }
    }

    /// Refresh all labels (order or setting changed).
    fn update_labels(&self) {
        for tab in self.tabs() {
            if let Some(page) = self.page_of(&tab) {
                page.set_title(&self.page_label(&tab, &page));
            }
        }
    }

    fn update_title(&self) {
        let title = self.imp().title.get().unwrap();
        // root in the focused pane: header bar red (approach C, setting)
        let root = crate::settings::context_headerbar()
            && self
                .current_tab()
                .and_then(|t| t.focused_pane().terminal().context_mark())
                .is_some_and(|m| m.kind == parkett_core::context::Kind::Superuser);
        if root != self.has_css_class("parkett-root") {
            if root {
                self.add_css_class("parkett-root");
            } else {
                self.remove_css_class("parkett-root");
            }
        }
        match self.current_tab() {
            Some(tab) => {
                // template from the preferences; {title} is the tab title (custom name or
                // the shell's)
                let position = self.page_of(&tab).map(|p| self.tab_view().page_position(&p) as usize + 1);
                let t = tab.render_label(&crate::settings::window_title_template(), position, Some(tab.title()));
                title.set_title(&t);
                if self.imp().layout_mode.get() {
                    title.set_subtitle(&gettext("Layout mode · Esc to exit"));
                } else {
                    title.set_subtitle(&tab.cwd().map(|c| tilde(&c)).unwrap_or_default());
                }
                self.set_title(Some(&t));
            }
            None => title.set_title("Parkett"),
        }
    }

    /// Remember the window size for the next window (only when not maximized).
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

    /// Align the shared input switches with the selected tab and pane,
    /// then its tab icon.
    fn update_sync_actions(&self) {
        self.sync_action_states();
        if let Some(tab) = self.current_tab() {
            self.update_page(&tab);
        }
    }

    fn sync_action_states(&self) {
        let tab = self.current_tab();
        let on = tab.as_ref().is_some_and(Tab::sync);
        if let Some(a) = self.lookup_action("toggle-sync").and_downcast::<gio::SimpleAction>() {
            a.set_state(&on.to_variant());
        }
        if let Some(a) = self.lookup_action("sync-pane").and_downcast::<gio::SimpleAction>() {
            let included = tab.as_ref().is_some_and(|t| !t.focused_pane().terminal().sync_excluded());
            a.set_enabled(on);
            a.set_state(&included.to_variant());
        }
    }

    /// Drag & drop of a pane (content `key`, see pane.rs) onto the tab `target`.
    fn move_pane_to(&self, key: &str, target: &Tab) -> bool {
        let Some((from, id)) = crate::pane::parse_drag_key(key) else { return false };
        let Some(app) = self.application() else { return false };
        let source = app
            .windows()
            .into_iter()
            .filter_map(|w| w.downcast::<Window>().ok())
            .flat_map(|w| w.tabs())
            .find(|t| crate::pane::tab_key(t) == from);
        let Some(source) = source else { return false };
        if &source == target {
            return false;
        }
        let Some(pane) = source.take_pane(id) else { return false };
        if !target.adopt_pane(&pane) {
            // no room: back into the old tab (which takes it as long as it was not empty)
            if !source.adopt_pane(&pane) {
                pane.terminal().terminate();
            }
            self.toast(&gettext("Not enough room in the tab"));
            return false;
        }
        self.select(target);
        true
    }

    pub fn toast(&self, text: &str) {
        self.imp().toasts.get().unwrap().add_toast(plain_toast(text));
    }

    /// Confirmation when programs are still running. `on_result(true)` on "Close", otherwise `false`
    /// (also Esc). Only evaluate `response`: `closed` can come before it (seen in the VM).
    fn confirm_close(&self, heading: &str, procs: &[String], on_result: impl Fn(bool) + 'static) {
        let mut names: Vec<&str> = procs.iter().map(String::as_str).collect();
        names.sort();
        names.dedup();
        let body = trf("Still running: {processes}. Closing ends these programs.", &[("processes", &names.join(", "))]);
        let dialog = adw::AlertDialog::new(Some(heading), Some(&body));
        dialog.add_responses(&[("cancel", gettext("Cancel").as_str()), ("close", gettext("Close").as_str())]);
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
            Some(&gettext("Rename tab")),
            Some(&gettext("Leave empty to use the terminal’s title.")),
        );
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", gettext("Cancel").as_str()), ("rename", gettext("Rename").as_str())]);
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("rename"));
        dialog.set_close_response("cancel");
        let e2 = entry.clone();
        dialog.connect_response(Some("rename"), move |_, _| tab.set_custom_title(Some(e2.text().to_string())));
        dialog.present(Some(self));
        // AdwAlertDialog otherwise focuses the default response, typing would go nowhere
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
        // Keys in layout mode (capture phase, before terminal and shortcuts)
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let weak = self.downgrade();
        keys.connect_key_pressed(move |_, key, _, state| match weak.upgrade() {
            // Only when a terminal has the focus – dialogs (confirmation, palette) get their keys
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
        // Command palette, Ctrl+Enter on a profile: new pane instead of new tab
        let a = gio::SimpleAction::new("split-profile", Some(glib::VariantTy::STRING));
        let weak = self.downgrade();
        a.connect_activate(move |_, param| {
            let (Some(w), Some(id)) = (weak.upgrade(), param.and_then(|p| p.get::<String>())) else { return };
            match w.current_tab() {
                Some(tab) if !tab.split_with(None, Some(&id), None, Default::default()) => {
                    w.toast(&gettext("Not enough room to split"))
                }
                Some(_) => {}
                None => {
                    w.add_tab_with_profile(None, &id);
                }
            }
        });
        self.add_action(&a);
        // Bookmarks: folder as new tab, new pane or `cd` in the active pane
        let a = gio::SimpleAction::new("open-bookmark", Some(glib::VariantTy::STRING));
        let weak = self.downgrade();
        a.connect_activate(move |_, param| {
            let (Some(w), Some(dir)) = (weak.upgrade(), param.and_then(|p| p.get::<String>())) else { return };
            let profile = ProfileManager::get().default_id();
            let tab = w.add_tab_with_profile(Some(dir), &profile);
            w.select(&tab);
        });
        self.add_action(&a);
        let a = gio::SimpleAction::new("bookmark-pane", Some(glib::VariantTy::STRING));
        let weak = self.downgrade();
        a.connect_activate(move |_, param| {
            let (Some(w), Some(dir)) = (weak.upgrade(), param.and_then(|p| p.get::<String>())) else { return };
            if let Some(tab) = w.current_tab() {
                if !tab.split_with(None, None, Some(dir), Default::default()) {
                    w.toast(&gettext("Not enough room to split"));
                }
            }
        });
        self.add_action(&a);
        let a = gio::SimpleAction::new("bookmark-cd", Some(glib::VariantTy::STRING));
        let weak = self.downgrade();
        a.connect_activate(move |_, param| {
            let (Some(w), Some(dir)) = (weak.upgrade(), param.and_then(|p| p.get::<String>())) else { return };
            if let Some(tab) = w.current_tab() {
                let quoted = glib::shell_quote(&dir).to_string_lossy().into_owned();
                tab.focused_pane().terminal().type_command(&format!("cd {quoted}"));
            }
        });
        self.add_action(&a);
        // Command or SSH target: (place, folder, command); place "tab", "pane" or "here". Typed
        // instead of started as a command – after `exit` the shell stays (like Tilix).
        let a = gio::SimpleAction::new("bookmark-command", Some(glib::VariantTy::new("(sss)").unwrap()));
        let weak = self.downgrade();
        a.connect_activate(move |_, param| {
            let (Some(w), Some((place, dir, line))) =
                (weak.upgrade(), param.and_then(|p| p.get::<(String, String, String)>()))
            else {
                return;
            };
            let dir = Some(dir).filter(|d| !d.is_empty());
            let terminal = match place.as_str() {
                "tab" => {
                    let tab = w.add_tab_with_profile(dir, &ProfileManager::get().default_id());
                    w.select(&tab);
                    Some(tab.focused_pane().terminal().clone())
                }
                "pane" => match w.current_tab() {
                    Some(tab) if tab.split_with(None, None, dir, Default::default()) => {
                        Some(tab.focused_pane().terminal().clone())
                    }
                    Some(_) => {
                        w.toast(&gettext("Not enough room to split"));
                        None
                    }
                    None => None,
                },
                _ => w.current_tab().map(|t| t.focused_pane().terminal().clone()),
            };
            if let Some(t) = terminal {
                t.type_command(&line);
            }
        });
        self.add_action(&a);
        action("add-bookmark", |w| {
            let Some(dir) = w.current_tab().and_then(|t| t.focused_pane().terminal().working_directory()) else {
                w.toast(&gettext("No folder known"));
                return;
            };
            match crate::bookmarks::add(&dir) {
                Ok(true) => w.toast(&trf("Bookmark “{name}” added", &[("name", &tilde(&dir))])),
                Ok(false) => w.toast(&gettext("This folder is already a bookmark")),
                Err(e) => w.toast(&trf("Bookmark not saved: {error}", &[("error", &e.to_string())])),
            }
        });
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
                w.toast(&gettext("Not enough room to split"));
            }
        }
        on_tab("split-right", |w, t| split(w, t, Some(Orientation::Horizontal)));
        on_tab("split-down", |w, t| split(w, t, Some(Orientation::Vertical)));
        on_tab("split-auto", |w, t| split(w, t, None));
        on_tab("toggle-zoom", |_, t| t.toggle_zoom());
        // Shared input (Tilix "Synchronize input"): stateful toggle so that menus
        // show the check mark; the state follows the selected tab (update_sync_actions)
        let a = gio::SimpleAction::new_stateful("toggle-sync", None, &false.to_variant());
        let weak = self.downgrade();
        a.connect_activate(move |_, _| {
            let Some(w) = weak.upgrade() else { return };
            let Some(tab) = w.current_tab() else { return };
            tab.set_sync(!tab.sync());
            w.update_sync_actions();
            let n = tab.sync_count();
            w.toast(&if tab.sync() {
                ntrf("Input goes to {n} pane", "Input goes to {n} panes", n as u32, &[])
            } else {
                gettext("Shared input off")
            });
        });
        self.add_action(&a);
        let a = gio::SimpleAction::new_stateful("sync-pane", None, &true.to_variant());
        a.set_enabled(false);
        let weak = self.downgrade();
        a.connect_activate(move |_, _| {
            let Some(w) = weak.upgrade() else { return };
            let Some(tab) = w.current_tab() else { return };
            let t = tab.focused_pane().terminal().clone();
            t.set_sync_excluded(!t.sync_excluded());
            tab.update_states();
            w.update_sync_actions();
        });
        self.add_action(&a);
        on_tab("equalize", |_, t| t.equalize());
        on_tab("close-pane", |w, t| {
            let pane = t.focused_pane();
            let id = pane.id();
            let fg = pane.terminal().foreground_process().filter(|_| crate::settings::confirm_close());
            match fg {
                None => t.close_pane(id),
                Some(p) => {
                    let tab = t.clone();
                    w.confirm_close(&gettext("Close pane?"), &[p], move |ok| {
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

        // Tab n: Ctrl+Alt+n, pane n: Alt+n (Tilix assignment)
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
        // Customizable shortcuts (Preferences › Shortcuts) in their own controller, rebuilt live.
        // Capture phase, so that e.g. Alt+Arrow does not end up in the shell.
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

    /// Show the tab and pane with this terminal. `false` if there is none here.
    pub fn show_terminal(&self, uid: u64) -> bool {
        for tab in self.tabs() {
            if let Some(id) = tab.pane_with_terminal(uid) {
                let view = self.tab_view();
                view.set_selected_page(&view.page(&tab));
                tab.focus_pane(id);
                self.present();
                return true;
            }
        }
        false
    }

    pub fn select(&self, tab: &Tab) {
        let view = self.tab_view();
        view.set_selected_page(&view.page(tab));
    }

    pub fn select_tab(&self, index: usize) {
        let view = self.tab_view();
        if (index as i32) < view.n_pages() {
            view.set_selected_page(&view.nth_page(index as i32));
        }
    }

    /// Runs an action from the shortcut list; `term.*` in the focused terminal.
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
            // all, not just the current one: on a tab switch the old one is already no longer current
            for tab in self.tabs() {
                tab.set_layout_mode(false);
            }
        }
        self.update_title();
    }

    fn terminal_has_focus(&self) -> bool {
        GtkWindowExt::focus(self).is_some_and(|f| f.ancestor(vte::Terminal::static_type()).is_some())
    }

    /// Keys in layout mode. Everything else is swallowed so that nothing ends up in the shell.
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
                    self.toast(&gettext("Not enough room to split"));
                }
            }
            Key::d => {
                if !tab.split(Some(Orientation::Vertical)) {
                    self.toast(&gettext("Not enough room to split"));
                }
            }
            Key::a => {
                if !tab.split(None) {
                    self.toast(&gettext("Not enough room to split"));
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
        // Refresh numbers and state after rebuilds (new pane, closing …)
        if let Some(t) = self.current_tab() {
            t.set_layout_mode(imp.layout_mode.get());
        }
        glib::Propagation::Stop
    }

    /// Dialog "Open template": choose the directory, show startup commands and turn them on/off.
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
            Some(&trf("Open template “{name}”", &[("name", &t.name)])),
            Some(&ntrf("Opens a new tab with {n} pane.", "Opens a new tab with {n} panes.", n as u32, &[])),
        );
        let list = gtk::ListBox::builder().css_classes(["boxed-list"]).selection_mode(gtk::SelectionMode::None).build();
        let dir_row = adw::ActionRow::builder()
            .title(gettext("Directory"))
            .subtitle(glib::markup_escape_text(&tilde(&dir.borrow().to_string_lossy())))
            .build();
        let pick = gtk::Button::builder()
            .icon_name("folder-open-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text(gettext("Choose directory"))
            .css_classes(["flat"])
            .build();
        dir_row.add_suffix(&pick);
        list.append(&dir_row);
        let commands = t.commands();
        let run = adw::SwitchRow::builder()
            .title(gettext("Run start commands"))
            // Subtitles are Pango markup: commands with "&&" or "<" would otherwise stay invisible (QA-07)
            .subtitle(if commands.is_empty() {
                gettext("No start commands")
            } else {
                glib::markup_escape_text(&commands.join(" · ")).to_string()
            })
            .active(!commands.is_empty())
            .sensitive(!commands.is_empty())
            .build();
        list.append(&run);
        dialog.set_extra_child(Some(&list));
        dialog.add_responses(&[("cancel", gettext("Cancel").as_str()), ("open", gettext("Open").as_str())]);
        dialog.set_response_appearance("open", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("open"));
        dialog.set_close_response("cancel");
        let d2 = dir.clone();
        let row2 = dir_row.clone();
        let weak = self.downgrade();
        pick.connect_clicked(move |_| {
            let Some(w) = weak.upgrade() else { return };
            let chooser = gtk::FileDialog::builder()
                .title(gettext("Directory for the template"))
                .initial_folder(&gtk::gio::File::for_path(&*d2.borrow()))
                .modal(true)
                .build();
            let d3 = d2.clone();
            let row3 = row2.clone();
            chooser.select_folder(Some(&w), gtk::gio::Cancellable::NONE, move |res| {
                if let Some(path) = res.ok().and_then(|f| f.path()) {
                    row3.set_subtitle(&glib::markup_escape_text(&tilde(&path.to_string_lossy())));
                    *d3.borrow_mut() = path;
                }
            });
        });
        let weak = self.downgrade();
        dialog.connect_response(Some("open"), move |_, _| {
            let Some(w) = weak.upgrade() else { return };
            let dir = dir.borrow().clone();
            if !dir.is_dir() {
                w.toast(&trf("Directory “{path}” does not exist", &[("path", &tilde(&dir.to_string_lossy()))]));
                return;
            }
            w.attach_tab(Tab::from_template(&t, &dir, run.is_active()));
        });
        dialog.present(Some(self));
    }

    /// Dialog "Save tab as template".
    fn save_template(&self) {
        let Some(tab) = self.current_tab() else { return };
        let suggestion = tab.custom_title().unwrap_or_else(|| {
            tab.cwd().and_then(|c| c.rsplit('/').next().map(String::from)).unwrap_or_else(|| gettext("Template"))
        });
        let (preview, _) = tab.to_template(&suggestion);
        let n = preview.layout.panes().len();
        let commands = preview.commands();
        let body = if commands.is_empty() {
            ntrf(
                "Saves the layout and profiles of the {n} pane. The directory is asked for when opening.",
                "Saves the layout and profiles of the {n} panes. The directory is asked for when opening.",
                n as u32,
                &[],
            )
        } else {
            ntrf("Saves the layout and profiles of the {n} pane and the running commands ({commands}). The directory is asked for when opening.", "Saves the layout and profiles of the {n} panes and the running commands ({commands}). The directory is asked for when opening.", n as u32, &[("commands", &commands.join(", "))])
        };
        let entry = gtk::Entry::builder().text(&suggestion).activates_default(true).build();
        let dialog = adw::AlertDialog::new(Some(&gettext("Save tab as template")), Some(&body));
        dialog.set_extra_child(Some(&entry));
        dialog.add_responses(&[("cancel", gettext("Cancel").as_str()), ("save", gettext("Save").as_str())]);
        dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("save"));
        dialog.set_close_response("cancel");
        let weak = self.downgrade();
        let e2 = entry.clone();
        dialog.connect_response(Some("save"), move |_, _| {
            let Some(w) = weak.upgrade() else { return };
            let name = e2.text().trim().to_string();
            let name = if name.is_empty() { gettext("Template") } else { name };
            let (t, _) = tab.to_template(&name);
            match ProfileManager::get().save_template(&t) {
                Ok(t) => w.toast(&trf("Template “{name}” saved", &[("name", &t.name)])),
                Err(e) => w.toast(&trf("Template not saved: {error}", &[("error", &e.to_string())])),
            }
        });
        dialog.present(Some(self));
        entry.grab_focus();
    }

    /// Window shortcuts off/on, e.g. while the preferences dialog records a shortcut.
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
        // AdwSplitButton passes its tooltip on to the inner button; so set it here
        // instead of when shown (the other buttons: `shortcuts::live_tooltip`)
        if let Some(button) = imp.new_button.get() {
            button.set_tooltip_text(Some(&crate::shortcuts::tooltip(&gettext("New tab"), "win.new-tab", &[])));
        }
    }
}

fn fill_new_menu(menu: &gio::Menu) {
    menu.remove_all();
    let manager = ProfileManager::get();
    let default = manager.default_id();
    let profiles = gio::Menu::new();
    // default profile first, then alphabetical
    let mut list = manager.profiles();
    list.sort_by_key(|p| p.id != default);
    for p in list {
        let item = gio::MenuItem::new(Some(&p.name), None);
        item.set_action_and_target_value(Some("win.new-tab-profile"), Some(&p.id.to_variant()));
        profiles.append_item(&item);
    }
    menu.append_section(Some(&gettext("New tab with profile")), &profiles);
    let templates = gio::Menu::new();
    fill_template_menu(&templates);
    menu.append_section(Some(&gettext("Templates")), &templates);
    // Split here too: in narrow windows the buttons are missing from the header bar (QA-13)
    let split = gio::Menu::new();
    split.append(Some(&gettext("Split right")), Some("win.split-right"));
    split.append(Some(&gettext("Split down")), Some("win.split-down"));
    menu.append_section(None, &split);
    let more = gio::Menu::new();
    more.append(Some(&gettext("New window")), Some("app.new-window"));
    more.append(Some(&gettext("Edit profiles…")), Some("app.preferences"));
    menu.append_section(None, &more);
}

/// Bookmarks as new tabs, then "Bookmark current folder".
fn fill_bookmark_menu(menu: &gio::Menu) {
    menu.remove_all();
    for b in crate::bookmarks::load() {
        let item = gio::MenuItem::new(Some(&b.name), None);
        let (action, target) = crate::bookmarks::action(&b, "tab");
        item.set_action_and_target_value(Some(action), Some(&target));
        menu.append_item(&item);
    }
    menu.append(Some(&gettext("Bookmark current folder")), Some("win.add-bookmark"));
}

/// Templates to open, then "Save current tab as template…".
fn fill_template_menu(menu: &gio::Menu) {
    menu.remove_all();
    for t in ProfileManager::get().templates() {
        let item = gio::MenuItem::new(Some(&t.name), None);
        item.set_action_and_target_value(Some("win.open-template"), Some(&t.id.to_variant()));
        menu.append_item(&item);
    }
    menu.append(Some(&gettext("Save current tab as template…")), Some("win.save-template"));
}

fn shortcut(accel: &str, action: &str) -> gtk::Shortcut {
    gtk::Shortcut::new(gtk::ShortcutTrigger::parse_string(accel), Some(gtk::NamedAction::new(action)))
}

fn tilde(path: &str) -> String {
    parkett_core::paths::tilde(path, &glib::home_dir())
}

/// Toast with plain text: toast titles are otherwise Pango markup, and names, paths or
/// error messages with "&" or "<" would get lost (QA-07).
pub fn plain_toast(text: &str) -> adw::Toast {
    adw::Toast::builder().title(text).use_markup(false).build()
}
