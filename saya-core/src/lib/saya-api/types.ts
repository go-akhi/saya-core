export interface PluginColumn {
  name: string;
  display: string;
  type: "main" | "secondary" | "filterable";
  dtype: "text" | "enum" | "number" | "date" | "boolean";
  sortable: boolean;
}

export interface ResultMapping {
  cognitive_axis: string;
  context_axis: string;
}

export interface AiAction {
  id: string;
  label: string;
  context_columns: string[];
  result_mapping: ResultMapping;
}

export interface FieldMapping {
  action_title: string;
  cognitive_axis: string;
  context_axis: string;
  source_type: string;
  source_id: string;
}

export interface ProvidedAction {
  label: string;
  target_types: string[];
  handler: string;
  field_mapping?: FieldMapping;
}

export interface PluginManifest {
  name: string;
  display_name: string;
  icon?: string;
  columns: PluginColumn[];
  ai_actions: AiAction[];
  provides_actions: ProvidedAction[];
}

export interface PluginInfo {
  name: string;
  display_name: string;
  icon: string | null;
  columns: PluginColumn[];
  ai_actions: AiAction[];
  provides_actions: ProvidedAction[];
  valid: boolean;
  errors: string[];
}

export interface Item {
  id: string;
  plugin_name: string;
  cognitive_axis: string | null;
  context_axis: string | null;
  [key: string]: unknown;
}

export interface QueryOptions {
  plugin: string;
  columns?: string[];
  filters?: Record<string, string | string[]>;
  sort?: { column: string; direction: "asc" | "desc" };
  limit?: number;
  offset?: number;
  operation?: "get_manifest" | "get_info";
}

export interface MutationOptions {
  plugin: string;
  operation: "create" | "update" | "delete";
  id?: string;
  data: Partial<Item>;
}

export interface AiActionRequest {
  plugin: string;
  action_id: string;
  item_ids: string[];
  context?: Record<string, unknown>;
}

export interface SubscriptionOptions {
  plugin: string;
  event: "items_changed" | "item_created" | "item_updated" | "item_deleted";
  callback: (payload: unknown) => void;
}

export type MessageType =
  | "query"
  | "mutate"
  | "subscribe"
  | "unsubscribe"
  | "ai_action"
  | "response"
  | "event";

export interface SayaMessage {
  id: string;
  type: MessageType;
  payload: unknown;
  source: "plugin" | "core";
  plugin?: string;
}

export interface ResponsePayload<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}
