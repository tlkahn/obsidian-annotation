use std::sync::LazyLock;
use regex::Regex;
use crate::types::*;

static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@(\d{4}-\d{2}(?:-\d{2})?)").unwrap()
});

static ANCHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\^"([^"]+)""#).unwrap()
});

static SCOPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(_{1,}|[0-9]_[0-9]|[0-9]\\[spf][0-9]|\\h|\\d|\\p(?:p+|_{1,})?|\\f(?:f+|_{1,})?|\\s(?:s+|_{1,})?)\s").unwrap()
});

/// Parse a compact form annotation from the inner text of an annotation comment.
/// Returns the parsed annotation (with char_start/char_end/original zeroed — caller fills those in).
pub fn parse_compact(inner: &str) -> Annotation {
    parse_compact_inner(inner, &[]).0
}

/// Whether the inner text has detectable annotation structure (type keyword,
/// certainty mark, scope token, anchor, pipe, date) or is block form.
/// Plain prose comments return false. An ID alone is not structure: a bare
/// [word] prefix is common in plain prose comments, and the migrate tool
/// must not rewrite those. (Custom mark codes are not consulted here — the
/// migrate tool only recognizes built-in structure.)
pub fn is_structured_annotation(inner: &str) -> bool {
    crate::parser::classify(inner, &[]).2
}

/// Whether the text after a candidate mark code is header material: a
/// certainty char (itself followed by header material), pipe, end of
/// comment, or (after whitespace) a scope token, anchor, date, or pipe.
/// Prose (e.g. "it is raining", "hi! everyone") is none of these.
fn mark_followed_by_header(after: &str) -> bool {
    match after.chars().next() {
        None => true,
        // A certainty char only counts when what follows it is also header
        // material — "hi! everyone" is prose, "sic? _" is a mark
        Some('?') | Some('!') | Some(':') => mark_followed_by_header(&after[1..]),
        Some('|') => true,
        Some(c) if c.is_whitespace() => {
            let t = after.trim_start();
            t.is_empty()
                || t.starts_with('|')
                || t.starts_with("^\"")
                || (t.starts_with('@') && DATE_RE.is_match(t))
                || SCOPE_RE.is_match(t)
                || (t.starts_with('_') && t.chars().all(|c| c == '_'))
                || Scope::try_parse(t).is_some()
        }
        _ => false,
    }
}

/// Parse the compact form, also returning whether any structure was detected.
pub(crate) fn parse_compact_inner(inner: &str, custom_marks: &[String]) -> (Annotation, bool) {
    let mut remaining = inner;
    let mut annotation_type = AnnotationType::Bare;
    let mut mark: Option<String> = None;
    let mut certainty = Certainty::Neutral;
    let mut scope = Scope::Sentence(1);
    let mut is_structured = false;

    // Step 1: Try to match type keyword at the start
    let type_keywords = ["todo", "app", "cf", "tr", "llm", "th", "n", "q"];
    for &kw in &type_keywords {
        if remaining.starts_with(kw) {
            let after = &remaining[kw.len()..];
            // Type must be followed by mark, whitespace, pipe, or end of string
            let next_ch = after.chars().next();
            if next_ch.is_none()
                || next_ch == Some('?')
                || next_ch == Some('!')
                || next_ch == Some(':')
                || next_ch == Some(' ')
                || next_ch == Some('|')
            {
                if let Some(t) = AnnotationType::from_str(kw) {
                    annotation_type = t;
                    remaining = after;
                    is_structured = true;
                    break;
                }
            }
        }
    }

    // Step 1.5: Try a philological mark code in the type slot (type keywords
    // take precedence). Several codes are common English words (it, hi, em,
    // st), so a code only counts as a mark when what follows is header
    // material — certainty, scope, anchor, pipe, or end of comment.
    if annotation_type == AnnotationType::Bare {
        let token: String = remaining
            .chars()
            .take_while(|c| c.is_ascii_lowercase())
            .collect();
        if is_builtin_mark(&token) || custom_marks.iter().any(|m| m == &token) {
            let after = &remaining[token.len()..];
            if mark_followed_by_header(after) {
                annotation_type = AnnotationType::Mark;
                mark = Some(token);
                remaining = after;
                // A lone code is NOT structure: the migrate tool must not
                // rewrite plain one-word legacy comments like <!-- nb -->.
                // It still parses as a Mark for triple-dash rendering.
                if !after.is_empty() {
                    is_structured = true;
                }
            }
        }
    }

    // Step 2: Try to match certainty mark
    if let Some(ch) = remaining.chars().next() {
        if ch == '?' || ch == '!' || ch == ':' {
            certainty = Certainty::from_char(ch);
            remaining = &remaining[1..];
            if ch != ':' {
                is_structured = true;
            }
        }
    }

    // Consume whitespace
    remaining = remaining.trim_start();

    // Step 3: Try to match scope (underscores or \p / \pp)
    // A new-style token (asymmetric, \h, \d) in a headerless comment is
    // ambiguous with prose (e.g. "2_4 is the ratio"); it only counts as
    // scope when a type/certainty preceded it or a | (or end) follows.
    let is_new_style = |sc: &Scope| {
        matches!(
            sc,
            Scope::Section
                | Scope::Document
                | Scope::AsymWords(..)
                | Scope::AsymSentence(..)
                | Scope::AsymParagraph(..)
                | Scope::AsymPage(..)
        )
    };
    if let Some(caps) = SCOPE_RE.captures(remaining) {
        let scope_str = caps.get(1).unwrap().as_str();
        let parsed = Scope::from_str(scope_str);
        let rest = &remaining[caps.get(0).unwrap().end()..];
        let ambiguous_prose =
            !is_structured && is_new_style(&parsed) && !rest.trim_start().starts_with('|');
        if !ambiguous_prose {
            scope = parsed;
            remaining = rest;
            is_structured = true;
        }
    } else if remaining.starts_with('_') && remaining.chars().all(|c| c == '_') {
        // Scope at end with no trailing space (e.g. "n: __")
        scope = Scope::from_str(remaining);
        remaining = "";
        is_structured = true;
    } else if Scope::try_parse(remaining).is_some() {
        // Scope at end: \p, \pp, \ppp, \f, \ff, \p__, \f___, etc.
        scope = Scope::from_str(remaining);
        remaining = "";
        is_structured = true;
    }

    remaining = remaining.trim_start();

    // Step 4: Try to match anchor ^"text"
    if let Some(caps) = ANCHOR_RE.captures(remaining) {
        scope = Scope::Anchor(caps.get(1).unwrap().as_str().to_string());
        remaining = &remaining[caps.get(0).unwrap().end()..];
        is_structured = true;
    }

    remaining = remaining.trim_start();

    // Step 5: Split on pipe for body
    let body_text = if let Some(idx) = remaining.find('|') {
        let after_pipe = remaining[idx + 1..].trim_start();
        is_structured = true;
        after_pipe
    } else {
        remaining
    };

    // Step 6: Extract date from body text
    let (body_clean, date) = if let Some(caps) = DATE_RE.captures(body_text) {
        let date_str = caps.get(1).unwrap().as_str().to_string();
        let before_date = body_text[..caps.get(0).unwrap().start()].trim_end();
        is_structured = true;
        (before_date, Some(date_str))
    } else {
        (body_text.trim_end(), None)
    };

    let body = if body_clean.is_empty() {
        None
    } else {
        Some(body_clean.to_string())
    };

    if !is_structured && mark.is_none() {
        // Nothing structured found — treat entire inner text as bare body
        return (
            Annotation {
                form: AnnotationForm::Compact,
                id: None,
                mark: None,
                annotation_type: AnnotationType::Bare,
                certainty: Certainty::Neutral,
                scope: Scope::Sentence(1),
                body: Some(inner.to_string()),
                date: None,
                char_start: 0,
                char_end: 0,
                original: String::new(),
            },
            false,
        );
    }

    (
        Annotation {
            form: AnnotationForm::Compact,
            id: None,
            mark,
            annotation_type,
            certainty,
            scope,
            body,
            date,
            char_start: 0,
            char_end: 0,
            original: String::new(),
        },
        is_structured,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_compact_annotation() {
        let ann = parse_compact("n? __ | same sense as TĀ 3.68? @2026-03");
        assert_eq!(ann.annotation_type, AnnotationType::Note);
        assert_eq!(ann.certainty, Certainty::Tentative);
        assert_eq!(ann.scope, Scope::Words(2));
        assert_eq!(ann.body, Some("same sense as TĀ 3.68?".to_string()));
        assert_eq!(ann.date, Some("2026-03".to_string()));
        assert_eq!(ann.form, AnnotationForm::Compact);
    }

    #[test]
    fn todo_firm_with_anchor() {
        let ann = parse_compact(r#"todo! ^"8th century" | Sanderson 2007 handout says 9th c."#);
        assert_eq!(ann.annotation_type, AnnotationType::Todo);
        assert_eq!(ann.certainty, Certainty::Firm);
        assert_eq!(ann.scope, Scope::Anchor("8th century".to_string()));
        assert_eq!(ann.body, Some("Sanderson 2007 handout says 9th c.".to_string()));
        assert_eq!(ann.date, None);
    }

    #[test]
    fn crossref_preceding_paragraph() {
        let ann = parse_compact(r"cf \pp");
        assert_eq!(ann.annotation_type, AnnotationType::CrossRef);
        assert_eq!(ann.certainty, Certainty::Neutral);
        assert_eq!(ann.scope, Scope::Paragraph(2));
        assert_eq!(ann.body, None);
    }

    #[test]
    fn note_with_colon_separator() {
        let ann = parse_compact("n: _ | seems wrong @2026-03");
        assert_eq!(ann.annotation_type, AnnotationType::Note);
        assert_eq!(ann.certainty, Certainty::Neutral);
        assert_eq!(ann.scope, Scope::Words(1));
        assert_eq!(ann.body, Some("seems wrong".to_string()));
        assert_eq!(ann.date, Some("2026-03".to_string()));
    }

    #[test]
    fn apparatus_type() {
        let ann = parse_compact("app: | variant reading in ms. B");
        assert_eq!(ann.annotation_type, AnnotationType::Apparatus);
        assert_eq!(ann.body, Some("variant reading in ms. B".to_string()));
    }

    #[test]
    fn type_only_no_body() {
        let ann = parse_compact("q?");
        assert_eq!(ann.annotation_type, AnnotationType::Question);
        assert_eq!(ann.certainty, Certainty::Tentative);
        assert_eq!(ann.body, None);
    }

    #[test]
    fn date_with_full_precision() {
        let ann = parse_compact("n: | a note @2026-03-28");
        assert_eq!(ann.date, Some("2026-03-28".to_string()));
    }

    #[test]
    fn bare_comment() {
        let ann = parse_compact("compare Vasugupta SpK 1.1");
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
        assert_eq!(ann.certainty, Certainty::Neutral);
        assert_eq!(ann.scope, Scope::Sentence(1));
        assert_eq!(ann.body, Some("compare Vasugupta SpK 1.1".to_string()));
    }

    #[test]
    fn body_only_with_pipe() {
        let ann = parse_compact("| just the body");
        // Pipe makes it structured, but no type → Bare type with extracted body
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
        assert_eq!(ann.body, Some("just the body".to_string()));
    }

    #[test]
    fn paragraph_scope() {
        let ann = parse_compact(r"n: \p | paragraph note");
        assert_eq!(ann.scope, Scope::Paragraph(1));
        assert_eq!(ann.body, Some("paragraph note".to_string()));
    }

    #[test]
    fn three_word_scope() {
        let ann = parse_compact("n: ___ | three words");
        assert_eq!(ann.scope, Scope::Words(3));
    }

    #[test]
    fn question_with_scope_and_anchor() {
        let ann = parse_compact(r#"q? ^"some phrase" | is this right?"#);
        assert_eq!(ann.annotation_type, AnnotationType::Question);
        assert_eq!(ann.scope, Scope::Anchor("some phrase".to_string()));
        assert_eq!(ann.body, Some("is this right?".to_string()));
    }

    #[test]
    fn translation_type() {
        let ann = parse_compact("tr: | Sanskrit translation of verse 3");
        assert_eq!(ann.annotation_type, AnnotationType::Translation);
        assert_eq!(ann.certainty, Certainty::Neutral);
        assert_eq!(ann.body, Some("Sanskrit translation of verse 3".to_string()));
    }

    #[test]
    fn translation_tentative_with_date() {
        let ann = parse_compact("tr? _ | tentative rendering @2026-03");
        assert_eq!(ann.annotation_type, AnnotationType::Translation);
        assert_eq!(ann.certainty, Certainty::Tentative);
        assert_eq!(ann.scope, Scope::Words(1));
        assert_eq!(ann.body, Some("tentative rendering".to_string()));
        assert_eq!(ann.date, Some("2026-03".to_string()));
    }

    #[test]
    fn page_scope() {
        let ann = parse_compact(r"n: \f | page-level note");
        assert_eq!(ann.scope, Scope::Page(1));
        assert_eq!(ann.body, Some("page-level note".to_string()));
    }

    #[test]
    fn page_scope_two() {
        let ann = parse_compact(r"n: \ff | this and preceding page");
        assert_eq!(ann.scope, Scope::Page(2));
    }

    #[test]
    fn page_scope_underscore_suffix() {
        let ann = parse_compact(r"cf \f__");
        assert_eq!(ann.annotation_type, AnnotationType::CrossRef);
        assert_eq!(ann.scope, Scope::Page(2));
        assert_eq!(ann.body, None);
    }

    #[test]
    fn paragraph_underscore_suffix_compact() {
        let ann = parse_compact(r"n: \p__ | two paragraphs");
        assert_eq!(ann.scope, Scope::Paragraph(2));
        assert_eq!(ann.body, Some("two paragraphs".to_string()));
    }

    #[test]
    fn page_scope_three_letters() {
        let ann = parse_compact(r"cf \fff");
        assert_eq!(ann.scope, Scope::Page(3));
    }

    #[test]
    fn page_scope_three_underscores() {
        let ann = parse_compact(r"cf \f___");
        assert_eq!(ann.scope, Scope::Page(3));
    }

    #[test]
    fn page_scope_equivalence() {
        let a = parse_compact(r"n: \f___ | note");
        let b = parse_compact(r"n: \fff | note");
        assert_eq!(a.scope, b.scope);
    }

    // Sentence scope

    #[test]
    fn sentence_scope() {
        let ann = parse_compact(r"n: \s | sentence-level note");
        assert_eq!(ann.scope, Scope::Sentence(1));
        assert_eq!(ann.body, Some("sentence-level note".to_string()));
    }

    #[test]
    fn sentence_scope_two() {
        let ann = parse_compact(r"n: \ss | two sentences");
        assert_eq!(ann.scope, Scope::Sentence(2));
    }

    #[test]
    fn sentence_scope_three_letters() {
        let ann = parse_compact(r"cf \sss");
        assert_eq!(ann.scope, Scope::Sentence(3));
    }

    #[test]
    fn sentence_scope_underscore_suffix() {
        let ann = parse_compact(r"cf \s__");
        assert_eq!(ann.annotation_type, AnnotationType::CrossRef);
        assert_eq!(ann.scope, Scope::Sentence(2));
        assert_eq!(ann.body, None);
    }

    #[test]
    fn sentence_scope_three_underscores() {
        let ann = parse_compact(r"cf \s___");
        assert_eq!(ann.scope, Scope::Sentence(3));
    }

    #[test]
    fn sentence_scope_equivalence() {
        let a = parse_compact(r"n: \s___ | note");
        let b = parse_compact(r"n: \sss | note");
        assert_eq!(a.scope, b.scope);
    }

    // Section / Document / asymmetric scopes

    #[test]
    fn section_scope_with_body() {
        let ann = parse_compact(r"n: \h | section note");
        assert_eq!(ann.scope, Scope::Section);
        assert_eq!(ann.body, Some("section note".to_string()));
    }

    #[test]
    fn document_scope_with_body() {
        let ann = parse_compact(r"llm \d | summarize entire document");
        assert_eq!(ann.annotation_type, AnnotationType::Llm);
        assert_eq!(ann.scope, Scope::Document);
    }

    #[test]
    fn section_scope_at_end() {
        let ann = parse_compact(r"cf \h");
        assert_eq!(ann.annotation_type, AnnotationType::CrossRef);
        assert_eq!(ann.scope, Scope::Section);
        assert_eq!(ann.body, None);
    }

    #[test]
    fn document_scope_at_end() {
        let ann = parse_compact(r"cf \d");
        assert_eq!(ann.scope, Scope::Document);
    }

    #[test]
    fn asym_paragraph_scope_with_body() {
        let ann = parse_compact(r"n: 2\p1 | two before one after");
        assert_eq!(ann.scope, Scope::AsymParagraph(2, 1));
        assert_eq!(ann.body, Some("two before one after".to_string()));
    }

    #[test]
    fn asym_words_scope_with_body() {
        let ann = parse_compact("n: 3_1 | words around");
        assert_eq!(ann.scope, Scope::AsymWords(3, 1));
    }

    #[test]
    fn asym_sentence_scope_at_end() {
        let ann = parse_compact(r"cf 2\s1");
        assert_eq!(ann.scope, Scope::AsymSentence(2, 1));
        assert_eq!(ann.body, None);
    }

    #[test]
    fn asym_page_scope_forward_only() {
        let ann = parse_compact(r"n: 0\s2 | forward only");
        assert_eq!(ann.scope, Scope::AsymSentence(0, 2));
    }

    // Headerless new-token guard: a new-style scope token (asym, \h, \d) with
    // no preceding type/certainty only counts as scope when followed by | or end

    #[test]
    fn digit_leading_bare_comment_stays_bare() {
        let ann = parse_compact("2_4 is the ratio");
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
        assert_eq!(ann.scope, Scope::Sentence(1));
        assert_eq!(ann.body, Some("2_4 is the ratio".to_string()));
    }

    #[test]
    fn document_token_bare_prose_stays_bare() {
        let ann = parse_compact(r"\d is a TeX macro");
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
        assert_eq!(ann.scope, Scope::Sentence(1));
        assert_eq!(ann.body, Some(r"\d is a TeX macro".to_string()));
    }

    #[test]
    fn headerless_asym_with_pipe_is_scope() {
        let ann = parse_compact("2_4 | note");
        assert_eq!(ann.scope, Scope::AsymWords(2, 4));
        assert_eq!(ann.body, Some("note".to_string()));
    }

    #[test]
    fn headerless_asym_alone_is_scope() {
        let ann = parse_compact("2_4");
        assert_eq!(ann.scope, Scope::AsymWords(2, 4));
        assert_eq!(ann.body, None);
    }

    #[test]
    fn typed_asym_prose_body_still_scope() {
        // With a type keyword present, the token is unambiguous header material
        let ann = parse_compact("n: 2_4 no pipe body");
        assert_eq!(ann.scope, Scope::AsymWords(2, 4));
        assert_eq!(ann.body, Some("no pipe body".to_string()));
    }

    #[test]
    fn unstructured_digit_leading_prose() {
        assert!(!is_structured_annotation("2_4 is the ratio"));
        assert!(is_structured_annotation("2_4 | x"));
    }

    // Philological marks

    #[test]
    fn mark_highlight_one_word() {
        let ann = parse_compact("hi _");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("hi".to_string()));
        assert_eq!(ann.scope, Scope::Words(1));
        assert_eq!(ann.body, None);
    }

    #[test]
    fn mark_tentative_sic() {
        let ann = parse_compact("sic? _");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("sic".to_string()));
        assert_eq!(ann.certainty, Certainty::Tentative);
        assert_eq!(ann.scope, Scope::Words(1));
    }

    #[test]
    fn mark_crux_with_body() {
        let ann = parse_compact("crux | dagger passage");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("crux".to_string()));
        assert_eq!(ann.scope, Scope::Sentence(1));
        assert_eq!(ann.body, Some("dagger passage".to_string()));
    }

    #[test]
    fn mark_emphasis_anchored() {
        let ann = parse_compact(r#"em ^"phrase" | emphasis here"#);
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("em".to_string()));
        assert_eq!(ann.scope, Scope::Anchor("phrase".to_string()));
        assert_eq!(ann.body, Some("emphasis here".to_string()));
    }

    #[test]
    fn mark_bold_paragraph() {
        let ann = parse_compact(r"nb \p");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("nb".to_string()));
        assert_eq!(ann.scope, Scope::Paragraph(1));
    }

    #[test]
    fn mark_alone_defaults_sentence() {
        let ann = parse_compact("gloss");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("gloss".to_string()));
        assert_eq!(ann.scope, Scope::Sentence(1));
    }

    #[test]
    fn mark_with_date() {
        let ann = parse_compact("hi _ | check this @2026-03");
        assert_eq!(ann.mark, Some("hi".to_string()));
        assert_eq!(ann.body, Some("check this".to_string()));
        assert_eq!(ann.date, Some("2026-03".to_string()));
    }

    #[test]
    fn all_sixteen_builtin_marks_parse() {
        for code in [
            "nb", "it", "ul", "st", "sc", "hi", "em", "sic", "crux", "lac",
            "del", "sup", "conj", "dub", "gloss", "interp",
        ] {
            let ann = parse_compact(&format!("{code} _"));
            assert_eq!(ann.annotation_type, AnnotationType::Mark, "code {code}");
            assert_eq!(ann.mark, Some(code.to_string()), "code {code}");
        }
    }

    #[test]
    fn mark_prose_words_stay_bare() {
        // it / hi / em / st are common words — a mark code followed by prose
        // is a bare comment, not a mark
        for inner in ["it is raining", "hi there everyone", "em dashes are nice", "st paul wrote"] {
            let ann = parse_compact(inner);
            assert_eq!(ann.annotation_type, AnnotationType::Bare, "inner {inner}");
            assert_eq!(ann.mark, None);
            assert_eq!(ann.body, Some(inner.to_string()));
        }
    }

    #[test]
    fn mark_certainty_followed_by_prose_stays_bare() {
        // Punctuation must not bypass the prose guard
        for inner in [
            "hi! everyone remember this",
            "it? not sure about this",
            "st: 21",
            "em: use a dash here",
        ] {
            let ann = parse_compact(inner);
            assert_eq!(ann.annotation_type, AnnotationType::Bare, "inner {inner}");
            assert_eq!(ann.mark, None);
            assert_eq!(ann.body, Some(inner.to_string()));
        }
    }

    #[test]
    fn mark_certainty_with_header_is_mark() {
        let ann = parse_compact("sic? _");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);

        let ann = parse_compact("hi! | body note");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.certainty, Certainty::Firm);
        assert_eq!(ann.body, Some("body note".to_string()));

        let ann = parse_compact("sic?");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.certainty, Certainty::Tentative);
    }

    #[test]
    fn mark_followed_by_date_is_mark() {
        let ann = parse_compact("hi @2026-03");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("hi".to_string()));
        assert_eq!(ann.date, Some("2026-03".to_string()));
        // But a bare @ mention is still prose
        let ann = parse_compact("hi @alice please review");
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
    }

    #[test]
    fn type_keyword_precedence_over_marks() {
        // n is always Note, never a mark, even though nb is a mark code
        let ann = parse_compact("n: _ | note");
        assert_eq!(ann.annotation_type, AnnotationType::Note);
        assert_eq!(ann.mark, None);
    }

    #[test]
    fn unknown_code_stays_bare() {
        let ann = parse_compact("zz is not a mark");
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
        assert_eq!(ann.mark, None);
    }

    #[test]
    fn structured_mark_annotation() {
        assert!(is_structured_annotation("hi _"));
        assert!(!is_structured_annotation("it is raining"));
    }

    #[test]
    fn lone_mark_code_is_not_structure() {
        // The migrate tool must not rewrite plain one-word legacy comments
        // like <!-- nb --> or <!-- sic -->
        assert!(!is_structured_annotation("nb"));
        assert!(!is_structured_annotation("sic"));
        assert!(!is_structured_annotation("gloss"));
    }

    #[test]
    fn lone_mark_code_still_parses_as_mark() {
        // In a triple-dash comment a lone code still renders as a mark
        let ann = parse_compact("sic");
        assert_eq!(ann.annotation_type, AnnotationType::Mark);
        assert_eq!(ann.mark, Some("sic".to_string()));
        assert_eq!(ann.body, None);
    }

    // llm / th types

    #[test]
    fn llm_type() {
        let ann = parse_compact("llm | summarize entire document");
        assert_eq!(ann.annotation_type, AnnotationType::Llm);
        assert_eq!(ann.body, Some("summarize entire document".to_string()));
    }

    #[test]
    fn thread_type_tentative() {
        let ann = parse_compact("th? | is this Jayaratha?");
        assert_eq!(ann.annotation_type, AnnotationType::Thread);
        assert_eq!(ann.certainty, Certainty::Tentative);
        assert_eq!(ann.body, Some("is this Jayaratha?".to_string()));
    }

    #[test]
    fn thread_type_only() {
        let ann = parse_compact("th");
        assert_eq!(ann.annotation_type, AnnotationType::Thread);
        assert_eq!(ann.body, None);
    }

    #[test]
    fn llm_with_scope() {
        let ann = parse_compact(r"llm: \p | rewrite this paragraph");
        assert_eq!(ann.annotation_type, AnnotationType::Llm);
        assert_eq!(ann.scope, Scope::Paragraph(1));
    }

    #[test]
    fn th_prefix_word_stays_bare() {
        let ann = parse_compact("throwaway note");
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
        assert_eq!(ann.body, Some("throwaway note".to_string()));
    }

    #[test]
    fn llm_prefix_word_stays_bare() {
        let ann = parse_compact("llms are interesting");
        assert_eq!(ann.annotation_type, AnnotationType::Bare);
        assert_eq!(ann.body, Some("llms are interesting".to_string()));
    }

    // is_structured_annotation

    #[test]
    fn structured_type_keyword() {
        assert!(is_structured_annotation("n: check TA 3.68"));
    }

    #[test]
    fn structured_certainty_and_date() {
        assert!(is_structured_annotation("todo! verify @2026-03"));
    }

    #[test]
    fn structured_scope_token() {
        assert!(is_structured_annotation(r"cf \pp"));
    }

    #[test]
    fn structured_pipe_only() {
        assert!(is_structured_annotation("| body only"));
    }

    #[test]
    fn structured_date_only() {
        assert!(is_structured_annotation("note @2026-03"));
    }

    #[test]
    fn structured_type_with_certainty() {
        assert!(is_structured_annotation("n?"));
    }

    #[test]
    fn structured_block_form() {
        assert!(is_structured_annotation("n\n---\nbody"));
    }

    #[test]
    fn id_with_structured_remainder() {
        assert!(is_structured_annotation("[x1] n: | note"));
        assert!(is_structured_annotation("[x1] todo! verify @2026-03"));
    }

    #[test]
    fn id_alone_is_not_structure() {
        // A bare [word] must not count as structure: the migrate tool would
        // otherwise rewrite plain legacy comments like <!-- [TODO] fix --> or
        // <!-- [1] see footnote --> into rendering annotations.
        assert!(!is_structured_annotation("[x1] hello"));
        assert!(!is_structured_annotation("[TODO] fix header wording"));
        assert!(!is_structured_annotation("[1] see footnote"));
    }

    #[test]
    fn unstructured_invalid_id() {
        assert!(!is_structured_annotation("[*b*] hello"));
        assert!(!is_structured_annotation("[[Wiki Link]] prose"));
    }

    #[test]
    fn unstructured_plain_prose() {
        assert!(!is_structured_annotation("fix this later"));
    }

    #[test]
    fn unstructured_raw_prefix() {
        assert!(!is_structured_annotation("raw: build marker"));
    }

    #[test]
    fn unstructured_empty() {
        assert!(!is_structured_annotation(""));
    }
}
