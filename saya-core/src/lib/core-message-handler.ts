import { invoke } from "@tauri-apps/api/core";
import type {
  SayaMessage,
  QueryOptions,
  MutationOptions,
  AiActionRequest,
  ResponsePayload,
  Item,
  ErrorPayload,
  CompletionRequest,
  CompletionResponse,
} from "./saya-api/types";

export interface PluginMessageHandler {
  handleMessage(message: SayaMessage, source: Window): void;
  subscribe(pluginName: string, event: string, callback: (payload: unknown) => void): string;
  unsubscribe(subscriptionId: string): void;
  onShowError: (payload: ErrorPayload, pluginName: string) => void;
}

const subscriptions = new Map<string, {
  plugin: string;
  event: string;
  callback: (payload: unknown) => void;
}>();

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

// Validation helpers
function isValidPluginName(name: string): boolean {
  return /^[a-z][a-z0-9-]{0,63}$/.test(name);
}

function isValidColumnType(value: unknown): value is "query" | "mutate" | "subscribe" | "unsubscribe" | "ai_action" | "show_error" | "complete" {
  return ["query", "mutate", "subscribe", "unsubscribe", "ai_action", "show_error", "complete"].includes(value as string);
}

function validateQueryPayload(payload: unknown): { valid: boolean; error?: string } {
  if (typeof payload !== "object" || payload === null) {
    return { valid: false, error: "Query payload must be an object" };
  }
  const p = payload as Record<string, unknown>;
  if (p.filters && typeof p.filters !== "object") {
    return { valid: false, error: "Filters must be an object" };
  }
  if (p.columns && !Array.isArray(p.columns)) {
    return { valid: false, error: "Columns must be an array" };
  }
  if (p.columns && (p.columns as unknown[]).some(c => typeof c !== "string" || c.length > 128)) {
    return { valid: false, error: "Each column must be a string (max 128 chars)" };
  }
  if (p.limit !== undefined && (typeof p.limit !== "number" || p.limit < 1 || p.limit > 10000)) {
    return { valid: false, error: "Limit must be a number between 1 and 10000" };
  }
  if (p.offset !== undefined && (typeof p.offset !== "number" || p.offset < 0)) {
    return { valid: false, error: "Offset must be a non-negative number" };
  }
  return { valid: true };
}

function validateMutationPayload(payload: unknown): { valid: boolean; error?: string } {
  if (typeof payload !== "object" || payload === null) {
    return { valid: false, error: "Mutation payload must be an object" };
  }
  const p = payload as Record<string, unknown>;
  if (!["create", "update", "delete", "save_settings", "load_settings"].includes(p.operation as string)) {
    return { valid: false, error: `Invalid mutation operation: ${p.operation}` };
  }
  if (p.operation === "update" || p.operation === "delete") {
    if (!p.id || typeof p.id !== "string" || p.id.length > 128) {
      return { valid: false, error: "Item ID must be a string (max 128 chars)" };
    }
  }
  if (p.data && typeof p.data !== "object") {
    return { valid: false, error: "Mutation data must be an object" };
  }
  return { valid: true };
}

function validateCompletionPayload(payload: unknown): { valid: boolean; error?: string } {
  if (typeof payload !== "object" || payload === null) {
    return { valid: false, error: "Completion payload must be an object" };
  }
  const p = payload as Record<string, unknown>;
  if (typeof p.user !== "string" || p.user.length === 0) {
    return { valid: false, error: "User message must be a non-empty string" };
  }
  if (p.user.length > 100000) {
    return { valid: false, error: "User message exceeds maximum length (100000 chars)" };
  }
  if (p.system !== undefined) {
    if (typeof p.system !== "string") {
      return { valid: false, error: "System message must be a string" };
    }
    if (p.system.length > 100000) {
      return { valid: false, error: "System message exceeds maximum length (100000 chars)" };
    }
  }
  if (p.temperature !== undefined && (typeof p.temperature !== "number" || p.temperature < 0 || p.temperature > 2)) {
    return { valid: false, error: "Temperature must be a number between 0 and 2" };
  }
  if (p.max_tokens !== undefined && (typeof p.max_tokens !== "number" || p.max_tokens < 1 || p.max_tokens > 100000)) {
    return { valid: false, error: "max_tokens must be a number between 1 and 100000" };
  }
  return { valid: true };
}

function validateErrorPayload(payload: unknown): { valid: boolean; error?: string } {
  if (typeof payload !== "object" || payload === null) {
    return { valid: false, error: "Error payload must be an object" };
  }
  const p = payload as Record<string, unknown>;
  if (typeof p.message !== "string" || p.message.length === 0) {
    return { valid: false, error: "Error message must be a non-empty string" };
  }
  if (p.message.length > 10000) {
    return { valid: false, error: "Error message exceeds maximum length" };
  }
  if (p.type !== undefined && !["error", "warning", "info"].includes(p.type as string)) {
    return { valid: false, error: "Error type must be error, warning, or info" };
  }
  return { valid: true };
}

let showErrorCallback: ((payload: ErrorPayload, pluginName: string) => void) | null = null;

export function createCoreMessageHandler(): PluginMessageHandler {
  const handler: PluginMessageHandler = {
    onShowError(payload: ErrorPayload, pluginName: string) {
      if (showErrorCallback) {
        showErrorCallback(payload, pluginName);
      }
    },
    handleMessage(message: SayaMessage, source: Window): void {
      // Validate message structure
      if (!message || typeof message !== "object") return;
      if (message.source !== "plugin") return;
      if (typeof message.id !== "string" || message.id.length > 256) return;
      if (!message.plugin || !isValidPluginName(message.plugin)) return;
      if (!isValidColumnType(message.type)) return;

      const responseId = message.id;
      const pluginName = message.plugin;

      const sendResponse = (payload: ResponsePayload) => {
        source.postMessage({
          id: responseId,
          type: "response",
          payload,
          source: "core",
          plugin: pluginName,
        }, "*");
      };

      try {
        switch (message.type) {
          case "query": {
            const validation = validateQueryPayload(message.payload);
            if (!validation.valid) {
              sendResponse({ success: false, error: validation.error });
              break;
            }
            handleQuery(pluginName, message.payload as QueryOptions)
              .then(data => sendResponse({ success: true, data }))
              .catch(error => sendResponse({ success: false, error: String(error) }));
            break;
          }

          case "mutate": {
            const validation = validateMutationPayload(message.payload);
            if (!validation.valid) {
              sendResponse({ success: false, error: validation.error });
              break;
            }
            handleMutation(pluginName, message.payload as MutationOptions)
              .then(data => sendResponse({ success: true, data }))
              .catch(error => sendResponse({ success: false, error: String(error) }));
            break;
          }

          case "ai_action": {
            if (typeof message.payload !== "object" || message.payload === null) {
              sendResponse({ success: false, error: "AI action payload must be an object" });
              break;
            }
            handleAiAction(pluginName, message.payload as AiActionRequest)
              .then(data => sendResponse({ success: true, data }))
              .catch(error => sendResponse({ success: false, error: String(error) }));
            break;
          }

          case "subscribe": {
            if (typeof message.payload !== "object" || message.payload === null) {
              sendResponse({ success: false, error: "Subscribe payload must be an object" });
              break;
            }
            const subPayload = message.payload as { event: string };
            if (typeof subPayload.event !== "string" || subPayload.event.length > 64) {
              sendResponse({ success: false, error: "Event name must be a string (max 64 chars)" });
              break;
            }
            const subId = generateId();
            subscriptions.set(subId, {
              plugin: pluginName,
              event: subPayload.event,
              callback: (payload: unknown) => {
                source.postMessage({
                  id: subId,
                  type: "event",
                  payload,
                  source: "core",
                  plugin: pluginName,
                }, "*");
              },
            });
            sendResponse({ success: true, data: { subscriptionId: subId } });
            break;
          }

          case "unsubscribe": {
            if (typeof message.payload !== "object" || message.payload === null) {
              sendResponse({ success: false, error: "Unsubscribe payload must be an object" });
              break;
            }
            const unsubPayload = message.payload as { subscriptionId: string };
            if (typeof unsubPayload.subscriptionId !== "string") {
              sendResponse({ success: false, error: "subscriptionId must be a string" });
              break;
            }
            subscriptions.delete(unsubPayload.subscriptionId);
            sendResponse({ success: true });
            break;
          }

          case "show_error": {
            const validation = validateErrorPayload(message.payload);
            if (!validation.valid) {
              sendResponse({ success: false, error: validation.error });
              break;
            }
            const errorPayload = message.payload as ErrorPayload;
            if (showErrorCallback) {
              showErrorCallback(errorPayload, pluginName);
            } else {
              console.warn("[Saya] showError callback not set, plugin:", pluginName);
            }
            sendResponse({ success: true });
            break;
          }

          case "complete": {
            const validation = validateCompletionPayload(message.payload);
            if (!validation.valid) {
              sendResponse({ success: false, error: validation.error });
              break;
            }
            const completionRequest = message.payload as CompletionRequest;
            invoke<CompletionResponse>("llm_complete", {
              system: completionRequest.system,
              user: completionRequest.user,
              temperature: completionRequest.temperature ?? 0.7,
              max_tokens: completionRequest.max_tokens ?? 1024,
            })
              .then(result => sendResponse({ success: true, data: result }))
              .catch(error => sendResponse({ success: false, error: String(error) }));
            break;
          }

          default:
            sendResponse({ success: false, error: `Unknown message type: ${message.type}` });
        }
      } catch (error) {
        sendResponse({ success: false, error: String(error) });
      }
    },

    subscribe(pluginName: string, event: string, callback: (payload: unknown) => void): string {
      const id = generateId();
      subscriptions.set(id, { plugin: pluginName, event, callback });
      return id;
    },

    unsubscribe(subscriptionId: string): void {
      subscriptions.delete(subscriptionId);
    },
  };

  return handler;
}

export function setShowErrorCallback(callback: (payload: ErrorPayload, pluginName: string) => void): void {
  showErrorCallback = callback;
}

async function handleQuery(pluginName: string, options: QueryOptions): Promise<Item[]> {
  if (options.operation === "get_manifest") {
    const result = await invoke<unknown>("get_plugin_manifest", { pluginName });
    return result as Item[];
  }

  if (options.operation === "get_info") {
    const result = await invoke<unknown>("get_plugin_info", { pluginName });
    return result as Item[];
  }

  const result = await invoke<unknown>("query_plugin_items", {
    pluginName,
    columns: options.columns,
    filters: options.filters,
    sortColumn: options.sort?.column,
    sortDirection: options.sort?.direction,
    limit: options.limit,
    offset: options.offset,
  });
  return result as Item[];
}

async function handleMutation(pluginName: string, options: MutationOptions): Promise<Item> {
  const result = await invoke<unknown>("mutate_plugin_item", {
    pluginName,
    operation: options.operation,
    id: options.id,
    data: options.data,
  });
  return result as Item;
}

async function handleAiAction(pluginName: string, options: AiActionRequest): Promise<Record<string, unknown>> {
  const result = await invoke<unknown>("execute_ai_action", {
    pluginName,
    actionId: options.action_id,
    itemIds: options.item_ids,
    context: options.context,
  });
  return result as Record<string, unknown>;
}

export function emitToSubscriptions(pluginName: string, event: string, payload: unknown): void {
  subscriptions.forEach(({ plugin, event: subEvent, callback }) => {
    if (plugin === pluginName && subEvent === event) {
      callback(payload);
    }
  });
}
