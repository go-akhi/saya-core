<script setup lang="ts">
import { useAxesStore, COGNITIVE_AXES, type CognitiveAxis } from "../stores/axes";
import { usePluginStore } from "../stores/plugins";

const axesStore = useAxesStore();
const pluginStore = usePluginStore();

function isActive(label: CognitiveAxis) {
  return axesStore.activeCognitiveAxis === label;
}

function toggle(label: CognitiveAxis) {
  axesStore.setActiveCognitiveAxis(isActive(label) ? null : label);
}
</script>

<template>
  <aside class="cognitive-axis">
    <div class="expand-zone">
      <button
        class="expand-btn"
        :class="{ visible: pluginStore.isCollapsed }"
        title="Expand sidebar"
        @click="pluginStore.toggleCollapse()"
      >
        <span class="expand-icon">&gt;&gt;</span>
      </button>
    </div>
    <button
      v-for="axis in COGNITIVE_AXES"
      :key="axis.label!"
      class="axis-btn"
      :class="{ active: isActive(axis.label) }"
      :style="isActive(axis.label) ? { backgroundColor: axis.color } : {}"
      @click="toggle(axis.label)"
    >
      <span
        class="axis-label"
        :style="isActive(axis.label) ? { color: 'white' } : {}"
      >{{ axis.label }}</span>
      <span
        v-if="(axesStore.badgeCounts[axis.label!] ?? 0) > 0"
        class="axis-badge"
        :style="{
          backgroundColor: isActive(axis.label) ? 'rgba(255,255,255,0.25)' : `${axis.color}15`,
          color: isActive(axis.label) ? 'white' : axis.color,
        }"
      >{{ axesStore.badgeCounts[axis.label!] }}</span>
    </button>
  </aside>
</template>

<style scoped>
.cognitive-axis {
  display: flex;
  flex-direction: column;
  background-color: var(--bg-sidebar);
  flex-shrink: 0;
  width: 44px;
  height: 100%;
}

.expand-zone {
  display: flex;
  justify-content: center;
  padding: 4px;
  min-height: 40px;
  border-bottom: 1px solid var(--border);
}

.expand-btn {
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
  opacity: 0;
  transition: opacity 100ms ease;
  pointer-events: none;
}

.expand-btn.visible {
  opacity: 1;
  pointer-events: auto;
}

.expand-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.expand-icon {
  font-size: 10px;
}

.axis-btn {
  position: relative;
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  border: none;
  border-right: 1px solid var(--border);
  background: transparent;
  cursor: pointer;
  transition: background-color 150ms;
}

.axis-btn:hover:not(.active) {
  background-color: var(--bg-hover);
}

.axis-label {
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  color: var(--text-secondary);
  user-select: none;
}

.axis-badge {
  position: absolute;
  top: 8px;
  right: 4px;
  min-width: 18px;
  padding: 0 4px;
  height: 18px;
  border-radius: 9px;
  font-size: 10px;
  font-weight: 600;
  line-height: 18px;
  text-align: center;
}
</style>
