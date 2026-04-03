# Proposed Annotation Types

Additional annotation types based on standard philological and text-critical conventions.

## High-value additions

| Abbrev | Full | Use case |
|--------|------|----------|
| `nb` | *nota bene* | Important/notable passage. Universally recognized in scholarship. |
| `em` | *emendatio* | Proposed correction to the text. Bread and butter of philology. |
| `crux` | *crux desperationis* (†) | Corrupt/unintelligible passage. Fundamental text-critical marker. |
| `var` | *varia lectio* | Variant reading across manuscripts/editions. Records a specific alternative (distinct from `app` which is a full apparatus entry). |
| `lac` | *lacuna* | Gap in the text (lost, illegible, damaged). |
| `gl` | *glossa* | Gloss — explanatory note on a difficult word/phrase. Lexical rather than discursive (distinct from `n`). |
| `par` | *parallela* | Parallel passage. More specific than `cf` — "this says the same thing" vs. "see also". |
| `sic` | *sic erat scriptum* | Reproduced as-is — flags something that looks wrong but is faithful to the source. |

## Why not `!!` or `bedeutend` for "important"?

- `!!` collides with `!` (certainty marker) in the parser — ambiguous in `<!-- !! _ | text -->`
- `bedeutend` is language-specific (German) and too long for a terse DSL
- `nb` is the clear winner: two characters, Latin (lingua franca of Western philology), instantly understood by any scholar

## "Review later" does not need a new type

Combine `todo` with the existing certainty/body system:

```
<!-- todo? | revisit after checking MS B @2026-06 -->
```

The `?` certainty + body + date already captures "review later" semantics. A separate `rev` type would overlap too much with `todo`.

## Priority for Sanskrit/Indological work

1. `em` — constantly needed for proposed emendations
2. `crux` — corrupt passages need flagging distinctly from questions
3. `var` — variant readings are constant companions in critical editions
4. `nb` — "pay attention here" without cluttering the body
5. `par` — parallel passages are a major scholarly tool, especially across texts

The rest (`lac`, `gl`, `sic`) are useful but more situational.

## Examples

```
<!-- nb _ | key term for Dharmakīrti's epistemology -->
<!-- em! __ | read "pramāṇa" for "pramāṇa" (eyeskip) -->
<!-- crux? ___ | text garbled in all witnesses -->
<!-- var _ | MS B reads "jñāna" for "vijñāna" -->
<!-- lac __ | folio damaged, approx 2 akṣaras lost -->
<!-- gl _ | Skt. "upādāna" — lit. "fuel", fig. "clinging" -->
<!-- par \p | cf. Abhidharmakośa III.28 -->
<!-- sic _ | sic in ed. Lévi 1907 -->
```
