use serde::{Deserialize, Serialize};

/// The annotation type keyword.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationType {
    Note,
    Question,
    Todo,
    CrossRef,
    Apparatus,
    Bare,
}

impl AnnotationType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "n" => Some(Self::Note),
            "q" => Some(Self::Question),
            "todo" => Some(Self::Todo),
            "cf" => Some(Self::CrossRef),
            "app" => Some(Self::Apparatus),
            _ => None,
        }
    }
}

/// Certainty mark following the type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Certainty {
    Tentative,
    Firm,
    Neutral,
}

impl Certainty {
    pub fn from_char(c: char) -> Self {
        match c {
            '?' => Self::Tentative,
            '!' => Self::Firm,
            _ => Self::Neutral,
        }
    }
}

/// Scope: how much surrounding text the annotation applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Scope {
    Words(u8),
    Paragraph,
    PrecedingParagraph,
    Anchor(String),
    Adjacency,
}

impl Scope {
    pub fn from_str(s: &str) -> Self {
        if s.starts_with('_') && s.chars().all(|c| c == '_') {
            Self::Words(s.len() as u8)
        } else if s == r"\pp" {
            Self::PrecedingParagraph
        } else if s == r"\p" {
            Self::Paragraph
        } else {
            Self::Adjacency
        }
    }
}

/// The form the annotation was written in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationForm {
    Compact,
    Block,
}

/// A fully parsed annotation with source positions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub form: AnnotationForm,
    pub annotation_type: AnnotationType,
    pub certainty: Certainty,
    pub scope: Scope,
    pub body: Option<String>,
    pub date: Option<String>,
    /// UTF-16 offset of the opening `<` of `<!--`
    pub char_start: usize,
    /// UTF-16 offset one past the closing `>` of `-->`
    pub char_end: usize,
    /// The original raw source text of the entire comment
    pub original: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // AnnotationType::from_str
    #[test]
    fn annotation_type_note() {
        assert_eq!(AnnotationType::from_str("n"), Some(AnnotationType::Note));
    }

    #[test]
    fn annotation_type_question() {
        assert_eq!(AnnotationType::from_str("q"), Some(AnnotationType::Question));
    }

    #[test]
    fn annotation_type_todo() {
        assert_eq!(AnnotationType::from_str("todo"), Some(AnnotationType::Todo));
    }

    #[test]
    fn annotation_type_crossref() {
        assert_eq!(AnnotationType::from_str("cf"), Some(AnnotationType::CrossRef));
    }

    #[test]
    fn annotation_type_apparatus() {
        assert_eq!(AnnotationType::from_str("app"), Some(AnnotationType::Apparatus));
    }

    #[test]
    fn annotation_type_unknown() {
        assert_eq!(AnnotationType::from_str("xyz"), None);
        assert_eq!(AnnotationType::from_str(""), None);
        assert_eq!(AnnotationType::from_str("N"), None); // case-sensitive
    }

    // Certainty::from_char
    #[test]
    fn certainty_tentative() {
        assert_eq!(Certainty::from_char('?'), Certainty::Tentative);
    }

    #[test]
    fn certainty_firm() {
        assert_eq!(Certainty::from_char('!'), Certainty::Firm);
    }

    #[test]
    fn certainty_neutral_colon() {
        assert_eq!(Certainty::from_char(':'), Certainty::Neutral);
    }

    #[test]
    fn certainty_neutral_other() {
        assert_eq!(Certainty::from_char('x'), Certainty::Neutral);
    }

    // Scope::from_str
    #[test]
    fn scope_one_word() {
        assert_eq!(Scope::from_str("_"), Scope::Words(1));
    }

    #[test]
    fn scope_three_words() {
        assert_eq!(Scope::from_str("___"), Scope::Words(3));
    }

    #[test]
    fn scope_paragraph() {
        assert_eq!(Scope::from_str(r"\p"), Scope::Paragraph);
    }

    #[test]
    fn scope_preceding_paragraph() {
        assert_eq!(Scope::from_str(r"\pp"), Scope::PrecedingParagraph);
    }

    #[test]
    fn scope_unrecognized_defaults_adjacency() {
        assert_eq!(Scope::from_str("unknown"), Scope::Adjacency);
    }

    // Serde round-trip
    #[test]
    fn annotation_serde_roundtrip() {
        let ann = Annotation {
            form: AnnotationForm::Compact,
            annotation_type: AnnotationType::Note,
            certainty: Certainty::Tentative,
            scope: Scope::Words(2),
            body: Some("a note".to_string()),
            date: Some("2026-03".to_string()),
            char_start: 10,
            char_end: 50,
            original: "<!-- n? __ | a note @2026-03 -->".to_string(),
        };
        let json = serde_json::to_string(&ann).unwrap();
        let parsed: Annotation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.annotation_type, AnnotationType::Note);
        assert_eq!(parsed.certainty, Certainty::Tentative);
        assert_eq!(parsed.scope, Scope::Words(2));
        assert_eq!(parsed.body, Some("a note".to_string()));
        assert_eq!(parsed.date, Some("2026-03".to_string()));
    }

    #[test]
    fn scope_serde_tagged() {
        let scope = Scope::Anchor("8th century".to_string());
        let json = serde_json::to_string(&scope).unwrap();
        // Verify round-trip works
        let parsed: Scope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, scope);

        // Verify variant name appears in JSON (serde rename_all lowercases it)
        let scope_adj = Scope::Adjacency;
        let json_adj = serde_json::to_string(&scope_adj).unwrap();
        let parsed_adj: Scope = serde_json::from_str(&json_adj).unwrap();
        assert_eq!(parsed_adj, Scope::Adjacency);

        let scope_words = Scope::Words(3);
        let json_words = serde_json::to_string(&scope_words).unwrap();
        let parsed_words: Scope = serde_json::from_str(&json_words).unwrap();
        assert_eq!(parsed_words, Scope::Words(3));
    }
}
