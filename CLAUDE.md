# Obsidian Annotation Plugin

Desktop-only Obsidian plugin that renders HTML comment annotations (`<!-- -->`) as styled widgets in edit mode (CM6). Rust core compiled to WASM handles parsing; TypeScript handles UI.

## Quick orientation

```
crates/core/src/     Rust parser + scope resolver (scanner → parser → Annotation, scope_resolver → range)
crates/wasm/src/     wasm-bindgen FFI: parse_annotations(str) -> JSON, resolve_scope_range(str,...) -> JSON
src/                 TypeScript: plugin lifecycle, CM6 decorations, widgets, scope highlight, side panel
styles.css           All widget CSS (callout, pill, marker, panel, scope highlight)
install.sh           Build + install to Obsidian vault
```

## Key concepts

- **Two annotation forms**: compact (single-line, inline) and block (multi-line, `---` separator)
- **Seven types**: `n` (note), `q` (question), `todo`, `cf` (cross-ref), `app` (apparatus), `tr` (translation), bare (untyped)
- **Scope system**: `_`/`__`/`___` (words), `\p`/`\pp`/`\ppp` (paragraphs), `\f`/`\ff`/`\fff` (pages), `^"text"` (anchor). Underscore suffix equivalent: `\p__` = `\pp`
- **Two display modes** for compact annotations: pill (inline colored chip) or footnote (superscript marker + side panel)
- **Block annotations** always render as foldable callouts
- **Scope hover highlight**: hovering a pill/marker highlights the scoped text (preceding N words, sentence, paragraph, page, or anchor match)
- **`raw:`** prefix opts a comment out of annotation rendering
- UTF-16 offsets throughout (CM6/JS compatibility)

## Build commands

```bash
npm run build          # full build (WASM + TS)
npm run build:wasm     # WASM only
npm run build:ts       # TypeScript only
npm run dev            # watch mode (cargo watch + esbuild watch)
npm run test           # all tests (Rust + TS)
cargo test -p annotation-core   # Rust unit tests
npx vitest run         # TS tests
./install.sh [vault]   # build + install to vault (default: ~/Documents/Ekuro)
```

## Architecture patterns

- WASM bridge: `FileSystemAdapter.readBinary()` + `initSync()` (same pattern as turboref)
- CM6: `EditorView.decorations.compute(["doc", "selection"], ...)` with cursor-proximity check to expand raw source
- Widgets: `WidgetType.toDOM()` creates DOM synchronously, `MarkdownRenderer.render()` fires async
- Widget clicks use `setTimeout(() => view.dispatch(...), 0)` to defer cursor placement
- Links inside widgets pass through to Obsidian navigation (checked via `isLinkClick()`)
- Scope hover: CM6 `StateEffect`/`StateField` for transient `Decoration.mark`; scope resolved on demand via WASM (`scope_resolver.rs`)
- Sentence splitting: `sentenza` crate (path dep at `../../../sentenza`) used for `Adjacency` scope

## DSL reference (compact form)

```
<!-- TYPE CERTAINTY SCOPE | BODY @DATE -->
```

Examples:
- `<!-- n? __ | same sense as TA 3.68? @2026-03 -->`
- `<!-- todo! ^"8th century" | verify date -->`
- `<!-- cf \pp -->`
- `<!-- tr: _ | tentative rendering @2026-03 -->`
- `<!-- just a bare comment -->`

## DSL reference (block form)

```html
<!--
TYPE CERTAINTY
SCOPE
@DATE
---
Markdown body
-->
```

## Files that matter most

- `crates/core/src/compact.rs` — compact form parser (regex-based, sequential greedy)
- `crates/core/src/block.rs` — block form parser (head/body split on `---`)
- `crates/core/src/scanner.rs` — HTML comment scanner with UTF-16 tracking
- `crates/core/src/scope_resolver.rs` — resolves scope to concrete UTF-16 text range (Words, Adjacency, Paragraph, Page, Anchor)
- `src/renderer/widgets.ts` — CalloutWidget, PillWidget, MarkerWidget DOM construction + hover handlers
- `src/renderer/live-mode.ts` — CM6 decoration layer + editable-range logic
- `src/renderer/scope-highlight.ts` — CM6 StateEffect/StateField for transient scope highlight mark
- `styles.css` — all visual styling (uses `--callout-color` CSS variable for theming)

## Testing

Rust tests cover all parser edge cases extensively (types, scope, certainty, anchors, dates, UTF-16 offsets, code fence skipping, bare annotations) plus scope resolver tests (Words, Adjacency, Paragraph, Page, Anchor). TS tests cover JSON deserialization from WASM output (Annotation + ScopeRange). Manual testing in an Obsidian vault is needed for widget rendering and scope hover highlighting.
