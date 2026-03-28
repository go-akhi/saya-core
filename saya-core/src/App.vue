<script setup lang="ts">
import TopBar from "./components/TopBar.vue";
import CognitiveAxis from "./components/CognitiveAxis.vue";
import PluginSidebar from "./components/PluginSidebar.vue";
import PluginHost from "./components/PluginHost.vue";
import ActionsBar from "./components/ActionsBar.vue";
import SettingsModal from "./components/SettingsModal.vue";
</script>

<template>
  <div class="app">
    <TopBar />
    <div class="main-area">
      <CognitiveAxis />
      <PluginSidebar />
      <PluginHost />
      <ActionsBar />
    </div>
    <SettingsModal />
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
</style>
