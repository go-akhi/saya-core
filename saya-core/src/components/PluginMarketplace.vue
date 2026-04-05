<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useMarketplaceStore, type MarketplacePlugin } from "../stores/marketplace";
import { usePluginStore } from "../stores/plugins";
import PluginDetail from "./PluginDetail.vue";

const emit = defineEmits<{
  (e: "close"): void;
}>();

const marketplaceStore = useMarketplaceStore();
const pluginStore = usePluginStore();

const selectedPlugin = ref<MarketplacePlugin | null>(null);
const showDetail = ref(false);

const installedPluginNames = computed(() => {
  return pluginStore.allPlugins.map((p) => p.name);
});

onMounted(async () => {
  await Promise.all([
    marketplaceStore.fetchRegistry(),
    pluginStore.loadAllPlugins(),
  ]);
});

function selectPlugin(plugin: MarketplacePlugin) {
  selectedPlugin.value = plugin;
  showDetail.value = true;
}

function closeDetail() {
  showDetail.value = false;
  selectedPlugin.value = null;
}

async function handleInstallSuccess() {
  // Re-discover to register newly installed plugin in DB, then reload
  await invoke("discover_plugins");
  await pluginStore.loadAllPlugins();
  closeDetail();
}

async function handleUninstallSuccess() {
  await pluginStore.loadAllPlugins();
  closeDetail();
}
</script>

<template>
  <div class="marketplace-modal" @click.self="emit('close')">
    <div class="marketplace-container">
      <div class="marketplace-header">
        <h2>Plugin Marketplace</h2>
        <button class="close-btn" @click="emit('close')">&#x2715;</button>
      </div>

      <div v-if="marketplaceStore.isLoading && marketplaceStore.plugins.length === 0" class="loading">
        <span class="spinner"></span>
        <p>Loading marketplace...</p>
      </div>

      <div v-else-if="marketplaceStore.error" class="error-state">
        <p class="error-message">{{ marketplaceStore.error }}</p>
        <button class="retry-btn" @click="marketplaceStore.fetchRegistry()">
          Retry
        </button>
      </div>

      <div v-else class="marketplace-content">
        <div v-if="marketplaceStore.verifiedPlugins.length > 0" class="plugin-section">
          <h3 class="section-title">
            <span class="verified-icon">&#10003;</span>
            Verified Plugins
          </h3>
          <div class="plugin-grid">
            <div
              v-for="plugin in marketplaceStore.verifiedPlugins"
              :key="plugin.name"
              class="plugin-card"
              @click="selectPlugin(plugin)"
            >
              <div class="plugin-icon">{{ plugin.icon }}</div>
              <div class="plugin-info">
                <div class="plugin-name">
                  {{ plugin.display_name }}
                  <span class="verified-badge">&#10003;</span>
                </div>
                <p class="plugin-description">{{ plugin.description }}</p>
                <span v-if="marketplaceStore.isInstalled(plugin.name, installedPluginNames)" class="installed-badge">
                  Installed
                </span>
              </div>
            </div>
          </div>
        </div>

        <div v-if="marketplaceStore.communityPlugins.length > 0" class="plugin-section">
          <h3 class="section-title">
            <span class="community-icon">&#9733;</span>
            Community Plugins
          </h3>
          <div class="plugin-grid">
            <div
              v-for="plugin in marketplaceStore.communityPlugins"
              :key="plugin.name"
              class="plugin-card"
              @click="selectPlugin(plugin)"
            >
              <div class="plugin-icon">{{ plugin.icon }}</div>
              <div class="plugin-info">
                <div class="plugin-name">
                  {{ plugin.display_name }}
                  <span class="community-badge">Community</span>
                </div>
                <p class="plugin-description">{{ plugin.description }}</p>
                <span v-if="marketplaceStore.isInstalled(plugin.name, installedPluginNames)" class="installed-badge">
                  Installed
                </span>
              </div>
            </div>
          </div>
        </div>

        <div v-if="marketplaceStore.plugins.length === 0" class="empty-state">
          <p>No plugins available in the marketplace.</p>
        </div>
      </div>

      <PluginDetail
        v-if="selectedPlugin && showDetail"
        :plugin="selectedPlugin"
        :installed="marketplaceStore.isInstalled(selectedPlugin.name, installedPluginNames)"
        @close="closeDetail"
        @install-success="handleInstallSuccess"
        @uninstall-success="handleUninstallSuccess"
      />
    </div>
  </div>
</template>

<style scoped>
.marketplace-modal {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.marketplace-container {
  background-color: var(--bg-card, #1e1e1e);
  border: 1px solid var(--border, #333);
  border-radius: var(--radius-large, 12px);
  width: 90%;
  max-width: 700px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.marketplace-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border, #333);
  background-color: var(--bg-sidebar, #252526);
}

.marketplace-header h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary, #e0e0e0);
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius, 4px);
  background: transparent;
  color: var(--text-secondary, #aaa);
  cursor: pointer;
  font-size: 14px;
  transition: background-color 150ms;
}

.close-btn:hover {
  background-color: var(--bg-hover, #3c3c3c);
  color: var(--text-primary, #e0e0e0);
}

.loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px;
  gap: 16px;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border, #333);
  border-top-color: var(--accent, #0078d4);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading p {
  margin: 0;
  color: var(--text-secondary, #aaa);
}

.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px;
  gap: 16px;
}

.error-message {
  margin: 0;
  color: #f44747;
  text-align: center;
}

.retry-btn {
  padding: 8px 16px;
  border: none;
  border-radius: var(--radius, 4px);
  background-color: var(--accent, #0078d4);
  color: white;
  cursor: pointer;
  font-size: 14px;
  transition: background-color 150ms;
}

.retry-btn:hover {
  background-color: var(--accent-hover, #106ebe);
}

.marketplace-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.plugin-section {
  margin-bottom: 24px;
}

.plugin-section:last-child {
  margin-bottom: 0;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 12px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary, #aaa);
}

.verified-icon {
  color: #4ec9b0;
}

.community-icon {
  color: #dcdcaa;
}

.plugin-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 12px;
}

.plugin-card {
  display: flex;
  gap: 12px;
  padding: 12px;
  background-color: var(--bg-hover, #2d2d2d);
  border: 1px solid var(--border, #333);
  border-radius: var(--radius, 8px);
  cursor: pointer;
  transition: background-color 150ms, border-color 150ms;
}

.plugin-card:hover {
  background-color: var(--bg-active, #383838);
  border-color: var(--accent, #0078d4);
}

.plugin-icon {
  font-size: 24px;
  line-height: 1;
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-card, #1e1e1e);
  border-radius: var(--radius, 4px);
}

.plugin-info {
  flex: 1;
  min-width: 0;
}

.plugin-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #e0e0e0);
}

.verified-badge {
  font-size: 10px;
  color: #4ec9b0;
  font-weight: normal;
}

.community-badge {
  font-size: 10px;
  color: #dcdcaa;
  font-weight: normal;
}

.plugin-description {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-secondary, #aaa);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.installed-badge {
  display: inline-block;
  margin-top: 6px;
  padding: 2px 8px;
  font-size: 10px;
  font-weight: 500;
  color: #4ec9b0;
  background-color: rgba(78, 201, 176, 0.15);
  border-radius: 10px;
}

.empty-state {
  text-align: center;
  padding: 48px;
  color: var(--text-secondary, #aaa);
}
</style>
