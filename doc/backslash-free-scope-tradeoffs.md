# Backslash-free scope notation: tradeoffs

Branch: `feat/backslash-free-scope`

## Context

The scope DSL requires a leading backslash for paragraph, sentence, and page scopes:

```
\p, \pp, \ppp, \p__, \p___   (paragraphs)
\s, \ss, \sss, \s__, \s___   (sentences)
\f, \ff, \fff, \f__, \f___   (pages)
```

This branch adds backslash-free alternatives (e.g. `p__`, `pp`, `ss`, `f__`) so that `<!-- n: p__ | nb -->` parses as Paragraph(2) instead of silently falling back to the default Sentence(1).

## What changed

- `SCOPE_RE` in `compact.rs` now matches `p(p+|_{1,})`, `s(s+|_{1,})`, `f(f+|_{1,})` (without leading `\`)
- `Scope::try_parse` in `types.rs` handles the same forms
- Single bare letters (`p`, `s`, `f`) still require the backslash to avoid ambiguity

## Parsing ambiguity introduced

The backslash serves as an unambiguous "this is a scope" signal. Without it, the parser greedily matches scope tokens before checking for the pipe separator, which can misparse body text as scope:

```
<!-- n: ss clearly a note -->
```

With this branch: scope = Sentence(2), body = "clearly a note"
Without this branch: scope = Sentence(1) (default), body = "ss clearly a note"

Similarly, bare comments can be misinterpreted:

```
<!-- pp -->
```

With this branch: scope = Paragraph(2), body = None (structured)
Without this branch: annotation_type = Bare, body = "pp"

## Risk assessment

- **Underscore-suffixed forms** (`p__`, `s__`, `f__`): safe. Nobody writes `p__` as prose.
- **Letter-repeated forms** (`pp`, `ss`, `ff`): mild risk. These are uncommon but could appear in abbreviations or body text.

## Options if merging

1. **Merge as-is** — accept the small ambiguity for `pp`/`ss`/`ff`.
2. **Only keep underscore-suffixed forms** — drop `pp`/`ss`/`ff` support, keep `p__`/`s__`/`f__`. This eliminates the ambiguity entirely since `p__` is never natural text.
3. **Don't merge** — keep the backslash requirement and document it more prominently.
