<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useLlmEndpointsStore, type LlmEndpoint, type LlmProvider } from "../../stores/llmEndpoints";

const store = useLlmEndpointsStore();

const isAdding = ref(false);
const editingId = ref<number | null>(null);
const testResult = ref<Record<number, { success: boolean; message: string }>>({});

const form = ref({
  name: "",
  provider: "openai" as LlmProvider,
  endpoint_url: "",
  api_key: "",
  model: "",
  is_default: false,
});

const providers: { value: LlmProvider; label: string }[] = [
  { value: "openai", label: "OpenAI" },
  { value: "anthropic", label: "Anthropic" },
  { value: "local", label: "Local" },
  { value: "bedrock", label: "Bedrock" },
];

function resetForm() {
  form.value = { name: "", provider: "openai", endpoint_url: "", api_key: "", model: "", is_default: false };
  isAdding.value = false;
  editingId.value = null;
}

function startEdit(ep: LlmEndpoint) {
  editingId.value = ep.id;
  isAdding.value = false;
  form.value = {
    name: ep.name,
    provider: ep.provider as LlmProvider,
    endpoint_url: ep.endpoint_url,
    api_key: ep.api_key || "",
    model: ep.model,
    is_default: ep.is_default,
  };
}

async function saveNew() {
  if (!form.value.name.trim() || !form.value.endpoint_url.trim() || !form.value.model.trim()) return;
  await store.createEndpoint({
    name: form.value.name,
    provider: form.value.provider,
    endpoint_url: form.value.endpoint_url,
    api_key: form.value.api_key || null,
    model: form.value.model,
    is_default: form.value.is_default,
  });
  resetForm();
}

async function saveEdit() {
  if (editingId.value === null) return;
  await store.updateEndpoint(editingId.value, {
    name: form.value.name,
    provider: form.value.provider,
    endpoint_url: form.value.endpoint_url,
    api_key: form.value.api_key || null,
    model: form.value.model,
    is_default: form.value.is_default,
  });
  resetForm();
}

async function remove(id: number) {
  await store.deleteEndpoint(id);
}

async function testConn(id: number) {
  testResult.value[id] = await store.testConnection(id);
  setTimeout(() => { delete testResult.value[id]; }, 3000);
}

function confirmDeleteDefault(id: number) {
  const ep = store.endpoints.find((e) => e.id === id);
  if (ep?.is_default) {
    if (!confirm("This is the default endpoint. Are you sure you want to delete it?")) return;
  }
  remove(id);
}

onMounted(() => store.loadEndpoints());
</script>

<template>
  <div class="settings-tab">
    <div class="tab-header">
      <h3>AI Configuration</h3>
      <button v-if="!isAdding && editingId === null" class="btn-add" @click="isAdding = true">
        + Add Endpoint
      </button>
    </div>

    <p v-if="store.isLoading" class="status-text">Loading...</p>
    <p v-else-if="store.endpoints.length === 0 && !isAdding" class="status-text">
      No LLM endpoints configured. Add one to enable AI features.
    </p>

    <div v-if="isAdding || editingId !== null" class="endpoint-form">
      <div class="form-row">
        <label class="field">
          <span class="field-label">Name</span>
          <input v-model="form.name" type="text" placeholder="e.g. Local Ollama" />
        </label>
        <label class="field">
          <span class="field-label">Provider</span>
          <select v-model="form.provider">
            <option v-for="p in providers" :key="p.value" :value="p.value">{{ p.label }}</option>
          </select>
        </label>
      </div>
      <label class="field">
        <span class="field-label">API URL</span>
        <input v-model="form.endpoint_url" type="text" placeholder="https://api.openai.com/v1" />
      </label>
      <div class="form-row">
        <label class="field">
          <span class="field-label">API Key</span>
          <input v-model="form.api_key" type="password" placeholder="sk-..." />
        </label>
        <label class="field">
          <span class="field-label">Model</span>
          <input v-model="form.model" type="text" placeholder="gpt-4o" />
        </label>
      </div>
      <label class="toggle-field">
        <input v-model="form.is_default" type="checkbox" />
        <span>Set as default</span>
      </label>
      <div class="form-actions">
        <button class="btn-cancel" @click="resetForm">Cancel</button>
        <button
          class="btn-save"
          :disabled="!form.name.trim() || !form.endpoint_url.trim() || !form.model.trim()"
          @click="editingId !== null ? saveEdit() : saveNew()"
        >
          {{ editingId !== null ? "Save" : "Add" }}
        </button>
      </div>
    </div>

    <div class="endpoint-list">
      <div v-for="ep in store.endpoints" :key="ep.id" class="endpoint-card">
        <div class="endpoint-info">
          <div class="endpoint-name">
            {{ ep.name }}
            <span v-if="ep.is_default" class="badge">Default</span>
          </div>
          <div class="endpoint-meta">
            {{ ep.provider }} &middot; {{ ep.model }}
          </div>
        </div>
        <div class="endpoint-actions">
          <button class="btn-icon-sm" title="Test connection" @click="testConn(ep.id)">&#9889;</button>
          <button class="btn-icon-sm" title="Edit" @click="startEdit(ep)">&#9998;</button>
          <button class="btn-icon-sm btn-danger" title="Remove" @click="confirmDeleteDefault(ep.id)">&times;</button>
        </div>
        <div v-if="testResult[ep.id]" class="test-result" :class="{ success: testResult[ep.id].success, error: !testResult[ep.id].success }">
          {{ testResult[ep.id].message }}
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

.status-text {
  color: var(--text-muted);
  font-size: 13px;
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

.endpoint-form {
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

.field input,
.field select {
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

.field input:focus,
.field select:focus {
  border-color: var(--accent);
}

.field input::placeholder {
  color: var(--text-muted);
}

.toggle-field {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
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

.endpoint-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.endpoint-card {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background-color: var(--bg-card);
  gap: 8px;
}

.endpoint-info {
  flex: 1;
  min-width: 0;
}

.endpoint-name {
  font-size: 13px;
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 6px;
}

.badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background-color: var(--accent);
  color: white;
  font-weight: 600;
}

.endpoint-meta {
  font-size: 12px;
  color: var(--text-muted);
}

.endpoint-actions {
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

.test-result {
  width: 100%;
  font-size: 12px;
  padding: 4px 0;
}

.test-result.success {
  color: #16a34a;
}

.test-result.error {
  color: #dc2626;
}
</style>
