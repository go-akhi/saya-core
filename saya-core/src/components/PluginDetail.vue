<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { useMarketplaceStore, type MarketplacePlugin } from "../stores/marketplace";
import { renderMarkdown } from "../lib/markdown";

const props = defineProps<{
  plugin: MarketplacePlugin;
  installed: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "install-success"): void;
}>();

const marketplaceStore = useMarketplaceStore();
const readmeHtml = ref("");
const isLoadingReadme = ref(false);

const repoParts = computed(() => {
  const url = props.plugin.repo_url.replace("https://github.com/", "");
  const parts = url.split("/");
  return {
    owner: parts[0],
    repo: parts[1],
  };
});

onMounted(async () => {
  isLoadingReadme.value = true;
  try {
    const readme = await marketplaceStore.fetchReadme(repoParts.value.owner, repoParts.value.repo);
    readmeHtml.value = renderMarkdown(readme);
  } catch (e) {
    readmeHtml.value = "<p>Failed to load README</p>";
  } finally {
    isLoadingReadme.value = false;
  }
});

async function handleInstall() {
  const success = await marketplaceStore.installPlugin(props.plugin.repo_url);
  if (success) {
    emit("install-success");
  }
}

function openRepo() {
  window.open(props.plugin.repo_url, "_blank");
}
</script>

<template>
  <div class="detail-overlay" @click.self="emit('close')">
    <div class="detail-modal">
      <div class="detail-header">
        <div class="plugin-header-info">
          <div class="plugin-icon">{{ plugin.icon }}</div>
          <div class="plugin-title">
            <h3>{{ plugin.display_name }}</h3>
            <span class="plugin-version">v{{ plugin.version }}</span>
            <span v-if="plugin.verified" class="verified-badge">&#10003; Verified</span>
            <span v-else class="community-badge">Community</span>
          </div>
        </div>
        <button class="close-btn" @click="emit('close')">&#x2715;</button>
      </div>

      <div class="detail-content">
        <div v-if="isLoadingReadme" class="readme-loading">
          <span class="spinner"></span>
          Loading README...
        </div>
        <div v-else class="readme-content" v-html="readmeHtml"></div>
      </div>

      <div class="detail-footer">
        <button class="repo-btn" @click="openRepo">
          View on GitHub
        </button>
        <button
          v-if="installed"
          class="install-btn installed"
          disabled
        >
          Installed
        </button>
        <button
          v-else
          class="install-btn"
          :disabled="marketplaceStore.isLoading"
          @click="handleInstall"
        >
          {{ marketplaceStore.isLoading ? "Installing..." : "Install Plugin" }}
        </button>
      </div>

      <div v-if="marketplaceStore.error" class="error-message">
        {{ marketplaceStore.error }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.detail-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1001;
}

.detail-modal {
  background-color: var(--bg-card, #1e1e1e);
  border: 1px solid var(--border, #333);
  border-radius: var(--radius-large, 12px);
  width: 90%;
  max-width: 700px;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.detail-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border, #333);
  background-color: var(--bg-sidebar, #252526);
}

.plugin-header-info {
  display: flex;
  gap: 12px;
}

.plugin-icon {
  font-size: 32px;
  line-height: 1;
  flex-shrink: 0;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-card, #1e1e1e);
  border-radius: var(--radius, 8px);
}

.plugin-title h3 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
  color: var(--text-primary, #e0e0e0);
}

.plugin-version {
  font-size: 12px;
  color: var(--text-muted, #666);
  margin-left: 8px;
}

.verified-badge {
  display: inline-block;
  margin-left: 8px;
  padding: 2px 8px;
  font-size: 10px;
  font-weight: 500;
  color: #4ec9b0;
  background-color: rgba(78, 201, 176, 0.15);
  border-radius: 10px;
}

.community-badge {
  display: inline-block;
  margin-left: 8px;
  padding: 2px 8px;
  font-size: 10px;
  font-weight: 500;
  color: #dcdcaa;
  background-color: rgba(220, 220, 170, 0.15);
  border-radius: 10px;
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

.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.readme-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 32px;
  color: var(--text-secondary, #aaa);
}

.spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border, #333);
  border-top-color: var(--accent, #0078d4);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.readme-content {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary, #e0e0e0);
}

.readme-content :deep(h1),
.readme-content :deep(h2),
.readme-content :deep(h3) {
  margin-top: 24px;
  margin-bottom: 12px;
  color: var(--text-primary, #e0e0e0);
}

.readme-content :deep(h1) { font-size: 24px; }
.readme-content :deep(h2) { font-size: 20px; }
.readme-content :deep(h3) { font-size: 16px; }

.readme-content :deep(p) {
  margin-bottom: 12px;
}

.readme-content :deep(code) {
  padding: 2px 6px;
  background-color: var(--bg-hover, #2d2d2d);
  border-radius: 4px;
  font-family: monospace;
  font-size: 13px;
}

.readme-content :deep(pre) {
  padding: 12px;
  background-color: var(--bg-hover, #2d2d2d);
  border-radius: var(--radius, 4px);
  overflow-x: auto;
}

.readme-content :deep(pre code) {
  padding: 0;
  background: none;
}

.readme-content :deep(ul),
.readme-content :deep(ol) {
  padding-left: 24px;
  margin-bottom: 12px;
}

.readme-content :deep(a) {
  color: var(--accent, #0078d4);
}

.detail-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid var(--border, #333);
  background-color: var(--bg-sidebar, #252526);
}

.repo-btn {
  padding: 8px 16px;
  border: 1px solid var(--border, #333);
  border-radius: var(--radius, 4px);
  background: transparent;
  color: var(--text-secondary, #aaa);
  cursor: pointer;
  font-size: 14px;
  transition: background-color 150ms, border-color 150ms;
}

.repo-btn:hover {
  background-color: var(--bg-hover, #3c3c3c);
  border-color: var(--text-secondary, #aaa);
  color: var(--text-primary, #e0e0e0);
}

.install-btn {
  padding: 8px 20px;
  border: none;
  border-radius: var(--radius, 4px);
  background-color: var(--accent, #0078d4);
  color: white;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: background-color 150ms;
}

.install-btn:hover:not(:disabled) {
  background-color: var(--accent-hover, #106ebe);
}

.install-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.install-btn.installed {
  background-color: #4ec9b0;
  cursor: default;
}

.error-message {
  padding: 8px 20px;
  background-color: rgba(244, 71, 71, 0.15);
  color: #f44747;
  font-size: 13px;
}
</style>
