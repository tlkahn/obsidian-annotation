# Implementation Notes

## Architecture

Rust core parses annotation DSL, compiled to WASM. TypeScript handles Obsidian integration and CM6 rendering.

```
Document text
  │
  ▼
scanner.rs ── find <!-- --> comments, skip code fences & raw:, emit UTF-16 offsets
  │
  ▼
parser.rs ── classify each comment (block vs compact) and dispatch
  │                     │
  ├─ compact.rs         ├─ block.rs
  │  (single-line)      │  (multi-line, --- separator)
  ▼                     ▼
Vec<Annotation> ── serialized to JSON via wasm-bindgen
  │
  ▼
bridge.ts ── loads WASM, deserializes JSON to TypeScript Annotation[]
  │
  ▼
live-mode.ts ── CM6 EditorView.decorations.compute()
  │                     │
  ├─ CalloutWidget      ├─ PillWidget       ├─ MarkerWidget
  │  (block form)       │  (compact/pill)   │  (compact/footnote)
  ▼                     ▼                   ▼
DOM widgets replace <!-- --> in editor, expand on cursor proximity
```

## DSL Syntax

### Compact form (single-line)

```
<!-- TYPE CERTAINTY SCOPE | BODY @DATE -->
```

| Field | Syntax | Examples |
|-------|--------|---------|
| Type | `n` `q` `todo` `cf` `app` `tr` | `n` = note, `cf` = cross-ref, `tr` = translation |
| Certainty | `?` (tentative), `!` (firm), `:` (neutral) | `n?`, `todo!`, `cf:` |
| Scope | `_` (1 word), `__` (2 words), `\p` (paragraph), `\pp` (2 para), `\f` (page), `\ff` (2 pages) | Also `\p__` = `\pp`, `\f___` = `\fff` |
| Anchor | `^"text"` | `^"8th century"` |
| Body | after `\|` | `\| this is the body` |
| Date | `@YYYY-MM` or `@YYYY-MM-DD` | `@2026-03` |

Everything is optional. A comment with no recognized structure becomes a "bare" annotation.

### Block form (multi-line)

```html
<!--
TYPE CERTAINTY
SCOPE
@DATE
^"ANCHOR"
---
Markdown body here.
-->
```

Head lines (above `---`) are parsed one-per-line for type, scope, date, anchor. Body (below `---`) is arbitrary Markdown.

### Opt-out

`<!-- raw: anything -->` is skipped entirely. Comments inside fenced code blocks are also skipped.

## Parsing Pipeline

### 1. Scanner (`scanner.rs`)

- Two-pass: first finds fenced code block byte ranges, then scans for `<!-- -->` outside those ranges
- UTF-16 offset tracking: incremental accumulation for CM6 compatibility (JS strings are UTF-16)
- Skips `raw:` prefix comments
- Returns `Vec<RawComment>` with `char_start`, `char_end`, `inner` (trimmed), `original`

### 2. Classifier (`parser.rs`)

Simple dispatch: if `inner` contains a line that is exactly `---`, route to block parser; otherwise compact parser.

### 3. Compact parser (`compact.rs`)

Sequential greedy matching:
1. Match type keyword at start (`todo`, `app`, `cf`, `tr`, `n`, `q` -- longest first to avoid prefix conflicts)
2. Match certainty mark (`?`, `!`, `:`)
3. Match scope (regex: underscores, `\p` variants, `\f` variants)
4. Match anchor (`^"text"`)
5. Split on `|` for body
6. Extract `@date` from body

Uses `is_structured` flag: if nothing structured was found, the entire text becomes a bare annotation body.

### 4. Block parser (`block.rs`)

1. Split on first `---` line into head and body
2. Parse head lines individually: date (`@...`), anchor (`^"..."`), scope, type+certainty
3. Body is everything after the separator, trimmed

### 5. Scope system

Generalized numeric scope with two equivalent notations:

| Notation | Scope | Count |
|----------|-------|-------|
| `_` | Words | 1 |
| `___` | Words | 3 |
| `\p` | Paragraph | 1 |
| `\pp` or `\p_` | Paragraph | 2 |
| `\ppp` or `\p__` | Paragraph | 3 |
| `\f` | Page | 1 |
| `\ff` or `\f_` | Page | 2 |
| `\fff` or `\f__` | Page | 3 |

Letter-repetition (`\pp`) and underscore-suffix (`\p__`) are equivalent. Count = number of repeated letters (including the first) or number of underscores.

## TypeScript Layer

### WASM Bridge (`bridge.ts`)

- Loads `.wasm` binary from plugin directory via `FileSystemAdapter.readBinary()`
- Calls `initSync()` (synchronous WASM init from bytes)
- Single function: `parseAnnotations(content) -> Annotation[]`

### Live Mode (`live-mode.ts`)

- `EditorView.decorations.compute(["doc", "selection"], ...)` recomputes on every doc change and cursor move
- `isInEditableRange()`: hides decoration when cursor is within +/-1 char buffer of the annotation, allowing the user to edit the raw source
- Routes: block -> `CalloutWidget`, compact -> `PillWidget` (default) or `MarkerWidget` (footnote mode)
- Decorations are `Decoration.replace({ widget })` that replace the full `<!-- -->` range

### Widgets (`widgets.ts`)

**CalloutWidget** (block annotations):
- Foldable callout with colored left border (4px, type-specific)
- Header: icon + type label + certainty + date + fold toggle (triangle)
- Body: Markdown-rendered via `MarkdownRenderer.render()` (async, fire-and-forget in `toDOM()`)
- Header click toggles fold; body click places cursor at annotation start
- Wikilinks and external links in body navigate instead of entering edit mode

**PillWidget** (compact annotations, default mode):
- Inline `<span>` with type-colored background tint and border
- Icon + certainty mark + body (truncated at 60 chars, Markdown-rendered) + date
- Click places cursor at annotation start (except link clicks)

**MarkerWidget** (compact annotations, footnote mode):
- Superscript `<sup>` with type letter + certainty mark (e.g., "N?")
- Type-colored text
- Click places cursor at annotation start

All widgets use `setTimeout(() => view.dispatch(...), 0)` to defer cursor placement outside the mousedown handler.

### Side Panel (`annotation-panel.ts`)

- `ItemView` registered as `"annotation-panel"`
- Lists all annotations from the active file, parsed via WASM bridge
- Each entry: type label (colored), certainty, date, line number, Markdown-rendered body
- Click navigates editor to annotation position
- Refreshed on `file-open` and `metadataCache.changed` events
- Auto-opens/closes when `inlineDisplayMode` setting changes

### Type/Color Mapping

| Type | Label | Lucide Icon | RGB |
|------|-------|-------------|-----|
| `n` (note) | Note | `lucide-pen-line` | `86, 154, 222` (blue) |
| `q` (question) | Question | `lucide-help-circle` | `236, 177, 0` (amber) |
| `todo` | Todo | `lucide-circle-check` | `72, 198, 123` (green) |
| `cf` (crossref) | Cross-ref | `lucide-arrow-up-right` | `168, 130, 214` (purple) |
| `app` (apparatus) | Apparatus | `lucide-git-branch` | `198, 120, 95` (terracotta) |
| `tr` (translation) | Translation | `lucide-languages` | `100, 180, 160` (teal) |
| bare | Annotation | `lucide-message-square` | `136, 136, 136` (gray) |

Colors are passed as CSS `--callout-color` RGB triplets, enabling `rgb()` and `rgba()` usage in stylesheets.

## Build System

- **Rust -> WASM**: `wasm-pack build crates/wasm --target web --release` (optional `wasm-opt -Oz`)
- **TypeScript -> JS**: esbuild, CommonJS format, externals for `obsidian`, `electron`, CM6 modules
- **Dev mode**: `concurrently` runs `cargo watch` and esbuild watch in parallel
- **Install**: `./install.sh [vault_path]` builds everything, runs tests, copies artifacts to vault's plugin directory

## Testing

- **Rust**: `cargo test -p annotation-core` -- unit tests per module (types, scanner, compact, block) + integration tests in parser
- **TypeScript**: `vitest run` -- JSON deserialization tests in `src/__tests__/bridge.test.ts`
- **Manual verification**: symlink/copy plugin into test vault, check live rendering in edit mode

## Design Decisions

1. **WASM for parsing**: Keeps the parser fast and portable. Parsing is stateless and pure -- no DOM, no Obsidian API dependency. The same Rust core could be reused in other editors.

2. **UTF-16 offsets**: CM6 (and JavaScript strings) use UTF-16 code units. The scanner tracks UTF-16 offsets incrementally to avoid O(n) rescanning.

3. **Bare annotations**: Any `<!-- -->` that doesn't match structured DSL syntax is treated as a bare annotation (gray pill). This ensures all comments are visible in edit mode.

4. **`raw:` opt-out**: Provides escape hatch for comments that should remain invisible (e.g., metadata, build markers).

5. **Cursor-proximity expansion**: When the cursor is within 1 char of an annotation, the decoration is hidden and the raw HTML comment is shown, enabling direct editing.

6. **Link passthrough**: Wikilinks (`[[...]]`) and external links (`[text](url)`) rendered inside annotation widgets navigate normally instead of entering edit mode. This is implemented by checking `isLinkClick(e)` before dispatching cursor placement.

7. **Scope generalization**: The underscore-suffix notation (`\p__` = 2 paragraphs) was added alongside letter-repetition (`\pp`) for consistency with the word scope (`__` = 2 words). Both notations are equivalent and interchangeable.
