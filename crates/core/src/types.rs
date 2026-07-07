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
    /// A philological mark (display-only styling); the code is in `Annotation::mark`
    Mark,
    Bare,
}

/// The 16 built-in philological mark codes from the DSL spec.
pub const BUILTIN_MARK_CODES: [&str; 16] = [
    "nb", "it", "ul", "st", "sc", "hi", "em", "sic", "crux", "lac", "del", "sup", "conj", "dub",
    "gloss", "interp",
];

/// Whether `s` is a built-in philological mark code.
pub fn is_builtin_mark(s: &str) -> bool {
    BUILTIN_MARK_CODES.contains(&s)
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
    /// `\h` — current markdown heading section
    Section,
    /// `\d` — entire document
    Document,
    /// `N_M` — N words before, M words after
    AsymWords(u8, u8),
    /// `N\sM` — N sentences before, M sentences after
    AsymSentence(u8, u8),
    /// `N\pM` — N paragraphs before, M paragraphs after
    AsymParagraph(u8, u8),
    /// `N\fM` — N pages before, M pages after
    AsymPage(u8, u8),
}

impl Scope {
    /// Parse the asymmetric form: `N_M` (words) or `N\sM` / `N\pM` / `N\fM`,
    /// where N and M are single digits.
    fn try_parse_asymmetric(s: &str) -> Option<Self> {
        let mut chars = s.chars();
        let before = chars.next()?.to_digit(10)? as u8;
        let sep = chars.next()?;
        let (ctor, after_ch): (fn(u8, u8) -> Self, char) = match sep {
            '_' => (Self::AsymWords, chars.next()?),
            '\\' => {
                let unit = chars.next()?;
                let ctor = match unit {
                    's' => Self::AsymSentence as fn(u8, u8) -> Self,
                    'p' => Self::AsymParagraph,
                    'f' => Self::AsymPage,
                    _ => return None,
                };
                (ctor, chars.next()?)
            }
            _ => return None,
        };
        let after = after_ch.to_digit(10)? as u8;
        if chars.next().is_some() {
            return None; // trailing characters — not a bare asymmetric token
        }
        Some(ctor(before, after))
    }

    /// Try to parse a scope string. Returns None for unrecognized patterns.
    pub fn try_parse(s: &str) -> Option<Self> {
        if let Some(asym) = Self::try_parse_asymmetric(s) {
            Some(asym)
        } else if !s.is_empty() && s.starts_with('_') && s.chars().all(|c| c == '_') {
            Some(Self::Words(s.len() as u8))
        } else if s == r"\h" {
            Some(Self::Section)
        } else if s == r"\d" {
            Some(Self::Document)
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
    /// The mark code when `annotation_type` is `Mark` (e.g. "sic", "hi")
    #[serde(default)]
    pub mark: Option<String>,
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

    // Section / Document scopes

    #[test]
    fn scope_section() {
        assert_eq!(Scope::from_str(r"\h"), Scope::Section);
        assert_eq!(Scope::try_parse(r"\h"), Some(Scope::Section));
    }

    #[test]
    fn scope_document() {
        assert_eq!(Scope::from_str(r"\d"), Scope::Document);
        assert_eq!(Scope::try_parse(r"\d"), Some(Scope::Document));
    }

    #[test]
    fn scope_section_document_no_suffixes() {
        assert_eq!(Scope::try_parse(r"\hh"), None);
        assert_eq!(Scope::try_parse(r"\h_"), None);
        assert_eq!(Scope::try_parse(r"\dd"), None);
        assert_eq!(Scope::try_parse(r"\d_"), None);
    }

    #[test]
    fn scope_section_document_serde() {
        let json = serde_json::to_string(&Scope::Section).unwrap();
        assert_eq!(json, r#"{"kind":"section"}"#);
        assert_eq!(serde_json::from_str::<Scope>(&json).unwrap(), Scope::Section);

        let json = serde_json::to_string(&Scope::Document).unwrap();
        assert_eq!(json, r#"{"kind":"document"}"#);
        assert_eq!(serde_json::from_str::<Scope>(&json).unwrap(), Scope::Document);
    }

    // Asymmetric scopes

    #[test]
    fn scope_asym_words() {
        assert_eq!(Scope::try_parse("3_1"), Some(Scope::AsymWords(3, 1)));
        assert_eq!(Scope::try_parse("0_2"), Some(Scope::AsymWords(0, 2)));
        assert_eq!(Scope::try_parse("9_9"), Some(Scope::AsymWords(9, 9)));
    }

    #[test]
    fn scope_asym_sentence() {
        assert_eq!(Scope::try_parse(r"2\s1"), Some(Scope::AsymSentence(2, 1)));
        assert_eq!(Scope::try_parse(r"0\s2"), Some(Scope::AsymSentence(0, 2)));
    }

    #[test]
    fn scope_asym_paragraph() {
        assert_eq!(Scope::try_parse(r"3\p1"), Some(Scope::AsymParagraph(3, 1)));
        assert_eq!(Scope::try_parse(r"2\p0"), Some(Scope::AsymParagraph(2, 0)));
    }

    #[test]
    fn scope_asym_page() {
        assert_eq!(Scope::try_parse(r"2\f0"), Some(Scope::AsymPage(2, 0)));
        assert_eq!(Scope::try_parse(r"1\f1"), Some(Scope::AsymPage(1, 1)));
    }

    #[test]
    fn scope_asym_invalid() {
        assert_eq!(Scope::try_parse("12_1"), None); // multi-digit
        assert_eq!(Scope::try_parse("3_12"), None);
        assert_eq!(Scope::try_parse(r"3\x1"), None); // unknown unit
        assert_eq!(Scope::try_parse(r"3\h1"), None); // section has no asym form
        assert_eq!(Scope::try_parse(r"3\d1"), None); // document has no asym form
        assert_eq!(Scope::try_parse("_3"), None);
        assert_eq!(Scope::try_parse("3_"), None);
        assert_eq!(Scope::try_parse(r"2\s"), None);
        assert_eq!(Scope::try_parse("31"), None);
    }

    #[test]
    fn scope_asym_serde() {
        let scope = Scope::AsymWords(3, 1);
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, r#"{"kind":"asym_words","value":[3,1]}"#);
        assert_eq!(serde_json::from_str::<Scope>(&json).unwrap(), scope);

        let scope = Scope::AsymSentence(0, 2);
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, r#"{"kind":"asym_sentence","value":[0,2]}"#);
        assert_eq!(serde_json::from_str::<Scope>(&json).unwrap(), scope);

        let scope = Scope::AsymParagraph(2, 1);
        assert_eq!(
            serde_json::from_str::<Scope>(&serde_json::to_string(&scope).unwrap()).unwrap(),
            scope
        );
        let scope = Scope::AsymPage(2, 0);
        assert_eq!(
            serde_json::from_str::<Scope>(&serde_json::to_string(&scope).unwrap()).unwrap(),
            scope
        );
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
            mark: None,
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
