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

    /// `multi`: Tab hat mehr als einen sichtbaren Bereich.
    pub fn set_state(&self, focused: bool, multi: bool) {
        let imp = self.imp();
        imp.pill.get().unwrap().set_visible(multi);
        imp.focus_line.get().unwrap().set_visible(multi && focused);
        imp.dim.get().unwrap().set_visible(multi && !focused && crate::settings::dim_unfocused());
        if focused {
            self.add_css_class("focused");
        } else {
            self.remove_css_class("focused");
        }
    }
}
