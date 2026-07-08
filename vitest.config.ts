import { defineConfig } from "vitest/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
    resolve: {
        alias: {
            // The obsidian package is types-only (no runtime entry); point it
            // at a stub so widget modules can be imported in tests.
            obsidian: fileURLToPath(new URL("./src/__tests__/obsidian-stub.ts", import.meta.url)),
        },
    },
    test: {
        include: ["src/**/__tests__/**/*.test.ts"],
    },
});
