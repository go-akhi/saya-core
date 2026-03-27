<script setup lang="ts">
import { ref, computed } from "vue";
import { usePluginStore } from "../stores/plugins";

const pluginStore = usePluginStore();
const isHovering = ref(false);

const aiTooltip = computed(() => {
  if (!pluginStore.activePlugin) return null;
  
  const manifest = pluginStore.pluginManifests[pluginStore.activePlugin];
  if (!manifest?.ai_actions?.length) return null;
  
  return manifest.ai_actions[0]?.label || null;
});

interface Action {
  id: string;
  label: string;
  icon: string;
  pluginName: string;
}

const pluginActions = computed<Action[]>(() => {
  const actions: Action[] = [];
  
  pluginStore.plugins.forEach((plugin) => {
    const manifest = pluginStore.pluginManifests[plugin.name];
    if (manifest?.provides_actions) {
      manifest.provides_actions.forEach((action, index) => {
        actions.push({
          id: `${plugin.name}-${index}`,
          label: action.label,
          icon: action.handler.startsWith("pipeline:") ? "→" : "⚡",
          pluginName: plugin.name,
        });
      });
    }
  });
  
  return actions;
});
</script>

<template>
  <aside
    class="dock"
    :class="{ visible: isHovering }"
    @mouseenter="isHovering = true"
    @mouseleave="isHovering = false"
  >
    <button
      class="dock-icon ai-button"
      :title="aiTooltip || undefined"
    >
      <span class="icon">☆</span>
    </button>

    <button
      v-for="action in pluginActions"
      :key="action.id"
      class="dock-icon"
      :title="action.label"
    >
      <span class="icon">{{ action.icon }}</span>
    </button>
  </aside>
</template>

<style scoped>
.dock {
  position: absolute;
  right: 0;
  top: 50%;
  transform: translateY(-50%) translateX(calc(100% - 8px));
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  width: 56px;
  background-color: rgba(245, 244, 241, 0.9);
  backdrop-filter: blur(12px);
  border: 1px solid var(--border);
  border-right: none;
  border-radius: 12px 0 0 12px;
  padding: 8px 0;
  gap: 4px;
  z-index: 100;
  transition: transform 250ms cubic-bezier(0.4, 0, 0.2, 1);
  box-shadow: -4px 0 20px rgba(0, 0, 0, 0.08);
}

.dock.visible {
  transform: translateY(-50%) translateX(0);
}

.dock-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  border: none;
  border-radius: 10px;
  background: transparent;
  cursor: pointer;
  transition: background-color 200ms ease;
}

.dock-icon:hover {
  background-color: rgba(0, 0, 0, 0.06);
}

.dock-icon .icon {
  font-size: 20px;
  color: var(--text-secondary);
  transition: color 150ms;
}

.dock-icon:hover .icon {
  color: var(--text-primary);
}
</style>
