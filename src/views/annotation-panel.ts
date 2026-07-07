import { ItemView, MarkdownRenderer, WorkspaceLeaf, TFile } from "obsidian";
import type AnnotationPlugin from "../main";
import type { Annotation } from "../bridge";

export const ANNOTATION_PANEL_VIEW_TYPE = "annotation-panel";

const TYPE_INFO: Record<string, { label: string; color: string }> = {
    note:       { label: "Note",       color: "86, 154, 222" },
    question:   { label: "Question",   color: "236, 177, 0" },
    todo:       { label: "Todo",       color: "72, 198, 123" },
    crossref:   { label: "Cross-ref",  color: "168, 130, 214" },
    apparatus:  { label: "Apparatus",  color: "198, 120, 95" },
    translation:{ label: "Translation",color: "100, 180, 160" },
    llm:        { label: "LLM",        color: "214, 93, 177" },
    thread:     { label: "Thread",     color: "240, 130, 40" },
    bare:       { label: "Annotation", color: "136, 136, 136" },
};

export class AnnotationPanelView extends ItemView {
    private plugin: AnnotationPlugin;

    constructor(leaf: WorkspaceLeaf, plugin: AnnotationPlugin) {
        super(leaf);
        this.plugin = plugin;
    }

    getViewType(): string {
        return ANNOTATION_PANEL_VIEW_TYPE;
    }

    getDisplayText(): string {
        return "Annotations";
    }

    getIcon(): string {
        return "lucide-message-square";
    }

    async onOpen() {
        this.renderPanel();
    }

    async onClose() {
        this.contentEl.empty();
    }

    public renderPanel() {
        const container = this.contentEl;
        container.empty();
        container.addClass("annotation-panel");

        const file = this.app.workspace.getActiveFile();
        if (!file) {
            container.createEl("p", {
                text: "No active file.",
                cls: "annotation-panel-empty",
            });
            return;
        }

        let content: string;
        try {
            // Use cached read for performance
            const cache = this.app.vault.getFileByPath(file.path);
            if (!cache) return;
            // We need sync access; use the bridge which already has the content
            // For the panel, re-read the document
            content = "";
        } catch {
            return;
        }

        // Read file content and parse annotations
        this.app.vault.cachedRead(file).then((text) => {
            this.renderAnnotations(container, text, file);
        });
    }

    private renderAnnotations(container: HTMLElement, content: string, file: TFile) {
        container.empty();

        let annotations: Annotation[];
        try {
            annotations = this.plugin.bridge.parseAnnotations(content);
        } catch {
            container.createEl("p", {
                text: "Failed to parse annotations.",
                cls: "annotation-panel-empty",
            });
            return;
        }

        if (annotations.length === 0) {
            container.createEl("p", {
                text: "No annotations in this file.",
                cls: "annotation-panel-empty",
            });
            return;
        }

        for (const ann of annotations) {
            const info = TYPE_INFO[ann.annotation_type] ?? TYPE_INFO["bare"];
            const entry = container.createDiv({ cls: "annotation-panel-entry" });
            entry.style.setProperty("--callout-color", info.color);

            // Header: type + certainty + date
            const header = entry.createDiv({ cls: "annotation-panel-entry-header" });
            const typeLabel = header.createSpan({ cls: "annotation-panel-entry-type" });
            typeLabel.textContent = info.label;
            if (ann.certainty === "tentative") {
                typeLabel.textContent += " (tentative)";
                typeLabel.addClass("annotation-certainty-tentative");
            } else if (ann.certainty === "firm") {
                typeLabel.textContent += " (important)";
                typeLabel.addClass("annotation-certainty-firm");
            }

            if (ann.date) {
                const dateEl = header.createSpan({ cls: "annotation-panel-entry-date" });
                dateEl.textContent = ann.date;
            }

            // Line number
            const lineNum = content.substring(0, ann.char_start).split("\n").length;
            const lineEl = header.createSpan({ cls: "annotation-panel-entry-line" });
            lineEl.textContent = `L${lineNum}`;

            // Body (full, markdown-rendered)
            if (ann.body) {
                const body = entry.createDiv({ cls: "annotation-panel-entry-body" });
                MarkdownRenderer.render(
                    this.app,
                    ann.body,
                    body,
                    file.path,
                    this.plugin,
                );
            }

            // Click → navigate to annotation in editor
            const charStart = ann.char_start;
            entry.addEventListener("click", () => {
                const leaf = this.app.workspace.getMostRecentLeaf();
                if (leaf) {
                    // @ts-ignore - accessing CM6 editor view
                    const editor = leaf.view?.editor;
                    if (editor) {
                        const pos = editor.offsetToPos(charStart);
                        editor.setCursor(pos);
                        editor.scrollIntoView(
                            { from: pos, to: pos },
                            true,
                        );
                    }
                }
            });
        }
    }
}
