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

Declares whether this plugin provides a settings UI. When `true`, the Settings button in the plugin toolbar will be enabled and display a popup with the plugin's settings interface.

```json
"has_settings": true
```

**Behavior:**
- If `true` or omitted: Settings button is enabled when this plugin is active
- If `false` or property absent: Settings button is grayed out
- Settings are rendered via iframe (path: `ui/settings.html`)

---

## Database Schema

Each plugin must define its SQLite table schema in `schema.sql`. The schema is automatically executed when the plugin is registered.

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

### Item Fields

When creating items via the API, these fields are automatically managed:

| Field | Type | Description |
|-------|------|-------------|
| `id` | TEXT | UUID auto-generated if not provided |
| `created_at` | TEXT | ISO 8601 timestamp auto-generated |

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

Use the `saya-api` library to communicate with core:

```javascript
import { SayaApi } from "./saya-api.js";

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

If your plugin declares `has_settings: true` in the manifest, you should provide a settings UI at `ui/settings.html`. This file is loaded into a popup when the user clicks the Settings button in the plugin toolbar.

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
        import { SayaApi } from "./saya-api.js";

        const api = new SayaApi("email");
        api.connect(window.parent);

        async function saveSettings() {
            const provider = document.getElementById("provider").value;
            await api.saveSettings({ provider });
        }
    </script>
</body>
</html>
```

**Settings Behavior:**
- Settings popup appears when the active plugin has `has_settings: true`
- The popup displays the plugin's `display_name` in the header
- Plugins can access settings storage via `api.saveSettings()` and `api.loadSettings()`

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

- **Pagination** — Always use `limit` and `offset` for large datasets
- **Indexing** — Create indexes on frequently filtered columns
- **Lazy loading** — Load data on demand, not all at once

### UX Consistency

- Follow Saya's design tokens (colors, spacing)
- Support keyboard navigation
- Provide loading states for async operations
- Show appropriate empty states

### Error Handling

```javascript
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
        import { SayaApi } from "./saya-api.js";

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

## Getting Help

- **API Reference** — See `Documentation/Working documents/Saya API.md`
- **Implementation Plan** — See `Documentation/Working documents/Implementation Plan.md`
- **Issues** — Report bugs at https://github.com/anomalyco/saya/issues

---

*Last updated: 2026-03-27*
