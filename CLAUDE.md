# Obsidian Annotation Plugin

Desktop-only Obsidian plugin that renders triple-dash HTML comment annotations (`<!--- --->`) as styled widgets in edit mode (CM6). Standard `<!-- -->` comments are ignored. Rust core compiled to WASM handles parsing; TypeScript handles UI.

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
- **Nine types**: `n` (note), `q` (question), `todo`, `cf` (cross-ref), `app` (apparatus), `tr` (translation), `llm` (LLM content), `th` (thread), bare (untyped)
- **Annotation IDs**: optional `[id]` immediately after `<!---` in both forms (charset: alphanumeric first char, then letters/digits/`-`/`_`/`.`); invalid IDs fall through to body text, and markdown links `[text](url)` are never treated as IDs
- **Philological marks**: 16 built-in display-only mark codes (`nb` `it` `ul` `st` `sc` `hi` `em` `sic` `crux` `lac` `del` `sup` `conj` `dub` `gloss` `interp`) in the type slot (`<!--- hi _ --->`); type keywords take precedence; a code followed by prose stays Bare (`it is raining`). Marks hide the comment and style the resolved scope range via CM6 `Decoration.mark` (`.annotation-mark-<code>` in styles.css); a body surfaces as a `title` tooltip. Custom marks come from `.lit/marks.toml` at the vault root (`[code]` label/icon + `[code.style]` CSS map, parsed by `marks.rs` via WASM `parse_marks_toml`, dynamic CSS injected at load)
- **Scope system**: `_`/`__`/`___` (words), `\s`/`\ss`/`\sss` (sentences, default), `\p`/`\pp`/`\ppp` (paragraphs), `\f`/`\ff`/`\fff` (pages), `\h` (heading section), `\d` (document), `^"text"` (anchor), asymmetric `N_M` / `N\sM` / `N\pM` / `N\fM` (N units before, M after, single digits). Underscore suffix equivalent: `\p__` = `\pp`
- **Scope resolution**: `resolve_scope_range(content, char_start, char_end, scope, lang, mode)` - graceful clamp to document boundaries; modes `backward` (default) and `bidirectional` (symmetric scopes extend both ways by the same count)
- **Two display modes** for compact annotations: pill (inline colored chip) or footnote (superscript marker + side panel)
- **Block annotations** always render as foldable callouts
- **Scope hover highlight**: hovering a pill/marker highlights the scoped text (N words/sentences/paragraphs/pages backward, heading section, whole document, anchor match, or asymmetric before/after ranges); ranges over 2000 chars use a fainter tint (`.annotation-scope-highlight-large`)
- **ID surfacing**: pills show `[id]` as a tooltip; callout headers and side-panel entries show a faint monospace `[id]` badge
- **ESC to exit edit mode**: pressing ESC when cursor is inside an annotation moves cursor out, re-rendering the widget
- **Standard `<!-- -->` comments are the opt-out**: only `<!--- --->` renders as an annotation
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
cargo run -p annotation-core --bin migrate -- <vault> [--dry-run] [--ext md]   # migrate legacy <!-- --> annotations to <!--- --->
```

## Architecture patterns

- WASM bridge: `FileSystemAdapter.readBinary()` + `initSync()` (same pattern as turboref)
- CM6: `EditorView.decorations.compute(["doc", "selection"], ...)` with cursor-proximity check to expand raw source
- Widgets: `WidgetType.toDOM()` creates DOM synchronously, `MarkdownRenderer.render()` fires async
- Widget clicks use `setTimeout(() => view.dispatch(...), 0)` to defer cursor placement
- Links inside widgets pass through to Obsidian navigation (checked via `isLinkClick()`)
- Scope hover: CM6 `StateEffect`/`StateField` for transient `Decoration.mark`; scope resolved on demand via WASM (`scope_resolver.rs`)
- ESC exit: CM6 keymap extension moves cursor to `char_end + 2` (must clear the `buffer=1` zone in `isInEditableRange`; `+1` is still inside). The logic works on absolute scanner offsets, so it is delimiter-length- and `[id]`-independent; `isInEditableRange` lives in `src/renderer/editable-range.ts` (standalone, unit-tested)
- Sentence splitting: `sentenza` crate (path dep at `../../../sentenza`) used for `Sentence` scope
- **Sentenza preprocessing pitfall**: `sentenza::split_sentences` preprocesses text before splitting (e.g. collapsing `\s{2,}` to single space). Returned sentence strings won't match the original text verbatim when the source has double spaces (common in LaTeX/PDF paste). The scope resolver uses `ws_flexible_find()` — a whitespace-tolerant search — instead of exact `str::find()` to locate sentences back in the original paragraph. Any new code that matches sentenza output against original text must account for this mismatch.

## DSL reference (compact form)

```
<!---[ID] TYPE-OR-MARK CERTAINTY SCOPE | BODY @DATE --->
```

All parts optional; SCOPE is a scope token or an anchor `^"text"` (an anchor replaces any scope token, they never compose). Examples:
- `<!--- n? __ | same sense as TA 3.68? @2026-03 --->`
- `<!--- todo! ^"8th century" | verify date --->`
- `<!--- cf \pp --->`
- `<!--- tr: _ | tentative rendering @2026-03 --->`
- `<!---[my-id] n: \p | body text --->` (annotation ID)
- `<!--- llm \d | summarize entire document --->` (document scope)
- `<!--- n: \h | section note --->` (heading-section scope)
- `<!--- n: 2\p1 | two paragraphs before, one after --->` (asymmetric)
- `<!--- hi _ --->` / `<!--- sic? _ --->` / `<!--- em ^"phrase" | note --->` (marks)
- `<!--- just a bare comment --->`

## DSL reference (block form)

```html
<!---[ID]
TYPE-OR-MARK CERTAINTY
SCOPE
@DATE
---
Markdown body
--->
```

Header lines can appear in any order; the first `---` line separates head from body. The Lit spec's legacy `%%! ... %%` delimiters are NOT supported by this plugin.

## Files that matter most

- `crates/core/src/compact.rs` — compact form parser (regex-based, sequential greedy)
- `crates/core/src/block.rs` — block form parser (head/body split on `---`)
- `crates/core/src/marks.rs` — custom mark definitions (`.lit/marks.toml` parsing)
- `crates/core/src/scanner.rs` — HTML comment scanner with UTF-16 tracking
- `crates/core/src/scope_resolver.rs` — resolves scope to concrete UTF-16 text range (Words, Sentence, Paragraph, Page, Anchor, Section, Document, asymmetric variants; backward/bidirectional modes)
- `src/renderer/widgets.ts` — CalloutWidget, PillWidget, MarkerWidget DOM construction + hover handlers
- `src/renderer/live-mode.ts` — CM6 decoration layer + editable-range logic
- `src/renderer/scope-highlight.ts` — CM6 StateEffect/StateField for transient scope highlight mark
- `src/renderer/escape-annotation.ts` — CM6 keymap (ESC exits annotation edit mode) + Obsidian command
- `styles.css` — all visual styling (uses `--callout-color` CSS variable for theming)

## Testing

Rust tests cover all parser edge cases extensively (types, scope, certainty, anchors, dates, UTF-16 offsets, code fence skipping, bare annotations) plus scope resolver tests (Words, Sentence, Paragraph, Page, Anchor, Section, Document, asymmetric variants, resolution modes, boundary clamping). TS tests cover JSON deserialization from WASM output (Annotation + ScopeRange). Manual testing in an Obsidian vault is needed for widget rendering and scope hover highlighting.
