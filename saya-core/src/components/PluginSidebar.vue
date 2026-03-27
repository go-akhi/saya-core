<script setup lang="ts">
import { computed, ref } from "vue";
import { usePluginStore, type AiAction, type ProvidedAction } from "../stores/plugins";
import { invoke } from "@tauri-apps/api/core";
import { onMounted, onUnmounted } from "vue";

const pluginStore = usePluginStore();
const isLoading = ref(true);
const showPluginSettings = ref(false);

interface DiscoveredPlugin {
  name: string | null;
  display_name: string;
  icon: string | null;
  columns: unknown[];
  ai_actions: AiAction[];
  provides_actions: ProvidedAction[];
  valid: boolean;
  errors: string[];
}

const sortedPlugins = computed(() => {
  return [...pluginStore.plugins].sort((a, b) => {
    const order = ["email", "tasks", "notes"];
    const aIdx = order.indexOf(a.name);
    const bIdx = order.indexOf(b.name);
    if (aIdx === -1 && bIdx === -1) return a.display_name.localeCompare(b.display_name);
    if (aIdx === -1) return 1;
    if (bIdx === -1) return -1;
    return aIdx - bIdx;
  });
});

function selectPlugin(name: string) {
  if (pluginStore.activePlugin === name) {
    pluginStore.setActivePlugin(null);
  } else {
    pluginStore.setActivePlugin(name);
  }
}

function getPluginTooltip(plugin: { name: string; display_name: string; is_enabled: boolean }) {
  if (!plugin.is_enabled) {
    return `${plugin.display_name} (Invalid)`;
  }
  return plugin.display_name;
}

function startResize(e: MouseEvent) {
  e.preventDefault();
  pluginStore.startResize();
  document.addEventListener("mousemove", handleResize);
  document.addEventListener("mouseup", stopResize);
}

function handleResize(e: MouseEvent) {
  if (pluginStore.isResizing) {
    pluginStore.setSidebarWidth(e.clientX);
  }
}

function stopResize() {
  pluginStore.endResize();
  document.removeEventListener("mousemove", handleResize);
  document.removeEventListener("mouseup", stopResize);
}

onMounted(async () => {
  try {
    const results = await invoke<DiscoveredPlugin[]>("discover_plugins");
    const validPlugins = results.filter((p) => p.valid && p.name);
    
    pluginStore.plugins = validPlugins.map((p) => ({
      name: p.name!,
      display_name: p.display_name,
      icon: p.icon,
      version: "0.1.0",
      is_enabled: p.valid,
    }));
    
    validPlugins.forEach((p) => {
      if (p.name) {
        pluginStore.pluginManifests[p.name] = {
          name: p.name,
          display_name: p.display_name,
          icon: p.icon || undefined,
          columns: p.columns,
          ai_actions: p.ai_actions,
          provides_actions: p.provides_actions,
        };
      }
    });
  } catch (e) {
    console.error("Failed to load plugins:", e);
  } finally {
    isLoading.value = false;
  }
});

onUnmounted(() => {
  document.removeEventListener("mousemove", handleResize);
  document.removeEventListener("mouseup", stopResize);
});
</script>

<template>
  <aside
    class="plugin-sidebar"
    :class="{ collapsed: pluginStore.isCollapsed }"
    :style="{ width: pluginStore.isCollapsed ? '0px' : pluginStore.sidebarWidth + 'px' }"
  >
    <div class="sidebar-header">
      <button
        class="collapse-btn"
        :class="{ fading: pluginStore.isCollapsed }"
        title="Collapse sidebar"
        @click="pluginStore.toggleCollapse()"
      >
        <span class="collapse-icon">&lt;&lt;</span>
      </button>
    </div>

    <div class="sidebar-icons">
      <button
        v-for="plugin in sortedPlugins"
        :key="plugin.name"
        class="plugin-btn"
        :class="{ active: pluginStore.activePlugin === plugin.name }"
        :title="getPluginTooltip(plugin)"
        @click="selectPlugin(plugin.name)"
      >
        <span class="plugin-icon">{{ plugin.icon || "&#9679;" }}</span>
        <span v-if="!pluginStore.isCollapsed" class="plugin-label">{{ plugin.display_name }}</span>
      </button>
    </div>

    <div class="sidebar-footer">
      <button class="plugin-btn add-plugin-btn" title="Add Plugin">
        <span class="add-icon">+</span>
        <span v-if="!pluginStore.isCollapsed" class="plugin-label">Add Plugin</span>
      </button>
      <button
        class="plugin-btn settings-btn"
        :class="{ disabled: !pluginStore.activePlugin || !pluginStore.activePluginHasSettings() }"
        :title="pluginStore.activePlugin ? (pluginStore.activePluginHasSettings() ? `${pluginStore.activePlugin} Settings` : 'No settings for this plugin') : 'Select a plugin first'"
        :disabled="!pluginStore.activePlugin || !pluginStore.activePluginHasSettings()"
        @click="showPluginSettings = true"
      >
        <span class="settings-icon">&#9881;</span>
        <span v-if="!pluginStore.isCollapsed" class="plugin-label">
          {{ pluginStore.activePlugin ? pluginStore.pluginManifests[pluginStore.activePlugin]?.display_name + ' Settings' : 'Settings' }}
        </span>
      </button>
    </div>

    <div v-if="showPluginSettings && pluginStore.activePlugin" class="plugin-settings-popup">
      <div class="settings-header">
        <span class="settings-title">{{ pluginStore.pluginManifests[pluginStore.activePlugin]?.display_name }} Settings</span>
        <button class="close-btn" @click="showPluginSettings = false">&#x2715;</button>
      </div>
      <div class="settings-content">
        <p class="settings-placeholder">Plugin settings would be rendered here via iframe.</p>
        <p class="settings-plugin-name">{{ pluginStore.activePlugin }}</p>
      </div>
    </div>

    <div
      v-if="!pluginStore.isCollapsed"
      class="resize-handle"
      @mousedown="startResize"
    />
  </aside>
</template>

<style scoped>
.plugin-sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: var(--bg-sidebar);
  border-right: 1px solid var(--border);
  flex-shrink: 0;
  position: relative;
  transition: width 200ms ease;
  overflow: hidden;
}

.plugin-sidebar.collapsed {
  border-right-width: 0;
}

.sidebar-header {
  display: flex;
  justify-content: flex-end;
  padding: 4px;
  min-height: 40px;
  border-bottom: 1px solid var(--border);
}

.collapse-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  cursor: pointer;
  color: var(--text-muted);
  opacity: 1;
  transition: opacity 100ms ease;
}

.collapse-btn.fading {
  opacity: 0;
  pointer-events: none;
}

.collapse-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.collapse-icon {
  font-size: 10px;
}

.sidebar-icons {
  display: flex;
  flex-direction: column;
  padding: 8px 4px;
  flex: 1;
  gap: 2px;
}

.sidebar-footer {
  display: flex;
  flex-direction: column;
  padding: 8px 4px;
  border-top: 1px solid var(--border);
  gap: 2px;
}

.plugin-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  cursor: pointer;
  color: var(--text-secondary);
  transition: background-color 150ms, color 150ms;
  text-align: left;
  width: 100%;
}

.plugin-btn:hover {
  background-color: var(--bg-hover);
}

.plugin-btn.active {
  background-color: var(--bg-hover);
  color: var(--accent);
}

.plugin-btn.disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.plugin-btn.disabled:hover {
  background-color: transparent;
}

.plugin-icon {
  font-size: 18px;
  line-height: 1;
  flex-shrink: 0;
  width: 24px;
  text-align: center;
}

.plugin-label {
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.settings-icon {
  font-size: 16px;
  line-height: 1;
  flex-shrink: 0;
  width: 24px;
  text-align: center;
}

.add-icon {
  font-size: 20px;
  line-height: 1;
  flex-shrink: 0;
  width: 24px;
  text-align: center;
}

.resize-handle {
  position: absolute;
  top: 0;
  right: 0;
  width: 4px;
  height: 100%;
  cursor: col-resize;
  background: transparent;
  transition: background-color 150ms;
}

.resize-handle:hover {
  background-color: var(--accent);
}

.plugin-settings-popup {
  position: absolute;
  left: 100%;
  top: 50%;
  transform: translateY(-50%);
  margin-left: 8px;
  width: 320px;
  max-height: 400px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-large);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  z-index: 100;
  overflow: hidden;
}

.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background-color: var(--bg-sidebar);
}

.settings-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 12px;
  transition: background-color 150ms;
}

.close-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.settings-content {
  padding: 16px;
}

.settings-placeholder {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 0;
}

.settings-plugin-name {
  color: var(--text-muted);
  font-size: 12px;
  margin: 8px 0 0;
}
</style>
