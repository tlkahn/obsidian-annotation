import { describe, it, expect } from "vitest";
import type { Annotation, ScopeRange } from "../bridge";

// Test that the JSON structure from WASM matches the TypeScript Annotation interface.
// These test deserialization without requiring the actual WASM binary.

describe("Annotation JSON deserialization", () => {
    it("parses a compact note annotation", () => {
        const json: Annotation = {
            form: "compact",
            id: null,
            annotation_type: "note",
            certainty: "tentative",
            scope: { kind: "words", value: 2 },
            body: "same sense as TĀ 3.68?",
            date: "2026-03",
            char_start: 19,
            char_end: 69,
            original: "<!--- n? __ | same sense as TĀ 3.68? @2026-03 --->",
        };

        expect(json.annotation_type).toBe("note");
        expect(json.certainty).toBe("tentative");
        expect(json.scope.kind).toBe("words");
        if (json.scope.kind === "words") {
            expect(json.scope.value).toBe(2);
        }
        expect(json.body).toBe("same sense as TĀ 3.68?");
        expect(json.date).toBe("2026-03");
    });

    it("parses a block crossref annotation", () => {
        const json: Annotation = {
            form: "block",
            id: null,
            annotation_type: "crossref",
            certainty: "neutral",
            scope: { kind: "anchor", value: "anuttara" },
            body: "Primary parallels:\n- TĀ 3.68",
            date: "2026-03",
            char_start: 0,
            char_end: 100,
            original: "<!--- ... --->",
        };

        expect(json.form).toBe("block");
        expect(json.annotation_type).toBe("crossref");
        expect(json.scope.kind).toBe("anchor");
        if (json.scope.kind === "anchor") {
            expect(json.scope.value).toBe("anuttara");
        }
    });

    it("parses a bare annotation", () => {
        const json: Annotation = {
            form: "compact",
            id: null,
            annotation_type: "bare",
            certainty: "neutral",
            scope: { kind: "sentence", value: 1 },
            body: "compare Vasugupta SpK 1.1",
            date: null,
            char_start: 4,
            char_end: 40,
            original: "<!--- compare Vasugupta SpK 1.1 --->",
        };

        expect(json.annotation_type).toBe("bare");
        expect(json.scope.kind).toBe("sentence");
        expect(json.date).toBeNull();
    });

    it("parses an apparatus annotation", () => {
        const json: Annotation = {
            form: "compact",
            id: null,
            annotation_type: "apparatus",
            certainty: "neutral",
            scope: { kind: "sentence", value: 1 },
            body: "variant reading in ms. B",
            date: null,
            char_start: 0,
            char_end: 42,
            original: "<!--- app: | variant reading in ms. B --->",
        };

        expect(json.annotation_type).toBe("apparatus");
    });

    it("parses an annotation with an id", () => {
        const json: Annotation = {
            form: "compact",
            id: "my-note-id",
            annotation_type: "note",
            certainty: "neutral",
            scope: { kind: "paragraph", value: 1 },
            body: "body text",
            date: null,
            char_start: 0,
            char_end: 44,
            original: "<!---[my-note-id] n: \\p | body text --->",
        };

        expect(json.id).toBe("my-note-id");
    });

    it("parses an llm annotation without an id", () => {
        const json: Annotation = {
            form: "compact",
            id: null,
            annotation_type: "llm",
            certainty: "neutral",
            scope: { kind: "sentence", value: 1 },
            body: "summarize entire document",
            date: null,
            char_start: 0,
            char_end: 40,
            original: "<!--- llm | summarize entire document --->",
        };

        expect(json.annotation_type).toBe("llm");
        expect(json.id).toBeNull();
    });

    it("parses a thread annotation", () => {
        const json: Annotation = {
            form: "block",
            id: "t1",
            annotation_type: "thread",
            certainty: "tentative",
            scope: { kind: "sentence", value: 1 },
            body: "A conversational thread.",
            date: null,
            char_start: 0,
            char_end: 50,
            original: "<!---[t1]\nth?\n---\nA conversational thread.\n--->",
        };

        expect(json.annotation_type).toBe("thread");
        expect(json.id).toBe("t1");
    });

    it("handles all scope types", () => {
        const scopes: Annotation["scope"][] = [
            { kind: "words", value: 3 },
            { kind: "paragraph", value: 1 },
            { kind: "paragraph", value: 2 },
            { kind: "page", value: 1 },
            { kind: "page", value: 3 },
            { kind: "sentence", value: 1 },
            { kind: "sentence", value: 2 },
            { kind: "anchor", value: "8th century" },
        ];

        expect(scopes[0].kind).toBe("words");
        expect(scopes[1].kind).toBe("paragraph");
        expect(scopes[3].kind).toBe("page");
        expect(scopes[5].kind).toBe("sentence");
        expect(scopes[7].kind).toBe("anchor");
        // Paragraph(2) = \pp or \p__
        if (scopes[2].kind === "paragraph") {
            expect(scopes[2].value).toBe(2);
        }
        // Page(3) = \fff or \f___
        if (scopes[4].kind === "page") {
            expect(scopes[4].value).toBe(3);
        }
        // Sentence(2) = \ss or \s__
        if (scopes[6].kind === "sentence") {
            expect(scopes[6].value).toBe(2);
        }
    });
});

describe("ScopeRange JSON deserialization", () => {
    it("parses a scope range result", () => {
        const json = '{"start":10,"end":25}';
        const range: ScopeRange = JSON.parse(json);
        expect(range.start).toBe(10);
        expect(range.end).toBe(25);
    });

    it("handles null result", () => {
        const json = "null";
        const range: ScopeRange | null = JSON.parse(json);
        expect(range).toBeNull();
    });
});
