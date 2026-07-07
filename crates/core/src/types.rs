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
    Translation,
    Llm,
    Thread,
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
            "tr" => Some(Self::Translation),
            "llm" => Some(Self::Llm),
            "th" => Some(Self::Thread),
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
    /// `_` = 1 word, `__` = 2 words, etc.
    Words(u8),
    /// `\p` = 1 (current paragraph), `\pp` or `\p__` = 2 (current + preceding), etc.
    Paragraph(u8),
    /// `\f` = 1 (current page), `\ff` or `\f__` = 2, etc.
    Page(u8),
    /// `\s` = 1 (current sentence), `\ss` or `\s__` = 2, etc.
    Sentence(u8),
    /// `^"text"` — explicit anchor by search key
    Anchor(String),
}

impl Scope {
    /// Try to parse a scope string. Returns None for unrecognized patterns.
    pub fn try_parse(s: &str) -> Option<Self> {
        if !s.is_empty() && s.starts_with('_') && s.chars().all(|c| c == '_') {
            Some(Self::Words(s.len() as u8))
        } else if s.starts_with(r"\p") {
            let rest = &s[2..];
            if rest.is_empty() || rest.chars().all(|c| c == 'p') {
                Some(Self::Paragraph((1 + rest.len()) as u8))
            } else if rest.chars().all(|c| c == '_') {
                Some(Self::Paragraph(rest.len() as u8))
            } else {
                None
            }
        } else if s.starts_with(r"\f") {
            let rest = &s[2..];
            if rest.is_empty() || rest.chars().all(|c| c == 'f') {
                Some(Self::Page((1 + rest.len()) as u8))
            } else if rest.chars().all(|c| c == '_') {
                Some(Self::Page(rest.len() as u8))
            } else {
                None
            }
        } else if s.starts_with(r"\s") {
            let rest = &s[2..];
            if rest.is_empty() || rest.chars().all(|c| c == 's') {
                Some(Self::Sentence((1 + rest.len()) as u8))
            } else if rest.chars().all(|c| c == '_') {
                Some(Self::Sentence(rest.len() as u8))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn from_str(s: &str) -> Self {
        Self::try_parse(s).unwrap_or(Self::Sentence(1))
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
    /// Optional `[id]` placed immediately after the opening delimiter
    #[serde(default)]
    pub id: Option<String>,
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
    fn annotation_type_translation() {
        assert_eq!(AnnotationType::from_str("tr"), Some(AnnotationType::Translation));
    }

    #[test]
    fn annotation_type_llm() {
        assert_eq!(AnnotationType::from_str("llm"), Some(AnnotationType::Llm));
    }

    #[test]
    fn annotation_type_thread() {
        assert_eq!(AnnotationType::from_str("th"), Some(AnnotationType::Thread));
    }

    #[test]
    fn annotation_type_llm_thread_serde() {
        assert_eq!(serde_json::to_string(&AnnotationType::Llm).unwrap(), "\"llm\"");
        assert_eq!(serde_json::to_string(&AnnotationType::Thread).unwrap(), "\"thread\"");
        let llm: AnnotationType = serde_json::from_str("\"llm\"").unwrap();
        assert_eq!(llm, AnnotationType::Llm);
        let th: AnnotationType = serde_json::from_str("\"thread\"").unwrap();
        assert_eq!(th, AnnotationType::Thread);
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
        assert_eq!(Scope::from_str(r"\p"), Scope::Paragraph(1));
    }

    #[test]
    fn scope_paragraph_two() {
        assert_eq!(Scope::from_str(r"\pp"), Scope::Paragraph(2));
    }

    #[test]
    fn scope_paragraph_three() {
        assert_eq!(Scope::from_str(r"\ppp"), Scope::Paragraph(3));
    }

    #[test]
    fn scope_paragraph_underscore_suffix() {
        assert_eq!(Scope::from_str(r"\p__"), Scope::Paragraph(2));
        assert_eq!(Scope::from_str(r"\p___"), Scope::Paragraph(3));
    }

    #[test]
    fn scope_paragraph_underscore_one() {
        // \p_ with 1 underscore = Paragraph(1), same as \p
        assert_eq!(Scope::from_str(r"\p_"), Scope::Paragraph(1));
    }

    #[test]
    fn scope_page() {
        assert_eq!(Scope::from_str(r"\f"), Scope::Page(1));
    }

    #[test]
    fn scope_page_two() {
        assert_eq!(Scope::from_str(r"\ff"), Scope::Page(2));
    }

    #[test]
    fn scope_page_three() {
        assert_eq!(Scope::from_str(r"\fff"), Scope::Page(3));
    }

    #[test]
    fn scope_page_underscore_suffix() {
        assert_eq!(Scope::from_str(r"\f__"), Scope::Page(2));
        assert_eq!(Scope::from_str(r"\f___"), Scope::Page(3));
    }

    #[test]
    fn scope_sentence() {
        assert_eq!(Scope::from_str(r"\s"), Scope::Sentence(1));
    }

    #[test]
    fn scope_sentence_two() {
        assert_eq!(Scope::from_str(r"\ss"), Scope::Sentence(2));
    }

    #[test]
    fn scope_sentence_three() {
        assert_eq!(Scope::from_str(r"\sss"), Scope::Sentence(3));
    }

    #[test]
    fn scope_sentence_underscore_suffix() {
        assert_eq!(Scope::from_str(r"\s__"), Scope::Sentence(2));
        assert_eq!(Scope::from_str(r"\s___"), Scope::Sentence(3));
    }

    #[test]
    fn scope_sentence_underscore_one() {
        // \s_ with 1 underscore = Sentence(1), same as \s
        assert_eq!(Scope::from_str(r"\s_"), Scope::Sentence(1));
    }

    #[test]
    fn scope_equivalences() {
        // \p__ = \pp, \f___ = \fff, \s__ = \ss
        assert_eq!(Scope::from_str(r"\p__"), Scope::from_str(r"\pp"));
        assert_eq!(Scope::from_str(r"\f___"), Scope::from_str(r"\fff"));
        assert_eq!(Scope::from_str(r"\s__"), Scope::from_str(r"\ss"));
    }

    #[test]
    fn scope_unrecognized_defaults_sentence() {
        assert_eq!(Scope::from_str("unknown"), Scope::Sentence(1));
        assert_eq!(Scope::from_str(r"\pf"), Scope::Sentence(1)); // mixed letters
        assert_eq!(Scope::from_str(r"\fp"), Scope::Sentence(1)); // mixed letters
    }

    // Serde round-trip
    #[test]
    fn annotation_serde_roundtrip() {
        let ann = Annotation {
            form: AnnotationForm::Compact,
            id: Some("test-id".to_string()),
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
        let parsed: Scope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, scope);

        let scope_adj = Scope::Sentence(1);
        let json_adj = serde_json::to_string(&scope_adj).unwrap();
        let parsed_adj: Scope = serde_json::from_str(&json_adj).unwrap();
        assert_eq!(parsed_adj, Scope::Sentence(1));

        let scope_words = Scope::Words(3);
        let json_words = serde_json::to_string(&scope_words).unwrap();
        let parsed_words: Scope = serde_json::from_str(&json_words).unwrap();
        assert_eq!(parsed_words, Scope::Words(3));

        // Paragraph and Page round-trip
        let scope_para = Scope::Paragraph(2);
        let json_para = serde_json::to_string(&scope_para).unwrap();
        let parsed_para: Scope = serde_json::from_str(&json_para).unwrap();
        assert_eq!(parsed_para, Scope::Paragraph(2));

        let scope_page = Scope::Page(3);
        let json_page = serde_json::to_string(&scope_page).unwrap();
        let parsed_page: Scope = serde_json::from_str(&json_page).unwrap();
        assert_eq!(parsed_page, Scope::Page(3));

        // Sentence round-trip
        let scope_sent = Scope::Sentence(2);
        let json_sent = serde_json::to_string(&scope_sent).unwrap();
        let parsed_sent: Scope = serde_json::from_str(&json_sent).unwrap();
        assert_eq!(parsed_sent, Scope::Sentence(2));
    }
}
