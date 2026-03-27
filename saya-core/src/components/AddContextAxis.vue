<script setup lang="ts">
import { ref } from "vue";
import { useAxesStore } from "../stores/axes";

const axesStore = useAxesStore();

const isOpen = ref(false);
const name = ref("");
const description = ref("");

function open() {
  isOpen.value = true;
}

function close() {
  isOpen.value = false;
  name.value = "";
  description.value = "";
}

function save() {
  if (!name.value.trim()) return;
  axesStore.addContextAxis({
    id: Date.now(),
    name: name.value.trim(),
    description: description.value.trim() || null,
    icon: null,
    color: null,
    is_default: false,
  });
  close();
}
</script>

<template>
  <button class="add-axis-btn" title="Add context axis" @click="open">
    <span class="plus">+</span>
  </button>

  <Teleport to="body">
    <div v-if="isOpen" class="modal-overlay" @click.self="close">
      <div class="modal">
        <header class="modal-header">
          <h2>New Context Axis</h2>
        </header>
        <div class="modal-body">
          <label class="field">
            <span class="field-label">Name</span>
            <input
              v-model="name"
              name="name"
              type="text"
              placeholder="e.g. Side Project, Family..."
              autofocus
            />
          </label>
          <label class="field">
            <span class="field-label">Description (optional)</span>
            <input
              v-model="description"
              name="description"
              type="text"
              placeholder="What this category is for"
            />
          </label>
        </div>
        <div class="modal-actions">
          <button class="btn-cancel" @click="close">Cancel</button>
          <button class="btn-save" :disabled="!name.trim()" @click="save">Add</button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.add-axis-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 32px;
  padding: 0 8px;
  height: 100%;
  border: none;
  border-radius: 8px 8px 0 0;
  background: transparent;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  transition: background-color 150ms, color 150ms;
  flex-shrink: 0;
}

.add-axis-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.plus {
  line-height: 1;
}

.modal-overlay {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 1000;
}

.modal {
  width: 400px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
}

.modal-header {
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.modal-header h2 {
  font-size: 15px;
  font-weight: 600;
}

.modal-body {
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field-label {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
}

.field input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 150ms;
}

.field input:focus {
  border-color: var(--accent);
}

.field input::placeholder {
  color: var(--text-muted);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 20px;
  border-top: 1px solid var(--border);
}

.btn-cancel,
.btn-save {
  padding: 6px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  font-size: 13px;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: background-color 150ms, border-color 150ms;
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
</style>
