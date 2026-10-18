// SPDX-License-Identifier: GPL-3.0-or-later
//! Die TOML-Beispiele in docs/BEDIENUNG.md müssen sich laden lassen.

use parkett_core::profile::{ExitAction, Profile};
use parkett_core::template::Template;

const DOC: &str = include_str!("../../docs/BEDIENUNG.md");

fn blocks(lang: &str) -> Vec<String> {
    let fence = format!("```{lang}\n");
    DOC.split(fence.as_str()).skip(1).map(|b| b.split("```").next().unwrap().to_string()).collect()
}

#[test]
fn profil_beispiel() {
    let p = Profile::from_toml("server", &blocks("toml")[0]).expect("Profil-Beispiel");
    assert_eq!(p.palette, "nord");
    assert!(p.login_shell);
    assert_eq!(p.exit_action, ExitAction::Hold);
    assert_eq!(p.valid_environment(), vec!["EDITOR=vim".to_string()]);
}

#[test]
fn vorlagen_beispiel() {
    let t = Template::from_toml("web", &blocks("toml")[1]).expect("Vorlagen-Beispiel");
    let names: Vec<_> = t.layout.panes().iter().map(|p| p.name.clone()).collect();
    assert_eq!(names, ["Editor", "Dev-Server", ""]);
    assert_eq!(t.layout.panes()[2].subdir, "tests");
}
