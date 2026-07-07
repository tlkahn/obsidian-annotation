import { describe, it, expect } from "vitest";
import { isInEditableRange } from "../renderer/editable-range";

// Characterization tests: the editable-range logic works on absolute
// UTF-16 offsets from the scanner, so it is independent of delimiter
// length (`<!--- --->` vs the legacy `<!-- -->`) and of an [id] prefix
// (which sits inside char_start..char_end).

describe("isInEditableRange", () => {
    // "<!--- n: _ | note --->" at offsets [10, 32)
    const start = 10;
    const end = 32;
    const cursor = (pos: number) => isInEditableRange(start, end, pos, pos, pos);

    it("cursor inside the annotation is editable", () => {
        expect(cursor(15)).toBe(true);
        expect(cursor(start)).toBe(true);
        expect(cursor(end)).toBe(true);
    });

    it("buffer=1 zone around the annotation is editable", () => {
        expect(cursor(start - 1)).toBe(true);
        expect(cursor(end + 1)).toBe(true);
    });

    it("outside the buffer zone is not editable", () => {
        expect(cursor(start - 2)).toBe(false);
        expect(cursor(end + 2)).toBe(false);
    });

    it("ESC target char_end + 2 clears the buffer zone", () => {
        // escape-annotation.ts moves the cursor to char_end + 2; that
        // position must not re-enter the editable range
        expect(cursor(end + 2)).toBe(false);
    });

    it("selection overlapping the annotation is editable", () => {
        expect(isInEditableRange(start, end, 0, 5, 15)).toBe(true);
        expect(isInEditableRange(start, end, 0, 20, 40)).toBe(true);
    });

    it("selection outside the annotation is not editable", () => {
        expect(isInEditableRange(start, end, 0, 0, 5)).toBe(false);
        expect(isInEditableRange(start, end, 0, end + 2, end + 10)).toBe(false);
    });

    it("selection touching only the buffer boundary is not editable", () => {
        // expandedEnd = end + 1; a selection starting exactly there does not overlap
        expect(isInEditableRange(start, end, 0, end + 1, end + 5)).toBe(false);
        // expandedStart = start - 1; a selection ending exactly there does not overlap
        expect(isInEditableRange(start, end, 0, 2, start - 1)).toBe(false);
    });
});
