//! Migration from legacy standard-comment annotations (`<!-- -->`) to
//! triple-dash annotation comments (`<!--- --->`).
//!
//! Only comments with detectable annotation structure are converted; plain
//! prose comments (and `raw:` comments) stay standard comments.

use crate::compact::is_structured_annotation;
use crate::scanner::{find_fenced_ranges, is_in_fenced_range};

/// Result of migrating one document.
pub struct MigrationResult {
    /// The migrated document text.
    pub output: String,
    /// Number of comments converted to triple-dash form.
    pub conversions: usize,
}

/// Convert legacy structured `<!-- -->` annotations in `content` to
/// `<!--- --->` form. Idempotent: already-migrated comments, plain prose
/// comments, fenced comments, and unclosed comments are left untouched.
pub fn migrate_content(content: &str) -> MigrationResult {
    let fenced_ranges = find_fenced_ranges(content);
    let mut output = String::with_capacity(content.len());
    let mut conversions = 0usize;
    let mut pos = 0usize; // byte offset

    while let Some(rel) = content[pos..].find("<!--") {
        let open = pos + rel;
        let after_open = open + 4;

        // Inside a fenced code block: leave untouched
        if is_in_fenced_range(open, &fenced_ranges) {
            output.push_str(&content[pos..after_open]);
            pos = after_open;
            continue;
        }

        // Already triple-dash: `find("<!--")` matches the `<!---` prefix too
        if content.as_bytes().get(after_open) == Some(&b'-') {
            output.push_str(&content[pos..after_open]);
            pos = after_open;
            continue;
        }

        if let Some(close_rel) = content[after_open..].find("-->") {
            let close = after_open + close_rel;
            let end = close + 3;
            let inner = &content[after_open..close];

            if !is_structured_annotation(inner.trim()) {
                // Plain prose (or raw:) comment: leave as a standard comment
                output.push_str(&content[pos..end]);
                pos = end;
                continue;
            }

            output.push_str(&content[pos..open]);
            output.push_str("<!---");
            output.push_str(inner); // raw inner, whitespace preserved
            output.push_str("--->");
            conversions += 1;
            pos = end;
        } else {
            // Unclosed comment: leave the rest untouched
            break;
        }
    }
    output.push_str(&content[pos..]);

    MigrationResult { output, conversions }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_structured_comment() {
        let result = migrate_content("<!-- n: | note -->");
        assert_eq!(result.output, "<!--- n: | note --->");
        assert_eq!(result.conversions, 1);
    }

    #[test]
    fn preserves_plain_prose_comment() {
        let result = migrate_content("<!-- fix this later -->");
        assert_eq!(result.output, "<!-- fix this later -->");
        assert_eq!(result.conversions, 0);
    }

    #[test]
    fn preserves_raw_comment() {
        let result = migrate_content("<!-- raw: build marker -->");
        assert_eq!(result.output, "<!-- raw: build marker -->");
        assert_eq!(result.conversions, 0);
    }

    #[test]
    fn idempotent_on_triple_dash() {
        let result = migrate_content("<!--- n: | note --->");
        assert_eq!(result.output, "<!--- n: | note --->");
        assert_eq!(result.conversions, 0);
    }

    #[test]
    fn skips_comment_in_code_fence() {
        let doc = "```\n<!-- n: | fenced -->\n```\n<!-- n: | outside -->";
        let result = migrate_content(doc);
        assert_eq!(
            result.output,
            "```\n<!-- n: | fenced -->\n```\n<!--- n: | outside --->"
        );
        assert_eq!(result.conversions, 1);
    }

    #[test]
    fn mixed_partial_vault() {
        let result = migrate_content("<!--- n: done ---> <!-- q? todo -->");
        assert_eq!(result.output, "<!--- n: done ---> <!--- q? todo --->");
        assert_eq!(result.conversions, 1);
    }

    #[test]
    fn preserves_multiline_block_form() {
        let result = migrate_content("<!--\nn!\n---\nbody\n-->");
        assert_eq!(result.output, "<!---\nn!\n---\nbody\n--->");
        assert_eq!(result.conversions, 1);
    }

    #[test]
    fn leaves_unclosed_comment() {
        let result = migrate_content("<!-- n: no end");
        assert_eq!(result.output, "<!-- n: no end");
        assert_eq!(result.conversions, 0);
    }

    #[test]
    fn double_run_is_noop() {
        let doc = "\
Intro.<!-- n: _ | note @2026-03 -->

<!-- plain prose -->

<!-- raw: marker -->

```
<!-- n: | fenced -->
```

<!--
todo!
---
Block body.
-->

<!--- q? already migrated --->
";
        let first = migrate_content(doc);
        assert_eq!(first.conversions, 2); // compact note + block todo
        let second = migrate_content(&first.output);
        assert_eq!(second.conversions, 0);
        assert_eq!(second.output, first.output);
    }
}
