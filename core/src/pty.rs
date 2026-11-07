//! Pseudo-terminal and child process without VTE (phase 2 of the libghostty backend, see
//! `docs/LIBGHOSTTY.md`). The child is started through `std::process::Command` with a
//! `pre_exec` hook (`setsid`, controlling terminal), never a raw `fork` in the multi-threaded
//! GTK process.
use crate::startup::TtyMode;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};

/// What to start.
#[derive(Clone, Debug, Default)]
pub struct SpawnOptions {
    /// Program and arguments; the program is searched in `PATH`.
    pub argv: Vec<String>,
    /// Working directory; `None` keeps Parkett's.
    pub cwd: Option<String>,
    /// `KEY=VALUE` entries added to (or, with `clean_env`, replacing) the environment.
    pub env: Vec<String>,
    /// Do not inherit Parkett's own environment (call to a running instance, QA-08).
    pub clean_env: bool,
    pub cols: u16,
    pub rows: u16,
    /// Start the program on the host from inside a Flatpak sandbox (`flatpak-spawn --host`).
    pub host: bool,
}

/// Are we inside a Flatpak sandbox?
pub fn in_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}

/// The login shell of the user on the host, asked through `flatpak-spawn --host` (inside a Flatpak
/// sandbox `/etc/passwd` and `$SHELL` describe the sandbox). `None` outside a sandbox or on failure.
pub fn host_user_shell() -> Option<String> {
    if !in_flatpak() {
        return None;
    }
    let out = std::process::Command::new("flatpak-spawn")
        .args(["--host", "sh", "-c", "getent passwd \"$(id -un)\" | cut -d: -f7"])
        .output()
        .ok()?;
    let shell = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && shell.starts_with('/')).then_some(shell)
}

/// `TERM` and `COLORTERM` like VTE sets them; the profile may override them.
const DEFAULT_ENV: [(&str, &str); 2] = [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")];

/// The command line that runs `opts.argv` on the host from inside a Flatpak sandbox:
/// `flatpak-spawn` hands its stdio (our PTY slave) to the host process through the session helper,
/// which gives the program its own session with that PTY as controlling terminal (checked:
/// session leader, `tty_nr` and foreground group of the PTY; a wrapper like `setsid -c` fails with
/// EPERM because the terminal already is one). The exit status comes back through `flatpak-spawn`.
/// `--watch-bus` ends the program when Parkett's connection to the bus goes away. The sandbox environment does not reach the
/// host, so the variables are passed one by one; `clean_env` also drops the helper's own.
pub fn host_argv(opts: &SpawnOptions) -> Vec<String> {
    let mut v: Vec<String> = ["flatpak-spawn", "--host", "--watch-bus"].map(String::from).into();
    if let Some(dir) = &opts.cwd {
        v.push(format!("--directory={dir}"));
    }
    if opts.clean_env {
        v.push("--clear-env".into());
    }
    for (k, val) in DEFAULT_ENV {
        if !opts.env.iter().any(|e| e.split_once('=').is_some_and(|(n, _)| n == k)) {
            v.push(format!("--env={k}={val}"));
        }
    }
    for e in opts.env.iter().filter(|e| e.contains('=')) {
        v.push(format!("--env={e}"));
    }
    v.push("--".into());
    v.extend(opts.argv.iter().cloned());
    v
}

/// Master side of a PTY plus the child on its slave side.
pub struct Pty {
    master: File,
    child: Child,
}

fn cvt(r: libc::c_int) -> io::Result<libc::c_int> {
    if r < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(r)
    }
}

fn winsize(cols: u16, rows: u16, px_w: u16, px_h: u16) -> libc::winsize {
    libc::winsize { ws_row: rows, ws_col: cols, ws_xpixel: px_w, ws_ypixel: px_h }
}

impl Pty {
    pub fn spawn(opts: &SpawnOptions) -> io::Result<Pty> {
        if opts.argv.is_empty() {
            return Err(io::Error::other("empty command"));
        }
        let host_cmd = opts.host.then(|| host_argv(opts));
        let argv = host_cmd.as_ref().unwrap_or(&opts.argv);
        let (program, args) = argv.split_first().ok_or_else(|| io::Error::other("empty command"))?;
        // SAFETY: plain libc calls; every descriptor is wrapped in OwnedFd right away.
        let (master, slave) = unsafe {
            let m = cvt(libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC))?;
            let master = OwnedFd::from_raw_fd(m);
            cvt(libc::grantpt(m))?;
            cvt(libc::unlockpt(m))?;
            let mut name = [0 as libc::c_char; 128];
            cvt(libc::ptsname_r(m, name.as_mut_ptr(), name.len()))?;
            let s = cvt(libc::open(name.as_ptr(), libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC))?;
            let ws = winsize(opts.cols.max(1), opts.rows.max(1), 0, 0);
            cvt(libc::ioctl(s, libc::TIOCSWINSZ, &ws))?;
            (master, OwnedFd::from_raw_fd(s))
        };

        let mut cmd = Command::new(program);
        cmd.args(args);
        if host_cmd.is_none() {
            if let Some(dir) = &opts.cwd {
                cmd.current_dir(dir);
            }
            if opts.clean_env {
                cmd.env_clear();
            }
            // Same defaults VTE sets; the profile may override them below.
            for (k, v) in DEFAULT_ENV {
                cmd.env(k, v);
            }
            for entry in &opts.env {
                if let Some((k, v)) = entry.split_once('=') {
                    cmd.env(OsStr::new(k), OsStr::new(v));
                }
            }
        }
        cmd.stdin(Stdio::from(slave.try_clone()?)).stdout(Stdio::from(slave.try_clone()?)).stderr(Stdio::from(slave));
        // SAFETY: only async-signal-safe calls (setsid, ioctl) between fork and exec.
        // With `flatpak-spawn` the PTY must not become the controlling terminal of that helper's
        // session: the session helper on the host takes the terminal over itself (see `host_argv`).
        let take_tty = host_cmd.is_none();
        unsafe {
            cmd.pre_exec(move || {
                cvt(libc::setsid())?;
                if take_tty {
                    cvt(libc::ioctl(0, libc::TIOCSCTTY, 0))?;
                }
                Ok(())
            });
        }
        let child = cmd.spawn()?;
        drop(cmd); // closes the parent's copies of the slave
        Ok(Pty { master: File::from(master), child })
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// A duplicate of the master descriptor, e.g. to watch it from an event loop.
    pub fn try_clone_master(&self) -> io::Result<OwnedFd> {
        self.master.as_fd().try_clone_to_owned()
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }

    /// Output of the program. `Ok(0)` and `EIO` both mean the child side is closed.
    pub fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.master.read(buf) {
            Err(e) if e.raw_os_error() == Some(libc::EIO) => Ok(0),
            r => r,
        }
    }

    /// Keys and pasted text for the program.
    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.master.write_all(data)
    }

    pub fn set_nonblocking(&self, on: bool) -> io::Result<()> {
        let fd = self.master.as_fd().as_raw_fd();
        // SAFETY: valid descriptor owned by `self`.
        unsafe {
            let fl = cvt(libc::fcntl(fd, libc::F_GETFL))?;
            let fl = if on { fl | libc::O_NONBLOCK } else { fl & !libc::O_NONBLOCK };
            cvt(libc::fcntl(fd, libc::F_SETFL, fl))?;
        }
        Ok(())
    }

    /// Tell the program the new size (SIGWINCH follows from the kernel).
    pub fn resize(&self, cols: u16, rows: u16, px_w: u16, px_h: u16) -> io::Result<()> {
        let ws = winsize(cols.max(1), rows.max(1), px_w, px_h);
        // SAFETY: valid descriptor, `ws` is a complete winsize.
        cvt(unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &ws) }).map(|_| ())
    }

    /// Process group in the foreground of the terminal (the running command, not the shell).
    pub fn foreground_pgid(&self) -> Option<i32> {
        // SAFETY: valid descriptor.
        let p = unsafe { libc::tcgetpgrp(self.master.as_raw_fd()) };
        (p > 0).then_some(p)
    }

    /// Line-wise input and echo: detects a password prompt (see `startup`).
    pub fn tty_mode(&self) -> Option<TtyMode> {
        // SAFETY: valid descriptor, `t` is a complete termios.
        let mut t: libc::termios = unsafe { std::mem::zeroed() };
        if unsafe { libc::tcgetattr(self.master.as_raw_fd(), &mut t) } != 0 {
            return None;
        }
        Some(TtyMode { canonical: t.c_lflag & libc::ICANON != 0, echo: t.c_lflag & libc::ECHO != 0 })
    }

    /// Exit status once the child has ended, `None` while it runs.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    /// Hang up the terminal like closing a window: SIGHUP to the child's process group.
    pub fn hangup(&self) {
        // SAFETY: signalling the group of our own child (it called setsid, so pgid == pid).
        unsafe { libc::kill(-(self.child.id() as i32), libc::SIGHUP) };
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        self.hangup();
        let _ = self.child.try_wait(); // reaps a child that already ended; others get SIGHUP
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn opts(script: &str) -> SpawnOptions {
        SpawnOptions { argv: vec!["sh".into(), "-c".into(), script.into()], cols: 80, rows: 24, ..Default::default() }
    }

    fn drain(p: &mut Pty) -> String {
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match p.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
            }
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn output_and_exit_status() {
        let mut p = Pty::spawn(&opts("echo hi; exit 3")).unwrap();
        assert_eq!(drain(&mut p), "hi\r\n");
        let st = loop {
            if let Some(s) = p.try_wait().unwrap() {
                break s;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(st.code(), Some(3));
    }

    #[test]
    fn cwd_env_size_and_controlling_tty() {
        let mut o = opts("pwd; echo $FOO $TERM; stty size; tty >/dev/null && echo tty");
        o.cwd = Some("/tmp".into());
        o.env = vec!["FOO=bar".into()];
        o.cols = 100;
        o.rows = 30;
        let mut p = Pty::spawn(&o).unwrap();
        assert_eq!(drain(&mut p), "/tmp\r\nbar xterm-256color\r\n30 100\r\ntty\r\n");
    }

    #[test]
    fn clean_env_drops_parent_variables() {
        std::env::set_var("PARKETT_PTY_TEST", "x");
        let mut o = opts("echo [${PARKETT_PTY_TEST}]");
        o.clean_env = true;
        let mut p = Pty::spawn(&o).unwrap();
        assert_eq!(drain(&mut p), "[]\r\n");
    }

    #[test]
    fn input_echo_resize_and_foreground() {
        let mut p =
            Pty::spawn(&SpawnOptions { argv: vec!["cat".into()], cols: 80, rows: 24, ..Default::default() }).unwrap();
        p.write_all(b"ping\n").unwrap();
        let mut buf = [0u8; 64];
        let mut got = String::new();
        while !got.contains("ping\r\nping") {
            let n = p.read(&mut buf).unwrap();
            got.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        assert_eq!(p.foreground_pgid(), Some(p.pid() as i32));
        let m = p.tty_mode().unwrap();
        assert!(m.canonical && m.echo);
        p.resize(120, 40, 0, 0).unwrap();
        drop(p); // SIGHUP ends cat
    }

    #[test]
    fn host_command_line() {
        let o = SpawnOptions {
            argv: vec!["bash".into(), "-l".into()],
            cwd: Some("/home/x".into()),
            env: vec!["FOO=bar".into(), "TERM=vt100".into()],
            clean_env: true,
            ..Default::default()
        };
        assert_eq!(
            host_argv(&o),
            [
                "flatpak-spawn",
                "--host",
                "--watch-bus",
                "--directory=/home/x",
                "--clear-env",
                "--env=COLORTERM=truecolor",
                "--env=FOO=bar",
                "--env=TERM=vt100",
                "--",
                "bash",
                "-l"
            ]
        );
    }

    #[test]
    fn hangup_ends_child() {
        let mut p = Pty::spawn(&opts("sleep 30")).unwrap();
        p.hangup();
        let st = loop {
            if let Some(s) = p.try_wait().unwrap() {
                break s;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(st.signal(), Some(libc::SIGHUP));
    }

    #[test]
    fn missing_program_is_an_error() {
        assert!(Pty::spawn(&opts_prog("/nonexistent/prog")).is_err());
    }

    fn opts_prog(p: &str) -> SpawnOptions {
        SpawnOptions { argv: vec![p.into()], cols: 80, rows: 24, ..Default::default() }
    }
}
