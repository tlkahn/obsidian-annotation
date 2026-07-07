import { EditorView, Decoration } from "@codemirror/view";
import { Extension } from "@codemirror/state";
import type AnnotationPlugin from "../main";
import type { Annotation, ScopeRange, WasmBridge } from "../bridge";
import { CalloutWidget, PillWidget, MarkerWidget } from "./widgets";
import { isInEditableRange } from "./editable-range";

interface DecorationEntry {
    start: number;
    end: number;
    decoration: Decoration;
}

// Mark scope resolutions are cached per document content: the decoration
// compute reruns on every selection change (each cursor move), and without
// a cache every mark would copy the whole document across the JS-WASM
// boundary each time.
let scopeCacheContent = "";
const scopeCache = new Map<string, ScopeRange | null>();

function resolveMarkScope(bridge: WasmBridge, content: string, ann: Annotation): ScopeRange | null {
    if (content !== scopeCacheContent) {
        scopeCache.clear();
        scopeCacheContent = content;
    }
    const key = `${ann.char_start}:${ann.char_end}:${JSON.stringify(ann.scope)}`;
    if (scopeCache.has(key)) return scopeCache.get(key) ?? null;
    const range = bridge.resolveScopeRange(content, ann.char_start, ann.char_end, ann.scope, "en");
    scopeCache.set(key, range);
    return range;
}

export function createLiveModeExtension(plugin: AnnotationPlugin): Extension {
    return EditorView.decorations.compute(["doc", "selection"], (state) => {
        if (!plugin.settings.enableLiveRendering) {
            return Decoration.none;
        }

        try {
            const content = state.doc.toString();
            const cursorPos = state.selection.main.head;
            const selStart = state.selection.main.from;
            const selEnd = state.selection.main.to;

            const file = plugin.app.workspace.getActiveFile();
            if (!file) return Decoration.none;

            const annotations = plugin.bridge.parseAnnotations(content);
            const entries: DecorationEntry[] = [];
            const isFootnoteMode = plugin.settings.inlineDisplayMode === "footnote";

            let markerIndex = 0;

            for (const ann of annotations) {
                const start = ann.char_start;
                const end = ann.char_end;

                if (isInEditableRange(start, end, cursorPos, selStart, selEnd)) continue;
                if (start < 0 || end > state.doc.length || start >= end) continue;

                if (ann.annotation_type === "mark") {
                    // Marks are display-only: hide the comment and apply
                    // persistent styling to the resolved scope range instead
                    // of rendering a widget. When the scope cannot be
                    // resolved (e.g. a stale anchor), fall through to the
                    // normal widget so the annotation stays discoverable.
                    const range = resolveMarkScope(plugin.bridge, content, ann);
                    if (range && range.start < range.end && range.end <= state.doc.length) {
                        entries.push({ start, end, decoration: Decoration.replace({}) });
                        const attributes: Record<string, string> = {};
                        if (ann.body) {
                            // A mark's note surfaces as a native tooltip
                            attributes["title"] = ann.body;
                        }
                        entries.push({
                            start: range.start,
                            end: range.end,
                            decoration: Decoration.mark({
                                class: `annotation-mark annotation-mark-${ann.mark}`,
                                attributes,
                            }),
                        });
                        continue;
                    }
                }

                let widget;
                if (ann.form === "block") {
                    // Block annotations always get callout widget
                    widget = new CalloutWidget(
                        ann, start, end,
                        plugin.app, file.path, plugin,
                    );
                } else if (isFootnoteMode) {
                    // Compact annotations in footnote mode get marker widget
                    widget = new MarkerWidget(ann, start, end, markerIndex++, plugin.bridge);
                } else {
                    // Compact annotations in default mode get pill widget
                    widget = new PillWidget(
                        ann, start, end,
                        plugin.app, file.path, plugin, plugin.bridge,
                    );
                }

                entries.push({
                    start,
                    end,
                    decoration: Decoration.replace({ widget }),
                });
            }

            // Decoration.set(..., true) sorts by from AND startSide — mark
            // ranges and replace ranges can share a `from`, which a plain
            // sort-by-start + RangeSetBuilder would reject.
            return Decoration.set(
                entries.map((e) => e.decoration.range(e.start, e.end)),
                true,
            );
        } catch (e) {
            console.error("[Annotation] Live-mode decoration error:", e);
            return Decoration.none;
        }
    });
}

export { isInEditableRange } from "./editable-range";
