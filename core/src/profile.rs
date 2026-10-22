// SPDX-License-Identifier: GPL-3.0-or-later
//! Profile: wie ein Terminal startet und aussieht. Gespeichert als TOML-Dateien
//! (`$XDG_CONFIG_HOME/parkett/profiles/<id>.toml`), damit man sie von Hand bearbeiten
//! und in Dotfiles mitnehmen kann.

use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::palette::DEFAULT_PALETTE;

/// Was passiert, wenn der Prozess im Bereich endet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExitAction {
    /// Leiste „Prozess beendet“ mit Neu starten / Schließen
    Hold,
    /// Bereich schließen, wenn der Prozess ohne Fehler endet; bei Fehler Leiste. Standard wie
    /// in GNOME Console, Ptyxis und Tilix: `exit` schließt (QA-11)
    #[default]
    Close,
    /// Sofort neu starten
    Restart,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorShape {
    #[default]
    Block,
    /// senkrechter Strich
    Ibeam,
    Underline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorBlink {
    /// wie in den GNOME-Einstellungen (Barrierefreiheit › Cursor blinkt)
    #[default]
    System,
    On,
    Off,
}

/// Was beim Steuerzeichen BEL (`\a`) passiert.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Bell {
    /// Systemton und Reiter markieren
    #[default]
    Sound,
    /// nur den Reiter markieren
    Mark,
    None,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    /// Id der Palette (Dateiname ohne Endung)
    pub palette: String,
    /// Statt der Shell ausführen (wird wie in einer Shell in Wörter zerlegt); leer = Shell
    pub command: String,
    /// Shell als Login-Shell starten (`-l`)
    pub login_shell: bool,
    /// Festes Startverzeichnis; leer = Einstellung „Verzeichnis neuer Terminals“
    pub directory: String,
    /// Zusätzliche Umgebungsvariablen, je `NAME=wert`
    pub environment: Vec<String>,
    pub exit_action: ExitAction,
    /// Pango-Schriftbeschreibung, z. B. „JetBrains Mono 11“; leer = System-Monospace
    pub font: String,
    pub cursor_shape: CursorShape,
    pub cursor_blink: CursorBlink,
    pub bell: Bell,
    /// Deckkraft des Hintergrunds, 1.0 = undurchsichtig (siehe [`Profile::opacity`])
    pub opacity: f64,
}

/// Kleinste erlaubte Deckkraft: darunter ist Text auf unruhigem Hintergrund kaum lesbar.
pub const MIN_OPACITY: f64 = 0.3;

impl Default for Profile {
    fn default() -> Self {
        Profile {
            id: String::new(),
            name: "Standard".into(),
            palette: DEFAULT_PALETTE.into(),
            command: String::new(),
            login_shell: false,
            directory: String::new(),
            environment: Vec::new(),
            exit_action: ExitAction::Close,
            font: String::new(),
            cursor_shape: CursorShape::Block,
            cursor_blink: CursorBlink::System,
            bell: Bell::Sound,
            opacity: 1.0,
        }
    }
}

impl Profile {
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("Profil lässt sich immer serialisieren")
    }

    pub fn from_toml(id: &str, text: &str) -> Result<Profile, toml::de::Error> {
        let mut p: Profile = toml::from_str(text)?;
        p.id = id.to_string();
        Ok(p)
    }

    /// Deckkraft im erlaubten Bereich (Handbearbeitung kann alles hineinschreiben).
    pub fn opacity(&self) -> f64 {
        if self.opacity.is_finite() {
            self.opacity.clamp(MIN_OPACITY, 1.0)
        } else {
            1.0
        }
    }

    /// Umgebung als eine Zeile für das Eingabefeld, so gequotet, dass ein Shell-Parser
    /// (`g_shell_parse_argv`) dieselben Einträge zurückliefert: `A=1 B='x y'` (R-03).
    pub fn environment_line(&self) -> String {
        self.environment.iter().map(|e| shell_quote_entry(e)).collect::<Vec<_>>().join(" ")
    }

    /// Nur gültige `NAME=wert`-Einträge.
    pub fn valid_environment(&self) -> Vec<String> {
        self.environment
            .iter()
            .filter(|e| e.split_once('=').is_some_and(|(k, _)| !k.is_empty() && !k.contains(char::is_whitespace)))
            .cloned()
            .collect()
    }
}

/// `NAME=wert` unverändert, wenn nichts zu quoten ist; sonst `NAME='wert'` (ein `'` im Wert
/// wird zu `'\''`). Ohne `=` wird der ganze Eintrag gequotet.
fn shell_quote_entry(entry: &str) -> String {
    let plain = |s: &str| s.chars().all(|c| c.is_alphanumeric() || "_-+=.,:/@%~^".contains(c));
    if plain(entry) {
        return entry.to_string();
    }
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    match entry.split_once('=') {
        Some((name, value)) if plain(name) && !name.is_empty() => format!("{name}={}", quote(value)),
        _ => quote(entry),
    }
}

/// Profile in einem Verzeichnis (eine Datei je Profil).
pub struct ProfileStore {
    dir: PathBuf,
}

impl ProfileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        ProfileStore { dir: dir.into() }
    }

    /// Alle Profile, nach Name sortiert. Legt „Standard“ an, wenn keines existiert.
    /// Unlesbare Dateien werden übersprungen und gemeldet, nicht gelöscht.
    pub fn load_all(&self) -> io::Result<(Vec<Profile>, Vec<String>)> {
        std::fs::create_dir_all(&self.dir)?;
        let mut profiles = Vec::new();
        let mut errors = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "toml") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()).map(String::from) else { continue };
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| Profile::from_toml(&id, &t).map_err(|e| e.to_string()))
            {
                Ok(p) => profiles.push(p),
                Err(e) => errors.push(format!("{}:\n{e}", path.display())),
            }
        }
        if profiles.is_empty() && errors.is_empty() {
            let p = Profile { id: "standard".into(), ..Profile::default() };
            self.save(&p)?;
            profiles.push(p);
        } else if profiles.is_empty() {
            // Nur kaputte Dateien: Standard im Speicher, nichts überschreiben. Die Id ist frei,
            // damit ein späteres Speichern die kaputte Datei nicht ersetzt.
            profiles.push(Profile { id: self.new_id("Standard"), ..Profile::default() });
        }
        profiles.sort_by_key(|p| p.name.to_lowercase());
        Ok((profiles, errors))
    }

    /// Schreibt atomar (temporäre Datei + Umbenennen).
    pub fn save(&self, p: &Profile) -> io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let target = self.dir.join(format!("{}.toml", p.id));
        let tmp = self.dir.join(format!(".{}.toml.tmp", p.id));
        crate::paths::write_private(&tmp, &target, &p.to_toml())
    }

    pub fn delete(&self, id: &str) -> io::Result<()> {
        std::fs::remove_file(self.dir.join(format!("{id}.toml")))
    }

    /// Freie Id aus dem Namen („Server (bender)“ → „server-bender“, sonst „-2“ usw.).
    pub fn new_id(&self, name: &str) -> String {
        let base = crate::palette::slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.dir.join(format!("{id}.toml")).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn environment_line_quotes_only_when_needed() {
        let p = Profile {
            environment: vec![
                "EDITOR=vim".into(),
                "JAVA_OPTS=-Xmx1g -Dx=1".into(),
                "GRUSS=it's".into(),
                "LEER=".into(),
                "PFAD=/opt/a b:$HOME".into(),
            ],
            ..Profile::default()
        };
        assert_eq!(
            p.environment_line(),
            r#"EDITOR=vim JAVA_OPTS='-Xmx1g -Dx=1' GRUSS='it'\''s' LEER= PFAD='/opt/a b:$HOME'"#
        );
    }

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("parkett-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn toml_roundtrip_and_defaults() {
        let p = Profile {
            id: "server".into(),
            name: "Server (bender)".into(),
            palette: "nord".into(),
            command: "ssh bender".into(),
            environment: vec!["LANG=de_DE.UTF-8".into()],
            exit_action: ExitAction::Restart,
            ..Profile::default()
        };
        let text = p.to_toml();
        assert!(text.contains("exit_action = \"restart\""), "{text}");
        assert_eq!(Profile::from_toml("server", &text).unwrap(), p);
        // fehlende Felder bekommen Standardwerte, unbekannte stören nicht
        let minimal = Profile::from_toml("x", "name = \"Nur Name\"\nzukunft = 1\n").unwrap();
        assert_eq!(minimal.palette, DEFAULT_PALETTE);
        assert_eq!(minimal.exit_action, ExitAction::Close, "QA-11");
    }

    #[test]
    fn environment_filter() {
        let p = Profile {
            environment: vec!["A=1".into(), "kaputt".into(), "=x".into(), "B C=2".into(), "EMPTY=".into()],
            ..Profile::default()
        };
        assert_eq!(p.valid_environment(), vec!["A=1".to_string(), "EMPTY=".to_string()]);
    }

    #[test]
    fn store_creates_default_saves_and_deletes() {
        let dir = temp_dir("store");
        let store = ProfileStore::new(&dir);
        let (all, errors) = store.load_all().unwrap();
        assert!(errors.is_empty());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "standard");

        let id = store.new_id("Server (bender)");
        assert_eq!(id, "server-bender");
        store.save(&Profile { id: id.clone(), name: "Server (bender)".into(), ..Profile::default() }).unwrap();
        assert_eq!(store.new_id("Server (bender)"), "server-bender-2");

        std::fs::write(dir.join("kaputt.toml"), "name = [").unwrap();
        let (all, errors) = store.load_all().unwrap();
        assert_eq!(all.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["Server (bender)", "Standard"]);
        assert_eq!(errors.len(), 1, "kaputte Datei wird gemeldet, nicht gelöscht");
        assert!(dir.join("kaputt.toml").exists());

        store.delete(&id).unwrap();
        assert_eq!(store.load_all().unwrap().0.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_broken_files_give_unsaved_default() {
        let dir = std::env::temp_dir().join(format!("parkett-broken-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("standard.toml"), "name = ").unwrap();
        let store = ProfileStore::new(dir.clone());
        let (profiles, errors) = store.load_all().unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(profiles.len(), 1);
        assert_ne!(profiles[0].id, "standard");
        assert_eq!(std::fs::read_to_string(dir.join("standard.toml")).unwrap(), "name = ");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn old_files_get_defaults_for_new_fields() {
        let p = Profile::from_toml("alt", "name = \"Alt\"\npalette = \"nord\"\n").unwrap();
        assert_eq!(
            (p.font.as_str(), p.cursor_shape, p.cursor_blink, p.bell),
            ("", CursorShape::Block, CursorBlink::System, Bell::Sound)
        );
        let p = Profile::from_toml(
            "neu",
            "cursor_shape = \"ibeam\"\ncursor_blink = \"off\"\nbell = \"mark\"\nfont = \"Mono 12\"\n",
        )
        .unwrap();
        assert_eq!(
            (p.font.as_str(), p.cursor_shape, p.cursor_blink, p.bell),
            ("Mono 12", CursorShape::Ibeam, CursorBlink::Off, Bell::Mark)
        );
    }

    #[test]
    fn opacity_is_clamped() {
        let p = |o: f64| Profile { opacity: o, ..Profile::default() }.opacity();
        assert_eq!(Profile::default().opacity(), 1.0);
        assert_eq!(p(0.8), 0.8);
        assert_eq!(p(0.0), MIN_OPACITY);
        assert_eq!(p(7.0), 1.0);
        assert_eq!(p(f64::NAN), 1.0);
        assert_eq!(Profile::from_toml("x", "opacity = 0.85\n").unwrap().opacity(), 0.85);
    }
}
