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
        | { kind: "anchor"; value: string };
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

export class WasmBridge {
    private initialized = false;
    private parseAnnotationsFn: ((content: string) => string) | null = null;
    private resolveScopeRangeFn: ((content: string, charStart: number, scopeJson: string, lang: string) => string) | null = null;

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

    resolveScopeRange(content: string, charStart: number, scope: Annotation["scope"], lang: string): ScopeRange | null {
        if (!this.initialized || !this.resolveScopeRangeFn) {
            return null;
        }
        const result = this.resolveScopeRangeFn(content, charStart, JSON.stringify(scope), lang);
        if (result === "null") return null;
        return JSON.parse(result);
    }
}
