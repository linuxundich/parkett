//! Kontext eines Bereichs für die Kontextfarbe (Vorschlag 1): läuft dort gerade eine
//! Verbindung zu einem anderen Rechner oder ein Programm mit Root-Rechten?
//! Die Erkennung arbeitet auf Prozessname, Befehlszeile und effektiver UID des
//! Vordergrundprozesses (gelesen aus /proc durch die App).

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Context {
    #[default]
    Local,
    Superuser,
    /// Hostname (ohne Benutzer), soweit aus der Befehlszeile erkennbar
    Remote(String),
}

const REMOTE: &[&str] = &["ssh", "mosh", "mosh-client", "autossh", "et", "telnet"];
const SUPERUSER: &[&str] = &["sudo", "su", "doas", "run0", "pkexec"];

/// Optionen von ssh, die einen Wert erwarten (`man ssh`).
const SSH_ARG_OPTS: &[&str] = &["-b", "-B", "-c", "-D", "-E", "-e", "-F", "-I", "-i", "-J", "-L", "-l", "-m", "-O", "-o", "-P", "-p", "-Q", "-R", "-S", "-W", "-w"];

/// `comm`: Prozessname, `args`: Befehlszeile, `euid`: effektive UID des Vordergrundprozesses.
pub fn classify(comm: &str, args: &[String], euid: Option<u32>) -> Context {
    let name = comm.rsplit('/').next().unwrap_or(comm);
    if REMOTE.contains(&name) {
        return Context::Remote(remote_host(args).unwrap_or_else(|| name.to_string()));
    }
    if euid == Some(0) || SUPERUSER.contains(&name) {
        return Context::Superuser;
    }
    Context::Local
}

/// Zielrechner aus einer ssh-/mosh-Befehlszeile: erstes Argument, das keine Option und kein
/// Optionswert ist; „user@host“ → „host“, „ssh://user@host:22“ → „host“.
pub fn remote_host(args: &[String]) -> Option<String> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--" {
            return it.next().map(|h| clean_host(h));
        }
        if a.starts_with('-') {
            // „-p22“ trägt den Wert schon, „-p 22“ nicht
            if SSH_ARG_OPTS.contains(&a.as_str()) {
                it.next();
            }
            continue;
        }
        return Some(clean_host(a));
    }
    None
}

fn clean_host(target: &str) -> String {
    let t = target.strip_prefix("ssh://").unwrap_or(target);
    let t = t.rsplit('@').next().unwrap_or(t);
    let t = t.split(':').next().unwrap_or(t);
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn ssh_hosts() {
        assert_eq!(remote_host(&v("ssh bender")), Some("bender".into()));
        assert_eq!(remote_host(&v("ssh -p 2223 -i ~/.ssh/key testt@localhost")), Some("localhost".into()));
        assert_eq!(remote_host(&v("ssh -A -o ProxyJump=x toff@bender.fritz.box ls")), Some("bender.fritz.box".into()));
        assert_eq!(remote_host(&v("ssh -p22 ssh://root@nas:2222")), Some("nas".into()));
        assert_eq!(remote_host(&v("ssh -- host")), Some("host".into()));
        assert_eq!(remote_host(&v("ssh -v")), None);
    }

    #[test]
    fn classify_processes() {
        assert_eq!(classify("ssh", &v("ssh bender"), Some(1000)), Context::Remote("bender".into()));
        assert_eq!(classify("mosh-client", &v("mosh-client"), Some(1000)), Context::Remote("mosh-client".into()));
        assert_eq!(classify("sudo", &v("sudo vim /etc/fstab"), Some(0)), Context::Superuser);
        assert_eq!(classify("bash", &v("bash"), Some(0)), Context::Superuser, "Root-Shell nach sudo -i");
        assert_eq!(classify("run0", &v("run0"), Some(1000)), Context::Superuser);
        assert_eq!(classify("vim", &v("vim"), Some(1000)), Context::Local);
        assert_eq!(classify("bash", &v("bash"), None), Context::Local);
    }
}
