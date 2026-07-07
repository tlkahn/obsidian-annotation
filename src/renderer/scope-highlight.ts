import { EditorView, Decoration } from "@codemirror/view";
import { StateEffect, StateField, Extension } from "@codemirror/state";
import type { DecorationSet } from "@codemirror/view";

/** Effect to set or clear the scope highlight range. */
export const setScopeHighlight = StateEffect.define<{ from: number; to: number } | null>();

const scopeHighlightMark = Decoration.mark({ class: "annotation-scope-highlight" });

// Very large scopes (whole document, big sections) get a fainter tint so a
// hover doesn't flash the entire file in full highlight color.
const LARGE_SCOPE_THRESHOLD = 2000;
const scopeHighlightMarkLarge = Decoration.mark({
    class: "annotation-scope-highlight annotation-scope-highlight-large",
});

/** StateField that holds the current scope highlight decoration. */
const scopeHighlightField = StateField.define<DecorationSet>({
    create() {
        return Decoration.none;
    },
    update(deco, tr) {
        for (const e of tr.effects) {
            if (e.is(setScopeHighlight)) {
                if (e.value === null) return Decoration.none;
                const { from, to } = e.value;
                if (from >= 0 && to > from && to <= tr.state.doc.length) {
                    const mark =
                        to - from > LARGE_SCOPE_THRESHOLD
                            ? scopeHighlightMarkLarge
                            : scopeHighlightMark;
                    return Decoration.set([mark.range(from, to)]);
                }
                return Decoration.none;
            }
        }
        return deco.map(tr.changes);
    },
    provide: (f) => EditorView.decorations.from(f),
});

/** Dispatch a scope highlight effect to the editor view. */
export function dispatchScopeHighlight(view: EditorView, from: number, to: number): void {
    view.dispatch({ effects: setScopeHighlight.of({ from, to }) });
}

/** Clear any active scope highlight. */
export function clearScopeHighlight(view: EditorView): void {
    view.dispatch({ effects: setScopeHighlight.of(null) });
}

/** CM6 extension that enables scope highlighting. */
export function scopeHighlightExtension(): Extension {
    return scopeHighlightField;
}
