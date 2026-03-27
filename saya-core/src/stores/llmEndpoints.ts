import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface LlmEndpoint {
  id: number;
  name: string;
  provider: string;
  endpoint_url: string;
  api_key: string | null;
  model: string;
  is_default: boolean;
}

export type LlmProvider = "openai" | "anthropic" | "local" | "bedrock";

export const useLlmEndpointsStore = defineStore("llmEndpoints", () => {
  const endpoints = ref<LlmEndpoint[]>([]);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  async function loadEndpoints() {
    isLoading.value = true;
    error.value = null;
    try {
      endpoints.value = await invoke<LlmEndpoint[]>("get_llm_endpoints");
    } catch (e) {
      error.value = String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function createEndpoint(data: Omit<LlmEndpoint, "id">) {
    try {
      const result = await invoke<LlmEndpoint>("create_llm_endpoint", {
        name: data.name,
        provider: data.provider,
        endpointUrl: data.endpoint_url,
        apiKey: data.api_key,
        model: data.model,
        isDefault: data.is_default,
      });
      endpoints.value.push(result);
      return result;
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  async function updateEndpoint(id: number, data: Partial<LlmEndpoint>) {
    try {
      await invoke("update_llm_endpoint", {
        id,
        name: data.name ?? null,
        provider: data.provider ?? null,
        endpointUrl: data.endpoint_url ?? null,
        apiKey: data.api_key ?? null,
        model: data.model ?? null,
        isDefault: data.is_default ?? null,
      });
      const idx = endpoints.value.findIndex((e) => e.id === id);
      if (idx !== -1) {
        endpoints.value[idx] = { ...endpoints.value[idx], ...data };
      }
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  async function deleteEndpoint(id: number) {
    try {
      await invoke("delete_llm_endpoint", { id });
      endpoints.value = endpoints.value.filter((e) => e.id !== id);
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  async function testConnection(id: number): Promise<{ success: boolean; message: string }> {
    try {
      return await invoke("test_llm_connection", { id });
    } catch (e) {
      return { success: false, message: String(e) };
    }
  }

  return {
    endpoints,
    isLoading,
    error,
    loadEndpoints,
    createEndpoint,
    updateEndpoint,
    deleteEndpoint,
    testConnection,
  };
});
