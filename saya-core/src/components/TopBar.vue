<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useAxesStore } from "../stores/axes";
import { useUiStore } from "../stores/ui";
import AddContextAxis from "./AddContextAxis.vue";

const axesStore = useAxesStore();
const uiStore = useUiStore();

onMounted(() => axesStore.loadAxes());

type Platform = "mac" | "windows" | "linux";

const platform = computed<Platform>(() => {
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac") || navigator.platform.toLowerCase().includes("mac")) {
    return "mac";
  }
  if (ua.includes("win") || navigator.platform.toLowerCase().includes("win")) {
    return "windows";
  }
  return "linux";
});

let appWindow: { minimize: () => Promise<void>; maximize: () => Promise<void>; unmaximize: () => Promise<void>; close: () => Promise<void>; isMaximized: () => Promise<boolean>; startDragging: () => Promise<void> } | null = null;
let maximized = false;

async function initWindow() {
  try {
    const { Window } = await import("@tauri-apps/api/window");
    appWindow = Window.getCurrent();
    maximized = await appWindow.isMaximized();
  } catch {
    appWindow = null;
  }
}

async function minimize() {
  await appWindow?.minimize();
}

async function toggleMaximize() {
  if (!appWindow) return;
  if (maximized) {
    await appWindow.unmaximize();
    maximized = false;
  } else {
    await appWindow.maximize();
    maximized = true;
  }
}

async function close() {
  await appWindow?.close();
}

initWindow();
</script>

<template>
  <header class="top-bar" data-tauri-drag-region>
    <div class="window-controls" :class="{ 'mac': platform === 'mac' }">
      <!-- macOS style -->
      <template v-if="platform === 'mac'">
        <button class="window-btn mac close" title="Close" @click="close">
          <span class="btn-icon">&#10005;</span>
        </button>
        <button class="window-btn mac minimize" title="Minimize" @click="minimize">
          <span class="btn-icon">&#10095;</span>
        </button>
        <button class="window-btn mac maximize" title="Maximize" @click="toggleMaximize">
          <span class="btn-icon">&#9633;</span>
        </button>
      </template>

      <!-- Windows/Linux style -->
      <template v-else>
        <button class="window-btn winlinux minimize" title="Minimize" @click="minimize">
          <span class="btn-icon">&#x2212;</span>
        </button>
        <button class="window-btn winlinux maximize" title="Maximize" @click="toggleMaximize">
          <span class="btn-icon">{{ maximized ? '&#x2752;' : '&#x25A1;' }}</span>
        </button>
        <button class="window-btn winlinux close" title="Close" @click="close">
          <span class="btn-icon">&#x2715;</span>
        </button>
      </template>
    </div>

    <div class="top-bar-left">
      <button class="batch-ai-btn" title="Saya Agent - Batch AI">
        <span class="star">&#9734;</span>
        <span class="btn-label">Saya Agent</span>
      </button>
      <div class="top-bar-divider" />
    </div>

    <nav class="filter-tabs">
      <button
        v-for="axis in axesStore.contextAxes"
        :key="axis.id"
        class="filter-tab"
        :class="{ active: axesStore.activeContextAxis === axis.id }"
        @click="axesStore.setActiveContextAxis(axis.id)"
      >
        <span v-if="axis.icon" class="tab-icon">{{ axis.icon }}</span>
        <span class="tab-label">{{ axis.name }}</span>
        <span v-if="axis.color" class="tab-indicator" :style="{ backgroundColor: axis.color }" />
      </button>
      <AddContextAxis />
    </nav>

    <div class="top-bar-right">
      <button class="icon-btn" title="Help">?</button>
      <button class="icon-btn" title="Settings" @click="uiStore.toggleSettings()">&#9881;</button>
    </div>
  </header>
</template>

<style scoped>
.top-bar {
  display: flex;
  align-items: stretch;
  height: 40px;
  background-color: var(--bg-bar);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  cursor: grab;
  user-select: none;
}

.top-bar:active {
  cursor: grabbing;
}

.window-controls {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px;
  height: 100%;
  -webkit-app-region: no-drag;
  cursor: default;
  order: 1;
}

/* macOS - controls on left */
.window-controls.mac {
  order: 0;
  margin-right: auto;
}

/* macOS style buttons */
.window-btn.mac {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
  border: none;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
  transition: opacity 150ms;
}

.window-btn.mac:hover {
  opacity: 0.8;
}

.window-btn.mac .btn-icon {
  font-size: 8px;
  color: var(--text-secondary);
  line-height: 1;
}

.window-btn.mac.close {
  background-color: #ff5f57;
}

.window-btn.mac.close:hover {
  background-color: #ff4136;
}

.window-btn.mac.minimize {
  background-color: #febc2e;
}

.window-btn.mac.minimize:hover {
  background-color: #fead21;
}

.window-btn.mac.maximize {
  background-color: #28c840;
}

.window-btn.mac.maximize:hover {
  background-color: #20bd4a;
}

/* Windows/Linux style buttons */
.window-btn.winlinux {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 100%;
  border: none;
  background: transparent;
  cursor: pointer;
  transition: background-color 150ms;
}

.window-btn.winlinux:hover {
  background-color: var(--bg-hover);
}

.window-btn.winlinux .btn-icon {
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1;
}

.window-btn.winlinux:hover .btn-icon {
  color: var(--text-primary);
}

.window-btn.winlinux.close:hover {
  background-color: #e81123;
}

.window-btn.winlinux.close:hover .btn-icon {
  color: white;
}

.filter-tabs {
  display: flex;
  align-items: stretch;
  gap: 1px;
  padding: 0 4px;
}

.filter-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 16px;
  border: none;
  border-radius: 8px 8px 0 0;
  background: var(--bg-sidebar);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  position: relative;
  transition: background-color 150ms, color 150ms;
}

.filter-tab:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.filter-tab.active {
  background-color: var(--bg-primary);
  color: var(--text-primary);
}

.filter-tab.active::after {
  content: "";
  position: absolute;
  bottom: -1px;
  left: 0;
  right: 0;
  height: 2px;
  background-color: var(--bg-primary);
}

.tab-icon {
  font-size: 14px;
}

.tab-label {
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.top-bar-divider {
  width: 1px;
  background-color: var(--border);
  margin: 4px 8px;
}

.top-bar-left {
  display: flex;
  align-items: center;
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 16px;
  cursor: pointer;
  transition: background-color 150ms, color 150ms;
}

.icon-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.batch-ai-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 100%;
  padding: 0 12px;
  border: none;
  border-radius: 0;
  background-color: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  font-family: "DM Sans", sans-serif;
  cursor: pointer;
  transition: color 150ms, background-color 150ms;
}

.batch-ai-btn:hover {
  background-color: var(--bg-hover);
  color: var(--accent);
}

.batch-ai-btn .star {
  font-size: 16px;
}

.batch-ai-btn .btn-label {
  letter-spacing: 0.5px;
  text-transform: uppercase;
}

.top-bar-right {
  display: flex;
  align-items: center;
  gap: 4px;
  padding-right: 8px;
  margin-left: auto;
}
</style>
