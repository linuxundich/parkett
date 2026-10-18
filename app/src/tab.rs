//! Ein Tab: Split-Layout aus `parkett_core::layout` plus die Bereichs-Widgets.
//!
//! Das Modell ist maßgeblich. Nach jeder Änderung baut `rebuild()` den GtkPaned-Baum neu
//! und hängt die vorhandenen Bereiche nur um – Terminals und ihre Prozesse bleiben erhalten.

use std::collections::HashMap;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::subclass::Signal;
use parkett_core::layout::{Direction, Layout, Node, Orientation, PaneId, Side};
use vte::prelude::*;

use crate::pane::Pane;
use crate::terminal::{MIN_COLUMNS, MIN_ROWS};

/// Schrittweite beim Verschieben eines Trenners per Tastatur (Anteil der Fläche).
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
                    // Titel, Verzeichnis, Fokus oder Prozesszustand haben sich geändert
                    Signal::builder("changed").build(),
                    // letzter Bereich wurde geschlossen
                    Signal::builder("empty").build(),
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
    pub fn new(cwd: Option<String>) -> Self {
        let tab: Tab = glib::Object::new();
        let overlay = gtk::Overlay::new();

        let zoom_label = gtk::Label::new(None);
        let show_all = gtk::Button::builder().label("Alle zeigen").action_name("win.toggle-zoom").build();
        let banner = gtk::Box::builder()
            .css_classes(["parkett-zoom-banner"])
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Start)
            .margin_top(8)
            .spacing(10)
            .visible(false)
            .build();
        banner.append(&gtk::Image::from_icon_name("view-fullscreen-symbolic"));
        banner.append(&zoom_label);
        banner.append(&show_all);
        overlay.add_overlay(&banner);
        tab.set_child(Some(&overlay));

        let imp = tab.imp();
        imp.overlay.set(overlay).ok();
        imp.zoom_banner.set(banner).ok();
        imp.zoom_label.set(zoom_label).ok();

        let first = imp.layout.borrow().focus();
        tab.add_pane(first, cwd);
        tab.rebuild();
        tab
    }

    pub fn connect_changed(&self, f: impl Fn(&Self) + 'static) {
        self.connect_local("changed", false, move |args| {
            f(&args[0].get::<Tab>().unwrap());
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

    fn add_pane(&self, id: PaneId, cwd: Option<String>) -> Pane {
        let pane = Pane::new(id, cwd);
        let weak = self.downgrade();
        pane.terminal().connect_state_changed(move || {
            if let Some(tab) = weak.upgrade() {
                tab.emit_changed();
            }
        });
        // „Schließen“ in der Leiste nach Prozessende
        let weak = self.downgrade();
        pane.terminal().connect_close(move || {
            if let Some(tab) = weak.upgrade() {
                tab.close_pane(id);
            }
        });
        // Fokus per Klick oder Tab-Taste ins Modell übernehmen
        let weak = self.downgrade();
        pane.terminal().vte().connect_has_focus_notify(move |vte| {
            if !vte.has_focus() {
                return;
            }
            let Some(tab) = weak.upgrade() else { return };
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

    /// Tab-Titel: eigener Name, sonst Titel des fokussierten Terminals; bei Zoom „(1/3)“.
    pub fn title(&self) -> String {
        let base = self.custom_title().or_else(|| self.focused_pane().terminal().title()).unwrap_or_else(|| "Shell".into());
        match self.layout().zoomed() {
            Some(_) => format!("{base} (1/{})", self.pane_count()),
            None => base,
        }
    }

    pub fn cwd(&self) -> Option<String> {
        self.focused_pane().terminal().cwd()
    }

    /// Startverzeichnis für ein neues Terminal nach Einstellung „new-terminal-directory“.
    pub fn inherited_cwd(&self) -> Option<String> {
        if crate::settings::inherit_directory() { self.cwd() } else { None }
    }

    /// Hat ein Bereich mit Fehler geendet?
    pub fn has_failed_pane(&self) -> bool {
        use crate::terminal::ExitStatus;
        self.imp().panes.borrow().values().any(|p| {
            matches!(p.terminal().exit_status(), Some(ExitStatus::Code(c)) if c != 0) || matches!(p.terminal().exit_status(), Some(ExitStatus::Signal(_)))
        })
    }

    /// Laufende Vordergrundprozesse aller Bereiche (für die Nachfrage beim Schließen).
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
        if w > 0.0 && h > 0.0 { (w, h) } else { (1000.0, 600.0) }
    }

    fn min_pane_size(&self) -> (f32, f32) {
        let vte = self.focused_pane().terminal().vte().clone();
        ((vte.char_width() * MIN_COLUMNS) as f32, (vte.char_height() * MIN_ROWS) as f32)
    }

    fn set_layout(&self, layout: Layout) {
        *self.imp().layout.borrow_mut() = layout;
        self.rebuild();
        self.emit_changed();
    }

    /// Teilt den fokussierten Bereich; `None` = entlang der längeren Seite.
    /// `false`, wenn ein Teil kleiner als die Mindestgröße würde.
    pub fn split(&self, orientation: Option<Orientation>) -> bool {
        let layout = self.layout();
        let focus = layout.focus();
        let (w, h) = self.size();
        let (min_w, min_h) = self.min_pane_size();
        let o = orientation.unwrap_or_else(|| {
            let r = layout.rect_of(focus, w, h).unwrap();
            if r.w / 2.0 >= r.h { Orientation::Horizontal } else { Orientation::Vertical }
        });
        // Bei Zoom gilt die volle Fläche – nach dem Teilen ist der Zoom ohnehin aufgehoben
        if layout.zoomed().is_none() && !layout.can_split(focus, o, w, h, min_w, min_h) {
            return false;
        }
        let cwd = self.inherited_cwd();
        let Some((next, id)) = layout.split(focus, o) else { return false };
        self.add_pane(id, cwd);
        self.set_layout(next);
        true
    }

    /// Schließt einen Bereich. Beim letzten Bereich meldet der Tab „empty“.
    pub fn close_pane(&self, id: PaneId) {
        match self.layout().close(id) {
            Some(next) => {
                let pane = self.imp().panes.borrow_mut().remove(&id);
                if let Some(pane) = pane {
                    pane.terminal().terminate();
                    detach(pane.upcast_ref());
                }
                self.set_layout(next);
            }
            None => self.emit_by_name::<()>("empty", &[]),
        }
    }

    pub fn focus_direction(&self, dir: Direction) {
        let layout = self.layout();
        let (w, h) = self.size();
        if let Some(n) = layout.neighbour(layout.focus(), dir, w, h) {
            self.focus_pane(n);
        }
    }

    /// Fokus auf Bereich `id`; hebt einen Zoom auf, wenn der Bereich verborgen ist.
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

    /// Bereich Nummer `n` (1-basiert, Lesereihenfolge).
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

    /// Baut den Widget-Baum aus dem Modell neu auf.
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
            if let Some(tab) = weak.upgrade() {
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
                paned.upcast()
            }
        }
    }

    /// Hält Trenner und Modell gleich: Bei neuer Größe (max-position) wird der Anteil aus dem
    /// Modell angewendet; zieht der Nutzer den Trenner, wandert der neue Anteil ins Modell.
    /// GtkPaned kennt nur Pixel – ohne das „driftet“ die Aufteilung (Tilix `session.d:1631`).
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
                debug(&format!("apply {path:?} total={total} max={max} ratio={ratio:?}"));
                if let Some(ratio) = ratio {
                    applying.set(true);
                    // GTK begrenzt selbst auf [min-position, max-position]
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
            // nur Änderungen bei unveränderter Größe stammen vom Nutzer
            debug(&format!("position {path:?} pos={} max={max} last={} applying={}", p.position(), last_max.get(), applying.get()));
            if applying.get() || max <= 0 || max != last_max.get() {
                return;
            }
            let Some(tab) = weak.upgrade() else { return };
            let total = extent(p);
            if total <= 0 {
                return;
            }
            let ratio = p.position() as f32 / total as f32;
            debug(&format!("user {path:?} pos={} total={total} ratio={ratio}", p.position()));
            let next = tab.imp().layout.borrow().set_ratio(&path, ratio);
            if let Some(next) = next {
                *tab.imp().layout.borrow_mut() = next;
            }
        });
    }

    pub fn update_states(&self) {
        let imp = self.imp();
        let layout = self.layout();
        let multi = layout.zoomed().is_none() && layout.panes().len() > 1;
        for (id, pane) in imp.panes.borrow().iter() {
            pane.set_state(*id == layout.focus(), multi);
        }
        let hidden = imp.panes.borrow().len().saturating_sub(1);
        let banner = imp.zoom_banner.get().unwrap();
        banner.set_visible(layout.zoomed().is_some());
        imp.zoom_label.get().unwrap().set_markup(&format!(
            "<b>Maximiert</b> · {hidden} {} ausgeblendet",
            if hidden == 1 { "Bereich" } else { "Bereiche" }
        ));
    }
}

/// Länge eines Paned entlang seiner Teilungsrichtung. `max-position` taugt dafür nicht:
/// GTK zieht dort schon die Mindestgröße des zweiten Kindes ab.
fn extent(p: &gtk::Paned) -> i32 {
    match p.orientation() {
        gtk::Orientation::Horizontal => p.width(),
        _ => p.height(),
    }
}

/// Nimmt ein Widget aus seinem GtkPaned oder Overlay heraus.
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

/// Ausgabe nur mit PARKETT_DEBUG_LAYOUT=1 (Fehlersuche beim Abgleich Trenner ⇄ Modell).
fn debug(msg: &str) {
    if std::env::var_os("PARKETT_DEBUG_LAYOUT").is_some() {
        eprintln!("[layout] {msg}");
    }
}
