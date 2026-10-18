//! Profile: wie ein Terminal startet und aussieht. Gespeichert als TOML-Dateien
//! (`$XDG_CONFIG_HOME/parkett/profiles/<id>.toml`), damit man sie von Hand bearbeiten
//! und in Dotfiles mitnehmen kann.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::palette::DEFAULT_PALETTE;

/// Was passiert, wenn der Prozess im Bereich endet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExitAction {
    /// Leiste „Prozess beendet“ mit Neu starten / Schließen
    #[default]
    Hold,
    /// Bereich schließen, wenn der Prozess ohne Fehler endet; bei Fehler Leiste
    Close,
    /// Sofort neu starten
    Restart,
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
}

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
            exit_action: ExitAction::Hold,
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

    /// Nur gültige `NAME=wert`-Einträge.
    pub fn valid_environment(&self) -> Vec<String> {
        self.environment
            .iter()
            .filter(|e| e.split_once('=').is_some_and(|(k, _)| !k.is_empty() && !k.contains(char::is_whitespace)))
            .cloned()
            .collect()
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

    pub fn dir(&self) -> &Path {
        &self.dir
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
            match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| Profile::from_toml(&id, &t).map_err(|e| e.to_string())) {
                Ok(p) => profiles.push(p),
                Err(e) => errors.push(format!("{}: {e}", path.display())),
            }
        }
        if profiles.is_empty() && errors.is_empty() {
            let p = Profile { id: "standard".into(), ..Profile::default() };
            self.save(&p)?;
            profiles.push(p);
        }
        profiles.sort_by_key(|p| p.name.to_lowercase());
        Ok((profiles, errors))
    }

    /// Schreibt atomar (temporäre Datei + Umbenennen).
    pub fn save(&self, p: &Profile) -> io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let target = self.dir.join(format!("{}.toml", p.id));
        let tmp = self.dir.join(format!(".{}.toml.tmp", p.id));
        std::fs::write(&tmp, p.to_toml())?;
        std::fs::rename(tmp, target)
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
            exit_action: ExitAction::Close,
            ..Profile::default()
        };
        let text = p.to_toml();
        assert!(text.contains("exit_action = \"close\""), "{text}");
        assert_eq!(Profile::from_toml("server", &text).unwrap(), p);
        // fehlende Felder bekommen Standardwerte, unbekannte stören nicht
        let minimal = Profile::from_toml("x", "name = \"Nur Name\"\nzukunft = 1\n").unwrap();
        assert_eq!(minimal.palette, DEFAULT_PALETTE);
        assert_eq!(minimal.exit_action, ExitAction::Hold);
    }

    #[test]
    fn environment_filter() {
        let p = Profile { environment: vec!["A=1".into(), "kaputt".into(), "=x".into(), "B C=2".into(), "EMPTY=".into()], ..Profile::default() };
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
}
