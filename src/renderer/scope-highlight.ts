import { EditorView, Decoration } from "@codemirror/view";
import { StateEffect, StateField, Extension } from "@codemirror/state";
import type { DecorationSet } from "@codemirror/view";

/** Effect to set or clear the scope highlight range. */
export const setScopeHighlight = StateEffect.define<{ from: number; to: number } | null>();

const scopeHighlightMark = Decoration.mark({ class: "annotation-scope-highlight" });

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
                    return Decoration.set([scopeHighlightMark.range(from, to)]);
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
