# Saya Plugin Development Guide

**A comprehensive guide to building plugins for Saya Core**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Plugin Structure](#plugin-structure)
3. [The Manifest](#the-manifest)
4. [Database Schema](#database-schema)
5. [UI Development](#ui-development)
6. [AI Actions](#ai-actions)
7. [Cross-Plugin Actions](#cross-plugin-actions)
8. [Testing](#testing)
9. [Validation & Security](#validation--security)
10. [Best Practices](#best-practices)
11. [Example: Email Plugin](#example-email-plugin)
12. [Publishing to the Marketplace](#publishing-to-the-marketplace)

---

## Introduction

Saya is a personal knowledge management system with a plugin-based architecture. Plugins extend Saya's functionality by providing:

- **Data storage** for custom item types (emails, tasks, notes)
- **AI classification** capabilities via the cognitive axis framework
- **Cross-plugin actions** to integrate with other plugins

### Core Concepts

**Cognitive Axis (4R Framework):**
- **Require** — Things you must act on
- **Review** — Things to evaluate periodically
- **Retain** — Reference material to keep
- **Relieve** — Completed or archived items

**Context Axis:**
- User-defined categories for organizing items (e.g., "Work", "Personal", "Side Project")
- Created and managed by users through the Saya Core UI
- Plugins should never create or modify context_axis values — just assign them to items

**Managing context_axis values:**

Context axes are created by users in Saya Core's settings (Settings → Context Axes). They cannot be created programmatically by plugins. When users create axes, they appear in the filter strips at the top of the UI.

| Setting | Description |
|---------|-------------|
| `name` | Unique identifier (e.g., "work", "personal") |
| `description` | Optional explanation |
| `icon` | Emoji for display |
| `color` | Color for visual distinction |

**How plugins use context_axis:**

```typescript
// When creating an item, assign a context
await api.mutate({
    operation: "create",
    data: {
        subject: "Project meeting",
        cognitive_axis: "require",
        context_axis: "work"  // Assign to "Work" context
    }
});

// Query items for a specific context
const workEmails = await api.query({
    filters: { context_axis: "work" }
});
```

**Important:** Do not hardcode context_axis values in your plugin. The context values are entirely user-defined. Your plugin should accept whatever values the user has configured.

**Items:**
- The fundamental data unit in Saya
- Every item belongs to a plugin and has `cognitive_axis` and `context_axis` fields

---

## Plugin Structure

Each plugin lives in its own directory under `~/.local/share/saya-core/plugins/`:

```
email/
├── manifest.json          # Plugin declaration
├── schema.sql            # Database schema
└── ui/
    ├── index.html        # Entry point
    ├── styles.css        # Plugin styles
    └── app.js           # Plugin logic
```

### Directory Requirements

| Path | Required | Description |
|------|----------|-------------|
| `manifest.json` | Yes | Plugin metadata and configuration |
| `schema.sql` | Yes | SQLite CREATE TABLE statements |
| `ui/index.html` | Yes | Entry point loaded in iframe |
| `ui/styles.css` | No | Optional styles |
| `ui/app.js` | No | Optional main script |

---

## The Manifest

The `manifest.json` is the plugin's declaration file. It defines the plugin's identity, data schema, and capabilities.

### Full Manifest Schema

```json
{
  "name": "email",
  "display_name": "Email",
  "icon": "📧",
  "columns": [...],
  "ai_actions": [...],
  "provides_actions": [...]
}
```

### Properties

#### `name` (required)

Unique identifier for the plugin. Must be lowercase, alphanumeric with hyphens allowed.

```json
"name": "email"
```

#### `display_name` (required)

Human-readable name shown in the UI.

```json
"display_name": "Email"
```

#### `icon` (optional)

Emoji or character displayed in the plugin sidebar.

```json
"icon": "📧"
```

#### `version` (optional)

Semver version string for the plugin. Used for marketplace listings and migration tracking.

```json
"version": "0.1.0"
```

#### `columns` (required)

Defines the data fields for items managed by this plugin.

```json
"columns": [
  {
    "name": "subject",
    "display": "Subject",
    "type": "main",
    "dtype": "text",
    "sortable": true
  },
  {
    "name": "sender",
    "display": "From",
    "type": "secondary",
    "dtype": "text",
    "sortable": false
  },
  {
    "name": "received_at",
    "display": "Received",
    "type": "secondary",
    "dtype": "date",
    "sortable": true
  },
  {
    "name": "cognitive_axis",
    "display": "Axis",
    "type": "filterable",
    "dtype": "enum",
    "sortable": true
  },
  {
    "name": "context_axis",
    "display": "Context",
    "type": "filterable",
    "dtype": "text",
    "sortable": false
  }
]
```

**Column Properties:**

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | Yes | Field name (snake_case) |
| `display` | string | Yes | Human-readable label |
| `type` | string | Yes | `main`, `secondary`, or `filterable` |
| `dtype` | string | Yes | `text`, `enum`, `number`, `date`, `boolean` |
| `sortable` | boolean | Yes | Can be used for sorting |

**Column Types:**

| Type | Description |
|------|-------------|
| `main` | Primary content field (subject, title, etc.) |
| `secondary` | Additional display fields |
| `filterable` | Fields used for filtering (including cognitive_axis, context_axis) |

**Required Columns:**

Every plugin **must** define these columns:

- `cognitive_axis` — Links to the 4R framework
- `context_axis` — Links to user context categories

#### `ai_actions` (optional)

Defines AI-powered classification actions.

```json
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
```

**AI Action Properties:**

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `id` | string | Yes | Unique action identifier |
| `label` | string | Yes | Display name (shown as tooltip) |
| `context_columns` | string[] | Yes | Fields sent to LLM for context |
| `result_mapping` | object | Yes | Maps LLM output to item fields |

**Result Mapping:**

| Property | Type | Description |
|----------|------|-------------|
| `cognitive_axis` | string | Column to store 4R classification |
| `context_axis` | string | Column to store context category |

#### `provides_actions` (optional)

Declares actions that this plugin can perform on other plugins' items.

```json
"provides_actions": [
  {
    "label": "Create Task",
    "target_types": ["email", "note"],
    "handler": "pipeline:create_task",
    "field_mapping": {
      "action_title": "source.title || source.subject",
      "cognitive_axis": "source.cognitive_axis",
      "context_axis": "source.context_axis",
      "source_type": "email",
      "source_id": "source.id"
    }
  }
]
```

**Provided Action Properties:**

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `label` | string | Yes | Display name shown in dock |
| `target_types` | string[] | Yes | Plugins this action applies to |
| `handler` | string | Yes | Handler identifier (format: `type:name`) |
| `field_mapping` | object | No | Maps source item fields to target |

**Handler Types:**

| Prefix | Description |
|--------|-------------|
| `pipeline:` | Cross-plugin data transfer |
| `action:` | Custom action handler |

#### `has_settings` (optional)

Declares whether this plugin provides a settings UI. When `true`, the Settings button in the plugin sidebar footer will be enabled. Clicking it opens a popup displaying the plugin's `ui/settings.html` in a fixed-width panel (320px wide, max 400px tall).

```json
"has_settings": true
```

**Behavior:**
- If `true`: Settings button is enabled when this plugin is active
- If `false` or property absent (default): Settings button is grayed out
- Settings are rendered via iframe (path: `ui/settings.html`)
- The settings iframe has access to the same `saya-api` library

---

## Database Schema

Each plugin must define its SQLite table schema in `schema.sql`. The schema is automatically executed when the plugin is registered.

### Important: One Table Per Plugin

Each plugin gets exactly **one table** for its items. You cannot create multiple tables in `schema.sql`. If your plugin needs related data (like chat messages for a chatbot), you should store them as JSON within your main table.

If you need more complex structures, consider:
- Using JSON columns (SQLite supports JSON extraction)
- Storing related items as separate rows with a type or parent ID field
- Using the core's settings storage via `api.saveSettings()` for configuration

### Schema Requirements

```sql
CREATE TABLE email_items (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    subject TEXT NOT NULL,
    sender TEXT,
    snippet TEXT,
    cognitive_axis TEXT DEFAULT 'review',
    context_axis TEXT
);

CREATE INDEX idx_email_cognitive ON email_items(cognitive_axis);
CREATE INDEX idx_email_context ON email_items(context_axis);
```

### Requirements

1. **Primary key** must be `id` as TEXT
2. **Created timestamp** must be `created_at` as TEXT
3. **Required columns** — `cognitive_axis` and `context_axis` must exist
4. **Naming convention** — Table name format: `{plugin_name}_items`

### Schema Migrations

When you update your plugin and need to change the database schema, you must handle migrations yourself. The core does **not** automatically re-run `schema.sql` on updates.

**Current limitation:** Plugins cannot execute raw SQL (ALTER TABLE, etc.). Schema changes require the user to reinstall the plugin.

**Recommended approach:**

1. Store a version number in your plugin settings
2. On first load, check if migrations are needed
3. If your schema version is outdated, show a message prompting the user to reinstall

```typescript
import { SayaApi } from "./saya-api/index.ts";

const api = new SayaApi("my-plugin");

async function checkSchemaVersion() {
    const settings = await api.loadSettings();
    const currentVersion = settings.schema_version || 0;
    const requiredVersion = 2; // Your current schema version

    if (currentVersion < requiredVersion) {
        showReinstallBanner("Schema outdated. Please reinstall the plugin.");
        return false;
    }
    return true;
}
```

**When to release a new schema version:**

1. Update `schema.sql` with new columns/tables
2. Increment your plugin version in `manifest.json` and `plugins.json`
3. Publish to the marketplace
4. Users reinstall to get the new schema

### Item Fields

When creating items via the API, these fields are automatically managed:

| Field | Type | Description |
|-------|------|-------------|
| `id` | TEXT | UUID auto-generated if not provided |
| `created_at` | TEXT | ISO 8601 timestamp auto-generated |

### JSON Columns

Since each plugin gets only one table, store complex/nested data as JSON in TEXT columns. SQLite supports JSON extraction via `JSON_EXTRACT()`.

**Example: Chatbot plugin storing messages as JSON**

```sql
CREATE TABLE chatbot_items (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    title TEXT NOT NULL,                    -- Thread title
    messages TEXT DEFAULT '[]',             -- JSON array of messages
    cognitive_axis TEXT DEFAULT 'review',
    context_axis TEXT
);
```

**Message JSON structure:**

```json
[
    {
        "role": "user",
        "content": "Hello",
        "timestamp": "2026-03-28T12:00:00Z"
    },
    {
        "role": "assistant",
        "content": "Hi! How can I help?",
        "timestamp": "2026-03-28T12:00:01Z"
    }
]
```

**Filtering by JSON content:**

The core API queries use SQLite under the hood. While the current API doesn't expose raw JSON filtering, you can:
1. Store key fields as separate columns for filtering
2. Keep JSON for display/complex data

```sql
-- For a chatbot thread, store the last message timestamp separately
CREATE TABLE chatbot_items (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    title TEXT NOT NULL,
    last_message_at TEXT,                    -- Filterable column
    message_count INTEGER DEFAULT 0,         -- Filterable column
    messages TEXT DEFAULT '[]',               -- JSON for display
    cognitive_axis TEXT DEFAULT 'review',
    context_axis TEXT
);
```

---

## UI Development

Plugins render their UI in an iframe embedded in Saya Core.

### Entry Point

`ui/index.html` is loaded when the plugin is selected:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Email</title>
    <style>
        /* Plugin styles */
    </style>
</head>
<body>
    <div id="app"></div>
    <script type="module">
        // Plugin logic
    </script>
</body>
</html>
```

### CSS Guidelines

Follow Saya's design tokens for consistency:

```css
:root {
    --bg-primary: #faf9f7;
    --bg-card: #ffffff;
    --bg-hover: #f0efec;
    --text-primary: #1a1a1a;
    --text-secondary: #6b6a67;
    --text-muted: #a09f9c;
    --border: #e5e4e1;
    --accent: #d97706;
    --radius: 6px;
}

body {
    font-family: system-ui, -apple-system, sans-serif;
    background-color: var(--bg-primary);
    color: var(--text-primary);
}
```

### Initializing the Plugin

Use the `saya-api` library to communicate with core. Plugins should copy the `saya-api` directory from the core into their `ui/` folder:

```
email/
├── manifest.json
├── schema.sql
└── ui/
    ├── index.html
    └── saya-api/
        ├── index.ts    # SayaApi class
        └── types.ts    # TypeScript interfaces
```

Then import:

```typescript
import { SayaApi } from "./saya-api/index.ts";

const api = new SayaApi("email");
api.connect(window.parent);

// Now you can use the API
async function loadEmails() {
    const emails = await api.query({
        filters: { cognitive_axis: "require" },
        sort: { column: "created_at", direction: "desc" }
    });
    renderEmails(emails);
}

loadEmails();
```

### Plugin Settings

If your plugin declares `has_settings: true` in the manifest, you should provide a settings UI at `ui/settings.html`. This file is loaded into a popup when the user clicks the Settings button in the plugin sidebar footer.

**The settings popup:**
- Appears as a flyout panel to the right of the sidebar
- Width: 320px, max height: 400px
- Contains your `ui/settings.html` in an iframe
- Has access to the same `saya-api` library
- Close button in the top-right corner

**Settings storage:**
- Use `api.saveSettings(data)` to persist settings (key-value store)
- Use `api.loadSettings()` to retrieve saved settings
- Settings are stored per-plugin in the core's database
- Settings persist across app restarts

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Email Settings</title>
    <style>
        :root {
            --bg-primary: #faf9f7;
            --text-primary: #1a1a1a;
            --text-secondary: #6b6a67;
            --border: #e5e4e1;
            --accent: #d97706;
        }

        body {
            font-family: system-ui, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            padding: 16px;
        }

        .setting-group {
            margin-bottom: 16px;
        }

        .setting-label {
            font-size: 13px;
            font-weight: 500;
            color: var(--text-secondary);
            margin-bottom: 4px;
        }

        input[type="text"] {
            width: 100%;
            padding: 8px 12px;
            border: 1px solid var(--border);
            border-radius: 6px;
            font-size: 14px;
        }

        button.save {
            background: var(--accent);
            color: white;
            border: none;
            padding: 8px 16px;
            border-radius: 6px;
            cursor: pointer;
        }
    </style>
</head>
<body>
    <div class="setting-group">
        <label class="setting-label">Email Provider</label>
        <input type="text" id="provider" placeholder="imap.example.com">
    </div>
    <button class="save" onclick="saveSettings()">Save</button>

    <script type="module">
        import { SayaApi } from "./saya-api/index.ts";

        const api = new SayaApi("email");
        api.connect(window.parent);

        async function saveSettings() {
            const provider = document.getElementById("provider").value;
            await api.saveSettings({ provider });
        }

        // Load existing settings
        async function loadExistingSettings() {
            const settings = await api.loadSettings();
            if (settings.provider) {
                document.getElementById("provider").value = settings.provider;
            }
        }

        loadExistingSettings();
    </script>
</body>
</html>
```

**Settings Behavior:**
- Settings popup appears when the active plugin has `has_settings: true`
- The popup displays the plugin's `display_name` in the header
- Plugins can access settings storage via `api.saveSettings()` and `api.loadSettings()`

### Event Subscriptions

Plugins can listen for changes to their items using the `subscribe` method:

```typescript
const subscriptionId = api.subscribe('items_changed', (payload) => {
    console.log('Items changed:', payload);
});

// Later, unsubscribe
api.unsubscribe(subscriptionId);
```

**Available events:**

| Event | Description |
|-------|-------------|
| `items_changed` | Any change to the plugin's items |
| `item_created` | A new item was created |
| `item_updated` | An existing item was updated |
| `item_deleted` | An item was deleted |

**Event payload structure:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | The event type (e.g., `item_created`) |
| `item` | object | The affected item (for `item_created`, `item_updated`, `item_deleted`) |
| `items` | object[] | Array of all items after the change (for `items_changed`) |

**Example payload for `item_created`:**

```json
{
    "type": "item_created",
    "item": {
        "id": "uuid-1234",
        "subject": "New Email",
        "sender": "test@example.com",
        "cognitive_axis": "require",
        "context_axis": "Work"
    }
}
```

**Example payload for `items_changed`:**

```json
{
    "type": "items_changed",
    "items": [
        { "id": "uuid-1234", "subject": "Email 1", "cognitive_axis": "require" },
        { "id": "uuid-5678", "subject": "Email 2", "cognitive_axis": "review" }
    ]
}
```

---

## AI Actions

AI actions enable plugins to leverage LLMs for automatic classification.

### How It Works

1. Plugin declares `ai_actions` in manifest
2. User triggers action via the dock
3. Core sends relevant columns to configured LLM endpoint
4. LLM returns classification
5. Core updates item fields per `result_mapping`

### Declaring AI Actions

```json
{
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

### Context Columns

Choose columns that provide meaningful context for classification:

- Good: `subject`, `sender`, `snippet`, `body`
- Avoid: `id`, `created_at`, internal fields

### Prompt Engineering

The LLM receives a structured prompt with:

1. The 4R definitions
2. Available context categories
3. The item's data from `context_columns`

Your plugin should ensure `context_columns` contain sufficient information for accurate classification.

---

## Cross-Plugin Actions

Cross-plugin actions allow plugins to create items based on other plugins' data.

### Important: Plugins Cannot Communicate Directly

Plugins are sandboxed in iframes and **cannot** call other plugins' APIs or access their data directly. This is by design for security and isolation.

The only way for plugins to interact is through `provides_actions`, which is mediated by the core.

**What plugins CANNOT do:**
- Call another plugin's API methods directly
- Access another plugin's iframe or DOM
- Read/write another plugin's data via `api.query()` or `api.mutate()` on other plugins
- Communicate via postMessage between plugin iframes

**What plugins CAN do:**
- Declare `provides_actions` to offer actions to other plugins
- Trigger AI actions on other plugins via the dock
- Use the core's settings storage shared by all plugins

### Use Cases

- Create a task from an email
- Convert a note to a task
- Archive related items across plugins

### Declaring Cross-Plugin Actions

```json
{
  "name": "tasks",
  "provides_actions": [
    {
      "label": "Create Task",
      "target_types": ["email", "note"],
      "handler": "pipeline:create_task",
      "field_mapping": {
        "action_title": "source.title || source.subject",
        "cognitive_axis": "source.cognitive_axis",
        "context_axis": "source.context_axis",
        "source_type": "email",
        "source_id": "source.id"
      }
    }
  ]
}
```

### Field Mapping

The `field_mapping` object defines how data flows from source to target:

| Property | Description |
|----------|-------------|
| `action_title` | Expression to set the item's title |
| `cognitive_axis` | Copy cognitive axis from source |
| `context_axis` | Copy context axis from source |
| `source_type` | The plugin being acted upon |
| `source_id` | Reference to the source item |

### Expression Syntax

Field mappings support simple expressions:

```javascript
"action_title": "source.subject"                    // Direct copy
"action_title": "'Task: ' + source.subject"       // Prepend
"action_title": "source.title || source.subject"    // Fallback
```

### Handler Types

**Pipeline Handlers** (`pipeline:`):
- Transfer data between plugins
- Core manages the cross-plugin logic

**Custom Handlers** (`action:`):
- Custom business logic
- Require implementation in the target plugin

---

## Testing

### Local Development (Hot Reload)

Saya Core watches your plugin's `ui/` directory for changes. When you edit any file in your plugin's UI folder, the iframe automatically reloads.

**Setup for development:**

1. Create a symlink or place your plugin directly in `~/.local/share/saya-core/plugins/`
   ```bash
   # Option A: Symlink (recommended for active development)
   ln -s /path/to/your/plugin ~/.local/share/saya-core/plugins/my-plugin

   # Option B: Copy directly
   cp -r /path/to/your/plugin ~/.local/share/saya-core/plugins/my-plugin
   ```
2. Launch Saya Core
3. Select your plugin in the sidebar
4. Edit files in your plugin's `ui/` folder — changes appear instantly

**What triggers a reload:**
- Editing `ui/index.html`
- Editing `ui/styles.css`
- Editing any `.js` or `.ts` file in `ui/`
- Adding/removing files in `ui/`

**What does NOT trigger a reload:**
- Changes to `manifest.json` (requires restart)
- Changes to `schema.sql` (requires reinstall)
- Changes outside `ui/` directory

### Manual Testing

1. Place your plugin in `~/.local/share/saya-core/plugins/`
2. Restart Saya Core
3. Plugin should appear in the sidebar
4. Check the console for errors

### Validation

Run the built-in plugin scanner:

```bash
# From saya-core directory
cargo run --bin plugin-validate -- email
```

The scanner checks:
- Manifest syntax
- Required columns
- Cross-plugin references
- Network isolation (no fetch/XHR)

### Test Data

Create test items via the API:

```javascript
const testItem = await api.mutate({
    operation: "create",
    data: {
        subject: "Test Email",
        sender: "test@example.com",
        cognitive_axis: "require"
    }
});
```

---

## Validation & Security

### Network Isolation

Plugins **cannot** make direct network requests. All external communication must go through Saya Core. The plugin scanner enforces this by:

1. Scanning all `.js`, `.html`, `.ts` files
2. Detecting `fetch()`, `XMLHttpRequest`, `axios`
3. Whitelisting only `saya://` protocol

**Allowed:**
```javascript
// Core API calls (sandboxed)
const items = await api.query();
```

**Forbidden:**
```javascript
// Direct network calls (blocked)
fetch('https://api.example.com');  // ❌
axios.get('/data');                  // ❌
new XMLHttpRequest();                 // ❌
```

### Manifest Validation

Plugins must pass validation to be registered:

1. **Required fields** — `name`, `display_name`, `columns`
2. **Required columns** — `cognitive_axis`, `context_axis`
3. **Valid references** — `target_types` must reference registered plugins
4. **Valid field mappings** — Source plugins must exist

---

## Best Practices

### Plugin Naming

- Use lowercase with hyphens: `my-plugin`
- Keep names concise: `email` not `email-client`
- Avoid conflicts with future core plugins

### Column Design

- **Main column** — One primary field (subject, title)
- **Secondary columns** — Supporting details
- **Filterable** — Fields users filter/sort by

### Performance

**Pagination:**

Always paginate queries for datasets over 50 items. Recommended page sizes:

| Dataset Size | Recommended Page Size | Notes |
|-------------|----------------------|-------|
| < 50 items | No pagination needed | Load all at once |
| 50-500 items | 50 items per page | Fast scrolling |
| 500-5000 items | 25 items per page | Balance UX and performance |
| 5000+ items | 20 items per page | Consider search instead |

```typescript
// Example: Paginated email list
const PAGE_SIZE = 50;
let page = 0;

async function loadPage() {
    const emails = await api.query({
        sort: { column: "created_at", direction: "desc" },
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE
    });
    renderEmails(emails);
}
```

**Indexing strategies:**

Always create indexes on:
- `cognitive_axis` — Used for filtering (always)
- `context_axis` — Used for filtering (always)
- Any column used in `sort` — e.g., `created_at`, `received_at`
- Any column frequently filtered — e.g., `sender`, `status`

```sql
-- Essential indexes
CREATE INDEX idx_plugin_cognitive ON {name}_items(cognitive_axis);
CREATE INDEX idx_plugin_context ON {name}_items(context_axis);

-- Recommended for common queries
CREATE INDEX idx_plugin_created ON {name}_items(created_at);
```

**Row count guidelines:**

| Plugin Type | Expected Volume | Design Consideration |
|-------------|-----------------|---------------------|
| Email | 10,000+ | Heavy pagination, archive old items |
| Tasks | 500-2000 | Keep completed items in "Relieve" axis |
| Notes | 100-500 | Most queries on recent items |
| Chatbot | 10,000+ messages | Store messages as JSON in single row |

**Lazy loading:**
- Load items on demand, not all at once
- Consider implementing infinite scroll for high-volume plugins
- Cache frequently accessed data in plugin settings

### UX Consistency

- Follow Saya's design tokens (colors, spacing)
- Support keyboard navigation
- Provide loading states for async operations
- Show appropriate empty states

### Error Handling

```typescript
try {
    const items = await api.query({ filters: { invalid: "x" } });
} catch (error) {
    if (error.message.includes("timed out")) {
        showRetryButton();
    } else {
        showErrorMessage(error.message);
    }
}
```

### Error Reference

| Error Pattern | Description | Recommended Action |
|---------------|-------------|-------------------|
| `timed out after {ms}ms` | Request exceeded timeout (default: 30s) | Retry the request |
| `Not connected to core` | API called before `connect()` | Call `api.connect(window.parent)` first |
| `Plugin not found` | Invalid plugin name | Check manifest `name` matches |
| `Invalid filter` | Unknown filter column name | Verify column exists in manifest |
| `Plugin '{name}' is missing the required 'cognitive_axis' column` | Schema validation error | Add `cognitive_axis` column |
| `Plugin '{name}' is missing the required 'context_axis' column` | Schema validation error | Add `context_axis` column |
| `Plugin '{name}' declares actions for '{target}' which is not registered` | Cross-plugin action error | Ensure target plugin exists |
| `Network isolation violation detected` | Plugin contains forbidden network calls | Remove fetch/XHR/axios calls |

---

## Example: Email Plugin

### manifest.json

```json
{
    "name": "email",
    "display_name": "Email",
    "icon": "📧",
    "has_settings": true,
    "columns": [
        { "name": "subject", "display": "Subject", "type": "main", "dtype": "text", "sortable": true },
        { "name": "sender", "display": "From", "type": "secondary", "dtype": "text", "sortable": false },
        { "name": "snippet", "display": "Preview", "type": "secondary", "dtype": "text", "sortable": false },
        { "name": "received_at", "display": "Received", "type": "secondary", "dtype": "date", "sortable": true },
        { "name": "cognitive_axis", "display": "Axis", "type": "filterable", "dtype": "enum", "sortable": true },
        { "name": "context_axis", "display": "Context", "type": "filterable", "dtype": "text", "sortable": false }
    ],
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
    ],
    "provides_actions": [
        {
            "label": "Create Task",
            "target_types": ["email"],
            "handler": "pipeline:create_task",
            "field_mapping": {
                "action_title": "'Task from: ' + source.subject",
                "cognitive_axis": "source.cognitive_axis",
                "context_axis": "source.context_axis",
                "source_type": "email",
                "source_id": "source.id"
            }
        }
    ]
}
```

### schema.sql

```sql
CREATE TABLE email_items (
    id TEXT PRIMARY KEY,
    created_at TEXT NOT NULL,
    subject TEXT NOT NULL,
    sender TEXT,
    snippet TEXT,
    body TEXT,
    received_at TEXT,
    cognitive_axis TEXT DEFAULT 'review',
    context_axis TEXT,
    is_read INTEGER DEFAULT 0,
    is_starred INTEGER DEFAULT 0
);

CREATE INDEX idx_email_cognitive ON email_items(cognitive_axis);
CREATE INDEX idx_email_context ON email_items(context_axis);
CREATE INDEX idx_email_received ON email_items(received_at);
CREATE INDEX idx_email_sender ON email_items(sender);
```

### ui/index.html

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Email</title>
    <style>
        :root {
            --bg-primary: #faf9f7;
            --bg-card: #ffffff;
            --bg-hover: #f0efec;
            --text-primary: #1a1a1a;
            --text-secondary: #6b6a67;
            --text-muted: #a09f9c;
            --border: #e5e4e1;
            --accent: #d97706;
        }

        * { box-sizing: border-box; margin: 0; padding: 0; }

        body {
            font-family: system-ui, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            height: 100vh;
            overflow: hidden;
        }

        .email-list {
            height: 100%;
            overflow-y: auto;
        }

        .email-item {
            padding: 12px 16px;
            border-bottom: 1px solid var(--border);
            cursor: pointer;
            transition: background-color 150ms;
        }

        .email-item:hover {
            background: var(--bg-hover);
        }

        .email-item.selected {
            background: var(--bg-card);
            border-left: 3px solid var(--accent);
        }

        .email-subject {
            font-weight: 500;
            margin-bottom: 4px;
        }

        .email-meta {
            font-size: 12px;
            color: var(--text-secondary);
        }

        .email-snippet {
            font-size: 13px;
            color: var(--text-muted);
            margin-top: 4px;
        }

        .empty-state {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100%;
            color: var(--text-muted);
        }
    </style>
</head>
<body>
    <div id="app">
        <div class="email-list" id="emailList">
            <div class="empty-state">Loading emails...</div>
        </div>
    </div>

    <script type="module">
        import { SayaApi } from "./saya-api/index.ts";

        const api = new SayaApi("email");
        api.connect(window.parent);

        let selectedId = null;

        async function loadEmails() {
            try {
                const emails = await api.query({
                    sort: { column: "received_at", direction: "desc" }
                });

                const list = document.getElementById("emailList");

                if (emails.length === 0) {
                    list.innerHTML = '<div class="empty-state">No emails yet</div>';
                    return;
                }

                list.innerHTML = emails.map(email => `
                    <div class="email-item ${email.id === selectedId ? 'selected' : ''}"
                         data-id="${email.id}">
                        <div class="email-subject">${escapeHtml(email.subject)}</div>
                        <div class="email-meta">${escapeHtml(email.sender)}</div>
                        <div class="email-snippet">${escapeHtml(email.snippet || '')}</div>
                    </div>
                `).join('');

                list.querySelectorAll('.email-item').forEach(item => {
                    item.addEventListener('click', () => selectEmail(item.dataset.id));
                });
            } catch (error) {
                console.error('Failed to load emails:', error);
                document.getElementById("emailList").innerHTML =
                    '<div class="empty-state">Error loading emails</div>';
            }
        }

        function selectEmail(id) {
            selectedId = id;
            document.querySelectorAll('.email-item').forEach(item => {
                item.classList.toggle('selected', item.dataset.id === id);
            });
        }

        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }

        // Initial load
        loadEmails();

        // Listen for changes
        api.subscribe('items_changed', () => loadEmails());
    </script>
</body>
</html>
```

---

## Publishing to the Marketplace

Once your plugin is built and tested, you can publish it to the Saya Plugin Marketplace so users can discover and install it directly from the app.

### Overview

The marketplace is a static JSON registry hosted on GitHub Pages at `https://<org>.github.io/saya/plugins.json`. Users browse available plugins through the in-app marketplace, view README previews, and install with one click.

### Step 1: Host Your Plugin on GitHub

Your plugin must live in its own public GitHub repository. The repo should contain:

```
plugin-email/
├── manifest.json
├── schema.sql
├── README.md          # Shown in the in-app marketplace detail view
└── ui/
    ├── index.html
    ├── styles.css     # (optional)
    └── app.js         # (optional)
```

The repo name should follow the convention `plugin-<name>`, matching the `name` field in your `manifest.json`.

### Step 2: Create a Release Tag

Tag your plugin with a semver version that matches the `version` you plan to register:

```bash
git tag v0.1.0
git push origin v0.1.0
```

### Step 3: Add Your Plugin to the Registry

Fork the main Saya repo and edit `plugins.json` at the root. Add an entry to the `plugins` array:

```json
{
  "name": "email",
  "display_name": "Email",
  "icon": "📧",
  "version": "0.1.0",
  "description": "Gmail and Outlook integration with AI-powered triage and the 4R cognitive framework.",
  "repo_url": "https://github.com/saya-org/plugin-email",
  "manifest": {
    "columns": [
      { "name": "subject", "display": "Subject", "type": "main", "dtype": "text", "sortable": true },
      { "name": "sender", "display": "From", "type": "secondary", "dtype": "text", "sortable": false },
      { "name": "cognitive_axis", "display": "Axis", "type": "filterable", "dtype": "enum", "sortable": true },
      { "name": "context_axis", "display": "Context", "type": "filterable", "dtype": "text", "sortable": false }
    ]
  }
}
```

**Field reference:**

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Must match `manifest.json` `name` in your plugin repo |
| `display_name` | Yes | Human-readable label shown in marketplace cards |
| `icon` | Yes | Emoji displayed on the card |
| `version` | Yes | Semver string matching your latest release tag |
| `description` | Yes | Short description (1-2 sentences) shown on the card |
| `repo_url` | Yes | Full URL to the GitHub repo |
| `manifest.columns` | Yes | Copy of your plugin's `columns` so the card view can show schema info without fetching the repo |

### Step 4: Submit a Pull Request

1. Commit your change to `plugins.json`
2. Open a PR against the main Saya repo
3. The PR will be reviewed for:
   - Valid JSON syntax
   - Matching `name` between registry entry and plugin `manifest.json`
   - Repo exists and is public
   - Description is clear and concise
4. The PR is reviewed for valid JSON, matching plugin name, and public repo
5. Once merged, the GitHub Actions workflow signs the registry and deploys to GitHub Pages

### Verified vs Community Plugins

Every plugin in the registry has a `verified` flag. This is determined by the core maintainers — plugin authors cannot request or set it themselves.

| Status | `verified` | Meaning |
|--------|-----------|---------|
| **Verified** | `true` | The core team has reviewed the plugin and considers it valuable, secure, and privacy-focused. Shown with a checkmark badge. Listed above community plugins in discovery. |
| **Community** | `false` | Listed in the registry but not reviewed by the core team. Shown with a neutral badge. Listed below verified plugins in discovery. |

**How a plugin becomes verified:**

There is no application or request process. The core maintainers proactively review plugins in the registry. When the team determines a plugin adds value to the platform, is secure, and respects user privacy, they set `verified: true` in a subsequent update to `plugins.json`.

This can happen at any time after a plugin is merged — it may be immediate if the plugin is simple and well-written, or it may take longer as the team gets to it. A plugin that is not yet verified is still fully installable and functional; the badge only reflects the platform's endorsement.

**What the core team looks at when reviewing:**

- Code quality and readability
- No unnecessary network calls or data exfiltration
- Minimal dependencies
- Clear, accurate README
- Adherence to the plugin contract (required columns, manifest structure)

**The `verified` field changes exactly two things in the app:**

1. The badge displayed on the plugin card (checkmark vs neutral)
2. Sort order in discovery — verified plugins appear above unverified plugins

Both verified and unverified plugins are fully installable. There is no technical restriction or warning for community plugins.

**Registry signing:**

The `plugins.json` file is signed with an Ed25519 signature so the app can verify the registry hasn't been tampered with. The `signature` and `public_key` fields are managed automatically by CI — plugin authors should not include or modify them:

```json
{
  "signature": "...",
  "public_key": "...",
  "plugins": [...]
}
```

On every fetch, the app verifies this signature. If verification fails, the app refuses to load the registry.

### How Installation Works

When a user clicks "Install Plugin" in the app:

1. The app downloads the repo as a zipball from `https://api.github.com/repos/{owner}/{repo}/zipball/v{version}`
2. The zip is extracted into `~/.local/share/saya-core/plugins/{name}/`
3. The app runs `discover_plugins` which reads the `manifest.json` and registers the plugin in the database
4. The plugin appears in the sidebar immediately

### README Tips

Your `README.md` is displayed directly in the app's plugin detail view. Keep it focused:

- **What the plugin does** — 1-2 paragraphs max
- **Setup requirements** — Any accounts, API keys, or configuration needed
- **Screenshots** — Optional but helpful (use absolute GitHub URLs for images)
- **Usage guide** — Brief walkthrough of core features

Avoid relative links, local image paths, or badges that won't render outside GitHub.

### Updating Your Plugin

1. Make changes to your plugin repo
2. Tag a new release (e.g. `v0.2.0`)
3. Update the `version` and any changed fields in `plugins.json` in the main Saya repo
4. Submit a new PR

Existing users will see the updated version in the marketplace and can reinstall.

---

## External Data Integration

Plugins currently sync data manually — users must trigger actions or the plugin polls via `api.query()`. Real-time sync and webhooks are not supported in v1.

**Out of scope for v1:**
- Webhook endpoints (e.g., receiving Gmail push notifications)
- Real-time sync services
- Background polling/cron jobs
- P2P sync between instances

**What plugins can do today:**
- Fetch data on user action (e.g., "Sync" button)
- Use AI actions to process data
- Store data locally and query it

**Future considerations:**
- Webhook support via core-managed endpoints
- Background sync service
- Real-time event streams from external services

---

## Getting Help

- **API Reference** — See `Documentation/Working documents/Saya API.md`
- **Implementation Plan** — See `Documentation/Working documents/Implementation Plan.md`
- **Issues** — Report bugs at https://github.com/anomalyco/saya/issues

---

*Last updated: 2026-03-28*
