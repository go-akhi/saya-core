import { invoke } from "@tauri-apps/api/core";
import type {
  SayaMessage,
  QueryOptions,
  MutationOptions,
  AiActionRequest,
  ResponsePayload,
  Item,
} from "./saya-api/types";

export interface PluginMessageHandler {
  handleMessage(message: SayaMessage, source: Window): void;
  subscribe(pluginName: string, event: string, callback: (payload: unknown) => void): string;
  unsubscribe(subscriptionId: string): void;
}

const subscriptions = new Map<string, {
  plugin: string;
  event: string;
  callback: (payload: unknown) => void;
}>();

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

export function createCoreMessageHandler(): PluginMessageHandler {
  return {
    handleMessage(message: SayaMessage, source: Window): void {
      if (message.source !== "plugin") return;

      const responseId = message.id;

      const sendResponse = (payload: ResponsePayload) => {
        source.postMessage({
          id: responseId,
          type: "response",
          payload,
          source: "core",
          plugin: message.plugin,
        }, "*");
      };

      try {
        switch (message.type) {
          case "query":
            handleQuery(message.payload as QueryOptions)
              .then(data => sendResponse({ success: true, data }))
              .catch(error => sendResponse({ success: false, error: String(error) }));
            break;

          case "mutate":
            handleMutation(message.payload as MutationOptions)
              .then(data => sendResponse({ success: true, data }))
              .catch(error => sendResponse({ success: false, error: String(error) }));
            break;

          case "ai_action":
            handleAiAction(message.payload as AiActionRequest)
              .then(data => sendResponse({ success: true, data }))
              .catch(error => sendResponse({ success: false, error: String(error) }));
            break;

          case "subscribe":
            const subPayload = message.payload as { plugin: string; event: string };
            const subId = generateId();
            subscriptions.set(subId, {
              plugin: subPayload.plugin,
              event: subPayload.event,
              callback: (payload: unknown) => {
                source.postMessage({
                  id: subId,
                  type: "event",
                  payload,
                  source: "core",
                  plugin: subPayload.plugin,
                }, "*");
              },
            });
            sendResponse({ success: true, data: { subscriptionId: subId } });
            break;

          case "unsubscribe":
            const unsubPayload = message.payload as { subscriptionId: string };
            subscriptions.delete(unsubPayload.subscriptionId);
            sendResponse({ success: true });
            break;

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
}

async function handleQuery(options: QueryOptions): Promise<Item[]> {
  if (options.operation === "get_manifest") {
    const result = await invoke<unknown>("get_plugin_manifest", { pluginName: options.plugin });
    return result as Item[];
  }

  if (options.operation === "get_info") {
    const result = await invoke<unknown>("get_plugin_info", { pluginName: options.plugin });
    return result as Item[];
  }

  const result = await invoke<unknown>("query_plugin_items", {
    pluginName: options.plugin,
    columns: options.columns,
    filters: options.filters,
    sortColumn: options.sort?.column,
    sortDirection: options.sort?.direction,
    limit: options.limit,
    offset: options.offset,
  });
  return result as Item[];
}

async function handleMutation(options: MutationOptions): Promise<Item> {
  const result = await invoke<unknown>("mutate_plugin_item", {
    pluginName: options.plugin,
    operation: options.operation,
    id: options.id,
    data: options.data,
  });
  return result as Item;
}

async function handleAiAction(options: AiActionRequest): Promise<Record<string, unknown>> {
  const result = await invoke<unknown>("execute_ai_action", {
    pluginName: options.plugin,
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
