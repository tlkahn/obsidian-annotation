import { FileSystemAdapter, Plugin } from "obsidian";
import { WasmBridge } from "./bridge";
import { DEFAULT_SETTINGS, PluginSettings } from "./config";
import { createLiveModeExtension } from "./renderer/live-mode";
import { scopeHighlightExtension } from "./renderer/scope-highlight";
import { AnnotationSettingTab } from "./settings";
import { AnnotationPanelView, ANNOTATION_PANEL_VIEW_TYPE } from "./views/annotation-panel";

export default class AnnotationPlugin extends Plugin {
    settings: PluginSettings = DEFAULT_SETTINGS;
    bridge: WasmBridge = new WasmBridge();

    async onload() {
        console.log("[Annotation] Loading plugin...");
        await this.loadSettings();

        // Initialize WASM bridge
        const adapter = this.app.vault.adapter;
        if (adapter instanceof FileSystemAdapter) {
            try {
                await this.bridge.init(this.manifest.dir!, adapter);
            } catch (e) {
                console.error("[Annotation] Failed to initialize WASM:", e);
                return;
            }
        } else {
            console.warn("[Annotation] Not a desktop environment, WASM unavailable.");
            return;
        }

        // Live editing-mode renderer
        this.registerEditorExtension(createLiveModeExtension(this));
        this.registerEditorExtension(scopeHighlightExtension());

        // Register annotation side panel view
        this.registerView(
            ANNOTATION_PANEL_VIEW_TYPE,
            (leaf) => new AnnotationPanelView(leaf, this),
        );

        // Update side panel when active file changes
        this.registerEvent(
            this.app.workspace.on("file-open", () => {
                this.refreshAnnotationPanel();
            }),
        );

        // Update side panel when document content changes
        this.registerEvent(
            this.app.metadataCache.on("changed", (file) => {
                const activeFile = this.app.workspace.getActiveFile();
                if (activeFile && file.path === activeFile.path) {
                    this.refreshAnnotationPanel();
                }
            }),
        );

        // Settings tab
        this.addSettingTab(new AnnotationSettingTab(this.app, this));

        console.log("[Annotation] Plugin loaded.");
    }

    onunload() {
        console.log("[Annotation] Plugin unloaded.");
    }

    /** Open or close the annotation side panel based on settings. */
    async toggleAnnotationPanel(show: boolean) {
        const existing = this.app.workspace.getLeavesOfType(ANNOTATION_PANEL_VIEW_TYPE);
        if (show && existing.length === 0) {
            const leaf = this.app.workspace.getRightLeaf(false);
            if (leaf) {
                await leaf.setViewState({
                    type: ANNOTATION_PANEL_VIEW_TYPE,
                    active: true,
                });
            }
        } else if (!show) {
            for (const leaf of existing) {
                leaf.detach();
            }
        }
    }

    /** Refresh the annotation panel content. */
    private refreshAnnotationPanel() {
        const leaves = this.app.workspace.getLeavesOfType(ANNOTATION_PANEL_VIEW_TYPE);
        for (const leaf of leaves) {
            const view = leaf.view as AnnotationPanelView;
            view.renderPanel();
        }
    }

    async loadSettings() {
        this.settings = Object.assign({}, DEFAULT_SETTINGS, await this.loadData());
    }

    async saveSettings() {
        await this.saveData(this.settings);
    }
}
