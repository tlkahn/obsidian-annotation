# Obsidian Annotation

An Obsidian plugin that renders triple-dash HTML comment annotations (`<!--- --->`) as styled widgets in edit mode. Designed for scholarly work -- textual notes, cross-references, critical apparatus, translation remarks, and open questions sit inline with your text without cluttering the reading view.

HTML comments are naturally hidden in Obsidian's reading/preview mode. This plugin makes them visible and useful while editing.

## What it looks like

**Compact annotations** appear as colored inline pills:

```
The term *anuttara* [N? same sense as TA 3.68?  2026-03] appears 47 times.
```

**Block annotations** appear as foldable callouts:

```
┌─ Note (important)                               2026-03-28  ▾ ─┐
│  Lambert's framing maps closely to Tainter's complexity brake.  │
└─────────────────────────────────────────────────────────────────┘
```

Place your cursor on any widget to expand it back to the raw HTML comment for editing.

## Annotation syntax

Annotations are written as triple-dash HTML comments: `<!--- ... --->`. Standard `<!-- -->` comments are ignored and remain ordinary, invisible comments.

### Compact form (inline)

```
<!---[ID] TYPE-OR-MARK CERTAINTY SCOPE ^"ANCHOR" | BODY @DATE --->
```

All fields are optional. Some examples:

| Example | Meaning |
|---------|---------|
| `<!--- n? __ \| same sense as TA 3.68? @2026-03 --->` | Note, tentative, applies to 2 preceding words |
| `<!--- todo! \| verify this claim --->` | Todo, important |
| `<!--- cf \pp --->` | Cross-reference, applies to current + preceding paragraph |
| `<!--- tr: _ \| tentative rendering @2026-03 --->` | Translation remark on preceding word |
| `<!--- app: \| ms. B reads *prakasa* instead of *vimarsa* --->` | Critical apparatus entry |
| `<!--- q? ^"8th century" \| Sanderson says 9th c. --->` | Question anchored to specific text |
| `<!---[my-id] n: \p \| body text --->` | Note with a stable annotation ID |
| `<!--- llm \d \| summarize entire document --->` | LLM content, whole-document scope |
| `<!--- th? \| is this Jayaratha? --->` | Conversational thread on the passage |
| `<!--- n: 2\p1 \| context --->` | Note on 2 paragraphs before + 1 after |
| `<!--- hi __ --->` | Highlight mark on 2 preceding words (no widget) |
| `<!--- just a plain comment --->` | Bare annotation (shown as gray pill) |

### Block form (multi-line)

For longer annotations, use a `---` separator between the header and body:

```html
<!---
n!
\p
@2026-03-28
---
Lambert's framing maps closely to Tainter's
complexity brake. See also [[collapse models]].
--->
```

The body supports full Markdown: `*italic*`, `[[wikilinks]]`, `[links](url)`, lists, etc.

### Annotation IDs

An optional ID in square brackets can be placed immediately after the opening delimiter, in both forms, so external systems can reference an annotation stably:

```
<!---[my-note-id] n: \p | body text --->
```

Valid ID characters: letters, digits, hyphens, underscores, and dots; the first character must be alphanumeric. An invalid ID is not an error - the bracketed text simply stays part of the body, and markdown links like `[text](url)` are never treated as IDs. IDs appear as a tooltip on pills and markers and as a badge in callout headers and the side panel.

### Types

| Keyword | Label | Color |
|---------|-------|-------|
| `n` | Note | Blue |
| `q` | Question | Amber |
| `todo` | Todo | Green |
| `cf` | Cross-ref | Purple |
| `app` | Apparatus | Terracotta |
| `tr` | Translation | Teal |
| `llm` | LLM (AI-generated or AI-directed content, render-only) | Magenta |
| `th` | Thread (conversational note; renders as a plain callout) | Orange |
| _(none)_ | Annotation | Gray |

### Certainty

| Mark | Meaning | Display |
|------|---------|---------|
| `?` | Tentative | Faded, italic, "(tentative)" label |
| `!` | Firm/important | Bold, accented, "(important)" label |
| `:` or none | Neutral | No extra indicator |

### Scope

Scope indicates how much surrounding text the annotation applies to.

| Syntax | Meaning |
|--------|---------|
| `_` / `__` / `___` | 1 / 2 / 3 preceding words |
| `\s` / `\ss` / `\s__` | 1 / 2 / 2 sentences (the default when no scope is given) |
| `\p` / `\pp` / `\p__` | 1 / 2 / 2 paragraphs (current + preceding) |
| `\f` / `\ff` / `\f__` | 1 / 2 / 2 pages (pages are separated by form-feed characters) |
| `\h` | Current markdown section (nearest heading to the next heading of equal or higher level) |
| `\d` | Entire document |
| `^"text"` | Anchored to specific text (searches backward) |
| `3_1` | Asymmetric words: 3 before, 1 after |
| `2\s1` / `3\p1` / `2\f0` | Asymmetric sentences / paragraphs / pages (N before, M after, single digits) |

When a scope requests more units than exist, it gracefully extends to the document boundary. Letter-repeat (`\pp`) and underscore-suffix (`\p__`) forms are equivalent.

### Philological marks

Instead of a type keyword, a mark code turns the annotation into pure text styling: the comment disappears and the scoped text is styled directly. Sixteen built-in codes:

| Codes | Effect |
|-------|--------|
| `nb` `it` `ul` `st` `sc` | bold, italic, underline, strikethrough, small caps |
| `hi` `em` | highlight, emphasis (underline + background) |
| `sic` `crux` `lac` `del` | wavy red underline, †daggered†, [bracketed], faded strikethrough |
| `sup` `conj` `dub` `gloss` `interp` | ⟨supplied⟩, conjecture, dubious, gloss, ⟦interpolation⟧ |

```
<!--- hi _ --->                          highlight 1 word
<!--- sic? _ --->                        tentative sic
<!--- crux \s | dagger this sentence --->
```

Type keywords win over mark codes (`n` is always a Note), and a code followed by ordinary prose stays a bare comment (`<!--- it is raining --->` is not an italic mark). A mark's body (and ID) appears as a hover tooltip on the styled text.

**Custom marks**: define your own codes in `.lit/marks.toml` at the vault root. Style values are restricted to simple CSS (no URLs, quotes, or slashes):

```toml
[mymark]
label = "my custom mark"
[mymark.style]
color = "purple"
font-weight = "bold"
```

### Date

Append `@YYYY-MM` or `@YYYY-MM-DD` at the end of the body:

```
<!--- n: | this seems wrong @2026-03-28 --->
```

### Opting out

Standard HTML comments are the opt-out: any `<!-- -->` comment is invisible to the plugin. Only triple-dash `<!--- --->` comments are treated as annotations. Annotation comments inside fenced code blocks are also ignored.

### Migrating existing annotations

If your vault contains annotations written with the old standard-comment delimiters, a migration CLI converts them in place:

```bash
cargo run -p annotation-core --bin migrate -- /path/to/vault --dry-run   # report what would change
cargo run -p annotation-core --bin migrate -- /path/to/vault             # rewrite files
```

Only comments with detectable annotation structure (type keyword, certainty mark, scope token, anchor, `|` pipe, `@date`, or block form) are converted to `<!--- --->`. Plain prose comments are deliberately left untouched, including tricky cases: `<!-- fix this later -->`, `<!-- [TODO] see below -->` (bracketed word, not an ID), `<!-- 2_4 is the ratio -->` (digit token, not an asymmetric scope), `<!-- it is raining -->` and `<!-- nb -->` (mark-code words in prose). The tool skips hidden directories (e.g. `.obsidian`), fenced code blocks, and is safe to run repeatedly (idempotent). Use `--ext` to migrate a different file extension (default `md`). The Lit spec's legacy `%%! ... %%` delimiters are not supported.

## Display modes

The plugin offers two display modes for compact (inline) annotations, configurable in settings:

**Pill mode** (default): Annotations appear as colored inline chips that flow with the text. Each pill shows an icon, certainty mark, truncated body, and date.

**Footnote mode**: Annotations are replaced by small superscript markers (e.g., N?, T!). A side panel lists all annotations in the file with full details. Click a panel entry to jump to its location in the editor.

Block annotations always render as foldable callouts regardless of the display mode setting.

## Interaction

- **Cursor near annotation**: The widget disappears and the raw `<!--- --->` comment is shown, so you can edit it directly
- **Press ESC**: When the cursor is inside an annotation's raw source, pressing ESC exits edit mode and re-renders the widget. Also available as the command "Exit annotation edit mode" in the command palette (rebindable in Obsidian's Hotkeys settings)
- **Click widget body**: Places the cursor at the annotation, expanding it for editing
- **Click fold toggle** (block callouts): Collapses/expands the body
- **Hover over annotation**: Highlights the text the annotation applies to, based on its scope (see below)
- **Links in annotations**: Wikilinks (`[[...]]`) and external links navigate normally

### Scope highlighting

When you hover over an inline annotation (pill or marker), the plugin highlights the text the annotation refers to:

| Scope | What gets highlighted |
|-------|-----------------------|
| `_` / `__` | 1 / 2 preceding words |
| `\s` / `\ss` | 1 / 2 preceding sentences |
| `\p` / `\pp` | Current / current + preceding paragraph |
| `\f` | Current page (from last form-feed) |
| `\h` | The current heading section |
| `\d` | The entire document (shown with a fainter tint) |
| `^"text"` | The nearest preceding occurrence of "text" |
| `2\p1` etc. | Asymmetric ranges before and after the annotation |
| _(none)_ | The preceding sentence (default) |

Highlights longer than ~2000 characters use a fainter tint so whole-document flashes stay subtle.

The sentence boundary detection (for the default scope) uses the [sentenza](https://github.com/user/sentenza) library, a multilingual rule-based sentence splitter supporting 244+ languages.

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) toolchain
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Node.js and npm
- The [sentenza](https://github.com/user/sentenza) crate checked out as a sibling directory (expected at `../sentenza` relative to this repo)
- (Optional) [binaryen](https://github.com/WebAssembly/binaryen) for `wasm-opt` binary size optimization

### Build and install

```bash
# Clone the repo
git clone <repo-url> obsidian-annotation
cd obsidian-annotation

# One-step build and install into your vault
./install.sh /path/to/your/vault

# Or build without installing
npm install
npm run build
```

The `install.sh` script:
1. Builds the WASM binary (Rust -> WebAssembly)
2. Optimizes with `wasm-opt` if available
3. Bundles the TypeScript
4. Runs all tests
5. Copies `main.js`, `manifest.json`, `styles.css`, and the WASM binary to your vault's plugin directory

After installing, restart Obsidian and enable "Annotation" in Settings > Community Plugins.

### Development

```bash
npm run dev    # watch mode: auto-rebuilds Rust and TypeScript on changes
npm run test   # run all tests (Rust + TypeScript)
```

## Settings

| Setting | Options | Default |
|---------|---------|---------|
| Live rendering | On / Off | On |
| Inline display mode | Pill / Footnote | Pill |

## Requirements

- Obsidian 0.15.0+
- Desktop only (WASM requires filesystem access)
