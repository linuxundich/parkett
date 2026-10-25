// SPDX-License-Identifier: GPL-3.0-or-later
//! Preferences dialog with the pages in a sidebar on the left (like GNOME Settings) instead of
//! the view switcher of AdwPreferencesDialog, which has no sidebar mode: AdwViewSwitcherSidebar
//! (libadwaita 1.9) over an AdwViewStack. Same interface as AdwPreferencesDialog for the pages
//! (`add`, `push_subpage`, `pop_subpage`, `add_toast`), plus a search over the rows of all pages.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::i18n::gettext;

/// Below this width the sidebar overlays the content and opens with a button.
const NARROW: f64 = 620.0;

mod imp {
    use std::cell::{OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct PrefsDialog {
        pub toasts: OnceCell<adw::ToastOverlay>,
        pub split: OnceCell<adw::OverlaySplitView>,
        /// content side: root page with the pages, subpages (profile editor, …) on top
        pub nav: OnceCell<adw::NavigationView>,
        pub root: OnceCell<adw::NavigationPage>,
        /// the pages, switched by the sidebar
        pub view: OnceCell<adw::ViewStack>,
        /// pages or search results
        pub content: OnceCell<gtk::Stack>,
        pub search_bar: OnceCell<gtk::SearchBar>,
        pub results: OnceCell<adw::PreferencesGroup>,
        pub pages: RefCell<Vec<adw::PreferencesPage>>,
        /// rows of the result list, removed on the next search
        pub result_rows: RefCell<Vec<adw::ActionRow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PrefsDialog {
        const NAME: &'static str = "ParkettPrefsDialog";
        type Type = super::PrefsDialog;
        type ParentType = adw::Dialog;
    }

    impl ObjectImpl for PrefsDialog {}
    impl WidgetImpl for PrefsDialog {}
    impl AdwDialogImpl for PrefsDialog {}
}

glib::wrapper! {
    pub struct PrefsDialog(ObjectSubclass<imp::PrefsDialog>)
        @extends adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PrefsDialog {
    pub fn new(title: &str) -> Self {
        let dialog: PrefsDialog = glib::Object::builder()
            .property("title", title)
            .property("content-width", 860)
            .property("content-height", 640)
            .build();
        dialog.build_ui(title);
        dialog
    }

    fn build_ui(&self, title: &str) {
        let imp = self.imp();
        let view = adw::ViewStack::new();

        // Sidebar: header with search, the pages of the view stack
        let search_button = gtk::ToggleButton::builder()
            .icon_name("system-search-symbolic")
            .tooltip_text(gettext("Search preferences"))
            .build();
        let sidebar_header = adw::HeaderBar::builder()
            .title_widget(&adw::WindowTitle::new(title, ""))
            .show_end_title_buttons(false)
            .build();
        sidebar_header.pack_start(&search_button);
        let entry = gtk::SearchEntry::builder().placeholder_text(gettext("Search preferences")).hexpand(true).build();
        let search_bar = gtk::SearchBar::builder().child(&entry).show_close_button(false).build();
        search_bar.connect_entry(&entry);
        search_button.bind_property("active", &search_bar, "search-mode-enabled").bidirectional().build();
        let switcher = adw::ViewSwitcherSidebar::builder().stack(&view).build();
        let sidebar = adw::ToolbarView::new();
        sidebar.add_top_bar(&sidebar_header);
        sidebar.add_top_bar(&search_bar);
        sidebar.set_content(Some(&switcher));

        // Content: the pages or the search results under a header bar; the button opens the
        // sidebar when narrow
        let results = adw::PreferencesGroup::new();
        let results_page = adw::PreferencesPage::new();
        results_page.add(&results);
        let content = gtk::Stack::builder().transition_type(gtk::StackTransitionType::Crossfade).build();
        content.add_named(&view, Some("pages"));
        content.add_named(&results_page, Some("search"));
        let sidebar_button = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .tooltip_text(gettext("Show pages"))
            .visible(false)
            .build();
        let content_header = adw::HeaderBar::new();
        content_header.pack_start(&sidebar_button);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&content_header);
        toolbar.set_content(Some(&content));
        let root = adw::NavigationPage::builder().title(title).child(&toolbar).build();
        let nav = adw::NavigationView::new();
        nav.add(&root);

        let split = adw::OverlaySplitView::builder()
            .sidebar(&sidebar)
            .content(&nav)
            .min_sidebar_width(200.0)
            .max_sidebar_width(240.0)
            .build();
        split.bind_property("show-sidebar", &sidebar_button, "active").bidirectional().sync_create().build();
        let toasts = adw::ToastOverlay::new();
        toasts.set_child(Some(&split));
        self.set_child(Some(&toasts));

        // narrow: sidebar overlays and is hidden until the button opens it
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            NARROW,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        breakpoint.add_setter(&sidebar_button, "visible", Some(&true.to_value()));
        self.add_breakpoint(breakpoint);

        // A page chosen in the sidebar ends the search and closes open subpages
        let weak = self.downgrade();
        view.connect_visible_child_notify(move |_| {
            if let Some(d) = weak.upgrade() {
                d.imp().search_bar.get().unwrap().set_search_mode(false);
                d.page_shown();
            }
        });
        let weak = self.downgrade();
        entry.connect_search_changed(move |e| {
            if let Some(d) = weak.upgrade() {
                d.search(e.text().as_str());
            }
        });
        let weak = self.downgrade();
        search_bar.connect_search_mode_enabled_notify(move |bar| {
            if let Some(d) = weak.upgrade().filter(|_| !bar.is_search_mode()) {
                d.search("");
            }
        });
        // Ctrl+F as in AdwPreferencesDialog (no key capture: typing belongs to the entry rows)
        let keys = gtk::ShortcutController::new();
        let sb = search_bar.downgrade();
        keys.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>f"),
            Some(gtk::CallbackAction::new(move |_, _| {
                if let Some(sb) = sb.upgrade() {
                    sb.set_search_mode(true);
                }
                glib::Propagation::Stop
            })),
        ));
        self.add_controller(keys);

        imp.toasts.set(toasts).ok();
        imp.split.set(split).ok();
        imp.nav.set(nav).ok();
        imp.root.set(root).ok();
        imp.view.set(view).ok();
        imp.content.set(content).ok();
        imp.search_bar.set(search_bar).ok();
        imp.results.set(results).ok();
    }

    /// Adds a page: entry in the sidebar with its icon and title.
    pub fn add(&self, page: &adw::PreferencesPage) {
        let imp = self.imp();
        let index = imp.pages.borrow().len();
        imp.view.get().unwrap().add_titled_with_icon(
            page,
            Some(&format!("page-{index}")),
            &page.title(),
            page.icon_name().as_deref().unwrap_or(""),
        );
        imp.pages.borrow_mut().push(page.clone());
        if index == 0 {
            self.page_shown();
        }
    }

    /// The visible page changed (sidebar or search hit): pages instead of results, title, no
    /// subpage, overlay sidebar closed.
    fn page_shown(&self) {
        let imp = self.imp();
        let Some(page) = imp.view.get().unwrap().visible_child().and_downcast::<adw::PreferencesPage>() else {
            return;
        };
        imp.content.get().unwrap().set_visible_child_name("pages");
        imp.root.get().unwrap().set_title(&page.title());
        imp.nav.get().unwrap().pop_to_page(imp.root.get().unwrap());
        let split = imp.split.get().unwrap();
        if split.is_collapsed() {
            split.set_show_sidebar(false);
        }
    }

    /// Subpage over the content (profile and template editor), like AdwPreferencesDialog.
    pub fn push_subpage(&self, page: &impl IsA<adw::NavigationPage>) {
        self.imp().nav.get().unwrap().push(page);
    }

    pub fn pop_subpage(&self) -> bool {
        self.imp().nav.get().unwrap().pop()
    }

    pub fn add_toast(&self, toast: adw::Toast) {
        self.imp().toasts.get().unwrap().add_toast(toast);
    }

    /// Search over the titles and subtitles of all rows; a hit leads to its page and row.
    fn search(&self, text: &str) {
        let imp = self.imp();
        let results = imp.results.get().unwrap();
        for row in imp.result_rows.borrow_mut().drain(..) {
            results.remove(&row);
        }
        let query = text.trim().to_lowercase();
        if query.is_empty() {
            self.page_shown();
            return;
        }
        imp.nav.get().unwrap().pop_to_page(imp.root.get().unwrap());
        let pages = imp.pages.borrow().clone();
        let mut found = 0;
        for page in &pages {
            for (row, title, subtitle) in rows_of(page.upcast_ref()) {
                let hay = format!("{title}\n{subtitle}").to_lowercase();
                if !hay.contains(&query) {
                    continue;
                }
                found += 1;
                let hit = adw::ActionRow::builder()
                    .title(glib::markup_escape_text(&title))
                    .subtitle(glib::markup_escape_text(&page.title()))
                    .activatable(true)
                    .build();
                hit.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                let weak = self.downgrade();
                let (page, target) = (page.downgrade(), row.downgrade());
                hit.connect_activated(move |_| {
                    let (Some(d), Some(page), Some(target)) = (weak.upgrade(), page.upgrade(), target.upgrade()) else {
                        return;
                    };
                    d.imp().search_bar.get().unwrap().set_search_mode(false);
                    d.imp().view.get().unwrap().set_visible_child(&page);
                    d.page_shown();
                    // focus scrolls the page to the row
                    glib::idle_add_local_once(move || {
                        target.grab_focus();
                    });
                });
                results.add(&hit);
                imp.result_rows.borrow_mut().push(hit);
            }
        }
        results.set_title(&if found == 0 { gettext("No results") } else { String::new() });
        imp.content.get().unwrap().set_visible_child_name("search");
        imp.root.get().unwrap().set_title(&gettext("Search results"));
    }
}

/// Visible rows of a page with title and subtitle (only rows that are shown).
fn rows_of(widget: &gtk::Widget) -> Vec<(adw::PreferencesRow, String, String)> {
    let mut out = Vec::new();
    let mut child = widget.first_child();
    while let Some(c) = child {
        if c.is_visible() {
            if let Some(row) = c.downcast_ref::<adw::PreferencesRow>() {
                let subtitle = row
                    .downcast_ref::<adw::ActionRow>()
                    .and_then(|r| r.subtitle())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                out.push((row.clone(), row.title().to_string(), subtitle));
            } else {
                out.extend(rows_of(&c));
            }
        }
        child = c.next_sibling();
    }
    out
}
