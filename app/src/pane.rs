// SPDX-License-Identifier: GPL-3.0-or-later
//! Ein Bereich im Split-Baum: das Terminal plus schwebender Titel, Fokuslinie und Abblendung.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use vte::prelude::*;

use crate::i18n::{gettext, ntrf, trf, N_};
use crate::shortcuts::live_tooltip;
use crate::terminal::{Launch, Terminal};
use parkett_core::layout::DropZone;

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct Pane {
        pub id: Cell<u32>,
        /// Reiter, dem der Bereich gehört; ändert sich beim Ziehen in einen anderen Reiter
        pub owner: RefCell<glib::WeakRef<crate::tab::Tab>>,
        pub terminal: OnceCell<Terminal>,
        pub pill: OnceCell<gtk::Box>,
        pub pill_label: OnceCell<gtk::Label>,
        pub focus_line: OnceCell<gtk::Box>,
        pub sync_line: OnceCell<gtk::Box>,
        /// Zone beim Ablegen eines gezogenen Bereichs
        pub drop_hint: OnceCell<gtk::Box>,
        pub context_line: OnceCell<gtk::DrawingArea>,
        pub multi: Cell<bool>,
        pub narrow: Cell<bool>,
        pub row: OnceCell<gtk::Box>,
        pub row_label: OnceCell<gtk::Label>,
        pub focused: Cell<bool>,
        pub dim: OnceCell<gtk::DrawingArea>,
        pub search: OnceCell<gtk::Revealer>,
        pub badge: OnceCell<gtk::Label>,
        /// Schloss oben links bei Schreibschutz
        pub lock: OnceCell<gtk::Image>,
        pub search_entry: OnceCell<gtk::SearchEntry>,
        pub search_count: OnceCell<gtk::Label>,
        pub search_case: OnceCell<gtk::ToggleButton>,
        pub search_regex: OnceCell<gtk::ToggleButton>,
        pub search_word: OnceCell<gtk::ToggleButton>,
        /// Zeiger im oberen Streifen (Design V2: Titel nur dann sichtbar)
        pub hover_top: Cell<bool>,
        /// Kurz nach einem Fokuswechsel sichtbar; Quelle des Ausblende-Timers
        pub flash: RefCell<Option<glib::SourceId>>,
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
    pub fn new(id: u32, cwd: Option<String>, profile_id: &str, launch: Launch) -> Self {
        let pane: Pane = glib::Object::new();
        let imp = pane.imp();
        imp.id.set(id);

        let terminal = Terminal::new(cwd, profile_id, launch);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(terminal.widget()));

        // Abblendung inaktiver Bereiche: halbtransparente Fläche in der Hintergrundfarbe des
        // Farbschemas (CSS kennt die Palette nicht, daher selbst gezeichnet)
        let dim = gtk::DrawingArea::builder().can_target(false).visible(false).build();
        {
            let t = terminal.downgrade();
            dim.set_draw_func(move |_, cr, w, h| {
                let Some(t) = t.upgrade() else { return };
                let (r, g, b) = t.background().to_f64();
                cr.set_source_rgba(r, g, b, 0.38);
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
        // gemeinsame Eingabe: kräftige Linie oben, über Fokus- und Kontextlinie
        let sync_line = gtk::Box::builder()
            .css_classes(["parkett-sync-line"])
            .valign(gtk::Align::Start)
            .height_request(4)
            .can_target(false)
            .visible(false)
            .tooltip_text(gettext("Shared input"))
            .build();

        // Kontextfarbe (root/SSH): 3 px Linie oben in der Farbe des Schemas
        let context_line = gtk::DrawingArea::builder()
            .valign(gtk::Align::Start)
            .height_request(3)
            .can_target(false)
            .visible(false)
            .build();
        {
            let t = terminal.downgrade();
            context_line.set_draw_func(move |_, cr, w, h| {
                if let Some((line, _)) = t.upgrade().and_then(|t| t.context_accent()) {
                    let (r, g, b) = line.to_f64();
                    cr.set_source_rgb(r, g, b);
                    cr.rectangle(0.0, 0.0, w as f64, h as f64);
                    let _ = cr.fill();
                }
            });
        }
        overlay.add_overlay(&context_line);
        overlay.add_overlay(&sync_line);
        let drop_hint = gtk::Box::builder().css_classes(["parkett-drop-hint"]).can_target(false).visible(false).build();
        overlay.add_overlay(&drop_hint);

        // Breite des Bereichs beobachten: schmale Bereiche blenden den Text des schwebenden
        // Titels aus, damit er die Prompt-Zeile nicht verdeckt (QA). Unsichtbare Zeichenfläche,
        // weil AdwBin einen Layout-Manager hat und size_allocate nicht überschreibbar ist.
        let probe = gtk::DrawingArea::builder().can_target(false).opacity(0.0).build();
        let weak = pane.downgrade();
        probe.connect_resize(move |_, width, _| {
            let Some(p) = weak.upgrade() else { return };
            let narrow = width < 380;
            if narrow != p.imp().narrow.get() {
                p.imp().narrow.set(narrow);
                let w2 = p.downgrade();
                glib::idle_add_local_once(move || {
                    if let Some(p) = w2.upgrade() {
                        p.update_visibility();
                    }
                });
            }
        });
        overlay.add_overlay(&probe);

        // Schwebender Titel, nur bei geteilten Tabs (Entscheidung A+: schwebend). Design V2:
        // normalerweise ausgeblendet, damit er keine Terminalzeile verdeckt (QA-02); sichtbar
        // kurz nach einem Fokuswechsel und solange der Zeiger im oberen Streifen steht.
        let pill_label = gtk::Label::builder().ellipsize(gtk::pango::EllipsizeMode::End).max_width_chars(28).build();
        let zoom = gtk::Button::builder()
            .icon_name("parkett-pane-maximize-symbolic")
            .css_classes(["flat", "circular"])
            .build();
        live_tooltip(&zoom, N_("Maximize pane"), "win.toggle-zoom", &[]);
        let close = gtk::Button::builder().icon_name("window-close-symbolic").css_classes(["flat", "circular"]).build();
        live_tooltip(&close, N_("Close pane"), "win.close-pane", &[]);
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
        let motion = gtk::EventControllerMotion::new();
        let set_hover = {
            let weak = pane.downgrade();
            move |inside: bool| {
                let Some(p) = weak.upgrade() else { return };
                if p.imp().hover_top.replace(inside) != inside {
                    p.update_visibility();
                }
            }
        };
        let hover = set_hover.clone();
        motion.connect_enter(move |_, _, y| hover(y < HOVER_STRIP));
        let hover = set_hover.clone();
        motion.connect_motion(move |_, _, y| hover(y < HOVER_STRIP));
        motion.connect_leave(move |_| set_hover(false));
        overlay.add_controller(motion);

        // Nummer im Layout-Modus (mittig, groß)
        let badge = gtk::Label::builder()
            .css_classes(["parkett-badge"])
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .can_target(false)
            .visible(false)
            .build();
        overlay.add_overlay(&badge);
        let lock = gtk::Image::builder()
            .icon_name("changes-prevent-symbolic")
            .tooltip_text(gettext("Read only"))
            .css_classes(["parkett-lock"])
            .halign(gtk::Align::Start)
            .valign(gtk::Align::End)
            .margin_start(8)
            .margin_bottom(8)
            .visible(false)
            .build();
        overlay.add_overlay(&lock);
        pane.imp().lock.set(lock).ok();

        // Suchleiste oben rechts (ersetzt dort den schwebenden Titel, solange sie offen ist)
        let entry = gtk::SearchEntry::builder().placeholder_text(gettext("Search in terminal")).width_chars(22).build();
        // feste Breite: sonst springt das Suchfeld bei jeder neuen Zahl
        let count = gtk::Label::builder()
            .css_classes(["dim-label", "numeric"])
            .width_chars(10)
            .xalign(1.0)
            .margin_start(4)
            .margin_end(2)
            .build();
        let case =
            gtk::ToggleButton::builder().label("Aa").tooltip_text(gettext("Match case")).css_classes(["flat"]).build();
        let word = gtk::ToggleButton::builder()
            .label("W")
            .tooltip_text(gettext("Whole words only"))
            .css_classes(["flat"])
            .build();
        let regex = gtk::ToggleButton::builder()
            .label(".*")
            .tooltip_text(gettext("Regular expression"))
            .css_classes(["flat"])
            .build();
        let up = gtk::Button::builder().icon_name("go-up-symbolic").css_classes(["flat"]).build();
        live_tooltip(&up, N_("Previous match"), "win.find-up", const { &[N_("Enter")] });
        let down = gtk::Button::builder().icon_name("go-down-symbolic").css_classes(["flat"]).build();
        live_tooltip(&down, N_("Next match"), "win.find-down", const { &[N_("Shift+Enter")] });
        let close_search = gtk::Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text(gettext("Close search (Esc)"))
            .css_classes(["flat"])
            .build();
        close_search.update_property(&[gtk::accessible::Property::Label(&gettext("Close search"))]);
        let bar = gtk::Box::builder().spacing(2).css_classes(["parkett-search"]).build();
        for w in [
            entry.upcast_ref::<gtk::Widget>(),
            count.upcast_ref(),
            case.upcast_ref(),
            word.upcast_ref(),
            regex.upcast_ref(),
            up.upcast_ref(),
            down.upcast_ref(),
            close_search.upcast_ref(),
        ] {
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

        // Titelzeile (Einstellung „Bereichstitel: Als Zeile“) über dem Terminal
        let row_label =
            gtk::Label::builder().xalign(0.0).hexpand(true).ellipsize(gtk::pango::EllipsizeMode::End).build();
        let row_zoom = gtk::Button::builder().icon_name("parkett-pane-maximize-symbolic").css_classes(["flat"]).build();
        live_tooltip(&row_zoom, N_("Maximize pane"), "win.toggle-zoom", &[]);
        let row_close = gtk::Button::builder().icon_name("window-close-symbolic").css_classes(["flat"]).build();
        live_tooltip(&row_close, N_("Close pane"), "win.close-pane", &[]);
        let row = gtk::Box::builder().css_classes(["parkett-ptitle"]).spacing(4).visible(false).build();
        row.append(&row_label);
        row.append(&row_zoom);
        row.append(&row_close);
        for (button, action) in [(&row_zoom, "win.toggle-zoom"), (&row_close, "win.close-pane")] {
            let weak = pane.downgrade();
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.focus_terminal();
                    let _ = p.activate_action(action, None);
                }
            });
        }
        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        column.append(&row);
        column.append(&overlay);
        overlay.set_vexpand(true);
        pane.set_child(Some(&column));
        pane.imp().row.set(row).ok();
        pane.imp().row_label.set(row_label).ok();
        // Umschalten schwebend ⇄ Zeile sofort übernehmen
        let weak = pane.downgrade();
        let id = crate::settings::settings().connect_changed(Some("pane-title"), move |_, _| {
            if let Some(p) = weak.upgrade() {
                p.update_visibility();
            }
        });
        let id = std::cell::RefCell::new(Some(id));
        pane.connect_destroy(move |_| {
            if let Some(id) = id.borrow_mut().take() {
                crate::settings::settings().disconnect(id);
            }
        });

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
                    p.refresh_title();
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
        imp.sync_line.set(sync_line).ok();
        imp.drop_hint.set(drop_hint).ok();
        imp.context_line.set(context_line).ok();
        imp.dim.set(dim).ok();
        imp.search.set(search).ok();
        imp.badge.set(badge).ok();
        imp.search_entry.set(entry.clone()).ok();
        imp.search_count.set(count).ok();
        imp.search_case.set(case.clone()).ok();
        imp.search_regex.set(regex.clone()).ok();
        imp.search_word.set(word.clone()).ok();

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
        let r3 = refresh.clone();
        word.connect_toggled(move |_| r3());
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
        pane.setup_drag();
        pane
    }

    /// Drag & Drop: Titel (schwebend oder Zeile) ziehen, auf einem anderen Bereich desselben
    /// Reiters ablegen – am Rand daneben einsetzen, in der Mitte tauschen.
    fn setup_drag(&self) {
        let imp = self.imp();
        // am ganzen Titel, nicht nur am Text: In schmalen Bereichen blendet der schwebende
        // Titel den Text aus. Die Knöpfe bleiben klickbar, gezogen wird erst nach einer Bewegung.
        let pill: gtk::Widget = imp.pill.get().unwrap().clone().upcast();
        let row: gtk::Widget = imp.row.get().unwrap().clone().upcast();
        for handle in [pill, row] {
            let source = gtk::DragSource::builder().actions(gtk::gdk::DragAction::MOVE).build();
            let weak = self.downgrade();
            source.connect_prepare(move |_, _, _| {
                let p = weak.upgrade()?;
                let tab = p.tab()?;
                Some(gtk::gdk::ContentProvider::for_value(&drag_key(&tab, p.id()).to_value()))
            });
            let h = handle.downgrade();
            source.connect_drag_begin(move |src, _| {
                if let Some(h) = h.upgrade() {
                    src.set_icon(Some(&gtk::WidgetPaintable::new(Some(&h))), 0, 0);
                }
            });
            handle.add_controller(source);
        }
        let target = gtk::DropTarget::new(String::static_type(), gtk::gdk::DragAction::MOVE);
        let weak = self.downgrade();
        target.connect_motion(move |_, x, y| {
            let Some(p) = weak.upgrade() else { return gtk::gdk::DragAction::empty() };
            p.show_drop_hint(Some(zone_at(&p, x, y)));
            gtk::gdk::DragAction::MOVE
        });
        let weak = self.downgrade();
        target.connect_leave(move |_| {
            if let Some(p) = weak.upgrade() {
                p.show_drop_hint(None);
            }
        });
        let weak = self.downgrade();
        target.connect_drop(move |_, value, x, y| {
            let Some(p) = weak.upgrade() else { return false };
            p.show_drop_hint(None);
            let (Ok(key), Some(tab)) = (value.get::<String>(), p.tab()) else { return false };
            // nur aus demselben Reiter; in andere Reiter über die Reiterleiste (Window)
            let Some((from, source)) = parse_drag_key(&key) else { return false };
            if from != tab_key(&tab) {
                return false;
            }
            tab.move_pane(source, p.id(), zone_at(&p, x, y))
        });
        self.add_controller(target);
    }

    /// Reiter, in dem der Bereich liegt.
    /// Reiter, dem der Bereich gehört (auch während „Rückgängig“, wenn er nicht angezeigt wird).
    pub fn tab(&self) -> Option<crate::tab::Tab> {
        self.imp().owner.borrow().upgrade()
    }

    pub fn set_owner(&self, tab: &crate::tab::Tab) {
        *self.imp().owner.borrow_mut() = tab.downgrade();
    }

    /// Zone als halbtransparente Fläche zeigen: Hälfte an der Seite, Mitte zum Tauschen.
    fn show_drop_hint(&self, zone: Option<DropZone>) {
        let hint = self.imp().drop_hint.get().unwrap();
        let Some(zone) = zone else {
            hint.set_visible(false);
            return;
        };
        let (w, h) = (self.width(), self.height());
        let (halign, valign, width, height) = match zone {
            DropZone::Left => (gtk::Align::Start, gtk::Align::Fill, w / 2, -1),
            DropZone::Right => (gtk::Align::End, gtk::Align::Fill, w / 2, -1),
            DropZone::Top => (gtk::Align::Fill, gtk::Align::Start, -1, h / 2),
            DropZone::Bottom => (gtk::Align::Fill, gtk::Align::End, -1, h / 2),
            DropZone::Center => (gtk::Align::Center, gtk::Align::Center, w / 2, h / 2),
        };
        hint.set_halign(halign);
        hint.set_valign(valign);
        hint.set_size_request(width, height);
        hint.set_visible(true);
    }

    pub fn id(&self) -> u32 {
        self.imp().id.get()
    }

    /// Neue Id, wenn ein Bereich beim Wiederherstellen einen anderen Platz bekommt.
    pub fn set_id(&self, id: u32) {
        self.imp().id.set(id);
    }

    pub fn terminal(&self) -> &Terminal {
        self.imp().terminal.get().expect("Terminal wird im Konstruktor gesetzt")
    }

    pub fn focus_terminal(&self) {
        self.terminal().vte().grab_focus();
    }

    /// Titel im schwebenden Feld, bei root/SSH mit farbigem Chip davor.
    fn refresh_title(&self) {
        self.imp().lock.get().unwrap().set_visible(self.terminal().read_only());
        let t = self.terminal();
        let title = glib::markup_escape_text(&t.title().unwrap_or_else(|| "Shell".into())).to_string();
        let chip = match (t.context(), t.context_accent()) {
            (parkett_core::context::Context::Remote(host), Some((_, bg))) => Some((host, bg)),
            (parkett_core::context::Context::Superuser, Some((_, bg))) => Some(("root".to_string(), bg)),
            _ => None,
        };
        let markup = match &chip {
            Some((text, bg)) => format!(
                "<span background=\"{}\" weight=\"bold\"> {} </span>  {title}",
                bg.hex(),
                glib::markup_escape_text(text)
            ),
            None => title.clone(),
        };
        self.imp().pill_label.get().unwrap().set_markup(&markup);
        self.imp().row_label.get().unwrap().set_markup(&markup);
        let imp = self.imp();
        imp.context_line.get().unwrap().set_visible(chip.is_some());
        imp.context_line.get().unwrap().queue_draw();
        self.update_visibility();
    }

    /// Linie für die gemeinsame Eingabe: Dieser Bereich empfängt, was man in einen anderen tippt.
    pub fn set_sync(&self, on: bool) {
        self.imp().sync_line.get().unwrap().set_visible(on);
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

    /// Titel-Feld und Linien: Titel bei geteilten Reitern oder Kontext (dann mit Chip),
    /// Kontextlinie ersetzt die Fokuslinie.
    pub fn update_visibility(&self) {
        let imp = self.imp();
        let context = self.terminal().context_accent().is_some();
        let searching = imp.search.get().is_some_and(|s| s.is_visible());
        let row_mode = crate::settings::settings().string("pane-title") == "row";
        let show = imp.multi.get() || context;
        let pill = imp.pill.get().unwrap();
        pill.set_visible(show && !row_mode && !searching);
        // Ausgeblendet nimmt er keine Klicks an, sonst lägen unsichtbare Knöpfe über dem Text
        let revealed = imp.hover_top.get() || imp.flash.borrow().is_some();
        pill.set_can_target(revealed);
        if revealed {
            pill.add_css_class("revealed");
        } else {
            pill.remove_css_class("revealed");
        }
        imp.pill_label.get().unwrap().set_visible(!imp.narrow.get());
        let row = imp.row.get().unwrap();
        row.set_visible(show && row_mode);
        if imp.focused.get() && imp.multi.get() {
            row.add_css_class("focused");
        } else {
            row.remove_css_class("focused");
        }
        // In der Zeile zeigt die Einfärbung den Fokus; die Linie nur im schwebenden Modus
        imp.focus_line.get().unwrap().set_visible(imp.multi.get() && imp.focused.get() && !context && !row_mode);
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
        let _ = self.terminal().set_search(None, crate::terminal::SearchQuery::default());
        // schwebenden Titel wiederherstellen, falls der Reiter geteilt ist
        let _ = self.activate_action("win.refresh-panes", None);
        self.focus_terminal();
    }

    fn apply_search(&self) {
        let imp = self.imp();
        let entry = imp.search_entry.get().unwrap();
        let text = entry.text().to_string();
        let query = crate::terminal::SearchQuery {
            regex: imp.search_regex.get().unwrap().is_active(),
            case_sensitive: imp.search_case.get().unwrap().is_active(),
            whole_words: imp.search_word.get().unwrap().is_active(),
        };
        let count = imp.search_count.get().unwrap();
        match self.terminal().set_search(Some(&text), query) {
            Ok(()) => {
                entry.set_tooltip_text(None);
                let found = text.is_empty() || self.terminal().search_step(true);
                set_error(entry, !found);
                set_count(count, self.terminal().search_count(&text, query));
            }
            Err(e) => {
                entry.set_tooltip_text(Some(&trf("Invalid expression: {error}", &[("error", &e.to_string())])));
                set_error(entry, true);
                set_count(count, None);
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
        imp.multi.set(multi);
        if imp.focused.replace(focused) != focused {
            if focused && multi {
                self.flash_title();
            } else if let Some(id) = imp.flash.borrow_mut().take() {
                id.remove();
            }
        }
        self.update_visibility();
        imp.dim.get().unwrap().set_visible(multi && !focused && crate::settings::dim_unfocused());
        if focused {
            self.add_css_class("focused");
        } else {
            self.remove_css_class("focused");
        }
    }
}

/// Höhe des Streifens oben im Bereich, in dem der Zeiger den Titel einblendet (px).
const HOVER_STRIP: f64 = 40.0;
/// So lange bleibt der Titel nach einem Fokuswechsel sichtbar.
const FLASH: std::time::Duration = std::time::Duration::from_millis(1500);

impl Pane {
    /// Titel nach einem Fokuswechsel kurz zeigen – nicht, wenn der Cursor in der obersten
    /// Zeile steht: Dort tippt man gerade, und genau die würde er verdecken.
    fn flash_title(&self) {
        let vte = self.terminal().vte();
        let top = vte.vadjustment().map_or(0.0, |a| a.value());
        if (vte.cursor_position().1 as f64 - top) < 1.0 {
            return;
        }
        let weak = self.downgrade();
        let id = glib::timeout_add_local_once(FLASH, move || {
            if let Some(p) = weak.upgrade() {
                p.imp().flash.borrow_mut().take();
                p.update_visibility();
            }
        });
        if let Some(old) = self.imp().flash.borrow_mut().replace(id) {
            old.remove();
        }
    }
}

/// Trefferzahl neben dem Suchfeld (QA-19); gezählt beim Ändern der Suche, nicht live bei
/// neuer Ausgabe.
fn set_count(label: &gtk::Label, n: Option<usize>) {
    let text = match n {
        None => String::new(),
        Some(n) if n >= crate::terminal::SEARCH_COUNT_MAX => {
            trf("{n}+ matches", &[("n", &crate::terminal::SEARCH_COUNT_MAX.to_string())])
        }
        Some(n) => ntrf("{n} match", "{n} matches", n as u32, &[]),
    };
    label.set_visible(!text.is_empty());
    label.set_label(&text);
}

/// Inhalt beim Ziehen: Reiter (Adresse) und Bereich, damit nur im selben Reiter abgelegt wird.
fn drag_key(tab: &crate::tab::Tab, pane: u32) -> String {
    format!("{}:{pane}", tab_key(tab))
}

/// (Reiter-Schlüssel, Bereich) aus dem Inhalt beim Ziehen.
pub fn parse_drag_key(key: &str) -> Option<(&str, u32)> {
    let (from, id) = key.rsplit_once(':')?;
    from.starts_with("parkett-pane:").then_some(())?;
    Some((from, id.parse().ok()?))
}

pub fn tab_key(tab: &crate::tab::Tab) -> String {
    format!("parkett-pane:{:p}", tab.as_ptr())
}

fn zone_at(p: &Pane, x: f64, y: f64) -> DropZone {
    parkett_core::layout::drop_zone(x as f32, y as f32, p.width() as f32, p.height() as f32)
}

fn set_error(entry: &gtk::SearchEntry, error: bool) {
    if error {
        entry.add_css_class("error");
    } else {
        entry.remove_css_class("error");
    }
}
