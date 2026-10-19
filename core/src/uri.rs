// SPDX-License-Identifier: GPL-3.0-or-later
//! `file:`-URIs aus dem Terminal (OSC 7 für das Arbeitsverzeichnis, OSC 8 für Links):
//! gehören sie zu diesem Rechner, und welcher Pfad steht darin?

/// `file:/pfad`, `file:///pfad`, `file://localhost/…` oder `file://<host>/…` mit dem eigenen
/// Rechnernamen `host`. Alles andere stammt z. B. aus einer SSH-Sitzung.
pub fn is_local_file(uri: &str, host: &str) -> bool {
    let Some(rest) = uri.get(5..).and_then(|r| r.strip_prefix("//")) else { return true };
    let h = rest.split('/').next().unwrap_or("");
    h.is_empty() || h.eq_ignore_ascii_case("localhost") || h.eq_ignore_ascii_case(host)
}

/// Lokaler Pfad einer `file:`-URI, Prozent-Kodierung aufgelöst (`%20` → Leerzeichen,
/// `%C3%9C` → Ü). `None` für fremde Rechner, andere Schemata, leere Pfade und Pfade, die
/// kein gültiges UTF-8 ergeben.
pub fn local_path(uri: &str, host: &str) -> Option<String> {
    if !uri.get(..5).is_some_and(|s| s.eq_ignore_ascii_case("file:")) || !is_local_file(uri, host) {
        return None;
    }
    let rest = &uri[5..];
    let path = match rest.strip_prefix("//") {
        Some(r) => &r[r.find('/')?..],
        None => rest,
    };
    // Abfrage und Fragment gehören nicht zum Pfad
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
        // OSC 7 aus einer SSH-Sitzung: Pfad gehört dem Server (QA-06)
        assert_eq!(local_path("file://entfernter-server/etc", "rechner"), None);
        assert_eq!(local_path("https://example.org/etc", "rechner"), None);
        assert_eq!(local_path("file:///tmp/%ZZ", "rechner"), None);
        assert_eq!(local_path("file:///tmp/%FF", "rechner"), None);
        assert_eq!(local_path("file:///tmp/%00x", "rechner"), None);
        assert_eq!(local_path("file://rechner", "rechner"), None);
    }
}
