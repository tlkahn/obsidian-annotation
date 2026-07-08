// Runtime stub for the "obsidian" package, which ships type definitions only.
// Wired in via a vitest alias (see vitest.config.ts) so that modules importing
// "obsidian" can load under vitest. Provides just the pieces the code under
// test touches.
export class App {}
export class Component {}
export class FileSystemAdapter {}
export const MarkdownRenderer = {
    render: async (): Promise<void> => {},
};
