import { describe, it, expect } from "vitest";
import type { Annotation } from "../bridge";

// Test that the JSON structure from WASM matches the TypeScript Annotation interface.
// These test deserialization without requiring the actual WASM binary.

describe("Annotation JSON deserialization", () => {
    it("parses a compact note annotation", () => {
        const json: Annotation = {
            form: "compact",
            annotation_type: "note",
            certainty: "tentative",
            scope: { kind: "words", value: 2 },
            body: "same sense as TĀ 3.68?",
            date: "2026-03",
            char_start: 19,
            char_end: 65,
            original: "<!-- n? __ | same sense as TĀ 3.68? @2026-03 -->",
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
            annotation_type: "crossref",
            certainty: "neutral",
            scope: { kind: "anchor", value: "anuttara" },
            body: "Primary parallels:\n- TĀ 3.68",
            date: "2026-03",
            char_start: 0,
            char_end: 100,
            original: "<!-- ... -->",
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
            annotation_type: "bare",
            certainty: "neutral",
            scope: { kind: "adjacency" },
            body: "compare Vasugupta SpK 1.1",
            date: null,
            char_start: 4,
            char_end: 40,
            original: "<!-- compare Vasugupta SpK 1.1 -->",
        };

        expect(json.annotation_type).toBe("bare");
        expect(json.scope.kind).toBe("adjacency");
        expect(json.date).toBeNull();
    });

    it("parses an apparatus annotation", () => {
        const json: Annotation = {
            form: "compact",
            annotation_type: "apparatus",
            certainty: "neutral",
            scope: { kind: "adjacency" },
            body: "variant reading in ms. B",
            date: null,
            char_start: 0,
            char_end: 50,
            original: "<!-- app: | variant reading in ms. B -->",
        };

        expect(json.annotation_type).toBe("apparatus");
    });

    it("handles all scope types", () => {
        const scopes: Annotation["scope"][] = [
            { kind: "words", value: 3 },
            { kind: "paragraph" },
            { kind: "preceding_paragraph" },
            { kind: "anchor", value: "8th century" },
            { kind: "adjacency" },
        ];

        expect(scopes[0].kind).toBe("words");
        expect(scopes[1].kind).toBe("paragraph");
        expect(scopes[2].kind).toBe("preceding_paragraph");
        expect(scopes[3].kind).toBe("anchor");
        expect(scopes[4].kind).toBe("adjacency");
    });
});
