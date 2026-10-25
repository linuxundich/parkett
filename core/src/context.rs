// SPDX-License-Identifier: GPL-3.0-or-later
//! Context of a pane for the context color (proposal 1): is a connection to another host, a
//! container, a program with root privileges or one of another user running there right now?
//! Detection works on the process name, command line and effective UID of the foreground
//! process (read from /proc by the app), for root on another host also on the terminal title.

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Context {
    #[default]
    Local,
    Superuser,
    /// Hostname (without user), as far as the command line reveals it
    Remote(String),
    /// root on another host: `ssh root@host`, or the remote shell's title starts with `root@`
    RemoteSuperuser(String),
    /// Container shell (toolbox, distrobox, `podman exec -it`, …): name of the container or tool
    Container(String),
    /// Another user than one's own, not root (`sudo -u postgres -i`, `su - www`)
    OtherUser(String),
}

/// What the context color says; decides color and how loud the marking is (approach C of
/// `mockups/context-colors.html`: the riskier, the louder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// root, here or on another host: red, tint, frame and badge
    Superuser,
    /// SSH: blue, line, badge and a light tint
    Remote,
    /// container: purple, dotted line and badge
    Container,
    /// other user: amber, line and badge
    OtherUser,
}

impl Context {
    pub fn kind(&self) -> Option<Kind> {
        match self {
            Context::Local => None,
            Context::Superuser | Context::RemoteSuperuser(_) => Some(Kind::Superuser),
            Context::Remote(_) => Some(Kind::Remote),
            Context::Container(_) => Some(Kind::Container),
            Context::OtherUser(_) => Some(Kind::OtherUser),
        }
    }

    /// Text of the badge: `root`, `ssh bender`, `root@bender`, `toolbox`, `postgres`.
    pub fn label(&self) -> Option<String> {
        match self {
            Context::Local => None,
            Context::Superuser => Some("root".into()),
            Context::Remote(host) => Some(format!("ssh {host}")),
            Context::RemoteSuperuser(host) => Some(format!("root@{host}")),
            Context::Container(name) | Context::OtherUser(name) => Some(name.clone()),
        }
    }

    /// Host of a connection; then local paths and directories mean nothing (R-04).
    pub fn remote_host(&self) -> Option<&str> {
        match self {
            Context::Remote(host) | Context::RemoteSuperuser(host) => Some(host),
            _ => None,
        }
    }
}

/// The foreground process of a pane, as the app reads it from /proc.
#[derive(Clone, Debug, Default)]
pub struct Process<'a> {
    /// process name (`/proc/PID/comm`)
    pub comm: &'a str,
    /// command line
    pub args: &'a [String],
    /// effective UID
    pub euid: Option<u32>,
    /// UID Parkett runs as
    pub own_uid: u32,
    /// user name for `euid` (from the user database)
    pub euid_name: Option<String>,
    /// terminal title (`xterm.title`); many distributions set `user@host: dir` there, which
    /// reveals root on the other host (Fedora, Debian, Ubuntu via PROMPT_COMMAND)
    pub title: Option<&'a str>,
}

const REMOTE: &[&str] = &["ssh", "mosh", "mosh-client", "autossh", "et", "telnet"];
const SUPERUSER: &[&str] = &["sudo", "su", "doas", "run0", "pkexec"];
/// Programs that only open a shell in a container
const CONTAINER_TOOLS: &[&str] = &["toolbox", "distrobox", "distrobox-enter", "systemd-nspawn", "lxc-attach"];
/// Programs that start or enter a container with `exec`/`run`/`shell`
const CONTAINER_ENGINES: &[&str] = &["podman", "docker", "nerdctl", "kubectl", "machinectl", "incus", "lxc"];

/// ssh option letters that take a value (`man ssh`).
const SSH_ARG_OPTS: &str = "bBcDEeFIiJLlmOoPpQRSWw";

pub fn classify(p: &Process) -> Context {
    let name = p.comm.rsplit('/').next().unwrap_or(p.comm);
    if REMOTE.contains(&name) {
        let host = remote_host(p.args).unwrap_or_else(|| name.to_string());
        let root_title = p.title.is_some_and(|t| t.trim_start().starts_with("root@"));
        if remote_user(p.args).as_deref() == Some("root") || root_title {
            return Context::RemoteSuperuser(host);
        }
        return Context::Remote(host);
    }
    if let Some(c) = container(name, p.args) {
        return Context::Container(c);
    }
    if SUPERUSER.contains(&name) {
        // sudo/su stay in the foreground with euid 0, also for `sudo -u postgres`
        return match target_user(name, p.args) {
            Some(user) if user != "root" && user != "0" => Context::OtherUser(user),
            _ => Context::Superuser,
        };
    }
    match p.euid {
        Some(0) => Context::Superuser,
        Some(uid) if uid != p.own_uid => Context::OtherUser(p.euid_name.clone().unwrap_or_else(|| uid.to_string())),
        _ => Context::Local,
    }
}

/// User of an ssh target: `user@host`, `ssh://user@host` or `-l user`.
pub fn remote_user(args: &[String]) -> Option<String> {
    let mut it = args.iter().skip(1);
    let mut from_option = None;
    while let Some(a) = it.next() {
        if a == "--" {
            break;
        }
        if let Some(flags) = a.strip_prefix('-') {
            if let Some(i) = flags.find(|c| SSH_ARG_OPTS.contains(c)) {
                let value = if i + 1 == flags.len() { it.next().cloned() } else { Some(flags[i + 1..].to_string()) };
                if flags.as_bytes()[i] == b'l' {
                    from_option = value;
                }
            }
            continue;
        }
        let t = a.strip_prefix("ssh://").unwrap_or(a);
        return t.rsplit_once('@').map(|(u, _)| u.to_string()).or(from_option);
    }
    from_option
}

/// Target user of sudo/su/doas/run0/pkexec; `None` = root.
fn target_user(name: &str, args: &[String]) -> Option<String> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if let Some(v) = a.strip_prefix("--user=") {
            return Some(v.to_string());
        }
        match (name, a.as_str()) {
            (_, "-u" | "--user") => return it.next().cloned(),
            ("su", "-c" | "--command" | "-s" | "--shell" | "-g" | "--group" | "-w") => {
                it.next();
            }
            // su takes the user as its first argument; sudo and the others the command
            ("su", a) if !a.starts_with('-') => return Some(a.to_string()),
            ("su", _) => {}
            (_, a) if a.starts_with('-') => {
                // a value right after `-u` (`-upostgres`) or bundled (`-iupostgres`) is rare; ignored
            }
            _ => return None,
        }
    }
    None
}

/// Container shell: name of the container, as far as the command line reveals it, otherwise
/// the tool. Engines only with a terminal (`-t`), otherwise it is a one-off command.
fn container(name: &str, args: &[String]) -> Option<String> {
    if CONTAINER_TOOLS.contains(&name) {
        let tool = name.trim_end_matches("-enter").trim_start_matches("systemd-");
        return Some(tool.to_string());
    }
    if !CONTAINER_ENGINES.contains(&name) {
        return None;
    }
    let pos = args.iter().position(|a| matches!(a.as_str(), "exec" | "run" | "shell" | "login" | "attach"))?;
    let verb = args[pos].as_str();
    let rest = &args[pos + 1..];
    let tty = rest
        .iter()
        .any(|a| a == "--tty" || a == "--tty=true" || (a.starts_with('-') && !a.starts_with("--") && a.contains('t')));
    if matches!(verb, "exec" | "run" | "attach") && !tty {
        return None;
    }
    // first positional argument after the verb: the container (exec) or the image (run)
    const WITH_VALUE: &[&str] =
        &["-e", "--env", "-u", "--user", "-w", "--workdir", "--env-file", "--detach-keys", "-c", "--container", "-n"];
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        if WITH_VALUE.contains(&a.as_str()) {
            it.next();
            continue;
        }
        if a.starts_with('-') {
            continue;
        }
        if verb != "run" {
            // `machinectl shell user@machine`
            let target = a.rsplit('@').next().unwrap_or(a);
            if !target.is_empty() && target != ".host" {
                return Some(target.to_string());
            }
        }
        break;
    }
    Some(name.to_string())
}

/// Target host from an ssh/mosh command line: the first argument that is neither an option nor
/// an option value; "user@host" → "host", "ssh://user@host:22" → "host".
pub fn remote_host(args: &[String]) -> Option<String> {
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        if a == "--" {
            return it.next().map(|h| clean_host(h));
        }
        if let Some(flags) = a.strip_prefix('-') {
            // Like getopt: flags can be bundled ("-vp"); the first letter that takes a value
            // takes the rest of the argument ("-p22", "-vp22") or, if nothing follows, the
            // next argument ("-p 22", "-vp 22")
            if let Some(i) = flags.find(|c| SSH_ARG_OPTS.contains(c)) {
                if i + 1 == flags.len() {
                    it.next();
                }
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
        assert_eq!(remote_host(&v("ssh -vp 22 host")), Some("host".into()), "R-11");
        assert_eq!(remote_host(&v("ssh -vp22 host")), Some("host".into()));
        assert_eq!(remote_host(&v("ssh -At -l root host")), Some("host".into()));
        assert_eq!(remote_host(&v("ssh -pv host")), Some("host".into()), "„v“ ist hier der Wert von -p");
        assert_eq!(remote_host(&v("ssh -v")), None);
    }

    fn proc<'a>(comm: &'a str, args: &'a [String], euid: u32) -> Process<'a> {
        Process { comm, args, euid: Some(euid), own_uid: 1000, ..Default::default() }
    }

    #[test]
    fn classify_processes() {
        let c = |comm: &str, line: &str, euid: u32| classify(&proc(comm, &v(line), euid));
        assert_eq!(c("ssh", "ssh bender", 1000), Context::Remote("bender".into()));
        assert_eq!(c("mosh-client", "mosh-client", 1000), Context::Remote("mosh-client".into()));
        assert_eq!(c("sudo", "sudo vim /etc/fstab", 0), Context::Superuser);
        assert_eq!(c("bash", "bash", 0), Context::Superuser, "root shell after sudo -i");
        assert_eq!(c("run0", "run0", 1000), Context::Superuser);
        assert_eq!(c("vim", "vim", 1000), Context::Local);
        assert_eq!(classify(&Process { comm: "bash", args: &v("bash"), ..Default::default() }), Context::Local);
    }

    #[test]
    fn root_on_another_host() {
        let c = |line: &str, title: Option<&str>| {
            let args = v(line);
            classify(&Process { title, ..proc("ssh", &args, 1000) })
        };
        assert_eq!(c("ssh root@bender", None), Context::RemoteSuperuser("bender".into()));
        assert_eq!(c("ssh -l root bender", None), Context::RemoteSuperuser("bender".into()));
        assert_eq!(c("ssh -lroot bender", None), Context::RemoteSuperuser("bender".into()));
        assert_eq!(c("ssh ssh://root@nas:22", None), Context::RemoteSuperuser("nas".into()));
        assert_eq!(c("ssh -p 22 toff@bender", None), Context::Remote("bender".into()));
        // sudo -i on the server: the title of the remote shell gives it away
        assert_eq!(c("ssh bender", Some("root@bender:~")), Context::RemoteSuperuser("bender".into()));
        assert_eq!(c("ssh bender", Some("toff@bender:~")), Context::Remote("bender".into()));
        assert_eq!(remote_user(&v("ssh -i key -l admin host")), Some("admin".into()));
        assert_eq!(remote_user(&v("ssh host")), None);
    }

    #[test]
    fn other_users() {
        let c = |comm: &str, line: &str, euid: u32| classify(&proc(comm, &v(line), euid));
        assert_eq!(c("sudo", "sudo -u postgres -i", 0), Context::OtherUser("postgres".into()));
        assert_eq!(c("sudo", "sudo --user=www-data bash", 0), Context::OtherUser("www-data".into()));
        assert_eq!(c("sudo", "sudo -u root -i", 0), Context::Superuser);
        assert_eq!(c("su", "su - postgres", 0), Context::OtherUser("postgres".into()));
        assert_eq!(c("su", "su -", 0), Context::Superuser);
        assert_eq!(c("su", "su -c ls", 0), Context::Superuser);
        assert_eq!(c("run0", "run0 --user=build", 0), Context::OtherUser("build".into()));
        let p = Process { euid_name: Some("postgres".into()), ..proc("psql", &[], 970) };
        assert_eq!(classify(&p), Context::OtherUser("postgres".into()));
    }

    #[test]
    fn containers() {
        let c = |comm: &str, line: &str| classify(&proc(comm, &v(line), 1000));
        // toolbox execs podman with the container name
        assert_eq!(
            c("podman", "podman exec --interactive --tty --user toff -w /home/toff fedora-toolbox-44 capsh"),
            Context::Container("fedora-toolbox-44".into())
        );
        assert_eq!(c("docker", "docker exec -it web bash"), Context::Container("web".into()));
        assert_eq!(c("docker", "docker run -it --rm debian bash"), Context::Container("docker".into()));
        assert_eq!(c("docker", "docker exec web ls"), Context::Local, "without a terminal: one-off command");
        assert_eq!(c("docker", "docker ps"), Context::Local);
        assert_eq!(c("toolbox", "toolbox enter"), Context::Container("toolbox".into()));
        assert_eq!(c("distrobox-enter", "distrobox-enter arch"), Context::Container("distrobox".into()));
        assert_eq!(c("machinectl", "machinectl shell toff@debian"), Context::Container("debian".into()));
        assert_eq!(c("systemd-nspawn", "systemd-nspawn -D /var/lib/machines/x"), Context::Container("nspawn".into()));
    }

    #[test]
    fn labels_and_kinds() {
        assert_eq!(Context::Local.label(), None);
        assert_eq!(Context::Remote("bender".into()).label().as_deref(), Some("ssh bender"));
        assert_eq!(Context::RemoteSuperuser("bender".into()).kind(), Some(Kind::Superuser));
        assert_eq!(Context::RemoteSuperuser("bender".into()).remote_host(), Some("bender"));
        assert_eq!(Context::Container("x".into()).remote_host(), None);
    }
}
