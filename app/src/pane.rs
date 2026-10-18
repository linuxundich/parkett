//! Ein Bereich im Split-Baum: das Terminal plus schwebender Titel, Fokuslinie und Abblendung.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use vte::prelude::*;

use crate::terminal::Terminal;

mod imp {
    use std::cell::{Cell, OnceCell};

    use super::*;

    #[derive(Default)]
    pub struct Pane {
        pub id: Cell<u32>,
        pub terminal: OnceCell<Terminal>,
        pub pill: OnceCell<gtk::Box>,
        pub pill_label: OnceCell<gtk::Label>,
        pub focus_line: OnceCell<gtk::Box>,
        pub dim: OnceCell<gtk::DrawingArea>,
        pub search: OnceCell<gtk::Revealer>,
        pub badge: OnceCell<gtk::Label>,
        pub search_entry: OnceCell<gtk::SearchEntry>,
        pub search_case: OnceCell<gtk::ToggleButton>,
        pub search_regex: OnceCell<gtk::ToggleButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Pane {
        const NAME: &'static str = "ParkettPane";
        type Type = super::Pane;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.set_css_name("parkett-pane");
        }
    }

    impl ObjectImpl for Pane {
        fn dispose(&self) {
            if let Some(t) = self.terminal.get() {
                t.terminate();
            }
        }
    }
    impl WidgetImpl for Pane {}
    impl BinImpl for Pane {}
}

glib::wrapper! {
    pub struct Pane(ObjectSubclass<imp::Pane>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Pane {
    pub fn new(id: u32, cwd: Option<String>, profile_id: &str) -> Self {
        let pane: Pane = glib::Object::new();
        let imp = pane.imp();
        imp.id.set(id);

        let terminal = Terminal::new(cwd, profile_id);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(terminal.widget()));

        // Abblendung inaktiver Bereiche: halbtransparente Fläche in der Hintergrundfarbe des
        // Farbschemas (CSS kennt die Palette nicht, daher selbst gezeichnet)
        let dim = gtk::DrawingArea::builder().can_target(false).visible(false).build();
        {
            let t = terminal.clone();
            dim.set_draw_func(move |_, cr, w, h| {
                let bg = t.background();
                cr.set_source_rgba(bg.r as f64 / 255.0, bg.g as f64 / 255.0, bg.b as f64 / 255.0, 0.38);
                cr.rectangle(0.0, 0.0, w as f64, h as f64);
                let _ = cr.fill();
            });
        }
        let d2 = dim.downgrade();
        terminal.connect_colors_changed(move || {
            if let Some(d) = d2.upgrade() {
                d.queue_draw();
            }
        });
        overlay.add_overlay(&dim);

        // Fokus: 2 px Akzentlinie oben – nie nur Abblendung (Barrierefreiheit)
        let focus_line = gtk::Box::builder()
            .css_classes(["parkett-focus-line"])
            .valign(gtk::Align::Start)
            .height_request(2)
            .can_target(false)
            .visible(false)
            .build();
        overlay.add_overlay(&focus_line);

        // Schwebender Titel, nur bei geteilten Tabs sichtbar (Entscheidung A+: schwebend)
        let pill_label = gtk::Label::builder().ellipsize(gtk::pango::EllipsizeMode::End).max_width_chars(28).build();
        let zoom = gtk::Button::builder()
            .icon_name("view-fullscreen-symbolic")
            .tooltip_text("Bereich maximieren (Strg+Umschalt+X)")
            .css_classes(["flat", "circular"])
            .build();
        let close = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Bereich schließen (Strg+Umschalt+W)")
            .css_classes(["flat", "circular"])
            .build();
        let pill = gtk::Box::builder()
            .css_classes(["parkett-pill"])
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .margin_top(6)
            .margin_end(8)
            .spacing(2)
            .visible(false)
            .build();
        pill.append(&pill_label);
        pill.append(&zoom);
        pill.append(&close);
        overlay.add_overlay(&pill);

        // Nummer im Layout-Modus (mittig, groß)
        let badge = gtk::Label::builder()
            .css_classes(["parkett-badge"])
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .can_target(false)
            .visible(false)
            .build();
        overlay.add_overlay(&badge);

        // Suchleiste oben rechts (ersetzt dort den schwebenden Titel, solange sie offen ist)
        let entry = gtk::SearchEntry::builder().placeholder_text("Im Terminal suchen").width_chars(22).build();
        let case = gtk::ToggleButton::builder().label("Aa").tooltip_text("Groß-/Kleinschreibung beachten").css_classes(["flat"]).build();
        let regex = gtk::ToggleButton::builder().label(".*").tooltip_text("Regulärer Ausdruck").css_classes(["flat"]).build();
        let up = gtk::Button::builder().icon_name("go-up-symbolic").tooltip_text("Treffer darüber (Eingabe, Strg+Umschalt+G)").css_classes(["flat"]).build();
        let down = gtk::Button::builder().icon_name("go-down-symbolic").tooltip_text("Treffer darunter (Umschalt+Eingabe, Strg+Umschalt+H)").css_classes(["flat"]).build();
        let close_search = gtk::Button::builder().icon_name("window-close-symbolic").tooltip_text("Suche schließen (Esc)").css_classes(["flat"]).build();
        let bar = gtk::Box::builder().spacing(2).css_classes(["parkett-search"]).build();
        for w in [entry.upcast_ref::<gtk::Widget>(), case.upcast_ref(), regex.upcast_ref(), up.upcast_ref(), down.upcast_ref(), close_search.upcast_ref()] {
            bar.append(w);
        }
        let search = gtk::Revealer::builder()
            .child(&bar)
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .margin_top(6)
            .margin_end(8)
            .visible(false)
            .build();
        overlay.add_overlay(&search);

        pane.set_child(Some(&overlay));

        // Knöpfe wirken auf diesen Bereich: erst fokussieren, dann die Fensteraktion
        for (button, action) in [(&zoom, "win.toggle-zoom"), (&close, "win.close-pane")] {
            let weak = pane.downgrade();
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.focus_terminal();
                    let _ = p.activate_action(action, None);
                }
            });
        }

        // Rechtsklick fokussiert vorher, damit das Kontextmenü für diesen Bereich gilt
        let click = gtk::GestureClick::builder().button(0).propagation_phase(gtk::PropagationPhase::Capture).build();
        let weak = pane.downgrade();
        click.connect_pressed(move |_, _, _, _| {
            if let Some(p) = weak.upgrade() {
                p.focus_terminal();
            }
        });
        pane.add_controller(click);

        let update = {
            let weak = pane.downgrade();
            move || {
                if let Some(p) = weak.upgrade() {
                    let t = p.terminal();
                    p.imp().pill_label.get().unwrap().set_label(&t.title().unwrap_or_else(|| "Shell".into()));
                }
            }
        };
        terminal.connect_state_changed(update.clone());

        // Mindestgröße 20×5 Zellen: Trenner lassen sich nicht weiter schieben
        let set_min = {
            let weak = pane.downgrade();
            move |v: &vte::Terminal| {
                if let Some(p) = weak.upgrade() {
                    p.set_size_request(
                        (v.char_width() * crate::terminal::MIN_COLUMNS) as i32,
                        (v.char_height() * crate::terminal::MIN_ROWS) as i32,
                    );
                }
            }
        };
        set_min(terminal.vte());
        terminal.vte().connect_char_size_changed(move |v, _, _| set_min(v));

        imp.terminal.set(terminal).ok();
        imp.pill.set(pill).ok();
        imp.pill_label.set(pill_label).ok();
        imp.focus_line.set(focus_line).ok();
        imp.dim.set(dim).ok();
        imp.search.set(search).ok();
        imp.badge.set(badge).ok();
        imp.search_entry.set(entry.clone()).ok();
        imp.search_case.set(case.clone()).ok();
        imp.search_regex.set(regex.clone()).ok();

        // Suche: bei jeder Änderung neu setzen und vom Ende her den ersten Treffer suchen
        let refresh = {
            let weak = pane.downgrade();
            move || {
                if let Some(p) = weak.upgrade() {
                    p.apply_search();
                }
            }
        };
        let r1 = refresh.clone();
        entry.connect_search_changed(move |_| r1());
        let r2 = refresh.clone();
        case.connect_toggled(move |_| r2());
        regex.connect_toggled(move |_| refresh());
        for (button, up_dir) in [(&up, true), (&down, false)] {
            let weak = pane.downgrade();
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.search_step(up_dir);
                }
            });
        }
        let weak = pane.downgrade();
        entry.connect_activate(move |_| {
            if let Some(p) = weak.upgrade() {
                p.search_step(true);
            }
        });
        let weak = pane.downgrade();
        entry.connect_stop_search(move |_| {
            if let Some(p) = weak.upgrade() {
                p.hide_search();
            }
        });
        let weak = pane.downgrade();
        close_search.connect_clicked(move |_| {
            if let Some(p) = weak.upgrade() {
                p.hide_search();
            }
        });
        // Umschalt+Eingabe = Treffer darunter
        let keys = gtk::EventControllerKey::new();
        let weak = pane.downgrade();
        keys.connect_key_pressed(move |_, key, _, state| {
            let enter = matches!(key, gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter);
            if enter && state.contains(gtk::gdk::ModifierType::SHIFT_MASK) {
                if let Some(p) = weak.upgrade() {
                    p.search_step(false);
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        entry.add_controller(keys);
        update();
        pane
    }

    pub fn id(&self) -> u32 {
        self.imp().id.get()
    }

    pub fn terminal(&self) -> &Terminal {
        self.imp().terminal.get().expect("Terminal wird im Konstruktor gesetzt")
    }

    pub fn focus_terminal(&self) {
        self.terminal().vte().grab_focus();
    }

    /// Nummer im Layout-Modus; `None` blendet sie aus.
    pub fn set_badge(&self, number: Option<u32>) {
        let badge = self.imp().badge.get().unwrap();
        match number {
            Some(n) => {
                badge.set_label(&n.to_string());
                badge.set_visible(true);
            }
            None => badge.set_visible(false),
        }
    }

    pub fn search_visible(&self) -> bool {
        self.imp().search.get().unwrap().is_visible()
    }

    /// Öffnet die Suche; markierter Text wird Suchbegriff.
    pub fn show_search(&self) {
        let imp = self.imp();
        let entry = imp.search_entry.get().unwrap();
        if let Some(sel) = self.terminal().vte().text_selected(vte::Format::Text) {
            let sel = sel.trim();
            if !sel.is_empty() && !sel.contains('\n') {
                entry.set_text(sel);
            }
        }
        let search = imp.search.get().unwrap();
        search.set_visible(true);
        search.set_reveal_child(true);
        imp.pill.get().unwrap().set_visible(false);
        entry.grab_focus();
        self.apply_search();
    }

    pub fn hide_search(&self) {
        let imp = self.imp();
        let search = imp.search.get().unwrap();
        search.set_reveal_child(false);
        search.set_visible(false);
        let _ = self.terminal().set_search(None, false, false);
        // schwebenden Titel wiederherstellen, falls der Reiter geteilt ist
        let _ = self.activate_action("win.refresh-panes", None);
        self.focus_terminal();
    }

    fn apply_search(&self) {
        let imp = self.imp();
        let entry = imp.search_entry.get().unwrap();
        let text = entry.text().to_string();
        match self.terminal().set_search(Some(&text), imp.search_regex.get().unwrap().is_active(), imp.search_case.get().unwrap().is_active()) {
            Ok(()) => {
                entry.set_tooltip_text(None);
                let found = text.is_empty() || self.terminal().search_step(true);
                set_error(entry, !found);
            }
            Err(e) => {
                entry.set_tooltip_text(Some(&format!("Ungültiger Ausdruck: {e}")));
                set_error(entry, true);
            }
        }
    }

    /// Nächster Treffer; öffnet die Suche, wenn sie zu ist.
    pub fn search_step(&self, up: bool) {
        if !self.search_visible() {
            self.show_search();
            return;
        }
        let entry = self.imp().search_entry.get().unwrap();
        if !entry.text().is_empty() {
            let found = self.terminal().search_step(up);
            set_error(entry, !found);
        }
    }

    /// `multi`: Tab hat mehr als einen sichtbaren Bereich.
    pub fn set_state(&self, focused: bool, multi: bool) {
        let imp = self.imp();
        imp.pill.get().unwrap().set_visible(multi && !imp.search.get().is_some_and(|s| s.is_visible()));
        imp.focus_line.get().unwrap().set_visible(multi && focused);
        imp.dim.get().unwrap().set_visible(multi && !focused && crate::settings::dim_unfocused());
        if focused {
            self.add_css_class("focused");
        } else {
            self.remove_css_class("focused");
        }
    }
}

fn set_error(entry: &gtk::SearchEntry, error: bool) {
    if error {
        entry.add_css_class("error");
    } else {
        entry.remove_css_class("error");
    }
}
