//! Manual check of the Flatpak host start (`cargo run -p parkett-core --example pty_host`):
//! inside a sandbox with `--talk-name=org.freedesktop.Flatpak` it runs a command on the host through
//! a PTY and prints what the program sees; outside a sandbox `HOST=1` forces the `flatpak-spawn`
//! command line (it fails there, which is expected).
use parkett_core::pty::{in_flatpak, Pty, SpawnOptions};
use std::time::Duration;

fn main() {
    println!("host login shell: {:?}", parkett_core::pty::host_user_shell());
    let script = std::env::args().nth(1).unwrap_or_else(|| {
        "tty; ps -o pid=,pgid=,sid=,tty= -p $$; stty size; echo FOO=$FOO TERM=$TERM; pwd; sleep 1; exit 3".into()
    });
    let mut pty = Pty::spawn(&SpawnOptions {
        argv: vec!["sh".into(), "-c".into(), script],
        cwd: Some("/tmp".into()),
        env: vec!["FOO=bar".into()],
        cols: 100,
        rows: 30,
        host: in_flatpak() || std::env::var_os("HOST").is_some(),
        ..Default::default()
    })
    .expect("spawn");
    let mut buf = [0u8; 4096];
    if std::env::var_os("CTRL_C").is_some() {
        // an interactive shell on the host: run sleep, then Ctrl+C must stop only the sleep
        std::thread::sleep(Duration::from_millis(1500));
        pty.write_all(b"sleep 30\n").unwrap();
        std::thread::sleep(Duration::from_millis(800));
        pty.write_all(&[3]).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        pty.write_all(b"echo STILL-ALIVE-$?; exit\n").unwrap();
    }
    if std::env::var_os("CHECK_FG").is_some() {
        // interactive bash on the host: find its pid in the OSC it prints, run sleep, ask for the foreground job
        let mut seen = String::new();
        let t0 = std::time::Instant::now();
        let mut pid = None;
        while pid.is_none() && t0.elapsed() < Duration::from_secs(5) {
            if let Ok(n) = pty.read(&mut buf) {
                seen.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
            pid = seen
                .split("parkett-host-pid;")
                .nth(1)
                .and_then(|r| r.split('\u{7}').next())
                .and_then(|p| p.parse::<u32>().ok());
            std::thread::sleep(Duration::from_millis(50));
        }
        println!("host pid from the OSC: {pid:?}");
        let pid = pid.expect("no host pid");
        println!("at the prompt: {:?}", parkett_core::pty::host_foreground(pid));
        pty.write_all(b"sleep 30\n").unwrap();
        std::thread::sleep(Duration::from_millis(800));
        println!("while sleep runs: {:?}", parkett_core::pty::host_foreground(pid));
        pty.write_all(&[3]).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        pty.write_all(b"exit\n").unwrap();
    }
    loop {
        match pty.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => print!("{}", String::from_utf8_lossy(&buf[..n]).replace('\r', "")),
        }
    }
    loop {
        if let Some(st) = pty.try_wait().unwrap() {
            println!("exit status: {st:?} (in_flatpak={})", in_flatpak());
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
