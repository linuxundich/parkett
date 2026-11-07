//! PTY (parkett-core) feeding the terminal state: the pipeline of the future backend, headless.
use ghostty_vt::Terminal;
use parkett_core::pty::{Pty, SpawnOptions};

#[test]
fn shell_output_lands_on_the_screen() {
    let mut pty = Pty::spawn(&SpawnOptions {
        argv: vec!["sh".into(), "-c".into(), "printf '\\033]2;demo\\007\\033[1;31mred\\033[0m\\nzwei'".into()],
        cwd: Some("/tmp".into()),
        cols: 40,
        rows: 5,
        ..Default::default()
    })
    .unwrap();
    let mut term = Terminal::new(40, 5).unwrap();
    let mut buf = [0u8; 4096];
    loop {
        match pty.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => term.write(&buf[..n]),
        }
    }
    assert_eq!(term.title().as_deref(), Some("demo"));
    assert_eq!(term.text(), "red\nzwei");
}
