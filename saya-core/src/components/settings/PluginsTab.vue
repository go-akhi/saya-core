<script setup lang="ts">
import { ref, onMounted } from "vue";
import { usePluginStore } from "../../stores/plugins";

interface PluginColumn {
  name: string;
  display: string;
  type: string;
  dtype: string;
  sortable: boolean;
}

const store = usePluginStore();
const expandedPlugin = ref<string | null>(null);

function toggleExpand(name: string) {
  expandedPlugin.value = expandedPlugin.value === name ? null : name;
}

async function toggleEnabled(name: string) {
  await store.toggleEnabled(name);
}

onMounted(() => store.loadAllPlugins());
</script>

<template>
  <div class="settings-tab">
    <div class="tab-header">
      <h3>Plugins</h3>
    </div>

    <p v-if="store.pluginsLoading" class="status-text">Loading...</p>
    <p v-else-if="store.allPlugins.length === 0" class="status-text">
      No plugins installed. Place plugin directories in the plugins folder.
    </p>

    <div class="plugin-list">
      <div v-for="plugin in store.allPlugins" :key="plugin.name" class="plugin-card">
        <div class="plugin-header" @click="toggleExpand(plugin.name)">
          <div class="plugin-left">
            <span v-if="plugin.icon" class="plugin-icon">{{ plugin.icon }}</span>
            <div class="plugin-info">
              <div class="plugin-name">
                {{ plugin.display_name }}
                <span class="plugin-version">v{{ plugin.version }}</span>
              </div>
              <div v-if="!plugin.is_enabled" class="plugin-disabled">Disabled</div>
            </div>
          </div>
          <div class="plugin-right" @click.stop>
            <label class="toggle-switch" :title="plugin.is_enabled ? 'Disable' : 'Enable'">
              <input
                type="checkbox"
                :checked="plugin.is_enabled"
                @change="toggleEnabled(plugin.name)"
              />
              <span class="toggle-track" />
            </label>
            <button class="expand-btn" :class="{ expanded: expandedPlugin === plugin.name }">
              &#9660;
            </button>
          </div>
        </div>

        <div v-if="expandedPlugin === plugin.name" class="plugin-details">
          <div v-if="plugin.columns && plugin.columns.length > 0" class="detail-section">
            <h4>Columns</h4>
            <div class="column-list">
              <div v-for="col in (plugin.columns as PluginColumn[])" :key="col.name" class="column-chip">
                <span class="col-name">{{ col.display }}</span>
                <span class="col-type">{{ col.dtype }}</span>
              </div>
            </div>
          </div>
          <div v-if="plugin.ai_actions && plugin.ai_actions.length > 0" class="detail-section">
            <h4>AI Actions</h4>
            <div class="action-list">
              <div v-for="action in plugin.ai_actions" :key="action.id" class="action-chip">
                {{ action.label }}
              </div>
            </div>
          </div>
          <div v-if="plugin.provides_actions && plugin.provides_actions.length > 0" class="detail-section">
            <h4>Provided Actions</h4>
            <div class="action-list">
              <div v-for="action in plugin.provides_actions" :key="action.handler" class="action-chip">
                {{ action.label }}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-tab {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.tab-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.tab-header h3 {
  font-size: 14px;
  font-weight: 600;
}

.status-text {
  color: var(--text-muted);
  font-size: 13px;
}

.plugin-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.plugin-card {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
  overflow: hidden;
}

.plugin-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  cursor: pointer;
  transition: background-color 150ms;
}

.plugin-header:hover {
  background-color: var(--bg-hover);
}

.plugin-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.plugin-icon {
  font-size: 20px;
}

.plugin-info {
  display: flex;
  flex-direction: column;
}

.plugin-name {
  font-size: 13px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 6px;
}

.plugin-version {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 400;
}

.plugin-disabled {
  font-size: 11px;
  color: #d97706;
}

.plugin-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toggle-switch {
  position: relative;
  cursor: pointer;
}

.toggle-switch input {
  display: none;
}

.toggle-track {
  display: block;
  width: 32px;
  height: 18px;
  border-radius: 9px;
  background-color: var(--bg-badge);
  transition: background-color 150ms;
  position: relative;
}

.toggle-track::after {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background-color: white;
  box-shadow: 0 1px 2px rgba(0,0,0,0.15);
  transition: transform 150ms;
}

.toggle-switch input:checked + .toggle-track {
  background-color: var(--accent);
}

.toggle-switch input:checked + .toggle-track::after {
  transform: translateX(14px);
}

.expand-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-muted);
  font-size: 10px;
  cursor: pointer;
  transition: transform 200ms;
  display: flex;
  align-items: center;
  justify-content: center;
}

.expand-btn.expanded {
  transform: rotate(180deg);
}

.plugin-details {
  padding: 0 12px 12px;
  border-top: 1px solid var(--border);
}

.detail-section {
  margin-top: 10px;
}

.detail-section h4 {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.3px;
  margin-bottom: 6px;
}

.column-list,
.action-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.column-chip,
.action-chip {
  font-size: 12px;
  padding: 3px 8px;
  border-radius: 4px;
  background-color: var(--bg-sidebar);
  border: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 6px;
}

.col-name {
  font-weight: 500;
}

.col-type {
  color: var(--text-muted);
  font-size: 11px;
}
</style>
