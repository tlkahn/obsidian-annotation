import { EditorView, WidgetType } from "@codemirror/view";
import { App, MarkdownRenderer, Component } from "obsidian";
import type { Annotation } from "../bridge";
import { WasmBridge } from "../bridge";
import { dispatchScopeHighlight, clearScopeHighlight } from "./scope-highlight";

/** Type → display info mapping. */
const TYPE_INFO: Record<string, { label: string; color: string }> = {
    note:       { label: "Note",        color: "86, 154, 222" },
    question:   { label: "Question",    color: "236, 177, 0" },
    todo:       { label: "Todo",        color: "72, 198, 123" },
    crossref:   { label: "Cross-ref",   color: "168, 130, 214" },
    apparatus:  { label: "Apparatus",   color: "198, 120, 95" },
    translation:{ label: "Translation", color: "100, 180, 160" },
    bare:       { label: "Annotation",  color: "136, 136, 136" },
};

function getTypeInfo(type: string) {
    return TYPE_INFO[type] ?? TYPE_INFO["bare"];
}

function certLabel(certainty: string): string {
    if (certainty === "tentative") return " (tentative)";
    if (certainty === "firm") return " (important)";
    return "";
}

/** Check if a click target is inside a rendered link (wikilink or external). */
function isLinkClick(e: MouseEvent): boolean {
    const target = e.target as HTMLElement;
    return !!target.closest("a.internal-link, a.external-link, a[href]");
}

/** Add mouseenter/mouseleave handlers to highlight the annotation's scope range. */
function addScopeHoverHandlers(
    el: HTMLElement,
    view: EditorView,
    annotation: Annotation,
    charStart: number,
    bridge: WasmBridge,
): void {
    el.addEventListener("mouseenter", () => {
        const content = view.state.doc.toString();
        const range = bridge.resolveScopeRange(content, charStart, annotation.scope, "en");
        if (range && range.start < range.end) {
            dispatchScopeHighlight(view, range.start, range.end);
        }
    });
    el.addEventListener("mouseleave", () => {
        clearScopeHighlight(view);
    });
}

/**
 * Foldable callout-like widget for block annotations.
 */
export class CalloutWidget extends WidgetType {
    private collapsed = false;

    constructor(
        private readonly annotation: Annotation,
        private readonly charStart: number,
        private readonly charEnd: number,
        private readonly app: App,
        private readonly sourcePath: string,
        private readonly component: Component,
    ) {
        super();
    }

    toDOM(view: EditorView): HTMLElement {
        const info = getTypeInfo(this.annotation.annotation_type);
        const wrapper = document.createElement("div");
        wrapper.className = "annotation-callout";
        wrapper.style.setProperty("--callout-color", info.color);

        // Header
        const header = wrapper.createDiv({ cls: "annotation-callout-title" });

        const titleText = header.createSpan({ cls: "annotation-callout-title-text" });
        titleText.textContent = info.label + certLabel(this.annotation.certainty);

        if (this.annotation.certainty === "tentative") {
            titleText.addClass("annotation-certainty-tentative");
        } else if (this.annotation.certainty === "firm") {
            titleText.addClass("annotation-certainty-firm");
        }

        // Date (right side of header)
        if (this.annotation.date) {
            const dateEl = header.createSpan({ cls: "annotation-callout-date" });
            dateEl.textContent = this.annotation.date;
        }

        // Fold toggle
        const foldIcon = header.createSpan({ cls: "annotation-callout-fold" });
        foldIcon.textContent = "▾";

        // Body (collapsible)
        const body = wrapper.createDiv({ cls: "annotation-callout-content" });
        if (this.annotation.body) {
            MarkdownRenderer.render(
                this.app,
                this.annotation.body,
                body,
                this.sourcePath,
                this.component,
            );
        }

        // Fold toggle click
        header.addEventListener("mousedown", (e) => {
            e.preventDefault();
            e.stopPropagation();
            this.collapsed = !this.collapsed;
            body.style.display = this.collapsed ? "none" : "";
            foldIcon.textContent = this.collapsed ? "▸" : "▾";
        });

        // Body click → expand to raw source (unless clicking a link)
        body.addEventListener("mousedown", (e) => {
            if (isLinkClick(e)) return; // let Obsidian handle link navigation
            e.preventDefault();
            e.stopPropagation();
            const charStart = this.charStart;
            setTimeout(() => {
                view.dispatch({ selection: { anchor: charStart } });
                view.focus();
            }, 0);
        });

        return wrapper;
    }

    eq(other: CalloutWidget): boolean {
        return (
            this.annotation.original === other.annotation.original &&
            this.charStart === other.charStart &&
            this.charEnd === other.charEnd
        );
    }
}

/**
 * Inline colored pill widget for compact annotations (default mode).
 */
export class PillWidget extends WidgetType {
    constructor(
        private readonly annotation: Annotation,
        private readonly charStart: number,
        private readonly charEnd: number,
        private readonly app: App,
        private readonly sourcePath: string,
        private readonly component: Component,
        private readonly bridge: WasmBridge,
    ) {
        super();
    }

    toDOM(view: EditorView): HTMLElement {
        const info = getTypeInfo(this.annotation.annotation_type);
        const wrapper = document.createElement("span");
        wrapper.className = `annotation-pill annotation-pill-${this.annotation.annotation_type}`;
        wrapper.style.setProperty("--callout-color", info.color);

        // Certainty mark
        if (this.annotation.certainty === "tentative") {
            const cert = wrapper.createSpan({ cls: "annotation-pill-certainty annotation-certainty-tentative" });
            cert.textContent = "?";
        } else if (this.annotation.certainty === "firm") {
            const cert = wrapper.createSpan({ cls: "annotation-pill-certainty annotation-certainty-firm" });
            cert.textContent = "!";
        }

        // Body (rendered markdown, truncated)
        if (this.annotation.body) {
            const bodyEl = wrapper.createSpan({ cls: "annotation-pill-body" });
            const truncated = this.annotation.body.length > 60
                ? this.annotation.body.slice(0, 60) + "…"
                : this.annotation.body;
            MarkdownRenderer.render(
                this.app,
                truncated,
                bodyEl,
                this.sourcePath,
                this.component,
            );
        }

        // Click → expand to raw source (unless clicking a link)
        const charStart = this.charStart;
        wrapper.addEventListener("mousedown", (e) => {
            if (isLinkClick(e)) return; // let Obsidian handle link navigation
            e.preventDefault();
            e.stopPropagation();
            setTimeout(() => {
                view.dispatch({ selection: { anchor: charStart } });
                view.focus();
            }, 0);
        });

        // Hover → highlight scoped text
        addScopeHoverHandlers(wrapper, view, this.annotation, this.charStart, this.bridge);

        return wrapper;
    }

    eq(other: PillWidget): boolean {
        return (
            this.annotation.original === other.annotation.original &&
            this.charStart === other.charStart &&
            this.charEnd === other.charEnd
        );
    }
}

/**
 * Superscript marker widget for inline annotations in footnote mode.
 */
export class MarkerWidget extends WidgetType {
    constructor(
        private readonly annotation: Annotation,
        private readonly charStart: number,
        private readonly charEnd: number,
        private readonly index: number,
        private readonly bridge: WasmBridge,
    ) {
        super();
    }

    toDOM(view: EditorView): HTMLElement {
        const info = getTypeInfo(this.annotation.annotation_type);
        const wrapper = document.createElement("sup");
        wrapper.className = "annotation-marker";
        wrapper.style.setProperty("--callout-color", info.color);

        // Type letter + certainty mark
        const label = info.label.charAt(0);
        const certMark = this.annotation.certainty === "tentative" ? "?"
            : this.annotation.certainty === "firm" ? "!" : "";
        wrapper.textContent = label + certMark;

        // Click → scroll to entry in side panel
        const charStart = this.charStart;
        wrapper.addEventListener("mousedown", (e) => {
            e.preventDefault();
            e.stopPropagation();
            setTimeout(() => {
                view.dispatch({ selection: { anchor: charStart } });
                view.focus();
            }, 0);
        });

        // Hover → highlight scoped text
        addScopeHoverHandlers(wrapper, view, this.annotation, this.charStart, this.bridge);

        return wrapper;
    }

    eq(other: MarkerWidget): boolean {
        return (
            this.annotation.original === other.annotation.original &&
            this.charStart === other.charStart &&
            this.charEnd === other.charEnd
        );
    }
}
