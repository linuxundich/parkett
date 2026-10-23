// SPDX-License-Identifier: GPL-3.0-or-later
//! Befehlszeile: `parkett [OPTIONEN] [-- BEFEHL …]`. Eigener kleiner Parser statt GOptions,
//! weil alles nach `-e`/`-x`/`--` unverändert der Befehl ist (wie bei gnome-terminal/Ptyxis)
//! und die Argumente einer zweiten Instanz an die laufende weitergereicht werden.

use crate::i18n::{gettext, trf, N_};

/// Hilfetext für `--help` (englischer msgid, übersetzt mit `help()`).
pub const HELP: &str = N_("Usage: parkett [OPTIONS] [-- COMMAND [ARGUMENTS …]]\n\n  -d, --working-directory=DIR   Start directory; repeat for one tab per directory\n  -e, -x, --                    Run the rest of the line instead of the shell\n                                (-e with a single argument: split like a shell)\n  -p, --profile=ID              Profile (file name without .toml)\n  -t, --title=TITLE             Tab title\n      --tab                     Open as a tab in the active window instead of a new window\n      --split-right             Split the active pane of the active window to the right\n      --split-down              Split the active pane of the active window downwards\n  -h, --help                    Show this help\n      --version                 Show version\n");

/// Übersetzter Hilfetext.
pub fn help() -> String {
    gettext(HELP)
}

/// `--split-right`/`--split-down`: statt eines Reiters den aktiven Bereich teilen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Split {
    Right,
    Down,
}

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
    pub split: Option<Split>,
    pub help: bool,
    pub version: bool,
}

impl Cli {
    /// Gab es irgendeine Vorgabe? Ohne Vorgabe darf die letzte Sitzung zurückkommen.
    pub fn is_plain(&self) -> bool {
        self.working_directories.is_empty()
            && self.command.is_none()
            && self.profile.is_none()
            && self.title.is_none()
            && self.split.is_none()
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
            args.get(i)
                .map(|v| v.as_ref().to_string())
                .ok_or_else(|| trf("{option} needs a value", &[("option", long)]))
        };
        // Schalter ohne Wert: „--tab=x“ ist ein Tippfehler, kein --tab (R-10)
        let flag = |set: &mut bool, v: bool| -> Result<(), String> {
            match inline {
                Some(_) => Err(trf("{option} takes no value (“{argument}”)", &[("option", name), ("argument", arg)])),
                None => {
                    *set = v;
                    Ok(())
                }
            }
        };
        match name {
            "-e" | "-x" | "--" | "--command" => {
                if inline.is_some() {
                    return Err(trf("Unknown option “{argument}” (see parkett --help)", &[("argument", arg)]));
                }
                let rest: Vec<String> = args[i + 1..].iter().map(|a| a.as_ref().to_string()).collect();
                if rest.is_empty() {
                    return Err(trf("{argument}: no command given", &[("argument", arg)]));
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
            "--split-right" | "--split-down" => {
                let mut on = false;
                flag(&mut on, true)?;
                cli.split = Some(if name == "--split-right" { Split::Right } else { Split::Down });
            }
            "-h" | "--help" => flag(&mut cli.help, true)?,
            "--version" => flag(&mut cli.version, true)?,
            _ => return Err(trf("Unknown option “{argument}” (see parkett --help)", &[("argument", arg)])),
        }
        i += 1;
    }
    Ok(cli)
}

/// Schützt das Trennzeichen `--` vor GLib: `g_option_context_parse` (GApplication) entfernt
/// ein `--`, wenn kein Argument danach mit „-“ beginnt. `parkett -- htop` kam dann als
/// `parkett htop` an und scheiterte mit „Unbekannte Option“ – aber nur beim ersten Start,
/// nicht bei einer laufenden Instanz. Das erste echte `--` (nicht der Wert von `-d`/`-p`/`-t`)
/// wird darum zum gleichbedeutenden `--command`, das GLib stehen lässt. `args[0]` ist das
/// Programm.
pub fn protect_separator(mut args: Vec<String>) -> Vec<String> {
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--" => {
                args[i] = "--command".into();
                break;
            }
            "-e" | "-x" | "--command" => break,
            "-d" | "--working-directory" | "-p" | "--profile" | "-t" | "--title" => i += 2,
            _ => i += 1,
        }
    }
    args
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
    fn separator_survives_glib() {
        let v = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(protect_separator(v(&["parkett", "--", "htop"])), v(&["parkett", "--command", "htop"]));
        // nur das erste, danach gehört alles zum Befehl
        assert_eq!(
            protect_separator(v(&["parkett", "-d", "/tmp", "--", "sh", "--", "x"])),
            v(&["parkett", "-d", "/tmp", "--command", "sh", "--", "x"])
        );
        // „--“ als Wert einer Option bleibt, ebenso alles nach -e
        assert_eq!(protect_separator(v(&["parkett", "-t", "--"])), v(&["parkett", "-t", "--"]));
        assert_eq!(protect_separator(v(&["parkett", "-e", "a", "--"])), v(&["parkett", "-e", "a", "--"]));
        // was herauskommt, versteht parse wie vorher „--“
        let cli = parse(&protect_separator(v(&["parkett", "--", "htop"]))[1..]).unwrap();
        assert_eq!(cli.command, Some(vec!["htop".into()]));
    }

    #[test]
    fn errors() {
        assert!(parse(&["--"]).is_err());
        assert!(parse(&["-d"]).is_err());
        assert_eq!(parse(&["--gibtsnicht"]), Err("Unknown option “--gibtsnicht” (see parkett --help)".to_string()));
        assert!(parse(&["--tab=x"]).is_err(), "R-10");
        assert!(parse(&["--help=1"]).is_err());
        assert!(parse(&["-e=htop"]).is_err());
    }

    #[test]
    fn split() {
        let cli = parse(&["--split-right", "-d", "/tmp"]).unwrap();
        assert_eq!(cli.split, Some(Split::Right));
        assert!(!cli.is_plain());
        assert_eq!(parse(&["--split-down", "--", "htop"]).unwrap().split, Some(Split::Down));
        assert!(parse(&["--split-down=1"]).is_err());
    }

    #[test]
    fn several_directories() {
        let cli = parse(&["-d", "/a", "--working-directory=/b", "-d", "/c"]).unwrap();
        assert_eq!(cli.working_directories, ["/a", "/b", "/c"]);
    }
}
