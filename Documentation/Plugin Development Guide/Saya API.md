# Saya API Client Library

**Plugin-to-Core communication via postMessage**

## Overview

The `saya-api` library provides a standardized interface for plugins to communicate with the Saya Core application. All communication happens through the browser's `postMessage` API, ensuring plugins remain sandboxed and cannot make direct network requests.

## Installation

Plugins access the API through the global `window.parent` postMessage interface. Copy the `saya-api` types and library into your plugin's `ui/` directory.

## Types

### Core Types

```typescript
interface Item {
  id: string;
  plugin_name: string;
  cognitive_axis: string | null;
  context_axis: string | null;
  [key: string]: unknown;
}

interface PluginColumn {
  name: string;
  display: string;
  type: "main" | "secondary" | "filterable";
  dtype: "text" | "enum" | "number" | "date" | "boolean";
  sortable: boolean;
}

interface PluginManifest {
  name: string;
  display_name: string;
  icon?: string;
  columns: PluginColumn[];
  ai_actions: AiAction[];
  provides_actions: ProvidedAction[];
}

interface PluginInfo {
  name: string;
  display_name: string;
  icon: string | null;
  columns: PluginColumn[];
  ai_actions: AiAction[];
  provides_actions: ProvidedAction[];
  valid: boolean;
  errors: string[];
}

interface ErrorPayload {
  message: string;
  title?: string;
  type?: "error" | "warning" | "info";
}

### Completion API

```typescript
interface CompletionRequest {
  user: string;              // Required: The user's message/prompt
  system?: string;           // Optional: System/instruction prompt prepended to user message
  temperature?: number;      // Optional: Sampling temperature 0-2 (default: 0.7)
  max_tokens?: number;       // Optional: Max tokens to generate (default: 1024)
}

interface CompletionResponse {
  content: string;           // The LLM's response text
  model: string;             // The model used for generation
  provider: string;          // The provider used (openai, anthropic, etc.)
}
```

### AI Actions

```typescript
interface ResultMapping {
  cognitive_axis: string;
  context_axis: string;
}

interface AiAction {
  id: string;
  label: string;
  context_columns: string[];
  result_mapping: ResultMapping;
}
```

### Cross-Plugin Actions

```typescript
interface FieldMapping {
  action_title: string;
  cognitive_axis: string;
  context_axis: string;
  source_type: string;
  source_id: string;
}

interface ProvidedAction {
  label: string;
  target_types: string[];
  handler: string;
  field_mapping?: FieldMapping;
}
```

## SayaApi Class

### Constructor

```typescript
const api = new SayaApi(pluginName: string);
```

### Methods

#### `connect(iframe: HTMLIFrameElement): void`

Connect the API to the parent window. Call this when your plugin initializes.

```typescript
const api = new SayaApi("email");
api.connect(window.parent);
```

#### `disconnect(): void`

Disconnect and clean up listeners.

```typescript
api.disconnect();
```

#### `query<T extends Item>(options: QueryOptions): Promise<T[]>`

Fetch items from the plugin's data store.

```typescript
// Get all items
const items = await api.query();

// Get specific columns
const items = await api.query({
  columns: ["id", "subject", "sender"]
});

// With filters
const items = await api.query({
  filters: { cognitive_axis: "require" }
});

// Sorted and paginated
const items = await api.query({
  sort: { column: "created_at", direction: "desc" },
  limit: 20,
  offset: 0
});
```

**QueryOptions:**

| Property | Type | Description |
|----------|------|-------------|
| `plugin` | `string` | Plugin name (auto-set) |
| `columns` | `string[]` | Columns to retrieve |
| `filters` | `Record<string, string \| string[]>` | Filter conditions |
| `sort` | `{ column: string, direction: "asc" \| "desc" }` | Sort order |
| `limit` | `number` | Max results |
| `offset` | `number` | Skip count |
| `operation` | `"get_manifest" \| "get_info"` | Special operations |

#### `mutate<T extends Item>(options: MutationOptions): Promise<T>`

Create, update, or delete items.

```typescript
// Create item
const newItem = await api.mutate({
  operation: "create",
  data: {
    subject: "New Email",
    sender: "user@example.com",
    cognitive_axis: "require"
  }
});

// Update item
await api.mutate({
  operation: "update",
  id: "item-uuid-here",
  data: { cognitive_axis: "review" }
});

// Delete item
await api.mutate({
  operation: "delete",
  id: "item-uuid-here"
});
```

**MutationOptions:**

| Property | Type | Description |
|----------|------|-------------|
| `plugin` | `string` | Plugin name (auto-set) |
| `operation` | `"create" \| "update" \| "delete"` | Operation type |
| `id` | `string` | Item ID (required for update/delete) |
| `data` | `Partial<Item>` | Fields to set |

#### `aiAction(options: AiActionRequest): Promise<Record<string, unknown>>`

Trigger an AI classification action.

```typescript
const result = await api.aiAction({
  action_id: "classify",
  item_ids: ["item-1", "item-2"]
});
```

#### `subscribe(event: string, callback: (payload: unknown) => void): string`

Subscribe to real-time events from core.

```typescript
const subId = api.subscribe("items_changed", (payload) => {
  console.log("Items changed:", payload);
});
```

**Available Events:**

| Event | Description |
|-------|-------------|
| `items_changed` | Item collection modified |
| `item_created` | New item created |
| `item_updated` | Existing item modified |
| `item_deleted` | Item removed |

#### `unsubscribe(subscriptionId: string): void`

Stop receiving events.

```typescript
api.unsubscribe(subId);
```

#### `getManifest(): Promise<PluginManifest>`

Get the plugin's manifest metadata.

```typescript
const manifest = await api.getManifest();
console.log(manifest.columns);
```

#### `getPluginInfo(): Promise<PluginInfo>`

Get full plugin information including validation status.

```typescript
const info = await api.getPluginInfo();
```

#### `showError(payload: ErrorPayload): Promise<void>`

Display an error notification to the user via the core UI.

```typescript
// Basic error
await api.showError({
  message: "Failed to sync with server"
});

// With title and type
await api.showError({
  title: "Sync Failed",
  message: "Could not connect to the server",
  type: "error" // or "warning", "info"
});
```

**ErrorPayload:**

| Property | Type | Description |
|----------|------|-------------|
| `message` | `string` | Error message (required) |
| `title` | `string` | Optional title |
| `type` | `"error" \| "warning" \| "info"` | Notification type (default: "error") |

#### `complete(options: CompletionRequest): Promise<CompletionResponse>`

Send a completion request to the configured LLM endpoint. This is useful for chatbots, text generation, or any plugin that needs general-purpose LLM access.

```typescript
// Basic completion - just a user message
const response = await api.complete({
  user: "Hello! How are you?"
});
console.log(response.content);

// With system instruction and user message
const response = await api.complete({
  system: "You are a helpful assistant that speaks like a pirate.",
  user: "What's the weather?"
});

// With custom parameters
const response = await api.complete({
  system: "You are a concise summarizer.",
  user: "Summarize this article: ...",
  temperature: 0.5,
  max_tokens: 200
});
```

**CompletionRequest:**

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `user` | `string` | Yes | The user's message or prompt |
| `system` | `string` | No | System/instruction prompt prepended to user message |
| `temperature` | `number` | No | Sampling temperature 0-2 (default: 0.7) |
| `max_tokens` | `number` | No | Max tokens to generate (default: 1024) |

> **Important:** This API uses `system`/`user` string fields, NOT the `messages` array format. The core combines them into a single prompt internally.

**CompletionResponse:**

| Property | Type | Description |
|----------|------|-------------|
| `content` | `string` | Generated text response |
| `model` | `string` | Model used for generation |
| `provider` | `string` | Provider used (openai, anthropic, local, bedrock) |

### Properties

#### `isConnected: boolean`

Check if the API is connected to core.

```typescript
if (api.isConnected) {
  console.log("Ready to communicate");
}
```

## Plugin Context Helpers

### `initPlugin(pluginName: string, iframe: HTMLIFrameElement): PluginContext`

Initialize a plugin with API connection and manifest.

```typescript
import { initPlugin } from "./saya-api/plugin-context";

const context = initPlugin("email", window.parent);
console.log(context.pluginName); // "email"
```

### `destroyPlugin(context: PluginContext): void`

Clean up plugin resources.

```typescript
destroyPlugin(context);
```

## Message Protocol

Communication uses the `saya://` protocol via postMessage:

```typescript
interface SayaMessage {
  id: string;           // Unique request ID
  type: MessageType;   // "query" | "mutate" | "subscribe" | "ai_action" | "show_error" | "complete"
  payload: unknown;     // Request-specific data
  source: "plugin";    // Always "plugin" for outgoing
  plugin?: string;      // Target plugin name
}

interface ResponsePayload<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}
```

## Error Handling

All methods throw errors on failure:

```typescript
try {
  const items = await api.query({ filters: { invalid_column: "x" } });
} catch (error) {
  if (error.message.includes("timed out")) {
    console.log("Request timed out");
  } else {
    console.error("API error:", error.message);
  }
}
```

## Complete Example

```typescript
import { SayaApi } from "./saya-api";

const api = new SayaApi("email");

// Initialize
api.connect(window.parent);

// Load items
const emails = await api.query({
  filters: { cognitive_axis: "require" },
  sort: { column: "created_at", direction: "desc" }
});

// Update with AI classification
await api.aiAction({
  action_id: "classify",
  item_ids: emails.map(e => e.id)
});

// Subscribe to changes
api.subscribe("item_updated", (payload) => {
  console.log("Email updated:", payload);
});

// Cleanup
window.addEventListener("beforeunload", () => {
  api.disconnect();
});
```

## Restrictions

- Plugins **cannot** make direct `fetch()` or `XMLHttpRequest` calls
- All network requests must go through the Saya API
- The `saya://` protocol is whitelisted for internal communication
- Network isolation is enforced by the plugin scanner

## Plugin Manifest Integration

Plugins can declare actions that appear in the Core Actions Dock:

### AI Actions (Tooltip for AI Button)

When a plugin defines `ai_actions`, the first action's `label` appears as the tooltip for the AI button in the dock when that plugin is active.

```json
{
  "name": "email",
  "display_name": "Email",
  "ai_actions": [
    {
      "id": "classify",
      "label": "Classify Email",
      "context_columns": ["subject", "sender", "snippet"],
      "result_mapping": {
        "cognitive_axis": "cognitive_axis",
        "context_axis": "context_axis"
      }
    }
  ]
}
```

### Provided Actions (Dock Buttons)

Plugins can declare actions that appear as buttons in the dock via `provides_actions`:

```json
{
  "name": "notes",
  "display_name": "Notes",
  "provides_actions": [
    {
      "label": "Create Note",
      "target_types": ["email"],
      "handler": "pipeline:create_note"
    }
  ]
}
```

The dock automatically:
- Shows an AI action button (☆) for each plugin with `ai_actions`
- Shows additional action buttons based on `provides_actions`
- Icons are auto-assigned: `→` for pipeline handlers, `⚡` for others
- Tooltips display the `label` from the manifest
