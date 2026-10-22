// SPDX-License-Identifier: GPL-3.0-or-later
//! Wann ein Startbefehl (Vorlage) getippt werden darf (R-09).
//!
//! Sofort nach dem Start getippt, landete er bei einem ssh-Profil in der Passwortabfrage.
//! Mit Shell-Integration (vte.sh) wartet Parkett auf den ersten Prompt (`vte.shell.precmd`);
//! ohne sie, bis die Ausgabe ruht und das Terminal nicht auf ein Passwort wartet.

use std::time::Duration;

/// Frühestens so lange nach dem Start
pub const MIN_AFTER_SPAWN: Duration = Duration::from_millis(1000);
/// So lange muss die Ausgabe ruhen (Prompt fertig gezeichnet)
pub const QUIET: Duration = Duration::from_millis(400);
/// Danach gibt Parkett auf (z. B. Passwort nie eingegeben)
pub const GIVE_UP: Duration = Duration::from_secs(300);

/// Einstellung der PTY (termios) zum Zeitpunkt der Prüfung.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TtyMode {
    /// zeilenweise Eingabe (ICANON)
    pub canonical: bool,
    /// Zeichen werden angezeigt (ECHO)
    pub echo: bool,
}

impl TtyMode {
    /// Passwortabfrage: ssh, sudo und `read -s` lesen zeilenweise ohne Echo. Eine Shell mit
    /// readline hat auch kein Echo, liest aber zeichenweise.
    pub fn is_password_prompt(&self) -> bool {
        self.canonical && !self.echo
    }
}

/// Darf der Startbefehl jetzt ohne Shell-Integration getippt werden?
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
