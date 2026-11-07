// SPDX-License-Identifier: GPL-3.0-or-later
//! A terminal pane: VTE widget, shell process and the "Process exited" bar.
//!
//! State (title, directory) comes from the VTE termprops (`xterm.title`, `vte.cwd`),
//! not from the signals deprecated since VTE 0.78.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk::{gio, glib, pango};

use parkett_core::context::{self, Context, Kind};
use parkett_core::link::{Action as LinkAction, CustomLink};
use parkett_core::palette::Rgb;
use parkett_core::profile::{Bell, ExitAction, Profile};

use crate::backend::{self, Backend, Look, ShellEvent, SpawnSpec};
use crate::i18n::{gettext, ntrf, trf, N_};
use crate::profiles::ProfileManager;

/// Smallest sensible size of a pane in cells (see docs/ANALYSIS.md, feature matrix).
pub const MIN_COLUMNS: i64 = 20;
pub const MIN_ROWS: i64 = 5;

type Callback = Box<dyn Fn()>;
type InputCallback = Box<dyn Fn(&str)>;

struct Inner {
    root: gtk::Box,
    scroller: gtk::ScrolledWindow,
    vte: Backend,
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
    /// Link under the pointer at right-click (for *Open link*/*Copy link* in the context menu)
    menu_link: RefCell<Option<String>>,
    /// custom link of the profile under the pointer at right-click
    menu_custom: RefCell<Option<LinkAction>>,
    /// custom links of the profile with their VTE tag
    custom_links: RefCell<Vec<(i32, CustomLink)>>,
    actions: gio::SimpleActionGroup,
    shortcut_controller: RefCell<Option<gtk::ShortcutController>>,
    /// Text typed into the shell after start (startup command of a template)
    pending_input: RefCell<Option<String>>,
    /// last output in the terminal (startup command only once it is quiet, R-09)
    last_output: Cell<Instant>,
    /// root, SSH, container or other user in the foreground (context color)
    context: RefCell<Context>,
    /// Marking of the context for line, frame and badge
    mark: RefCell<Option<ContextMark>>,
    /// Dimming of the pane when inactive (`Face::dim_alpha`)
    dim_alpha: Cell<f64>,
    child_pid: Cell<Option<glib::Pid>>,
    exit: Cell<Option<ExitStatus>>,
    on_state_changed: RefCell<Vec<Callback>>,
    /// User input (keys, paste), for the shared input in the tab
    on_input: RefCell<Vec<InputCallback>>,
    /// excluded from the tab's shared input
    sync_excluded: Cell<bool>,
    /// Profile whose appearance currently applies (automatic switch by host/folder)
    appearance: RefCell<String>,
    /// Monitor like Tilix: report new output or silence when the pane is not visible
    monitor_activity: Cell<bool>,
    monitor_silence: Cell<bool>,
    /// Activity already reported; armed again as soon as the pane is seen
    activity_reported: Cell<bool>,
    silence_timer: RefCell<Option<glib::SourceId>>,
    on_close: RefCell<Vec<Callback>>,
    /// Bell or command end since the tab was last viewed
    attention: Cell<bool>,
    /// sequential number to find the terminal again from a notification
    uid: u64,
    /// Start of the running command (termprop vte.shell.preexec from vte.sh)
    command_started: Cell<Option<Instant>>,
    /// Name of the running command, remembered every second (for the notification)
    command_name: RefCell<Option<String>>,
    /// Program in the short tab label, last reported (QA-17)
    label_program: RefCell<Option<String>>,
    /// next poll of the foreground process (`watch_context`)
    poll_source: RefCell<Option<glib::SourceId>>,
    /// early poll after output or a shell mark (`poll_soon`)
    quick_poll: RefCell<Option<glib::SourceId>>,
    /// last poll, to throttle `poll_soon`
    last_poll: Cell<Instant>,
    /// Background opacity from the profile
    opacity: Cell<f64>,
    /// Ended by Parkett (closing): no restart, no bar (QA-03)
    terminated: Cell<bool>,
}

/// From this run time on, a command reports its end when its pane is not visible.
const NOTIFY_AFTER: std::time::Duration = std::time::Duration::from_secs(10);
/// This long without output counts as silence (Tilix: 10 s)
const SILENCE_AFTER: std::time::Duration = std::time::Duration::from_secs(10);
/// Early context poll: wait time until ssh/sudo has been started via exec, and
/// minimum gap between two polls while output is running
const QUICK_DELAY: Duration = Duration::from_millis(40);
const QUICK_GAP: Duration = Duration::from_millis(200);

static NEXT_UID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

#[derive(Clone)]
pub struct Terminal(Rc<Inner>);

/// How a pane marks its context (approach C, `mockups/context-colors.html`).
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMark {
    pub kind: Kind,
    /// strong color for line, frame and badge
    pub color: Rgb,
    /// text of the badge (`root`, `ssh bender`, …)
    pub label: String,
}

/// Foreground of the PTY (`read_foreground`).
struct Foreground {
    pgid: i32,
    /// the shell itself is waiting at the prompt
    is_shell: bool,
    comm: String,
    args: Vec<String>,
    euid: Option<u32>,
}

/// Weak reference for closures on the pane's widgets (draw functions): a strong one
/// would hold the terminal as long as the widget lives (L2).
#[derive(Clone)]
pub struct WeakTerminal(std::rc::Weak<Inner>);

impl WeakTerminal {
    pub fn upgrade(&self) -> Option<Terminal> {
        self.0.upgrade().map(Terminal)
    }
}

/// What a pane starts differently from the profile (command line).
#[derive(Clone, Default)]
pub struct Launch {
    /// run instead of the shell or profile command (`parkett -- …`)
    pub command: Option<Vec<String>>,
    /// Environment of the caller when a running instance receives the call (QA-08);
    /// `None` = Parkett's environment
    pub environment: Option<Vec<String>>,
}

impl Terminal {
    pub fn new(cwd: Option<String>, profile_id: &str, launch: Launch) -> Self {
        let vte = Backend::new(crate::settings::scrollback_lines());

        let exit_label = gtk::Label::builder().xalign(0.0).hexpand(true).ellipsize(pango::EllipsizeMode::End).build();
        let restart = gtk::Button::with_label(&gettext("Restart"));
        let close = gtk::Button::with_label(&gettext("Close"));
        let bar = gtk::Box::builder().spacing(10).css_classes(["parkett-exit-bar"]).build();
        let exit_icon = gtk::Image::from_icon_name("dialog-warning-symbolic");
        bar.append(&exit_icon);
        bar.append(&exit_label);
        bar.append(&restart);
        bar.append(&close);
        // When hidden, also make it invisible: a collapsed GtkRevealer otherwise keeps
        // requesting the bar's width, and panes could not get narrower.
        let exit_bar = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .child(&bar)
            .visible(false)
            .build();

        // Scrollbar like Ptyxis: VTE in a ScrolledWindow, overlay and narrow
        let scroller = gtk::ScrolledWindow::builder().hscrollbar_policy(gtk::PolicyType::Never).vexpand(true).build();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        scroller.set_child(Some(&*vte));
        root.append(&scroller);
        root.append(&exit_bar);

        let t = Terminal(Rc::new(Inner {
            root,
            scroller,
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
            menu_custom: RefCell::new(None),
            custom_links: RefCell::default(),
            actions: gio::SimpleActionGroup::new(),
            shortcut_controller: RefCell::new(None),
            pending_input: RefCell::new(None),
            last_output: Cell::new(Instant::now()),
            context: RefCell::new(Context::Local),
            mark: RefCell::new(None),
            dim_alpha: Cell::new(0.25),
            child_pid: Cell::new(None),
            exit: Cell::new(None),
            on_state_changed: RefCell::default(),
            on_input: RefCell::default(),
            sync_excluded: Cell::new(false),
            appearance: RefCell::default(),
            monitor_activity: Cell::new(false),
            monitor_silence: Cell::new(false),
            activity_reported: Cell::new(false),
            silence_timer: RefCell::default(),
            on_close: RefCell::default(),
            attention: Cell::new(false),
            uid: NEXT_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            command_started: Cell::new(None),
            command_name: RefCell::new(None),
            label_program: RefCell::new(None),
            poll_source: RefCell::new(None),
            quick_poll: RefCell::new(None),
            last_poll: Cell::new(Instant::now()),
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

    /// The widget that takes the keyboard focus (for comparisons and `GtkRoot::focus`).
    pub fn focus_widget(&self) -> &gtk::Widget {
        &self.0.vte
    }

    /// Is `w` inside the terminal's focusable widget (or that widget itself)?
    pub fn owns_focus_widget(w: &gtk::Widget) -> bool {
        Backend::is_terminal_widget(w)
    }

    pub fn grab_focus(&self) {
        self.0.vte.grab_focus();
    }

    /// The terminal got the keyboard focus.
    pub fn connect_focus_gained(&self, f: impl Fn() + 'static) {
        self.0.vte.connect_has_focus_notify(move |v| {
            if v.has_focus() {
                f();
            }
        });
    }

    pub fn connect_cursor_moved(&self, f: impl Fn() + 'static) {
        self.0.vte.connect_cursor_moved(f);
    }

    /// Size of one character cell in pixels.
    pub fn char_size(&self) -> (i64, i64) {
        self.0.vte.char_size()
    }

    pub fn connect_char_size_changed(&self, f: impl Fn() + 'static) {
        self.0.vte.connect_char_size_changed(f);
    }

    /// Is the cursor in the top visible row (where the floating title would cover it)?
    pub fn cursor_on_top_row(&self) -> bool {
        self.0.vte.cursor_on_top_row()
    }

    /// The selected text, if any.
    pub fn selected_text(&self) -> Option<String> {
        self.0.vte.selected_text()
    }

    /// Run a `term.*` action of this terminal.
    pub fn activate_action(&self, action: &str) {
        let _ = self.0.vte.activate_action(action, None);
    }

    pub fn connect_state_changed(&self, f: impl Fn() + 'static) {
        self.0.on_state_changed.borrow_mut().push(Box::new(f));
    }

    /// Text the user types or pastes into this terminal (VTE `commit`).
    pub fn connect_input(&self, f: impl Fn(&str) + 'static) {
        self.0.on_input.borrow_mut().push(Box::new(f));
    }

    /// Send text to the process as if the user had typed it here.
    pub fn feed(&self, text: &str) {
        if self.0.exit.get().is_none() {
            self.0.vte.feed(text.as_bytes());
        }
    }

    /// Read only (input blocked)?
    pub fn read_only(&self) -> bool {
        !self.0.vte.is_input_enabled()
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

    /// Switches the profile for colors and future restarts (running process stays).
    pub fn set_profile(&self, id: &str) {
        *self.0.profile_id.borrow_mut() = id.to_string();
        self.apply_colors();
        self.emit_state_changed();
    }

    /// Background color of the active variant (for dimming in the pane).
    /// Background not fully opaque (profile *Background opacity* below 100 %).
    pub fn is_translucent(&self) -> bool {
        self.0.opacity.get() < 1.0
    }

    pub fn background(&self) -> Rgb {
        self.0.background.get()
    }

    pub fn connect_close(&self, f: impl Fn() + 'static) {
        self.0.on_close.borrow_mut().push(Box::new(f));
    }

    /// Window title the shell set via OSC 0/2; otherwise the profile's command.
    pub fn title(&self) -> Option<String> {
        self.0.vte.title().filter(|s| !s.is_empty()).or_else(|| {
            // Command from the command line, otherwise the profile's
            let command = match &self.0.command {
                Some(c) => c.join(" "),
                None => ProfileManager::get().profile(&self.profile_id()).command,
            };
            Some(command.trim().to_string()).filter(|c| !c.is_empty())
        })
    }

    /// Working directory for the session: `vte.cwd`, otherwise the process's (/proc), otherwise
    /// the one at start – panes without shell integration report no `vte.cwd`.
    pub fn working_directory(&self) -> Option<String> {
        self.cwd()
            .or_else(|| {
                if parkett_core::pty::in_flatpak() {
                    return None;
                }
                let pid = self.0.child_pid.get()?.0;
                std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?.to_str().map(String::from)
            })
            .or_else(|| self.0.cwd_at_start.clone())
    }

    /// Working directory from `vte.cwd` (OSC 7, requires vte.sh in the shell).
    /// Only paths on this host, decoded: an OSC 7 from an SSH session names the path
    /// on the server and must not set a local start directory (QA-05, QA-06).
    pub fn cwd(&self) -> Option<String> {
        let uri = self.0.vte.cwd_uri();
        uri.and_then(|u| parkett_core::uri::local_path(u.to_str().as_str(), glib::host_name().as_str()))
    }

    /// How the process ended; `None` while it is running.
    pub fn exit_status(&self) -> Option<ExitStatus> {
        self.0.exit.get()
    }

    /// Ends the shell (SIGHUP as when closing a terminal window; the shell passes
    /// it on to its jobs). VTE does that too when the widget is freed – done here
    /// explicitly so no process stays alive through an overlooked reference.
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

    /// Is the terminal visible right now (tab selected, window active)?
    fn is_seen(&self) -> bool {
        let vte = &self.0.vte;
        vte.is_mapped() && vte.root().and_downcast::<gtk::Window>().is_some_and(|w| w.is_active())
    }

    /// A command finished: if it ran long and the pane is not visible, mark the tab
    /// and send a notification (clicking it shows the pane).
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
        let n = gio::Notification::new(&gettext("Command finished"));
        let what =
            self.0.command_name.borrow_mut().take().or_else(|| self.title()).unwrap_or_else(|| gettext("Command"));
        n.set_body(Some(&trf("{command} · after {duration}", &[("command", &what), ("duration", &duration)])));
        n.set_default_action_and_target_value("app.show-terminal", Some(&self.0.uid.to_variant()));
        app.send_notification(Some(&format!("command-{}", self.0.uid)), &n);
    }

    /// New output: report activity, restart the silence timer (one per pane).
    fn watch_output(&self) {
        let inner = &self.0;
        if inner.monitor_activity.get() {
            if self.is_seen() {
                inner.activity_reported.set(false);
            } else if !inner.activity_reported.replace(true) {
                self.notify_monitor("activity", &gettext("New output"));
            }
        }
        if inner.monitor_silence.get() {
            if let Some(old) = inner.silence_timer.borrow_mut().take() {
                old.remove();
            }
            let weak = Rc::downgrade(&self.0);
            let id = glib::timeout_add_local_once(SILENCE_AFTER, move || {
                let Some(inner) = weak.upgrade() else { return };
                inner.silence_timer.borrow_mut().take();
                let t = Terminal(inner);
                if t.0.monitor_silence.get() && !t.is_seen() && t.0.exit.get().is_none() {
                    t.notify_monitor(
                        "silence",
                        &trf("No output for {seconds} s", &[("seconds", &SILENCE_AFTER.as_secs().to_string())]),
                    );
                }
            });
            *inner.silence_timer.borrow_mut() = Some(id);
        }
    }

    /// Mark the tab and send a notification that shows the pane on click.
    fn notify_monitor(&self, kind: &str, title: &str) {
        self.0.attention.set(true);
        self.emit_state_changed();
        let Some(app) = gio::Application::default() else { return };
        let n = gio::Notification::new(title);
        n.set_body(Some(&self.title().unwrap_or_else(|| "Shell".into())));
        n.set_default_action_and_target_value("app.show-terminal", Some(&self.0.uid.to_variant()));
        app.send_notification(Some(&format!("{kind}-{}", self.0.uid)), &n);
    }

    /// Returns and clears the mark (bell, command end).
    pub fn take_attention(&self) -> bool {
        self.0.attention.replace(false)
    }

    pub fn context(&self) -> Context {
        self.0.context.borrow().clone()
    }

    /// Marking of the current context; `None` = local or turned off.
    pub fn context_mark(&self) -> Option<ContextMark> {
        self.0.mark.borrow().clone()
    }

    /// Opacity of the dimming when the pane is inactive: text stays at 4.5:1.
    pub fn dim_alpha(&self) -> f64 {
        self.0.dim_alpha.get()
    }

    /// Polls the foreground process (like Ptyxis, but per pane): every second while
    /// the pane is visible and its window active, otherwise every 5 s – background tabs
    /// and unfocused windows wake the CPU less often (L3). When the pane comes into view or
    /// gets focus, immediately. 5 s is enough for the notification too: it only reports
    /// commands running 10 s or longer, and the poll knows their names by then.
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

    /// Poll now, then schedule the next one (a scheduled one is replaced).
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
                // already expired: just forget it, do not remove it
                inner.poll_source.borrow_mut().take();
                Terminal(inner).poll_now();
            }
        });
        *self.0.poll_source.borrow_mut() = Some(id);
    }

    /// Bring the poll forward: after `ssh host` or `sudo -i` the one-second tick colored
    /// up to 1 s late. Output (echo of the input, password prompt, banner) and the
    /// shell marks trigger it; at most every `QUICK_GAP`, and only in the visible pane,
    /// so constant output in the background does not wake the CPU (L3).
    fn poll_soon(&self) {
        if self.0.quick_poll.borrow().is_some() || !self.is_seen() {
            return;
        }
        let delay = QUICK_DELAY.max(QUICK_GAP.saturating_sub(self.0.last_poll.get().elapsed()));
        let weak = Rc::downgrade(&self.0);
        let id = glib::timeout_add_local_once(delay, move || {
            if let Some(inner) = weak.upgrade() {
                inner.quick_poll.borrow_mut().take();
                Terminal(inner).poll_now();
            }
        });
        *self.0.quick_poll.borrow_mut() = Some(id);
    }

    fn poll(&self) {
        self.0.last_poll.set(Instant::now());
        // Read the foreground once, for notification, context and label (D2)
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
        } else {
            // Folder rules: nobody reports a cd without shell integration, so check here
            self.check_appearance();
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

    /// Type a command into the shell and submit it with Enter (proposal 5: the shell stays
    /// usable afterwards). If the shell is not running yet, it is typed after start.
    pub fn type_command(&self, command: &str) {
        let text = format!("{}\n", command.trim_end());
        if self.0.child_pid.get().is_some() && self.0.exit.get().is_none() {
            self.0.vte.feed(text.as_bytes());
        } else {
            *self.0.pending_input.borrow_mut() = Some(text);
        }
    }

    /// Foreground of the PTY, read once for context, label, notification and
    /// template (D2). Pattern like Ptyxis (`agent/ptyxis-process-impl.c`): tcgetpgrp on the
    /// PTY, details from /proc. Only meaningful without the Flatpak sandbox.
    fn read_foreground(&self) -> Option<Foreground> {
        // In a Flatpak sandbox /proc shows the sandbox, not the host the shell runs on
        if self.0.exit.get().is_some() || parkett_core::pty::in_flatpak() {
            return None;
        }
        let pgid = self.0.vte.foreground_pgid()?;
        let comm = std::fs::read_to_string(format!("/proc/{pgid}/comm")).unwrap_or_default().trim().to_string();
        let args: Vec<String> = std::fs::read(format!("/proc/{pgid}/cmdline"))
            .unwrap_or_default()
            .split(|b| *b == 0)
            .filter(|a| !a.is_empty())
            .map(|a| String::from_utf8_lossy(a).into_owned())
            .collect();
        // `Uid:\treal\teffective\t…`
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
            Some(f) => {
                let own_uid = unsafe { libc::getuid() };
                let title = self.0.vte.title();
                context::classify(&context::Process {
                    comm: &f.comm,
                    args: &f.args,
                    euid: f.euid,
                    own_uid,
                    euid_name: f.euid.filter(|&u| u != 0 && u != own_uid).and_then(user_name),
                    title: title.as_deref(),
                })
            }
            None => Context::Local,
        }
    }

    /// Command line of the foreground process (for *Save tab as template*), otherwise `None`.
    pub fn foreground_command(&self) -> Option<String> {
        let fg = self.read_foreground().filter(|f| !f.is_shell)?;
        let args: Vec<String> = fg
            .args
            .into_iter()
            .map(|a| {
                // quote only where the shell needs it – `npm run dev` instead of `'npm' 'run' 'dev'`
                if !a.is_empty() && a.chars().all(|c| c.is_alphanumeric() || "-_./:=@%+,".contains(c)) {
                    a
                } else {
                    glib::shell_quote(&a).to_string_lossy().into_owned()
                }
            })
            .collect();
        Some(args.join(" ")).filter(|s| !s.is_empty())
    }

    /// Program for the short tab label: with SSH `ssh host`, otherwise the program in the
    /// foreground, at the prompt the shell itself.
    pub fn label_program(&self) -> Option<String> {
        self.label_program_with(self.foreground_process())
    }

    fn label_program_with(&self, foreground: Option<String>) -> Option<String> {
        if self.0.exit.get().is_some() {
            return None;
        }
        if let Some(host) = self.context().remote_host().map(String::from) {
            let name = foreground.unwrap_or_else(|| "ssh".into());
            return Some(if name == host { name } else { format!("{name} {host}") });
        }
        foreground.or_else(|| {
            let pid = self.0.child_pid.get()?.0;
            let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
            Some(comm.trim().to_string()).filter(|c| !c.is_empty())
        })
    }

    /// Name of the foreground process if it is not the shell itself (e.g. `vim`, `ssh`).
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

    /// `focus`: move focus into the terminal afterwards (not on automatic restart in the background).
    fn spawn(&self, focus: bool) {
        self.0.terminated.set(false);
        self.0.exit_bar.set_reveal_child(false);
        self.0.exit_bar.set_visible(false);
        self.0.exit.set(None);
        self.0.child_pid.set(None);
        self.0.started_at.set(Some(Instant::now()));
        let profile = ProfileManager::get().profile(&self.profile_id());

        // Command from the profile (split like in a shell), otherwise the user's login shell
        let argv: Vec<String> = if let Some(cmd) = &self.0.command {
            cmd.clone()
        } else if profile.command.trim().is_empty() {
            let mut a = vec![user_shell(&self.0.vte)];
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
        // Call to a running instance: its environment instead of Parkett's (QA-08)
        let (env, clean_env) = match &self.0.environment {
            Some(caller) => {
                (parkett_core::environment::for_caller(caller, &own_environment(), &profile.valid_environment()), true)
            }
            None => (profile.valid_environment(), false),
        };
        let argv_ref: Vec<&str> = argv.iter().map(String::as_str).collect();
        let env_ref: Vec<&str> = env.iter().map(String::as_str).collect();
        let weak = Rc::downgrade(&self.0);
        let program = argv[0].clone();
        self.0.vte.spawn(
            &SpawnSpec { argv: &argv_ref, dir: dir.as_deref(), env: &env_ref, clean_env },
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

    /// Type the startup command once the shell is ready (R-09): with shell integration at the
    /// first prompt (see `vte.shell.precmd`), otherwise when output is quiet and the PTY is not
    /// waiting for a password – typed immediately it ended up in ssh's password prompt.
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
            self.0.vte.feed(text.as_bytes());
        }
    }

    /// termios of the PTY: line-wise input and echo (detects a password prompt).
    fn tty_mode(&self) -> Option<parkett_core::startup::TtyMode> {
        self.0.vte.tty_mode()
    }

    fn fail(&self, program: &str, message: &str) {
        self.0.exit.set(Some(ExitStatus::Code(-1)));
        self.show_exit(
            &trf(
                "<b>{program}</b> could not be started: {error}",
                &[("program", &glib::markup_escape_text(program)), ("error", &glib::markup_escape_text(message))],
            ),
            true,
        );
        self.emit_state_changed();
    }

    /// `error`: warning icon and red tint; a normal exit (exit code 0) gets a
    /// neutral bar, otherwise `exit` looks like an error (QA-11).
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
        self.0.vte.connect_child_exited(move |status| {
            let Some(inner) = weak.upgrade() else { return };
            let t = Terminal(inner);
            let exit = decode_wait_status(status);
            t.0.exit.set(Some(exit));
            // ended on purpose: do not restart, otherwise a new process would run invisibly
            // in the closed tab (QA-03, e.g. an ssh connection)
            if t.0.terminated.get() {
                return;
            }
            let action = ProfileManager::get().profile(&t.profile_id()).exit_action;
            let quick = t.0.started_at.get().is_some_and(|s| s.elapsed() < Duration::from_secs(2));
            match action {
                ExitAction::Close if exit == ExitStatus::Code(0) => {
                    // Fill the bar right away: if *Undo* brings the pane back, it is otherwise
                    // a dead terminal without a hint and without *Restart* (R-05)
                    t.show_exit(&gettext("<b>Process exited</b>"), false);
                    // close after the signal handler, not in the middle of the VTE callback
                    let weak = Rc::downgrade(&t.0);
                    glib::idle_add_local_once(move || {
                        if let Some(inner) = weak.upgrade() {
                            Terminal(inner).emit_close();
                        }
                    });
                    t.emit_state_changed();
                    return;
                }
                // Restart, unless the process ends again immediately (otherwise an endless loop)
                ExitAction::Restart if !quick => {
                    t.spawn(false);
                    return;
                }
                _ => {}
            }
            let text = match exit {
                ExitStatus::Code(0) => gettext("<b>Process exited</b>"),
                ExitStatus::Code(c) => trf("<b>Process exited</b> · exit code {code}", &[("code", &c.to_string())]),
                ExitStatus::Signal(s) => trf("<b>Process exited</b> · signal {signal}", &[("signal", &s.to_string())]),
            };
            let text = if action == ExitAction::Restart {
                trf("{text} · no automatic restart, exited immediately", &[("text", &text)])
            } else {
                text
            };
            // a restart profile whose process ends immediately is a problem, even with code 0
            t.show_exit(&text, exit != ExitStatus::Code(0) || action == ExitAction::Restart);
            t.emit_state_changed();
        });

        // Bell: mark the tab (VTE makes the sound itself, see set_audible_bell)
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_bell(move || {
            let Some(inner) = weak.upgrade() else { return };
            let t = Terminal(inner);
            if ProfileManager::get().profile(&t.profile_id()).bell != Bell::None {
                t.0.attention.set(true);
                t.emit_state_changed();
            }
        });

        // Pass input on for the shared input in the tab
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_commit(move |text| {
            if let Some(inner) = weak.upgrade() {
                for f in inner.on_input.borrow().iter() {
                    f(text);
                }
            }
        });

        // Copy on select (setting, like Tilix)
        let backend = self.0.vte.downgrade();
        self.0.vte.connect_selection_changed(move || {
            let Some(b) = backend.upgrade() else { return };
            if b.has_selection() && crate::settings::settings().boolean("copy-on-select") {
                b.copy(false);
            }
        });

        // Time of the last output for the startup command (R-09)
        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_contents_changed(move || {
            if let Some(inner) = weak.upgrade() {
                inner.last_output.set(Instant::now());
                let t = Terminal(inner);
                t.watch_output();
                t.poll_soon();
            }
        });

        let weak = Rc::downgrade(&self.0);
        self.0.vte.connect_shell_event(move |event| {
            let Some(inner) = weak.upgrade() else { return };
            match event {
                ShellEvent::Title => {
                    let t = Terminal(inner);
                    t.emit_state_changed();
                    // `root@host` in the title of a remote shell: root on the other host
                    t.poll_soon();
                }
                ShellEvent::Cwd => {
                    let t = Terminal(inner);
                    t.check_appearance();
                    t.emit_state_changed();
                }
                ShellEvent::Preexec => {
                    inner.command_started.set(Some(Instant::now()));
                    inner.command_name.borrow_mut().take();
                    Terminal(inner).poll_soon();
                }
                ShellEvent::Precmd => {
                    // first prompt: a waiting startup command can come now (R-09)
                    Terminal(inner.clone()).feed_pending();
                    if let Some(start) = inner.command_started.take() {
                        Terminal(inner.clone()).command_finished(start.elapsed());
                    }
                    // back at the prompt: release the context color immediately
                    Terminal(inner).poll_soon();
                }
            }
        });
    }

    /// Profile for the appearance: a profile with a matching rule (host of the SSH connection,
    /// otherwise local folder), otherwise its own. Behavior (bell, process exit, restart)
    /// stays with its own profile.
    fn appearance_profile(&self) -> Profile {
        let manager = ProfileManager::get();
        let own = manager.profile(&self.profile_id());
        let host = self.0.context.borrow().remote_host().map(String::from);
        // like the tab name: OSC 7, otherwise the shell's folder (/proc) – without vte.sh there
        // is no OSC 7, and folder rules would never apply
        let cwd = if host.is_none() { self.working_directory() } else { None };
        let home = glib::home_dir().to_string_lossy().into_owned();
        parkett_core::profile::automatic(&manager.profiles(), host.as_deref(), cwd.as_deref(), &home)
            .cloned()
            .unwrap_or(own)
    }

    /// After a folder change: recolor if a folder rule now applies differently.
    fn check_appearance(&self) {
        if self.appearance_profile().id != *self.0.appearance.borrow() {
            self.apply_colors();
        }
    }

    /// Profile for the appearance if it is not its own (automatic switch).
    pub fn automatic_profile(&self) -> Option<String> {
        let id = self.0.appearance.borrow().clone();
        (!id.is_empty() && id != self.profile_id()).then(|| ProfileManager::get().profile(&id).name)
    }

    /// Colors from the profile's palette (variant by system style), font, cursor, bell.
    fn apply_colors(&self) {
        let sm = adw::StyleManager::default();
        let manager = ProfileManager::get();
        let look = self.appearance_profile();
        *self.0.appearance.borrow_mut() = look.id.clone();
        let chosen = look.palette.clone();
        let high_contrast = sm.is_high_contrast() && crate::settings::follow_high_contrast();
        let palette = manager.palette(parkett_core::palette::effective_id(&chosen, high_contrast));
        let face = palette.face(sm.is_dark());
        let colors: Vec<gtk::gdk::RGBA> = face.colors.iter().map(rgba).collect();
        // Context color: fixed meaning, shade from the scheme; only root and SSH tint the
        // background, with contrast protection (`Face::context_background`)
        let mark = if crate::settings::context_colors() {
            let context = self.0.context.borrow();
            context.kind().map(|kind| ContextMark {
                kind,
                color: face.context_color(kind),
                label: context.label().unwrap_or_default(),
            })
        } else {
            None
        };
        let background = mark.as_ref().map_or(face.background, |m| face.context_background(m.kind));
        self.0.dim_alpha.set(face.dim_alpha(&background));
        let marked = mark.is_some();
        *self.0.mark.borrow_mut() = mark;
        // Transparency: only the background gets alpha; the window behind it is switched
        // to translucent for that (Window::update_translucency)
        // a context draws opaque: the marking must not depend on what is behind the window,
        // and text behind it would shine through (setting, on by default)
        let opacity = if marked && crate::settings::context_opaque() { 1.0 } else { look.opacity() };
        let mut bg = rgba(&background);
        bg.set_alpha(opacity as f32);
        self.0.opacity.set(opacity);
        // Font and cursor from the appearance profile, bell from its own
        let font = if look.font.trim().is_empty() { sm.monospace_font_name() } else { look.font.trim().into() };
        let own = manager.profile(&self.profile_id());
        self.apply_links(&own.links);
        self.0.vte.apply(&Look {
            foreground: rgba(&face.foreground),
            background: bg,
            palette: &colors,
            cursor: face.cursor.as_ref().map(rgba),
            cursor_foreground: face.cursor_foreground.as_ref().map(rgba),
            // Selection in the accent color instead of inverted
            highlight: rgba(&face.selection(&from_rgba(&sm.accent_color_rgba()))),
            font: font.to_string(),
            bold_is_bright: look.bold_is_bright,
            line_spacing: Profile::spacing(look.line_spacing),
            letter_spacing: Profile::spacing(look.letter_spacing),
            scroll_on_output: own.scroll_on_output,
            scroll_on_keystroke: own.scroll_on_keystroke,
            cursor_shape: look.cursor_shape,
            cursor_blink: look.cursor_blink,
            audible_bell: own.bell == Bell::Sound,
            sixel: backend::sixel_supported() && crate::settings::settings().boolean("enable-sixel"),
        });
        // without a bar the mouse wheel keeps working (External = area scrolls, no bar)
        self.0.scroller.set_vscrollbar_policy(if look.scrollbar {
            gtk::PolicyType::Automatic
        } else {
            gtk::PolicyType::External
        });
        self.0.background.set(background);
        for f in self.0.on_colors_changed.borrow().iter() {
            f();
        }
        // Window checks translucency, tabs their display
        self.emit_state_changed();
    }

    /// Drag files from the file manager into the terminal: paste quoted paths (like Tilix,
    /// Ptyxis), via paste – with the shell's bracketed paste this executes nothing.
    fn setup_file_drop(&self) {
        let target = gtk::DropTarget::new(gtk::gdk::FileList::static_type(), gtk::gdk::DragAction::COPY);
        let vte = self.0.vte.downgrade();
        target.connect_drop(move |_, value, _, _| {
            let (Some(vte), Ok(files)) = (vte.upgrade(), value.get::<gtk::gdk::FileList>()) else { return false };
            let paths: Vec<String> = files
                .files()
                .iter()
                .filter_map(|f| f.path())
                .map(|p| glib::shell_quote(&p).to_string_lossy().into_owned())
                .collect();
            if paths.is_empty() {
                return false;
            }
            vte.paste(&format!("{} ", paths.join(" ")));
            vte.grab_focus();
            true
        });
        self.0.vte.add_controller(target);
    }

    fn setup_links(&self) {
        let vte = &self.0.vte;
        for pattern in URL_PATTERNS {
            if let Err(e) = vte.add_match(pattern) {
                glib::g_warning!("parkett", "URL pattern: {e}");
            }
        }
        // Ctrl+click opens the link; without Ctrl, clicking stays selecting
        let click = gtk::GestureClick::builder().button(1).propagation_phase(gtk::PropagationPhase::Capture).build();
        let weak = Rc::downgrade(&self.0);
        click.connect_pressed(move |g, _, x, y| {
            let Some(inner) = weak.upgrade() else { return };
            if !g.current_event_state().contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                return;
            }
            let t = Terminal(inner);
            if let Some(action) = t.custom_link_at(x, y) {
                g.set_state(gtk::EventSequenceState::Claimed);
                t.run_link(action);
            } else if let Some(uri) = t.link_at(x, y) {
                g.set_state(gtk::EventSequenceState::Claimed);
                t.open_link(&uri);
            }
        });
        vte.add_controller(click);
        // Right-click remembers the link for the context menu
        let right = gtk::GestureClick::builder().button(3).propagation_phase(gtk::PropagationPhase::Capture).build();
        let weak = Rc::downgrade(&self.0);
        right.connect_pressed(move |_, _, x, y| {
            if let Some(inner) = weak.upgrade() {
                let t = Terminal(inner);
                let custom = t.custom_link_at(x, y);
                let link = if custom.is_some() { None } else { t.link_at(x, y) };
                for name in ["open-link", "copy-link"] {
                    if let Some(a) = t.0.actions.lookup_action(name).and_downcast::<gio::SimpleAction>() {
                        a.set_enabled(link.is_some() || custom.is_some());
                    }
                }
                *t.0.menu_link.borrow_mut() = link;
                *t.0.menu_custom.borrow_mut() = custom;
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
                let custom = t.0.menu_custom.borrow().clone();
                if let Some(action) = custom {
                    match (open, action) {
                        (true, action) => t.run_link(action),
                        (false, LinkAction::Open(text) | LinkAction::Run(text)) => t.0.vte.clipboard().set_text(&text),
                    }
                    return;
                }
                let Some(uri) = t.0.menu_link.borrow().clone() else { return };
                if open {
                    t.open_link(&uri);
                } else {
                    t.0.vte.clipboard().set_text(&uri);
                }
            });
            self.0.actions.add_action(&a);
        }
        // OSC 8 links show their real target as a tooltip – the visible text can deceive.
        vte.show_hyperlink_tooltips();
    }

    /// Register the profile's custom links again when they have changed.
    fn apply_links(&self, links: &[CustomLink]) {
        let wanted: Vec<&CustomLink> = links.iter().filter(|l| l.is_usable()).collect();
        let mut current = self.0.custom_links.borrow_mut();
        if current.len() == wanted.len() && current.iter().zip(&wanted).all(|((_, a), b)| a == *b) {
            return;
        }
        for (tag, _) in current.drain(..) {
            self.0.vte.remove_match(tag);
        }
        for link in wanted {
            match self.0.vte.add_match(&link.pattern) {
                Ok(tag) => current.push((tag, link.clone())),
                Err(e) => glib::g_warning!("parkett", "Custom link “{}”: {e}", link.pattern),
            }
        }
    }

    /// Custom link of the profile at the position, with groups substituted.
    fn custom_link_at(&self, x: f64, y: f64) -> Option<LinkAction> {
        let (text, tag) = self.0.vte.match_at(x, y);
        let text = text?;
        let links = self.0.custom_links.borrow();
        let link = links.iter().find(|(t, _)| *t == tag).map(|(_, l)| l)?;
        // VTE only returns the whole match; the same expression fetches the groups again
        let re = glib::Regex::new(&link.pattern, glib::RegexCompileFlags::empty(), glib::RegexMatchFlags::empty())
            .ok()
            .flatten()?;
        let text = glib::GString::from(text);
        let info = re.match_(text.as_gstr(), glib::RegexMatchFlags::empty()).ok().filter(|i| i.matches())?;
        let groups: Vec<Option<glib::GString>> = (0..10).map(|i| info.fetch(i)).collect();
        let groups: Vec<Option<&str>> = groups.iter().map(|g| g.as_deref()).collect();
        Some(parkett_core::link::expand(&link.target, &groups))
    }

    /// Target of a custom link: open the address or run the command in the pane's folder.
    fn run_link(&self, action: LinkAction) {
        let line = match action {
            LinkAction::Open(uri) => return self.open_link(&uri),
            LinkAction::Run(line) => line,
        };
        let result = glib::shell_parse_argv(&line).map_err(|e| e.to_string()).and_then(|argv| {
            let launcher = gio::SubprocessLauncher::new(gio::SubprocessFlags::NONE);
            if let Some(dir) = self.working_directory() {
                launcher.set_cwd(&dir);
            }
            let argv: Vec<&std::ffi::OsStr> = argv.iter().map(|a| a.as_os_str()).collect();
            launcher.spawn(&argv).map(|_| ()).map_err(|e| e.to_string())
        });
        if let Err(e) = result {
            glib::g_warning!("parkett", "Custom link command failed: {e}");
            if let Some(w) = self.0.vte.root().and_downcast::<crate::window::Window>() {
                w.toast(&trf("Command failed: {error}", &[("error", &e)]));
            }
        }
    }

    /// OSC 8 link or detected address at the position (widget coordinates).
    pub fn link_at(&self, x: f64, y: f64) -> Option<String> {
        self.0.vte.hyperlink_at(x, y).or_else(|| self.0.vte.match_at(x, y).0).filter(|u| !u.is_empty())
    }

    /// File path from the output in the file manager: open the folder, for files the folder with
    /// the file selected (audit T-53). In an SSH session the path is on the other host.
    fn open_path(&self, text: &str) {
        let window = self.0.vte.root().and_downcast::<gtk::Window>();
        let toast = |msg: &str| {
            if let Some(w) = window.as_ref().and_then(|w| w.clone().downcast::<crate::window::Window>().ok()) {
                w.toast(msg);
            }
        };
        if self.detect_context().remote_host().is_some() {
            toast(&gettext("The path is on the other computer"));
            return;
        }
        let cwd = self.working_directory();
        let Some(path) = parkett_core::paths::output_path(text, cwd.as_deref(), &glib::home_dir()) else {
            return;
        };
        if !path.exists() {
            toast(&trf("“{path}” does not exist", &[("path", &path.display().to_string())]));
            return;
        }
        let launcher = gtk::FileLauncher::new(Some(&gio::File::for_path(&path)));
        let report = move |res: Result<(), glib::Error>| {
            if let Err(e) = res {
                glib::g_warning!("parkett", "Path not opened: {e}");
            }
        };
        if path.is_dir() {
            launcher.launch(window.as_ref(), gio::Cancellable::NONE, report);
        } else {
            launcher.open_containing_folder(window.as_ref(), gio::Cancellable::NONE, report);
        }
    }

    /// Opens a link with the default application, but only with an allowed scheme; a
    /// detected file path (without scheme) goes to the file manager.
    pub fn open_link(&self, uri: &str) {
        if uri.starts_with('/') || uri.starts_with('~') || uri.starts_with('.') {
            self.open_path(uri);
            return;
        }
        let scheme = uri.split(':').next().unwrap_or("").to_ascii_lowercase();
        if !SAFE_SCHEMES.contains(&scheme.as_str()) {
            glib::g_warning!("parkett", "Link with scheme “{scheme}” not opened");
            return;
        }
        // file: links only on this host: an OSC 8 link from an SSH session names the
        // remote host; a different file is meant there than here (like gnome-terminal).
        // Many programs omit the host (`file:///path`) – if a connection is currently running
        // in the pane, that one is not from here either (R-04)
        if scheme == "file"
            && (self.detect_context().remote_host().is_some()
                || !parkett_core::uri::is_local_file(uri, glib::host_name().as_str()))
        {
            glib::g_warning!("parkett", "file: link to another computer not opened: {uri}");
            if let Some(w) = self.0.vte.root().and_downcast::<gtk::Window>() {
                let d = adw::AlertDialog::new(
                    Some(&gettext("Link not opened")),
                    Some(&gettext("The link points to a file on another computer.")),
                );
                d.add_response("ok", &gettext("OK"));
                d.present(Some(&w));
            }
            return;
        }
        let window = self.0.vte.root().and_downcast::<gtk::Window>();
        let weak = window.as_ref().map(|w| w.downgrade());
        gtk::UriLauncher::new(uri).launch(window.as_ref(), gio::Cancellable::NONE, move |res| {
            let Err(e) = res else { return };
            glib::g_warning!("parkett", "Link not opened: {e}");
            // Otherwise nothing happens after Ctrl+click (VM 2026-10-22: default app was missing)
            if e.matches(gtk::DialogError::Dismissed) || e.matches(gtk::DialogError::Cancelled) {
                return;
            }
            if let Some(w) = weak.and_then(|w| w.upgrade()).and_downcast::<crate::window::Window>() {
                w.toast(&trf("Link not opened: {error}", &[("error", e.message())]));
            }
        });
    }

    /// Sets the search pattern; `None` clears the search. Returns an error for an invalid regex.
    pub fn set_search(&self, text: Option<&str>, query: SearchQuery) -> Result<(), String> {
        let pattern = text.filter(|t| !t.is_empty()).map(|t| query.pattern(t));
        self.0.vte.set_search(pattern.as_deref(), query.case_sensitive)
    }

    /// Matches in the whole scrollback (QA-19), at most `SEARCH_COUNT_MAX`. VTE does not count
    /// itself; counting uses GRegex, which like VTE builds on PCRE2. VTE returns wrapped lines
    /// joined, so a match across the wrap counts too.
    pub fn search_count(&self, text: &str, query: SearchQuery) -> Option<usize> {
        if text.is_empty() {
            return None;
        }
        let pattern = query.pattern(text);
        let mut flags = glib::RegexCompileFlags::MULTILINE | glib::RegexCompileFlags::OPTIMIZE;
        if !query.case_sensitive {
            flags |= glib::RegexCompileFlags::CASELESS;
        }
        let re = glib::Regex::new(&pattern, flags, glib::RegexMatchFlags::empty()).ok()??;
        self.0.vte.count_matches(&re, SEARCH_COUNT_MAX)
    }

    /// Next match upward (older output) or downward. `false` = nothing found.
    pub fn search_step(&self, up: bool) -> bool {
        self.0.vte.search_step(up)
    }

    fn setup_style(&self) {
        self.apply_colors();
        // Only weak references: StyleManager, ProfileManager and GSettings live as long
        // as the app. A strong reference would hold the terminal – it would never be freed
        // and the shell never ended (found in the VM: shell and `sleep` kept running).
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
        // Scrollback length follows the setting
        let vte = self.0.vte.downgrade();
        let scrollback = crate::settings::settings().connect_changed(Some("scrollback-lines"), move |_, _| {
            if let Some(vte) = vte.upgrade() {
                vte.set_scrollback_lines(crate::settings::scrollback_lines());
            }
        });
        // Apply context color and Sixel on/off immediately
        let weak = Rc::downgrade(&self.0);
        let ctx = crate::settings::settings().connect_changed(None, move |_, key| {
            if !matches!(
                key,
                "context-colors" | "context-opaque" | "context-headerbar" | "enable-sixel" | "follow-high-contrast"
            ) {
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
        let add = |name: &str, f: Box<dyn Fn(&Backend)>| {
            let a = gio::SimpleAction::new(name, None);
            let vte = self.0.vte.downgrade();
            a.connect_activate(move |_, _| {
                if let Some(vte) = vte.upgrade() {
                    f(&vte);
                }
            });
            group.add_action(&a);
        };
        add("copy", Box::new(|v| v.copy(false)));
        // Paste through Parkett: multi-line text is shown first (feature matrix P0) –
        // without the shell's bracketed paste the lines would otherwise run immediately
        add("paste", Box::new(paste_with_check));
        add("paste-advanced", Box::new(paste_advanced));
        add("select-all", Box::new(|v| v.select_all()));
        add("zoom-in", Box::new(|v| v.set_font_scale((v.font_scale() * 1.1).min(4.0))));
        add("zoom-out", Box::new(|v| v.set_font_scale((v.font_scale() / 1.1).max(0.3))));
        add("zoom-reset", Box::new(|v| v.set_font_scale(1.0)));
        // Reset like in Tilix: after broken control sequences (e.g. `cat` of a
        // binary file); "clear" also discards the scrollback
        add("copy-html", Box::new(|v| v.copy(true)));
        add("reset", Box::new(|v| v.reset(false)));
        add("reset-clear", Box::new(|v| v.reset(true)));
        add("save-output", Box::new(save_output));
        // Read only (Tilix "Read-Only"): keys and paste do not reach the process
        let a = gio::SimpleAction::new_stateful("read-only", None, &false.to_variant());
        let weak = Rc::downgrade(&self.0);
        a.connect_activate(move |a, _| {
            let Some(inner) = weak.upgrade() else { return };
            let on = !a.state().and_then(|v| v.get::<bool>()).unwrap_or(false);
            a.set_state(&on.to_variant());
            inner.vte.set_input_enabled(!on);
            // VTE discards pasted text then anyway; no confirmation dialog before it
            for name in ["paste", "paste-advanced"] {
                if let Some(p) = inner.actions.lookup_action(name).and_downcast::<gio::SimpleAction>() {
                    p.set_enabled(!on);
                }
            }
            Terminal(inner).emit_state_changed();
        });
        group.add_action(&a);
        // Monitor: stateful toggle for the check mark in the context menu
        for name in ["monitor-activity", "monitor-silence"] {
            let a = gio::SimpleAction::new_stateful(name, None, &false.to_variant());
            let weak = Rc::downgrade(&self.0);
            a.connect_activate(move |a, _| {
                let Some(inner) = weak.upgrade() else { return };
                let on = !a.state().and_then(|v| v.get::<bool>()).unwrap_or(false);
                a.set_state(&on.to_variant());
                if name == "monitor-activity" {
                    inner.monitor_activity.set(on);
                    inner.activity_reported.set(false);
                } else {
                    inner.monitor_silence.set(on);
                    if let Some(old) = inner.silence_timer.borrow_mut().take() {
                        old.remove();
                    }
                }
            });
            group.add_action(&a);
        }
        self.0.vte.insert_action_group("term", Some(&group));
        self.setup_file_drop();

        self.rebuild_shortcuts();
        // Apply custom shortcuts from the settings immediately
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

    /// Build shortcuts and context menu (with shortcut labels) from the current assignment.
    /// The shortcuts hang on the terminal itself and run in the capture phase before
    /// VTE's key handling. Global app accels would be resolved at the window and would not find
    /// this terminal's actions (found in the VM: Ctrl+= typed "=").
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
        // Link entries only visible if a link was under the pointer at right-click
        let link = gio::Menu::new();
        for (label, action) in [(N_("Open link"), "term.open-link"), (N_("Copy link"), "term.copy-link")] {
            let it = gio::MenuItem::new(Some(&gettext(label)), Some(action));
            it.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
            link.append_item(&it);
        }
        menu.append_section(None, &link);
        let edit = gio::Menu::new();
        item(&edit, &gettext("Copy"), "term.copy");
        item(&edit, &gettext("Copy as HTML"), "term.copy-html");
        item(&edit, &gettext("Paste"), "term.paste");
        item(&edit, &gettext("Paste special…"), "term.paste-advanced");
        item(&edit, &gettext("Select all"), "term.select-all");
        menu.append_section(None, &edit);
        let zoom = gio::Menu::new();
        item(&zoom, &gettext("Zoom in"), "term.zoom-in");
        item(&zoom, &gettext("Zoom out"), "term.zoom-out");
        item(&zoom, &gettext("Normal size"), "term.zoom-reset");
        menu.append_section(None, &zoom);
        // Pane actions live on the window (win.*); they apply to the focused pane,
        // which a right-click sets beforehand (see pane.rs).
        let search = gio::Menu::new();
        item(&search, &gettext("Search…"), "win.find");
        menu.append_section(None, &search);
        let split = gio::Menu::new();
        item(&split, &gettext("Split right"), "win.split-right");
        item(&split, &gettext("Split down"), "win.split-down");
        item(&split, &gettext("Maximize pane"), "win.toggle-zoom");
        item(&split, &gettext("Equalize sizes"), "win.equalize");
        menu.append_section(None, &split);
        // Less used entries in the submenu, otherwise the menu gets taller than small screens
        let more = gio::Menu::new();
        let output = gio::Menu::new();
        item(&output, &gettext("Save output…"), "term.save-output");
        item(&output, &gettext("Reset"), "term.reset");
        item(&output, &gettext("Reset and clear"), "term.reset-clear");
        item(&output, &gettext("Read only"), "term.read-only");
        more.append_section(None, &output);
        let monitor = gio::Menu::new();
        item(&monitor, &gettext("Monitor activity"), "term.monitor-activity");
        item(&monitor, &gettext("Monitor silence"), "term.monitor-silence");
        more.append_section(None, &monitor);
        let sub = gio::Menu::new();
        sub.append_submenu(Some(&gettext("Terminal")), &more);
        menu.append_section(None, &sub);
        let sync = gio::Menu::new();
        item(&sync, &gettext("Input to all panes"), "win.toggle-sync");
        let it = gio::MenuItem::new(Some(&gettext("This pane receives shared input")), Some("win.sync-pane"));
        it.set_attribute_value("hidden-when", Some(&"action-disabled".to_variant()));
        sync.append_item(&it);
        menu.append_section(None, &sync);
        let close = gio::Menu::new();
        item(&close, &gettext("Close pane"), "win.close-pane");
        menu.append_section(None, &close);
        self.0.vte.set_context_menu(&menu);
    }
}

/// Allowed schemes when opening links – no `javascript:`, `data:` or the like.
const SAFE_SCHEMES: &[&str] = &["http", "https", "ftp", "mailto", "file"];

/// Detect addresses without OSC 8 (PCRE2). Trailing punctuation is not part of them.
const URL_PATTERNS: &[&str] = &[
    r#"\b(?:https?|ftp|file)://[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}]"#,
    r#"\bmailto:[^\s<>"'`@]+@[^\s<>"'`]*[^\s<>"'`.,;:!?)\]}]"#,
    // File paths (audit T-53): /…, ~/…, ./…, ../…, also with :line:column; not in the middle of
    // words, addresses (`https://`) or dates (`10/23`)
    r#"(?<![\w/~.:@-])(?:~|\.\.?)?/[\w.+@%~/-]*[\w+@%~/-](?::\d+){0,2}"#,
];

/// How to search (toggles of the search bar).
#[derive(Clone, Copy, Default)]
pub struct SearchQuery {
    pub regex: bool,
    pub case_sensitive: bool,
    /// whole words only (Tilix "Match whole word"): pattern in \b…\b
    pub whole_words: bool,
}

impl SearchQuery {
    /// PCRE pattern for VTE and counting (both PCRE2)
    pub fn pattern(&self, text: &str) -> String {
        let p = if self.regex { text.to_string() } else { glib::Regex::escape_string(text).to_string() };
        if self.whole_words {
            format!("\\b(?:{p})\\b")
        } else {
            p
        }
    }
}

/// Above this the search shows "999+": otherwise counting takes noticeable time with a long scrollback.
pub const SEARCH_COUNT_MAX: usize = 999;
/// Save scrollback and visible text as a file (Tilix "Save…").
fn save_output(vte: &Backend) {
    let dialog = gtk::FileDialog::builder()
        .title(gettext("Save output"))
        .initial_name(gettext("terminal-output.txt"))
        .modal(true)
        .build();
    let window = vte.root().and_downcast::<gtk::Window>();
    let vte = vte.clone();
    dialog.save(window.as_ref(), gio::Cancellable::NONE, move |res| {
        let Ok(file) = res else { return };
        let written = file
            .replace(None, false, gio::FileCreateFlags::REPLACE_DESTINATION, gio::Cancellable::NONE)
            .and_then(|stream| {
                vte.write_contents(stream.upcast_ref())?;
                stream.close(gio::Cancellable::NONE)
            });
        if let Some(w) = vte.root().and_downcast::<crate::window::Window>() {
            match written {
                Ok(()) => w.toast(&trf(
                    "Output saved to “{path}”",
                    &[("path", &file.path().map(|p| p.display().to_string()).unwrap_or_default())],
                )),
                Err(e) => w.toast(&trf("Output not saved: {error}", &[("error", e.message())])),
            }
        }
    });
}

fn paste_with_check(vte: &Backend) {
    let vte = vte.clone();
    vte.clipboard().read_text_async(gio::Cancellable::NONE, move |res| {
        let Ok(Some(text)) = res else { return };
        let text = text.to_string();
        // Every line break counts, including one at the end of a single line (QA-04)
        let check = parkett_core::paste::check(&text);
        if !check.executes || !crate::settings::settings().boolean("warn-multiline-paste") {
            vte.paste(&text);
            return;
        }
        let lines = check.lines.len();
        let preview: Vec<&str> = check.lines.iter().take(6).copied().collect();
        let more = if lines > 6 {
            format!("\n{}", ntrf("… ({n} more line)", "… ({n} more lines)", (lines - 6) as u32, &[]))
        } else {
            String::new()
        };
        let (heading, body) = if lines == 1 {
            (
                gettext("Paste text with line break?"),
                gettext("The text ends with a line break. If the shell does not support “bracketed paste”, it runs immediately."),
            )
        } else {
            (
                gettext("Paste multi-line text?"),
                trf(
                    "The text has {lines} lines. If the shell does not support “bracketed paste”, they run immediately.",
                    &[("lines", &lines.to_string())],
                ),
            )
        };
        let dialog = adw::AlertDialog::new(Some(&heading), Some(&body));
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
        dialog.add_responses(&[("cancel", &gettext("Cancel")), ("paste", &gettext("Paste"))]);
        dialog.set_response_appearance("paste", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let v2 = vte.clone();
        dialog.connect_response(Some("paste"), move |_, _| v2.paste(&text));
        dialog.present(Some(&*vte));
    });
}

/// Paste special: edit clipboard text first, convert tabs and
/// line endings (Tilix "Advanced Paste"). Replaces the confirmation for line breaks –
/// the text is already fully shown in the dialog.
fn paste_advanced(vte: &Backend) {
    let vte = vte.clone();
    vte.clipboard().read_text_async(gio::Cancellable::NONE, move |res| {
        let text = match res {
            Ok(Some(t)) => t.to_string(),
            _ => String::new(),
        };
        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&text);
        let view = gtk::TextView::builder()
            .buffer(&buffer)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .top_margin(8)
            .bottom_margin(8)
            .left_margin(10)
            .right_margin(10)
            .build();
        let scroll = gtk::ScrolledWindow::builder()
            .child(&view)
            .min_content_height(160)
            .max_content_height(360)
            .propagate_natural_height(true)
            .min_content_width(420)
            .build();
        let frame = gtk::Frame::builder().child(&scroll).build();
        let tabs = gtk::CheckButton::with_label(&gettext("Convert tabs to spaces"));
        let width = gtk::SpinButton::with_range(1.0, 8.0, 1.0);
        width.set_value(4.0);
        width.set_tooltip_text(Some(&gettext("Spaces per tab")));
        tabs.bind_property("active", &width, "sensitive").sync_create().build();
        let tab_row = gtk::Box::builder().spacing(8).build();
        tab_row.append(&tabs);
        tab_row.append(&width);
        let crlf = gtk::CheckButton::with_label(&gettext("Convert Windows line endings (CRLF) to LF"));
        crlf.set_active(text.contains('\r'));
        let content = gtk::Box::builder().orientation(gtk::Orientation::Vertical).spacing(8).build();
        content.append(&frame);
        content.append(&tab_row);
        content.append(&crlf);
        let dialog = adw::AlertDialog::new(Some(&gettext("Paste special")), None);
        dialog.set_extra_child(Some(&content));
        dialog.add_responses(&[("cancel", &gettext("Cancel")), ("paste", &gettext("Paste"))]);
        dialog.set_response_appearance("paste", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("paste"));
        dialog.set_close_response("cancel");
        let v2 = vte.clone();
        dialog.connect_response(Some("paste"), move |_, _| {
            let (start, end) = buffer.bounds();
            let raw = buffer.text(&start, &end, false);
            let width = tabs.is_active().then(|| width.value_as_int().max(1) as usize);
            v2.paste(&parkett_core::paste::transform(&raw, width, crlf.is_active()));
        });
        dialog.present(Some(&*vte));
        view.grab_focus();
    });
}

fn rgba(c: &Rgb) -> gtk::gdk::RGBA {
    let (r, g, b) = c.to_f64();
    gtk::gdk::RGBA::new(r as f32, g as f32, b as f32, 1.0)
}

fn from_rgba(c: &gtk::gdk::RGBA) -> Rgb {
    Rgb::from_f64(c.red().into(), c.green().into(), c.blue().into())
}

/// Resolve `~` and `~/…` to the home folder; empty = no preset.
fn expand_home(dir: &str) -> Option<String> {
    (!dir.is_empty()).then(|| parkett_core::paths::expand_home(dir, &glib::home_dir()).to_string_lossy().into_owned())
}

/// The user's login shell according to VTE (passwd), otherwise $SHELL, otherwise /bin/sh.
/// Parkett's environment as `NAME=value`.
fn own_environment() -> Vec<String> {
    std::env::vars_os().map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy())).collect()
}

fn user_shell(backend: &Backend) -> String {
    backend
        .user_shell()
        .or_else(|| std::env::var("SHELL").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".into())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExitStatus {
    Code(i32),
    Signal(i32),
}

/// `child-exited` returns the raw wait() status.
fn decode_wait_status(status: i32) -> ExitStatus {
    if status & 0x7f == 0 {
        ExitStatus::Code((status >> 8) & 0xff)
    } else {
        ExitStatus::Signal(status & 0x7f)
    }
}

/// Name of a local user from the user database, for the badge of another user.
fn user_name(uid: u32) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|l| {
        let mut f = l.split(':');
        let name = f.next()?;
        (f.nth(1)?.parse::<u32>().ok()? == uid).then(|| name.to_string())
    })
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
