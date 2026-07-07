import { keymap, EditorView } from "@codemirror/view";
import { Extension } from "@codemirror/state";
import type AnnotationPlugin from "../main";
import type { Annotation } from "../bridge";
import { isInEditableRange } from "./editable-range";

export function findAnnotationAtCursor(
    plugin: AnnotationPlugin,
    content: string,
    cursorPos: number,
): Annotation | null {
    const annotations = plugin.bridge.parseAnnotations(content);
    for (const ann of annotations) {
        if (isInEditableRange(ann.char_start, ann.char_end, cursorPos, cursorPos, cursorPos)) {
            return ann;
        }
    }
    return null;
}

export function createEscapeAnnotationExtension(plugin: AnnotationPlugin): Extension {
    return keymap.of([{
        key: "Escape",
        run: (view: EditorView): boolean => {
            const sel = view.state.selection.main;
            if (sel.from !== sel.to) return false;

            const cursorPos = sel.head;
            const content = view.state.doc.toString();
            const ann = findAnnotationAtCursor(plugin, content, cursorPos);
            if (!ann) return false;

            // Must clear the buffer=1 zone used by isInEditableRange
            let target = ann.char_end + 2;
            if (target > content.length) {
                target = Math.max(0, ann.char_start - 2);
            }

            view.dispatch({ selection: { anchor: target } });
            return true;
        },
    }]);
}
