import { FileSystemAdapter } from "obsidian";

export interface Annotation {
    form: "compact" | "block";
    annotation_type: "note" | "question" | "todo" | "crossref" | "apparatus" | "bare";
    certainty: "tentative" | "firm" | "neutral";
    scope:
        | { kind: "words"; value: number }
        | { kind: "paragraph" }
        | { kind: "preceding_paragraph" }
        | { kind: "anchor"; value: string }
        | { kind: "adjacency" };
    body: string | null;
    date: string | null;
    char_start: number;
    char_end: number;
    original: string;
}

export class WasmBridge {
    private initialized = false;
    private parseAnnotationsFn: ((content: string) => string) | null = null;

    async init(pluginDir: string, adapter: FileSystemAdapter): Promise<void> {
        if (this.initialized) return;

        const wasmPath = `${pluginDir}/annotation_wasm_bg.wasm`;
        const wasmBinary = await adapter.readBinary(wasmPath);

        const wasmModule = await import("../crates/wasm/pkg/annotation_wasm");
        wasmModule.initSync({ module: wasmBinary });
        this.parseAnnotationsFn = wasmModule.parse_annotations;

        this.initialized = true;
        console.log("[Annotation] WASM initialized successfully");
    }

    parseAnnotations(content: string): Annotation[] {
        if (!this.initialized || !this.parseAnnotationsFn) {
            throw new Error("[Annotation] WASM not initialized. Call init() first.");
        }
        return JSON.parse(this.parseAnnotationsFn(content));
    }
}
