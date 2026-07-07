/// Extract an optional annotation ID from the start of a comment's inner text.
///
/// Per the DSL spec, an ID is `[...]` placed immediately after the opening
/// delimiter. Valid ID characters: letters, digits, `-`, `_`, `.`; the first
/// character must be alphanumeric. An invalid or unterminated ID is not an
/// error — the text (brackets included) is left for normal parsing.
///
/// Returns the ID (if valid) and the remaining inner text with leading
/// whitespace trimmed.
pub fn extract_id(inner: &str) -> (Option<String>, &str) {
    let Some(rest) = inner.strip_prefix('[') else {
        return (None, inner);
    };
    let Some(close) = rest.find(']') else {
        return (None, inner);
    };
    let candidate = &rest[..close];
    let mut chars = candidate.chars();
    let valid = match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        }
        _ => false,
    };
    if !valid {
        return (None, inner);
    }
    // `[text](url)` / `[text][ref]` are markdown links, not IDs
    let after = &rest[close + 1..];
    if after.starts_with('(') || after.starts_with('[') {
        return (None, inner);
    }
    (Some(candidate.to_string()), after.trim_start())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_id() {
        assert_eq!(extract_id("[my-id] n: rest"), (Some("my-id".to_string()), "n: rest"));
    }

    #[test]
    fn uuid_id() {
        let inner = "[550e8400-e29b-41d4-a716-446655440000] body";
        assert_eq!(
            extract_id(inner),
            (Some("550e8400-e29b-41d4-a716-446655440000".to_string()), "body")
        );
    }

    #[test]
    fn id_with_dots_underscores_hyphens() {
        assert_eq!(extract_id("[a.b_c-1]"), (Some("a.b_c-1".to_string()), ""));
    }

    #[test]
    fn digit_first_char_valid() {
        assert_eq!(extract_id("[5abc] x"), (Some("5abc".to_string()), "x"));
    }

    #[test]
    fn id_only_no_remainder() {
        assert_eq!(extract_id("[note1]"), (Some("note1".to_string()), ""));
    }

    #[test]
    fn id_immediately_followed_by_pipe() {
        assert_eq!(extract_id("[id1]| body"), (Some("id1".to_string()), "| body"));
    }

    #[test]
    fn block_form_remainder_preserves_newlines() {
        assert_eq!(
            extract_id("[id2]\nn!\n---\nbody"),
            (Some("id2".to_string()), "n!\n---\nbody")
        );
    }

    #[test]
    fn empty_brackets_invalid() {
        assert_eq!(extract_id("[] x"), (None, "[] x"));
    }

    #[test]
    fn dot_first_char_invalid() {
        assert_eq!(extract_id("[.abc] x"), (None, "[.abc] x"));
    }

    #[test]
    fn hyphen_first_char_invalid() {
        assert_eq!(extract_id("[-abc] x"), (None, "[-abc] x"));
    }

    #[test]
    fn space_inside_invalid() {
        assert_eq!(extract_id("[my id] x"), (None, "[my id] x"));
    }

    #[test]
    fn markdown_chars_invalid() {
        assert_eq!(extract_id("[*bold*] x"), (None, "[*bold*] x"));
    }

    #[test]
    fn wikilink_invalid() {
        assert_eq!(extract_id("[[Note Name]] x"), (None, "[[Note Name]] x"));
    }

    #[test]
    fn unterminated_bracket_invalid() {
        assert_eq!(extract_id("[unterminated x"), (None, "[unterminated x"));
    }

    #[test]
    fn markdown_inline_link_not_an_id() {
        let inner = "[homepage](https://example.com) worth reading";
        assert_eq!(extract_id(inner), (None, inner));
    }

    #[test]
    fn markdown_reference_link_not_an_id() {
        let inner = "[text][ref] more";
        assert_eq!(extract_id(inner), (None, inner));
    }

    #[test]
    fn id_followed_by_spaced_paren_still_valid() {
        assert_eq!(
            extract_id("[id3] (parenthetical) note"),
            (Some("id3".to_string()), "(parenthetical) note")
        );
    }

    #[test]
    fn not_at_start_ignored() {
        assert_eq!(extract_id("x [id] y"), (None, "x [id] y"));
    }

    #[test]
    fn no_brackets_at_all() {
        assert_eq!(extract_id("n: | plain"), (None, "n: | plain"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(extract_id(""), (None, ""));
    }
}
