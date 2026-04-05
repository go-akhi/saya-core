<script setup lang="ts">
import TopBar from "./components/TopBar.vue";
import CognitiveAxis from "./components/CognitiveAxis.vue";
import PluginSidebar from "./components/PluginSidebar.vue";
import PluginHost from "./components/PluginHost.vue";
import ActionsBar from "./components/ActionsBar.vue";
import SettingsModal from "./components/SettingsModal.vue";
import ErrorNotification from "./components/ErrorNotification.vue";
import { setShowErrorCallback } from "./lib/core-message-handler";
import { useUiStore } from "./stores/ui";

const uiStore = useUiStore();

const debugLog: string[] = [];

setShowErrorCallback((payload, pluginName) => {
  debugLog.push(`showError: ${payload.message} (${pluginName})`);
  uiStore.showError({
    id: `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
    title: payload.title,
    message: payload.message,
    type: payload.type || "error",
    pluginName,
  });
});

window.addEventListener("message", (e) => {
  const msg = e.data;
  if (msg?.source === "plugin" && msg?.type === "show_error") {
    debugLog.push(`Received show_error: ${JSON.stringify(msg.payload)}`);
  }
});
</script>

<template>
  <div class="app">
    <div v-if="debugLog.length" class="debug-panel">
      <div v-for="(log, i) in debugLog" :key="i">{{ log }}</div>
    </div>
    <TopBar />
    <div class="main-area">
      <CognitiveAxis />
      <PluginSidebar />
      <PluginHost />
      <ActionsBar />
    </div>
    <SettingsModal />
    <ErrorNotification />
  </div>
</template>

<style>
@import url("https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=DM+Sans:wght@400;500;600;700&display=swap");

:root {
  --bg-primary: #faf9f7;
  --bg-bar: #ffffff;
  --bg-sidebar: #f5f4f1;
  --bg-card: #ffffff;
  --bg-hover: #f0efec;
  --bg-badge: #e8e7e4;
  --text-primary: #1a1a1a;
  --text-secondary: #6b6a67;
  --text-muted: #a09f9c;
  --border: #e5e4e1;
  --accent: #d97706;
  --accent-hover: #b45309;
  --radius: 6px;
  --radius-lg: 10px;
  --font-sans: "Inter", system-ui, -apple-system, sans-serif;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body {
  height: 100%;
  overflow: hidden;
}

body {
  font-family: var(--font-sans);
  background-color: transparent;
  color: var(--text-primary);
  font-size: 14px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#app {
  height: 100%;
}

.app {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-primary);
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 0 0 1px rgba(128, 128, 128, 0.15);
}

.main-area {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.debug-panel {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  background: #1a1a1a;
  color: #00ff00;
  font-family: monospace;
  font-size: 12px;
  padding: 8px;
  z-index: 9999;
  max-height: 150px;
  overflow-y: auto;
}
</style>
