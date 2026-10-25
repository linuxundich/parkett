// SPDX-License-Identifier: GPL-3.0-or-later
//! `file:` URIs from the terminal (OSC 7 for the working directory, OSC 8 for links):
//! do they belong to this host, and which path do they contain?

/// `file:/path`, `file:///path`, `file://localhost/…` or `file://<host>/…` with this
/// machine's hostname `host`. Anything else comes e.g. from an SSH session.
pub fn is_local_file(uri: &str, host: &str) -> bool {
    let Some(rest) = uri.get(5..).and_then(|r| r.strip_prefix("//")) else { return true };
    let h = rest.split('/').next().unwrap_or("");
    h.is_empty() || h.eq_ignore_ascii_case("localhost") || h.eq_ignore_ascii_case(host)
}

/// Local path of a `file:` URI, percent-encoding decoded (`%20` → space,
/// `%C3%9C` → Ü). `None` for other hosts, other schemes, empty paths and paths that
/// do not yield valid UTF-8.
pub fn local_path(uri: &str, host: &str) -> Option<String> {
    if !uri.get(..5).is_some_and(|s| s.eq_ignore_ascii_case("file:")) || !is_local_file(uri, host) {
        return None;
    }
    let rest = &uri[5..];
    let path = match rest.strip_prefix("//") {
        Some(r) => &r[r.find('/')?..],
        None => rest,
    };
    // query and fragment are not part of the path
    let path = path.split(['?', '#']).next().unwrap_or("");
    let decoded = percent_decode(path)?;
    (decoded.starts_with('/') && !decoded.contains('\0')).then_some(decoded)
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_hosts() {
        assert!(is_local_file("file:///etc", "rechner"));
        assert!(is_local_file("file:/etc", "rechner"));
        assert!(is_local_file("file://localhost/etc", "rechner"));
        assert!(is_local_file("file://Rechner/etc", "rechner"));
        assert!(!is_local_file("file://server/etc", "rechner"));
    }

    #[test]
    fn decodes_spaces_and_umlauts() {
        assert_eq!(local_path("file://rechner/tmp/a%20b/%C3%9C", "rechner").as_deref(), Some("/tmp/a b/Ü"));
        assert_eq!(local_path("file:///usr/share", "rechner").as_deref(), Some("/usr/share"));
        assert_eq!(local_path("file:///tmp/x%3Fy?frage#teil", "rechner").as_deref(), Some("/tmp/x?y"));
    }

    #[test]
    fn rejects_foreign_and_broken() {
        // OSC 7 from an SSH session: the path belongs to the server (QA-06)
        assert_eq!(local_path("file://entfernter-server/etc", "rechner"), None);
        assert_eq!(local_path("https://example.org/etc", "rechner"), None);
        assert_eq!(local_path("file:///tmp/%ZZ", "rechner"), None);
        assert_eq!(local_path("file:///tmp/%FF", "rechner"), None);
        assert_eq!(local_path("file:///tmp/%00x", "rechner"), None);
        assert_eq!(local_path("file://rechner", "rechner"), None);
    }
}
