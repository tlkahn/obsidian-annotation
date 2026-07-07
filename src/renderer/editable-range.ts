/**
 * Whether the cursor/selection is close enough to an annotation that its
 * raw source should be shown for editing. Offsets are absolute UTF-16
 * positions from the scanner, so the logic is independent of delimiter
 * length (`<!--- --->`) and of an `[id]` prefix inside the comment.
 *
 * Standalone module (no CM6/Obsidian imports) so it is unit-testable.
 */
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
