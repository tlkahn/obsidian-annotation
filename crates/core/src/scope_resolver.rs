use crate::scanner::utf16_len;
use crate::types::Scope;

/// How a symmetric scope extends from the annotation position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResolutionMode {
    /// Scope extends only backward from the annotation (spec default).
    #[default]
    Backward,
    /// Scope extends both backward and forward by the same count.
    /// Used in specific UI contexts.
    Bidirectional,
}

impl ResolutionMode {
    /// Parse a mode string from the FFI boundary. Unknown values are an
    /// error (None) rather than a silent backward fallback.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "backward" => Some(Self::Backward),
            "bidirectional" => Some(Self::Bidirectional),
            _ => None,
        }
    }
}

/// Resolve the text range that an annotation's scope refers to.
///
/// Given the full document content, the annotation's UTF-16 start and end
/// offsets, its scope, and the resolution mode, returns
/// `Some((scope_start, scope_end))` in UTF-16 offsets, or `None` if the
/// scope cannot be resolved. `char_end` is only consulted by scopes that
/// walk forward past the annotation's own text (asymmetric scopes and
/// bidirectional mode).
pub fn resolve_scope_range(
    content: &str,
    char_start: usize,
    char_end: usize,
    scope: &Scope,
    lang: &str,
    mode: ResolutionMode,
) -> Option<(usize, usize)> {
    // Bidirectional mode extends symmetric scopes forward by the same count
    if mode == ResolutionMode::Bidirectional {
        let asym = match scope {
            Scope::Words(n) => Some(Scope::AsymWords(*n, *n)),
            Scope::Sentence(n) => Some(Scope::AsymSentence(*n, *n)),
            Scope::Paragraph(n) => Some(Scope::AsymParagraph(*n, *n)),
            Scope::Page(n) => Some(Scope::AsymPage(*n, *n)),
            _ => None,
        };
        if let Some(asym) = asym {
            return resolve_scope_range(
                content,
                char_start,
                char_end,
                &asym,
                lang,
                ResolutionMode::Backward,
            );
        }
    }
    match scope {
        Scope::Words(n) => resolve_words(content, char_start, *n as usize),
        Scope::Sentence(n) => resolve_sentence(content, char_start, *n as usize, lang),
        Scope::Paragraph(n) => resolve_paragraph(content, char_start, *n as usize),
        Scope::Page(n) => resolve_page(content, char_start, *n as usize),
        Scope::Anchor(text) => resolve_anchor(content, char_start, text),
        Scope::Section => resolve_section(content, char_start),
        Scope::Document => resolve_document(content),
        Scope::AsymWords(n, m) => resolve_asym(
            content, char_start, char_end, *n as usize, *m as usize,
            words_before, words_after,
        ),
        Scope::AsymSentence(n, m) => resolve_asym(
            content, char_start, char_end, *n as usize, *m as usize,
            |c, b, k| sentences_before(c, b, k, lang),
            |c, b, k| sentences_after(c, b, k, lang),
        ),
        Scope::AsymParagraph(n, m) => resolve_asym(
            content, char_start, char_end, *n as usize, *m as usize,
            paragraphs_before, paragraphs_after,
        ),
        Scope::AsymPage(n, m) => resolve_asym(
            content, char_start, char_end, *n as usize, *m as usize,
            pages_before, pages_after,
        ),
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

/// Byte range of the N words preceding `byte_start`, clamped to the
/// document start when fewer are available.
fn words_before(content: &str, byte_start: usize, n: usize) -> Option<(usize, usize)> {
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

    Some((scope_start_byte, scope_end_byte))
}

/// Byte offset after skipping leading whitespace from `byte_end`.
fn skip_leading_ws(content: &str, byte_end: usize) -> usize {
    let after = &content[byte_end..];
    byte_end + (after.len() - after.trim_start().len())
}

/// Byte range of the M words following `byte_end`, clamped to the document
/// end when fewer are available.
fn words_after(content: &str, byte_end: usize, m: usize) -> Option<(usize, usize)> {
    let scope_start_byte = skip_leading_ws(content, byte_end);
    let text = &content[scope_start_byte..];
    if text.trim_end().is_empty() {
        return None;
    }

    let mut words_found = 0;
    let mut scope_end_byte = scope_start_byte + text.trim_end().len();
    let mut in_word = false;

    for (i, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if in_word {
                words_found += 1;
                if words_found >= m {
                    scope_end_byte = scope_start_byte + i;
                    break;
                }
                in_word = false;
            }
        } else {
            in_word = true;
        }
    }

    Some((scope_start_byte, scope_end_byte))
}

/// Combine optional backward and forward byte ranges into a UTF-16 range.
fn combine_ranges(
    content: &str,
    back: Option<(usize, usize)>,
    fwd: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let (start, end) = match (back, fwd) {
        (Some((s, _)), Some((_, e))) => (s, e),
        (Some((s, e)), None) => (s, e),
        (None, Some((s, e))) => (s, e),
        (None, None) => return None,
    };
    Some((utf16_len(&content[..start]), utf16_len(&content[..end])))
}

/// Resolve `Words(n)` scope: find the N preceding words before `char_start`.
fn resolve_words(content: &str, char_start: usize, n: usize) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    combine_ranges(content, words_before(content, byte_start, n), None)
}

/// Shared asymmetric combinator: N units backward from the annotation start,
/// M units forward from its end, via the given per-unit byte-range walkers.
fn resolve_asym(
    content: &str,
    char_start: usize,
    char_end: usize,
    n: usize,
    m: usize,
    before: impl Fn(&str, usize, usize) -> Option<(usize, usize)>,
    after: impl Fn(&str, usize, usize) -> Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    if n == 0 && m == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    let byte_end = utf16_to_byte(content, char_end);
    let back = if n > 0 { before(content, byte_start, n) } else { None };
    let fwd = if m > 0 { after(content, byte_end, m) } else { None };
    combine_ranges(content, back, fwd)
}

/// Blank every `<!---` … `--->` annotation comment in `text`, replacing each span
/// with spaces byte-for-byte. Length-preserving: `<!---` and `--->` are pure ASCII
/// (5 and 4 bytes), so every cut lands on a char boundary and all byte offsets in
/// the result match the original text. An unterminated `<!---` blanks to the end
/// of the string. Standard `<!-- -->` comments are left untouched.
///
/// Rationale: sentenza's preprocessing normalizes dash runs to em dashes (and
/// collapses comma/space runs), so an annotation comment inside a paragraph comes
/// back mangled (`<!— … —>`) and can never be located in the original text —
/// blanking it out before splitting sidesteps the mismatch entirely. Fenced code
/// blocks are deliberately ignored here: this operates on small paragraph slices,
/// and the annotation set itself comes from `scanner::scan_comments`, which
/// already respects fences.
fn blank_comments(text: &str) -> String {
    let mut out = text.to_string();
    let mut search_from = 0;
    while let Some(rel) = out[search_from..].find("<!---") {
        let start = search_from + rel;
        let end = match out[start..].find("--->") {
            Some(e) => start + e + "--->".len(),
            None => out.len(),
        };
        // SAFETY of offsets: start/end are found via ASCII delimiters, so both
        // lie on char boundaries; the replacement is the same number of bytes.
        out.replace_range(start..end, &" ".repeat(end - start));
        search_from = end;
    }
    out
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

/// Locate every sentence in `paragraph` sequentially: each search starts at
/// the previous sentence's end, so duplicated sentence text cannot re-match
/// an earlier occurrence. Returns byte ranges relative to `paragraph`.
fn locate_sentences(paragraph: &str, sentences: &[String]) -> Option<Vec<(usize, usize)>> {
    let mut positions = Vec::with_capacity(sentences.len());
    let mut cursor = 0;
    for sentence in sentences {
        let (start, end) = ws_flexible_find(paragraph, sentence, cursor)?;
        positions.push((start, end));
        cursor = end;
    }
    Some(positions)
}

/// Byte range of the last N sentences before `byte_start` using sentenza.
/// Extracts the current paragraph (up to `byte_start`) and splits into sentences.
fn sentences_before(content: &str, byte_start: usize, n: usize, lang: &str) -> Option<(usize, usize)> {
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

    // Blank annotation comments first: sentenza's preprocessing would mangle
    // them beyond recognition (see `blank_comments`). Blanking is
    // length-preserving, so offsets in the blanked text equal offsets in the
    // original, and `ws_flexible_find` tolerates the whitespace runs it leaves.
    let blanked = blank_comments(paragraph);
    let sentences = sentenza::split_sentences(&blanked, lang);
    if sentences.is_empty() {
        return None;
    }

    // Take the last n sentences (or all if fewer available). Sequential
    // location handles both sentenza's whitespace normalization and
    // duplicated sentence text.
    let take = n.min(sentences.len());
    let positions = locate_sentences(&blanked, &sentences)?;
    let first_start = positions[positions.len() - take].0;
    let last_end = positions[positions.len() - 1].1;

    let scope_start_byte = para_byte_start + first_start;
    let scope_end_byte = (para_byte_start + last_end).min(trimmed.len());

    Some((scope_start_byte, scope_end_byte))
}

/// Byte range of the first M sentences following `byte_end` (starting with
/// the remainder of the current sentence), limited to the current paragraph
/// and clamped to what is available. An annotation at the end of its
/// paragraph contributes nothing forward — the `\n\n` cut applies before
/// any whitespace is skipped, so resolution never jumps into the next
/// paragraph.
fn sentences_after(content: &str, byte_end: usize, m: usize, lang: &str) -> Option<(usize, usize)> {
    let after = &content[byte_end..];

    // Limit to the current paragraph FIRST, then trim within it
    let para_len = after.find("\n\n").unwrap_or(after.len());
    let para_slice = &after[..para_len];
    let start = byte_end + (para_slice.len() - para_slice.trim_start().len());
    let paragraph = para_slice.trim();
    if paragraph.is_empty() {
        return None;
    }

    let sentences = sentenza::split_sentences(paragraph, lang);
    if sentences.is_empty() {
        return None;
    }

    let take = m.min(sentences.len());
    let positions = locate_sentences(paragraph, &sentences)?;
    let first_start = positions[0].0;
    let last_end = positions[take - 1].1;

    Some((start + first_start, start + last_end))
}

/// Resolve `Sentence(n)` scope: find the last N sentences before `char_start`.
fn resolve_sentence(content: &str, char_start: usize, n: usize, lang: &str) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    combine_ranges(content, sentences_before(content, byte_start, n, lang), None)
}

/// Byte range of the current paragraph + n-1 preceding paragraphs before
/// `byte_start`. Paragraphs are delimited by double newlines (`\n\n`).
fn paragraphs_before(content: &str, byte_start: usize, n: usize) -> Option<(usize, usize)> {
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

    Some((scope_start_byte, scope_end_byte))
}

/// Byte range of the M paragraphs following `byte_end` (starting with the
/// remainder of the current paragraph), clamped to the document end.
fn paragraphs_after(content: &str, byte_end: usize, m: usize) -> Option<(usize, usize)> {
    let start = skip_leading_ws(content, byte_end);
    let text = &content[start..];
    if text.trim_end().is_empty() {
        return None;
    }

    let mut boundaries_found = 0;
    let mut end = start + text.trim_end().len();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\n' && bytes[i + 1] == b'\n' {
            boundaries_found += 1;
            if boundaries_found >= m {
                end = start + text[..i].trim_end().len();
                break;
            }
            let mut e = i + 2;
            while e < bytes.len() && bytes[e] == b'\n' {
                e += 1;
            }
            i = e;
        } else {
            i += 1;
        }
    }

    Some((start, end))
}

/// Resolve `Paragraph(n)` scope: find the current paragraph + n-1 preceding paragraphs.
/// Paragraphs are delimited by double newlines (`\n\n`).
fn resolve_paragraph(content: &str, char_start: usize, n: usize) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    combine_ranges(content, paragraphs_before(content, byte_start, n), None)
}

/// Byte range of the current page + n-1 preceding pages before `byte_start`.
/// Pages are delimited by form feed (`\x0C`) characters.
///
/// Intentional fall-through: `\x0C` is Unicode whitespace, so an annotation
/// sitting alone at the very top of a page has its empty current page
/// trimmed away and resolves to the preceding page — the same behavior the
/// spec's `2\p1` example relies on for paragraphs (an annotation in an
/// otherwise-empty unit falls through to the adjacent unit).
fn pages_before(content: &str, byte_start: usize, n: usize) -> Option<(usize, usize)> {
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

    Some((scope_start_byte, scope_end_byte))
}

/// Byte range of the M pages following `byte_end` (starting with the
/// remainder of the current page), clamped to the document end.
fn pages_after(content: &str, byte_end: usize, m: usize) -> Option<(usize, usize)> {
    let start = skip_leading_ws(content, byte_end);
    let text = &content[start..];
    if text.trim_end().is_empty() {
        return None;
    }

    let mut boundaries_found = 0;
    let mut end = start + text.trim_end().len();
    for (i, b) in text.bytes().enumerate() {
        if b == b'\x0C' {
            boundaries_found += 1;
            if boundaries_found >= m {
                end = start + text[..i].trim_end().len();
                break;
            }
        }
    }

    Some((start, end))
}

/// Resolve `Page(n)` scope: find pages delimited by form feed (`\x0C`) characters.
fn resolve_page(content: &str, char_start: usize, n: usize) -> Option<(usize, usize)> {
    if n == 0 {
        return None;
    }
    let byte_start = utf16_to_byte(content, char_start);
    combine_ranges(content, pages_before(content, byte_start, n), None)
}

/// Resolve `Document` scope: the entire file.
fn resolve_document(content: &str) -> Option<(usize, usize)> {
    if content.is_empty() {
        return None;
    }
    Some((0, utf16_len(content)))
}

/// The ATX heading level of a markdown line (1-6), or None. Per CommonMark,
/// at most 3 leading spaces are allowed; 4+ spaces or a tab make the line
/// indented code, not a heading.
fn heading_level(line: &str) -> Option<u8> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let trimmed = &line[indent..];
    if trimmed.starts_with('\t') {
        return None;
    }
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if (1..=6).contains(&hashes) {
        let rest = &trimmed[hashes..];
        if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
            return Some(hashes as u8);
        }
    }
    None
}

/// Resolve `Section` scope: from the nearest heading at-or-before the
/// annotation to just before the next heading of equal or higher level
/// (trimmed of trailing whitespace), or EOF. With no preceding heading the
/// section clamps to the document start and any heading terminates it.
/// Headings inside fenced code blocks are ignored.
fn resolve_section(content: &str, char_start: usize) -> Option<(usize, usize)> {
    let byte_start = utf16_to_byte(content, char_start);
    let fenced = crate::scanner::find_fenced_ranges(content);

    // Collect heading lines as (line_start_byte, level)
    let mut headings: Vec<(usize, u8)> = Vec::new();
    let mut offset = 0usize;
    for line in content.split('\n') {
        if !crate::scanner::is_in_fenced_range(offset, &fenced) {
            if let Some(level) = heading_level(line) {
                headings.push((offset, level));
            }
        }
        offset += line.len() + 1;
    }

    // Nearest heading at-or-before the annotation; without one, the section
    // is the document preamble (level 7 = terminated by any heading).
    let (sec_start, level) = headings
        .iter()
        .rev()
        .find(|(pos, _)| *pos <= byte_start)
        .copied()
        .unwrap_or((0, 7));

    let sec_end = headings
        .iter()
        .find(|(pos, lvl)| *pos > sec_start && *lvl <= level)
        .map(|&(pos, _)| pos)
        .unwrap_or(content.len());

    let end_trimmed = content[..sec_end].trim_end().len();
    if end_trimmed <= sec_start {
        return None;
    }
    Some((
        utf16_len(&content[..sec_start]),
        utf16_len(&content[..end_trimmed]),
    ))
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
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Words(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 5))); // "hello"
    }

    #[test]
    fn words_2_two_preceding_words() {
        // "the quick brown fox <!-- ... -->"
        //                ^^^^^^^^^ "brown fox" = offsets 10..19
        let content = "the quick brown fox <!-- n: __ | note -->";
        let char_start = 20; // position of '<'
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Words(2), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((10, 19))); // "brown fox"
    }

    #[test]
    fn words_3_three_preceding_words() {
        let content = "the quick brown fox <!-- n: ___ | note -->";
        let char_start = 20;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Words(3), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((4, 19))); // "quick brown fox"
    }

    #[test]
    fn words_more_than_available() {
        // Only 2 words but requesting 5 — should highlight all available
        let content = "brown fox <!-- n: | note -->";
        let char_start = 10;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Words(5), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 9))); // "brown fox"
    }

    #[test]
    fn words_with_cjk() {
        // CJK: 你好 世界 — 2 words, each 2 UTF-16 units
        let content = "你好 世界 <!-- n: __ | note -->";
        let char_start = 5; // 你(1) 好(1) space(1) 世(1) 界(1) = 5 UTF-16 units, then space before <!--
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Words(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((3, 5))); // "世界"
    }

    #[test]
    fn words_no_preceding_text() {
        let content = "<!-- n: _ | note -->";
        let char_start = 0;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Words(1), "en", ResolutionMode::Backward);
        assert_eq!(result, None);
    }

    #[test]
    fn words_only_whitespace_before() {
        let content = "   <!-- n: _ | note -->";
        let char_start = 3;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Words(1), "en", ResolutionMode::Backward);
        assert_eq!(result, None);
    }

    // ── Sentence scope ──

    #[test]
    fn sentence_single_sentence() {
        let content = "The cat sat on the mat.<!-- n: | note -->";
        let char_start = 23;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Sentence(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 23))); // "The cat sat on the mat."
    }

    #[test]
    fn sentence_last_of_multiple_sentences() {
        let content = "The dog ran. The cat sat.<!-- n: | note -->";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Sentence(1), "en", ResolutionMode::Backward);
        // Should highlight "The cat sat." (last sentence)
        assert_eq!(result, Some((13, 25)));
    }

    #[test]
    fn sentence_two_of_multiple() {
        let content = "First one. The dog ran. The cat sat.<!-- n: \\ss | note -->";
        let char_start = 36;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Sentence(2), "en", ResolutionMode::Backward);
        // Should highlight "The dog ran. The cat sat."
        assert_eq!(result, Some((11, 36)));
    }

    #[test]
    fn sentence_more_than_available() {
        let content = "The dog ran. The cat sat.<!-- n: \\sss | note -->";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Sentence(3), "en", ResolutionMode::Backward);
        // Only 2 sentences available — should highlight both
        assert_eq!(result, Some((0, 25)));
    }

    #[test]
    fn sentence_mid_sentence() {
        // Annotation is in the middle of a sentence
        let content = "The dog ran. The cat sat<!-- n: | note --> on the mat.";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Sentence(1), "en", ResolutionMode::Backward);
        // Should highlight "The cat sat" (partial sentence before annotation)
        assert_eq!(result, Some((13, 25)));
    }

    #[test]
    fn sentence_no_preceding_text() {
        let content = "<!-- n: | note -->";
        let char_start = 0;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Sentence(1), "en", ResolutionMode::Backward);
        assert_eq!(result, None);
    }

    // ── Paragraph scope ──

    #[test]
    fn paragraph_1_current_paragraph() {
        let content = "First paragraph.\n\nSecond paragraph text.<!-- n: \\p | note -->";
        // "First paragraph.\n\nSecond paragraph text." = 18 + 22 = 40 chars
        let char_start = 40;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Paragraph(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((18, 40))); // "Second paragraph text."
    }

    #[test]
    fn paragraph_2_current_and_preceding() {
        let content = "First para.\n\nSecond para.\n\nThird para.<!-- n: \\pp | note -->";
        let char_start = 38;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Paragraph(2), "en", ResolutionMode::Backward);
        // Should include "Second para.\n\nThird para."
        assert_eq!(result, Some((13, 38)));
    }

    #[test]
    fn paragraph_more_than_available() {
        let content = "Only paragraph.<!-- n: \\ppp | note -->";
        let char_start = 15;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Paragraph(3), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 15))); // "Only paragraph."
    }

    #[test]
    fn paragraph_no_preceding_text() {
        let content = "<!-- n: \\p | note -->";
        let char_start = 0;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Paragraph(1), "en", ResolutionMode::Backward);
        assert_eq!(result, None);
    }

    // ── Page scope ──

    #[test]
    fn page_1_current_page() {
        // \x0C is form feed (page break)
        let content = "Page one.\x0CPage two text.<!-- n: \\f | note -->";
        let char_start = 25;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Page(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((10, 25))); // "Page two text."
    }

    #[test]
    fn page_2_current_and_preceding() {
        let content = "Page one.\x0CPage two.\x0CPage three.<!-- n: | note -->";
        let char_start = 31;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Page(2), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((10, 31))); // "Page two.\x0CPage three."
    }

    #[test]
    fn page_no_form_feed() {
        // No form feed — treat entire text as one page
        let content = "All one page.<!-- n: \\f | note -->";
        let char_start = 14;
        let result = resolve_scope_range(content, char_start, char_start, &Scope::Page(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 14)));
    }

    // ── Page/paragraph boundary fall-through (intentional, locked) ──

    #[test]
    fn page_annotation_at_top_of_page_falls_through_to_previous() {
        // \x0C is whitespace: an annotation alone at the top of page 2 has an
        // empty current page, so \f falls through to page 1 — mirroring the
        // paragraph behavior the spec's 2\p1 example relies on.
        let content = "One.\x0C<!--- n: \\f | x --->";
        let ann = content.find("<!---").unwrap();
        let result = resolve_scope_range(content, ann, content.len(), &Scope::Page(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 4))); // "One."
    }

    #[test]
    fn page_forward_just_before_form_feed_falls_through_to_next() {
        let content = "One.<!--- n: 0\\f1 | x --->\x0CTwo.\x0CThree.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\f1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymPage(0, 1), "en", ResolutionMode::Backward);
        let start = content.find("Two.").unwrap();
        assert_eq!(result, Some((start, start + 4))); // next page, not beyond
    }

    #[test]
    fn paragraph_annotation_own_paragraph_falls_through_to_previous() {
        let content = "Para A.\n\n<!--- n: \\p | x --->";
        let ann = content.find("<!---").unwrap();
        let result = resolve_scope_range(content, ann, content.len(), &Scope::Paragraph(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 7))); // "Para A."
    }

    // ── Anchor scope ──

    #[test]
    fn anchor_found() {
        let content = "The term anuttara appears in this text.<!-- n: ^\"anuttara\" | note -->";
        let char_start = 39;
        let result = resolve_scope_range(
            content, char_start, char_start,
            &Scope::Anchor("anuttara".to_string()), "en", ResolutionMode::Backward,
        );
        assert_eq!(result, Some((9, 17))); // "anuttara" at offset 9..17
    }

    #[test]
    fn anchor_not_found() {
        let content = "No match here.<!-- n: ^\"missing\" | note -->";
        let char_start = 15;
        let result = resolve_scope_range(
            content, char_start, char_start,
            &Scope::Anchor("missing".to_string()), "en", ResolutionMode::Backward,
        );
        assert_eq!(result, None);
    }

    // ── Document scope ──

    #[test]
    fn document_whole_file() {
        let content = "First paragraph.\n\nSecond paragraph.<!--- n: \\d | note --->\n\nThird.";
        let ann = content.find("<!---").unwrap();
        let result = resolve_scope_range(content, ann, ann + 22, &Scope::Document, "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, content.len())));
    }

    #[test]
    fn document_utf16_cjk() {
        // 你好世界 = 4 UTF-16 units; total = 4 + 22 comment chars
        let content = "你好世界<!--- llm \\d | 总结 --->";
        let result = resolve_scope_range(content, 4, 25, &Scope::Document, "en", ResolutionMode::Backward);
        let expected_len = content.chars().map(|c| c.len_utf16()).sum::<usize>();
        assert_eq!(result, Some((0, expected_len)));
    }

    // ── Section scope ──

    #[test]
    fn section_basic() {
        let content = "## Methods\n\nSome methodology text here.\n\n<!--- n: \\h | note --->\n\n## Results\n\nMore.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: \\h | note --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Section, "en", ResolutionMode::Backward);
        // From the start of "## Methods" up to (trimmed) just before "## Results"
        assert_eq!(result, Some((0, ann_end)));
    }

    #[test]
    fn section_nearest_subheading() {
        let content = "## Methods\n\n### Detail\n\nDetail text.<!--- n: \\h | x --->\n\n## Results";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: \\h | x --->".len();
        let start = content.find("### Detail").unwrap();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Section, "en", ResolutionMode::Backward);
        assert_eq!(result, Some((start, ann_end)));
    }

    #[test]
    fn section_spans_lower_level_headings() {
        let content = "# Top\n\nIntro.<!--- n: \\h | x --->\n\n## Sub\n\nSub text.\n\n# Next";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: \\h | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Section, "en", ResolutionMode::Backward);
        // Section of "# Top" runs through "## Sub" (lower level) up to "# Next"
        let expected_end = content.find("\n\n# Next").unwrap();
        assert_eq!(result, Some((0, expected_end)));
    }

    #[test]
    fn section_no_preceding_heading_clamps_to_doc_start() {
        let content = "Preamble text.<!--- n: \\h | x --->\n\n# First\n\nBody.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: \\h | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Section, "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, ann_end)));
    }

    #[test]
    fn section_extends_to_eof() {
        let content = "## Last\n\nFinal text.<!--- n: \\h | x --->\n";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: \\h | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Section, "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, ann_end)));
    }

    #[test]
    fn section_ignores_heading_in_code_fence() {
        let content = "## Real\n\ntext<!--- n: \\h | x --->\n\n```\n# fenced\n```\n\n## Next";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: \\h | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Section, "en", ResolutionMode::Backward);
        // Section ends just before "## Next" (trimmed), running through the fence
        let expected_end = content.rfind("\n\n## Next").unwrap();
        assert_eq!(result, Some((0, expected_end)));
    }

    #[test]
    fn section_ignores_indented_code_hash_line() {
        // 4-space-indented lines are code, not ATX headings (CommonMark)
        let content = "## Real\n\ntext<!--- n: \\h | x --->\n\n    # indented code comment\n\nmore text\n\n## Next";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: \\h | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Section, "en", ResolutionMode::Backward);
        let expected_end = content.rfind("\n\n## Next").unwrap();
        assert_eq!(result, Some((0, expected_end)));
    }

    #[test]
    fn heading_up_to_three_spaces_indent_ok() {
        assert_eq!(heading_level("   ## ok"), Some(2));
        assert_eq!(heading_level("    ## code"), None);
        assert_eq!(heading_level("\t# tab code"), None);
        assert_eq!(heading_level("# plain"), Some(1));
    }

    #[test]
    fn section_utf16_cjk() {
        let content = "## 标题\n\n你好世界<!--- n: \\h | 注 --->\n\n## 下节";
        let ann_utf16: usize = content[..content.find("<!---").unwrap()]
            .chars().map(|c| c.len_utf16()).sum();
        let comment_utf16: usize = "<!--- n: \\h | 注 --->".chars().map(|c| c.len_utf16()).sum();
        let result = resolve_scope_range(content, ann_utf16, ann_utf16 + comment_utf16, &Scope::Section, "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, ann_utf16 + comment_utf16)));
    }

    // ── Forward sentence stays in the current paragraph ──

    #[test]
    fn forward_sentence_at_paragraph_end_contributes_nothing() {
        let content = "A one. <!--- n: 0\\s1 | x --->\n\nNext para sentence.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\s1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(0, 1), "en", ResolutionMode::Backward);
        assert_eq!(result, None);
    }

    #[test]
    fn asym_sentence_at_paragraph_end_backward_only() {
        let content = "A one. <!--- n: 1\\s1 | x --->\n\nNext para sentence.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 1\\s1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(1, 1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 6))); // "A one." — forward side empty
    }

    // ── Duplicated sentence text (sequential location) ──

    #[test]
    fn duplicate_sentences_backward() {
        let content = "Stop now. Stop now.<!--- n: \\ss | x --->";
        let ann = content.find("<!---").unwrap();
        let result = resolve_scope_range(content, ann, content.len(), &Scope::Sentence(2), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 19))); // both occurrences, not a re-match of the first
    }

    #[test]
    fn duplicate_sentences_forward() {
        let content = "Intro. <!--- n: 0\\s2 | x ---> Stop now. Stop now.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\s2 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(0, 2), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((30, 49)));
    }

    #[test]
    fn duplicate_sentence_backward_single() {
        // \s over a paragraph whose last sentence duplicates an earlier one
        let content = "Stop now. Middle bit. Stop now.<!--- n: \\s | x --->";
        let ann = content.find("<!---").unwrap();
        let result = resolve_scope_range(content, ann, content.len(), &Scope::Sentence(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((22, 31))); // the second "Stop now.", not the first
    }

    // ── ResolutionMode parsing ──

    #[test]
    fn resolution_mode_from_str_strict() {
        assert_eq!(ResolutionMode::from_str("backward"), Some(ResolutionMode::Backward));
        assert_eq!(ResolutionMode::from_str("bidirectional"), Some(ResolutionMode::Bidirectional));
        assert_eq!(ResolutionMode::from_str("Bidirectional"), None);
        assert_eq!(ResolutionMode::from_str("bidi"), None);
        assert_eq!(ResolutionMode::from_str(""), None);
    }

    // ── Bidirectional mode ──

    #[test]
    fn bidirectional_words() {
        let content = "alpha beta <!--- x ---> gamma delta";
        let result = resolve_scope_range(content, 11, 23, &Scope::Words(1), "en", ResolutionMode::Bidirectional);
        assert_eq!(result, Some((6, 29))); // "beta" + "gamma"
    }

    #[test]
    fn bidirectional_equals_symmetric_asym() {
        let content = "one two three <!--- x ---> four five six";
        let bidi = resolve_scope_range(content, 14, 26, &Scope::Words(2), "en", ResolutionMode::Bidirectional);
        let asym = resolve_scope_range(content, 14, 26, &Scope::AsymWords(2, 2), "en", ResolutionMode::Backward);
        assert_eq!(bidi, asym);
        assert!(bidi.is_some());
    }

    #[test]
    fn bidirectional_paragraph() {
        let content = "Para A.\n\nPara B.\n\n<!--- x --->\n\nPara C.\n\nPara D.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- x --->".len();
        let bidi = resolve_scope_range(content, ann, ann_end, &Scope::Paragraph(1), "en", ResolutionMode::Bidirectional);
        let b_start = content.find("Para B.").unwrap();
        let c_end = content.find("Para C.").unwrap() + "Para C.".len();
        assert_eq!(bidi, Some((b_start, c_end)));
    }

    #[test]
    fn bidirectional_sentence() {
        let content = "First one. Second two. <!--- x ---> Third three. Fourth four.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- x --->".len();
        let bidi = resolve_scope_range(content, ann, ann_end, &Scope::Sentence(1), "en", ResolutionMode::Bidirectional);
        let asym = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(1, 1), "en", ResolutionMode::Backward);
        assert_eq!(bidi, asym);
        let back_start = content.find("Second two.").unwrap();
        let fwd_end = content.find("Third three.").unwrap() + "Third three.".len();
        assert_eq!(bidi, Some((back_start, fwd_end)));
    }

    #[test]
    fn bidirectional_page() {
        let content = "One.\x0CTwo.<!--- x ---> more.\x0CThree.\x0CFour.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- x --->".len();
        let bidi = resolve_scope_range(content, ann, ann_end, &Scope::Page(1), "en", ResolutionMode::Bidirectional);
        // backward: "Two." on the current page; forward: " more." to the next form feed
        let back_start = content.find("Two.").unwrap();
        let fwd_end = content.find(" more.").unwrap() + " more.".len();
        assert_eq!(bidi, Some((back_start, fwd_end)));
    }

    #[test]
    fn asym_page_forward_clamps_to_eof() {
        let content = "One.\x0CTwo.<!--- n: 0\\f9 | x ---> rest of page.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\f9 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymPage(0, 9), "en", ResolutionMode::Backward);
        let start = content.find("rest of page.").unwrap();
        assert_eq!(result, Some((start, content.len())));
    }

    #[test]
    fn asym_words_backward_clamps_to_doc_start() {
        // 2 words available, 9 requested — clamp to document start
        let content = "alpha beta <!--- n: 9_1 | x ---> gamma";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 9_1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymWords(9, 1), "en", ResolutionMode::Backward);
        let fwd_end = content.find("gamma").unwrap() + "gamma".len();
        assert_eq!(result, Some((0, fwd_end)));
    }

    #[test]
    fn bidirectional_does_not_affect_anchor() {
        let content = "The term anuttara appears.<!--- x --->";
        let backward = resolve_scope_range(content, 26, 38, &Scope::Anchor("anuttara".to_string()), "en", ResolutionMode::Backward);
        let bidi = resolve_scope_range(content, 26, 38, &Scope::Anchor("anuttara".to_string()), "en", ResolutionMode::Bidirectional);
        assert_eq!(backward, bidi);
        assert!(backward.is_some());
    }

    #[test]
    fn backward_mode_words_unchanged() {
        let content = "alpha beta <!--- x ---> gamma delta";
        let result = resolve_scope_range(content, 11, 23, &Scope::Words(1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((6, 10))); // "beta" only
    }

    // ── Asymmetric words ──

    #[test]
    fn asym_words_before_and_after() {
        let content = "alpha beta <!--- n: 2_1 | x ---> gamma delta";
        let ann = 11;
        let ann_end = 32;
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymWords(2, 1), "en", ResolutionMode::Backward);
        // "alpha beta" backward + "gamma" forward
        assert_eq!(result, Some((0, 38)));
    }

    #[test]
    fn asym_words_forward_only() {
        let content = "alpha beta <!--- n: 0_1 | x ---> gamma delta";
        let result = resolve_scope_range(content, 11, 32, &Scope::AsymWords(0, 1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((33, 38))); // "gamma"
    }

    #[test]
    fn asym_words_backward_only() {
        let content = "alpha beta <!--- n: 2_0 | x ---> gamma delta";
        let result = resolve_scope_range(content, 11, 32, &Scope::AsymWords(2, 0), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 10))); // "alpha beta"
    }

    #[test]
    fn asym_words_zero_zero() {
        let content = "alpha <!--- x ---> beta";
        let result = resolve_scope_range(content, 6, 18, &Scope::AsymWords(0, 0), "en", ResolutionMode::Backward);
        assert_eq!(result, None);
    }

    #[test]
    fn asym_words_forward_clamps_to_eof() {
        let content = "alpha beta <!--- n: 0_5 | x ---> gamma delta";
        let result = resolve_scope_range(content, 11, 32, &Scope::AsymWords(0, 5), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((33, 44))); // "gamma delta" — all that's available
    }

    #[test]
    fn asym_words_no_text_after() {
        let content = "alpha beta <!--- n: 1_1 | x --->";
        let result = resolve_scope_range(content, 11, 32, &Scope::AsymWords(1, 1), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((6, 10))); // backward "beta" only; forward side empty
    }

    // ── Asymmetric paragraphs ──

    #[test]
    fn asym_paragraph_spec_example() {
        // Spec: `2\p1` targets A and B above, plus one paragraph below
        let content = "Paragraph A.\n\nParagraph B.\n\n<!--- n: 2\\p1 | x --->\n\nParagraph C.\n\nParagraph D.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 2\\p1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymParagraph(2, 1), "en", ResolutionMode::Backward);
        let c_end = content.find("Paragraph C.").unwrap() + "Paragraph C.".len();
        assert_eq!(result, Some((0, c_end)));
    }

    #[test]
    fn asym_paragraph_forward_only() {
        let content = "Paragraph A.\n\n<!--- n: 0\\p2 | x --->\n\nParagraph C.\n\nParagraph D.\n\nParagraph E.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\p2 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymParagraph(0, 2), "en", ResolutionMode::Backward);
        let c_start = content.find("Paragraph C.").unwrap();
        let d_end = content.find("Paragraph D.").unwrap() + "Paragraph D.".len();
        assert_eq!(result, Some((c_start, d_end)));
    }

    #[test]
    fn asym_paragraph_backward_only() {
        let content = "Paragraph A.\n\nParagraph B.\n\n<!--- n: 2\\p0 | x --->\n\nParagraph C.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 2\\p0 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymParagraph(2, 0), "en", ResolutionMode::Backward);
        let b_end = content.find("Paragraph B.").unwrap() + "Paragraph B.".len();
        assert_eq!(result, Some((0, b_end)));
    }

    #[test]
    fn asym_paragraph_forward_clamps_to_eof() {
        let content = "A.\n\n<!--- n: 0\\p9 | x --->\n\nOnly one after.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\p9 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymParagraph(0, 9), "en", ResolutionMode::Backward);
        let start = content.find("Only one after.").unwrap();
        assert_eq!(result, Some((start, content.len())));
    }

    #[test]
    fn asym_paragraph_mid_paragraph_forward_rest() {
        // Annotation mid-paragraph: forward 1 = rest of the current paragraph
        let content = "Before text <!--- n: 0\\p1 | x ---> rest of paragraph.\n\nNext para.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\p1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymParagraph(0, 1), "en", ResolutionMode::Backward);
        let start = content.find("rest of").unwrap();
        let end = content.find("rest of paragraph.").unwrap() + "rest of paragraph.".len();
        assert_eq!(result, Some((start, end)));
    }

    // ── Asymmetric pages ──

    #[test]
    fn asym_page_before_and_after() {
        let content = "One.\x0CTwo.<!--- n: 1\\f1 | x ---> more.\x0CThree.\x0CFour.";
        let ann = 9;
        let ann_end = ann + "<!--- n: 1\\f1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymPage(1, 1), "en", ResolutionMode::Backward);
        // backward: "Two." (5..9); forward: " more." to the next form feed
        let fwd_end = content.find(" more.").unwrap() + " more.".len();
        assert_eq!(result, Some((5, fwd_end)));
    }

    #[test]
    fn asym_page_forward_two() {
        let content = "One.\x0CTwo.<!--- n: 0\\f2 | x ---> more.\x0CThree.\x0CFour.";
        let ann = 9;
        let ann_end = ann + "<!--- n: 0\\f2 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymPage(0, 2), "en", ResolutionMode::Backward);
        let start = content.find("more.").unwrap();
        let end = content.find("Three.").unwrap() + "Three.".len();
        assert_eq!(result, Some((start, end)));
    }

    #[test]
    fn asym_page_spec_backward_only() {
        // Spec: `2\f0` — 2 pages before, 0 after
        let content = "One.\x0CTwo.<!--- n: 2\\f0 | x --->\x0CThree.";
        let ann = 9;
        let ann_end = ann + "<!--- n: 2\\f0 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymPage(2, 0), "en", ResolutionMode::Backward);
        assert_eq!(result, Some((0, 9))); // "One.\x0CTwo."
    }

    // ── Asymmetric sentences ──

    #[test]
    fn asym_sentence_before_and_after() {
        let content = "First one. Second two. <!--- n: 1\\s1 | x ---> Third three. Fourth four.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 1\\s1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(1, 1), "en", ResolutionMode::Backward);
        let back_start = content.find("Second two.").unwrap();
        let fwd_end = content.find("Third three.").unwrap() + "Third three.".len();
        assert_eq!(result, Some((back_start, fwd_end)));
    }

    #[test]
    fn asym_sentence_spec_forward_only() {
        // Spec: `0\s2` — 0 sentences before, 2 after (forward only)
        let content = "First one. Second two. <!--- n: 0\\s2 | x ---> Third three. Fourth four.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\s2 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(0, 2), "en", ResolutionMode::Backward);
        let start = content.find("Third three.").unwrap();
        assert_eq!(result, Some((start, content.len())));
    }

    #[test]
    fn asym_sentence_forward_clamps_to_paragraph() {
        let content = "A one. <!--- n: 0\\s5 | x ---> B two. C three.\n\nNew paragraph sentence.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\s5 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(0, 5), "en", ResolutionMode::Backward);
        let start = content.find("B two.").unwrap();
        let end = content.find("C three.").unwrap() + "C three.".len();
        assert_eq!(result, Some((start, end)));
    }

    #[test]
    fn asym_sentence_forward_double_spaces() {
        // The sentenza whitespace pitfall applies to forward resolution too
        let content = "Intro. <!--- n: 0\\s1 | x ---> Forward  has  double  spaces. Tail.";
        let ann = content.find("<!---").unwrap();
        let ann_end = ann + "<!--- n: 0\\s1 | x --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::AsymSentence(0, 1), "en", ResolutionMode::Backward);
        let start = content.find("Forward").unwrap();
        let end = content.find("spaces.").unwrap() + "spaces.".len();
        assert_eq!(result, Some((start, end)));
    }

    // ── Sentence scope with whitespace normalization ──

    #[test]
    fn sentence_with_double_spaces() {
        // Double spaces (common in LaTeX paste) caused find() to fail because sentenza
        // collapses \s{2,} to a single space during preprocessing.
        let content = "Maximum depth  $d = 5$  and composition.<!-- n: | note -->";
        let ann_start = content.find("<!--").unwrap();
        let result = resolve_scope_range(content, ann_start, content.len(), &Scope::Sentence(1), "en", ResolutionMode::Backward);
        assert!(result.is_some(), "scope should resolve despite double spaces");
        let (start, end) = result.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, ann_start);
    }

    #[test]
    fn sentence_double_spaces_multi_sentence() {
        let content = "First sentence. Second  has  double  spaces.<!-- n: | note -->";
        let ann_start = content.find("<!--").unwrap();
        let result = resolve_scope_range(content, ann_start, content.len(), &Scope::Sentence(1), "en", ResolutionMode::Backward);
        assert!(result.is_some());
        let (start, end) = result.unwrap();
        // Should highlight only the second sentence (with its original double spaces)
        assert_eq!(start, 16); // after "First sentence. "
        assert_eq!(end, ann_start);
    }

    // ── HTML comment blanking in sentence scope (issue #14) ──

    #[test]
    fn sentence_backward_comment_in_same_paragraph() {
        // Exact repro from issue #14: an earlier annotation in the same
        // paragraph gets mangled by sentenza preprocessing (`<!---` → `<!—`),
        // so the sentence containing it could not be located and the whole
        // paragraph failed to resolve.
        let content = "Some text here. Final sentence. <!--- n | first note --->\n\n<!--- n | second note --->\n\nNext paragraph.";
        let ann = content.rfind("<!--- n | second note --->").unwrap();
        let ann_end = ann + "<!--- n | second note --->".len();
        let result = resolve_scope_range(content, ann, ann_end, &Scope::Sentence(1), "en", ResolutionMode::Backward);
        let start = content.find("Final sentence.").unwrap();
        assert_eq!(result, Some((start, start + "Final sentence.".len())));
    }

    #[test]
    fn sentence_backward_comment_inside_sentence() {
        // A comment interrupting the target sentence is blanked away; the
        // resolved range still covers the interrupted sentence end to end.
        let content = "First one. Beta <!--- n | x ---> sentence continues here.<!--- target --->";
        let ann = content.find("<!--- target --->").unwrap();
        let result = resolve_scope_range(content, ann, content.len(), &Scope::Sentence(1), "en", ResolutionMode::Backward);
        let start = content.find("Beta").unwrap();
        let end = content.find("here.").unwrap() + "here.".len();
        assert_eq!(result, Some((start, end)));
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

    // ── blank_comments ──

    #[test]
    fn blank_comments_triple_dash() {
        let input = "p <!--- note ---> q";
        let out = blank_comments(input);
        assert!(!out.contains("<!---"), "annotation comment must be blanked");
        assert_eq!(out.len(), input.len(), "blanking must preserve byte length");
        assert_eq!(&out[0..1], "p");
        assert_eq!(&out[18..19], "q");
    }

    #[test]
    fn blank_comments_standard_comment_untouched() {
        let input = "a <!-- x --> b";
        assert_eq!(blank_comments(input), input);
    }

    #[test]
    fn blank_comments_unclosed() {
        let input = "keep <!--- oops";
        let out = blank_comments(input);
        assert_eq!(out.len(), input.len());
        assert_eq!(out, format!("keep {}", " ".repeat(input.len() - 5)));
    }

    #[test]
    fn blank_comments_multiple() {
        let input = "One. <!--- a ---> Two. <!--- b ---> Three.";
        let out = blank_comments(input);
        assert_eq!(out.len(), input.len());
        assert!(!out.contains("<!---"));
        assert_eq!(&out[0..4], "One.");
        let two = input.find("Two.").unwrap();
        assert_eq!(&out[two..two + 4], "Two.");
        let three = input.find("Three.").unwrap();
        assert_eq!(&out[three..three + 6], "Three.");
    }

    #[test]
    fn blank_comments_multibyte_inside() {
        let input = "café <!--- naïve ---> x";
        let out = blank_comments(input);
        assert_eq!(out.len(), input.len(), "byte length preserved with multibyte content");
        assert!(!out.contains("<!---"));
        assert!(out.starts_with("café "));
        assert!(out.ends_with(" x"));
    }

    #[test]
    fn blank_comments_no_comment() {
        let input = "plain text, nothing to blank.";
        assert_eq!(blank_comments(input), input);
    }
}
