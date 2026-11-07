// SPDX-License-Identifier: GPL-3.0-or-later
//! A tab: split layout from `parkett_core::layout` plus the pane widgets.
//!
//! The model is authoritative. After every change `rebuild()` rebuilds the GtkPaned tree
//! and only re-parents the existing panes – terminals and their processes are kept.

use std::collections::HashMap;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::subclass::Signal;
use parkett_core::layout::{Direction, Layout, Node, Orientation, PaneId, Side};

use crate::i18n::{gettext, ntrf, N_};
use crate::pane::Pane;
use crate::terminal::{Launch, MIN_COLUMNS, MIN_ROWS};

/// Output only with PARKETT_DEBUG_LAYOUT=1 (debugging the sync divider ⇄ model).
/// A macro, so that no text is formatted without the variable (D6).
macro_rules! debug {
    ($($arg:tt)*) => {
        if debug_enabled() {
            eprintln!("[layout] {}", format_args!($($arg)*));
        }
    };
}

/// Step size when moving a divider by keyboard (fraction of the area).
const RESIZE_STEP: f32 = 0.03;

mod imp {
    use std::cell::{OnceCell, RefCell};
    use std::sync::OnceLock;

    use super::*;

    #[derive(Default)]
    pub struct Tab {
        pub layout: RefCell<Layout>,
        pub panes: RefCell<HashMap<PaneId, Pane>>,
        pub custom_title: RefCell<Option<String>>,
        pub overlay: OnceCell<gtk::Overlay>,
        pub zoom_banner: OnceCell<gtk::Box>,
        pub zoom_label: OnceCell<gtk::Label>,
        pub mode_bar: OnceCell<adw::WrapBox>,
        /// Closed panes while "Undo" is offered: (number, pane,
        /// layout before, layout after)
        pub closed: RefCell<Vec<(u64, Pane, Layout, Layout)>>,
        pub closed_serial: std::cell::Cell<u64>,
        pub layout_mode: std::cell::Cell<bool>,
        /// shared input: typing goes to all panes that are not excluded
        pub sync: std::cell::Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Tab {
        const NAME: &'static str = "ParkettTab";
        type Type = super::Tab;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for Tab {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    // title, directory, focus or process state changed
                    Signal::builder("changed").build(),
                    // last pane was closed
                    Signal::builder("empty").build(),
                    // a pane was closed and can be restored (number)
                    Signal::builder("pane-closed").param_types([u64::static_type()]).build(),
                ]
            })
        }
    }
    impl WidgetImpl for Tab {}
    impl BinImpl for Tab {}
}

glib::wrapper! {
    pub struct Tab(ObjectSubclass<imp::Tab>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Tab {
    pub fn new(cwd: Option<String>, profile_id: &str) -> Self {
        Tab::with_launch(cwd, profile_id, Launch::default())
    }

    /// Tab with one pane according to `launch` (command, caller's environment).
    pub fn with_launch(cwd: Option<String>, profile_id: &str, launch: Launch) -> Self {
        let tab = Tab::empty();
        let first = tab.imp().layout.borrow().focus();
        tab.add_pane_with(first, cwd, profile_id, launch);
        tab.rebuild();
        tab
    }

    /// Tab from a template in the directory `dir`. With `run_commands` the
    /// startup commands are typed into the shells.
    pub fn from_template(
        template: &parkett_core::template::Template,
        dir: &std::path::Path,
        run_commands: bool,
    ) -> Self {
        let tab = Tab::empty();
        let (layout, specs) = template.instantiate();
        let default = crate::profiles::ProfileManager::get().default_id();
        for (id, spec) in specs {
            let path = parkett_core::paths::join_subdir(dir, &spec.subdir);
            let cwd = path.to_str().map(String::from);
            let profile = if spec.profile.is_empty() { default.clone() } else { spec.profile.clone() };
            let pane = tab.add_pane(id, cwd, &profile);
            if run_commands && !spec.command.trim().is_empty() {
                pane.terminal().type_command(&spec.command);
            }
        }
        tab.set_custom_title(Some(template.name.clone()));
        *tab.imp().layout.borrow_mut() = layout;
        tab.rebuild();
        tab
    }

    /// State for the session: split, profile and directory per pane, custom title.
    pub fn to_session(&self) -> parkett_core::template::Template {
        let panes = self.imp().panes.borrow().clone();
        let spec = |id: PaneId| {
            let t = panes[&id].terminal();
            parkett_core::session::pane(&t.profile_id(), t.working_directory().as_deref())
        };
        let mut t = parkett_core::template::Template::from_layout(
            &self.custom_title().unwrap_or_default(),
            &self.layout(),
            &spec,
        );
        t.directory = "/".into();
        t
    }

    /// Template from this tab: split, profiles, running commands; subdirectories
    /// relative to the directory of the first pane. Also returns that directory.
    pub fn to_template(&self, name: &str) -> (parkett_core::template::Template, Option<String>) {
        use parkett_core::template::{relative_subdir, PaneSpec, Template};
        let layout = self.layout();
        let panes = self.imp().panes.borrow().clone();
        let base = layout.panes().first().and_then(|id| panes[id].terminal().cwd());
        let default = crate::profiles::ProfileManager::get().default_id();
        let spec = |id: PaneId| {
            let t = panes[&id].terminal();
            let profile = t.profile_id();
            PaneSpec {
                name: t.foreground_process().unwrap_or_else(|| "Shell".into()),
                profile: if profile == default { String::new() } else { profile },
                command: t.foreground_command().unwrap_or_default(),
                subdir: match (&base, t.cwd()) {
                    (Some(b), Some(c)) => relative_subdir(std::path::Path::new(b), std::path::Path::new(&c)),
                    _ => String::new(),
                },
            }
        };
        (Template::from_layout(name, &layout, &spec), base)
    }

    fn empty() -> Self {
        let tab: Tab = glib::Object::new();
        let overlay = gtk::Overlay::new();

        let zoom_label = gtk::Label::new(None);
        let show_all = gtk::Button::builder().label(gettext("Show all")).action_name("win.toggle-zoom").build();
        let banner = gtk::Box::builder()
            .css_classes(["parkett-zoom-banner"])
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Start)
            .margin_top(8)
            .spacing(10)
            .visible(false)
            .build();
        banner.append(&gtk::Image::from_icon_name("parkett-pane-maximize-symbolic"));
        banner.append(&zoom_label);
        banner.append(&show_all);
        overlay.add_overlay(&banner);
        tab.set_child(Some(&overlay));

        let imp = tab.imp();
        imp.overlay.set(overlay).ok();
        imp.zoom_banner.set(banner).ok();
        imp.zoom_label.set(zoom_label).ok();

        // help bar of the layout mode (bottom center)
        let hints = [
            ("←↑↓→", N_("focus")),
            (N_("Shift+Arrow"), N_("size")),
            ("R/D", N_("split")),
            (N_("M+Arrow"), N_("swap")),
            ("Z", N_("maximize")),
            ("=", N_("equalize")),
            ("X", N_("close")),
            ("1–9", N_("jump")),
            ("Esc", N_("done")),
        ];
        // wraps in narrow windows instead of being cut off
        let bar = adw::WrapBox::builder()
            .css_classes(["parkett-mode-bar"])
            .halign(gtk::Align::Center)
            .valign(gtk::Align::End)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .child_spacing(14)
            .line_spacing(6)
            .align(0.5)
            .can_target(false)
            .visible(false)
            .build();
        bar.append(&gtk::Label::builder().label("LAYOUT").css_classes(["parkett-mode-tag"]).build());
        for (key, what) in hints {
            let item = gtk::Box::builder().spacing(5).build();
            item.append(&gtk::Label::builder().label(gettext(key)).css_classes(["parkett-key"]).build());
            item.append(&gtk::Label::new(Some(&gettext(what))));
            bar.append(&item);
        }
        tab.imp().overlay.get().unwrap().add_overlay(&bar);
        tab.imp().mode_bar.set(bar).ok();
        tab
    }

    /// Layout mode: numbers on the panes and help bar on/off.
    pub fn set_layout_mode(&self, on: bool) {
        let imp = self.imp();
        imp.layout_mode.set(on);
        imp.mode_bar.get().unwrap().set_visible(on);
        self.update_states();
    }

    /// Swap the focused pane with its neighbor in direction `dir`.
    pub fn swap_direction(&self, dir: Direction) {
        let layout = self.layout();
        let (w, h) = self.size();
        let focus = layout.focus();
        if let Some(n) = layout.neighbour(focus, dir, w, h) {
            if let Some(next) = layout.swap(focus, n) {
                self.set_layout(next);
            }
        }
    }

    pub fn connect_changed(&self, f: impl Fn(&Self) + 'static) {
        self.connect_local("changed", false, move |args| {
            f(&args[0].get::<Tab>().unwrap());
            None
        });
    }

    pub fn connect_pane_closed(&self, f: impl Fn(&Self, u64) + 'static) {
        self.connect_local("pane-closed", false, move |args| {
            f(&args[0].get::<Tab>().unwrap(), args[1].get::<u64>().unwrap());
            None
        });
    }

    pub fn connect_empty(&self, f: impl Fn(&Self) + 'static) {
        self.connect_local("empty", false, move |args| {
            f(&args[0].get::<Tab>().unwrap());
            None
        });
    }

    fn emit_changed(&self) {
        self.emit_by_name::<()>("changed", &[]);
    }

    fn add_pane(&self, id: PaneId, cwd: Option<String>, profile_id: &str) -> Pane {
        self.add_pane_with(id, cwd, profile_id, Launch::default())
    }

    fn add_pane_with(&self, id: PaneId, cwd: Option<String>, profile_id: &str, launch: Launch) -> Pane {
        let pane = Pane::new(id, cwd, profile_id, launch);
        pane.set_owner(self);
        // All handlers ask the pane for its tab when called: it can move to another one by drag &
        // drop.
        let weak_pane = pane.downgrade();
        let source = pane.terminal().downgrade();
        pane.terminal().connect_input(move |text| {
            if let (Some(tab), Some(source)) = (weak_pane.upgrade().and_then(|p| p.tab()), source.upgrade()) {
                tab.forward_input(&source, text);
            }
        });
        let weak_pane = pane.downgrade();
        pane.terminal().connect_state_changed(move || {
            if let Some(tab) = weak_pane.upgrade().and_then(|p| p.tab()) {
                tab.emit_changed();
            }
        });
        // "Close" in the bar after the process exited. Read the id only when called: undo
        // at a new position gives the pane a new one.
        let weak_pane = pane.downgrade();
        pane.terminal().connect_close(move || {
            if let Some(pane) = weak_pane.upgrade() {
                if let Some(tab) = pane.tab() {
                    tab.close_pane(pane.id());
                }
            }
        });
        // Take focus by click or Tab key into the model
        let weak_pane = pane.downgrade();
        pane.terminal().connect_focus_gained(move || {
            let Some(pane) = weak_pane.upgrade() else { return };
            let Some(tab) = pane.tab() else { return };
            let id = pane.id();
            let changed = {
                let mut layout = tab.imp().layout.borrow_mut();
                if layout.focus() == id {
                    false
                } else if let Some(l) = layout.with_focus(id) {
                    *layout = l;
                    true
                } else {
                    false
                }
            };
            if changed {
                tab.update_states();
                tab.emit_changed();
            }
        });
        self.imp().panes.borrow_mut().insert(id, pane.clone());
        pane
    }

    pub fn layout(&self) -> Layout {
        self.imp().layout.borrow().clone()
    }

    pub fn pane(&self, id: PaneId) -> Option<Pane> {
        self.imp().panes.borrow().get(&id).cloned()
    }

    pub fn layout_mode(&self) -> bool {
        self.imp().layout_mode.get()
    }

    pub fn focused_pane(&self) -> Pane {
        let id = self.imp().layout.borrow().focus();
        self.imp().panes.borrow()[&id].clone()
    }

    pub fn pane_count(&self) -> usize {
        self.imp().panes.borrow().len()
    }

    pub fn set_custom_title(&self, title: Option<String>) {
        *self.imp().custom_title.borrow_mut() = title.filter(|t| !t.trim().is_empty());
        self.emit_changed();
    }

    pub fn custom_title(&self) -> Option<String> {
        self.imp().custom_title.borrow().clone()
    }

    /// Tab title: custom name, otherwise title of the focused terminal; when zoomed "(1/3)".
    pub fn title(&self) -> String {
        let base =
            self.custom_title().or_else(|| self.focused_pane().terminal().title()).unwrap_or_else(|| "Shell".into());
        match self.layout().zoomed() {
            Some(_) => format!("{base} (1/{})", self.pane_count()),
            None => base,
        }
    }

    /// Short label "1 htop · parkett" for the tab bar (QA-17); a custom name takes
    /// precedence. `number`: position of the tab, 1-based.
    pub fn short_label(&self, number: Option<usize>) -> String {
        let zoom = self.layout().zoomed().map(|_| format!(" (1/{})", self.pane_count())).unwrap_or_default();
        if let Some(custom) = self.custom_title() {
            return match number.filter(|n| (1..=9).contains(n)) {
                Some(n) => format!("{n} {custom}{zoom}"),
                None => format!("{custom}{zoom}"),
            };
        }
        let own = crate::profiles::ProfileManager::get().profile(&self.focused_pane().terminal().profile_id());
        // the profile's template before the setting
        let template =
            if own.tab_label.trim().is_empty() { crate::settings::tab_label_template() } else { own.tab_label.clone() };
        format!("{}{zoom}", self.render_label(&template, number, None))
    }

    /// Text from a template (`core::label::render`) with the values of the active pane,
    /// for tab names and window titles. `title`: use instead of the shell title.
    pub fn render_label(&self, template: &str, number: Option<usize>, title: Option<String>) -> String {
        let pane = self.focused_pane();
        let t = pane.terminal();
        // In an SSH session the local directory (that of ssh) is meaningless
        let (dir, host) = match t.context().remote_host() {
            Some(host) => (None, Some(host.to_string())),
            None => (t.working_directory(), None),
        };
        let program = t.label_program();
        let title = title.or_else(|| t.title());
        let profile = crate::profiles::ProfileManager::get().profile(&t.profile_id()).name;
        let vars = parkett_core::label::Vars {
            number,
            program: program.as_deref(),
            dir: dir.as_deref(),
            host: host.as_deref(),
            title: title.as_deref(),
            profile: Some(&profile),
        };
        parkett_core::label::render(template, &vars, &glib::home_dir())
    }

    pub fn cwd(&self) -> Option<String> {
        self.focused_pane().terminal().cwd()
    }

    /// Start directory for a new terminal according to the setting "new-terminal-directory".
    pub fn inherited_cwd(&self) -> Option<String> {
        if crate::settings::inherit_directory() {
            // without local OSC 7 (e.g. SSH session) the directory of the shell process
            self.focused_pane().terminal().working_directory()
        } else {
            None
        }
    }

    /// Bell or command end in a pane since the last call (clears the mark).
    pub fn take_attention(&self) -> bool {
        // reset all panes, do not stop at the first match
        let mut any = false;
        for p in self.imp().panes.borrow().values() {
            any |= p.terminal().take_attention();
        }
        any
    }

    pub fn is_translucent(&self) -> bool {
        self.imp().panes.borrow().values().any(|p| p.terminal().is_translucent())
    }

    pub fn has_failed_pane(&self) -> bool {
        use crate::terminal::ExitStatus;
        self.imp().panes.borrow().values().any(|p| {
            matches!(p.terminal().exit_status(), Some(ExitStatus::Code(c)) if c != 0)
                || matches!(p.terminal().exit_status(), Some(ExitStatus::Signal(_)))
        })
    }

    /// Running foreground processes of all panes (for the confirmation on close).
    pub fn foreground_processes(&self) -> Vec<String> {
        if !crate::settings::confirm_close() {
            return Vec::new();
        }
        self.imp().panes.borrow().values().filter_map(|p| p.terminal().foreground_process()).collect()
    }

    pub fn focus_terminal(&self) {
        self.focused_pane().focus_terminal();
    }

    fn size(&self) -> (f32, f32) {
        let (w, h) = (self.width() as f32, self.height() as f32);
        if w > 0.0 && h > 0.0 {
            (w, h)
        } else {
            (1000.0, 600.0)
        }
    }

    fn min_pane_size(&self) -> (f32, f32) {
        let (w, h) = self.focused_pane().terminal().char_size();
        ((w * MIN_COLUMNS) as f32, (h * MIN_ROWS) as f32)
    }

    /// Drag & drop of a pane (see `Layout::move_pane`); `false` if nothing happens.
    pub fn move_pane(&self, source: PaneId, target: PaneId, zone: parkett_core::layout::DropZone) -> bool {
        let Some(next) = self.layout().move_pane(source, target, zone) else { return false };
        self.set_layout(next);
        if let Some(p) = self.pane(source) {
            p.focus_terminal();
        }
        crate::session::schedule_save();
        true
    }

    fn set_layout(&self, layout: Layout) {
        *self.imp().layout.borrow_mut() = layout;
        self.rebuild();
        self.emit_changed();
    }

    /// Splits the focused pane; `None` = along the longer side.
    /// `false` if one part would become smaller than the minimum size.
    pub fn split(&self, orientation: Option<Orientation>) -> bool {
        self.split_with(orientation, None, None, Launch::default())
    }

    /// Splits the active pane; the new one gets `profile` (otherwise that of the active one) and
    /// starts in `cwd` (otherwise according to the setting "Directory for new terminals") with `launch`.
    pub fn split_with(
        &self,
        orientation: Option<Orientation>,
        profile: Option<&str>,
        cwd: Option<String>,
        launch: Launch,
    ) -> bool {
        let layout = self.layout();
        let focus = layout.focus();
        let (w, h) = self.size();
        let (min_w, min_h) = self.min_pane_size();
        let o = orientation.unwrap_or_else(|| {
            let r = layout.rect_of(focus, w, h).unwrap();
            if r.w / 2.0 >= r.h {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            }
        });
        // When zoomed the full area counts – splitting cancels the zoom anyway
        if layout.zoomed().is_none() && !layout.can_split(focus, o, w, h, min_w, min_h) {
            return false;
        }
        let cwd = cwd.or_else(|| self.inherited_cwd());
        // new pane inherits the profile of the active one
        let profile = profile.map(String::from).unwrap_or_else(|| self.focused_pane().terminal().profile_id());
        let Some((next, id)) = layout.split(focus, o) else { return false };
        self.add_pane_with(id, cwd, &profile, launch);
        self.set_layout(next);
        true
    }

    /// Closes a pane. It is kept together with its process until `discard_closed` comes
    /// (proposal 4: undo). For the last pane the tab reports "empty".
    pub fn close_pane(&self, id: PaneId) {
        let before = self.layout();
        // Unknown id (pane already closed, confirmation outdated): do nothing – otherwise
        // `close` would take it for the last pane and the whole tab would close.
        if !before.contains(id) {
            return;
        }
        match before.close(id) {
            Some(next) => {
                let pane = self.imp().panes.borrow_mut().remove(&id);
                if let Some(pane) = pane {
                    detach(pane.upcast_ref());
                    let serial = self.imp().closed_serial.get() + 1;
                    self.imp().closed_serial.set(serial);
                    self.imp().closed.borrow_mut().push((serial, pane, before, next.clone()));
                    self.set_layout(next);
                    self.emit_by_name::<()>("pane-closed", &[&serial]);
                } else {
                    self.set_layout(next);
                }
            }
            None => self.emit_by_name::<()>("empty", &[]),
        }
    }

    /// Brings back a closed pane. If the layout is unchanged since then, to its
    /// old place; otherwise next to the active pane.
    pub fn restore_closed(&self, serial: u64) {
        let entry = {
            let mut closed = self.imp().closed.borrow_mut();
            closed.iter().position(|(s, ..)| *s == serial).map(|i| closed.remove(i))
        };
        let Some((_, pane, before, after)) = entry else { return };
        let current = self.layout();
        let next = if current.same_shape(&after) {
            before
        } else {
            let (w, h) = self.size();
            match current.split_auto(current.focus(), w, h) {
                Some((l, new_id)) => {
                    pane.set_id(new_id);
                    l
                }
                None => {
                    pane.terminal().terminate();
                    return;
                }
            }
        };
        self.imp().panes.borrow_mut().insert(pane.id(), pane);
        self.set_layout(next);
    }

    /// Takes out a pane together with its running process (drag & drop into another
    /// tab). For the last pane the tab stays empty; the window closes it.
    pub fn take_pane(&self, id: PaneId) -> Option<Pane> {
        let before = self.layout();
        if !before.contains(id) {
            return None;
        }
        let pane = self.imp().panes.borrow_mut().remove(&id)?;
        detach(pane.upcast_ref());
        match before.close(id) {
            Some(next) => self.set_layout(next),
            None => self.emit_by_name::<()>("empty", &[]),
        }
        Some(pane)
    }

    /// Takes in a pane from another tab, next to the active one. `false` if there is no
    /// room; then the pane belongs to nobody, and the caller gives it back.
    pub fn adopt_pane(&self, pane: &Pane) -> bool {
        let current = self.layout();
        let (w, h) = self.size();
        let Some((next, new_id)) = current.split_auto(current.focus(), w, h) else { return false };
        pane.set_id(new_id);
        pane.set_owner(self);
        self.imp().panes.borrow_mut().insert(new_id, pane.clone());
        let next = next.with_focus(new_id).unwrap_or(next);
        self.set_layout(next);
        pane.focus_terminal();
        crate::session::schedule_save();
        true
    }

    /// "Undo" has expired: end the pane's process and release it.
    pub fn discard_closed(&self, serial: u64) {
        let entry = {
            let mut closed = self.imp().closed.borrow_mut();
            closed.iter().position(|(s, ..)| *s == serial).map(|i| closed.remove(i))
        };
        if let Some((_, pane, ..)) = entry {
            pane.terminal().terminate();
        }
    }

    /// End all processes of this tab (tab closed for good).
    pub fn terminate_all(&self) {
        for pane in self.imp().panes.borrow().values() {
            pane.terminal().terminate();
        }
        for (_, pane, ..) in self.imp().closed.borrow_mut().drain(..) {
            pane.terminal().terminate();
        }
    }

    pub fn focus_direction(&self, dir: Direction) {
        let layout = self.layout();
        let (w, h) = self.size();
        if let Some(n) = layout.neighbour(layout.focus(), dir, w, h) {
            self.focus_pane(n);
        }
    }

    /// Focus on pane `id`; cancels a zoom if the pane is hidden.
    pub fn focus_pane(&self, id: PaneId) {
        let layout = self.layout();
        let Some(next) = layout.with_focus(id) else { return };
        if layout.zoomed().is_some_and(|z| z != id) {
            self.set_layout(next.without_zoom());
        } else {
            *self.imp().layout.borrow_mut() = next;
            self.update_states();
            self.emit_changed();
        }
        self.focus_terminal();
    }

    /// Pane with this terminal (see `Terminal::uid`).
    pub fn pane_with_terminal(&self, uid: u64) -> Option<PaneId> {
        self.imp().panes.borrow().iter().find(|(_, p)| p.terminal().uid() == uid).map(|(id, _)| *id)
    }

    pub fn focus_index(&self, n: usize) {
        if let Some(id) = self.layout().panes().get(n.wrapping_sub(1)) {
            self.focus_pane(*id);
        }
    }

    pub fn resize(&self, dir: Direction) {
        let layout = self.layout();
        if let Some(next) = layout.resize(layout.focus(), dir, RESIZE_STEP) {
            self.set_layout(next);
        }
    }

    pub fn equalize(&self) {
        self.set_layout(self.layout().equalize());
    }

    pub fn toggle_zoom(&self) {
        let layout = self.layout();
        if layout.panes().len() < 2 {
            return;
        }
        if let Some(next) = layout.toggle_zoom(layout.focus()) {
            self.set_layout(next);
        }
    }

    /// Rebuilds the widget tree from the model.
    fn rebuild(&self) {
        let imp = self.imp();
        for pane in imp.panes.borrow().values() {
            detach(pane.upcast_ref());
        }
        let layout = self.layout();
        let root: gtk::Widget = match layout.zoomed() {
            Some(z) => imp.panes.borrow()[&z].clone().upcast(),
            None => self.build_node(layout.root(), Vec::new()),
        };
        imp.overlay.get().unwrap().set_child(Some(&root));
        self.update_states();
        let weak = self.downgrade();
        glib::idle_add_local_once(move || {
            // tabs in the background (not mapped) do not grab the focus
            if let Some(tab) = weak.upgrade().filter(|t| t.is_mapped()) {
                tab.focus_terminal();
            }
        });
    }

    fn build_node(&self, node: &Node, path: Vec<Side>) -> gtk::Widget {
        match node {
            Node::Pane(id) => self.imp().panes.borrow()[id].clone().upcast(),
            Node::Split { orientation, first, second, .. } => {
                let paned = gtk::Paned::builder()
                    .orientation(match orientation {
                        Orientation::Horizontal => gtk::Orientation::Horizontal,
                        Orientation::Vertical => gtk::Orientation::Vertical,
                    })
                    .shrink_start_child(false)
                    .shrink_end_child(false)
                    .resize_start_child(true)
                    .resize_end_child(true)
                    .build();
                let mut p1 = path.clone();
                p1.push(Side::First);
                let mut p2 = path.clone();
                p2.push(Side::Second);
                paned.set_start_child(Some(&self.build_node(first, p1)));
                paned.set_end_child(Some(&self.build_node(second, p2)));
                self.sync_ratio(&paned, path);
                // Double-click on the divider equalizes all sizes (like Tilix). Capture phase,
                // because VTE otherwise takes the clicks itself; only near the divider counts. We count the
                // clicks ourselves: GtkPaned claims the press on the handle for its drag gesture,
                // which cancels this gesture and `n_press` stays at 1 (VM test).
                let click =
                    gtk::GestureClick::builder().button(1).propagation_phase(gtk::PropagationPhase::Capture).build();
                let weak = self.downgrade();
                let last = std::cell::Cell::new(0u32);
                click.connect_pressed(move |g, _, x, y| {
                    let Some(paned) = g.widget().and_downcast::<gtk::Paned>() else { return };
                    let at = if paned.orientation() == gtk::Orientation::Horizontal { x } else { y };
                    if (at - paned.position() as f64).abs() > 6.0 {
                        return;
                    }
                    let now = g.current_event_time();
                    let limit = gtk::Settings::default().map_or(400, |s| s.gtk_double_click_time()) as u32;
                    let double = last.get() != 0 && now.wrapping_sub(last.get()) <= limit;
                    last.set(if double { 0 } else { now });
                    if double {
                        if let Some(tab) = weak.upgrade() {
                            tab.equalize();
                            crate::session::schedule_save();
                        }
                    }
                });
                paned.add_controller(click);
                paned.upcast()
            }
        }
    }

    /// Keeps divider and model in sync: on a new size (max-position) the fraction from the
    /// model is applied; if the user drags the divider, the new fraction goes into the model.
    /// GtkPaned only knows pixels – without this the split "drifts" (Tilix `session.d:1631`).
    fn sync_ratio(&self, paned: &gtk::Paned, path: Vec<Side>) {
        use std::cell::Cell;
        use std::rc::Rc;
        let applying = Rc::new(Cell::new(false));
        let last_max = Rc::new(Cell::new(-1));

        let apply = {
            let weak = self.downgrade();
            let path = path.clone();
            let applying = applying.clone();
            let last_max = last_max.clone();
            move |p: &gtk::Paned| {
                let Some(tab) = weak.upgrade() else { return };
                let (max, total) = (p.max_position(), extent(p));
                if max <= 0 || total <= 0 {
                    return;
                }
                last_max.set(max);
                let ratio = tab.imp().layout.borrow().ratio_at(&path);
                debug!("apply {path:?} total={total} max={max} ratio={ratio:?}");
                if let Some(ratio) = ratio {
                    applying.set(true);
                    // GTK clamps to [min-position, max-position] itself
                    p.set_position((ratio * total as f32).round() as i32);
                    applying.set(false);
                }
            }
        };
        paned.connect_max_position_notify(apply.clone());
        paned.connect_map(apply);

        let weak = self.downgrade();
        paned.connect_position_notify(move |p| {
            let max = p.max_position();
            // only changes at unchanged size come from the user
            debug!(
                "position {path:?} pos={} max={max} last={} applying={}",
                p.position(),
                last_max.get(),
                applying.get()
            );
            if applying.get() || max <= 0 || max != last_max.get() {
                return;
            }
            let Some(tab) = weak.upgrade() else { return };
            let total = extent(p);
            if total <= 0 {
                return;
            }
            let ratio = p.position() as f32 / total as f32;
            debug!("user {path:?} pos={} total={total} ratio={ratio}", p.position());
            let next = tab.imp().layout.borrow().set_ratio(&path, ratio);
            if let Some(next) = next {
                *tab.imp().layout.borrow_mut() = next;
                crate::session::schedule_save();
            }
        });
    }

    pub fn sync(&self) -> bool {
        self.imp().sync.get()
    }

    /// Shared input on/off; excluded panes stay excluded.
    pub fn set_sync(&self, on: bool) {
        self.imp().sync.set(on);
        self.update_states();
        self.emit_changed();
    }

    /// Panes that receive shared input (none when input is off).
    pub fn sync_count(&self) -> usize {
        if !self.sync() {
            return 0;
        }
        self.imp().panes.borrow().values().filter(|p| !p.terminal().sync_excluded()).count()
    }

    /// Input from `source` to the other participating panes. The lock prevents forwarded
    /// text from being forwarded again.
    fn forward_input(&self, source: &crate::terminal::Terminal, text: &str) {
        thread_local! {
            static FORWARDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        }
        if !self.sync() || source.sync_excluded() || FORWARDING.with(|f| f.get()) {
            return;
        }
        let targets: Vec<_> = self
            .imp()
            .panes
            .borrow()
            .values()
            .map(|p| p.terminal().clone())
            .filter(|t| !t.sync_excluded() && t.uid() != source.uid())
            .collect();
        FORWARDING.with(|f| f.set(true));
        for t in targets {
            t.feed(text);
        }
        FORWARDING.with(|f| f.set(false));
    }

    pub fn update_states(&self) {
        let imp = self.imp();
        let layout = self.layout();
        let multi = layout.zoomed().is_none() && layout.panes().len() > 1;
        let order = layout.panes();
        let mode = imp.layout_mode.get();
        for (id, pane) in imp.panes.borrow().iter() {
            pane.set_state(*id == layout.focus(), multi);
            pane.set_sync(imp.sync.get() && !pane.terminal().sync_excluded());
            let number = order.iter().position(|p| p == id).map(|i| i as u32 + 1);
            pane.set_badge(if mode && multi { number } else { None });
        }
        let hidden = imp.panes.borrow().len().saturating_sub(1);
        let banner = imp.zoom_banner.get().unwrap();
        banner.set_visible(layout.zoomed().is_some());
        imp.zoom_label.get().unwrap().set_markup(&ntrf(
            "<b>Maximized</b> · {n} pane hidden",
            "<b>Maximized</b> · {n} panes hidden",
            hidden as u32,
            &[],
        ));
    }
}

/// Length of a paned along its split direction. `max-position` is no good for that:
/// GTK already subtracts the minimum size of the second child there.
fn extent(p: &gtk::Paned) -> i32 {
    match p.orientation() {
        gtk::Orientation::Horizontal => p.width(),
        _ => p.height(),
    }
}

/// Takes a widget out of its GtkPaned or overlay.
fn detach(w: &gtk::Widget) {
    let Some(parent) = w.parent() else { return };
    if let Some(paned) = parent.downcast_ref::<gtk::Paned>() {
        if paned.start_child().as_ref() == Some(w) {
            paned.set_start_child(gtk::Widget::NONE);
        } else {
            paned.set_end_child(gtk::Widget::NONE);
        }
    } else if let Some(overlay) = parent.downcast_ref::<gtk::Overlay>() {
        overlay.set_child(gtk::Widget::NONE);
    } else {
        w.unparent();
    }
}

/// PARKETT_DEBUG_LAYOUT set? Read once instead of on every divider move (D6).
fn debug_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PARKETT_DEBUG_LAYOUT").is_some())
}
