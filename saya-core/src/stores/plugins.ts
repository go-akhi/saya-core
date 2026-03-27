import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface PluginInfo {
  name: string;
  display_name: string;
  icon: string | null;
  version: string;
  is_enabled: boolean;
  columns?: unknown[];
  ai_actions?: AiAction[];
  provides_actions?: ProvidedAction[];
}

export interface AiAction {
  id: string;
  label: string;
  context_columns: string[];
  result_mapping: {
    cognitive_axis: string;
    context_axis: string;
  };
}

export interface ProvidedAction {
  label: string;
  target_types: string[];
  handler: string;
  field_mapping?: {
    action_title: string;
    cognitive_axis: string;
    context_axis: string;
    source_type: string;
    source_id: string;
  };
}

export interface PluginManifest {
  name: string;
  display_name: string;
  icon?: string;
  columns: unknown[];
  ai_actions: AiAction[];
  provides_actions: ProvidedAction[];
  has_settings?: boolean;
}

export const usePluginStore = defineStore("plugins", () => {
  const plugins = ref<PluginInfo[]>([]);
  const pluginManifests = ref<Record<string, PluginManifest>>({});
  const activePlugin = ref<string | null>(null);
  const isCollapsed = ref(false);
  const sidebarWidth = ref(200);
  const isResizing = ref(false);

  const minWidth = 48;
  const maxWidth = 320;

  function setActivePlugin(name: string | null) {
    activePlugin.value = name;
  }

  function toggleCollapse() {
    isCollapsed.value = !isCollapsed.value;
  }

  function setSidebarWidth(width: number) {
    sidebarWidth.value = Math.max(minWidth, Math.min(maxWidth, width));
  }

  function startResize() {
    isResizing.value = true;
  }

  function endResize() {
    isResizing.value = false;
  }

  function activePluginHasSettings(): boolean {
    if (!activePlugin.value) return false;
    const manifest = pluginManifests.value[activePlugin.value];
    return manifest?.has_settings === true;
  }

  const allPlugins = ref<PluginInfo[]>([]);
  const pluginsLoading = ref(false);

  async function loadAllPlugins() {
    pluginsLoading.value = true;
    try {
      allPlugins.value = await invoke<PluginInfo[]>("get_all_plugins");
    } catch (e) {
      console.error("Failed to load plugins:", e);
    } finally {
      pluginsLoading.value = false;
    }
  }

  async function toggleEnabled(pluginName: string) {
    try {
      const newState = await invoke<boolean>("toggle_plugin_enabled", { pluginName });
      const idx = allPlugins.value.findIndex((p) => p.name === pluginName);
      if (idx !== -1) {
        allPlugins.value[idx].is_enabled = newState;
      }
    } catch (e) {
      throw e;
    }
  }

  return {
    plugins,
    pluginManifests,
    activePlugin,
    isCollapsed,
    sidebarWidth,
    isResizing,
    minWidth,
    maxWidth,
    allPlugins,
    pluginsLoading,
    setActivePlugin,
    toggleCollapse,
    setSidebarWidth,
    startResize,
    endResize,
    activePluginHasSettings,
    loadAllPlugins,
    toggleEnabled,
  };
});
