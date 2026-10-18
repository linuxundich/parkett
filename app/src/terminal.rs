//! Ein Terminal-Bereich: VTE-Widget, Shell-Prozess und die Leiste „Prozess beendet“.
//!
//! Zustand (Titel, Verzeichnis) kommt aus den VTE-Termprops (`xterm.title`, `vte.cwd`),
//! nicht aus den seit VTE 0.78 veralteten Signalen.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};
use std::os::fd::AsRawFd;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib, pango};
use gtk::glib::translate::{from_glib_full, ToGlibPtr};
use vte::prelude::*;

use parkett_core::context::{self, Context};
use parkett_core::palette::Rgb;
use parkett_core::profile::ExitAction;

use crate::profiles::ProfileManager;

/// Kleinste sinnvolle Größe eines Bereichs in Zellen (siehe docs/ANALYSE.md, Funktionsmatrix).
pub const MIN_COLUMNS: i64 = 20;
pub const MIN_ROWS: i64 = 5;

type Callback = Box<dyn Fn()>;

struct Inner {
    root: gtk::Box,
    vte: vte::Terminal,
    exit_bar: gtk::Revealer,
    exit_label: gtk::Label,
    cwd_at_start: Option<String>,
    profile_id: RefCell<String>,
    started_at: Cell<Option<Instant>>,
    background: Cell<Rgb>,
    on_colors_changed: RefCell<Vec<Callback>>,
    /// Link unter dem Zeiger beim Rechtsklick (für „Link öffnen/kopieren“ im Kontextmenü)
    menu_link: RefCell<Option<String>>,
    actions: gio::SimpleActionGroup,
    shortcut_controller: RefCell<Option<gtk::ShortcutController>>,
    /// Text, der nach dem Start in die Shell getippt wird (Startbefehl einer Vorlage)
    pending_input: RefCell<Option<String>>,
    /// root/SSH im Vordergrund (Kontextfarbe)
    context: RefCell<Context>,
    /// Akzentfarbe des Kontexts für Linie und Chip
    accent: Cell<Option<(Rgb, Rgb)>>,
    child_pid: Cell<Option<glib::Pid>>,
    exit: Cell<Option<ExitStatus>>,
    on_state_changed: RefCell<Vec<Callback>>,
    on_close: RefCell<Vec<Callback>>,
}

#[derive(Clone)]
pub struct Terminal(Rc<Inner>);

impl Terminal {
    pub fn new(cwd: Option<String>, profile_id: &str) -> Self {
        let vte = vte::Terminal::builder()
            .hexpand(true)
            .vexpand(true)
            .allow_hyperlink(true)
            .pointer_autohide(true)
            .bold_is_bright(false)
            .enable_a11y(true)
            .build();
        vte.set_size(100, 30);
        // Setter statt Builder: nur er nimmt -1 (unbegrenzt) an
        vte.set_scrollback_lines(crate::settings::scrollback_lines());

        let exit_label = gtk::Label::builder().xalign(0.0).hexpand(true).ellipsize(pango::EllipsizeMode::End).build();
        let restart = gtk::Button::with_label("Neu starten");
        let close = gtk::Button::with_label("Schließen");
        let bar = gtk::Box::builder().spacing(10).css_classes(["parkett-exit-bar"]).build();
        bar.append(&gtk::Image::from_icon_name("dialog-warning-symbolic"));
        bar.append(&exit_label);
        bar.append(&restart);
        bar.append(&close);
        // Verborgen auch unsichtbar schalten: ein zugeklappter GtkRevealer fordert sonst
        // weiter die Breite der Leiste an, und Bereiche könnten nicht schmaler werden.
        let exit_bar = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .child(&bar)
            .visible(false)
            .build();

        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.append(&vte);
        root.append(&exit_bar);

        let t = Terminal(Rc::new(Inner {
            root,
            vte,
            exit_bar,
            exit_label,
            cwd_at_start: cwd,
            profile_id: RefCell::new(profile_id.to_string()),
            started_at: Cell::new(None),
            background: Cell::new(Rgb { r: 255, g: 255, b: 255 }),
            on_colors_changed: RefCell::default(),
            menu_link: RefCell::new(None),
            actions: gio::SimpleActionGroup::new(),
            shortcut_controller: RefCell::new(None),
            pending_input: RefCell::new(None),
            context: RefCell::new(Context::Local),
            accent: Cell::new(None),
            child_pid: Cell::new(None),
            exit: Cell::new(None),
            on_state_changed: RefCell::default(),
            on_close: RefCell::default(),
        }));

        t.setup_style();
        t.setup_actions();
        t.setup_signals();
        t.setup_links();
        t.watch_context();

        let weak = Rc::downgrade(&t.0);
        restart.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                Terminal(inner).spawn();
            }
        });
        let weak = Rc::downgrade(&t.0);
        close.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                Terminal(inner).emit_close();
            }
        });

        t.spawn();
        t
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.0.root
    }

    pub fn vte(&self) -> &vte::Terminal {
        &self.0.vte
    }

    pub fn connect_state_changed(&self, f: impl Fn() + 'static) {
        self.0.on_state_changed.borrow_mut().push(Box::new(f));
    }

    pub fn connect_colors_changed(&self, f: impl Fn() + 'static) {
        self.0.on_colors_changed.borrow_mut().push(Box::new(f));
    }

    pub fn profile_id(&self) -> String {
        self.0.profile_id.borrow().clone()
    }

    /// Wechselt das Profil für Farben und künftige Neustarts (laufender Prozess bleibt).
    pub fn set_profile(&self, id: &str) {
        *self.0.profile_id.borrow_mut() = id.to_string();
        self.apply_colors();
        self.emit_state_changed();
    }

    /// Hintergrundfarbe der aktiven Fassung (für die Abblendung im Bereich).
    pub fn background(&self) -> Rgb {
        self.0.background.get()
    }

    pub fn connect_close(&self, f: impl Fn() + 'static) {
        self.0.on_close.borrow_mut().push(Box::new(f));
    }

    /// Fenstertitel, den die Shell per OSC 0/2 gesetzt hat; sonst der Befehl des Profils.
    pub fn title(&self) -> Option<String> {
        self.0.vte.termprop_string("xterm.title").0.map(String::from).filter(|s| !s.is_empty()).or_else(|| {
            let command = ProfileManager::get().profile(&self.profile_id()).command;
            Some(command.trim().to_string()).filter(|c| !c.is_empty())
        })
    }

    /// Arbeitsverzeichnis aus `vte.cwd` (OSC 7, setzt vte.sh in der Shell voraus).
    /// vte4-rs bindet `ref_termprop_uri` nicht; daher der eine FFI-Aufruf.
    pub fn cwd(&self) -> Option<String> {
        let uri: Option<glib::Uri> = unsafe {
            from_glib_full(vte::ffi::vte_terminal_ref_termprop_uri(
                self.0.vte.to_glib_none().0,
                c"vte.cwd".as_ptr(),
            ))
        };
        uri.map(|u| u.path().to_string()).filter(|p| !p.is_empty())
    }

    /// Wie der Prozess geendet hat; `None`, solange er läuft.
    pub fn exit_status(&self) -> Option<ExitStatus> {
        self.0.exit.get()
    }

    /// Beendet die Shell (SIGHUP wie beim Schließen eines Terminalfensters; die Shell gibt
    /// es an ihre Jobs weiter). VTE tut das auch beim Freigeben des Widgets – hier
    /// ausdrücklich, damit kein Prozess an einer übersehenen Referenz hängen bleibt.
    pub fn terminate(&self) {
        if self.0.exit.get().is_none() {
            if let Some(pid) = self.0.child_pid.get() {
                unsafe { libc::kill(pid.0, libc::SIGHUP) };
            }
        }
    }

    pub fn context(&self) -> Context {
        self.0.context.borrow().clone()
    }

    /// (Linie, Chip-Hintergrund) für den aktuellen Kontext; `None` = lokal oder abgeschaltet.
    pub fn context_accent(&self) -> Option<(Rgb, Rgb)> {
        self.0.accent.get()
    }

    /// Fragt einmal pro Sekunde den Vordergrundprozess ab (wie Ptyxis, aber je Bereich).
    /// /proc lesen ist billig; der Timer endet mit dem Terminal.
    fn watch_context(&self) {
        let weak = Rc::downgrade(&self.0);
        glib::timeout_add_local(Duration::from_millis(1000), move || {
            let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
            let t = Terminal(inner);
            let now = t.detect_context();
            if now != *t.0.context.borrow() {
                *t.0.context.borrow_mut() = now;
                t.apply_colors();
                t.emit_state_changed();
            }
            glib::ControlFlow::Continue
        });
    }

    fn detect_context(&self) -> Context {
        if self.0.exit.get().is_some() {
            return Context::Local;
        }
        let Some(pty) = self.0.vte.pty() else { return Context::Local };
        let pgid = unsafe { libc::tcgetpgrp(pty.fd().as_raw_fd()) };
        if pgid <= 0 {
            return Context::Local;
        }
        let comm = std::fs::read_to_string(format!("/proc/{pgid}/comm")).unwrap_or_default();
        let args: Vec<String> = std::fs::read(format!("/proc/{pgid}/cmdline"))
            .unwrap_or_default()
            .split(|b| *b == 0)
            .filter(|a| !a.is_empty())
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        // „Uid:\treal\teffektiv\t…“
        let euid = std::fs::read_to_string(format!("/proc/{pgid}/status"))
            .ok()
            .and_then(|s| s.lines().find(|l| l.starts_with("Uid:")).and_then(|l| l.split_whitespace().nth(2)).and_then(|u| u.parse().ok()));
        context::classify(comm.trim(), &args, euid)
    }

    /// Befehl in die Shell tippen und mit Eingabe abschicken (Vorschlag 5: Shell bleibt
    /// danach nutzbar). Läuft die Shell noch nicht, wird er nach dem Start getippt.
    pub fn type_command(&self, command: &str) {
        let text = format!("{}\n", command.trim_end());
        if self.0.child_pid.get().is_some() && self.0.exit.get().is_none() {
            self.0.vte.feed_child(text.as_bytes());
        } else {
            *self.0.pending_input.borrow_mut() = Some(text);
        }
    }

    /// Befehlszeile des Vordergrundprozesses (für „Als Vorlage speichern“), sonst `None`.
    pub fn foreground_command(&self) -> Option<String> {
        self.foreground_process()?;
        let child = self.0.child_pid.get()?.0;
        let pty = self.0.vte.pty()?;
        let pgid = unsafe { libc::tcgetpgrp(pty.fd().as_raw_fd()) };
        if pgid <= 0 || pgid == child {
            return None;
        }
        let raw = std::fs::read(format!("/proc/{pgid}/cmdline")).ok()?;
        let args: Vec<String> = raw
            .split(|b| *b == 0)
            .filter(|a| !a.is_empty())
            .map(|a| {
                let a = String::from_utf8_lossy(a).into_owned();
                // nur quoten, wo die Shell es braucht – „npm run dev“ statt „'npm' 'run' 'dev'“
                if !a.is_empty() && a.chars().all(|c| c.is_alphanumeric() || "-_./:=@%+,~".contains(c)) {
                    a
                } else {
                    glib::shell_quote(&a).to_string_lossy().into_owned()
                }
            })
            .collect();
        Some(args.join(" ")).filter(|s| !s.is_empty())
    }

    /// Name des Vordergrundprozesses, wenn er nicht die Shell selbst ist (z. B. „vim“, „ssh“).
    /// Muster wie Ptyxis (`agent/ptyxis-process-impl.c`): tcgetpgrp auf der PTY gegen die
    /// PID der Shell vergleichen, Name aus /proc. Nur ohne Flatpak-Sandbox aussagekräftig.
    pub fn foreground_process(&self) -> Option<String> {
        if self.0.exit.get().is_some() {
            return None;
        }
        let child = self.0.child_pid.get()?.0;
        let pty = self.0.vte.pty()?;
        let pgid = unsafe { libc::tcgetpgrp(pty.fd().as_raw_fd()) };
        if pgid <= 0 || pgid == child {
            return None;
        }
        let comm = std::fs::read_to_string(format!("/proc/{pgid}/comm")).unwrap_or_default();
        Some(comm.trim().to_string()).filter(|c| !c.is_empty()).or(Some(format!("PID {pgid}")))
    }

    fn emit_state_changed(&self) {
        for f in self.0.on_state_changed.borrow().iter() {
            f();
        }
    }

    fn emit_close(&self) {
        for f in self.0.on_close.borrow().iter() {
            f();
        }
    }

    fn spawn(&self) {
        self.0.exit_bar.set_reveal_child(false);
        self.0.exit_bar.set_visible(false);
        self.0.exit.set(None);
        self.0.child_pid.set(None);
        self.0.started_at.set(Some(Instant::now()));
        let profile = ProfileManager::get().profile(&self.profile_id());

        // Befehl aus dem Profil (wie in einer Shell zerlegt), sonst die Login-Shell des Nutzers
        let argv: Vec<String> = if profile.command.trim().is_empty() {
            let mut a = vec![user_shell()];
            if profile.login_shell {
                a.push("-l".into());
            }
            a
        } else {
            match glib::shell_parse_argv(profile.command.trim()) {
                Ok(v) => v.into_iter().map(|s| s.to_string_lossy().into_owned()).collect(),
                Err(e) => {
                    self.fail(&profile.command, e.message());
                    return;
                }
            }
        };
        let dir = expand_home(profile.directory.trim())
            .or_else(|| self.0.cwd_at_start.clone())
            .or_else(|| glib::home_dir().to_str().map(String::from));
        let env = profile.valid_environment();
        let argv_ref: Vec<&str> = argv.iter().map(String::as_str).collect();
        let env_ref: Vec<&str> = env.iter().map(String::as_str).collect();
        let weak = Rc::downgrade(&self.0);
        let program = argv[0].clone();
        self.0.vte.spawn_async(
            vte::PtyFlags::DEFAULT,
            dir.as_deref(),
            &argv_ref,
            &env_ref,
            glib::SpawnFlags::SEARCH_PATH,
            || {},
            -1,
            gio::Cancellable::NONE,
            move |result| {
                let Some(inner) = weak.upgrade() else { return };
                match result {
                    Ok(pid) => {
                        inner.child_pid.set(Some(pid));
                        // Startbefehl erst jetzt tippen; die PTY puffert, bis die Shell liest
                        if let Some(text) = inner.pending_input.borrow_mut().take() {
                            inner.vte.feed_child(text.as_bytes());
                        }
                    }
                    Err(err) => Terminal(inner).fail(&program, err.message()),
                }
            },
        );
        self.0.vte.grab_focus();
    }

    fn fail(&self, program: &str, message: &str) {
        self.0.exit.set(Some(ExitStatus::Code(-1)));
        self.show_exit(&format!(
            "<b>{}</b> konnte nicht gestartet werden: {}",
            glib::markup_escape_text(program),
            glib::markup_escape_text(message)
        ));
        self.emit_state_changed();
    }

    fn show_exit(&self, markup: &str) {
        self.0.exit_label.set_markup(markup);
        self.0.exit_bar.set_visible(true);
        self.0.exit_bar.set_reveal_child(true);
    }

    fn setup_signals(&self) {
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_child_exited(move |_, status| {
            let Some(inner) = weak.upgrade() else { return };
            let t = Terminal(inner);
            let exit = decode_wait_status(status);
            t.0.exit.set(Some(exit));
            let action = ProfileManager::get().profile(&t.profile_id()).exit_action;
            let quick = t.0.started_at.get().is_some_and(|s| s.elapsed() < Duration::from_secs(2));
            match action {
                ExitAction::Close if exit == ExitStatus::Code(0) => {
                    // nach dem Signal-Handler schließen, nicht mitten im VTE-Callback
                    let weak = Rc::downgrade(&t.0);
                    glib::idle_add_local_once(move || {
                        if let Some(inner) = weak.upgrade() {
                            Terminal(inner).emit_close();
                        }
                    });
                    t.emit_state_changed();
                    return;
                }
                // Neustart, außer der Prozess endet sofort wieder (sonst Endlosschleife)
                ExitAction::Restart if !quick => {
                    t.spawn();
                    return;
                }
                _ => {}
            }
            let text = match exit {
                ExitStatus::Code(0) => "<b>Prozess beendet</b>".to_string(),
                ExitStatus::Code(c) => format!("<b>Prozess beendet</b> · Exit-Code {c}"),
                ExitStatus::Signal(s) => format!("<b>Prozess beendet</b> · Signal {s}"),
            };
            let text = if action == ExitAction::Restart { format!("{text} · kein automatischer Neustart, endete sofort") } else { text };
            t.show_exit(&text);
            t.emit_state_changed();
        });

        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_termprop_changed(None, move |_, name| {
            if name == "xterm.title" || name == "vte.cwd" {
                if let Some(inner) = weak.upgrade() {
                    Terminal(inner).emit_state_changed();
                }
            }
        });
    }

    /// Farben aus der Palette des Profils, Fassung nach Systemstil; Schrift nach System.
    fn apply_colors(&self) {
        let sm = adw::StyleManager::default();
        let manager = ProfileManager::get();
        let palette = manager.palette(&manager.profile(&self.profile_id()).palette);
        let face = palette.face(sm.is_dark());
        let colors: Vec<gtk::gdk::RGBA> = face.colors.iter().map(rgba).collect();
        let refs: Vec<&gtk::gdk::RGBA> = colors.iter().collect();
        // Kontextfarbe: Hintergrund leicht in Richtung der Palettenfarbe für root/SSH tönen,
        // Linie und Chip in der kräftigen Farbe (Rot bzw. Blau des Schemas)
        let (background, accent) = match (&*self.0.context.borrow(), crate::settings::context_colors()) {
            (Context::Superuser, true) => (face.background.mix(&face.superuser_background, 0.35), Some((face.colors[9], face.superuser_background))),
            (Context::Remote(_), true) => (face.background.mix(&face.remote_background, 0.35), Some((face.colors[12], face.remote_background))),
            _ => (face.background, None),
        };
        self.0.accent.set(accent);
        let vte = &self.0.vte;
        vte.set_colors(Some(&rgba(&face.foreground)), Some(&rgba(&background)), &refs);
        vte.set_color_cursor(face.cursor.as_ref().map(rgba).as_ref());
        vte.set_color_cursor_foreground(face.cursor_foreground.as_ref().map(rgba).as_ref());
        vte.set_font(Some(&pango::FontDescription::from_string(&sm.monospace_font_name())));
        self.0.background.set(background);
        for f in self.0.on_colors_changed.borrow().iter() {
            f();
        }
    }

    fn setup_links(&self) {
        let vte = &self.0.vte;
        for pattern in URL_PATTERNS {
            match vte::Regex::for_match(pattern, PCRE2_FLAGS) {
                Ok(re) => {
                    let tag = vte.match_add_regex(&re, 0);
                    vte.match_set_cursor_name(tag, "pointer");
                }
                Err(e) => glib::g_warning!("parkett", "URL-Muster: {e}"),
            }
        }
        // Strg+Klick öffnet den Link; ohne Strg bleibt Klicken Markieren
        let click = gtk::GestureClick::builder().button(1).propagation_phase(gtk::PropagationPhase::Capture).build();
        let weak = Rc::downgrade(&self.0);
        click.connect_pressed(move |g, _, x, y| {
            let Some(inner) = weak.upgrade() else { return };
            if !g.current_event_state().contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                return;
            }
            let t = Terminal(inner);
            if let Some(uri) = t.link_at(x, y) {
                g.set_state(gtk::EventSequenceState::Claimed);
                t.open_link(&uri);
            }
        });
        vte.add_controller(click);
        // Rechtsklick merkt sich den Link für das Kontextmenü
        let right = gtk::GestureClick::builder().button(3).propagation_phase(gtk::PropagationPhase::Capture).build();
        let weak = Rc::downgrade(&self.0);
        right.connect_pressed(move |_, _, x, y| {
            if let Some(inner) = weak.upgrade() {
                let t = Terminal(inner);
                let link = t.link_at(x, y);
                for name in ["open-link", "copy-link"] {
                    if let Some(a) = t.0.actions.lookup_action(name).and_downcast::<gio::SimpleAction>() {
                        a.set_enabled(link.is_some());
                    }
                }
                *t.0.menu_link.borrow_mut() = link;
            }
        });
        vte.add_controller(right);
        for (name, open) in [("open-link", true), ("copy-link", false)] {
            let a = gio::SimpleAction::new(name, None);
            a.set_enabled(false);
            let weak = Rc::downgrade(&self.0);
            a.connect_activate(move |_, _| {
                let Some(inner) = weak.upgrade() else { return };
                let t = Terminal(inner);
                let Some(uri) = t.0.menu_link.borrow().clone() else { return };
                if open {
                    t.open_link(&uri);
                } else {
                    t.0.vte.clipboard().set_text(&uri);
                }
            });
            self.0.actions.add_action(&a);
        }
        // OSC-8-Links zeigen ihr wahres Ziel als Tooltip – der sichtbare Text kann täuschen.
        // vte4-rs bindet das Signal nicht, daher connect_local.
        vte.connect_local("hyperlink-hover-uri-changed", false, |args| {
            let vte = args[0].get::<vte::Terminal>().ok()?;
            let uri = args[1].get::<Option<String>>().ok().flatten();
            vte.set_tooltip_text(uri.as_deref());
            None
        });
    }

    /// OSC-8-Link oder erkannte Adresse an der Position (Widget-Koordinaten).
    pub fn link_at(&self, x: f64, y: f64) -> Option<String> {
        self.0
            .vte
            .check_hyperlink_at(x, y)
            .map(String::from)
            .or_else(|| self.0.vte.check_match_at(x, y).0.map(String::from))
            .filter(|u| !u.is_empty())
    }

    /// Öffnet einen Link mit der Standardanwendung, aber nur mit erlaubtem Schema.
    pub fn open_link(&self, uri: &str) {
        let scheme = uri.split(':').next().unwrap_or("").to_ascii_lowercase();
        if !SAFE_SCHEMES.contains(&scheme.as_str()) {
            glib::g_warning!("parkett", "Link mit Schema „{scheme}“ nicht geöffnet");
            return;
        }
        let window = self.0.vte.root().and_downcast::<gtk::Window>();
        gtk::UriLauncher::new(uri).launch(window.as_ref(), gio::Cancellable::NONE, |res| {
            if let Err(e) = res {
                glib::g_warning!("parkett", "Link nicht geöffnet: {e}");
            }
        });
    }

    /// Setzt das Suchmuster; `None` hebt die Suche auf. Gibt einen Fehler bei ungültigem Regex.
    pub fn set_search(&self, text: Option<&str>, regex: bool, case_sensitive: bool) -> Result<(), String> {
        let vte = &self.0.vte;
        let Some(text) = text.filter(|t| !t.is_empty()) else {
            vte.search_set_regex(None, 0);
            vte.unselect_all();
            return Ok(());
        };
        let pattern = if regex { text.to_string() } else { glib::Regex::escape_string(text).to_string() };
        let flags = PCRE2_FLAGS | if case_sensitive { 0 } else { PCRE2_CASELESS };
        let re = vte::Regex::for_search(&pattern, flags).map_err(|e| e.message().to_string())?;
        vte.search_set_regex(Some(&re), 0);
        vte.search_set_wrap_around(true);
        Ok(())
    }

    /// Nächster Treffer nach oben (ältere Ausgabe) oder unten. `false` = nichts gefunden.
    pub fn search_step(&self, up: bool) -> bool {
        if up { self.0.vte.search_find_previous() } else { self.0.vte.search_find_next() }
    }

    fn setup_style(&self) {
        self.apply_colors();
        // Nur schwache Referenzen: StyleManager, ProfileManager und GSettings leben so lange
        // wie die App. Eine starke Referenz hielte das Terminal fest – es würde nie freigegeben
        // und die Shell nie beendet (in der VM gefunden: Shell und „sleep“ liefen weiter).
        let style = adw::StyleManager::default();
        let weak = Rc::downgrade(&self.0);
        let reapply = move || {
            if let Some(inner) = weak.upgrade() {
                Terminal(inner).apply_colors();
            }
        };
        let (r1, r2) = (reapply.clone(), reapply.clone());
        let handlers = RefCell::new(Some((
            style.connect_dark_notify(move |_| r1()),
            style.connect_monospace_font_name_notify(move |_| r2()),
            ProfileManager::get().connect_changed(reapply),
        )));
        // Verlaufslänge folgt der Einstellung
        let vte = self.0.vte.downgrade();
        let scrollback = crate::settings::settings().connect_changed(Some("scrollback-lines"), move |_, _| {
            if let Some(vte) = vte.upgrade() {
                vte.set_scrollback_lines(crate::settings::scrollback_lines());
            }
        });
        // Kontextfarbe ein/aus sofort übernehmen
        let weak = Rc::downgrade(&self.0);
        let ctx = crate::settings::settings().connect_changed(Some("context-colors"), move |_, _| {
            if let Some(inner) = weak.upgrade() {
                let t = Terminal(inner);
                t.apply_colors();
                t.emit_state_changed();
            }
        });
        let scrollback = RefCell::new(Some((scrollback, ctx)));
        self.0.vte.connect_destroy(move |_| {
            if let Some((a, b, c)) = handlers.borrow_mut().take() {
                let style = adw::StyleManager::default();
                style.disconnect(a);
                style.disconnect(b);
                ProfileManager::get().disconnect(c);
            }
            if let Some((a, b)) = scrollback.borrow_mut().take() {
                crate::settings::settings().disconnect(a);
                crate::settings::settings().disconnect(b);
            }
        });
    }

    fn setup_actions(&self) {
        let group = self.0.actions.clone();
        let add = |name: &str, f: Box<dyn Fn(&vte::Terminal)>| {
            let a = gio::SimpleAction::new(name, None);
            let vte = self.0.vte.downgrade();
            a.connect_activate(move |_, _| {
                if let Some(vte) = vte.upgrade() {
                    f(&vte);
                }
            });
            group.add_action(&a);
        };
        add("copy", Box::new(|v| v.copy_clipboard_format(vte::Format::Text)));
        // Einfügen über Parkett: mehrzeiliger Text wird vorher gezeigt (Funktionsmatrix P0) –
        // ohne Bracketed Paste der Shell würden die Zeilen sonst sofort ausgeführt
        add("paste", Box::new(paste_with_check));
        add("select-all", Box::new(|v| v.select_all()));
        add("zoom-in", Box::new(|v| v.set_font_scale((v.font_scale() * 1.1).min(4.0))));
        add("zoom-out", Box::new(|v| v.set_font_scale((v.font_scale() / 1.1).max(0.3))));
        add("zoom-reset", Box::new(|v| v.set_font_scale(1.0)));
        self.0.vte.insert_action_group("term", Some(&group));

        self.rebuild_shortcuts();
        // Eigene Kürzel aus den Einstellungen sofort übernehmen
        let weak = Rc::downgrade(&self.0);
        let id = crate::settings::settings().connect_changed(Some("custom-shortcuts"), move |_, _| {
            if let Some(inner) = weak.upgrade() {
                Terminal(inner).rebuild_shortcuts();
            }
        });
        let id = RefCell::new(Some(id));
        self.0.vte.connect_destroy(move |_| {
            if let Some(id) = id.borrow_mut().take() {
                crate::settings::settings().disconnect(id);
            }
        });
    }

    /// Kürzel und Kontextmenü (mit Kürzelanzeige) nach aktueller Belegung aufbauen.
    /// Die Kürzel hängen am Terminal selbst und laufen in der Capture-Phase vor der
    /// Tastaturbehandlung von VTE. Globale App-Accels würden am Fenster aufgelöst und fänden
    /// die Aktionen dieses Terminals nicht (in der VM gefunden: Strg+= tippte „=“).
    fn rebuild_shortcuts(&self) {
        if let Some(old) = self.0.shortcut_controller.borrow_mut().take() {
            self.0.vte.remove_controller(&old);
        }
        let controller = crate::shortcuts::controller("term.");
        self.0.vte.add_controller(controller.clone());
        *self.0.shortcut_controller.borrow_mut() = Some(controller);

        let item = |menu: &gio::Menu, label: &str, action: &str| {
            let it = gio::MenuItem::new(Some(label), Some(action));
            if let Some(accel) = crate::shortcuts::effective(action).first() {
                it.set_attribute_value("accel", Some(&accel.to_variant()));
            }
            menu.append_item(&it);
        };
        let menu = gio::Menu::new();
        // Link-Einträge nur sichtbar, wenn beim Rechtsklick ein Link unter dem Zeiger lag
        let link = gio::Menu::new();
        for (label, action) in [("Link öffnen", "term.open-link"), ("Link kopieren", "term.copy-link")] {
            let it = gio::MenuItem::new(Some(label), Some(action));
            it.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
            link.append_item(&it);
        }
        menu.append_section(None, &link);
        let edit = gio::Menu::new();
        item(&edit, "Kopieren", "term.copy");
        item(&edit, "Einfügen", "term.paste");
        item(&edit, "Alles auswählen", "term.select-all");
        menu.append_section(None, &edit);
        let zoom = gio::Menu::new();
        item(&zoom, "Vergrößern", "term.zoom-in");
        item(&zoom, "Verkleinern", "term.zoom-out");
        item(&zoom, "Normale Größe", "term.zoom-reset");
        menu.append_section(None, &zoom);
        // Bereichsaktionen leben am Fenster (win.*); sie gelten für den fokussierten Bereich,
        // den ein Rechtsklick vorher setzt (siehe pane.rs).
        let search = gio::Menu::new();
        item(&search, "Suchen …", "win.find");
        menu.append_section(None, &search);
        let split = gio::Menu::new();
        item(&split, "Rechts teilen", "win.split-right");
        item(&split, "Unten teilen", "win.split-down");
        item(&split, "Bereich maximieren", "win.toggle-zoom");
        item(&split, "Größen angleichen", "win.equalize");
        menu.append_section(None, &split);
        let close = gio::Menu::new();
        item(&close, "Bereich schließen", "win.close-pane");
        menu.append_section(None, &close);
        self.0.vte.set_context_menu_model(Some(&menu));
    }
}

/// Erlaubte Schemata beim Öffnen von Links – keine „javascript:“, „data:“ o. Ä.
const SAFE_SCHEMES: &[&str] = &["http", "https", "ftp", "mailto", "file"];

/// Adressen ohne OSC 8 erkennen (PCRE2). Satzzeichen am Ende gehören nicht dazu.
const URL_PATTERNS: &[&str] = &[
    r#"\b(?:https?|ftp|file)://[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}]"#,
    r#"\bmailto:[^\s<>"'`@]+@[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}]"#,
];

/// PCRE2-Flags wie bei Ptyxis: UTF, UCP und MULTILINE (verlangt VTE).
pub const PCRE2_FLAGS: u32 = 0x0008_0000 | 0x4000_0000 | 0x0002_0000 | 0x0000_0400;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;


fn paste_with_check(vte: &vte::Terminal) {
    let vte = vte.clone();
    vte.clipboard().read_text_async(gio::Cancellable::NONE, move |res| {
        let Ok(Some(text)) = res else { return };
        let text = text.to_string();
        let lines = text.trim_end_matches('\n').lines().count();
        if lines <= 1 || !crate::settings::settings().boolean("warn-multiline-paste") {
            vte.paste_text(&text);
            return;
        }
        let preview: Vec<&str> = text.lines().take(6).collect();
        let more = if lines > 6 { format!("\n… ({} weitere Zeilen)", lines - 6) } else { String::new() };
        let body = format!("Der Text hat {lines} Zeilen. Unterstützt die Shell kein „Bracketed Paste“, werden sie sofort ausgeführt.");
        let dialog = adw::AlertDialog::new(Some("Mehrzeiligen Text einfügen?"), Some(&body));
        let label = gtk::Label::builder()
            .label(format!("{}{more}", preview.join("\n")))
            .xalign(0.0)
            .wrap(true)
            .selectable(true)
            .css_classes(["monospace"])
            .margin_top(8)
            .margin_bottom(8)
            .margin_start(10)
            .margin_end(10)
            .build();
        let frame = gtk::Frame::builder().child(&label).build();
        dialog.set_extra_child(Some(&frame));
        dialog.add_responses(&[("cancel", "Abbrechen"), ("paste", "Einfügen")]);
        dialog.set_response_appearance("paste", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let v2 = vte.clone();
        dialog.connect_response(Some("paste"), move |_, _| v2.paste_text(&text));
        dialog.present(Some(&vte));
    });
}

fn rgba(c: &Rgb) -> gtk::gdk::RGBA {
    gtk::gdk::RGBA::new(c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, 1.0)
}

/// `~` und `~/…` zum Persönlichen Ordner auflösen; leer = keine Vorgabe.
fn expand_home(dir: &str) -> Option<String> {
    if dir.is_empty() {
        return None;
    }
    let home = glib::home_dir();
    match dir.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => Some(format!("{}{rest}", home.display())),
        _ => Some(dir.to_string()),
    }
}

/// Login-Shell des Benutzers laut VTE (passwd), sonst $SHELL, sonst /bin/sh.
fn user_shell() -> String {
    let from_vte: Option<String> =
        unsafe { from_glib_full::<_, Option<glib::GString>>(vte::ffi::vte_get_user_shell()) }.map(String::from);
    from_vte
        .or_else(|| std::env::var("SHELL").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".into())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
}

/// `child-exited` liefert den rohen wait()-Status.
fn decode_wait_status(status: i32) -> ExitStatus {
    if status & 0x7f == 0 {
        ExitStatus::Code((status >> 8) & 0xff)
    } else {
        ExitStatus::Signal(status & 0x7f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_status_decoding() {
        assert_eq!(decode_wait_status(0), ExitStatus::Code(0));
        assert_eq!(decode_wait_status(101 << 8), ExitStatus::Code(101));
        assert_eq!(decode_wait_status(9), ExitStatus::Signal(9));
    }
}
