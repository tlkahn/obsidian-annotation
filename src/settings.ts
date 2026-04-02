import { App, PluginSettingTab, Setting } from "obsidian";
import type AnnotationPlugin from "./main";

export class AnnotationSettingTab extends PluginSettingTab {
    plugin: AnnotationPlugin;

    constructor(app: App, plugin: AnnotationPlugin) {
        super(app, plugin);
        this.plugin = plugin;
    }

    display(): void {
        const { containerEl } = this;
        containerEl.empty();

        new Setting(containerEl)
            .setName("Live rendering")
            .setDesc("Render annotation comments as widgets in edit mode")
            .addToggle((toggle) =>
                toggle
                    .setValue(this.plugin.settings.enableLiveRendering)
                    .onChange(async (value) => {
                        this.plugin.settings.enableLiveRendering = value;
                        await this.plugin.saveSettings();
                    })
            );

        new Setting(containerEl)
            .setName("Inline display mode")
            .setDesc("How compact (inline) annotations are displayed")
            .addDropdown((dropdown) =>
                dropdown
                    .addOption("pill", "Colored pill (inline)")
                    .addOption("footnote", "Footnote marker + side panel")
                    .setValue(this.plugin.settings.inlineDisplayMode)
                    .onChange(async (value) => {
                        this.plugin.settings.inlineDisplayMode = value as "pill" | "footnote";
                        await this.plugin.saveSettings();
                        await this.plugin.toggleAnnotationPanel(value === "footnote");
                    })
            );
    }
}
