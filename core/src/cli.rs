// SPDX-License-Identifier: GPL-3.0-or-later
//! Befehlszeile: `parkett [OPTIONEN] [-- BEFEHL …]`. Eigener kleiner Parser statt GOptions,
//! weil alles nach `-e`/`-x`/`--` unverändert der Befehl ist (wie bei gnome-terminal/Ptyxis)
//! und die Argumente einer zweiten Instanz an die laufende weitergereicht werden.

pub const HELP: &str = "\
Aufruf: parkett [OPTIONEN] [-- BEFEHL [ARGUMENTE …]]

  -d, --working-directory=DIR   Startverzeichnis; mehrfach = ein Reiter je Verzeichnis
  -e, -x, --                    Rest der Zeile statt der Shell ausführen
                                (-e mit nur einem Argument: wie in einer Shell zerlegt)
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
    /// `-e "htop -d 5"`: ein einziges Argument nach `-e` ist eine Befehlszeile (wie bei
    /// gnome-terminal) und muss noch wie in einer Shell zerlegt werden – das macht die App mit
    /// `glib::shell_parse_argv`, wie für den Befehl im Profil
    pub command_needs_split: bool,
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
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (arg, None),
        };
        // Wert als „--opt=wert“ oder als nächstes Argument
        let mut value = |long: &str| -> Result<String, String> {
            if let Some(v) = inline {
                return Ok(v.to_string());
            }
            i += 1;
            args.get(i).map(|v| v.as_ref().to_string()).ok_or(format!("{long} braucht einen Wert"))
        };
        // Schalter ohne Wert: „--tab=x“ ist ein Tippfehler, kein --tab (R-10)
        let flag = |set: &mut bool, v: bool| -> Result<(), String> {
            match inline {
                Some(_) => Err(format!("{name} nimmt keinen Wert („{arg}“)")),
                None => {
                    *set = v;
                    Ok(())
                }
            }
        };
        match name {
            "-e" | "-x" | "--" | "--command" => {
                if inline.is_some() {
                    return Err(format!("Unbekannte Option „{arg}“ (siehe parkett --help)"));
                }
                let rest: Vec<String> = args[i + 1..].iter().map(|a| a.as_ref().to_string()).collect();
                if rest.is_empty() {
                    return Err(format!("{arg}: kein Befehl angegeben"));
                }
                cli.command_needs_split = name == "-e" && rest.len() == 1;
                cli.command = Some(rest);
                return Ok(cli);
            }
            "-d" | "--working-directory" => cli.working_directories.push(value("--working-directory")?),
            "-p" | "--profile" => cli.profile = Some(value("--profile")?),
            "-t" | "--title" => cli.title = Some(value("--title")?),
            "--tab" => flag(&mut cli.tab, true)?,
            "--new-window" => flag(&mut cli.tab, false)?,
            "-h" | "--help" => flag(&mut cli.help, true)?,
            "--version" => flag(&mut cli.version, true)?,
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
        assert!(!cli.command_needs_split);
    }

    #[test]
    fn single_argument_after_e_is_a_command_line() {
        let cli = parse(&["-e", "htop -d 5"]).unwrap();
        assert_eq!(cli.command, Some(vec!["htop -d 5".into()]));
        assert!(cli.command_needs_split, "R-10");
        // nach -x und -- bleibt ein Argument ein Argument (Pfad mit Leerzeichen)
        assert!(!parse(&["--", "/opt/mein programm"]).unwrap().command_needs_split);
        assert!(!parse(&["-x", "htop -d 5"]).unwrap().command_needs_split);
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
        assert!(parse(&["--tab=x"]).is_err(), "R-10");
        assert!(parse(&["--help=1"]).is_err());
        assert!(parse(&["-e=htop"]).is_err());
    }

    #[test]
    fn several_directories() {
        let cli = parse(&["-d", "/a", "--working-directory=/b", "-d", "/c"]).unwrap();
        assert_eq!(cli.working_directories, ["/a", "/b", "/c"]);
    }
}
