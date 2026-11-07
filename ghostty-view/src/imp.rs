use crate::a11y::{self, Granularity};
use crate::links::{Detector, Link, RowText, Span};
use crate::search::{self, Match};
use ghostty_vt::edit::PasteResult;
use ghostty_vt::input::{self, KeyEncoder, KeyEvent, MouseAction, MouseEncoder, MouseEvent, MouseGeometry};
use ghostty_vt::render::{self, CursorShape, Frame, RenderState, Rgb};
use ghostty_vt::{Prompt, Terminal};
use gtk::gdk;
use gtk::glib;
use gtk::graphene;
use gtk::pango;
use gtk::prelude::*;
use gtk::subclass::accessible_text::AccessibleTextImpl;
use gtk::subclass::prelude::*;
use parkett_core::profile::{CursorBlink, CursorShape as CursorKind};
use parkett_core::pty::{Pty, SpawnOptions};
use std::cell::{Cell, RefCell};
use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::rc::Rc;
use std::sync::OnceLock;

type ExitCallback = Box<dyn Fn(i32)>;
type Callback = Box<dyn Fn()>;
type ShellCallback = Box<dyn Fn(crate::ShellEvent)>;
type TextCallback = Box<dyn Fn(&str)>;
type LinkCallback = Box<dyn Fn(&Link)>;

enum Event {
    Bell,
    Prompt(Prompt),
    /// VTE's own marks (`OSC 666`), sent by the distribution's `vte.sh`.
    Mark(crate::ShellEvent),
}

#[derive(Default)]
struct Search {
    regex: Option<glib::Regex>,
    matches: Vec<Match>,
    current: Option<usize>,
}

pub(crate) struct State {
    pub term: Terminal,
    render: RenderState,
    keys: KeyEncoder,
    mouse: MouseEncoder,
    pty: Option<Pty>,
    /// Replies the terminal produced while parsing; sent to the PTY after the write.
    outbox: Rc<RefCell<Vec<u8>>>,
    /// Bell and prompt marks seen while parsing; delivered after the write, outside the borrow.
    events: Rc<RefCell<Vec<Event>>>,
    last_cursor: (u16, u16),
    last_pwd: Option<String>,
    watch: Option<glib::SourceId>,
    last_title: Option<String>,
    cols: u16,
    rows: u16,
}

pub struct GhosttyView {
    pub(crate) state: RefCell<Option<State>>,
    font: RefCell<pango::FontDescription>,
    cell: Cell<(f64, f64)>,
    /// Space around the text (the app's CSS gives VTE `padding: 6px 8px` too).
    pad: Cell<(f64, f64)>,
    focused: Cell<bool>,
    im: gtk::IMMulticontext,
    /// Cell where the current drag selection started.
    anchor: Cell<Option<(u16, u16)>>,
    /// Mouse buttons currently held (bit per button number).
    buttons: Cell<u32>,
    /// The running press goes to the program, not to the selection.
    reporting: Cell<bool>,
    pub(crate) unsafe_paste_cbs: RefCell<Vec<TextCallback>>,
    detector: RefCell<Detector>,
    hover: RefCell<Option<Span>>,
    last_frame: RefCell<Option<Rc<Frame>>>,
    pub(crate) link_cbs: RefCell<Vec<LinkCallback>>,
    search: RefCell<Search>,
    pub(crate) title_cbs: RefCell<Vec<Box<dyn Fn()>>>,
    pub(crate) exit_cbs: RefCell<Vec<ExitCallback>>,
    pub(crate) bell_cbs: RefCell<Vec<Callback>>,
    pub(crate) commit_cbs: RefCell<Vec<TextCallback>>,
    pub(crate) selection_cbs: RefCell<Vec<Callback>>,
    pub(crate) contents_cbs: RefCell<Vec<Callback>>,
    pub(crate) cursor_cbs: RefCell<Vec<Callback>>,
    pub(crate) char_cbs: RefCell<Vec<Callback>>,
    pub(crate) shell_cbs: RefCell<Vec<ShellCallback>>,
    look: RefCell<Option<crate::Look>>,
    bg_alpha: Cell<f32>,
    highlight: Cell<Option<Rgb>>,
    line_scale: Cell<f64>,
    font_scale: Cell<f64>,
    text_pad: Cell<f64>,
    scroll_on_output: Cell<bool>,
    scroll_on_keystroke: Cell<bool>,
    audible_bell: Cell<bool>,
    input_enabled: Cell<bool>,
    menu: RefCell<Option<gtk::PopoverMenu>>,
    /// Scroll position for a scrollbar next to the widget: value = first visible row.
    /// Set by the `GtkScrolledWindow` around the widget (GtkScrollable); our own until then.
    vadj: RefCell<gtk::Adjustment>,
    hadj: RefCell<Option<gtk::Adjustment>>,
    vadj_handler: RefCell<Option<glib::SignalHandlerId>>,
    hpolicy: Cell<gtk::ScrollablePolicy>,
    vpolicy: Cell<gtk::ScrollablePolicy>,
    adj_updating: Cell<bool>,
    /// What the screen reader was last told (to send only the difference)
    a11y_prev: RefCell<a11y::Snapshot>,
}

impl Default for GhosttyView {
    fn default() -> Self {
        Self {
            state: RefCell::new(None),
            font: RefCell::new(pango::FontDescription::from_string("Monospace 11")),
            cell: Cell::new((9.0, 18.0)),
            pad: Cell::new((8.0, 6.0)),
            focused: Cell::new(false),
            im: gtk::IMMulticontext::new(),
            anchor: Cell::new(None),
            buttons: Cell::new(0),
            reporting: Cell::new(false),
            unsafe_paste_cbs: RefCell::new(Vec::new()),
            detector: RefCell::new(Detector::default()),
            hover: RefCell::new(None),
            last_frame: RefCell::new(None),
            link_cbs: RefCell::new(Vec::new()),
            search: RefCell::new(Search::default()),
            title_cbs: RefCell::new(Vec::new()),
            exit_cbs: RefCell::new(Vec::new()),
            bell_cbs: RefCell::new(Vec::new()),
            commit_cbs: RefCell::new(Vec::new()),
            selection_cbs: RefCell::new(Vec::new()),
            contents_cbs: RefCell::new(Vec::new()),
            cursor_cbs: RefCell::new(Vec::new()),
            char_cbs: RefCell::new(Vec::new()),
            shell_cbs: RefCell::new(Vec::new()),
            look: RefCell::new(None),
            bg_alpha: Cell::new(1.0),
            highlight: Cell::new(None),
            line_scale: Cell::new(1.0),
            font_scale: Cell::new(1.0),
            text_pad: Cell::new(0.0),
            scroll_on_output: Cell::new(false),
            scroll_on_keystroke: Cell::new(true),
            audible_bell: Cell::new(false),
            input_enabled: Cell::new(true),
            menu: RefCell::new(None),
            vadj: RefCell::new(gtk::Adjustment::new(0.0, 0.0, 1.0, 1.0, 1.0, 1.0)),
            hadj: RefCell::new(None),
            vadj_handler: RefCell::new(None),
            hpolicy: Cell::new(gtk::ScrollablePolicy::Minimum),
            vpolicy: Cell::new(gtk::ScrollablePolicy::Minimum),
            adj_updating: Cell::new(false),
            a11y_prev: RefCell::new(a11y::Snapshot::default()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for GhosttyView {
    const NAME: &'static str = "ParkettGhosttyView";
    type Type = super::GhosttyView;
    type ParentType = gtk::Widget;
    type Interfaces = (gtk::Scrollable, gtk::AccessibleText);

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("parkett-terminal");
        klass.set_accessible_role(gtk::AccessibleRole::Terminal);
    }
}

impl ObjectImpl for GhosttyView {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPS: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
        PROPS.get_or_init(|| {
            ["hadjustment", "vadjustment", "hscroll-policy", "vscroll-policy"]
                .into_iter()
                .map(glib::ParamSpecOverride::for_interface::<gtk::Scrollable>)
                .collect()
        })
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "hadjustment" => *self.hadj.borrow_mut() = value.get().ok(),
            "vadjustment" => {
                let adj: Option<gtk::Adjustment> = value.get().ok().flatten();
                self.set_vadjustment(adj);
            }
            "hscroll-policy" => self.hpolicy.set(value.get().unwrap_or(gtk::ScrollablePolicy::Minimum)),
            "vscroll-policy" => self.vpolicy.set(value.get().unwrap_or(gtk::ScrollablePolicy::Minimum)),
            _ => {}
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "hadjustment" => self.hadj.borrow().to_value(),
            "vadjustment" => self.vadj.borrow().to_value(),
            "hscroll-policy" => self.hpolicy.get().to_value(),
            "vscroll-policy" => self.vpolicy.get().to_value(),
            _ => glib::Value::from_type(glib::Type::INVALID),
        }
    }

    fn constructed(&self) {
        self.parent_constructed();
        let obj = self.obj();
        obj.set_focusable(true);
        obj.set_can_focus(true);
        obj.set_hexpand(true);
        obj.set_vexpand(true);
        obj.set_cursor_from_name(Some("text"));
        obj.update_property(&[gtk::accessible::Property::Label("Terminal")]);
        self.im.set_client_widget(Some(&*obj));
        self.connect_vadjustment();
        self.update_metrics();

        let keys = gtk::EventControllerKey::new();
        keys.set_im_context(Some(&self.im));
        let w = obj.downgrade();
        keys.connect_key_pressed(move |c, keyval, code, state| {
            let Some(v) = w.upgrade() else { return glib::Propagation::Proceed };
            v.imp().key(c, keyval, code, state, true)
        });
        let w = obj.downgrade();
        keys.connect_key_released(move |c, keyval, code, state| {
            if let Some(v) = w.upgrade() {
                v.imp().key(c, keyval, code, state, false);
            }
        });
        obj.add_controller(keys);

        let w = obj.downgrade();
        self.im.connect_commit(move |_, text| {
            if let Some(v) = w.upgrade() {
                v.imp().typed(text.as_bytes());
            }
        });

        let focus = gtk::EventControllerFocus::new();
        let w = obj.downgrade();
        focus.connect_enter(move |_| {
            if let Some(v) = w.upgrade() {
                v.imp().focused.set(true);
                v.imp().im.focus_in();
                v.queue_draw();
            }
        });
        let w = obj.downgrade();
        focus.connect_leave(move |_| {
            if let Some(v) = w.upgrade() {
                v.imp().focused.set(false);
                v.imp().im.focus_out();
                v.queue_draw();
            }
        });
        obj.add_controller(focus);

        let click = gtk::GestureClick::new();
        click.set_button(0);
        let w = obj.downgrade();
        click.connect_pressed(move |g, n, x, y| {
            if let Some(v) = w.upgrade() {
                v.grab_focus();
                v.imp().mouse_button(g.current_button(), g.current_event_state(), n, x, y, true);
            }
        });
        let w = obj.downgrade();
        click.connect_released(move |g, n, x, y| {
            if let Some(v) = w.upgrade() {
                v.imp().mouse_button(g.current_button(), g.current_event_state(), n, x, y, false);
            }
        });
        obj.add_controller(click);

        let motion = gtk::EventControllerMotion::new();
        let w = obj.downgrade();
        motion.connect_motion(move |c, x, y| {
            if let Some(v) = w.upgrade() {
                v.imp().mouse_motion(c.current_event_state(), x, y);
            }
        });
        obj.add_controller(motion);

        let wheel = gtk::EventControllerScroll::new(
            gtk::EventControllerScrollFlags::VERTICAL | gtk::EventControllerScrollFlags::DISCRETE,
        );
        let w = obj.downgrade();
        wheel.connect_scroll(move |c, _, dy| {
            if let Some(v) = w.upgrade() {
                v.imp().wheel(c.current_event_state(), dy);
            }
            glib::Propagation::Stop
        });
        obj.add_controller(wheel);
    }

    fn dispose(&self) {
        if let Some(m) = self.menu.borrow_mut().take() {
            m.unparent();
        }
        if let Some(mut s) = self.state.borrow_mut().take() {
            if let Some(id) = s.watch.take() {
                id.remove();
            }
        }
    }
}

impl ScrollableImpl for GhosttyView {}

impl AccessibleTextImpl for GhosttyView {
    fn contents(&self, start: u32, end: u32) -> Option<glib::Bytes> {
        Some(glib::Bytes::from_owned(self.a11y_snapshot().contents(start, end).into_bytes()))
    }

    fn contents_at(&self, offset: u32, g: gtk::AccessibleTextGranularity) -> Option<(u32, u32, glib::Bytes)> {
        let g = match g {
            gtk::AccessibleTextGranularity::Character => Granularity::Character,
            gtk::AccessibleTextGranularity::Word => Granularity::Word,
            _ => Granularity::Line,
        };
        let (a, b, text) = self.a11y_snapshot().contents_at(offset, g);
        Some((a, b, glib::Bytes::from_owned(text.into_bytes())))
    }

    fn caret_position(&self) -> u32 {
        self.a11y_snapshot().caret()
    }

    fn selection(&self) -> Vec<gtk::AccessibleTextRange> {
        self.a11y_snapshot()
            .selection()
            .map(|(a, b)| vec![gtk::AccessibleTextRange::new(a as usize, (b - a) as usize)])
            .unwrap_or_default()
    }

    fn attributes(&self, _offset: u32) -> Vec<(gtk::AccessibleTextRange, glib::GString, glib::GString)> {
        Vec::new()
    }

    fn default_attributes(&self) -> Vec<(glib::GString, glib::GString)> {
        Vec::new()
    }

    fn extents(&self, start: u32, end: u32) -> Option<graphene::Rect> {
        let snap = self.a11y_snapshot();
        let (cw, ch) = self.cell.get();
        let (px, py) = self.pad.get();
        let (r0, c0) = snap.locate(start);
        let (r1, c1) = snap.locate(end.max(start + 1) - 1);
        let (x0, x1) = if r0 == r1 {
            (c0 as f64, c1 as f64 + 1.0)
        } else {
            (0.0, self.with_state(|s| s.cols).unwrap_or(1) as f64)
        };
        Some(graphene::Rect::new(
            (px + x0 * cw) as f32,
            (py + r0 as f64 * ch) as f32,
            ((x1 - x0) * cw) as f32,
            ((r1 - r0 + 1) as f64 * ch) as f32,
        ))
    }

    fn offset(&self, point: &graphene::Point) -> Option<u32> {
        let (x, y) = self.cell_at(point.x() as f64, point.y() as f64);
        Some(self.a11y_snapshot().offset_at(y as usize, x))
    }
}

impl WidgetImpl for GhosttyView {
    fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        let (cw, ch) = self.cell.get();
        let (px, py) = self.pad.get();
        if orientation == gtk::Orientation::Horizontal {
            ((cw * 10.0 + 2.0 * px) as i32, (cw * 80.0 + 2.0 * px) as i32, -1, -1)
        } else {
            ((ch * 3.0 + 2.0 * py) as i32, (ch * 24.0 + 2.0 * py) as i32, -1, -1)
        }
    }

    fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
        let (cw, ch) = self.cell.get();
        let (px, py) = self.pad.get();
        let cols = (((width as f64 - 2.0 * px) / cw).floor().max(1.0) as u16).max(1);
        let rows = (((height as f64 - 2.0 * py) / ch).floor().max(1.0) as u16).max(1);
        self.with_state_mut(|s| {
            if (cols, rows) != (s.cols, s.rows) {
                s.cols = cols;
                s.rows = rows;
                let _ = s.term.resize(cols, rows, cw as u32, ch as u32);
                if let Some(p) = &s.pty {
                    let _ = p.resize(cols, rows, width.clamp(0, 65535) as u16, height.clamp(0, 65535) as u16);
                }
            }
        });
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        self.sync_adjustment();
        let obj = self.obj();
        let (w, h) = (obj.width() as f32, obj.height() as f32);
        let frame = {
            let mut guard = self.state.borrow_mut();
            let Some(s) = guard.as_mut() else { return };
            match s.render.frame(&s.term) {
                Ok(f) => f,
                Err(_) => return,
            }
        };
        let frame = Rc::new(frame);
        *self.last_frame.borrow_mut() = Some(frame.clone());
        self.notify_a11y(&frame);
        let offset = self.with_state(|s| s.term.scrollbar().offset).unwrap_or(0);
        snapshot.append_color(&rgba(frame.bg, self.bg_alpha.get()), &graphene::Rect::new(0.0, 0.0, w, h));
        let (px, py) = self.pad.get();
        snapshot.save();
        snapshot.translate(&graphene::Point::new(px as f32, py as f32));
        self.draw_frame(snapshot, &frame, offset);
        snapshot.restore();
    }

    fn grab_focus(&self) -> bool {
        self.parent_grab_focus()
    }
}

fn rgba(c: Rgb, a: f32) -> gdk::RGBA {
    gdk::RGBA::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, a)
}

impl GhosttyView {
    pub(crate) fn with_state<R>(&self, f: impl FnOnce(&State) -> R) -> Option<R> {
        self.state.borrow().as_ref().map(f)
    }

    fn with_state_mut<R>(&self, f: impl FnOnce(&mut State) -> R) -> Option<R> {
        self.state.borrow_mut().as_mut().map(f)
    }

    pub(crate) fn cell_size(&self) -> (f64, f64) {
        self.cell.get()
    }

    pub(crate) fn set_font(&self, font: &str) {
        *self.font.borrow_mut() = pango::FontDescription::from_string(font);
        self.update_metrics();
        self.obj().queue_resize();
    }

    /// The font at the current zoom.
    fn scaled_font(&self) -> pango::FontDescription {
        let mut f = self.font.borrow().clone();
        let scale = self.font_scale.get();
        if (scale - 1.0).abs() > f64::EPSILON && f.size() > 0 {
            f.set_size((f.size() as f64 * scale).round() as i32);
        }
        f
    }

    pub(crate) fn font_scale(&self) -> f64 {
        self.font_scale.get()
    }

    pub(crate) fn set_font_scale(&self, scale: f64) {
        self.font_scale.set(scale.clamp(0.25, 5.0));
        self.update_metrics();
        self.obj().queue_resize();
    }

    fn update_metrics(&self) {
        // Measure real text: the advance of ten digits, so the grid matches what Pango draws.
        let obj = self.obj();
        let layout = obj.create_pango_layout(Some("0123456789"));
        layout.set_font_description(Some(&self.scaled_font()));
        let (_, logical) = layout.extents();
        let s = pango::SCALE as f64;
        let cw = (logical.width() as f64 / s / 10.0).max(1.0);
        // Row height like VTE: ascent + descent of the font, not less than one laid-out line
        let m = obj.pango_context().metrics(Some(&self.scaled_font()), None);
        let font_h = (logical.height() as f64 / s).max((m.ascent() + m.descent()) as f64 / s).ceil().max(1.0);
        let ch = (font_h * self.line_scale.get()).round().max(1.0);
        self.text_pad.set((ch - font_h) / 2.0);
        let old = self.cell.replace((cw, ch));
        if old != (cw, ch) {
            for cb in self.char_cbs.borrow().iter() {
                cb();
            }
        }
    }

    pub(crate) fn set_look(&self, look: crate::Look) {
        *self.font.borrow_mut() = pango::FontDescription::from_string(&look.font);
        self.line_scale.set(look.line_spacing.max(0.5));
        self.bg_alpha.set(look.background.alpha());
        self.highlight.set(Some(Rgb {
            r: (look.highlight.red() * 255.0).round() as u8,
            g: (look.highlight.green() * 255.0).round() as u8,
            b: (look.highlight.blue() * 255.0).round() as u8,
        }));
        self.scroll_on_output.set(look.scroll_on_output);
        self.scroll_on_keystroke.set(look.scroll_on_keystroke);
        self.audible_bell.set(look.audible_bell);
        *self.look.borrow_mut() = Some(look);
        self.apply_look_to_terminal();
        self.update_metrics();
        self.obj().queue_resize();
        self.obj().queue_draw();
    }

    /// Colors, cursor and scrollback go into the terminal state, so programs that query them
    /// (OSC 10/11, `COLORFGBG`-style checks) see the same values the user sees.
    fn apply_look_to_terminal(&self) {
        let look = self.look.borrow();
        let Some(l) = look.as_ref() else { return };
        let b = |c: &gdk::RGBA| {
            [(c.red() * 255.0).round() as u8, (c.green() * 255.0).round() as u8, (c.blue() * 255.0).round() as u8]
        };
        let palette: Vec<[u8; 3]> = l.palette.iter().map(b).collect();
        self.with_state_mut(|s| {
            s.term.set_colors(b(&l.foreground), b(&l.background), l.cursor.as_ref().map(b), &palette);
            s.term.set_default_cursor(
                match l.cursor_shape {
                    CursorKind::Block => render::CursorShape::Block,
                    CursorKind::Ibeam => render::CursorShape::Bar,
                    CursorKind::Underline => render::CursorShape::Underline,
                },
                !matches!(l.cursor_blink, CursorBlink::Off),
            );
            s.term.set_scrollback_lines((l.scrollback >= 0).then_some(l.scrollback as usize));
        });
    }

    pub(crate) fn set_scrollback_lines(&self, lines: i64) {
        if let Some(l) = self.look.borrow_mut().as_mut() {
            l.scrollback = lines;
        }
        self.with_state_mut(|s| s.term.set_scrollback_lines((lines >= 0).then_some(lines as usize)));
    }

    /// The visible screen as a screen reader sees it (from the last drawn frame).
    fn a11y_snapshot(&self) -> a11y::Snapshot {
        self.last_frame.borrow().as_ref().map(|f| a11y::Snapshot::from_frame(f)).unwrap_or_default()
    }

    /// Tell a screen reader what changed since the last frame: text per row, caret, selection.
    /// GTK drops the calls when no assistive technology is connected; the pass over the cells
    /// per frame is cheap (GTK's `realized` property of the AT context needs 4.24).
    fn notify_a11y(&self, frame: &Frame) {
        let obj = self.obj();
        let now = a11y::Snapshot::from_frame(frame);
        let prev = self.a11y_prev.replace(now.clone());
        for e in a11y::diff(&prev.lines, &now.lines) {
            match e {
                a11y::Edit::Remove(a, b) => obj.update_contents(gtk::AccessibleTextContentChange::Remove, a, b),
                a11y::Edit::Insert(a, b) => obj.update_contents(gtk::AccessibleTextContentChange::Insert, a, b),
            }
        }
        if prev.caret() != now.caret() {
            obj.update_caret_position();
        }
        if prev.selection() != now.selection() {
            obj.update_selection_bound();
        }
    }

    pub(crate) fn adjustment(&self) -> gtk::Adjustment {
        self.vadj.borrow().clone()
    }

    /// Follow the adjustment the scrolled window gave us: dragging it scrolls the terminal.
    fn set_vadjustment(&self, adj: Option<gtk::Adjustment>) {
        if let Some(id) = self.vadj_handler.borrow_mut().take() {
            self.vadj.borrow().disconnect(id);
        }
        *self.vadj.borrow_mut() = adj.unwrap_or_else(|| gtk::Adjustment::new(0.0, 0.0, 1.0, 1.0, 1.0, 1.0));
        self.connect_vadjustment();
        self.sync_adjustment();
    }

    fn connect_vadjustment(&self) {
        let w = self.obj().downgrade();
        let id = self.vadj.borrow().connect_value_changed(move |a| {
            let Some(v) = w.upgrade() else { return };
            if v.imp().adj_updating.get() {
                return;
            }
            v.imp().with_state_mut(|s| s.term.scroll_to_row(a.value().max(0.0) as u64));
            v.queue_draw();
        });
        *self.vadj_handler.borrow_mut() = Some(id);
    }

    /// Mirror the terminal's scroll position into the adjustment (once per drawn frame).
    fn sync_adjustment(&self) {
        let Some(sb) = self.with_state(|s| s.term.scrollbar()) else { return };
        let (total, offset, len) = (sb.total as f64, sb.offset as f64, sb.len.max(1) as f64);
        let a = self.vadj.borrow().clone();
        if a.upper() == total.max(len) && a.value() == offset && a.page_size() == len {
            return;
        }
        self.adj_updating.set(true);
        a.configure(offset, 0.0, total.max(len), 1.0, len, len);
        self.adj_updating.set(false);
    }

    pub(crate) fn set_input_enabled(&self, on: bool) {
        self.input_enabled.set(on);
    }

    pub(crate) fn is_input_enabled(&self) -> bool {
        self.input_enabled.get()
    }

    /// Text or key bytes the user produced: to the program, to listeners (shared input).
    fn typed(&self, data: &[u8]) {
        if !self.input_enabled.get() {
            return;
        }
        if self.scroll_on_keystroke.get() {
            self.scroll_bottom();
        }
        self.send(data);
        let text = String::from_utf8_lossy(data);
        for cb in self.commit_cbs.borrow().iter() {
            cb(&text);
        }
    }

    pub(crate) fn set_context_menu(&self, model: &gtk::gio::MenuModel) {
        let obj = self.obj();
        if let Some(old) = self.menu.borrow_mut().take() {
            old.unparent();
        }
        let m = gtk::PopoverMenu::from_model(Some(model));
        m.set_parent(&*obj);
        m.set_has_arrow(false);
        m.set_halign(gtk::Align::Start);
        *self.menu.borrow_mut() = Some(m);
    }

    fn show_menu(&self, x: f64, y: f64) {
        if let Some(m) = self.menu.borrow().as_ref() {
            m.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            m.popup();
        }
    }

    pub(crate) fn child_pid(&self) -> Option<u32> {
        self.with_state(|s| s.pty.as_ref().map(|p| p.pid())).flatten()
    }

    pub(crate) fn foreground_pgid(&self) -> Option<i32> {
        self.with_state(|s| s.pty.as_ref().and_then(|p| p.foreground_pgid())).flatten()
    }

    pub(crate) fn tty_mode(&self) -> Option<parkett_core::startup::TtyMode> {
        self.with_state(|s| s.pty.as_ref().and_then(|p| p.tty_mode())).flatten()
    }

    pub(crate) fn hterminate(&self) {
        self.with_state(|s| {
            if let Some(p) = &s.pty {
                p.hangup();
            }
        });
    }

    pub(crate) fn reset(&self, clear: bool) {
        self.with_state_mut(|s| s.term.reset(clear));
        self.obj().queue_draw();
    }

    pub(crate) fn cursor_on_top_row(&self) -> bool {
        self.with_state(|s| {
            let sb = s.term.scrollbar();
            let (_, y) = s.term.cursor();
            // cursor row counted from the oldest line; the active screen is the last `rows` rows
            let row = sb.total.saturating_sub(s.rows as u64) + y as u64;
            (row as f64 - sb.offset as f64) < 1.0
        })
        .unwrap_or(false)
    }

    fn emit(cbs: &RefCell<Vec<Callback>>) {
        for cb in cbs.borrow().iter() {
            cb();
        }
    }

    fn selection_changed(&self) {
        Self::emit(&self.selection_cbs);
    }

    pub(crate) fn add_pattern(&self, pattern: &str) -> Result<i32, String> {
        self.detector.borrow_mut().add_pattern(pattern)
    }

    pub(crate) fn remove_pattern(&self, tag: i32) {
        self.detector.borrow_mut().remove_pattern(tag);
    }

    pub(crate) fn set_builtin_links(&self, on: bool) {
        self.detector.borrow_mut().set_builtin(on);
    }

    /// Host pattern at pixel position: matched text and tag.
    pub(crate) fn pattern_at(&self, x: f64, y: f64) -> (Option<String>, i32) {
        let (cx, cy) = self.cell_at(x, y);
        let Some(frame) = self.last_frame.borrow().clone() else { return (None, 0) };
        let Some(row) = frame.rows.get(cy as usize) else { return (None, 0) };
        match self.detector.borrow().find_pattern(&RowText::new(row), cx) {
            Some((text, tag, _, _)) => (Some(text), tag),
            None => (None, 0),
        }
    }

    pub(crate) fn hyperlink_at_point(&self, x: f64, y: f64) -> Option<String> {
        let (cx, cy) = self.cell_at(x, y);
        self.with_state(|s| s.term.hyperlink_at(cx, cy)).flatten()
    }

    pub(crate) fn count_matches(&self, re: &glib::Regex, max: usize) -> usize {
        let lines = self.with_state(|s| s.term.screen_lines()).unwrap_or_default();
        search::find_all(&lines, re).len().min(max)
    }

    fn scroll_bottom(&self) {
        self.with_state_mut(|s| s.term.scroll_to_bottom());
    }

    pub(crate) fn send(&self, data: &[u8]) {
        self.with_state_mut(|s| {
            if let Some(p) = &mut s.pty {
                let _ = p.write_all(data);
            }
        });
    }

    pub(crate) fn start(&self, mut opts: SpawnOptions) -> std::io::Result<()> {
        let obj = self.obj();
        let (cols, rows) = {
            let (cw, ch) = self.cell.get();
            let (px, py) = self.pad.get();
            let c = if obj.width() > 0 { ((obj.width() as f64 - 2.0 * px) / cw).max(1.0) as u16 } else { 80 };
            let r = if obj.height() > 0 { ((obj.height() as f64 - 2.0 * py) / ch).max(1.0) as u16 } else { 24 };
            (c.max(1), r.max(1))
        };
        opts.cols = cols;
        opts.rows = rows;
        let pty = Pty::spawn(&opts)?;
        pty.set_nonblocking(true)?;
        let fd = pty.as_raw_fd();

        let mut term = Terminal::new(cols, rows).map_err(|e| std::io::Error::other(format!("terminal: {e}")))?;
        let outbox = Rc::new(RefCell::new(Vec::new()));
        let ob = outbox.clone();
        term.on_pty_write(move |d| ob.borrow_mut().extend_from_slice(d));
        let events = Rc::new(RefCell::new(Vec::new()));
        let ev = events.clone();
        term.on_bell(move || ev.borrow_mut().push(Event::Bell));
        let ev = events.clone();
        term.on_prompt(move |p| ev.borrow_mut().push(Event::Prompt(p)));
        // `vte.sh` also reports command start/end as OSC 666; it has no OSC 133 `C`.
        let ev = events.clone();
        term.on_unknown_osc(move |c| {
            let text = String::from_utf8_lossy(c);
            if text.starts_with("666;vte.shell.preexec") {
                ev.borrow_mut().push(Event::Mark(crate::ShellEvent::Preexec));
            } else if text.starts_with("666;vte.shell.precmd") {
                ev.borrow_mut().push(Event::Mark(crate::ShellEvent::Precmd));
            }
        });
        let st = State {
            term,
            render: RenderState::new().map_err(|e| std::io::Error::other(format!("render state: {e}")))?,
            keys: KeyEncoder::new().map_err(|e| std::io::Error::other(format!("key encoder: {e}")))?,
            mouse: MouseEncoder::new().map_err(|e| std::io::Error::other(format!("mouse encoder: {e}")))?,
            pty: Some(pty),
            outbox,
            events,
            last_cursor: (0, 0),
            last_pwd: None,
            watch: None,
            last_title: None,
            cols,
            rows,
        };
        // A restart replaces the old state (its source is removed on drop of the id below).
        if let Some(mut old) = self.state.borrow_mut().replace(st) {
            if let Some(id) = old.watch.take() {
                id.remove();
            }
        }
        self.apply_look_to_terminal();

        // The closure must be `Send`; the guard makes it so and panics if it ever runs on another thread.
        let guard = glib::thread_guard::ThreadGuard::new(obj.downgrade());
        let source = glib_unix::fd_source_new(
            fd,
            glib::IOCondition::IN | glib::IOCondition::HUP | glib::IOCondition::ERR,
            Some("parkett-pty"),
            glib::Priority::DEFAULT,
            move |_, _| match guard.get_ref().upgrade() {
                Some(v) => v.imp().pump(),
                None => glib::ControlFlow::Break,
            },
        );
        let id = source.attach(None);
        self.with_state_mut(|s| s.watch = Some(id));
        obj.queue_draw();
        Ok(())
    }

    /// Read what the program wrote, feed the terminal, answer its queries.
    fn pump(&self) -> glib::ControlFlow {
        let mut closed = false;
        let mut got_data = false;
        let mut title_changed = false;
        let mut cwd_changed = false;
        let mut cursor_moved = false;
        let mut events = Vec::new();
        let scroll_on_output = self.scroll_on_output.get();
        self.with_state_mut(|s| {
            let Some(pty) = s.pty.as_mut() else { return };
            let mut buf = [0u8; 16384];
            // Bounded per wake-up so a flood does not starve the main loop.
            for _ in 0..16 {
                match pty.read(&mut buf) {
                    Ok(0) => {
                        closed = true;
                        break;
                    }
                    Ok(n) => {
                        got_data = true;
                        s.term.write(&buf[..n]);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(_) => {
                        closed = true;
                        break;
                    }
                }
            }
            let reply = std::mem::take(&mut *s.outbox.borrow_mut());
            if !reply.is_empty() {
                let _ = pty.write_all(&reply);
            }
            if got_data && scroll_on_output {
                s.term.scroll_to_bottom();
            }
            let t = s.term.title();
            if t != s.last_title {
                s.last_title = t;
                title_changed = true;
            }
            let pwd = s.term.pwd();
            if pwd != s.last_pwd {
                s.last_pwd = pwd;
                cwd_changed = true;
            }
            let cur = s.term.cursor();
            if cur != s.last_cursor {
                s.last_cursor = cur;
                cursor_moved = true;
            }
            events = std::mem::take(&mut *s.events.borrow_mut());
        });
        let obj = self.obj();
        obj.queue_draw();
        if got_data {
            Self::emit(&self.contents_cbs);
        }
        if cursor_moved {
            Self::emit(&self.cursor_cbs);
        }
        for ev in events {
            match ev {
                Event::Bell => {
                    if self.audible_bell.get() {
                        obj.display().beep();
                    }
                    Self::emit(&self.bell_cbs);
                }
                Event::Mark(e) => {
                    for cb in self.shell_cbs.borrow().iter() {
                        cb(e);
                    }
                }
                Event::Prompt(p) => {
                    let e = match p {
                        Prompt::OutputStart => crate::ShellEvent::Preexec,
                        Prompt::PromptStart => crate::ShellEvent::Precmd,
                        _ => continue,
                    };
                    for cb in self.shell_cbs.borrow().iter() {
                        cb(e);
                    }
                }
            }
        }
        if title_changed {
            for cb in self.title_cbs.borrow().iter() {
                cb();
            }
            for cb in self.shell_cbs.borrow().iter() {
                cb(crate::ShellEvent::Title);
            }
        }
        if cwd_changed {
            for cb in self.shell_cbs.borrow().iter() {
                cb(crate::ShellEvent::Cwd);
            }
        }
        if closed {
            self.with_state_mut(|s| s.watch = None);
            self.finish();
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    }

    /// Output ended: collect the exit status (the kernel may need a moment) and tell listeners.
    fn finish(&self) {
        let weak = self.obj().downgrade();
        glib::timeout_add_local(std::time::Duration::from_millis(20), move || {
            let Some(v) = weak.upgrade() else { return glib::ControlFlow::Break };
            let st = v.imp().with_state_mut(|s| s.pty.as_mut().and_then(|p| p.try_wait().ok().flatten()));
            match st {
                Some(Some(status)) => {
                    for cb in v.imp().exit_cbs.borrow().iter() {
                        cb(status.into_raw());
                    }
                    glib::ControlFlow::Break
                }
                Some(None) => glib::ControlFlow::Continue,
                None => glib::ControlFlow::Break,
            }
        });
    }

    fn key(
        &self,
        ctrl: &gtk::EventControllerKey,
        keyval: gdk::Key,
        code: u32,
        state: gdk::ModifierType,
        press: bool,
    ) -> glib::Propagation {
        if press && state.contains(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK) {
            match keyval.to_lower() {
                gdk::Key::c => {
                    self.copy(false);
                    return glib::Propagation::Stop;
                }
                gdk::Key::v => {
                    self.paste_clipboard();
                    return glib::Propagation::Stop;
                }
                gdk::Key::a => {
                    self.select_all();
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
        }
        if press && !self.input_enabled.get() {
            return glib::Propagation::Stop;
        }
        if press {
            if let Some(ev) = ctrl.current_event() {
                if self.im.filter_keypress(&ev) {
                    return glib::Propagation::Stop;
                }
            }
        }
        // GDK key codes are evdev + 8
        let Some(key) = code.checked_sub(8).and_then(input::key_from_evdev) else {
            return glib::Propagation::Proceed;
        };
        let mut mods = 0;
        for (flag, bit) in [
            (gdk::ModifierType::SHIFT_MASK, input::MOD_SHIFT),
            (gdk::ModifierType::CONTROL_MASK, input::MOD_CTRL),
            (gdk::ModifierType::ALT_MASK, input::MOD_ALT),
            (gdk::ModifierType::SUPER_MASK, input::MOD_SUPER),
            (gdk::ModifierType::LOCK_MASK, input::MOD_CAPS_LOCK),
        ] {
            if state.contains(flag) {
                mods |= bit;
            }
        }
        let text: String = keyval.to_unicode().filter(|c| !c.is_control()).map(String::from).unwrap_or_default();
        let unshifted = keyval.to_lower().to_unicode().map_or(0, |c| c as u32);
        let consumed = if !text.is_empty() { mods & input::MOD_SHIFT } else { 0 };
        let ev = KeyEvent { key, mods, utf8: &text, unshifted, press, consumed_mods: consumed };
        let bytes = self.with_state_mut(|s| s.keys.encode(&s.term, &ev)).unwrap_or_default();
        let sent = !bytes.is_empty();
        if sent && press {
            self.typed(&bytes);
        } else if sent {
            // key releases (Kitty protocol) go to the program, they are not "typed text"
            self.send(&bytes);
        }
        if sent {
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    }

    /// The link at a cell: OSC 8 first (the terminal knows it exactly), then detected text.
    pub(crate) fn link_at_cell(&self, x: u16, y: u16) -> Option<Span> {
        if let Some(uri) = self.with_state(|s| s.term.hyperlink_at(x, y)).flatten() {
            // Extent: neighbouring cells with the same target.
            let same =
                |cx: u16| self.with_state(|s| s.term.hyperlink_at(cx, y)).flatten().as_deref() == Some(uri.as_str());
            let cols = self.with_state(|s| s.cols).unwrap_or(1);
            let (mut x0, mut x1) = (x, x + 1);
            while x0 > 0 && same(x0 - 1) {
                x0 -= 1;
            }
            while x1 < cols && same(x1) {
                x1 += 1;
            }
            return Some(Span { y, x0, x1, link: Link::Uri(uri) });
        }
        let frame = self.last_frame.borrow().clone()?;
        let row = frame.rows.get(y as usize)?;
        self.detector.borrow().find(y, &RowText::new(row), x)
    }

    fn set_hover(&self, span: Option<Span>) {
        if *self.hover.borrow() == span {
            return;
        }
        let obj = self.obj();
        obj.set_cursor_from_name(Some(if span.is_some() { "pointer" } else { "text" }));
        // The real target of an OSC 8 link is shown: the visible text can deceive.
        obj.set_tooltip_text(match &span {
            Some(Span { link: Link::Uri(u), .. }) => Some(u.as_str()),
            _ => None,
        });
        *self.hover.borrow_mut() = span;
        obj.queue_draw();
    }

    pub(crate) fn set_custom_links(&self, links: &[parkett_core::link::CustomLink]) {
        self.detector.borrow_mut().set_custom(links);
    }

    pub(crate) fn search_set(&self, pattern: Option<&str>, case_sensitive: bool) -> Result<usize, String> {
        let mut st = self.search.borrow_mut();
        *st = Search::default();
        let Some(p) = pattern.filter(|p| !p.is_empty()) else {
            self.obj().queue_draw();
            return Ok(0);
        };
        let re = search::compile(p, case_sensitive)?;
        let lines = self.with_state(|s| s.term.screen_lines()).unwrap_or_default();
        st.matches = search::find_all(&lines, &re);
        st.regex = Some(re);
        self.obj().queue_draw();
        Ok(st.matches.len())
    }

    pub(crate) fn search_count(&self) -> usize {
        self.search.borrow().matches.len()
    }

    /// 0-based index of the selected hit.
    pub(crate) fn search_index(&self) -> Option<usize> {
        self.search.borrow().current
    }

    /// Select the next (`up == false`) or previous hit, wrapping around, and scroll to it.
    pub(crate) fn search_step(&self, up: bool) -> bool {
        let mut st = self.search.borrow_mut();
        let n = st.matches.len();
        if n == 0 {
            return false;
        }
        let sb = self.with_state(|s| s.term.scrollbar()).unwrap_or(ghostty_vt::find::Scrollbar {
            total: 0,
            offset: 0,
            len: 0,
        });
        let idx = match (st.current, up) {
            (Some(i), false) => (i + 1) % n,
            (Some(i), true) => (i + n - 1) % n,
            (None, false) => st.matches.partition_point(|m| m.y < sb.offset) % n,
            (None, true) => st.matches.partition_point(|m| m.y < sb.offset + sb.len).checked_sub(1).unwrap_or(n - 1),
        };
        st.current = Some(idx);
        let m = st.matches[idx];
        drop(st);
        self.with_state_mut(|s| {
            s.term.select_screen_range((m.x0, m.y), (m.x1 - 1, m.y));
            if m.y < sb.offset || m.y >= sb.offset + sb.len {
                s.term.scroll_to_row(m.y.saturating_sub(sb.len / 2));
            }
        });
        self.obj().queue_draw();
        true
    }

    fn mods_of(state: gdk::ModifierType) -> u32 {
        let mut m = 0;
        for (flag, bit) in [
            (gdk::ModifierType::SHIFT_MASK, input::MOD_SHIFT),
            (gdk::ModifierType::CONTROL_MASK, input::MOD_CTRL),
            (gdk::ModifierType::ALT_MASK, input::MOD_ALT),
            (gdk::ModifierType::SUPER_MASK, input::MOD_SUPER),
        ] {
            if state.contains(flag) {
                m |= bit;
            }
        }
        m
    }

    fn cell_at(&self, x: f64, y: f64) -> (u16, u16) {
        let (cw, ch) = self.cell.get();
        let (px, py) = self.pad.get();
        let (x, y) = (x - px, y - py);
        let (cols, rows) = self.with_state(|s| (s.cols, s.rows)).unwrap_or((1, 1));
        (((x / cw).max(0.0) as u16).min(cols.saturating_sub(1)), ((y / ch).max(0.0) as u16).min(rows.saturating_sub(1)))
    }

    /// Report an event to the program if it asked for mouse events; `true` when it was reported.
    fn report_mouse(&self, action: MouseAction, button: Option<u32>, mods: u32, x: f64, y: f64) -> bool {
        let (cw, ch) = self.cell.get();
        let obj = self.obj();
        let (px, py) = self.pad.get();
        let (x, y) = (x - px, y - py);
        let g = MouseGeometry {
            width: (obj.width() as f64 - 2.0 * px).max(1.0) as u32,
            height: (obj.height() as f64 - 2.0 * py).max(1.0) as u32,
            cell_w: cw as u32,
            cell_h: ch as u32,
        };
        let any = self.buttons.get() != 0;
        self.with_state_mut(|s| {
            if !s.term.mouse_tracking() {
                return false;
            }
            let bytes = s.mouse.encode(&s.term, g, any, &MouseEvent { action, button, mods, x: x as f32, y: y as f32 });
            if let Some(p) = &mut s.pty {
                if !bytes.is_empty() {
                    let _ = p.write_all(&bytes);
                }
            }
            true
        })
        .unwrap_or(false)
    }

    fn mouse_button(&self, button: u32, state: gdk::ModifierType, n_press: i32, x: f64, y: f64, press: bool) {
        let lib_button = match button {
            1 => input::BUTTON_LEFT,
            2 => input::BUTTON_MIDDLE,
            3 => input::BUTTON_RIGHT,
            _ => return,
        };
        let mods = Self::mods_of(state);
        let bit = 1u32 << button.min(31);
        if press && button == 1 && mods & input::MOD_CTRL != 0 {
            let (cx, cy) = self.cell_at(x, y);
            if let Some(span) = self.link_at_cell(cx, cy) {
                for cb in self.link_cbs.borrow().iter() {
                    cb(&span.link);
                }
                return;
            }
        }
        if press {
            // Shift forces selection even while a program tracks the mouse.
            let tracked =
                mods & input::MOD_SHIFT == 0 && self.report_mouse(MouseAction::Press, Some(lib_button), mods, x, y);
            self.reporting.set(tracked);
            self.buttons.set(self.buttons.get() | bit);
            if tracked {
                return;
            }
            match button {
                3 => self.show_menu(x, y),
                1 => {
                    let cell = self.cell_at(x, y);
                    self.anchor.set(Some(cell));
                    self.with_state_mut(|s| match n_press {
                        2 => {
                            s.term.select_word(cell.0, cell.1);
                        }
                        3 => {
                            s.term.select_line(cell.0, cell.1);
                        }
                        _ => s.term.clear_selection(),
                    });
                    self.selection_changed();
                    self.obj().queue_draw();
                }
                2 => self.paste_primary(),
                _ => {}
            }
        } else {
            let was_reported = self.reporting.get();
            if was_reported {
                self.report_mouse(MouseAction::Release, Some(lib_button), mods, x, y);
            }
            self.buttons.set(self.buttons.get() & !bit);
            if self.buttons.get() == 0 {
                self.reporting.set(false);
            }
            if button == 1 && !was_reported {
                self.anchor.set(None);
                // Selecting copies to the primary selection (middle click pastes it).
                if self.with_state(|s| s.term.has_selection()).unwrap_or(false) {
                    self.copy_to(&self.obj().primary_clipboard(), false);
                }
            }
        }
    }

    fn mouse_motion(&self, state: gdk::ModifierType, x: f64, y: f64) {
        let mods = Self::mods_of(state);
        if self.buttons.get() == 0 {
            let (cx, cy) = self.cell_at(x, y);
            self.set_hover(self.link_at_cell(cx, cy));
        }
        if self.reporting.get() || (self.buttons.get() == 0 && mods & input::MOD_SHIFT == 0) {
            let held = if self.buttons.get() & 2 != 0 { Some(input::BUTTON_LEFT) } else { None };
            if self.report_mouse(MouseAction::Motion, held, mods, x, y) || self.reporting.get() {
                return;
            }
        }
        if let Some(a) = self.anchor.get() {
            let c = self.cell_at(x, y);
            let rect = state.contains(gdk::ModifierType::ALT_MASK);
            self.with_state_mut(|s| s.term.select_range(a, c, rect));
            self.selection_changed();
            self.obj().queue_draw();
        }
    }

    fn wheel(&self, state: gdk::ModifierType, dy: f64) {
        let mods = Self::mods_of(state);
        let up = dy < 0.0;
        if mods & input::MOD_SHIFT == 0 {
            let button = if up { input::BUTTON_WHEEL_UP } else { input::BUTTON_WHEEL_DOWN };
            let tracked = self.with_state(|s| s.term.mouse_tracking()).unwrap_or(false);
            if tracked {
                for _ in 0..dy.abs().round().max(1.0) as usize {
                    self.report_mouse(MouseAction::Press, Some(button), mods, 0.0, 0.0);
                }
                return;
            }
        }
        self.with_state_mut(|s| s.term.scroll((dy * 3.0).round() as isize));
        self.obj().queue_draw();
    }

    pub(crate) fn has_selection(&self) -> bool {
        self.with_state(|s| s.term.has_selection()).unwrap_or(false)
    }

    pub(crate) fn select_all(&self) {
        self.with_state_mut(|s| s.term.select_all());
        self.selection_changed();
        self.obj().queue_draw();
    }

    pub(crate) fn clear_selection(&self) {
        self.with_state_mut(|s| s.term.clear_selection());
        self.selection_changed();
        self.obj().queue_draw();
    }

    pub(crate) fn selected_text(&self, html: bool) -> Option<String> {
        self.with_state(|s| s.term.selected_text(html)).flatten()
    }

    fn copy_to(&self, clipboard: &gdk::Clipboard, html: bool) {
        let Some(plain) = self.selected_text(false) else { return };
        if html {
            if let Some(h) = self.selected_text(true) {
                let provider = gdk::ContentProvider::new_union(&[
                    gdk::ContentProvider::for_bytes("text/html", &glib::Bytes::from_owned(h.into_bytes())),
                    gdk::ContentProvider::for_value(&plain.to_value()),
                ]);
                let _ = clipboard.set_content(Some(&provider));
                return;
            }
        }
        clipboard.set_text(&plain);
    }

    /// Copy the selection to the clipboard; `html` also offers it as `text/html`.
    pub(crate) fn copy(&self, html: bool) {
        self.copy_to(&self.obj().clipboard(), html);
    }

    pub(crate) fn paste_clipboard(&self) {
        self.paste_from(self.obj().clipboard());
    }

    fn paste_primary(&self) {
        self.paste_from(self.obj().primary_clipboard());
    }

    fn paste_from(&self, clipboard: gdk::Clipboard) {
        let weak = self.obj().downgrade();
        clipboard.read_text_async(gtk::gio::Cancellable::NONE, move |res| {
            if let (Some(v), Ok(Some(text))) = (weak.upgrade(), res) {
                v.imp().paste_text(&text, false);
            }
        });
    }

    /// Paste as the program wants it; text that could run commands goes to the
    /// `connect_unsafe_paste` listeners first (they decide and call again with `allow_unsafe`).
    pub(crate) fn paste_text(&self, text: &str, allow_unsafe: bool) -> PasteResult {
        if !self.input_enabled.get() {
            return PasteResult::Empty;
        }
        let r = self.with_state_mut(|s| {
            s.term.scroll_to_bottom();
            let r = s.term.paste(text, allow_unsafe);
            let reply = std::mem::take(&mut *s.outbox.borrow_mut());
            if let (Some(p), false) = (s.pty.as_mut(), reply.is_empty()) {
                let _ = p.write_all(&reply);
            }
            r
        });
        let r = r.unwrap_or(PasteResult::Empty);
        if r == PasteResult::Pasted {
            for cb in self.commit_cbs.borrow().iter() {
                cb(text);
            }
        }
        if r == PasteResult::Unsafe {
            for cb in self.unsafe_paste_cbs.borrow().iter() {
                cb(text);
            }
        }
        r
    }

    fn draw_frame(&self, snapshot: &gtk::Snapshot, frame: &Frame, offset: u64) {
        let obj = self.obj();
        let (cw, ch) = self.cell.get();
        let (cw, ch) = (cw as f32, ch as f32);
        let focused = self.focused.get();
        let font = self.scaled_font();
        let pad = self.text_pad.get() as f32;
        let highlight = self.highlight.get();
        let cursor = frame.cursor;
        let block_at = |x: usize, y: usize| {
            focused && cursor.is_some_and(|c| c.shape == CursorShape::Block && c.x as usize == x && c.y as usize == y)
        };

        for (y, row) in frame.rows.iter().enumerate() {
            let fy = y as f32 * ch;
            // Effective colors per cell, then background rectangles and text runs.
            let mut bg_run: Option<(usize, usize, Rgb)> = None; // first cell, cells, color
            let mut run: Option<(usize, String, Rgb, f32, u16, u8)> = None; // x0, text, fg, alpha, flags, underline
                                                                            // Two passes per row: all backgrounds first, then the text, so a later background span
                                                                            // can never paint over earlier text.
            type TextRun = (usize, String, Rgb, f32, u16, u8);
            let bgs: RefCell<Vec<(usize, usize, Rgb)>> = RefCell::new(Vec::new());
            let texts: RefCell<Vec<TextRun>> = RefCell::new(Vec::new());
            let flush_bg = |bg: &mut Option<(usize, usize, Rgb)>| {
                if let Some(b) = bg.take() {
                    bgs.borrow_mut().push(b);
                }
            };
            let flush = |run: &mut Option<TextRun>| {
                if let Some(t) = run.take() {
                    texts.borrow_mut().push(t);
                }
            };
            let draw_text = |(x0, text, fg, alpha, flags, ul): TextRun| {
                if text.trim().is_empty() && ul == 0 && flags & render::STRIKETHROUGH == 0 {
                    return;
                }
                let layout = obj.create_pango_layout(Some(&text));
                let mut fd = font.clone();
                if flags & render::BOLD != 0 {
                    fd.set_weight(pango::Weight::Bold);
                }
                if flags & render::ITALIC != 0 {
                    fd.set_style(pango::Style::Italic);
                }
                layout.set_font_description(Some(&fd));
                let attrs = pango::AttrList::new();
                if ul != 0 {
                    attrs.insert(pango::AttrInt::new_underline(match ul {
                        2 => pango::Underline::Double,
                        3 => pango::Underline::Error,
                        _ => pango::Underline::Single,
                    }));
                }
                if flags & render::STRIKETHROUGH != 0 {
                    attrs.insert(pango::AttrInt::new_strikethrough(true));
                }
                if flags & render::OVERLINE != 0 {
                    attrs.insert(pango::AttrInt::new_overline(pango::Overline::Single));
                }
                layout.set_attributes(Some(&attrs));
                snapshot.save();
                snapshot.translate(&graphene::Point::new(x0 as f32 * cw, fy + pad));
                snapshot.append_layout(&layout, &rgba(fg, alpha));
                snapshot.restore();
            };

            for (x, cell) in row.iter().enumerate() {
                if cell.spacer {
                    continue;
                }
                let mut fg = cell.fg.unwrap_or(frame.fg);
                let mut bg = cell.bg;
                let mut inverse = cell.flags & render::INVERSE != 0;
                if (cell.selected && highlight.is_none()) || block_at(x, y) {
                    inverse = !inverse;
                }
                if cell.selected && !block_at(x, y) && highlight.is_some() {
                    // Selection in the accent color instead of inverted (profile palette)
                    bg = highlight;
                    inverse = false;
                }
                if inverse {
                    let new_fg = bg.unwrap_or(frame.bg);
                    bg = Some(fg);
                    fg = new_fg;
                }
                let span = if cell.wide { 2 } else { 1 };
                match (bg, &mut bg_run) {
                    (Some(c), Some(r)) if r.2 == c && r.0 + r.1 == x => r.1 += span,
                    (Some(c), _) => {
                        flush_bg(&mut bg_run);
                        bg_run = Some((x, span, c));
                    }
                    (None, _) => flush_bg(&mut bg_run),
                }
                if cell.flags & render::INVISIBLE != 0 {
                    flush(&mut run);
                    continue;
                }
                let alpha = if cell.flags & render::FAINT != 0 { 0.6 } else { 1.0 };
                let glyph = if cell.text.is_empty() { " " } else { cell.text.as_str() };
                let same = |r: &(usize, String, Rgb, f32, u16, u8)| {
                    r.2 == fg && r.3 == alpha && r.4 == cell.flags && r.5 == cell.underline
                };
                if cell.wide {
                    // Wide glyphs get their own run so fallback fonts cannot shift the line.
                    flush(&mut run);
                    run = Some((x, glyph.to_string(), fg, alpha, cell.flags, cell.underline));
                    flush(&mut run);
                    continue;
                }
                match &mut run {
                    Some(r) if same(r) => r.1.push_str(glyph),
                    _ => {
                        flush(&mut run);
                        run = Some((x, glyph.to_string(), fg, alpha, cell.flags, cell.underline));
                    }
                }
            }
            flush_bg(&mut bg_run);
            flush(&mut run);
            for (x0, n, c) in bgs.take() {
                snapshot.append_color(&rgba(c, 1.0), &graphene::Rect::new(x0 as f32 * cw, fy, n as f32 * cw, ch));
            }
            // Search hits sit between the cell backgrounds and the text.
            {
                let search = self.search.borrow();
                let lo = search.matches.partition_point(|m| m.y < offset + y as u64);
                for (i, m) in search.matches[lo..].iter().enumerate().take_while(|(_, m)| m.y == offset + y as u64) {
                    let current = search.current == Some(lo + i);
                    let color = if current {
                        gdk::RGBA::new(1.0, 0.55, 0.0, 0.65)
                    } else {
                        gdk::RGBA::new(1.0, 0.85, 0.0, 0.4)
                    };
                    snapshot.append_color(
                        &color,
                        &graphene::Rect::new(m.x0 as f32 * cw, fy, (m.x1 - m.x0) as f32 * cw, ch),
                    );
                }
            }
            for t in texts.take() {
                draw_text(t);
            }
            if let Some(h) = self.hover.borrow().as_ref().filter(|h| h.y as usize == y) {
                let line = 1.0_f32.max((ch / 14.0).round());
                snapshot.append_color(
                    &rgba(frame.fg, 1.0),
                    &graphene::Rect::new(h.x0 as f32 * cw, fy + ch - line, (h.x1 - h.x0) as f32 * cw, line),
                );
            }
        }

        if let Some(c) = cursor {
            let (x, y) = (c.x as f32 * cw, c.y as f32 * ch);
            let color = rgba(frame.cursor_color.unwrap_or(frame.fg), 1.0);
            let wcells = if c.wide_tail { 2.0 } else { 1.0 };
            let line = 2.0_f32.max((ch / 12.0).round());
            match (c.shape, focused) {
                (CursorShape::Block, true) => {}
                (CursorShape::Bar, true) => snapshot.append_color(&color, &graphene::Rect::new(x, y, line, ch)),
                (CursorShape::Underline, true) => {
                    snapshot.append_color(&color, &graphene::Rect::new(x, y + ch - line, cw * wcells, line))
                }
                _ => {
                    // hollow block, also for every shape while the widget has no focus
                    let b = 1.0;
                    for r in [
                        graphene::Rect::new(x, y, cw, b),
                        graphene::Rect::new(x, y + ch - b, cw, b),
                        graphene::Rect::new(x, y, b, ch),
                        graphene::Rect::new(x + cw - b, y, b, ch),
                    ] {
                        snapshot.append_color(&color, &r);
                    }
                }
            }
        }
    }
}
