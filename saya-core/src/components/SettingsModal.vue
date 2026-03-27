<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useUiStore } from "../stores/ui";
import { useSettingsStore } from "../stores/settings";
import AiConfigTab from "./settings/AiConfigTab.vue";
import AccountsTab from "./settings/AccountsTab.vue";
import ContextAxesTab from "./settings/ContextAxesTab.vue";
import PluginsTab from "./settings/PluginsTab.vue";
import SyncTab from "./settings/SyncTab.vue";
import GeneralTab from "./settings/GeneralTab.vue";

const uiStore = useUiStore();
const settingsStore = useSettingsStore();

const activeTab = ref("ai");

interface Tab {
  id: string;
  label: string;
}

const tabs: Tab[] = [
  { id: "ai", label: "AI Configuration" },
  { id: "accounts", label: "Accounts" },
  { id: "axes", label: "Context Axes" },
  { id: "plugins", label: "Plugins" },
  { id: "sync", label: "Sync" },
  { id: "general", label: "General" },
];

function close() {
  uiStore.toggleSettings();
}

function selectTab(id: string) {
  activeTab.value = id;
}

onMounted(() => {
  settingsStore.initTheme();
});
</script>

<template>
  <Teleport to="body">
    <div v-if="uiStore.isSettingsOpen" class="modal-overlay" @click.self="close">
      <div class="modal">
        <header class="modal-header">
          <h2>Settings</h2>
          <button class="close-btn" @click="close">&times;</button>
        </header>
        <div class="modal-content">
          <nav class="tab-nav">
            <button
              v-for="tab in tabs"
              :key="tab.id"
              class="tab-btn"
              :class="{ active: activeTab === tab.id }"
              @click="selectTab(tab.id)"
            >
              <span class="tab-label">{{ tab.label }}</span>
            </button>
          </nav>
          <div class="tab-panel">
            <AiConfigTab v-if="activeTab === 'ai'" />
            <AccountsTab v-if="activeTab === 'accounts'" />
            <ContextAxesTab v-if="activeTab === 'axes'" />
            <PluginsTab v-if="activeTab === 'plugins'" />
            <SyncTab v-if="activeTab === 'sync'" />
            <GeneralTab v-if="activeTab === 'general'" />
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 1000;
}

.modal {
  width: 640px;
  max-height: 80vh;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.modal-header h2 {
  font-size: 15px;
  font-weight: 600;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 20px;
  cursor: pointer;
  transition: background-color 150ms;
}

.close-btn:hover {
  background-color: var(--bg-hover);
}

.modal-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.tab-nav {
  display: flex;
  flex-direction: column;
  width: 180px;
  border-right: 1px solid var(--border);
  padding: 8px;
  gap: 2px;
  flex-shrink: 0;
  background-color: var(--bg-sidebar);
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: background-color 150ms, color 150ms;
  text-align: left;
}

.tab-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.tab-btn.active {
  background-color: var(--bg-card);
  color: var(--text-primary);
  font-weight: 500;
}

.tab-label {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-panel {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}
</style>
