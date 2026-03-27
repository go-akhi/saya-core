<script setup lang="ts">
import { useSettingsStore, type Theme } from "../../stores/settings";
import { COGNITIVE_AXES, type CognitiveAxis } from "../../stores/axes";

const settingsStore = useSettingsStore();

const themes: { value: Theme; label: string }[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "system", label: "System" },
];

const version = "0.1.0";

async function exportData() {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const status = await invoke<string>("get_db_status");
    alert(status + "\n\nFull database export will be available in a future release.");
  } catch {
    alert("Unable to access database.");
  }
}
</script>

<template>
  <div class="settings-tab">
    <div class="tab-header">
      <h3>General</h3>
    </div>

    <div class="setting-group">
      <label class="setting-row">
        <span class="setting-label">Theme</span>
        <select :value="settingsStore.theme" @change="settingsStore.setTheme(($event.target as HTMLSelectElement).value as Theme)">
          <option v-for="t in themes" :key="t.value" :value="t.value">{{ t.label }}</option>
        </select>
      </label>

      <label class="setting-row">
        <span class="setting-label">Default cognitive axis</span>
        <select
          :value="settingsStore.defaultCognitiveAxis ?? undefined"
          @change="settingsStore.setDefaultCognitiveAxis(($event.target as HTMLSelectElement).value as CognitiveAxis)"
        >
          <option v-for="axis in COGNITIVE_AXES" :key="String(axis.label)" :value="axis.label ?? ''">
            {{ axis.label }}
          </option>
        </select>
      </label>

      <div class="setting-row">
        <span class="setting-label">Export data</span>
        <button class="btn-secondary" @click="exportData">Export</button>
      </div>
    </div>

    <div class="about-section">
      <div class="about-label">Version</div>
      <div class="about-value">{{ version }}</div>
    </div>
  </div>
</template>

<style scoped>
.settings-tab {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.tab-header h3 {
  font-size: 14px;
  font-weight: 600;
}

.setting-group {
  display: flex;
  flex-direction: column;
  gap: 1px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background-color: var(--bg-card);
  cursor: default;
}

.setting-row + .setting-row {
  border-top: 1px solid var(--border);
}

.setting-label {
  font-size: 13px;
  font-weight: 500;
}

.setting-row select {
  padding: 4px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-primary);
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  outline: none;
  cursor: pointer;
}

.setting-row select:focus {
  border-color: var(--accent);
}

.btn-secondary {
  padding: 4px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: background-color 150ms, color 150ms;
}

.btn-secondary:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.about-section {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
}

.about-label {
  font-size: 13px;
  font-weight: 500;
}

.about-value {
  font-size: 12px;
  color: var(--text-muted);
  font-family: monospace;
}
</style>
