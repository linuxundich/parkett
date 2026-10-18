//! Ein Terminal-Bereich: VTE-Widget, Shell-Prozess und die Leiste „Prozess beendet“.
//!
//! Zustand (Titel, Verzeichnis) kommt aus den VTE-Termprops (`xterm.title`, `vte.cwd`),
//! nicht aus den seit VTE 0.78 veralteten Signalen.

use std::cell::{Cell, RefCell};
use std::os::fd::AsRawFd;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib, pango};
use gtk::glib::translate::{from_glib_full, ToGlibPtr};
use vte::prelude::*;

use crate::colors::{self, Scheme};

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
    child_pid: Cell<Option<glib::Pid>>,
    exit: Cell<Option<ExitStatus>>,
    on_state_changed: RefCell<Vec<Callback>>,
    on_close: RefCell<Vec<Callback>>,
}

#[derive(Clone)]
pub struct Terminal(Rc<Inner>);

impl Terminal {
    pub fn new(cwd: Option<String>) -> Self {
        let vte = vte::Terminal::builder()
            .hexpand(true)
            .vexpand(true)
            .scrollback_lines(10_000)
            .allow_hyperlink(true)
            .pointer_autohide(true)
            .bold_is_bright(false)
            .enable_a11y(true)
            .build();
        vte.set_size(100, 30);

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
            child_pid: Cell::new(None),
            exit: Cell::new(None),
            on_state_changed: RefCell::default(),
            on_close: RefCell::default(),
        }));

        t.setup_style();
        t.setup_actions();
        t.setup_signals();

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

    pub fn connect_close(&self, f: impl Fn() + 'static) {
        self.0.on_close.borrow_mut().push(Box::new(f));
    }

    /// Fenstertitel, den die Shell per OSC 0/2 gesetzt hat.
    pub fn title(&self) -> Option<String> {
        self.0.vte.termprop_string("xterm.title").0.map(String::from).filter(|s| !s.is_empty())
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
        let shell = user_shell();
        let cwd = self.0.cwd_at_start.clone().or_else(|| glib::home_dir().to_str().map(String::from));
        let weak = Rc::downgrade(&self.0);
        let shell_for_error = shell.clone();
        self.0.vte.spawn_async(
            vte::PtyFlags::DEFAULT,
            cwd.as_deref(),
            &[shell.as_str()],
            &[],
            glib::SpawnFlags::DEFAULT,
            || {},
            -1,
            gio::Cancellable::NONE,
            move |result| {
                let Some(inner) = weak.upgrade() else { return };
                match result {
                    Ok(pid) => inner.child_pid.set(Some(pid)),
                    Err(err) => {
                    let t = Terminal(inner);
                    t.0.exit.set(Some(ExitStatus::Code(-1)));
                    t.show_exit(&format!("<b>{}</b> konnte nicht gestartet werden: {}",
                        glib::markup_escape_text(&shell_for_error), glib::markup_escape_text(err.message())));
                    t.emit_state_changed();
                    }
                }
            },
        );
        self.0.vte.grab_focus();
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
            let text = match exit {
                ExitStatus::Code(0) => "<b>Prozess beendet</b>".to_string(),
                ExitStatus::Code(c) => format!("<b>Prozess beendet</b> · Exit-Code {c}"),
                ExitStatus::Signal(s) => format!("<b>Prozess beendet</b> · Signal {s}"),
            };
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

    fn setup_style(&self) {
        let style = adw::StyleManager::default();
        // Nur schwache Referenz: der StyleManager lebt so lange wie die App. Eine starke
        // Referenz hielte das VTE-Widget fest – es würde nie freigegeben und die Shell nie
        // beendet (in der VM gefunden: Shell und „sleep“ liefen nach dem Schließen weiter).
        let apply = {
            let vte = self.0.vte.downgrade();
            move |sm: &adw::StyleManager| {
                let Some(vte) = vte.upgrade() else { return };
                let s = if sm.is_dark() { &colors::DARK } else { &colors::LIGHT };
                let palette: Vec<_> = s.palette.iter().map(|c| Scheme::rgba(c)).collect();
                let refs: Vec<_> = palette.iter().collect();
                vte.set_colors(Some(&Scheme::rgba(s.fg)), Some(&Scheme::rgba(s.bg)), &refs);
                vte.set_font(Some(&pango::FontDescription::from_string(&sm.monospace_font_name())));
            }
        };
        apply(&style);
        let handlers = std::cell::RefCell::new(Some((
            style.connect_dark_notify(apply.clone()),
            style.connect_monospace_font_name_notify(apply),
        )));
        self.0.vte.connect_destroy(move |_| {
            if let Some((a, b)) = handlers.borrow_mut().take() {
                let style = adw::StyleManager::default();
                style.disconnect(a);
                style.disconnect(b);
            }
        });
    }

    fn setup_actions(&self) {
        let group = gio::SimpleActionGroup::new();
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
        add("paste", Box::new(|v| v.paste_clipboard()));
        add("select-all", Box::new(|v| v.select_all()));
        add("zoom-in", Box::new(|v| v.set_font_scale((v.font_scale() * 1.1).min(4.0))));
        add("zoom-out", Box::new(|v| v.set_font_scale((v.font_scale() / 1.1).max(0.3))));
        add("zoom-reset", Box::new(|v| v.set_font_scale(1.0)));
        self.0.vte.insert_action_group("term", Some(&group));

        // Kürzel hängen am Terminal selbst und laufen in der Capture-Phase vor der
        // Tastaturbehandlung von VTE. Globale App-Accels würden am Fenster aufgelöst und
        // fänden die Aktionen dieses Terminals nicht (in der VM gefunden: Strg+= tippte „=“).
        let shortcuts = gtk::ShortcutController::new();
        shortcuts.set_propagation_phase(gtk::PropagationPhase::Capture);
        for (accels, action) in SHORTCUTS {
            shortcuts.add_shortcut(gtk::Shortcut::new(
                gtk::ShortcutTrigger::parse_string(accels),
                Some(gtk::NamedAction::new(&format!("term.{action}"))),
            ));
        }
        self.0.vte.add_controller(shortcuts);

        let item = |menu: &gio::Menu, label: &str, action: &str| {
            let it = gio::MenuItem::new(Some(label), Some(&format!("term.{action}")));
            if let Some((accels, _)) = SHORTCUTS.iter().find(|(_, a)| *a == action) {
                // erstes Kürzel als Anzeige im Kontextmenü
                it.set_attribute_value("accel", Some(&accels.split('|').next().unwrap_or("").to_variant()));
            }
            menu.append_item(&it);
        };
        let menu = gio::Menu::new();
        let edit = gio::Menu::new();
        item(&edit, "Kopieren", "copy");
        item(&edit, "Einfügen", "paste");
        item(&edit, "Alles auswählen", "select-all");
        menu.append_section(None, &edit);
        let zoom = gio::Menu::new();
        item(&zoom, "Vergrößern", "zoom-in");
        item(&zoom, "Verkleinern", "zoom-out");
        item(&zoom, "Normale Größe", "zoom-reset");
        menu.append_section(None, &zoom);
        // Bereichsaktionen leben am Fenster (win.*); sie gelten für den fokussierten Bereich,
        // den ein Rechtsklick vorher setzt (siehe pane.rs).
        let split = gio::Menu::new();
        split.append(Some("Rechts teilen"), Some("win.split-right"));
        split.append(Some("Unten teilen"), Some("win.split-down"));
        split.append(Some("Bereich maximieren"), Some("win.toggle-zoom"));
        split.append(Some("Größen angleichen"), Some("win.equalize"));
        menu.append_section(None, &split);
        let close = gio::Menu::new();
        close.append(Some("Bereich schließen"), Some("win.close-pane"));
        menu.append_section(None, &close);
        self.0.vte.set_context_menu_model(Some(&menu));
    }
}

/// Kürzel je Terminal-Aktion; mehrere Auslöser mit „|“ (Syntax von GtkShortcutTrigger).
const SHORTCUTS: &[(&str, &str)] = &[
    ("<Control><Shift>c", "copy"),
    ("<Control><Shift>v|<Shift>Insert", "paste"),
    ("<Control><Shift>a", "select-all"),
    ("<Control>plus|<Control>equal|<Control>KP_Add", "zoom-in"),
    ("<Control>minus|<Control>KP_Subtract", "zoom-out"),
    ("<Control>0|<Control>KP_0", "zoom-reset"),
];

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
