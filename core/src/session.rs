// SPDX-License-Identifier: GPL-3.0-or-later
//! Sitzung: offene Fenster und Reiter, damit sie nach einem Neustart wiederkommen.
//!
//! Ein Reiter wird als [`Template`] gespeichert: Verzeichnis `/`, das Arbeitsverzeichnis je
//! Bereich als Unterverzeichnis (absoluter Pfad ohne führenden Schrägstrich), keine
//! Startbefehle – Prozesse kommen nicht zurück, nur Aufteilung, Profile, Verzeichnisse und
//! eigene Titel. Gespeichert als JSON (`$XDG_STATE_HOME/parkett/session.json`).

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::template::{PaneSpec, Template};

pub const VERSION: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub windows: Vec<SessionWindow>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionWindow {
    /// Index des gewählten Reiters
    pub selected: usize,
    pub tabs: Vec<Template>,
}

/// Angaben eines Bereichs für die Sitzung.
pub fn pane(profile: &str, cwd: Option<&str>) -> PaneSpec {
    PaneSpec {
        profile: profile.to_string(),
        subdir: cwd.map(|c| c.trim_start_matches('/').to_string()).unwrap_or_default(),
        ..PaneSpec::default()
    }
}

impl Session {
    /// Fehlende oder kaputte Datei = keine Sitzung (`None`); eine neuere Version auch.
    pub fn load(path: &Path) -> Option<Session> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut s: Session = serde_json::from_str(&text).ok()?;
        if s.version > VERSION {
            return None;
        }
        s.windows.retain(|w| !w.tabs.is_empty());
        for w in &mut s.windows {
            w.selected = w.selected.min(w.tabs.len() - 1);
            for t in &mut w.tabs {
                // Verzeichnis steht immer auf „/“; eine Datei von Hand mit etwas anderem nicht
                // als Basis für beliebige Orte nutzen
                t.directory = "/".into();
                for p in t.layout.panes_mut() {
                    p.command.clear();
                }
            }
        }
        (!s.windows.is_empty()).then_some(s)
    }

    /// Schreibt atomar (temporäre Datei + Umbenennen).
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        crate::paths::write_private(&tmp, path, &serde_json::to_string_pretty(self).map_err(io::Error::other)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Layout, Orientation};
    use crate::paths::join_subdir;

    fn sample() -> Session {
        let (layout, _) = Layout::new().split(1, Orientation::Horizontal).unwrap();
        let tab = Template::from_layout("", &layout, &|id| {
            if id == 1 {
                pane("standard", Some("/home/a/src"))
            } else {
                pane("server", Some("/tmp"))
            }
        });
        let mut tab = tab;
        tab.directory = "/".into();
        Session { version: VERSION, windows: vec![SessionWindow { selected: 0, tabs: vec![tab] }] }
    }

    #[test]
    fn roundtrip_and_paths() {
        let dir = std::env::temp_dir().join(format!("parkett-session-{}", std::process::id()));
        let path = dir.join("session.json");
        let s = sample();
        s.save(&path).unwrap();
        let back = Session::load(&path).unwrap();
        assert_eq!(back, s);
        let (_, specs) = back.windows[0].tabs[0].instantiate();
        let dirs: Vec<_> = specs.iter().map(|(_, p)| join_subdir(Path::new("/"), &p.subdir)).collect();
        assert_eq!(dirs, [Path::new("/home/a/src"), Path::new("/tmp")]);
        assert_eq!(specs[1].1.profile, "server");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn broken_or_empty_is_none() {
        let dir = std::env::temp_dir().join(format!("parkett-session-b-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        assert!(Session::load(&path).is_none());
        std::fs::write(&path, "{kaputt").unwrap();
        assert!(Session::load(&path).is_none());
        std::fs::write(&path, r#"{"version":1,"windows":[{"selected":3,"tabs":[]}]}"#).unwrap();
        assert!(Session::load(&path).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn commands_are_dropped_on_load() {
        let dir = std::env::temp_dir().join(format!("parkett-session-c-{}", std::process::id()));
        let path = dir.join("session.json");
        let mut s = sample();
        s.windows[0].tabs[0].layout.panes_mut()[0].command = "rm -rf ~".into();
        s.windows[0].tabs[0].directory = "/etc".into();
        s.windows[0].selected = 7;
        s.save(&path).unwrap();
        let back = Session::load(&path).unwrap();
        assert!(back.windows[0].tabs[0].commands().is_empty());
        assert_eq!(back.windows[0].tabs[0].directory, "/");
        assert_eq!(back.windows[0].selected, 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
