//! Trefferbewertung für die Befehlspalette: jedes Wort der Anfrage muss im Text vorkommen
//! (Groß/Klein egal); Treffer am Wortanfang und im Anfang des Textes zählen mehr.

/// Punktzahl oder `None`, wenn nicht jedes Wort vorkommt. Leere Anfrage passt immer (0).
pub fn score(query: &str, text: &str) -> Option<i32> {
    let text_l = text.to_lowercase();
    let mut total = 0;
    for word in query.to_lowercase().split_whitespace() {
        let pos = text_l.find(word)?;
        let at_word_start = pos == 0 || !text_l[..pos].chars().last().is_some_and(char::is_alphanumeric);
        total += 10 + if at_word_start { 20 } else { 0 } + if pos == 0 { 10 } else { 0 } - (pos.min(30) as i32 / 3);
    }
    Some(total)
}

/// Präfix der Anfrage für eine Ergebnisart: „>“ Aktionen, „@“ Reiter und Bereiche, „#“ Vorlagen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    All,
    Actions,
    Places,
    Templates,
}

/// Trennt das Präfix ab: („> teil“) → (Actions, „teil“).
pub fn split_scope(query: &str) -> (Scope, &str) {
    let q = query.trim_start();
    match q.chars().next() {
        Some('>') => (Scope::Actions, q[1..].trim_start()),
        Some('@') => (Scope::Places, q[1..].trim_start()),
        Some('#') => (Scope::Templates, q[1..].trim_start()),
        _ => (Scope::All, q),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_words_must_match() {
        assert!(score("rechts teilen", "Rechts teilen").is_some());
        assert!(score("teilen rechts", "Rechts teilen").is_some(), "Reihenfolge egal");
        assert!(score("rechts oben", "Rechts teilen").is_none());
        assert_eq!(score("", "egal"), Some(0));
    }

    #[test]
    fn word_start_and_prefix_rank_higher() {
        let start = score("be", "bender").unwrap();
        let word = score("be", "ssh bender").unwrap();
        let inner = score("be", "lui-theme").unwrap_or(-100);
        assert!(start > word, "{start} > {word}");
        assert!(word > inner, "{word} > {inner}");
    }

    #[test]
    fn umlauts_and_case() {
        assert!(score("GRÖSSEN", "Größen angleichen").is_none(), "ß ist kein SS – bewusst kein Ersetzen");
        assert!(score("größen", "Größen angleichen").is_some());
        assert!(score("ÜBERSICHT", "Reiter-Übersicht").is_some());
    }

    #[test]
    fn scopes() {
        assert_eq!(split_scope("> teil"), (Scope::Actions, "teil"));
        assert_eq!(split_scope("@bender"), (Scope::Places, "bender"));
        assert_eq!(split_scope(" #web"), (Scope::Templates, "web"));
        assert_eq!(split_scope("bender"), (Scope::All, "bender"));
    }
}
