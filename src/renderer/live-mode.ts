import { EditorView, Decoration } from "@codemirror/view";
import { Extension, RangeSetBuilder } from "@codemirror/state";
import type AnnotationPlugin from "../main";
import { CalloutWidget, PillWidget, MarkerWidget } from "./widgets";

interface DecorationEntry {
    start: number;
    end: number;
    decoration: Decoration;
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

                let widget;
                if (ann.form === "block") {
                    // Block annotations always get callout widget
                    widget = new CalloutWidget(
                        ann, start, end,
                        plugin.app, file.path, plugin,
                    );
                } else if (isFootnoteMode) {
                    // Compact annotations in footnote mode get marker widget
                    widget = new MarkerWidget(ann, start, end, markerIndex++);
                } else {
                    // Compact annotations in default mode get pill widget
                    widget = new PillWidget(
                        ann, start, end,
                        plugin.app, file.path, plugin,
                    );
                }

                entries.push({
                    start,
                    end,
                    decoration: Decoration.replace({ widget }),
                });
            }

            entries.sort((a, b) => a.start - b.start);

            const builder = new RangeSetBuilder<Decoration>();
            for (const entry of entries) {
                builder.add(entry.start, entry.end, entry.decoration);
            }
            return builder.finish();
        } catch (e) {
            console.error("[Annotation] Live-mode decoration error:", e);
            return Decoration.none;
        }
    });
}

export function isInEditableRange(
    refStart: number,
    refEnd: number,
    cursorPos: number,
    selStart: number,
    selEnd: number
): boolean {
    const buffer = 1;
    const expandedStart = Math.max(0, refStart - buffer);
    const expandedEnd = refEnd + buffer;

    if (selStart !== selEnd) {
        return !(expandedEnd <= selStart || expandedStart >= selEnd);
    }

    return cursorPos >= expandedStart && cursorPos <= expandedEnd;
}
