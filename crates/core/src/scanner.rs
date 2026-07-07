/// A raw annotation comment extracted from the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawComment {
    /// UTF-16 offset of the `<` in `<!---`
    pub char_start: usize,
    /// UTF-16 offset one past the `>` in `--->`
    pub char_end: usize,
    /// The text between `<!---` and `--->`, trimmed of leading/trailing whitespace
    pub inner: String,
    /// The full original source including delimiters
    pub original: String,
}

/// Count UTF-16 code units for a string.
pub(crate) fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

/// Scan a document for triple-dash annotation comments (`<!--- --->`),
/// returning them with UTF-16 offsets. Standard `<!-- -->` comments are ignored.
///
/// Skips:
/// - Comments inside fenced code blocks (``` or ~~~)
pub fn scan_comments(content: &str) -> Vec<RawComment> {
    // Pass 1: find code fence byte ranges to exclude
    let fenced_ranges = find_fenced_ranges(content);

    // Pass 2: find all <!--- ---> comments, filtering out those in fenced ranges
    let mut results = Vec::new();
    let mut search_from = 0usize; // byte offset

    // Build a parallel UTF-16 offset map: for each byte offset of `<!---`,
    // compute the UTF-16 offset. We do this incrementally.
    let mut last_byte = 0usize;
    let mut utf16_acc = 0usize;

    while let Some(rel) = content[search_from..].find("<!---") {
        let open_byte = search_from + rel;

        // Check if this comment start is inside a fenced code block
        if is_in_fenced_range(open_byte, &fenced_ranges) {
            search_from = open_byte + 5;
            continue;
        }

        // Find the closing --->
        let after_open = open_byte + 5;
        if let Some(close_rel) = content[after_open..].find("--->") {
            let close_byte = after_open + close_rel;
            let end_byte = close_byte + 4;

            // Compute UTF-16 offsets incrementally
            utf16_acc += utf16_len(&content[last_byte..open_byte]);
            let comment_utf16_start = utf16_acc;

            let original = &content[open_byte..end_byte];
            let comment_utf16_end = comment_utf16_start + utf16_len(original);

            let inner_raw = &content[after_open..close_byte];
            let inner = inner_raw.trim().to_string();

            results.push(RawComment {
                char_start: comment_utf16_start,
                char_end: comment_utf16_end,
                inner,
                original: original.to_string(),
            });

            // Advance past this comment
            last_byte = open_byte;
            search_from = end_byte;
        } else {
            // No closing --> found; stop
            break;
        }
    }

    results
}

/// A byte range [start, end) representing a fenced code block.
pub(crate) struct FencedRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Find all fenced code block byte ranges in the document.
pub(crate) fn find_fenced_ranges(content: &str) -> Vec<FencedRange> {
    let mut ranges = Vec::new();
    let mut in_fence = false;
    let mut fence_marker = String::new();
    let mut fence_start_byte = 0usize;
    let mut byte_offset = 0usize;

    for line in content.split('\n') {
        let trimmed = line.trim_start();

        if !in_fence {
            if let Some(marker) = detect_fence_open(trimmed) {
                in_fence = true;
                fence_marker = marker;
                fence_start_byte = byte_offset;
            }
        } else if detect_fence_close(trimmed, &fence_marker) {
            let fence_end_byte = byte_offset + line.len();
            ranges.push(FencedRange {
                start: fence_start_byte,
                end: fence_end_byte,
            });
            in_fence = false;
            fence_marker.clear();
        }

        byte_offset += line.len() + 1; // +1 for \n
    }

    // If still in fence at EOF, extend to end
    if in_fence {
        ranges.push(FencedRange {
            start: fence_start_byte,
            end: content.len(),
        });
    }

    ranges
}

fn detect_fence_open(trimmed: &str) -> Option<String> {
    if trimmed.starts_with("```") {
        let fence_len = trimmed.chars().take_while(|&c| c == '`').count();
        Some("`".repeat(fence_len))
    } else if trimmed.starts_with("~~~") {
        let fence_len = trimmed.chars().take_while(|&c| c == '~').count();
        Some("~".repeat(fence_len))
    } else {
        None
    }
}

fn detect_fence_close(trimmed: &str, marker: &str) -> bool {
    if marker.starts_with('`') {
        trimmed.starts_with(marker) && trimmed.trim().chars().all(|c| c == '`')
    } else {
        trimmed.starts_with(marker) && trimmed.trim().chars().all(|c| c == '~')
    }
}

pub(crate) fn is_in_fenced_range(byte_offset: usize, ranges: &[FencedRange]) -> bool {
    ranges.iter().any(|r| byte_offset >= r.start && byte_offset < r.end)
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Basic scanning ===

    #[test]
    fn single_line_comment() {
        let doc = "hello <!--- world ---> end";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "world");
        assert_eq!(comments[0].original, "<!--- world --->");
        assert_eq!(comments[0].char_start, 6);
        assert_eq!(comments[0].char_end, 22);
    }

    #[test]
    fn multi_line_comment() {
        let doc = "before\n<!---\nfoo\nbar\n--->\nafter";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "foo\nbar");
        assert_eq!(comments[0].original, "<!---\nfoo\nbar\n--->");
        assert_eq!(comments[0].char_start, 7); // after "before\n"
    }

    #[test]
    fn multiple_comments() {
        let doc = "<!--- a ---> text <!--- b --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].inner, "a");
        assert_eq!(comments[1].inner, "b");
    }

    #[test]
    fn empty_document() {
        assert_eq!(scan_comments("").len(), 0);
    }

    #[test]
    fn no_comments() {
        assert_eq!(scan_comments("just regular text").len(), 0);
    }

    #[test]
    fn empty_comment() {
        let comments = scan_comments("<!--- --->");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "");
    }

    #[test]
    fn comment_no_spaces() {
        let comments = scan_comments("<!---text--->");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "text");
    }

    // === Code fence skipping ===

    #[test]
    fn skip_comment_in_backtick_fence() {
        let doc = "before\n```\n<!--- skip --->\n```\nafter <!--- keep --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "keep");
    }

    #[test]
    fn skip_comment_in_tilde_fence() {
        let doc = "~~~\n<!--- skip --->\n~~~\n<!--- keep --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "keep");
    }

    #[test]
    fn skip_comment_in_four_backtick_fence() {
        let doc = "````\n```\n<!--- skip --->\n```\n````\n<!--- keep --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "keep");
    }

    #[test]
    fn fence_with_language_tag() {
        let doc = "```rust\n<!--- skip --->\n```\n<!--- keep --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "keep");
    }

    // === no raw: opt-out (standard comments are the opt-out) ===

    #[test]
    fn emits_former_raw_comment() {
        let doc = "<!--- raw: this is raw ---> <!--- keep --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].inner, "raw: this is raw");
        assert_eq!(comments[1].inner, "keep");
    }

    // === UTF-16 offsets ===

    #[test]
    fn utf16_offsets_ascii() {
        let doc = "ab <!--- c ---> de";
        let comments = scan_comments(doc);
        assert_eq!(comments[0].char_start, 3);
        assert_eq!(comments[0].char_end, 15);
    }

    #[test]
    fn utf16_offsets_cjk() {
        // CJK chars: 1 UTF-16 code unit each
        // "<!--- note --->" = 15 chars
        let doc = "你好<!--- note --->";
        let comments = scan_comments(doc);
        assert_eq!(comments[0].char_start, 2);
        assert_eq!(comments[0].char_end, 17); // 2 + 15
    }

    #[test]
    fn utf16_offsets_emoji() {
        // 🎉 = U+1F389 = 2 UTF-16 code units
        // "<!--- hi --->" = 13 chars
        let doc = "🎉<!--- hi --->";
        let comments = scan_comments(doc);
        assert_eq!(comments[0].char_start, 2);
        assert_eq!(comments[0].char_end, 15); // 2 + 13
    }

    #[test]
    fn utf16_offsets_mixed() {
        // "a你🎉" = 1 + 1 + 2 = 4 UTF-16 units
        let doc = "a你🎉<!--- x --->";
        let comments = scan_comments(doc);
        assert_eq!(comments[0].char_start, 4);
    }

    // === Edge cases ===

    #[test]
    fn unclosed_comment() {
        let doc = "<!--- no end";
        assert_eq!(scan_comments(doc).len(), 0);
    }

    #[test]
    fn comment_at_document_start() {
        let doc = "<!--- first --->";
        let comments = scan_comments(doc);
        assert_eq!(comments[0].char_start, 0);
    }

    #[test]
    fn adjacent_comments() {
        let doc = "<!--- a ---><!--- b --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].inner, "a");
        assert_eq!(comments[1].inner, "b");
    }

    #[test]
    fn ignores_standard_double_dash_comment() {
        let doc = "<!-- world -->";
        assert_eq!(scan_comments(doc).len(), 0);
    }

    #[test]
    fn standard_comment_adjacent_to_annotation() {
        let doc = "<!-- std --><!--- keep --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "keep");
        assert_eq!(comments[0].char_start, 12);
    }

    #[test]
    fn four_dash_opener_leaks_into_inner() {
        // Opener consumes exactly 5 chars; the 4th dash falls into inner
        let doc = "<!---- x --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].inner, "- x");
    }

    #[test]
    fn triple_open_double_close_unclosed() {
        // No ---> closer exists; treated as unclosed
        let doc = "<!--- a -->";
        assert_eq!(scan_comments(doc).len(), 0);
    }

    #[test]
    fn comment_after_multiline() {
        let doc = "<!---\nblock\n--->\n<!--- inline --->";
        let comments = scan_comments(doc);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].inner, "block");
        assert_eq!(comments[1].inner, "inline");
    }
}
