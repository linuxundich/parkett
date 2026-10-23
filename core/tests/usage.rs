// SPDX-License-Identifier: GPL-3.0-or-later
//! Die TOML-Beispiele in docs/USAGE.md müssen sich laden lassen.

use parkett_core::profile::{ExitAction, Profile};
use parkett_core::template::Template;

const DOC: &str = include_str!("../../docs/USAGE.md");

fn blocks(lang: &str) -> Vec<String> {
    let fence = format!("```{lang}\n");
    DOC.split(fence.as_str()).skip(1).map(|b| b.split("```").next().unwrap().to_string()).collect()
}

/// Erster TOML-Block, der `marker` enthält (Reihenfolge in der Doku egal).
fn block_with(marker: &str) -> String {
    blocks("toml").into_iter().find(|b| b.contains(marker)).unwrap_or_else(|| panic!("kein Beispiel mit {marker}"))
}

#[test]
fn profil_beispiel() {
    let p = Profile::from_toml("server", &block_with("palette =")).expect("Profil-Beispiel");
    assert_eq!(p.palette, "nord");
    assert!(p.login_shell);
    assert_eq!(p.exit_action, ExitAction::Hold);
    assert_eq!(p.valid_environment(), vec!["EDITOR=vim".to_string()]);
    assert_eq!(p.font, "JetBrains Mono 11");
    assert_eq!(p.cursor_shape, parkett_core::profile::CursorShape::Ibeam);
    assert_eq!(p.bell, parkett_core::profile::Bell::Mark);
    assert_eq!(p.opacity(), 0.9);
    assert_eq!(p.hosts, ["bender", "*.tuxedo.de"]);
    assert_eq!(p.directories, ["~/kunde"]);
    assert_eq!(p.line_spacing, 1.2);
    assert_eq!(p.links.len(), 1);
    assert_eq!(p.links[0].pattern, r"\bPARK-(\d+)\b");
}

#[test]
fn vorlagen_beispiel() {
    let t = Template::from_toml("web", &block_with("[layout]")).expect("Vorlagen-Beispiel");
    let names: Vec<_> = t.layout.panes().iter().map(|p| p.name.clone()).collect();
    assert_eq!(names, ["Editor", "Dev-Server", ""]);
    assert_eq!(t.layout.panes()[2].subdir, "tests");
}

#[test]
fn lesezeichen_beispiel() {
    let list = parkett_core::bookmark::parse(&block_with("[[bookmark]]")).expect("Lesezeichen-Beispiel");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].path, "~/05_Projekte");
    assert_eq!(list[1].shell_line().as_deref(), Some("journalctl -f"));
    assert_eq!(list[2].shell_line().as_deref(), Some("ssh -p 2222 toff@bender"));
}
