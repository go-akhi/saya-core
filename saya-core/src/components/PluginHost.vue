<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref } from "vue";
import { usePluginStore } from "../stores/plugins";
import { createCoreMessageHandler } from "../lib/core-message-handler";
import type { SayaMessage } from "../lib/saya-api";
import { listen } from "@tauri-apps/api/event";

const pluginStore = usePluginStore();
const iframeRef = ref<HTMLIFrameElement | null>(null);
const messageHandler = createCoreMessageHandler();
let unlistenHotReload: (() => void) | null = null;

const handleMessage = (event: MessageEvent) => {
  const message = event.data as SayaMessage;
  if (!message || typeof message !== "object") return;
  if (message.source === "plugin" && message.plugin === pluginStore.activePlugin) {
    messageHandler.handleMessage(message, iframeRef.value?.contentWindow || window);
  }
};

function reloadIframe() {
  if (iframeRef.value) {
    const src = iframeRef.value.src;
    iframeRef.value.src = src;
  }
}

onMounted(async () => {
  window.addEventListener("message", handleMessage);

  unlistenHotReload = await listen<{ plugin_name: string }>("plugin-file-changed", (event) => {
    if (event.payload.plugin_name === pluginStore.activePlugin) {
      reloadIframe();
    }
  });
});

onUnmounted(() => {
  window.removeEventListener("message", handleMessage);
  if (unlistenHotReload) unlistenHotReload();
});

watch(() => pluginStore.activePlugin, () => {
  iframeRef.value = document.querySelector(".plugin-iframe");
});
</script>

<template>
  <div class="plugin-host">
    <iframe
      v-if="pluginStore.activePlugin"
      :src="`/plugins/${pluginStore.activePlugin}/ui/index.html`"
      class="plugin-iframe"
      sandbox="allow-scripts allow-same-origin"
    />
    <div v-else class="plugin-empty">
      <div class="empty-state">
        <span class="empty-icon">&#9634;</span>
        <p class="empty-text">Select a plugin to get started</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.plugin-host {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.plugin-iframe {
  width: 100%;
  height: 100%;
  border: none;
  background-color: var(--bg-primary);
}

.plugin-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  color: var(--text-muted);
}

.empty-icon {
  font-size: 48px;
  opacity: 0.3;
}

.empty-text {
  font-size: 14px;
}
</style>
