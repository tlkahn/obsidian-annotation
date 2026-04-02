use crate::types::Annotation;
use crate::scanner::scan_comments;
use crate::compact::parse_compact;
use crate::block::{is_block_form, parse_block};

/// Parse all annotation comments in a document.
/// Returns annotations ordered by their position in the document.
pub fn parse_annotations(content: &str) -> Vec<Annotation> {
    let raw_comments = scan_comments(content);
    let mut annotations = Vec::with_capacity(raw_comments.len());

    for rc in raw_comments {
        let mut ann = if is_block_form(&rc.inner) {
            parse_block(&rc.inner)
        } else {
            parse_compact(&rc.inner)
        };

        // Fill in position and original text from scanner
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
        let doc = "The term *anuttara*<!-- n? __ | same sense as TĀ 3.68? @2026-03 --> appears.";
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
    fn single_block_annotation() {
        let doc = "Text before.\n<!--\nn!\n\\p\n@2026-03-28\n---\nThe body.\n-->\nText after.";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[0].certainty, Certainty::Firm);
        assert_eq!(anns[0].scope, Scope::Paragraph);
        assert_eq!(anns[0].form, AnnotationForm::Block);
        assert_eq!(anns[0].body, Some("The body.".to_string()));
    }

    #[test]
    fn mixed_compact_and_block() {
        let doc = "<!-- n: | inline note -->\n\nParagraph.\n\n<!--\ncf\n---\nBlock crossref.\n-->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 2);
        assert_eq!(anns[0].form, AnnotationForm::Compact);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
        assert_eq!(anns[1].form, AnnotationForm::Block);
        assert_eq!(anns[1].annotation_type, AnnotationType::CrossRef);
    }

    #[test]
    fn bare_annotation() {
        let doc = "text<!-- compare Vasugupta SpK 1.1 -->more";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Bare);
        assert_eq!(anns[0].body, Some("compare Vasugupta SpK 1.1".to_string()));
    }

    #[test]
    fn skip_raw_comments() {
        let doc = "<!-- raw: ignore this --> <!-- n: | keep -->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 1);
        assert_eq!(anns[0].annotation_type, AnnotationType::Note);
    }

    #[test]
    fn skip_code_fenced_comments() {
        let doc = "```\n<!-- skip -->\n```\n<!-- q? | keep -->";
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
        let doc = "<!-- a --> middle <!-- b -->";
        let anns = parse_annotations(doc);
        assert_eq!(anns.len(), 2);
        assert!(anns[0].char_start < anns[1].char_start);
    }

    #[test]
    fn original_preserved() {
        let doc = "<!-- todo! | fix this -->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].original, "<!-- todo! | fix this -->");
    }

    #[test]
    fn utf16_offsets_with_cjk() {
        // 你好 = 2 UTF-16 units
        let doc = "你好<!-- n: | 注释 -->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].char_start, 2);
        assert_eq!(anns[0].body, Some("注释".to_string()));
    }

    #[test]
    fn apparatus_type_integration() {
        let doc = "<!-- app: | variant: ms. B has *prakāśa* -->";
        let anns = parse_annotations(doc);
        assert_eq!(anns[0].annotation_type, AnnotationType::Apparatus);
    }

    #[test]
    fn multiple_annotations_with_blocks_and_compact() {
        let doc = "\
First paragraph.<!-- n: _ | marginal note @2026-03 -->

<!--
todo!
\\p
@2026-03-28
---
Need to verify this claim.
-->

Second paragraph.<!-- cf \\pp -->
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
