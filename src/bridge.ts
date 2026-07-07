import { FileSystemAdapter } from "obsidian";

export interface Annotation {
    form: "compact" | "block";
    id: string | null;
    annotation_type: "note" | "question" | "todo" | "crossref" | "apparatus" | "translation" | "llm" | "thread" | "bare";
    certainty: "tentative" | "firm" | "neutral";
    scope:
        | { kind: "words"; value: number }
        | { kind: "paragraph"; value: number }
        | { kind: "page"; value: number }
        | { kind: "sentence"; value: number }
        | { kind: "anchor"; value: string }
        | { kind: "section" }
        | { kind: "document" }
        | { kind: "asym_words"; value: [number, number] }
        | { kind: "asym_sentence"; value: [number, number] }
        | { kind: "asym_paragraph"; value: [number, number] }
        | { kind: "asym_page"; value: [number, number] };
    body: string | null;
    date: string | null;
    char_start: number;
    char_end: number;
    original: string;
}

export interface ScopeRange {
    start: number;
    end: number;
}

/** How a symmetric scope extends from the annotation position. */
export type ResolutionMode = "backward" | "bidirectional";

export class WasmBridge {
    private initialized = false;
    private parseAnnotationsFn: ((content: string) => string) | null = null;
    private resolveScopeRangeFn: ((content: string, charStart: number, charEnd: number, scopeJson: string, lang: string, mode: string) => string) | null = null;

    async init(pluginDir: string, adapter: FileSystemAdapter): Promise<void> {
        if (this.initialized) return;

        const wasmPath = `${pluginDir}/annotation_wasm_bg.wasm`;
        const wasmBinary = await adapter.readBinary(wasmPath);

        const wasmModule = await import("../crates/wasm/pkg/annotation_wasm");
        wasmModule.initSync({ module: wasmBinary });
        this.parseAnnotationsFn = wasmModule.parse_annotations;
        this.resolveScopeRangeFn = wasmModule.resolve_scope_range;

        this.initialized = true;
        console.log("[Annotation] WASM initialized successfully");
    }

    parseAnnotations(content: string): Annotation[] {
        if (!this.initialized || !this.parseAnnotationsFn) {
            throw new Error("[Annotation] WASM not initialized. Call init() first.");
        }
        return JSON.parse(this.parseAnnotationsFn(content));
    }

    resolveScopeRange(
        content: string,
        charStart: number,
        charEnd: number,
        scope: Annotation["scope"],
        lang: string,
        mode: ResolutionMode = "backward",
    ): ScopeRange | null {
        if (!this.initialized || !this.resolveScopeRangeFn) {
            return null;
        }
        const result = this.resolveScopeRangeFn(content, charStart, charEnd, JSON.stringify(scope), lang, mode);
        if (result === "null") return null;
        return JSON.parse(result);
    }
}
