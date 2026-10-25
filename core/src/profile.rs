// SPDX-License-Identifier: GPL-3.0-or-later
//! Profiles: how a terminal starts and looks. Stored as TOML files
//! (`$XDG_CONFIG_HOME/parkett/profiles/<id>.toml`), so they can be edited by hand
//! and carried along in dotfiles.

use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::palette::DEFAULT_PALETTE;

/// What happens when the process in the pane exits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExitAction {
    /// "Process exited" bar with Restart / Close
    Hold,
    /// Close the pane if the process exits without error; on error show the bar. Default as
    /// in GNOME Console, Ptyxis and Tilix: `exit` closes (QA-11)
    #[default]
    Close,
    /// Restart immediately
    Restart,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorShape {
    #[default]
    Block,
    /// vertical bar
    Ibeam,
    Underline,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorBlink {
    /// as in GNOME Settings (Accessibility › Cursor Blinking)
    #[default]
    System,
    On,
    Off,
}

/// What happens on the control character BEL (`\a`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Bell {
    /// system sound and mark the tab
    #[default]
    Sound,
    /// only mark the tab
    Mark,
    None,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    #[serde(skip)]
    pub id: String,
    pub name: String,
    /// palette id (file name without extension)
    pub palette: String,
    /// Run instead of the shell (split into words like in a shell); empty = shell
    pub command: String,
    /// start the shell as a login shell (`-l`)
    pub login_shell: bool,
    /// Fixed start directory; empty = setting "Directory for new terminals"
    pub directory: String,
    /// Additional environment variables, each `NAME=value`
    pub environment: Vec<String>,
    pub exit_action: ExitAction,
    /// Pango font description, e.g. "JetBrains Mono 11"; empty = system monospace
    pub font: String,
    pub cursor_shape: CursorShape,
    pub cursor_blink: CursorBlink,
    pub bell: Bell,
    /// background opacity, 1.0 = opaque (see [`Profile::opacity`])
    pub opacity: f64,
    /// Automatic for these hosts (SSH), `*` and `?` as wildcards: panes then show
    /// this profile's look while the connection lasts
    pub hosts: Vec<String>,
    /// Automatic for these folders (local, `~` allowed; `~/kunde/*` also applies below)
    pub directories: Vec<String>,
    /// Template for short tab names (see `label::render`); empty = setting
    pub tab_label: String,
    /// Show the scrollbar (overlay as in GNOME); off: scrolling still works
    pub scrollbar: bool,
    /// Draw bold text in the bright colors 8–15 (old terminals)
    pub bold_is_bright: bool,
    /// line and character spacing, 1.0 = normal (see [`Profile::spacing`])
    pub line_spacing: f64,
    pub letter_spacing: f64,
    /// Scroll to the end on new output
    pub scroll_on_output: bool,
    /// Scroll to the end on a keystroke
    pub scroll_on_keystroke: bool,
    /// Custom links (regex → address or command), see `link`
    pub links: Vec<crate::link::CustomLink>,
}

/// Smallest allowed opacity: below it, text on a busy background is hard to read.
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
            hosts: Vec::new(),
            directories: Vec::new(),
            tab_label: String::new(),
            scrollbar: true,
            bold_is_bright: false,
            line_spacing: 1.0,
            letter_spacing: 1.0,
            scroll_on_output: false,
            scroll_on_keystroke: true,
            links: Vec::new(),
        }
    }
}

/// Wildcards `*` (any number of characters) and `?` (one), case-insensitive.
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// Profile whose look a pane gets automatically: first by host (for an
/// SSH connection), otherwise by local folder. Folder patterns also apply to subfolders;
/// `~` stands for `home`. With several matches the first profile in `profiles` wins.
pub fn automatic<'a>(
    profiles: &'a [Profile],
    host: Option<&str>,
    cwd: Option<&str>,
    home: &str,
) -> Option<&'a Profile> {
    if let Some(host) = host.filter(|h| !h.is_empty()) {
        // compare "user@host" and "host:port" from the command line only by hostname
        let host = host.rsplit('@').next().unwrap_or(host);
        let host = host.split(':').next().unwrap_or(host);
        return profiles.iter().find(|p| p.hosts.iter().any(|pat| wildcard_match(pat.trim(), host)));
    }
    let cwd = cwd?.trim_end_matches('/');
    profiles.iter().find(|p| {
        p.directories.iter().any(|pat| {
            let pat = pat.trim();
            let pat = match pat.strip_prefix('~') {
                Some(rest) => format!("{home}{rest}"),
                None => pat.to_string(),
            };
            let pat = pat.trim_end_matches('/');
            !pat.is_empty() && (wildcard_match(pat, cwd) || wildcard_match(&format!("{pat}/*"), cwd))
        })
    })
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

    /// Spacing within the allowed range 1.0–2.0 (VTE: cell height or width as a multiple).
    pub fn spacing(v: f64) -> f64 {
        if v.is_finite() {
            v.clamp(1.0, 2.0)
        } else {
            1.0
        }
    }

    /// Opacity within the allowed range (hand editing can write anything into it).
    pub fn opacity(&self) -> f64 {
        if self.opacity.is_finite() {
            self.opacity.clamp(MIN_OPACITY, 1.0)
        } else {
            1.0
        }
    }

    /// Environment as one line for the entry field, quoted so that a shell parser
    /// (`g_shell_parse_argv`) returns the same entries: `A=1 B='x y'` (R-03).
    pub fn environment_line(&self) -> String {
        self.environment.iter().map(|e| shell_quote_entry(e)).collect::<Vec<_>>().join(" ")
    }

    /// Only valid `NAME=value` entries.
    pub fn valid_environment(&self) -> Vec<String> {
        self.environment
            .iter()
            .filter(|e| e.split_once('=').is_some_and(|(k, _)| !k.is_empty() && !k.contains(char::is_whitespace)))
            .cloned()
            .collect()
    }
}

/// `NAME=value` unchanged if there is nothing to quote; otherwise `NAME='value'` (a `'` in the
/// value becomes `'\''`). Without `=` the whole entry is quoted.
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

/// Profiles in a directory (one file per profile).
pub struct ProfileStore {
    dir: PathBuf,
}

impl ProfileStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        ProfileStore { dir: dir.into() }
    }

    /// All profiles, sorted by name. Creates "Standard" if none exists.
    /// Unreadable files are skipped and reported, not deleted.
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
            // only broken files: default in memory, overwrite nothing. The id is free,
            // so that a later save does not replace the broken file.
            profiles.push(Profile { id: self.new_id("Standard"), ..Profile::default() });
        }
        profiles.sort_by_key(|p| p.name.to_lowercase());
        Ok((profiles, errors))
    }

    /// Writes atomically (temporary file + rename).
    pub fn save(&self, p: &Profile) -> io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let target = self.dir.join(format!("{}.toml", p.id));
        let tmp = self.dir.join(format!(".{}.toml.tmp", p.id));
        crate::paths::write_private(&tmp, &target, &p.to_toml())
    }

    pub fn delete(&self, id: &str) -> io::Result<()> {
        std::fs::remove_file(self.dir.join(format!("{id}.toml")))
    }

    /// Free id from the name ("Server (bender)" → "server-bender", otherwise "-2" etc.).
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
        // missing fields get default values, unknown ones do no harm
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

    #[test]
    fn wildcards() {
        assert!(wildcard_match("bender", "bender"));
        assert!(wildcard_match("*.tuxedo.de", "build.tuxedo.de"));
        assert!(!wildcard_match("*.tuxedo.de", "tuxedo.de"));
        assert!(wildcard_match("web?", "web1"));
        assert!(wildcard_match("BENDER", "bender"));
        assert!(!wildcard_match("bend", "bender"));
        assert!(wildcard_match("*", ""));
    }

    #[test]
    fn automatic_profile_by_host_then_directory() {
        let p = |id: &str, hosts: &[&str], dirs: &[&str]| Profile {
            id: id.into(),
            hosts: hosts.iter().map(|s| s.to_string()).collect(),
            directories: dirs.iter().map(|s| s.to_string()).collect(),
            ..Profile::default()
        };
        let list = vec![p("server", &["bender", "*.tuxedo.de"], &[]), p("kunde", &[], &["~/kunde", "/srv/web*"])];
        let id = |host: Option<&str>, cwd: Option<&str>| {
            automatic(&list, host, cwd, "/home/t").map(|p| p.id.clone()).unwrap_or_default()
        };
        assert_eq!(id(Some("bender"), None), "server");
        assert_eq!(id(Some("toff@bender"), Some("/home/t/kunde")), "server", "Rechner vor Ordner");
        assert_eq!(id(Some("build.tuxedo.de:2222"), None), "server");
        assert_eq!(id(Some("anderer"), Some("/home/t/kunde")), "", "Ordner nur lokal");
        assert_eq!(id(None, Some("/home/t/kunde")), "kunde");
        assert_eq!(id(None, Some("/home/t/kunde/projekt/src")), "kunde", "auch darunter");
        assert_eq!(id(None, Some("/home/t/kundenliste")), "", "nur ganze Ordner");
        assert_eq!(id(None, Some("/srv/webseite")), "kunde");
        assert_eq!(id(None, None), "");
    }

    #[test]
    fn old_profiles_without_rules_still_load() {
        let p = Profile::from_toml("alt", "name = \"Alt\"\n").unwrap();
        assert!(p.hosts.is_empty() && p.directories.is_empty());
    }

    #[test]
    fn text_options_defaults_and_limits() {
        let p = Profile::from_toml("alt", "name = \"Alt\"\n").unwrap();
        assert!(!p.bold_is_bright && p.scroll_on_keystroke && !p.scroll_on_output);
        assert_eq!((p.line_spacing, p.letter_spacing), (1.0, 1.0));
        assert_eq!(Profile::spacing(0.5), 1.0);
        assert_eq!(Profile::spacing(1.3), 1.3);
        assert_eq!(Profile::spacing(9.0), 2.0);
        assert_eq!(Profile::spacing(f64::NAN), 1.0);
    }

    #[test]
    fn custom_links_round_trip() {
        let mut p = Profile { id: "x".into(), ..Default::default() };
        p.links
            .push(crate::link::CustomLink { pattern: r"\bPARK-(\d+)".into(), target: "https://t.example/$1".into() });
        let text = p.to_toml();
        assert!(text.contains("[[links]]"), "{text}");
        assert_eq!(Profile::from_toml("x", &text).unwrap().links, p.links);
    }
}
