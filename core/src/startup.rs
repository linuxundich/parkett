// SPDX-License-Identifier: GPL-3.0-or-later
//! When a startup command (template) may be typed (R-09).
//!
//! Typed right after start, it ended up in the password prompt of an ssh profile.
//! With shell integration (vte.sh) Parkett waits for the first prompt (`vte.shell.precmd`);
//! without it, until the output is quiet and the terminal is not waiting for a password.

use std::time::Duration;

/// No earlier than this after start
pub const MIN_AFTER_SPAWN: Duration = Duration::from_millis(1000);
/// The output must be quiet this long (prompt fully drawn)
pub const QUIET: Duration = Duration::from_millis(400);
/// After that Parkett gives up (e.g. password never entered)
pub const GIVE_UP: Duration = Duration::from_secs(300);

/// PTY settings (termios) at the time of the check.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TtyMode {
    /// line-by-line input (ICANON)
    pub canonical: bool,
    /// characters are echoed (ECHO)
    pub echo: bool,
}

impl TtyMode {
    /// Password prompt: ssh, sudo and `read -s` read line by line without echo. A shell with
    /// readline has no echo either, but reads character by character.
    pub fn is_password_prompt(&self) -> bool {
        self.canonical && !self.echo
    }
}

/// May the startup command be typed now without shell integration?
pub fn may_type(since_spawn: Duration, since_output: Duration, mode: Option<TtyMode>) -> bool {
    since_spawn >= MIN_AFTER_SPAWN && since_output >= QUIET && !mode.is_some_and(|m| m.is_password_prompt())
}

#[cfg(test)]
mod tests {
    use super::*;

    const READLINE: TtyMode = TtyMode { canonical: false, echo: false };
    const PASSWORD: TtyMode = TtyMode { canonical: true, echo: false };
    const COOKED: TtyMode = TtyMode { canonical: true, echo: true };
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn waits_for_quiet_prompt() {
        assert!(!may_type(ms(300), ms(300), Some(READLINE)), "zu früh nach dem Start");
        assert!(!may_type(ms(2000), ms(100), Some(READLINE)), "Ausgabe läuft noch");
        assert!(may_type(ms(2000), ms(500), Some(READLINE)));
        assert!(may_type(ms(2000), ms(500), Some(COOKED)), "sh ohne readline");
        assert!(may_type(ms(2000), ms(500), None), "Einstellung nicht lesbar");
    }

    #[test]
    fn never_into_a_password_prompt() {
        assert!(PASSWORD.is_password_prompt());
        assert!(!READLINE.is_password_prompt());
        assert!(!may_type(ms(60_000), ms(60_000), Some(PASSWORD)), "R-09");
    }
}
