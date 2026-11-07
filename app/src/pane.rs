// SPDX-License-Identifier: GPL-3.0-or-later
//! A pane in the split tree: the terminal plus floating title, focus line and dimming.

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;

use crate::i18n::{gettext, ntrf, trf, N_};
use crate::shortcuts::live_tooltip;
use crate::terminal::{Launch, Terminal};
use parkett_core::context::Kind;
use parkett_core::layout::DropZone;

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use super::*;

    #[derive(Default)]
    pub struct Pane {
        pub id: Cell<u32>,
        /// Tab the pane belongs to; changes when it is dragged into another tab
        pub owner: RefCell<glib::WeakRef<crate::tab::Tab>>,
        pub terminal: OnceCell<Terminal>,
        pub pill: OnceCell<gtk::Box>,
        pub pill_label: OnceCell<gtk::Label>,
        pub focus_line: OnceCell<gtk::Box>,
        pub sync_line: OnceCell<gtk::Box>,
        /// Zone while dropping a dragged pane
        pub drop_hint: OnceCell<gtk::Box>,
        pub context_line: OnceCell<gtk::DrawingArea>,
        /// Frame in the context color (root)
        pub frame: OnceCell<gtk::DrawingArea>,
        /// Badge with the context (`root`, `ssh bender`) at the top right, always visible
        pub context_badge: OnceCell<gtk::Label>,
        /// CSS class of the badge's color (`style::color_class`)
        pub badge_class: RefCell<Option<String>>,
        pub multi: Cell<bool>,
        pub narrow: Cell<bool>,
        pub row: OnceCell<gtk::Box>,
        pub row_label: OnceCell<gtk::Label>,
        pub focused: Cell<bool>,
        pub dim: OnceCell<gtk::DrawingArea>,
        pub search: OnceCell<gtk::Revealer>,
        pub badge: OnceCell<gtk::Label>,
        /// Lock at the top left when read only
        pub lock: OnceCell<gtk::Image>,
        pub search_entry: OnceCell<gtk::SearchEntry>,
        pub search_count: OnceCell<gtk::Label>,
        pub search_case: OnceCell<gtk::ToggleButton>,
        pub search_regex: OnceCell<gtk::ToggleButton>,
        pub search_word: OnceCell<gtk::ToggleButton>,
        /// Pointer in the top strip (design V2: title visible only then)
        pub hover_top: Cell<bool>,
        /// Visible briefly after a focus change; source of the hide timer
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

        // Dimming of inactive panes: semi-transparent area in the background color of the
        // color scheme (CSS does not know the palette, hence drawn by hand)
        let dim = gtk::DrawingArea::builder().can_target(false).visible(false).build();
        {
            let t = terminal.downgrade();
            dim.set_draw_func(move |_, cr, w, h| {
                let Some(t) = t.upgrade() else { return };
                let (r, g, b) = t.background().to_f64();
                cr.set_source_rgba(r, g, b, t.dim_alpha());
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

        // Focus: 2 px accent line at the top – never dimming alone (accessibility)
        let focus_line = gtk::Box::builder()
            .css_classes(["parkett-focus-line"])
            .valign(gtk::Align::Start)
            .height_request(2)
            .can_target(false)
            .visible(false)
            .build();
        overlay.add_overlay(&focus_line);
        // shared input: strong line at the top, above the focus and context lines
        let sync_line = gtk::Box::builder()
            .css_classes(["parkett-sync-line"])
            .valign(gtk::Align::Start)
            .height_request(4)
            .can_target(false)
            .visible(false)
            .tooltip_text(gettext("Shared input"))
            .build();

        // Context color, graded by risk (approach C): SSH and other users a 3 px line at the
        // top, containers a dotted one, root a frame around the whole pane
        let context_line = gtk::DrawingArea::builder()
            .valign(gtk::Align::Start)
            .height_request(CONTEXT_LINE)
            .can_target(false)
            .visible(false)
            .build();
        {
            let t = terminal.downgrade();
            context_line.set_draw_func(move |_, cr, w, h| {
                let Some(mark) = t.upgrade().and_then(|t| t.context_mark()) else { return };
                let (r, g, b) = mark.color.to_f64();
                cr.set_source_rgb(r, g, b);
                if mark.kind == Kind::Container {
                    // shape besides color: dashes, not only purple
                    let mut x = 0.0;
                    while x < w as f64 {
                        cr.rectangle(x, 0.0, 6.0, h as f64);
                        x += 10.0;
                    }
                } else {
                    cr.rectangle(0.0, 0.0, w as f64, h as f64);
                }
                let _ = cr.fill();
            });
        }
        overlay.add_overlay(&context_line);
        let frame = gtk::DrawingArea::builder().can_target(false).visible(false).build();
        {
            let t = terminal.downgrade();
            frame.set_draw_func(move |_, cr, w, h| {
                let Some(mark) = t.upgrade().and_then(|t| t.context_mark()) else { return };
                let (r, g, b) = mark.color.to_f64();
                cr.set_source_rgb(r, g, b);
                let half = FRAME / 2.0;
                cr.set_line_width(FRAME);
                cr.rectangle(half, half, w as f64 - FRAME, h as f64 - FRAME);
                let _ = cr.stroke();
            });
        }
        overlay.add_overlay(&frame);
        // Badge in text, not only color (accessibility): at the top right, where the floating
        // title appears – while that is shown it carries the chip itself
        let context_badge = gtk::Label::builder()
            .css_classes(["parkett-context-badge"])
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .margin_end(8)
            .can_target(false)
            .visible(false)
            .build();
        overlay.add_overlay(&context_badge);
        // the badge gives way when the cursor gets to the top line
        let weak = pane.downgrade();
        terminal.connect_cursor_moved(move || {
            if let Some(p) = weak.upgrade() {
                p.place_badge();
            }
        });
        overlay.add_overlay(&sync_line);
        let drop_hint = gtk::Box::builder().css_classes(["parkett-drop-hint"]).can_target(false).visible(false).build();
        overlay.add_overlay(&drop_hint);

        // Watch the pane's width: narrow panes hide the text of the floating
        // title so it does not cover the prompt line (QA). Invisible drawing area,
        // because AdwBin has a layout manager and size_allocate cannot be overridden.
        let probe = gtk::DrawingArea::builder().can_target(false).opacity(0.0).build();
        let weak = pane.downgrade();
        probe.connect_resize(move |_, width, _| {
            let Some(p) = weak.upgrade() else { return };
            // VTE rewraps on resize without reporting the cursor move: place the badge again
            let w2 = p.downgrade();
            glib::idle_add_local_once(move || {
                if let Some(p) = w2.upgrade() {
                    p.place_badge();
                }
            });
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

        // Floating title, only in split tabs (decision A+: floating). Design V2:
        // hidden by default so it does not cover a terminal line (QA-02); visible
        // briefly after a focus change and while the pointer is in the top strip.
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

        // Number in layout mode (centered, large)
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

        // Search bar at the top right (replaces the floating title there while it is open)
        let entry = gtk::SearchEntry::builder().placeholder_text(gettext("Search in terminal")).width_chars(22).build();
        // fixed width: otherwise the search field jumps with every new count
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

        // Title row (setting "Pane titles in split tabs: As a row") above the terminal
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
        // Apply switching floating ⇄ row immediately
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

        // Buttons act on this pane: focus it first, then run the window action
        for (button, action) in [(&zoom, "win.toggle-zoom"), (&close, "win.close-pane")] {
            let weak = pane.downgrade();
            button.connect_clicked(move |_| {
                if let Some(p) = weak.upgrade() {
                    p.focus_terminal();
                    let _ = p.activate_action(action, None);
                }
            });
        }

        // Right-click focuses first, so the context menu applies to this pane
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

        // Minimum size 20×5 cells: dividers cannot be pushed further
        let set_min = {
            let weak = pane.downgrade();
            move |t: &Terminal| {
                if let Some(p) = weak.upgrade() {
                    let (w, h) = t.char_size();
                    p.set_size_request(
                        (w * crate::terminal::MIN_COLUMNS) as i32,
                        (h * crate::terminal::MIN_ROWS) as i32,
                    );
                }
            }
        };
        set_min(&terminal);
        {
            let t = terminal.downgrade();
            terminal.connect_char_size_changed(move || {
                if let Some(t) = t.upgrade() {
                    set_min(&t);
                }
            });
        }

        imp.terminal.set(terminal).ok();
        imp.pill.set(pill).ok();
        imp.pill_label.set(pill_label).ok();
        imp.focus_line.set(focus_line).ok();
        imp.sync_line.set(sync_line).ok();
        imp.drop_hint.set(drop_hint).ok();
        imp.context_line.set(context_line).ok();
        imp.frame.set(frame).ok();
        imp.context_badge.set(context_badge).ok();
        imp.dim.set(dim).ok();
        imp.search.set(search).ok();
        imp.badge.set(badge).ok();
        imp.search_entry.set(entry.clone()).ok();
        imp.search_count.set(count).ok();
        imp.search_case.set(case.clone()).ok();
        imp.search_regex.set(regex.clone()).ok();
        imp.search_word.set(word.clone()).ok();

        // Search: set again on every change and look for the first match from the end
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
        // Shift+Enter = match below
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

    /// Drag and drop: drag the title (floating or row), drop it on another pane of the same
    /// tab – at the edge to insert beside it, in the middle to swap.
    fn setup_drag(&self) {
        let imp = self.imp();
        // on the whole title, not just the text: in narrow panes the floating title
        // hides the text. The buttons stay clickable; dragging starts only after a movement.
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
            // only from the same tab; into other tabs via the tab bar (Window)
            let Some((from, source)) = parse_drag_key(&key) else { return false };
            if from != tab_key(&tab) {
                return false;
            }
            tab.move_pane(source, p.id(), zone_at(&p, x, y))
        });
        self.add_controller(target);
    }

    /// Tab the pane lives in.
    /// Tab the pane belongs to (also during *Undo*, when it is not shown).
    pub fn tab(&self) -> Option<crate::tab::Tab> {
        self.imp().owner.borrow().upgrade()
    }

    pub fn set_owner(&self, tab: &crate::tab::Tab) {
        *self.imp().owner.borrow_mut() = tab.downgrade();
    }

    /// Show the zone as a semi-transparent area: half at the side, middle to swap.
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

    /// New id when a pane gets a different place during restore.
    pub fn set_id(&self, id: u32) {
        self.imp().id.set(id);
    }

    pub fn terminal(&self) -> &Terminal {
        self.imp().terminal.get().expect("Terminal wird im Konstruktor gesetzt")
    }

    pub fn focus_terminal(&self) {
        self.terminal().grab_focus();
    }

    /// Title in the floating field, with a colored chip in front for a context; marking
    /// (line, frame, badge) by the kind of context.
    fn refresh_title(&self) {
        self.imp().lock.get().unwrap().set_visible(self.terminal().read_only());
        let t = self.terminal();
        let title = glib::markup_escape_text(&t.title().unwrap_or_else(|| "Shell".into())).to_string();
        let mark = t.context_mark();
        let markup = match &mark {
            Some(m) => {
                let black = parkett_core::palette::Rgb { r: 0, g: 0, b: 0 };
                let white = parkett_core::palette::Rgb { r: 255, g: 255, b: 255 };
                let fg = if m.color.contrast(&black) > m.color.contrast(&white) { "#000000" } else { "#ffffff" };
                format!(
                    "<span background=\"{}\" foreground=\"{fg}\" weight=\"bold\"> {} </span>  {title}",
                    m.color.hex(),
                    glib::markup_escape_text(&m.label)
                )
            }
            None => title.clone(),
        };
        let imp = self.imp();
        imp.pill_label.get().unwrap().set_markup(&markup);
        imp.row_label.get().unwrap().set_markup(&markup);
        let root = mark.as_ref().is_some_and(|m| m.kind == Kind::Superuser);
        let line = imp.context_line.get().unwrap();
        line.set_visible(mark.is_some() && !root);
        line.queue_draw();
        let frame = imp.frame.get().unwrap();
        frame.set_visible(root);
        frame.queue_draw();
        let badge = imp.context_badge.get().unwrap();
        if let Some(old) = imp.badge_class.borrow_mut().take() {
            badge.remove_css_class(&old);
        }
        if let Some(m) = &mark {
            let class = crate::style::color_class(m.color);
            badge.add_css_class(&class);
            *imp.badge_class.borrow_mut() = Some(class);
            badge.set_label(&m.label);
        }
        // Focus line below the context line or inside the frame
        let focus = imp.focus_line.get().unwrap();
        let (top, side) = match &mark {
            Some(_) if root => (FRAME as i32, FRAME as i32),
            Some(_) => (CONTEXT_LINE, 0),
            None => (0, 0),
        };
        focus.set_margin_top(top);
        focus.set_margin_start(side);
        focus.set_margin_end(side);
        self.update_visibility();
    }

    /// Badge at the top right; at the bottom right while the cursor is in the top line, so it
    /// does not cover what is being typed (like the floating title, QA-02).
    fn place_badge(&self) {
        let imp = self.imp();
        let badge = imp.context_badge.get().unwrap();
        let cursor_on_top = self.terminal().cursor_on_top_row();
        let offset = if imp.frame.get().unwrap().is_visible() { FRAME as i32 } else { CONTEXT_LINE };
        if cursor_on_top {
            badge.set_valign(gtk::Align::End);
            badge.set_margin_top(0);
            badge.set_margin_bottom(8 + offset);
        } else {
            badge.set_valign(gtk::Align::Start);
            badge.set_margin_top(6 + offset);
            badge.set_margin_bottom(0);
        }
    }

    /// Line for shared input: this pane receives what is typed into another one.
    pub fn set_sync(&self, on: bool) {
        self.imp().sync_line.get().unwrap().set_visible(on);
    }

    /// Number in layout mode; `None` hides it.
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

    /// Title field, badge and lines: title in split tabs or with context (then with chip); the
    /// focus line stays visible next to the context marking.
    pub fn update_visibility(&self) {
        let imp = self.imp();
        let context = self.terminal().context_mark().is_some();
        let searching = imp.search.get().is_some_and(|s| s.is_visible());
        let row_mode = crate::settings::settings().string("pane-title") == "row";
        let show = imp.multi.get() || context;
        let pill = imp.pill.get().unwrap();
        pill.set_visible(show && !row_mode && !searching);
        // When hidden it takes no clicks, otherwise invisible buttons would lie over the text
        let revealed = imp.hover_top.get() || imp.flash.borrow().is_some();
        pill.set_can_target(revealed);
        if revealed {
            pill.add_css_class("revealed");
        } else {
            pill.remove_css_class("revealed");
        }
        imp.pill_label.get().unwrap().set_visible(!imp.narrow.get());
        // the badge yields to the revealed title (it shows the chip) and the search; in the
        // title row the chip is in the row
        let badge_shown = !(revealed && pill.is_visible()) && !searching && !row_mode;
        imp.context_badge.get().unwrap().set_visible(context && badge_shown);
        self.place_badge();
        let row = imp.row.get().unwrap();
        row.set_visible(show && row_mode);
        if imp.focused.get() && imp.multi.get() {
            row.add_css_class("focused");
        } else {
            row.remove_css_class("focused");
        }
        // In the row the tint shows focus; the line only in floating mode
        imp.focus_line.get().unwrap().set_visible(imp.multi.get() && imp.focused.get() && !row_mode);
    }

    pub fn search_visible(&self) -> bool {
        self.imp().search.get().unwrap().is_visible()
    }

    /// Opens the search; selected text becomes the search term.
    pub fn show_search(&self) {
        let imp = self.imp();
        let entry = imp.search_entry.get().unwrap();
        if let Some(sel) = self.terminal().selected_text() {
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
        // restore the floating title if the tab is split
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

    /// Next match; opens the search if it is closed.
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

    /// `multi`: tab has more than one visible pane.
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

/// Height of the context line at the top (px).
const CONTEXT_LINE: i32 = 3;
/// Width of the root frame (px).
const FRAME: f64 = 2.0;
/// Height of the strip at the top of the pane where the pointer reveals the title (px).
const HOVER_STRIP: f64 = 40.0;
/// How long the title stays visible after a focus change.
const FLASH: std::time::Duration = std::time::Duration::from_millis(1500);

impl Pane {
    /// Show the title briefly after a focus change – not when the cursor is in the top
    /// row: that is where one is typing, and exactly that row would be covered.
    fn flash_title(&self) {
        if self.terminal().cursor_on_top_row() {
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

/// Match count next to the search field (QA-19); counted when the search changes, not live on
/// new output.
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

/// Drag content: tab (address) and pane, so it can only be dropped in the same tab.
fn drag_key(tab: &crate::tab::Tab, pane: u32) -> String {
    format!("{}:{pane}", tab_key(tab))
}

/// (Tab key, pane) from the drag content.
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
