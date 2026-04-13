use crate::scanner::utf16_len;
use crate::types::Scope;

/// Resolve the text range that an annotation's scope refers to.
///
/// Given the full document content, the annotation's UTF-16 start offset,
/// and its scope, returns `Some((scope_start, scope_end))` in UTF-16 offsets,
/// or `None` if the scope cannot be resolved.
pub fn resolve_scope_range(
    content: &str,
    char_start: usize,
    scope: &Scope,
    lang: &str,
) -> Option<(usize, usize)> {
    match scope {
        Scope::Words(n) => resolve_words(content, char_start, *n as usize),
        Scope::Sentence(n) => resolve_sentence(content, char_start, *n as usize, lang),
        Scope::Paragraph(n) => resolve_paragraph(content, char_start, *n as usize),
        Scope::Page(n) => resolve_page(content, char_start, *n as usize),
        Scope::Anchor(text) => resolve_anchor(content, char_start, text),
    }
}

/// Convert a UTF-16 offset to a byte offset in the string.
fn utf16_to_byte(s: &str, utf16_offset: usize) -> usize {
    let mut utf16_acc = 0;
    for (byte_idx, ch) in s.char_indices() {
        if utf16_acc >= utf16_offset {
            return byte_idx;
        }
        utf16_acc += ch.len_utf16();
    }
    s.len()
}

/// Resolve `Words(n)` scope: find the N preceding words before `char_start`.
fn resolve_words(content: &str, char_start: usize, n: usize) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    let text_before = &content[..byte_start];

    // Trim trailing whitespace to find the end of actual text
    let trimmed = text_before.trim_end();
    if trimmed.is_empty() {
        return None;
    }
    let scope_end_byte = trimmed.len();

    // Walk backwards counting words (whitespace-delimited)
    let mut words_found = 0;
    let mut scope_start_byte = 0;
    let mut in_word = false;

    for (i, ch) in trimmed.char_indices().rev() {
        if ch.is_whitespace() {
            if in_word {
                words_found += 1;
                if words_found >= n {
                    scope_start_byte = i + ch.len_utf8();
                    break;
                }
                in_word = false;
            }
        } else {
            in_word = true;
        }
    }

    // If we ran out of text before finding enough words, start from beginning
    if words_found < n && in_word {
        words_found += 1;
    }
    if words_found < n {
        scope_start_byte = 0;
    }

    let scope_start_utf16 = utf16_len(&content[..scope_start_byte]);
    let scope_end_utf16 = utf16_len(&content[..scope_end_byte]);

    Some((scope_start_utf16, scope_end_utf16))
}

/// Find `needle` in `haystack[start_from..]`, treating any run of whitespace in the
/// needle as matching any non-empty run of whitespace in the haystack.  This is needed
/// because sentenza's preprocessing may collapse double spaces, so the returned sentence
/// text won't exactly match the original paragraph text.
/// Returns `(match_start, match_end)` as byte indices into `haystack`.
fn ws_flexible_find(haystack: &str, needle: &str, start_from: usize) -> Option<(usize, usize)> {
    let parts: Vec<&str> = needle.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut offset = start_from;
    loop {
        let rel_pos = haystack[offset..].find(parts[0])?;
        let match_start = offset + rel_pos;
        let mut cursor = match_start + parts[0].len();

        let mut ok = true;
        for part in &parts[1..] {
            let rest = &haystack[cursor..];
            let ws = rest.len() - rest.trim_start().len();
            if ws == 0 {
                ok = false;
                break;
            }
            cursor += ws;
            if haystack[cursor..].starts_with(part) {
                cursor += part.len();
            } else {
                ok = false;
                break;
            }
        }

        if ok {
            return Some((match_start, cursor));
        }

        // Advance past current position by one character
        match haystack[offset + rel_pos..].char_indices().nth(1) {
            Some((next, _)) => offset += rel_pos + next,
            None => return None,
        }
    }
}

/// Resolve `Sentence(n)` scope: find the last N sentences before `char_start` using sentenza.
/// Extracts the current paragraph (up to `char_start`) and splits into sentences.
fn resolve_sentence(content: &str, char_start: usize, n: usize, lang: &str) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    let text_before = &content[..byte_start];
    let trimmed = text_before.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    // Find the current paragraph: look for the last double-newline before the annotation
    let para_byte_start = trimmed.rfind("\n\n").map(|i| i + 2).unwrap_or(0);
    let paragraph = &trimmed[para_byte_start..];

    if paragraph.trim().is_empty() {
        return None;
    }

    // Split into sentences using sentenza
    let sentences = sentenza::split_sentences(paragraph, lang);
    if sentences.is_empty() {
        return None;
    }

    // Take the last n sentences (or all if fewer available)
    let take = n.min(sentences.len());
    let first_sentence = &sentences[sentences.len() - take];
    let last_sentence = &sentences[sentences.len() - 1];

    // Use whitespace-flexible search: sentenza may normalize whitespace during
    // preprocessing (e.g. collapsing double spaces), so the returned sentence text
    // might not match the original paragraph verbatim.
    let (first_start, _) = ws_flexible_find(paragraph, first_sentence, 0)?;
    let (_, last_end) = ws_flexible_find(paragraph, last_sentence, first_start)?;

    let scope_start_byte = para_byte_start + first_start;
    let scope_end_byte = (para_byte_start + last_end).min(trimmed.len());

    let scope_start_utf16 = utf16_len(&content[..scope_start_byte]);
    let scope_end_utf16 = utf16_len(&content[..scope_end_byte]);

    Some((scope_start_utf16, scope_end_utf16))
}

/// Resolve `Paragraph(n)` scope: find the current paragraph + n-1 preceding paragraphs.
/// Paragraphs are delimited by double newlines (`\n\n`).
fn resolve_paragraph(content: &str, char_start: usize, n: usize) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    let text_before = &content[..byte_start];
    let trimmed = text_before.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    let scope_end_byte = trimmed.len();

    // Collect paragraph boundary positions (byte offsets right after each "\n\n")
    let mut para_boundaries: Vec<usize> = vec![0]; // start of document is always a boundary
    let mut i = 0;
    let bytes = trimmed.as_bytes();
    while i + 1 < bytes.len() {
        if bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            // Skip consecutive newlines
            let mut end = i + 2;
            while end < bytes.len() && bytes[end] == b'\n' {
                end += 1;
            }
            para_boundaries.push(end);
            i = end;
        } else {
            i += 1;
        }
    }

    // The annotation is in the last paragraph. Walk back n paragraph boundaries.
    let boundary_idx = if para_boundaries.len() >= n {
        para_boundaries.len() - n
    } else {
        0
    };
    let scope_start_byte = para_boundaries[boundary_idx];

    let scope_start_utf16 = utf16_len(&content[..scope_start_byte]);
    let scope_end_utf16 = utf16_len(&content[..scope_end_byte]);

    Some((scope_start_utf16, scope_end_utf16))
}

/// Resolve `Page(n)` scope: find pages delimited by form feed (`\x0C`) characters.
fn resolve_page(content: &str, char_start: usize, n: usize) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    let text_before = &content[..byte_start];
    let trimmed = text_before.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    let scope_end_byte = trimmed.len();

    // Collect form-feed boundary positions
    let mut page_boundaries: Vec<usize> = vec![0];
    for (i, b) in trimmed.bytes().enumerate() {
        if b == b'\x0C' {
            page_boundaries.push(i + 1); // byte after the form feed
        }
    }

    let boundary_idx = if page_boundaries.len() >= n {
        page_boundaries.len() - n
    } else {
        0
    };
    let scope_start_byte = page_boundaries[boundary_idx];

    let scope_start_utf16 = utf16_len(&content[..scope_start_byte]);
    let scope_end_utf16 = utf16_len(&content[..scope_end_byte]);

    Some((scope_start_utf16, scope_end_utf16))
}

/// Resolve `Anchor("text")` scope: find the anchor text before the annotation.
fn resolve_anchor(content: &str, char_start: usize, anchor: &str) -> Option<(usize, usize)> {
    let byte_start = utf16_to_byte(content, char_start);
    let text_before = &content[..byte_start];

    // Find the last occurrence of the anchor text before the annotation
    let pos = text_before.rfind(anchor)?;
    let scope_start_utf16 = utf16_len(&content[..pos]);
    let scope_end_utf16 = utf16_len(&content[..pos + anchor.len()]);

    Some((scope_start_utf16, scope_end_utf16))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Words scope ──

    #[test]
    fn words_1_single_preceding_word() {
        let content = "hello <!-- n: _ | note -->";
        let char_start = 6;
        let result = resolve_scope_range(content, char_start, &Scope::Words(1), "en");
        assert_eq!(result, Some((0, 5))); // "hello"
    }

    #[test]
    fn words_2_two_preceding_words() {
        // "the quick brown fox <!-- ... -->"
        //                ^^^^^^^^^ "brown fox" = offsets 10..19
        let content = "the quick brown fox <!-- n: __ | note -->";
        let char_start = 20; // position of '<'
        let result = resolve_scope_range(content, char_start, &Scope::Words(2), "en");
        assert_eq!(result, Some((10, 19))); // "brown fox"
    }

    #[test]
    fn words_3_three_preceding_words() {
        let content = "the quick brown fox <!-- n: ___ | note -->";
        let char_start = 20;
        let result = resolve_scope_range(content, char_start, &Scope::Words(3), "en");
        assert_eq!(result, Some((4, 19))); // "quick brown fox"
    }

    #[test]
    fn words_more_than_available() {
        // Only 2 words but requesting 5 — should highlight all available
        let content = "brown fox <!-- n: | note -->";
        let char_start = 10;
        let result = resolve_scope_range(content, char_start, &Scope::Words(5), "en");
        assert_eq!(result, Some((0, 9))); // "brown fox"
    }

    #[test]
    fn words_with_cjk() {
        // CJK: 你好 世界 — 2 words, each 2 UTF-16 units
        let content = "你好 世界 <!-- n: __ | note -->";
        let char_start = 5; // 你(1) 好(1) space(1) 世(1) 界(1) = 5 UTF-16 units, then space before <!--
        let result = resolve_scope_range(content, char_start, &Scope::Words(1), "en");
        assert_eq!(result, Some((3, 5))); // "世界"
    }

    #[test]
    fn words_no_preceding_text() {
        let content = "<!-- n: _ | note -->";
        let char_start = 0;
        let result = resolve_scope_range(content, char_start, &Scope::Words(1), "en");
        assert_eq!(result, None);
    }

    #[test]
    fn words_only_whitespace_before() {
        let content = "   <!-- n: _ | note -->";
        let char_start = 3;
        let result = resolve_scope_range(content, char_start, &Scope::Words(1), "en");
        assert_eq!(result, None);
    }

    // ── Sentence scope ──

    #[test]
    fn sentence_single_sentence() {
        let content = "The cat sat on the mat.<!-- n: | note -->";
        let char_start = 23;
        let result = resolve_scope_range(content, char_start, &Scope::Sentence(1), "en");
        assert_eq!(result, Some((0, 23))); // "The cat sat on the mat."
    }

    #[test]
    fn sentence_last_of_multiple_sentences() {
        let content = "The dog ran. The cat sat.<!-- n: | note -->";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, &Scope::Sentence(1), "en");
        // Should highlight "The cat sat." (last sentence)
        assert_eq!(result, Some((13, 25)));
    }

    #[test]
    fn sentence_two_of_multiple() {
        let content = "First one. The dog ran. The cat sat.<!-- n: \\ss | note -->";
        let char_start = 36;
        let result = resolve_scope_range(content, char_start, &Scope::Sentence(2), "en");
        // Should highlight "The dog ran. The cat sat."
        assert_eq!(result, Some((11, 36)));
    }

    #[test]
    fn sentence_more_than_available() {
        let content = "The dog ran. The cat sat.<!-- n: \\sss | note -->";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, &Scope::Sentence(3), "en");
        // Only 2 sentences available — should highlight both
        assert_eq!(result, Some((0, 25)));
    }

    #[test]
    fn sentence_mid_sentence() {
        // Annotation is in the middle of a sentence
        let content = "The dog ran. The cat sat<!-- n: | note --> on the mat.";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, &Scope::Sentence(1), "en");
        // Should highlight "The cat sat" (partial sentence before annotation)
        assert_eq!(result, Some((13, 25)));
    }

    #[test]
    fn sentence_no_preceding_text() {
        let content = "<!-- n: | note -->";
        let char_start = 0;
        let result = resolve_scope_range(content, char_start, &Scope::Sentence(1), "en");
        assert_eq!(result, None);
    }

    // ── Paragraph scope ──

    #[test]
    fn paragraph_1_current_paragraph() {
        let content = "First paragraph.\n\nSecond paragraph text.<!-- n: \\p | note -->";
        // "First paragraph.\n\nSecond paragraph text." = 18 + 22 = 40 chars
        let char_start = 40;
        let result = resolve_scope_range(content, char_start, &Scope::Paragraph(1), "en");
        assert_eq!(result, Some((18, 40))); // "Second paragraph text."
    }

    #[test]
    fn paragraph_2_current_and_preceding() {
        let content = "First para.\n\nSecond para.\n\nThird para.<!-- n: \\pp | note -->";
        let char_start = 38;
        let result = resolve_scope_range(content, char_start, &Scope::Paragraph(2), "en");
        // Should include "Second para.\n\nThird para."
        assert_eq!(result, Some((13, 38)));
    }

    #[test]
    fn paragraph_more_than_available() {
        let content = "Only paragraph.<!-- n: \\ppp | note -->";
        let char_start = 15;
        let result = resolve_scope_range(content, char_start, &Scope::Paragraph(3), "en");
        assert_eq!(result, Some((0, 15))); // "Only paragraph."
    }

    #[test]
    fn paragraph_no_preceding_text() {
        let content = "<!-- n: \\p | note -->";
        let char_start = 0;
        let result = resolve_scope_range(content, char_start, &Scope::Paragraph(1), "en");
        assert_eq!(result, None);
    }

    // ── Page scope ──

    #[test]
    fn page_1_current_page() {
        // \x0C is form feed (page break)
        let content = "Page one.\x0CPage two text.<!-- n: \\f | note -->";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, &Scope::Page(1), "en");
        assert_eq!(result, Some((10, 25))); // "Page two text."
    }

    #[test]
    fn page_2_current_and_preceding() {
        let content = "Page one.\x0CPage two.\x0CPage three.<!-- n: | note -->";
        let char_start = 31;
        let result = resolve_scope_range(content, char_start, &Scope::Page(2), "en");
        assert_eq!(result, Some((10, 31))); // "Page two.\x0CPage three."
    }

    #[test]
    fn page_no_form_feed() {
        // No form feed — treat entire text as one page
        let content = "All one page.<!-- n: \\f | note -->";
        let char_start = 14;
        let result = resolve_scope_range(content, char_start, &Scope::Page(1), "en");
        assert_eq!(result, Some((0, 14)));
    }

    // ── Anchor scope ──

    #[test]
    fn anchor_found() {
        let content = "The term anuttara appears in this text.<!-- n: ^\"anuttara\" | note -->";
        let char_start = 39;
        let result = resolve_scope_range(
            content, char_start,
            &Scope::Anchor("anuttara".to_string()), "en",
        );
        assert_eq!(result, Some((9, 17))); // "anuttara" at offset 9..17
    }

    #[test]
    fn anchor_not_found() {
        let content = "No match here.<!-- n: ^\"missing\" | note -->";
        let char_start = 15;
        let result = resolve_scope_range(
            content, char_start,
            &Scope::Anchor("missing".to_string()), "en",
        );
        assert_eq!(result, None);
    }

    // ── Sentence scope with whitespace normalization ──

    #[test]
    fn sentence_with_double_spaces() {
        // Double spaces (common in LaTeX paste) caused find() to fail because sentenza
        // collapses \s{2,} to a single space during preprocessing.
        let content = "Maximum depth  $d = 5$  and composition.<!-- n: | note -->";
        let ann_start = content.find("<!--").unwrap();
        let result = resolve_scope_range(content, ann_start, &Scope::Sentence(1), "en");
        assert!(result.is_some(), "scope should resolve despite double spaces");
        let (start, end) = result.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, ann_start);
    }

    #[test]
    fn sentence_double_spaces_multi_sentence() {
        let content = "First sentence. Second  has  double  spaces.<!-- n: | note -->";
        let ann_start = content.find("<!--").unwrap();
        let result = resolve_scope_range(content, ann_start, &Scope::Sentence(1), "en");
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        // Should highlight only the second sentence (with its original double spaces)
        assert_eq!(start, 16); // after "First sentence. "
        assert_eq!(end, ann_start);
    }

    // ── ws_flexible_find unit tests ──

    #[test]
    fn ws_flex_exact_match() {
        assert_eq!(ws_flexible_find("hello world", "hello world", 0), Some((0, 11)));
    }

    #[test]
    fn ws_flex_double_space_in_haystack() {
        assert_eq!(ws_flexible_find("hello  world", "hello world", 0), Some((0, 12)));
    }

    #[test]
    fn ws_flex_multiple_double_spaces() {
        assert_eq!(ws_flexible_find("a  b  c", "a b c", 0), Some((0, 7)));
    }

    #[test]
    fn ws_flex_start_offset() {
        assert_eq!(ws_flexible_find("xx hello  world", "hello world", 3), Some((3, 15)));
    }

    #[test]
    fn ws_flex_no_match() {
        assert_eq!(ws_flexible_find("hello world", "goodbye", 0), None);
    }
}
