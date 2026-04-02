export interface PluginSettings {
    enableLiveRendering: boolean;
    inlineDisplayMode: "pill" | "footnote";
}

export const DEFAULT_SETTINGS: PluginSettings = {
    enableLiveRendering: true,
    inlineDisplayMode: "pill",
};
