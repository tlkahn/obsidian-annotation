# Obsidian Annotation

An Obsidian plugin that renders HTML comment annotations as styled widgets in edit mode. Designed for scholarly work -- textual notes, cross-references, critical apparatus, translation remarks, and open questions sit inline with your text without cluttering the reading view.

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

Annotations are written as standard HTML comments. Any `<!-- -->` comment in your note is treated as an annotation.

### Compact form (inline)

```
<!-- TYPE CERTAINTY SCOPE | BODY @DATE -->
```

All fields are optional. Some examples:

| Example | Meaning |
|---------|---------|
| `<!-- n? __ \| same sense as TA 3.68? @2026-03 -->` | Note, tentative, applies to 2 preceding words |
| `<!-- todo! \| verify this claim -->` | Todo, important |
| `<!-- cf \pp -->` | Cross-reference, applies to current + preceding paragraph |
| `<!-- tr: _ \| tentative rendering @2026-03 -->` | Translation remark on preceding word |
| `<!-- app: \| ms. B reads *prakasa* instead of *vimarsa* -->` | Critical apparatus entry |
| `<!-- q? ^"8th century" \| Sanderson says 9th c. -->` | Question anchored to specific text |
| `<!-- just a plain comment -->` | Bare annotation (shown as gray pill) |

### Block form (multi-line)

For longer annotations, use a `---` separator between the header and body:

```html
<!--
n!
\p
@2026-03-28
---
Lambert's framing maps closely to Tainter's
complexity brake. See also [[collapse models]].
-->
```

The body supports full Markdown: `*italic*`, `[[wikilinks]]`, `[links](url)`, lists, etc.

### Types

| Keyword | Label | Color |
|---------|-------|-------|
| `n` | Note | Blue |
| `q` | Question | Amber |
| `todo` | Todo | Green |
| `cf` | Cross-ref | Purple |
| `app` | Apparatus | Terracotta |
| `tr` | Translation | Teal |
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
| `_` | 1 preceding word |
| `__` | 2 preceding words |
| `___` | 3 preceding words |
| `\p` | Current paragraph |
| `\pp` or `\p_` | 2 paragraphs (current + preceding) |
| `\ppp` or `\p__` | 3 paragraphs |
| `\f` | Current page |
| `\ff` or `\f_` | 2 pages |
| `^"text"` | Anchored to specific text |

### Date

Append `@YYYY-MM` or `@YYYY-MM-DD` at the end of the body:

```
<!-- n: | this seems wrong @2026-03-28 -->
```

### Opting out

Prefix with `raw:` to keep a comment invisible to the plugin:

```
<!-- raw: this comment is ignored by the annotation plugin -->
```

Comments inside fenced code blocks are also ignored.

## Display modes

The plugin offers two display modes for compact (inline) annotations, configurable in settings:

**Pill mode** (default): Annotations appear as colored inline chips that flow with the text. Each pill shows an icon, certainty mark, truncated body, and date.

**Footnote mode**: Annotations are replaced by small superscript markers (e.g., N?, T!). A side panel lists all annotations in the file with full details. Click a panel entry to jump to its location in the editor.

Block annotations always render as foldable callouts regardless of the display mode setting.

## Interaction

- **Cursor near annotation**: The widget disappears and the raw `<!-- -->` comment is shown, so you can edit it directly
- **Press ESC**: When the cursor is inside an annotation's raw source, pressing ESC exits edit mode and re-renders the widget. Also available as the command "Exit annotation edit mode" in the command palette (rebindable in Obsidian's Hotkeys settings)
- **Click widget body**: Places the cursor at the annotation, expanding it for editing
- **Click fold toggle** (block callouts): Collapses/expands the body
- **Hover over annotation**: Highlights the text the annotation applies to, based on its scope (see below)
- **Links in annotations**: Wikilinks (`[[...]]`) and external links navigate normally

### Scope highlighting

When you hover over an inline annotation (pill or marker), the plugin highlights the text the annotation refers to:

| Scope | What gets highlighted |
|-------|-----------------------|
| `_` | 1 preceding word |
| `__` | 2 preceding words |
| `\p` | Current paragraph |
| `\pp` | Current + preceding paragraph |
| `\f` | Current page (from last form-feed) |
| `^"text"` | The nearest preceding occurrence of "text" |
| _(none)_ | The preceding sentence (default) |

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
