use crate::types::Annotation;
use crate::scanner::scan_comments;
use crate::compact::parse_compact_inner;
use crate::block::{is_block_form, parse_block};
use crate::id::extract_id;

/// Classify a comment's inner text: extract the optional leading ID, parse
/// the remainder in block or compact form, and report whether it has
/// detectable structure. Single entry point shared by `parse_annotations`
/// and `is_structured_annotation` so the two can't diverge.
/// `custom_marks` are workspace-defined mark codes (from `.lit/marks.toml`)
/// recognized alongside the built-in codes.
pub(crate) fn classify(inner: &str, custom_marks: &[String]) -> (Option<String>, Annotation, bool) {
    let (id, rest) = extract_id(inner);
    if is_block_form(rest) {
        (id, parse_block(rest, custom_marks), true)
    } else {
        let (ann, structured) = parse_compact_inner(rest, custom_marks);
        (id, ann, structured)
    }
}

/// Parse all annotation comments in a document (built-in mark codes only).
/// Returns annotations ordered by their position in the document.
pub fn parse_annotations(content: &str) -> Vec<Annotation> {
    parse_annotations_with_marks(content, &[])
}

/// Parse all annotation comments in a document, also recognizing the given
/// custom mark codes in the type slot.
pub fn parse_annotations_with_marks(content: &str, custom_marks: &[String]) -> Vec<Annotation> {
    let raw_comments = scan_comments(content);
    let mut annotations = Vec::with_capacity(raw_comments.len());

    for rc in raw_comments {
        let (id, mut ann, _) = classify(&rc.inner, custom_marks);

        // Fill in position, id, and original text from scanner
        ann.id = id;
        ann.char_start = rc.char_start;
        ann.char_end = rc.char_end;
        ann.original = rc.original;

        annotations.push(ann);
    }

    annotations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn single_compact_annotation() {
        let doc = "The term *anuttara*<!--- n? __ | same sense as TĀ 3.68? @2026-03 ---> appears.";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[0].certainty, Certainty::Tentative);
        assert_eq!(anns[0].scope, Scope::Words(2));
        assert_eq!(anns[0].body, Some("same sense as TĀ 3.68?".to_string()));
        assert_eq!(anns[0].date, Some("2026-03".to_string()));
        assert_eq!(anns[0].form, AnnotationForm::Compact);
        assert!(anns[0].char_start > 0);
        assert!(anns[0].char_end > anns[0].char_start);
    }

    #[test]
    fn compact_with_id() {
        let doc = r"<!---[my-note-id] n: \p | body text --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].id, Some("my-note-id".to_string()));
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[0].scope, Scope::Paragraph(1));
        assert_eq!(anns[0].body, Some("body text".to_string()));
    }

    #[test]
    fn compact_with_spaced_id() {
        // Spec grammar shows `<!--- [ID] TYPE ...`; leading whitespace is fine
        let doc = "<!--- [id42] q? | is this right? --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].id, Some("id42".to_string()));
        assert_eq!(anns[0].annotation_type, AnnotationType::Question);
    }

    #[test]
    fn compact_without_id() {
        let doc = "<!--- n: | no id here --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].id, None);
    }

    #[test]
    fn invalid_id_falls_back_to_body() {
        let doc = "<!--- [*bold*] note --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].id, None);
        assert_eq!(anns[0].annotation_type, AnnotationType::Bare);
        assert_eq!(anns[0].body, Some("[*bold*] note".to_string()));
    }

    #[test]
    fn id_with_bare_body() {
        let doc = "<!---[x1] hello world --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].id, Some("x1".to_string()));
        assert_eq!(anns[0].annotation_type, AnnotationType::Bare);
        assert_eq!(anns[0].body, Some("hello world".to_string()));
    }

    #[test]
    fn single_block_annotation() {
        let doc = "Text before.\n<!---\nn!\n\\p\n@2026-03-28\n---\nThe body.\n--->\nText after.";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[0].certainty, Certainty::Firm);
        assert_eq!(anns[0].scope, Scope::Paragraph(1));
        assert_eq!(anns[0].form, AnnotationForm::Block);
        assert_eq!(anns[0].body, Some("The body.".to_string()));
    }

    #[test]
    fn block_with_id() {
        let doc = "<!---[550e8400-e29b-41d4-a716-446655440000]\nn!\n\\p\n@2026-03-28\n---\nBody text here.\n--->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].form, AnnotationForm::Block);
        assert_eq!(anns[0].id, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[0].certainty, Certainty::Firm);
        assert_eq!(anns[0].scope, Scope::Paragraph(1));
        assert_eq!(anns[0].date, Some("2026-03-28".to_string()));
        assert_eq!(anns[0].body, Some("Body text here.".to_string()));
    }

    #[test]
    fn block_without_id() {
        let doc = "<!---\ncf\n---\nNo id.\n--->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].id, None);
        assert_eq!(anns[0].form, AnnotationForm::Block);
    }

    #[test]
    fn mixed_compact_and_block() {
        let doc = "<!--- n: | inline note --->\n\nParagraph.\n\n<!---\ncf\n---\nBlock crossref.\n--->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 2);
        assert_eq!(anns[0].form, AnnotationForm::Compact);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[1].form, AnnotationForm::Block);
        assert_eq!(anns[1].annotation_type, AnnotationType::CrossRef);
    }

    #[test]
    fn bare_annotation() {
        let doc = "text<!--- compare Vasugupta SpK 1.1 --->more";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Bare);
        assert_eq!(anns[0].body, Some("compare Vasugupta SpK 1.1".to_string()));
    }

    #[test]
    fn former_raw_now_parsed() {
        let doc = "<!--- raw: ignore this ---> <!--- n: | keep --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 2);
        assert_eq!(anns[0].annotation_type, AnnotationType::Bare);
        assert_eq!(anns[0].body, Some("raw: ignore this".to_string()));
        assert_eq!(anns[1].annotation_type, AnnotationType::Note);
    }

    #[test]
    fn skip_code_fenced_comments() {
        let doc = "```\n<!--- skip --->\n```\n<!--- q? | keep --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Question);
    }

    #[test]
    fn no_annotations() {
        assert_eq!(parse_annotations("no comments here").len(), 0);
    }

    #[test]
    fn ordering_by_position() {
        let doc = "<!--- a ---> middle <!--- b --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 2);
        assert!(anns[0].char_start < anns[1].char_start);
    }

    #[test]
    fn original_preserved() {
        let doc = "<!--- todo! | fix this --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].original, "<!--- todo! | fix this --->");
    }

    #[test]
    fn utf16_offsets_with_cjk() {
        // 你好 = 2 UTF-16 units
        let doc = "你好<!--- n: | 注释 --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].char_start, 2);
        assert_eq!(anns[0].body, Some("注释".to_string()));
    }

    #[test]
    fn section_scope_integration() {
        let doc = r"<!--- n: \h | section note --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].scope, Scope::Section);
    }

    #[test]
    fn document_scope_integration() {
        let doc = r"<!--- llm \d | summarize entire document --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].scope, Scope::Document);
    }

    #[test]
    fn asym_scope_integration() {
        let doc = r"<!--- q? 0\s2 | forward question --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].scope, Scope::AsymSentence(0, 2));
        assert_eq!(anns[0].annotation_type, AnnotationType::Question);
    }

    #[test]
    fn apparatus_type_integration() {
        let doc = "<!--- app: | variant: ms. B has *prakāśa* --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].annotation_type, AnnotationType::Apparatus);
    }

    #[test]
    fn mark_integration() {
        let doc = "<!--- hi _ ---> and <!--- sic? __ --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 2);
        assert_eq!(anns[0].annotation_type, AnnotationType::Mark);
        assert_eq!(anns[0].mark, Some("hi".to_string()));
        assert_eq!(anns[1].mark, Some("sic".to_string()));
        assert_eq!(anns[1].certainty, Certainty::Tentative);
    }

    #[test]
    fn custom_mark_recognized() {
        let doc = "<!--- mymark _ | styled --->";
        let custom = vec!["mymark".to_string()];
        let anns = parse_annotations_with_marks(doc, &custom);
        assert_eq!(anns[0].annotation_type, AnnotationType::Mark);
        assert_eq!(anns[0].mark, Some("mymark".to_string()));
        assert_eq!(anns[0].scope, Scope::Words(1));
        assert_eq!(anns[0].body, Some("styled".to_string()));
    }

    #[test]
    fn custom_mark_unknown_without_registration() {
        let doc = "<!--- mymark _ | styled --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].annotation_type, AnnotationType::Bare);
        assert_eq!(anns[0].mark, None);
    }

    #[test]
    fn custom_mark_cannot_shadow_type_keyword() {
        let doc = "<!--- n: _ | note --->";
        let custom = vec!["n".to_string()];
        let anns = parse_annotations_with_marks(doc, &custom);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[0].mark, None);
    }

    #[test]
    fn custom_mark_in_block_form() {
        let doc = "<!---\nmymark\n---\nBody.\n--->";
        let custom = vec!["mymark".to_string()];
        let anns = parse_annotations_with_marks(doc, &custom);
        assert_eq!(anns[0].annotation_type, AnnotationType::Mark);
        assert_eq!(anns[0].mark, Some("mymark".to_string()));
    }

    #[test]
    fn custom_mark_prose_guard_applies() {
        // Custom codes get the same prose-ambiguity guard as built-ins
        let doc = "<!--- mymark went to town --->";
        let custom = vec!["mymark".to_string()];
        let anns = parse_annotations_with_marks(doc, &custom);
        assert_eq!(anns[0].annotation_type, AnnotationType::Bare);
        assert_eq!(anns[0].body, Some("mymark went to town".to_string()));
    }

    #[test]
    fn translation_type_integration() {
        let doc = "<!--- tr: _ | cf. Tibetan version @2026-03 --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Translation);
        assert_eq!(anns[0].scope, Scope::Words(1));
        assert_eq!(anns[0].body, Some("cf. Tibetan version".to_string()));
        assert_eq!(anns[0].date, Some("2026-03".to_string()));
    }

    #[test]
    fn llm_type_integration() {
        let doc = "<!--- llm | summarize entire document --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].annotation_type, AnnotationType::Llm);
    }

    #[test]
    fn thread_with_id_integration() {
        let doc = "<!---[t1] th? | thread here --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].id, Some("t1".to_string()));
        assert_eq!(anns[0].annotation_type, AnnotationType::Thread);
        assert_eq!(anns[0].certainty, Certainty::Tentative);
    }

    #[test]
    fn page_scope_compact_integration() {
        let doc = r"<!--- n: \f | page-level note --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].scope, Scope::Page(1));
    }

    #[test]
    fn page_scope_block_integration() {
        let doc = "<!---\ncf\n\\ff\n---\nTwo pages.\n--->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].scope, Scope::Page(2));
    }

    #[test]
    fn underscore_suffix_scope_integration() {
        let doc = r"<!--- n: \p__ | two paragraphs --->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].scope, Scope::Paragraph(2));
        // Equivalent to \pp
        let doc2 = r"<!--- n: \pp | two paragraphs --->";
        let anns2 = parse_annotations(doc2);
        assert_eq!(anns[0].scope, anns2[0].scope);
    }

    #[test]
    fn multiple_annotations_with_blocks_and_compact() {
        let doc = "\
First paragraph.<!--- n: _ | marginal note @2026-03 --->

<!---
todo!
\\p
@2026-03-28
---
Need to verify this claim.
--->

Second paragraph.<!--- cf \\pp --->
";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 3);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[0].form, AnnotationForm::Compact);
        assert_eq!(anns[1].annotation_type, AnnotationType::Todo);
        assert_eq!(anns[1].form, AnnotationForm::Block);
        assert_eq!(anns[2].annotation_type, AnnotationType::CrossRef);
        assert_eq!(anns[2].form, AnnotationForm::Compact);
    }
}
