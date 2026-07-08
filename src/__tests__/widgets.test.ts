// @vitest-environment happy-dom
import { describe, it, expect, vi, beforeEach } from "vitest";
import type { EditorView } from "@codemirror/view";
import type { Annotation, WasmBridge } from "../bridge";

// The obsidian package ships types only (no runtime); vitest.config.ts
// aliases it to ./obsidian-stub.ts so widget modules can be imported here.
import { CalloutWidget } from "../renderer/widgets";

// Obsidian augments HTMLElement with DOM helper methods; widgets.ts relies
// on createDiv/createSpan/addClass, so polyfill them for happy-dom.
(HTMLElement.prototype as any).createDiv = function (opts?: { cls?: string }) {
    const el = document.createElement("div");
    if (opts?.cls) el.className = opts.cls;
    this.appendChild(el);
    return el;
};
(HTMLElement.prototype as any).createSpan = function (opts?: { cls?: string }) {
    const el = document.createElement("span");
    if (opts?.cls) el.className = opts.cls;
    this.appendChild(el);
    return el;
};
(HTMLElement.prototype as any).addClass = function (...cls: string[]) {
    this.classList.add(...cls);
};

const docText = "Some scoped sentence. <!---\nn\n---\nbody\n--->";

function makeAnnotation(): Annotation {
    return {
        form: "block",
        id: null,
        mark: null,
        annotation_type: "note",
        certainty: "neutral",
        scope: { kind: "sentence", value: 1 },
        body: "body",
        date: null,
        char_start: 22,
        char_end: docText.length,
        original: docText.slice(22),
    };
}

function makeView(): EditorView {
    return {
        state: { doc: { toString: () => docText, length: docText.length } },
        dispatch: vi.fn(),
        focus: vi.fn(),
    } as unknown as EditorView;
}

function makeBridge(): WasmBridge {
    return {
        resolveScopeRange: vi.fn(() => ({ start: 0, end: 21 })),
    } as unknown as WasmBridge;
}

describe("CalloutWidget scope hover", () => {
    let annotation: Annotation;
    let view: EditorView;
    let bridge: WasmBridge;
    let header: HTMLElement;

    beforeEach(() => {
        annotation = makeAnnotation();
        view = makeView();
        bridge = makeBridge();
        const widget = new CalloutWidget(
            annotation,
            annotation.char_start,
            annotation.char_end,
            new (class {})() as any, // App stub
            "note.md",
            new (class {})() as any, // Component stub
            bridge,
        );
        const wrapper = widget.toDOM(view);
        header = wrapper.querySelector(".annotation-callout-title") as HTMLElement;
        expect(header).toBeTruthy();
    });

    it("callout header hover dispatches scope highlight", () => {
        header.dispatchEvent(new Event("mouseenter"));

        expect(bridge.resolveScopeRange).toHaveBeenCalledWith(
            docText,
            annotation.char_start,
            annotation.char_end,
            annotation.scope,
            "en",
        );
        expect(view.dispatch).toHaveBeenCalled();
    });

    it("callout header mouseleave clears highlight", () => {
        header.dispatchEvent(new Event("mouseenter"));
        const callsAfterEnter = (view.dispatch as ReturnType<typeof vi.fn>).mock.calls.length;

        header.dispatchEvent(new Event("mouseleave"));

        expect((view.dispatch as ReturnType<typeof vi.fn>).mock.calls.length).toBe(callsAfterEnter + 1);
    });
});
