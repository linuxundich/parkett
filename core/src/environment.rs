// SPDX-License-Identifier: GPL-3.0-or-later
//! Umgebung für Shells, die ein zweiter Aufruf öffnet (`FOO=1 parkett --tab`, QA-08).
//!
//! Eine laufende Instanz bekommt die Umgebung des Aufrufers über D-Bus mit. Die neue Shell
//! soll sie haben (venv, `SSH_AUTH_SOCK`, eigenes `PATH`), wie bei GNOME Terminal und Ptyxis –
//! aber ohne Variablen, die zum Terminal oder Start des Aufrufers gehören.

/// Einzelne Variablen des Aufrufers, die nicht weitergegeben werden.
const DROP: &[&str] = &[
    // setzt VTE selbst für dieses Terminal
    "TERM",
    "COLORTERM",
    "VTE_VERSION",
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "WINDOWID",
    // Shell- und Prozesszustand des Aufrufers
    "PWD",
    "OLDPWD",
    "SHLVL",
    "_",
    // Start des Aufrufers (Desktop-Datei, Aktivierung, systemd-Unit)
    "GIO_LAUNCHED_DESKTOP_FILE",
    "GIO_LAUNCHED_DESKTOP_FILE_PID",
    "DESKTOP_STARTUP_ID",
    "XDG_ACTIVATION_TOKEN",
    "INVOCATION_ID",
    "JOURNAL_STREAM",
    "SYSTEMD_EXEC_PID",
    "MANAGERPID",
];

/// Sitzung, in der das Fenster liegt: immer von Parkett, nicht vom Aufrufer. Kommt der
/// Aufruf aus ssh, cron oder einer systemd-Unit, fehlen sie dort, und grafische Programme
/// aus dem neuen Reiter starteten nicht.
pub const SESSION: &[&str] = &[
    "WAYLAND_DISPLAY",
    "DISPLAY",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "XDG_SESSION_DESKTOP",
    "DESKTOP_SESSION",
];

/// Präfixe von Variablen anderer Terminals, aus denen der Aufruf kommen kann.
const DROP_PREFIX: &[&str] =
    &["GNOME_TERMINAL_", "PTYXIS_", "KONSOLE_", "TILIX_", "KITTY_", "ALACRITTY_", "WEZTERM_", "GHOSTTY_"];

fn name(entry: &str) -> Option<&str> {
    entry.split_once('=').map(|(k, _)| k).filter(|k| !k.is_empty())
}

/// Umgebung des Aufrufers ohne die Variablen oben, dazu die Sitzung (`SESSION`) aus `own`,
/// der Umgebung von Parkett, danach die des Profils (`NAME=wert`; gleiche Namen ersetzen
/// die vorigen).
pub fn for_caller(caller: &[String], own: &[String], profile: &[String]) -> Vec<String> {
    let overridden = |k: &str| profile.iter().any(|p| name(p) == Some(k));
    let session = |k: &str| SESSION.contains(&k);
    let from_caller = caller.iter().filter(|e| {
        name(e).is_some_and(|k| {
            !DROP.contains(&k) && !DROP_PREFIX.iter().any(|p| k.starts_with(p)) && !session(k) && !overridden(k)
        })
    });
    let from_own = own.iter().filter(|e| name(e).is_some_and(|k| session(k) && !overridden(k)));
    from_caller.chain(from_own).chain(profile.iter()).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn caller_variables_reach_the_shell() {
        let env = for_caller(&v(&["QA_ENV=gesetzt", "PATH=/venv/bin:/usr/bin", "SSH_AUTH_SOCK=/run/a"]), &[], &[]);
        assert_eq!(env, v(&["QA_ENV=gesetzt", "PATH=/venv/bin:/usr/bin", "SSH_AUTH_SOCK=/run/a"]));
    }

    #[test]
    fn terminal_and_launch_variables_are_dropped() {
        let env = for_caller(
            &v(&[
                "TERM=xterm-kitty",
                "KITTY_WINDOW_ID=3",
                "GNOME_TERMINAL_SCREEN=/org/x",
                "SHLVL=2",
                "GIO_LAUNCHED_DESKTOP_FILE_PID=12",
                "INVOCATION_ID=abc",
                "HOME=/home/t",
                "kaputt",
                "=leer",
            ]),
            &[],
            &[],
        );
        assert_eq!(env, v(&["HOME=/home/t"]));
    }

    #[test]
    fn profile_wins() {
        let env = for_caller(&v(&["EDITOR=nano", "LANG=de_DE.UTF-8"]), &[], &v(&["EDITOR=vim"]));
        assert_eq!(env, v(&["LANG=de_DE.UTF-8", "EDITOR=vim"]));
    }

    #[test]
    fn session_comes_from_parkett() {
        // Aufruf aus ssh: kein Wayland, fremdes DISPLAY wird ersetzt
        let env = for_caller(
            &v(&["DISPLAY=localhost:10.0", "QA=1"]),
            &v(&["WAYLAND_DISPLAY=wayland-0", "DISPLAY=:0", "HOME=/home/parkett", "XDG_RUNTIME_DIR=/run/user/1000"]),
            &[],
        );
        assert_eq!(env, v(&["QA=1", "WAYLAND_DISPLAY=wayland-0", "DISPLAY=:0", "XDG_RUNTIME_DIR=/run/user/1000"]));
    }
}
