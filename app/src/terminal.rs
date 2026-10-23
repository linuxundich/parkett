// SPDX-License-Identifier: GPL-3.0-or-later
//! Ein Terminal-Bereich: VTE-Widget, Shell-Prozess und die Leiste „Prozess beendet“.
//!
//! Zustand (Titel, Verzeichnis) kommt aus den VTE-Termprops (`xterm.title`, `vte.cwd`),
//! nicht aus den seit VTE 0.78 veralteten Signalen.

use std::cell::{Cell, RefCell};
use std::os::fd::AsRawFd;
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk::glib::translate::{from_glib_full, ToGlibPtr};
use gtk::{gio, glib, pango};
use vte::prelude::*;

use parkett_core::context::{self, Context};
use parkett_core::palette::Rgb;
use parkett_core::profile::{Bell, CursorBlink, CursorShape, ExitAction};

use crate::profiles::ProfileManager;

/// Kleinste sinnvolle Größe eines Bereichs in Zellen (siehe docs/ANALYSIS.md, Funktionsmatrix).
pub const MIN_COLUMNS: i64 = 20;
pub const MIN_ROWS: i64 = 5;

type Callback = Box<dyn Fn()>;
type InputCallback = Box<dyn Fn(&str)>;

struct Inner {
    root: gtk::Box,
    vte: vte::Terminal,
    exit_bar: gtk::Revealer,
    exit_icon: gtk::Image,
    exit_label: gtk::Label,
    cwd_at_start: Option<String>,
    command: Option<Vec<String>>,
    environment: Option<Vec<String>>,
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
    /// letzte Ausgabe im Terminal (Startbefehl erst, wenn sie ruht, R-09)
    last_output: Cell<Instant>,
    /// root/SSH im Vordergrund (Kontextfarbe)
    context: RefCell<Context>,
    /// Akzentfarbe des Kontexts für Linie und Chip
    accent: Cell<Option<(Rgb, Rgb)>>,
    child_pid: Cell<Option<glib::Pid>>,
    exit: Cell<Option<ExitStatus>>,
    on_state_changed: RefCell<Vec<Callback>>,
    /// Eingabe des Nutzers (Tasten, Einfügen), für die gemeinsame Eingabe im Reiter
    on_input: RefCell<Vec<InputCallback>>,
    /// von der gemeinsamen Eingabe des Reiters ausgenommen
    sync_excluded: Cell<bool>,
    on_close: RefCell<Vec<Callback>>,
    /// Glocke oder Befehlsende, seit der Reiter zuletzt angesehen wurde
    attention: Cell<bool>,
    /// fortlaufende Nummer, um das Terminal aus einer Benachrichtigung wiederzufinden
    uid: u64,
    /// Start des laufenden Befehls (Termprop vte.shell.preexec aus vte.sh)
    command_started: Cell<Option<Instant>>,
    /// Name des laufenden Befehls, sekündlich gemerkt (für die Benachrichtigung)
    command_name: RefCell<Option<String>>,
    /// Programm im kurzen Reiter-Label, zuletzt gemeldet (QA-17)
    label_program: RefCell<Option<String>>,
    /// nächste Abfrage des Vordergrundprozesses (`watch_context`)
    poll_source: RefCell<Option<glib::SourceId>>,
    /// Deckkraft des Hintergrunds aus dem Profil
    opacity: Cell<f64>,
    /// Von Parkett beendet (Schließen): kein Neustart, keine Leiste (QA-03)
    terminated: Cell<bool>,
}

/// Ab dieser Laufzeit meldet ein Befehl sein Ende, wenn sein Bereich nicht zu sehen ist.
const NOTIFY_AFTER: std::time::Duration = std::time::Duration::from_secs(10);

static NEXT_UID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[derive(Clone)]
pub struct Terminal(Rc<Inner>);

/// Vordergrund der PTY (`read_foreground`).
struct Foreground {
    pgid: i32,
    /// die Shell selbst wartet am Prompt
    is_shell: bool,
    comm: String,
    args: Vec<String>,
    euid: Option<u32>,
}

/// Schwache Referenz für Closures an Widgets des Bereichs (Draw-Funktionen): eine starke
/// hielte das Terminal fest, solange das Widget lebt (L2).
#[derive(Clone)]
pub struct WeakTerminal(std::rc::Weak<Inner>);

impl WeakTerminal {
    pub fn upgrade(&self) -> Option<Terminal> {
        self.0.upgrade().map(Terminal)
    }
}

/// Was ein Bereich abweichend vom Profil startet (Befehlszeile).
#[derive(Clone, Default)]
pub struct Launch {
    /// statt Shell bzw. Profilbefehl ausführen (`parkett -- …`)
    pub command: Option<Vec<String>>,
    /// Umgebung des Aufrufers, wenn eine laufende Instanz den Aufruf bekommt (QA-08);
    /// `None` = die Umgebung von Parkett
    pub environment: Option<Vec<String>>,
}

impl Terminal {
    pub fn new(cwd: Option<String>, profile_id: &str, launch: Launch) -> Self {
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
        let exit_icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
        bar.append(&exit_icon);
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
            exit_icon,
            exit_label,
            cwd_at_start: cwd,
            command: launch.command.filter(|c| !c.is_empty()),
            environment: launch.environment,
            profile_id: RefCell::new(profile_id.to_string()),
            started_at: Cell::new(None),
            background: Cell::new(Rgb { r: 255, g: 255, b: 255 }),
            on_colors_changed: RefCell::default(),
            menu_link: RefCell::new(None),
            actions: gio::SimpleActionGroup::new(),
            shortcut_controller: RefCell::new(None),
            pending_input: RefCell::new(None),
            last_output: Cell::new(Instant::now()),
            context: RefCell::new(Context::Local),
            accent: Cell::new(None),
            child_pid: Cell::new(None),
            exit: Cell::new(None),
            on_state_changed: RefCell::default(),
            on_input: RefCell::default(),
            sync_excluded: Cell::new(false),
            on_close: RefCell::default(),
            attention: Cell::new(false),
            uid: NEXT_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            command_started: Cell::new(None),
            command_name: RefCell::new(None),
            label_program: RefCell::new(None),
            poll_source: RefCell::new(None),
            opacity: Cell::new(1.0),
            terminated: Cell::new(false),
        }));

        t.setup_style();
        t.setup_actions();
        t.setup_signals();
        t.setup_links();
        t.watch_context();

        let weak = Rc::downgrade(&t.0);
        restart.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                Terminal(inner).spawn(true);
            }
        });
        let weak = Rc::downgrade(&t.0);
        close.connect_clicked(move |_| {
            if let Some(inner) = weak.upgrade() {
                Terminal(inner).emit_close();
            }
        });

        t.spawn(true);
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

    /// Text, den der Nutzer in dieses Terminal tippt oder einfügt (VTE „commit“).
    pub fn connect_input(&self, f: impl Fn(&str) + 'static) {
        self.0.on_input.borrow_mut().push(Box::new(f));
    }

    /// Text an den Prozess schicken, als hätte der Nutzer ihn hier getippt.
    pub fn feed(&self, text: &str) {
        if self.0.exit.get().is_none() {
            self.0.vte.feed_child(text.as_bytes());
        }
    }

    pub fn sync_excluded(&self) -> bool {
        self.0.sync_excluded.get()
    }

    pub fn set_sync_excluded(&self, excluded: bool) {
        self.0.sync_excluded.set(excluded);
    }

    pub fn connect_colors_changed(&self, f: impl Fn() + 'static) {
        self.0.on_colors_changed.borrow_mut().push(Box::new(f));
    }

    pub fn downgrade(&self) -> WeakTerminal {
        WeakTerminal(Rc::downgrade(&self.0))
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
    /// Hintergrund nicht voll deckend (Profil „Deckkraft“ unter 100 %).
    pub fn is_translucent(&self) -> bool {
        self.0.opacity.get() < 1.0
    }

    pub fn background(&self) -> Rgb {
        self.0.background.get()
    }

    pub fn connect_close(&self, f: impl Fn() + 'static) {
        self.0.on_close.borrow_mut().push(Box::new(f));
    }

    /// Fenstertitel, den die Shell per OSC 0/2 gesetzt hat; sonst der Befehl des Profils.
    pub fn title(&self) -> Option<String> {
        self.0.vte.termprop_string("xterm.title").0.map(String::from).filter(|s| !s.is_empty()).or_else(|| {
            // Befehl von der Befehlszeile, sonst der des Profils
            let command = match &self.0.command {
                Some(c) => c.join(" "),
                None => ProfileManager::get().profile(&self.profile_id()).command,
            };
            Some(command.trim().to_string()).filter(|c| !c.is_empty())
        })
    }

    /// Arbeitsverzeichnis für die Sitzung: `vte.cwd`, sonst das des Prozesses (/proc), sonst
    /// das beim Start – Bereiche ohne Shell-Integration melden kein `vte.cwd`.
    pub fn working_directory(&self) -> Option<String> {
        self.cwd()
            .or_else(|| {
                let pid = self.0.child_pid.get()?.0;
                std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?.to_str().map(String::from)
            })
            .or_else(|| self.0.cwd_at_start.clone())
    }

    /// Arbeitsverzeichnis aus `vte.cwd` (OSC 7, setzt vte.sh in der Shell voraus).
    /// vte4-rs bindet `ref_termprop_uri` nicht; daher der eine FFI-Aufruf.
    /// Nur Pfade dieses Rechners, dekodiert: Ein OSC 7 aus einer SSH-Sitzung nennt den Pfad
    /// auf dem Server und darf kein lokales Startverzeichnis setzen (QA-05, QA-06).
    pub fn cwd(&self) -> Option<String> {
        let uri: Option<glib::Uri> = unsafe {
            from_glib_full(vte::ffi::vte_terminal_ref_termprop_uri(self.0.vte.to_glib_none().0, c"vte.cwd".as_ptr()))
        };
        uri.and_then(|u| parkett_core::uri::local_path(u.to_str().as_str(), glib::host_name().as_str()))
    }

    /// Wie der Prozess geendet hat; `None`, solange er läuft.
    pub fn exit_status(&self) -> Option<ExitStatus> {
        self.0.exit.get()
    }

    /// Beendet die Shell (SIGHUP wie beim Schließen eines Terminalfensters; die Shell gibt
    /// es an ihre Jobs weiter). VTE tut das auch beim Freigeben des Widgets – hier
    /// ausdrücklich, damit kein Prozess an einer übersehenen Referenz hängen bleibt.
    pub fn terminate(&self) {
        self.0.terminated.set(true);
        if self.0.exit.get().is_none() {
            if let Some(pid) = self.0.child_pid.get() {
                unsafe { libc::kill(pid.0, libc::SIGHUP) };
            }
        }
    }

    pub fn uid(&self) -> u64 {
        self.0.uid
    }

    /// Ist das Terminal gerade zu sehen (Reiter gewählt, Fenster aktiv)?
    fn is_seen(&self) -> bool {
        let vte = &self.0.vte;
        vte.is_mapped() && vte.root().and_downcast::<gtk::Window>().is_some_and(|w| w.is_active())
    }

    /// Ein Befehl ist fertig: lief er lange und ist der Bereich nicht zu sehen, Reiter
    /// markieren und eine Benachrichtigung schicken (Klick darauf zeigt den Bereich).
    fn command_finished(&self, took: std::time::Duration) {
        if took < NOTIFY_AFTER || self.is_seen() {
            return;
        }
        self.0.attention.set(true);
        self.emit_state_changed();
        if !crate::settings::settings().boolean("notify-command-finished") {
            return;
        }
        let Some(app) = gio::Application::default() else { return };
        let secs = took.as_secs_f64().round() as u64;
        let duration = if secs >= 60 { format!("{} min {} s", secs / 60, secs % 60) } else { format!("{secs} s") };
        let n = gio::Notification::new("Befehl beendet");
        let what = self.0.command_name.borrow_mut().take().or_else(|| self.title()).unwrap_or_else(|| "Befehl".into());
        n.set_body(Some(&format!("{what} · nach {duration}")));
        n.set_default_action_and_target_value("app.show-terminal", Some(&self.0.uid.to_variant()));
        app.send_notification(Some(&format!("command-{}", self.0.uid)), &n);
    }

    /// Liefert und löscht die Markierung (Glocke, Befehlsende).
    pub fn take_attention(&self) -> bool {
        self.0.attention.replace(false)
    }

    pub fn context(&self) -> Context {
        self.0.context.borrow().clone()
    }

    /// (Linie, Chip-Hintergrund) für den aktuellen Kontext; `None` = lokal oder abgeschaltet.
    pub fn context_accent(&self) -> Option<(Rgb, Rgb)> {
        self.0.accent.get()
    }

    /// Fragt den Vordergrundprozess ab (wie Ptyxis, aber je Bereich): jede Sekunde, solange
    /// der Bereich zu sehen und sein Fenster aktiv ist, sonst alle 5 s – Reiter im Hintergrund
    /// und Fenster ohne Fokus wecken die CPU seltener (L3). Kommt der Bereich in Sicht oder
    /// bekommt er den Fokus, sofort. 5 s reichen auch für die Benachrichtigung: Sie meldet nur
    /// Befehle ab 10 s Laufzeit, deren Namen kennt die Abfrage bis dahin.
    fn watch_context(&self) {
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_map(move |_| {
            if let Some(inner) = weak.upgrade() {
                Terminal(inner).poll_now();
            }
        });
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_has_focus_notify(move |vte| {
            if let Some(inner) = weak.upgrade().filter(|_| vte.has_focus()) {
                Terminal(inner).poll_now();
            }
        });
        self.poll_now();
    }

    /// Abfrage jetzt, dann die nächste planen (eine geplante wird ersetzt).
    fn poll_now(&self) {
        if let Some(id) = self.0.poll_source.borrow_mut().take() {
            id.remove();
        }
        self.poll();
        let watched =
            self.0.vte.is_mapped() && self.0.vte.root().and_downcast::<gtk::Window>().is_some_and(|w| w.is_active());
        let weak = Rc::downgrade(&self.0);
        let id = glib::timeout_add_local_once(Duration::from_millis(if watched { 1000 } else { 5000 }), move || {
            if let Some(inner) = weak.upgrade() {
                // schon abgelaufen: nur vergessen, nicht entfernen
                inner.poll_source.borrow_mut().take();
                Terminal(inner).poll_now();
            }
        });
        *self.0.poll_source.borrow_mut() = Some(id);
    }

    fn poll(&self) {
        // Vordergrund einmal lesen, für Benachrichtigung, Kontext und Label (D2)
        let fg = self.read_foreground();
        let foreground = Self::name_of(fg.as_ref());
        if self.0.command_started.get().is_some() {
            if let Some(name) = &foreground {
                *self.0.command_name.borrow_mut() = Some(name.clone());
            }
        }
        let now = self.context_of(fg.as_ref());
        let mut changed = false;
        if now != *self.0.context.borrow() {
            *self.0.context.borrow_mut() = now;
            self.apply_colors();
            changed = true;
        }
        let program = self.label_program_with(foreground);
        if program != *self.0.label_program.borrow() {
            *self.0.label_program.borrow_mut() = program;
            changed = true;
        }
        if changed {
            self.emit_state_changed();
        }
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

    /// Vordergrund der PTY, einmal gelesen für Kontext, Label, Benachrichtigung und
    /// Vorlage (D2). Muster wie Ptyxis (`agent/ptyxis-process-impl.c`): tcgetpgrp auf der
    /// PTY, Angaben aus /proc. Nur ohne Flatpak-Sandbox aussagekräftig.
    fn read_foreground(&self) -> Option<Foreground> {
        if self.0.exit.get().is_some() {
            return None;
        }
        let pty = self.0.vte.pty()?;
        let pgid = unsafe { libc::tcgetpgrp(pty.fd().as_raw_fd()) };
        if pgid <= 0 {
            return None;
        }
        let comm = std::fs::read_to_string(format!("/proc/{pgid}/comm")).unwrap_or_default().trim().to_string();
        let args: Vec<String> = std::fs::read(format!("/proc/{pgid}/cmdline"))
            .unwrap_or_default()
            .split(|b| *b == 0)
            .filter(|a| !a.is_empty())
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        // „Uid:\treal\teffektiv\t…“
        let euid = std::fs::read_to_string(format!("/proc/{pgid}/status")).ok().and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(2))
                .and_then(|u| u.parse().ok())
        });
        let is_shell = self.0.child_pid.get().is_some_and(|c| c.0 == pgid);
        Some(Foreground { pgid, is_shell, comm, args, euid })
    }

    fn detect_context(&self) -> Context {
        self.context_of(self.read_foreground().as_ref())
    }

    fn context_of(&self, fg: Option<&Foreground>) -> Context {
        match fg {
            Some(f) => context::classify(&f.comm, &f.args, f.euid),
            None => Context::Local,
        }
    }

    /// Befehlszeile des Vordergrundprozesses (für „Als Vorlage speichern“), sonst `None`.
    pub fn foreground_command(&self) -> Option<String> {
        let fg = self.read_foreground().filter(|f| !f.is_shell)?;
        let args: Vec<String> = fg
            .args
            .into_iter()
            .map(|a| {
                // nur quoten, wo die Shell es braucht – „npm run dev“ statt „'npm' 'run' 'dev'“
                if !a.is_empty() && a.chars().all(|c| c.is_alphanumeric() || "-_./:=@%+,".contains(c)) {
                    a
                } else {
                    glib::shell_quote(&a).to_string_lossy().into_owned()
                }
            })
            .collect();
        Some(args.join(" ")).filter(|s| !s.is_empty())
    }

    /// Programm für das kurze Reiter-Label: bei SSH „ssh host“, sonst das Programm im
    /// Vordergrund, am Prompt die Shell selbst.
    pub fn label_program(&self) -> Option<String> {
        self.label_program_with(self.foreground_process())
    }

    fn label_program_with(&self, foreground: Option<String>) -> Option<String> {
        if self.0.exit.get().is_some() {
            return None;
        }
        if let Context::Remote(host) = self.context() {
            let name = foreground.unwrap_or_else(|| "ssh".into());
            return Some(if name == host { name } else { format!("{name} {host}") });
        }
        foreground.or_else(|| {
            let pid = self.0.child_pid.get()?.0;
            let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
            Some(comm.trim().to_string()).filter(|c| !c.is_empty())
        })
    }

    /// Name des Vordergrundprozesses, wenn er nicht die Shell selbst ist (z. B. „vim“, „ssh“).
    pub fn foreground_process(&self) -> Option<String> {
        Self::name_of(self.read_foreground().as_ref())
    }

    fn name_of(fg: Option<&Foreground>) -> Option<String> {
        let f = fg.filter(|f| !f.is_shell)?;
        Some(f.comm.clone()).filter(|c| !c.is_empty()).or(Some(format!("PID {}", f.pgid)))
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

    /// `focus`: danach den Fokus ins Terminal (nicht beim automatischen Neustart im Hintergrund).
    fn spawn(&self, focus: bool) {
        self.0.terminated.set(false);
        self.0.exit_bar.set_reveal_child(false);
        self.0.exit_bar.set_visible(false);
        self.0.exit.set(None);
        self.0.child_pid.set(None);
        self.0.started_at.set(Some(Instant::now()));
        let profile = ProfileManager::get().profile(&self.profile_id());

        // Befehl aus dem Profil (wie in einer Shell zerlegt), sonst die Login-Shell des Nutzers
        let argv: Vec<String> = if let Some(cmd) = &self.0.command {
            cmd.clone()
        } else if profile.command.trim().is_empty() {
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
        // Aufruf an eine laufende Instanz: dessen Umgebung statt der von Parkett (QA-08)
        let (env, flags) = match &self.0.environment {
            Some(caller) => (
                parkett_core::environment::for_caller(caller, &own_environment(), &profile.valid_environment()),
                glib::SpawnFlags::SEARCH_PATH | glib::SpawnFlags::from_bits_retain(VTE_SPAWN_NO_PARENT_ENVV),
            ),
            None => (profile.valid_environment(), glib::SpawnFlags::SEARCH_PATH),
        };
        let argv_ref: Vec<&str> = argv.iter().map(String::as_str).collect();
        let env_ref: Vec<&str> = env.iter().map(String::as_str).collect();
        let weak = Rc::downgrade(&self.0);
        let program = argv[0].clone();
        self.0.vte.spawn_async(
            vte::PtyFlags::DEFAULT,
            dir.as_deref(),
            &argv_ref,
            &env_ref,
            flags,
            || {},
            -1,
            gio::Cancellable::NONE,
            move |result| {
                let Some(inner) = weak.upgrade() else { return };
                match result {
                    Ok(pid) => {
                        inner.child_pid.set(Some(pid));
                        if inner.pending_input.borrow().is_some() {
                            Terminal(inner).wait_to_type();
                        }
                    }
                    Err(err) => Terminal(inner).fail(&program, err.message()),
                }
            },
        );
        if focus {
            self.0.vte.grab_focus();
        }
    }

    /// Startbefehl tippen, sobald die Shell bereit ist (R-09): mit Shell-Integration beim
    /// ersten Prompt (siehe `vte.shell.precmd`), sonst wenn die Ausgabe ruht und die PTY nicht
    /// auf ein Passwort wartet – sofort getippt landete er in der Passwortabfrage von ssh.
    fn wait_to_type(&self) {
        use parkett_core::startup;
        let weak = Rc::downgrade(&self.0);
        glib::timeout_add_local(Duration::from_millis(250), move || {
            let Some(inner) = weak.upgrade() else { return glib::ControlFlow::Break };
            if inner.pending_input.borrow().is_none() {
                return glib::ControlFlow::Break;
            }
            let since_spawn = inner.started_at.get().map_or(Duration::ZERO, |s| s.elapsed());
            if inner.exit.get().is_some() || since_spawn > startup::GIVE_UP {
                inner.pending_input.borrow_mut().take();
                return glib::ControlFlow::Break;
            }
            let t = Terminal(inner);
            if startup::may_type(since_spawn, t.0.last_output.get().elapsed(), t.tty_mode()) {
                t.feed_pending();
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }

    fn feed_pending(&self) {
        if let Some(text) = self.0.pending_input.borrow_mut().take() {
            self.0.vte.feed_child(text.as_bytes());
        }
    }

    /// termios der PTY: zeilenweise Eingabe und Echo (Passwortabfrage erkennen).
    fn tty_mode(&self) -> Option<parkett_core::startup::TtyMode> {
        let pty = self.0.vte.pty()?;
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        // SAFETY: gültiger Deskriptor der PTY, `t` ist ein vollständiges termios
        if unsafe { libc::tcgetattr(pty.fd().as_raw_fd(), &mut t) } != 0 {
            return None;
        }
        Some(parkett_core::startup::TtyMode {
            canonical: t.c_lflag & libc::ICANON != 0,
            echo: t.c_lflag & libc::ECHO != 0,
        })
    }

    fn fail(&self, program: &str, message: &str) {
        self.0.exit.set(Some(ExitStatus::Code(-1)));
        self.show_exit(
            &format!(
                "<b>{}</b> konnte nicht gestartet werden: {}",
                glib::markup_escape_text(program),
                glib::markup_escape_text(message)
            ),
            true,
        );
        self.emit_state_changed();
    }

    /// `error`: Warnsymbol und rote Tönung; ein normales Ende (Exit-Code 0) bekommt eine
    /// neutrale Leiste, sonst sieht `exit` wie ein Fehler aus (QA-11).
    fn show_exit(&self, markup: &str, error: bool) {
        self.0.exit_label.set_markup(markup);
        self.0.exit_icon.set_visible(error);
        if let Some(bar) = self.0.exit_bar.child() {
            if error {
                bar.remove_css_class("neutral");
            } else {
                bar.add_css_class("neutral");
            }
        }
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
            // bewusst beendet: nicht neu starten, sonst liefe im geschlossenen Reiter
            // unsichtbar ein neuer Prozess (QA-03, z. B. eine ssh-Verbindung)
            if t.0.terminated.get() {
                return;
            }
            let action = ProfileManager::get().profile(&t.profile_id()).exit_action;
            let quick = t.0.started_at.get().is_some_and(|s| s.elapsed() < Duration::from_secs(2));
            match action {
                ExitAction::Close if exit == ExitStatus::Code(0) => {
                    // Leiste schon jetzt füllen: Holt „Rückgängig“ den Bereich zurück, ist er
                    // sonst ein totes Terminal ohne Hinweis und ohne „Neu starten“ (R-05)
                    t.show_exit("<b>Prozess beendet</b>", false);
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
                    t.spawn(false);
                    return;
                }
                _ => {}
            }
            let text = match exit {
                ExitStatus::Code(0) => "<b>Prozess beendet</b>".to_string(),
                ExitStatus::Code(c) => format!("<b>Prozess beendet</b> · Exit-Code {c}"),
                ExitStatus::Signal(s) => format!("<b>Prozess beendet</b> · Signal {s}"),
            };
            let text = if action == ExitAction::Restart {
                format!("{text} · kein automatischer Neustart, endete sofort")
            } else {
                text
            };
            // ein Neustart-Profil, dessen Prozess sofort endet, ist ein Problem, auch mit Code 0
            t.show_exit(&text, exit != ExitStatus::Code(0) || action == ExitAction::Restart);
            t.emit_state_changed();
        });

        // Glocke: Reiter markieren (Ton selbst macht VTE, siehe set_audible_bell)
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_bell(move |_| {
            let Some(inner) = weak.upgrade() else { return };
            let t = Terminal(inner);
            if ProfileManager::get().profile(&t.profile_id()).bell != Bell::None {
                t.0.attention.set(true);
                t.emit_state_changed();
            }
        });

        // Eingabe für die gemeinsame Eingabe im Reiter weiterreichen
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_commit(move |_, text, _| {
            if let Some(inner) = weak.upgrade() {
                for f in inner.on_input.borrow().iter() {
                    f(text);
                }
            }
        });

        // Zeitpunkt der letzten Ausgabe für den Startbefehl (R-09)
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_contents_changed(move |_| {
            if let Some(inner) = weak.upgrade() {
                inner.last_output.set(Instant::now());
            }
        });

        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_termprop_changed(None, move |_, name| {
            let Some(inner) = weak.upgrade() else { return };
            match name {
                "xterm.title" | "vte.cwd" => Terminal(inner).emit_state_changed(),
                // Shell-Integration (vte.sh): Befehl beginnt bzw. Prompt kommt wieder
                "vte.shell.preexec" => {
                    inner.command_started.set(Some(Instant::now()));
                    inner.command_name.borrow_mut().take();
                }
                "vte.shell.precmd" => {
                    // erster Prompt: wartender Startbefehl kann jetzt kommen (R-09)
                    Terminal(inner.clone()).feed_pending();
                    if let Some(start) = inner.command_started.take() {
                        Terminal(inner).command_finished(start.elapsed());
                    }
                }
                _ => {}
            }
        });
    }

    /// Farben aus der Palette des Profils (Fassung nach Systemstil), Schrift, Cursor, Glocke.
    fn apply_colors(&self) {
        let sm = adw::StyleManager::default();
        let manager = ProfileManager::get();
        let chosen = manager.profile(&self.profile_id()).palette;
        let high_contrast = sm.is_high_contrast() && crate::settings::follow_high_contrast();
        let palette = manager.palette(parkett_core::palette::effective_id(&chosen, high_contrast));
        let face = palette.face(sm.is_dark());
        let colors: Vec<gtk::gdk::RGBA> = face.colors.iter().map(rgba).collect();
        let refs: Vec<&gtk::gdk::RGBA> = colors.iter().collect();
        // Kontextfarbe: Hintergrund leicht in Richtung der Palettenfarbe für root/SSH tönen,
        // Linie und Chip in der kräftigen Farbe (Rot bzw. Blau des Schemas)
        let (background, accent) = match (&*self.0.context.borrow(), crate::settings::context_colors()) {
            (Context::Superuser, true) => (
                face.background.mix(&face.superuser_background, 0.35),
                Some((face.colors[9], face.superuser_background)),
            ),
            (Context::Remote(_), true) => {
                (face.background.mix(&face.remote_background, 0.35), Some((face.colors[12], face.remote_background)))
            }
            _ => (face.background, None),
        };
        self.0.accent.set(accent);
        let vte = &self.0.vte;
        // Transparenz: nur der Hintergrund bekommt Alpha; das Fenster dahinter wird dafür
        // durchsichtig geschaltet (Window::update_translucency)
        let opacity = manager.profile(&self.profile_id()).opacity();
        let mut bg = rgba(&background);
        bg.set_alpha(opacity as f32);
        vte.set_colors(Some(&rgba(&face.foreground)), Some(&bg), &refs);
        self.0.opacity.set(opacity);
        vte.set_color_cursor(face.cursor.as_ref().map(rgba).as_ref());
        vte.set_color_cursor_foreground(face.cursor_foreground.as_ref().map(rgba).as_ref());
        // Auswahl in der Akzentfarbe statt invertiert
        vte.set_color_highlight(Some(&rgba(&face.selection(&from_rgba(&sm.accent_color_rgba())))));
        // Schrift, Cursor und Glocke aus dem Profil
        let profile = manager.profile(&self.profile_id());
        let font = if profile.font.trim().is_empty() { sm.monospace_font_name() } else { profile.font.trim().into() };
        vte.set_font(Some(&pango::FontDescription::from_string(&font)));
        vte.set_cursor_shape(match profile.cursor_shape {
            CursorShape::Block => vte::CursorShape::Block,
            CursorShape::Ibeam => vte::CursorShape::Ibeam,
            CursorShape::Underline => vte::CursorShape::Underline,
        });
        // blinkender Text (SGR 5) nur im Bereich mit Fokus, wie gnome-terminal: in allen
        // Bereichen kostete er im Leerlauf 7,8 % CPU in der VM (QA-14)
        vte.set_text_blink_mode(vte::TextBlinkMode::Focused);
        vte.set_cursor_blink_mode(match profile.cursor_blink {
            CursorBlink::System => vte::CursorBlinkMode::System,
            CursorBlink::On => vte::CursorBlinkMode::On,
            CursorBlink::Off => vte::CursorBlinkMode::Off,
        });
        vte.set_audible_bell(profile.bell == Bell::Sound);
        vte.set_enable_sixel(sixel_supported() && crate::settings::settings().boolean("enable-sixel"));
        self.0.background.set(background);
        for f in self.0.on_colors_changed.borrow().iter() {
            f();
        }
        // Fenster prüft Durchsichtigkeit, Reiter ihre Anzeige
        self.emit_state_changed();
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
        // file:-Links nur auf diesem Rechner: Ein OSC-8-Link aus einer SSH-Sitzung nennt den
        // fremden Rechner; dort gemeint ist eine andere Datei als hier (wie gnome-terminal).
        // Viele Programme lassen den Rechner weg (`file:///pfad`) – läuft im Bereich gerade
        // eine Verbindung, ist auch der nicht von hier (R-04)
        if scheme == "file"
            && (matches!(self.detect_context(), Context::Remote(_))
                || !parkett_core::uri::is_local_file(uri, glib::host_name().as_str()))
        {
            glib::g_warning!("parkett", "file:-Link auf anderen Rechner nicht geöffnet: {uri}");
            if let Some(w) = self.0.vte.root().and_downcast::<gtk::Window>() {
                let d = adw::AlertDialog::new(
                    Some("Link nicht geöffnet"),
                    Some("Der Link zeigt auf eine Datei auf einem anderen Rechner."),
                );
                d.add_response("ok", "OK");
                d.present(Some(&w));
            }
            return;
        }
        let window = self.0.vte.root().and_downcast::<gtk::Window>();
        let weak = window.as_ref().map(|w| w.downgrade());
        gtk::UriLauncher::new(uri).launch(window.as_ref(), gio::Cancellable::NONE, move |res| {
            let Err(e) = res else { return };
            glib::g_warning!("parkett", "Link nicht geöffnet: {e}");
            // Sonst geschieht nach Strg+Klick einfach nichts (VM 2026-10-22: Standard-App fehlte)
            if e.matches(gtk::DialogError::Dismissed) || e.matches(gtk::DialogError::Cancelled) {
                return;
            }
            if let Some(w) = weak.and_then(|w| w.upgrade()).and_downcast::<crate::window::Window>() {
                w.toast(&format!("Link nicht geöffnet: {}", e.message()));
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

    /// Treffer im ganzen Verlauf (QA-19), höchstens `SEARCH_COUNT_MAX`. VTE zählt selbst
    /// nicht; gezählt wird mit GRegex, das wie VTE auf PCRE2 baut. Umbrochene Zeilen liefert
    /// VTE zusammenhängend, ein Treffer über den Umbruch zählt also mit.
    pub fn search_count(&self, text: &str, regex: bool, case_sensitive: bool) -> Option<usize> {
        if text.is_empty() {
            return None;
        }
        let pattern = if regex { text.to_string() } else { glib::Regex::escape_string(text).to_string() };
        let mut flags = glib::RegexCompileFlags::MULTILINE | glib::RegexCompileFlags::OPTIMIZE;
        if !case_sensitive {
            flags |= glib::RegexCompileFlags::CASELESS;
        }
        let re = glib::Regex::new(&pattern, flags, glib::RegexMatchFlags::empty()).ok()??;
        let vte = &self.0.vte;
        let adj = vte.vadjustment()?;
        let (first, last) = (adj.lower() as libc::c_long, adj.upper() as libc::c_long);
        // Von unten in Blöcken lesen und bei SEARCH_COUNT_MAX aufhören: Den ganzen Verlauf als
        // Text zu holen kostete bei 300 000 Zeilen etwa 0,27 s je Taste (VM). Ein Treffer über
        // eine Blockgrenze (umbrochene Zeile genau dort) kann fehlen – die Zahl ist ein Überblick.
        let mut n = 0;
        let mut end = last;
        while end > first && n < SEARCH_COUNT_MAX {
            // Zeilen start..end, ohne end (die gehört zum Block darunter)
            let start = (end - SEARCH_COUNT_ROWS).max(first);
            let (content, _) = vte.text_range_format(vte::Format::Text, start, 0, end - 1, vte.column_count());
            if let Some(content) = content {
                if let Ok(info) = re.match_(content.as_gstr(), glib::RegexMatchFlags::empty()) {
                    while info.matches() && n < SEARCH_COUNT_MAX {
                        n += 1;
                        if !info.next().unwrap_or(false) {
                            break;
                        }
                    }
                }
            }
            end = start;
        }
        Some(n)
    }

    /// Nächster Treffer nach oben (ältere Ausgabe) oder unten. `false` = nichts gefunden.
    pub fn search_step(&self, up: bool) -> bool {
        if up {
            self.0.vte.search_find_previous()
        } else {
            self.0.vte.search_find_next()
        }
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
        let (r1, r2, r3, r4) = (reapply.clone(), reapply.clone(), reapply.clone(), reapply.clone());
        let handlers = RefCell::new(Some((
            [
                style.connect_dark_notify(move |_| r1()),
                style.connect_monospace_font_name_notify(move |_| r2()),
                style.connect_high_contrast_notify(move |_| r3()),
                style.connect_accent_color_rgba_notify(move |_| r4()),
            ],
            ProfileManager::get().connect_changed(reapply),
        )));
        // Verlaufslänge folgt der Einstellung
        let vte = self.0.vte.downgrade();
        let scrollback = crate::settings::settings().connect_changed(Some("scrollback-lines"), move |_, _| {
            if let Some(vte) = vte.upgrade() {
                vte.set_scrollback_lines(crate::settings::scrollback_lines());
            }
        });
        // Kontextfarbe und Sixel ein/aus sofort übernehmen
        let weak = Rc::downgrade(&self.0);
        let ctx = crate::settings::settings().connect_changed(None, move |_, key| {
            if !matches!(key, "context-colors" | "enable-sixel" | "follow-high-contrast") {
                return;
            }
            if let Some(inner) = weak.upgrade() {
                let t = Terminal(inner);
                t.apply_colors();
                t.emit_state_changed();
            }
        });
        let scrollback = RefCell::new(Some((scrollback, ctx)));
        self.0.vte.connect_destroy(move |_| {
            if let Some((style_ids, c)) = handlers.borrow_mut().take() {
                let style = adw::StyleManager::default();
                for id in style_ids {
                    style.disconnect(id);
                }
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
        let sync = gio::Menu::new();
        item(&sync, "Eingabe an alle Bereiche", "win.toggle-sync");
        let it = gio::MenuItem::new(Some("Dieser Bereich empfängt gemeinsame Eingabe"), Some("win.sync-pane"));
        it.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
        sync.append_item(&it);
        menu.append_section(None, &sync);
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

/// PCRE2-Flags wie bei Ptyxis: UTF, NO_UTF_CHECK, UCP und MULTILINE (verlangt VTE).
/// `VTE_SPAWN_NO_PARENT_ENVV` (vtepty.h): nur `envv` statt der Umgebung von Parkett; TERM,
/// COLORTERM und VTE_VERSION setzt VTE trotzdem. Fehlt in vte4-rs.
const VTE_SPAWN_NO_PARENT_ENVV: u32 = 1 << 25;

/// Darüber zeigt die Suche „999+“: Zählen kostet bei langem Verlauf sonst spürbar Zeit.
pub const SEARCH_COUNT_MAX: usize = 999;
/// Zeilen je Block beim Zählen (von unten nach oben)
const SEARCH_COUNT_ROWS: libc::c_long = 20_000;

pub const PCRE2_FLAGS: u32 = 0x0008_0000 | 0x4000_0000 | 0x0002_0000 | 0x0000_0400;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;

fn paste_with_check(vte: &vte::Terminal) {
    let vte = vte.clone();
    vte.clipboard().read_text_async(gio::Cancellable::NONE, move |res| {
        let Ok(Some(text)) = res else { return };
        let text = text.to_string();
        // Jeder Umbruch zählt, auch einer am Ende einer einzelnen Zeile (QA-04)
        let check = parkett_core::paste::check(&text);
        if !check.executes || !crate::settings::settings().boolean("warn-multiline-paste") {
            vte.paste_text(&text);
            return;
        }
        let lines = check.lines.len();
        let preview: Vec<&str> = check.lines.iter().take(6).copied().collect();
        let more = if lines > 6 { format!("\n… ({} weitere Zeilen)", lines - 6) } else { String::new() };
        let (heading, body) = if lines == 1 {
            (
                "Text mit Zeilenumbruch einfügen?",
                "Der Text endet mit einem Zeilenumbruch. Unterstützt die Shell kein „Bracketed Paste“, wird er sofort ausgeführt.".to_string(),
            )
        } else {
            (
                "Mehrzeiligen Text einfügen?",
                format!(
                    "Der Text hat {lines} Zeilen. Unterstützt die Shell kein „Bracketed Paste“, werden sie sofort ausgeführt."
                ),
            )
        };
        let dialog = adw::AlertDialog::new(Some(heading), Some(&body));
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

/// Ist die installierte VTE mit Sixel gebaut? (Arch und Fedora 44 sind es nicht.)
pub fn sixel_supported() -> bool {
    // vte4-rs bindet vte_get_feature_flags nicht; reine Abfrage ohne Argumente
    let flags = unsafe { vte::ffi::vte_get_feature_flags() };
    flags & vte::ffi::VTE_FEATURE_FLAG_SIXEL != 0
}

fn rgba(c: &Rgb) -> gtk::gdk::RGBA {
    let (r, g, b) = c.to_f64();
    gtk::gdk::RGBA::new(r as f32, g as f32, b as f32, 1.0)
}

fn from_rgba(c: &gtk::gdk::RGBA) -> Rgb {
    Rgb::from_f64(c.red().into(), c.green().into(), c.blue().into())
}

/// `~` und `~/…` zum Persönlichen Ordner auflösen; leer = keine Vorgabe.
fn expand_home(dir: &str) -> Option<String> {
    (!dir.is_empty()).then(|| parkett_core::paths::expand_home(dir, &glib::home_dir()).to_string_lossy().into_owned())
}

/// Login-Shell des Benutzers laut VTE (passwd), sonst $SHELL, sonst /bin/sh.
/// Umgebung von Parkett als `NAME=wert`.
fn own_environment() -> Vec<String> {
    std::env::vars_os().map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy())).collect()
}

fn user_shell() -> String {
    let from_vte: Option<String> =
        unsafe { from_glib_full::<_, Option<glib::GString>>(vte::ffi::vte_get_user_shell()) }.map(String::from);
    from_vte.or_else(|| std::env::var("SHELL").ok()).filter(|s| !s.is_empty()).unwrap_or_else(|| "/bin/sh".into())
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
