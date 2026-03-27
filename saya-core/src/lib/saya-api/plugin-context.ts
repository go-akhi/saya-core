import type { PluginManifest } from "./types";
import { createSayaApi, type SayaApi } from "./index";

export interface PluginContext {
  api: SayaApi;
  manifest: PluginManifest;
  pluginName: string;
}

export function initPlugin(pluginName: string, iframe: HTMLIFrameElement): PluginContext {
  const api = createSayaApi(pluginName);
  api.connect(iframe);

  return {
    api,
    manifest: {
      name: pluginName,
      display_name: "",
      columns: [],
      ai_actions: [],
      provides_actions: [],
    },
    pluginName,
  };
}

export function destroyPlugin(context: PluginContext): void {
  context.api.disconnect();
}
