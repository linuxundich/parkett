// SPDX-License-Identifier: GPL-3.0-or-later
//! Befehlszeile: `parkett [OPTIONEN] [-- BEFEHL …]`. Eigener kleiner Parser statt GOptions,
//! weil alles nach `-e`/`-x`/`--` unverändert der Befehl ist (wie bei gnome-terminal/Ptyxis)
//! und die Argumente einer zweiten Instanz an die laufende weitergereicht werden.

pub const HELP: &str = "\
Aufruf: parkett [OPTIONEN] [-- BEFEHL [ARGUMENTE …]]

  -d, --working-directory=DIR   Startverzeichnis; mehrfach = ein Reiter je Verzeichnis
  -e, -x, --                    Rest der Zeile statt der Shell ausführen
  -p, --profile=ID              Profil (Dateiname ohne .toml)
  -t, --title=TITEL             Titel des Reiters
      --tab                     als Reiter im aktiven Fenster statt neuem Fenster
  -h, --help                    diese Hilfe
      --version                 Version
";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Cli {
    /// leer = Verzeichnis des Aufrufers; mehrere = ein Reiter je Verzeichnis
    pub working_directories: Vec<String>,
    pub command: Option<Vec<String>>,
    pub profile: Option<String>,
    pub title: Option<String>,
    pub tab: bool,
    pub help: bool,
    pub version: bool,
}

impl Cli {
    /// Gab es irgendeine Vorgabe? Ohne Vorgabe darf die letzte Sitzung zurückkommen.
    pub fn is_plain(&self) -> bool {
        self.working_directories.is_empty() && self.command.is_none() && self.profile.is_none() && self.title.is_none()
    }
}

/// `args` ohne den Programmnamen.
pub fn parse<S: AsRef<str>>(args: &[S]) -> Result<Cli, String> {
    let mut cli = Cli::default();
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_ref();
        // Wert als „--opt=wert“ oder als nächstes Argument
        let mut value = |name: &str| -> Result<String, String> {
            if let Some(v) = arg.split_once('=').map(|(_, v)| v) {
                return Ok(v.to_string());
            }
            i += 1;
            args.get(i).map(|v| v.as_ref().to_string()).ok_or(format!("{name} braucht einen Wert"))
        };
        match arg.split('=').next().unwrap_or(arg) {
            "-e" | "-x" | "--" | "--command" => {
                let rest: Vec<String> = args[i + 1..].iter().map(|a| a.as_ref().to_string()).collect();
                if rest.is_empty() {
                    return Err(format!("{arg}: kein Befehl angegeben"));
                }
                cli.command = Some(rest);
                return Ok(cli);
            }
            "-d" | "--working-directory" => cli.working_directories.push(value("--working-directory")?),
            "-p" | "--profile" => cli.profile = Some(value("--profile")?),
            "-t" | "--title" => cli.title = Some(value("--title")?),
            "--tab" => cli.tab = true,
            "--new-window" => cli.tab = false,
            "-h" | "--help" => cli.help = true,
            "--version" => cli.version = true,
            _ => return Err(format!("Unbekannte Option „{arg}“ (siehe parkett --help)")),
        }
        i += 1;
    }
    Ok(cli)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain() {
        let cli = parse::<&str>(&[]).unwrap();
        assert!(cli.is_plain());
        assert!(!cli.tab);
    }

    #[test]
    fn command_takes_the_rest() {
        let cli = parse(&["-d", "/tmp", "--", "htop", "-d", "5"]).unwrap();
        assert_eq!(cli.working_directories, ["/tmp"]);
        assert_eq!(cli.command, Some(vec!["htop".into(), "-d".into(), "5".into()]));
        let cli = parse(&["-e", "ssh", "bender"]).unwrap();
        assert_eq!(cli.command, Some(vec!["ssh".into(), "bender".into()]));
    }

    #[test]
    fn equals_and_separate_values() {
        let cli = parse(&["--working-directory=/a b", "--profile", "server", "-t=Logs", "--tab"]).unwrap();
        assert_eq!(cli.working_directories, ["/a b"]);
        assert_eq!(cli.profile.as_deref(), Some("server"));
        assert_eq!(cli.title.as_deref(), Some("Logs"));
        assert!(cli.tab);
        assert!(!cli.is_plain());
    }

    #[test]
    fn errors() {
        assert!(parse(&["--"]).is_err());
        assert!(parse(&["-d"]).is_err());
        assert!(parse(&["--gibtsnicht"]).is_err());
    }

    #[test]
    fn several_directories() {
        let cli = parse(&["-d", "/a", "--working-directory=/b", "-d", "/c"]).unwrap();
        assert_eq!(cli.working_directories, ["/a", "/b", "/c"]);
    }
}
