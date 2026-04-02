use std::sync::LazyLock;
use regex::Regex;
use crate::types::*;

static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^@(\d{4}-\d{2}(?:-\d{2})?)$").unwrap()
});

static ANCHOR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\^"([^"]+)"$"#).unwrap()
});

/// Parse a block form annotation. The inner text is expected to contain at least one `---` line.
pub fn parse_block(inner: &str) -> Annotation {
    // Split on first line that is exactly "---" (possibly with surrounding whitespace)
    let (head, body) = split_head_body(inner);

    // Parse head lines
    let mut annotation_type = AnnotationType::Bare;
    let mut certainty = Certainty::Neutral;
    let mut scope = Scope::Adjacency;
    let mut date = None;

    for line in head.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Try date line
        if let Some(caps) = DATE_RE.captures(line) {
            date = Some(caps.get(1).unwrap().as_str().to_string());
            continue;
        }

        // Try anchor line
        if let Some(caps) = ANCHOR_RE.captures(line) {
            scope = Scope::Anchor(caps.get(1).unwrap().as_str().to_string());
            continue;
        }

        // Try scope line
        if Scope::try_parse(line).is_some() {
            scope = Scope::from_str(line);
            continue;
        }

        // Try type + optional certainty (e.g. "n!", "q?", "todo", "cf")
        if annotation_type == AnnotationType::Bare {
            let (type_part, cert_part) = if line.ends_with('?') || line.ends_with('!') {
                let mark = line.chars().last().unwrap();
                (&line[..line.len() - 1], Some(mark))
            } else {
                (line, None)
            };

            if let Some(t) = AnnotationType::from_str(type_part) {
                annotation_type = t;
                if let Some(mark) = cert_part {
                    certainty = Certainty::from_char(mark);
                }
            }
        }
    }

    let body = body
        .map(|b| b.trim())
        .filter(|b| !b.is_empty())
        .map(|b| b.to_string());

    Annotation {
        form: AnnotationForm::Block,
        annotation_type,
        certainty,
        scope,
        body,
        date,
        char_start: 0,
        char_end: 0,
        original: String::new(),
    }
}

/// Split inner text on the first `---` line (a line that is exactly `---` after trimming).
/// Returns (head, Some(body)) or (entire_text, None) if no separator found.
fn split_head_body(inner: &str) -> (&str, Option<&str>) {
    let mut byte_offset: usize = 0;
    for line in inner.split('\n') {
        if line.trim() == "---" {
            let head = &inner[..byte_offset.saturating_sub(1)]; // exclude the \n before ---
            let body_start = byte_offset + line.len() + 1; // skip the --- line and its \n
            let body = if body_start <= inner.len() {
                Some(&inner[body_start..])
            } else {
                None
            };
            return (head, body);
        }
        byte_offset += line.len() + 1; // +1 for \n
    }
    (inner, None)
}

/// Check if the inner text of a comment looks like block form (has a `---` separator line).
pub fn is_block_form(inner: &str) -> bool {
    inner.lines().any(|line| line.trim() == "---")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_block() {
        let inner = "n!\n\\p\n@2026-03-28\n---\nLambert's framing maps closely to Tainter's\ncomplexity brake.";
        let ann = parse_block(inner);
        assert_eq!(ann.form, AnnotationForm::Block);
        assert_eq!(ann.annotation_type, AnnotationType::Note);
        assert_eq!(ann.certainty, Certainty::Firm);
        assert_eq!(ann.scope, Scope::Paragraph(1));
        assert_eq!(ann.date, Some("2026-03-28".to_string()));
        assert_eq!(
            ann.body,
            Some("Lambert's framing maps closely to Tainter's\ncomplexity brake.".to_string())
        );
    }

    #[test]
    fn block_with_anchor() {
        let inner = "cf\n^\"anuttara\"\n@2026-03\n---\nPrimary parallels:\n- TĀ 3.68";
        let ann = parse_block(inner);
        assert_eq!(ann.annotation_type, AnnotationType::CrossRef);
        assert_eq!(ann.scope, Scope::Anchor("anuttara".to_string()));
        assert_eq!(ann.date, Some("2026-03".to_string()));
        assert!(ann.body.unwrap().contains("Primary parallels:"));
    }

    #[test]
    fn block_question_tentative() {
        let inner = "q?\n@2026-03-28\n---\nIs this Jayaratha or Abhinavagupta?";
        let ann = parse_block(inner);
        assert_eq!(ann.annotation_type, AnnotationType::Question);
        assert_eq!(ann.certainty, Certainty::Tentative);
        assert_eq!(ann.body, Some("Is this Jayaratha or Abhinavagupta?".to_string()));
    }

    #[test]
    fn block_with_multiple_body_sections() {
        let inner = "cf\n---\nFirst section.\n\n---\n\nSecond section.";
        let ann = parse_block(inner);
        assert_eq!(ann.annotation_type, AnnotationType::CrossRef);
        // Only first --- splits head/body; subsequent --- are in the body
        let body = ann.body.unwrap();
        assert!(body.contains("First section."));
        assert!(body.contains("---"));
        assert!(body.contains("Second section."));
    }

    #[test]
    fn block_no_body() {
        let inner = "todo\n\\p\n---";
        let ann = parse_block(inner);
        assert_eq!(ann.annotation_type, AnnotationType::Todo);
        assert_eq!(ann.scope, Scope::Paragraph(1));
        assert_eq!(ann.body, None);
    }

    #[test]
    fn block_apparatus() {
        let inner = "app\n---\nms. B reads *prakāśa* instead of *vimarśa*";
        let ann = parse_block(inner);
        assert_eq!(ann.annotation_type, AnnotationType::Apparatus);
        assert!(ann.body.unwrap().contains("ms. B reads"));
    }

    #[test]
    fn block_date_only_head() {
        let inner = "n\n@2026-03\n---\nSome note.";
        let ann = parse_block(inner);
        assert_eq!(ann.date, Some("2026-03".to_string()));
    }

    #[test]
    fn block_scope_underscores() {
        let inner = "n\n__\n---\nTwo words.";
        let ann = parse_block(inner);
        assert_eq!(ann.scope, Scope::Words(2));
    }

    #[test]
    fn block_page_scope() {
        let inner = "n\n\\f\n---\nPage-level note.";
        let ann = parse_block(inner);
        assert_eq!(ann.scope, Scope::Page(1));
    }

    #[test]
    fn block_page_scope_two() {
        let inner = "cf\n\\ff\n---\nCross-ref spanning two pages.";
        let ann = parse_block(inner);
        assert_eq!(ann.scope, Scope::Page(2));
    }

    #[test]
    fn block_paragraph_underscore_suffix() {
        let inner = "n\n\\p__\n---\nTwo paragraphs.";
        let ann = parse_block(inner);
        assert_eq!(ann.scope, Scope::Paragraph(2));
    }

    #[test]
    fn block_page_underscore_suffix() {
        let inner = "cf\n\\f___\n---\nThree pages.";
        let ann = parse_block(inner);
        assert_eq!(ann.scope, Scope::Page(3));
    }

    #[test]
    fn block_paragraph_three_letters() {
        let inner = "n\n\\ppp\n---\nThree paragraphs.";
        let ann = parse_block(inner);
        assert_eq!(ann.scope, Scope::Paragraph(3));
    }

    // is_block_form detection

    #[test]
    fn detect_block_form() {
        assert!(is_block_form("n\n---\nbody"));
        assert!(is_block_form("  ---  "));
        assert!(!is_block_form("no separator here"));
        assert!(!is_block_form("text --- inline")); // not on its own line
    }
}
