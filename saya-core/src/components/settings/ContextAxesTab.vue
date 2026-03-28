<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useAxesStore } from "../../stores/axes";

const store = useAxesStore();

const isAdding = ref(false);
const editingId = ref<number | null>(null);
const showEmojiPicker = ref(false);

const form = ref({
  name: "",
  icon: "",
  color: "#6B7280",
});

const colorPresets = ["#3B82F6", "#10B981", "#D97706", "#DC5F3F", "#8B5CF6", "#EC4899", "#6B7280", "#14B8A6"];

const emojiList = [
  "📋", "📌", "🎯", "💡", "🔧", "🛠", "⚙", "📊", "📈", "📉",
  "🗂", "📁", "📂", "📝", "✏", "🖊", "🖋", "🔍", "🔎", "🧪",
  "🔬", "💻", "🖥", "📱", "🚀", "⭐", "🌟", "✨", "🔥", "❤",
  "🟣", "🔵", "🟢", "🟡", "🟠", "🔴", "⚪", "⚫", "🟤", "🩵",
  "🏠", "📚", "📖", "🎵", "🎮", "🎨", "🏋", "🧘", "💼", "🏠",
  "🌳", "🌍", "⏰", "📞", "🤝", "👥", "🧠", "🩺", "🍕", "☕",
];

function selectEmoji(emoji: string) {
  form.value.icon = emoji;
  showEmojiPicker.value = false;
}

function resetForm() {
  form.value = { name: "", icon: "", color: "#6B7280" };
  isAdding.value = false;
  editingId.value = null;
  showEmojiPicker.value = false;
}

function startEdit(axis: { id: number; name: string; icon: string | null; color: string | null }) {
  editingId.value = axis.id;
  isAdding.value = false;
  form.value = {
    name: axis.name,
    icon: axis.icon || "",
    color: axis.color || "#6B7280",
  };
}

async function saveNew() {
  if (!form.value.name.trim()) return;
  await store.createAxis({
    name: form.value.name,
    icon: form.value.icon || undefined,
    color: form.value.color,
  });
  resetForm();
}

async function saveEdit() {
  if (editingId.value === null) return;
  await store.updateAxis(editingId.value, {
    name: form.value.name,
    icon: form.value.icon || undefined,
    color: form.value.color,
  });
  resetForm();
}

async function remove(id: number) {
  try {
    await store.deleteAxis(id);
  } catch {
    alert("Cannot delete this axis. It may be a default axis.");
  }
}

onMounted(() => store.loadAxes());
</script>

<template>
  <div class="settings-tab">
    <div class="tab-header">
      <h3>Context Axes</h3>
      <button v-if="!isAdding && editingId === null" class="btn-add" @click="isAdding = true">
        + Add Axis
      </button>
    </div>

    <div v-if="isAdding || editingId !== null" class="axis-form">
      <label class="field">
        <span class="field-label">Name</span>
        <input v-model="form.name" type="text" placeholder="e.g. Side Project" autofocus />
      </label>
      <div class="form-row">
        <label class="field emoji-field">
          <span class="field-label">Emoji</span>
          <div class="emoji-input-row">
            <button class="emoji-trigger" @click="showEmojiPicker = !showEmojiPicker">
              <span v-if="form.icon" class="emoji-preview">{{ form.icon }}</span>
              <span v-else class="emoji-placeholder">Pick…</span>
            </button>
            <button v-if="form.icon" class="emoji-clear" @click="form.icon = ''">&times;</button>
          </div>
          <div v-if="showEmojiPicker" class="emoji-picker">
            <button
              v-for="e in emojiList"
              :key="e"
              class="emoji-option"
              @click="selectEmoji(e)"
            >{{ e }}</button>
          </div>
        </label>
        <label class="field">
          <span class="field-label">Color</span>
          <div class="color-picker">
            <input v-model="form.color" type="color" class="color-input" />
            <div class="color-presets">
              <button
                v-for="c in colorPresets"
                :key="c"
                class="color-swatch"
                :class="{ active: form.color === c }"
                :style="{ backgroundColor: c }"
                @click="form.color = c"
              />
            </div>
          </div>
        </label>
      </div>
      <div class="form-actions">
        <button class="btn-cancel" @click="resetForm">Cancel</button>
        <button
          class="btn-save"
          :disabled="!form.name.trim()"
          @click="editingId !== null ? saveEdit() : saveNew()"
        >
          {{ editingId !== null ? "Save" : "Add" }}
        </button>
      </div>
    </div>

    <div class="axis-list">
      <div v-for="axis in store.contextAxes" :key="axis.id" class="axis-card">
        <div class="axis-left">
          <span v-if="axis.color" class="axis-dot" :style="{ backgroundColor: axis.color }" />
          <span v-if="axis.icon" class="axis-icon">{{ axis.icon }}</span>
          <span class="axis-name">{{ axis.name }}</span>
          <span v-if="axis.is_default" class="badge">Default</span>
        </div>
        <div class="axis-actions">
          <button class="btn-icon-sm" title="Edit" @click="startEdit(axis)">&#9998;</button>
          <button
            v-if="!axis.is_default"
            class="btn-icon-sm btn-danger"
            title="Delete"
            @click="remove(axis.id)"
          >&times;</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-tab {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.tab-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.tab-header h3 {
  font-size: 14px;
  font-weight: 600;
}

.btn-add {
  padding: 4px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: transparent;
  color: var(--accent);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 150ms;
}

.btn-add:hover {
  background-color: var(--bg-hover);
}

.axis-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-sidebar);
}

.form-row {
  display: flex;
  gap: 12px;
}

.form-row .field {
  flex: 1;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.field input[type="text"] {
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 150ms;
}

.field input[type="text"]:focus {
  border-color: var(--accent);
}

.field input::placeholder {
  color: var(--text-muted);
}

.emoji-field {
  position: relative;
}

.emoji-input-row {
  display: flex;
  align-items: center;
  gap: 4px;
}

.emoji-trigger {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 32px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
  cursor: pointer;
  transition: border-color 150ms;
  font-size: 18px;
  font-family: inherit;
}

.emoji-trigger:hover {
  border-color: var(--accent);
}

.emoji-preview {
  font-size: 20px;
  line-height: 1;
}

.emoji-placeholder {
  font-size: 12px;
  color: var(--text-muted);
}

.emoji-clear {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
}

.emoji-clear:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.emoji-picker {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 10;
  display: grid;
  grid-template-columns: repeat(10, 1fr);
  gap: 2px;
  padding: 6px;
  margin-top: 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  max-height: 200px;
  overflow-y: auto;
}

.emoji-option {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 4px;
  background: transparent;
  font-size: 16px;
  cursor: pointer;
  transition: background-color 100ms;
}

.emoji-option:hover {
  background-color: var(--bg-hover);
}

.color-picker {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.color-input {
  width: 100%;
  height: 32px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: 2px;
  cursor: pointer;
}

.color-presets {
  display: flex;
  gap: 4px;
}

.color-swatch {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: border-color 150ms, transform 100ms;
}

.color-swatch:hover {
  transform: scale(1.15);
}

.color-swatch.active {
  border-color: var(--text-primary);
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.btn-cancel,
.btn-save {
  padding: 4px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 12px;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: background-color 150ms;
}

.btn-cancel {
  background: transparent;
  color: var(--text-secondary);
}

.btn-cancel:hover {
  background-color: var(--bg-hover);
}

.btn-save {
  background-color: var(--accent);
  border-color: var(--accent);
  color: white;
}

.btn-save:hover:not(:disabled) {
  background-color: var(--accent-hover);
}

.btn-save:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.axis-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.axis-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
}

.axis-left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.axis-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.axis-icon {
  font-size: 16px;
  line-height: 1;
}

.axis-name {
  font-size: 13px;
  font-weight: 500;
}

.badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background-color: var(--bg-badge);
  color: var(--text-muted);
  font-weight: 600;
}

.axis-actions {
  display: flex;
  gap: 4px;
}

.btn-icon-sm {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  transition: background-color 150ms, color 150ms;
}

.btn-icon-sm:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.btn-icon-sm.btn-danger:hover {
  background-color: #fee2e2;
  color: #dc2626;
}
</style>
