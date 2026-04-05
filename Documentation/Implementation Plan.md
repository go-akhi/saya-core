# Saya Core App — Implementation Plan

**Status:** Draft  
**Last updated:** 2026-03-27

---

## 1. Vision

Saya is a plugin platform from day one. The core app provides the organizational framework — the 4R cognitive axes, AI integration, user account management, and sync — and every feature the user interacts with is delivered by a plugin.

Email is a plugin. Tasks are a plugin. Notes are a plugin. The core app has no opinion about what the user stores — it only provides the structure to organize it and the intelligence to help with it.

---

## 2. Core App Responsibilities

The core app handles exactly four things:

| Responsibility | Scope |
|---|---|
| **Axes** | 4R cognitive framework (Require, Review, Retain, Relieve) + user-defined context axes (Work, Personal, Finance, etc.) |
| **AI Bridge** | LLM endpoint management, model registry, interface for plugins to request AI operations |
| **Accounts & Auth** | OAuth credential management, token refresh, secure storage |


---

## 3. Plugin System

### 3.1 What Is a Plugin?

A plugin is a self-contained directory providing:
- A data table (SQLite schema)
- A UI (HTML/CSS/JS in iframe)
- Processing logic (Rust module)
- AI action declarations
- Cross-plugin action declarations

### 3.2 Plugin Directory Structure

```
plugins/
└── <plugin_name>/
    ├── manifest.json       # Plugin metadata
    ├── schema.sql          # Table definitions
    └── ui/
        └── index.html      # Plugin UI entry point
```

Plugin processing logic lives in Rust modules under `src/plugins/<plugin_name>/`.

### 3.3 The Manifest

Each plugin declares itself via `manifest.json`:

```json
{
  "name": "email",
  "display_name": "Email",
  "icon": "📧",
  "columns": [
    { "name": "subject", "display": "Subject", "type": "main", "dtype": "text", "sortable": true },
    { "name": "sender", "display": "From", "type": "secondary", "dtype": "text", "sortable": false },
    { "name": "cognitive_axis", "display": "Axis", "type": "filterable", "dtype": "enum", "sortable": true },
    { "name": "context_axis", "display": "Context", "type": "filterable", "dtype": "text", "sortable": false }
  ],
  "ai_actions": [
    {
      "id": "classify",
      "label": "Classify",
      "context_columns": ["subject", "sender", "snippet"],
      "result_mapping": { "cognitive_axis": "cognitive_axis", "context_axis": "context_axis" }
    }
  ],
  "provides_actions": [
    {
      "label": "Push to Notes",
      "target_types": ["email"],
      "handler": "pipeline:push_to_note"
    }
  ]
}
```

### 3.4 Core Plugin Contract

Every plugin must declare columns for:
- `cognitive_axis` — links to the 4R framework
- `context_axis` — links to user context axes

### 3.5 Cross-Plugin Field Mapping

When a plugin provides actions for other plugins' items, it declares `field_mapping` rules referencing the source plugin's columns. This creates a soft dependency — the plugin must know the source schema to integrate with it.

```json
{
  "label": "Create Task",
  "target_types": ["email", "note"],
  "field_mapping": {
    "action_title": "source.title || source.subject",
    "cognitive_axis": "source.cognitive_axis",
    "context_axis": "source.context_axis",
    "source_type": "source_plugin_name",
    "source_id": "source_primary_key"
  }
}
```

**Validation:** At startup, the core validates every declared mapping against `plugin_columns`. If a referenced column doesn't exist (e.g., a plugin renamed a column), the core flags the broken mapping and disables the action.

This is a conscious trade-off: plugins must know each other's schemas to integrate, but the core enforces the contract at load time, not at runtime.

### 3.6 Plugin Validation

When a plugin is installed or loaded, the core runs a validation pass that checks the manifest and UI code against the current plugin registry. Failed plugins are not loaded; the user and developer are shown actionable errors.

**What the core checks:**

| Check | Error message |
|---|---|
| `field_mapping` references a missing column | `"This plugin references a column 'subject' in plugin 'email' which does not exist."` |
| `field_mapping` references a missing plugin | `"This plugin depends on 'calendar' which is not installed."` |
| `target_types` references an unknown plugin | `"This plugin declares actions for 'bookmarks' which is not registered."` |
| Missing `cognitive_axis` or `context_axis` columns | `"This plugin is missing the required 'cognitive_axis' column."` |
| UI code contains `fetch()` or `XMLHttpRequest` with external URLs | `"This plugin connects to the internet directly, which is not allowed. Use the core's AI bridge or sync modules instead."` |
| Manifest syntax errors | `"This plugin's manifest.json is not valid JSON."` |

**How it works:**

1. Core reads the manifest
2. Core statically scans the plugin's `ui/` directory for network call patterns (`fetch(`, `XMLHttpRequest`, `axios`, etc.)
3. Core cross-references `field_mapping` expressions against `plugin_columns` for all referenced plugins
4. Core checks `schema.sql` for required columns (`cognitive_axis`, `context_axis`)
5. If any check fails, the plugin is marked `validation_error` in the `plugins` table with a human-readable reason

The validation error is shown in Settings → Plugin Management so the developer (or user installing a third-party plugin) knows exactly what to fix.

---

## 4. Data Model

### 4.1 Core Tables

#### `plugins`
Registry of installed plugins.

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER | Primary key, autoincrement |
| `name` | TEXT | Unique plugin identifier (e.g. "email") |
| `display_name` | TEXT | Human-readable label |
| `icon` | TEXT | Emoji or icon path |
| `version` | TEXT | Semver string |
| `is_enabled` | BOOLEAN | Whether plugin is active |
| `created_at` | DATETIME | Row creation timestamp |

#### `plugin_columns`
Column metadata for all plugins. Populated at startup from manifests.

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER | Primary key, autoincrement |
| `plugin_name` | TEXT | Which plugin owns this column |
| `name` | TEXT | Actual column name in plugin's table |
| `display` | TEXT | Human-readable label |
| `type` | TEXT | `main`, `secondary`, `filterable`, or `hidden` |
| `dtype` | TEXT | `text`, `datetime`, `integer`, `boolean`, `binary`, `enum` |
| `sortable` | BOOLEAN | Whether the user can sort by this column |

#### `ContextAxis`
User-defined context categories. Seeded with defaults (Work, Personal).

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER | Primary key, autoincrement |
| `name` | TEXT | Unique axis name (e.g. "Work") |
| `icon` | TEXT | Optional emoji |
| `color` | TEXT | Hex color for UI rendering |
| `is_default` | BOOLEAN | True for seed axes |
| `created_at` | DATETIME | Row creation timestamp |

#### `UserAccount`
OAuth credentials and account metadata.

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER | Primary key, autoincrement |
| `provider` | TEXT | Account type (e.g. "gmail", "outlook") |
| `email` | TEXT | Account email address |
| `access_token` | TEXT | Encrypted at rest |
| `refresh_token` | TEXT | Encrypted at rest |
| `token_expiry` | DATETIME | When the access token expires |
| `is_active` | BOOLEAN | Whether account is currently in use |
| `created_at` | DATETIME | Row creation timestamp |
| `updated_at` | DATETIME | Last modification timestamp |

#### `LLMEndpoint`
LLM API configuration. Multiple endpoints can coexist.

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER | Primary key, autoincrement |
| `name` | TEXT | Endpoint label (e.g. "Local Ollama") |
| `provider` | TEXT | Provider type (e.g. "openai", "anthropic", "local") |
| `endpoint_url` | TEXT | Base API URL |
| `api_key` | TEXT | Encrypted at rest |
| `model` | TEXT | Model identifier (e.g. "gpt-4o", "claude-3") |
| `is_default` | BOOLEAN | Whether this is the default endpoint |
| `created_at` | DATETIME | Row creation timestamp |
| `updated_at` | DATETIME | Last modification timestamp |

### 4.2 Plugin Tables

| Plugin | Table | Purpose |
|---|---|---|
| `email` | `EmailRecords` | Email messages with 4R classification |
| `tasks` | `tasks` | Tasks derived from emails or created manually |
| `notes` | `notes` | Promoted notes with source pointers |
| `drafts` | `drafts` | Ghostwriter-generated drafts |

### 4.3 Unified Items View

A SQL view that unions emails, tasks, and notes:

```sql
CREATE VIEW unified_items AS
SELECT 'email' AS source_type, uid AS source_id, cognitive_axis, context_axis, subject AS title FROM EmailRecords
UNION ALL
SELECT 'task', id, cognitive_axis, context_axis, action_title FROM tasks
UNION ALL
SELECT 'note', id, cognitive_axis, context_axis, title FROM notes;
```

---

## 5. UI Architecture

### 5.1 Layout Zones

```
┌─────────────────────────────────────────────────────────────────────┐
│  ☆ SAYA AGENT  │  [💼 Work] [🏠 Personal] [+]  │  [📧] [📋]  [?] [⚙] │
├────┬───────────┼──────────────────────────────────────────────────┤
│    │           │                                                  │
│  ◀▸│    >>    │                   plugin iframe                   │
│    │           │                  (postMessage API)                 │
│ R  │           │                                                  │
│ E  │           │                                                  │
│ Q  │           │                                                  │
│ U  │           │                                                  │
│ I  │           │                                                  │
│ R  │           ├──────────────────────────────────────────────────┤
│ E  │           │                                        ┌───┐    │
│    │           │                                        │ ☆ │    │
│    │           │                                        └───┘    │
│    │           │                                        (dock)    │
└────┴───────────┴──────────────────────────────────────────────────┘
        ↑                              ↑
    Plugin                      Hover-activated
    Sidebar                     Actions Dock
    (collapsible)
```

### 5.2 Top Bar
- Far Left: **Saya Agent** branding button (☆) — DM Sans font, uppercase, hover reveals accent color
- Left: Context axis filter tabs (Chrome-style, rounded corners, colored indicators)
- Add Context Axis button (+) — inline with tabs
- Center: Plugin picker icons
- Right: Help (?) and Settings (⚙)

### 5.3 Left Sidebar
- **Plugin Sidebar** (right of cognitive axis):
  - Collapsible with `<<` / `>>` button
  - Collapse button fades to cognitive axis when collapsed
  - Resizable (drag right edge, 48-320px range)
  - Shows plugin icons with labels when expanded

### 5.4 Cognitive Axis Strip
- Vertical column on far left edge
- 4R buttons with colors: Require (terracotta), Review (amber), Retain (gray), Relieve (light gray)
- Badge counts per category
- Expand button appears at top when plugin sidebar is collapsed

### 5.5 Content Area — Plugin Iframe
- iframe hosting active plugin's UI
- Communication via `postMessage` API
- `saya-api.js` client library wraps protocol

### 5.6 Actions Dock — Right Edge
- **Hover-activated floating dock** — peeks 8px when hidden
- Slides in on mouse hover over right edge
- Frosted glass background with blur effect
- Vertically centered, auto-sized to content
- Shows AI action button (☆) with tooltip from active plugin's manifest
- Plugin-provided actions appear below (from `provides_actions` in manifest)
- Tooltips for each action

---

## 6. Settings

The settings modal is opened by the ⚙ button in the top bar. It is a core-level modal (not a plugin iframe) that provides a tabbed interface for configuring the app. Each tab maps to a core table or feature area.

### 6.1 Settings Tabs

| Tab | Icon | Data Source | Description |
|---|---|---|---|
| **AI Configuration** | 🤖 | `LLMEndpoint` | Add, edit, remove LLM endpoints. Set default model. Test connection. |
| **Accounts** | 👤 | `UserAccount` | Add/remove OAuth accounts (Gmail, Outlook). Re-authenticate expired tokens. Toggle active accounts. |
| **Context Axes** | 🏷️ | `ContextAxis` | Create, rename, reorder, delete context axes. Pick emoji and color. Cannot delete axes that have items assigned. |
| **Plugins** | 🧩 | `plugins` | List installed plugins with version and status. Enable/disable plugins. Show validation errors with actionable messages. |
| **Sync** | 🔄 | (future) | Iroh sync configuration. Device pairing. Sync status. Conflict resolution preferences. |
| **General** | ⚙️ | (local) | Theme (light/dark/auto). Default cognitive axis for new items. Data export. About/version info. |

### 6.2 AI Configuration Tab

Settings for managing LLM endpoints that power the AI Bridge.

| Setting | Type | Source | Description |
|---|---|---|---|
| Endpoint name | Text input | `LLMEndpoint.name` | Label for this endpoint (e.g. "Local Ollama") |
| Provider | Dropdown | `LLMEndpoint.provider` | `openai`, `anthropic`, `local`, `bedrock` |
| API URL | Text input | `LLMEndpoint.endpoint_url` | Base URL for the API |
| API Key | Password input | `LLMEndpoint.api_key` | Stored encrypted at rest |
| Model | Text input | `LLMEndpoint.model` | Model identifier (e.g. "gpt-4o", "llama3") |
| Default | Toggle | `LLMEndpoint.is_default` | Only one endpoint can be default |
| Test Connection | Button | — | Sends a minimal request to verify the endpoint works |
| Remove | Button | — | Deletes the endpoint (confirms if it is the default) |

### 6.3 Accounts Tab

OAuth account management for email and calendar providers.

| Setting | Type | Source | Description |
|---|---|---|---|
| Connected accounts | List | `UserAccount` | Shows provider icon, email, and active/expired status |
| Add account | Button | — | Opens OAuth flow for selected provider |
| Active toggle | Toggle | `UserAccount.is_active` | Enable/disable account without removing credentials |
| Re-authenticate | Button | — | Re-triggers OAuth flow for expired tokens |
| Remove account | Button | — | Deletes credentials and all associated data (confirms) |

### 6.4 Context Axes Tab

User-defined organizational categories that appear in the top bar.

| Setting | Type | Source | Description |
|---|---|---|---|
| Axis list | Drag-to-reorder | `ContextAxis` | Shows emoji, name, color swatch |
| Add axis | Button | — | Inline form: name, emoji picker, color picker |
| Edit axis | Click to edit | — | Rename, change emoji/color |
| Delete axis | Button | — | Blocked if items are assigned to this axis |
| Default axes | Badge | `ContextAxis.is_default` | Seed axes (Work, Personal) are marked; can be renamed but not deleted |

### 6.5 Plugins Tab

Plugin management and diagnostics.

| Setting | Type | Source | Description |
|---|---|---|---|
| Installed plugins | List | `plugins` | Name, version, icon, enabled/disabled status |
| Enable/disable | Toggle | `plugins.is_enabled` | Toggle plugin activation |
| Validation errors | Alert | `plugins.validation_error` | Shows human-readable error with fix instructions |
| Plugin details | Expandable | — | Columns, AI actions, provided actions, schema |

### 6.6 General Tab

App-wide preferences not tied to a specific feature.

| Setting | Type | Source | Description |
|---|---|---|---|
| Theme | Dropdown | Local storage | Light / Dark / System |
| Default cognitive axis | Dropdown | Local storage | Axis assigned to new items by default |
| Export data | Button | — | Exports SQLite database as a downloadable file |
| Version | Read-only | App metadata | Current app version and build hash |

---

## 7. Design Language

| Element | Value |
|---|---|
| Background Primary | `#faf9f7` |
| Background Bar | `#ffffff` |
| Background Sidebar | `#f5f4f1` |
| Background Card | `#ffffff` |
| Background Hover | `#f0efec` |
| Background Badge | `#e8e7e4` |
| Text Primary | `#1a1a1a` |
| Text Secondary | `#6b6a67` |
| Text Muted | `#a09f9c` |
| Borders | `#e5e4e1` |
| Primary accent | `#d97706` (amber) |
| Accent Hover | `#b45309` |
| Typography | **DM Sans** for UI, Inter fallback |
| Branding Font | DM Sans, uppercase, letter-spacing 0.5px |
| Corners | `6px` (default), `10px` (large) |
| Transitions | 150ms default, 200ms for panels, cubic-bezier(0.4, 0, 0.2, 1) for dock |

**Cognitive Axis Colors:**
| Axis | Color |
|---|---|
| Require | `#DC5F3F` (terracotta) |
| Review | `#D97706` (amber) |
| Retain | `#706F6C` (warm gray) |
| Relieve | `#A8A7A3` (light gray) |

**Dock Styling:**
- Frosted glass: `rgba(245, 244, 241, 0.9)` with `blur(12px)`
- Border radius: `12px` corners
- Shadow: `-4px 0 20px rgba(0, 0, 0, 0.08)`
- Icon hover: subtle scale + dot indicator

---

## 8. Implementation Phases

### Phase 1: Project Scaffolding
**Goal:** Empty shell that runs

| Step | Task |
|---|---|
| 1.1 | Initialize Tauri project with Vue + TypeScript |
| 1.2 | Set up Rust project structure (src-tauri/) |
| 1.3 | Create SQLite database module with schema version table |
| 1.4 | Set up logging (tracing for Rust, frontend console) |
| 1.5 | Verify IPC communication (frontend → Tauri commands) |
| 1.6 | Create empty index.html placeholder |

### Phase 2: Core Shell
**Goal:** Functional app shell with layout but no plugins

| Step | Task |
|---|---|
| 2.1 | Implement top bar component |
| 2.2 | Implement cognitive axis column (4R buttons) |
| 2.3 | Create iframe host for plugins |
| 2.4 | Implement actions bar (right edge, hidden by default) |
| 2.5 | Build empty settings modal |
| 2.6 | Wire up basic state management (Pinia store) |
| 2.7 | Apply design language (colors, typography, spacing) |

### Phase 3: Core Database & API
**Goal:** Core tables, plugin discovery, and validation

| Step | Task |
|---|---|
| 3.1 | Create `plugins` table |
| 3.2 | Create `plugin_columns` table |
| 3.3 | Create `ContextAxis` table with seed data |
| 3.4 | Create `UserAccount` table |
| 3.5 | Create `LLMEndpoint` table |
| 3.6 | Build plugin discovery (scan plugins/ directory, read manifests) |
| 3.7 | Build column registry API (register + query columns) |
| 3.8 | Implement manifest validation (JSON structure, required fields) |
| 3.9 | Implement column compatibility checker (field_mapping vs plugin_columns) |
| 3.10 | Implement network isolation scanner (static scan of ui/ for fetch/XMLHttpRequest) |
| 3.11 | Add validation error display in Settings → Plugin Management |

### Phase 4: Saya API Client Library
**Goal:** Standard interface for plugins to communicate with core

| Step | Task |
|---|---|
| 4.1 | Create `saya-api.js` core library |
| 4.2 | Implement postMessage wrapper functions |
| 4.3 | Add type definitions for TypeScript support |
| 4.4 | Build plugin-to-core API (query, mutate, subscribe) |

### Phase 5: AI Bridge
**Goal:** Core provides the button, plugins provide the brain

| Step | Task |
|---|---|
| 6.1 | Implement AI request router in core |
| 6.2 | Create AI action executor (fetch columns, send to LLM, write back) |
| 6.3 | Build individual AI trigger (actions bar) |
| 6.4 | Build batch AI trigger (top bar ☆) |
| 6.5 | Add AI request isolation per plugin |
| 6.6 | Create LLM endpoint configuration UI |

### Phase 6: Iroh Sync
**Goal:** P2P sync between devices using Iroh

| Step | Task |
|---|---|
| 6.1 | Implement Iroh sync transport |
| 6.2 | Build conflict resolution strategy |
| 6.3 | Create sync status UI |

---

## 9. Key Files to Create

### Frontend
```
src/
├── App.vue                              # Root layout
├── main.ts                              # Entry point
├── components/
│   ├── TopBar.vue                       # Top bar with Saya Agent, filter tabs
│   ├── CognitiveAxis.vue                # Left column 4R (vertical)
│   ├── PluginSidebar.vue                # Collapsible plugin sidebar
│   ├── PluginHost.vue                   # iframe host
│   ├── ActionsBar.vue                   # Hover-activated dock
│   ├── AddContextAxis.vue              # Add context axis modal
│   └── SettingsModal.vue                # Settings
├── stores/
│   ├── plugins.ts                       # Plugin registry + manifests store
│   ├── axes.ts                          # Cognitive + context axes
│   └── ui.ts                            # UI state
├── lib/
│   ├── saya-api/                        # Core client library
│   │   ├── index.ts                     # SayaApi class
│   │   ├── types.ts                    # TypeScript interfaces
│   │   └── plugin-context.ts           # Plugin initialization helpers
│   ├── saya-api.ts                      # Re-exports
│   └── core-message-handler.ts          # Core-side message processor
├── __tests__/                           # Test files
└── assets/
```

### Saya API Client Library (src/lib/saya-api/)
```
├── index.ts                             # SayaApi class with postMessage wrapper
│                                       # Methods: query, mutate, subscribe, aiAction
├── types.ts                            # TypeScript interfaces
│                                       # Item, PluginManifest, QueryOptions, etc.
├── plugin-context.ts                   # initPlugin, destroyPlugin helpers
└── saya-api.ts                        # Main export file
```

### Rust Backend (src-tauri/)
```
src-tauri/
├── src/
│   ├── main.rs                          # Tauri entry point
│   ├── lib.rs                           # Tauri commands + plugin API
│   ├── db/
│   │   ├── mod.rs                       # SQLite connection
│   │   ├── schema.sql                   # Core tables
│   │   └── migrate.rs                   # Schema migrations
│   └── plugins/
│       ├── mod.rs                       # Plugin discovery + validation
│       └── registry.rs                   # Plugin registry + CRUD operations
└── Cargo.toml
```

---

## 10. Dependencies

### Frontend (npm)
- `@tauri-apps/api` ^2 — Tauri IPC
- `vue` ^3.5 — UI framework
- `pinia` ^2.2 — State management
- `vite` ^6 — Build tool
- `@vitejs/plugin-vue` — Vue support
- `vitest` — Testing
- `@vue/test-utils` — Vue testing utilities
- `typescript` — TypeScript support

### Rust (Cargo.toml)
- `tauri` ^2 — Desktop app framework
- `tauri-plugin-opener` ^2 — Tauri plugin for opening URLs
- `rusqlite` ^0.31 — SQLite bindings
- `serde` + `serde_json` — Serialization
- `tokio` — Async runtime
- `tracing` + `tracing-subscriber` — Logging
- `dirs` — Platform-specific directories
- `uuid` — UUID generation for items

---

## 11. Verification

After each phase:
1. Run `npm run build` or `npx vue-tsc --noEmit` to verify TypeScript compiles
2. Run `npm run test` to verify all frontend tests pass (vitest)
3. Run `cargo test` to test Rust modules
4. Manual smoke test of new functionality
5. Verify all existing tests still pass

---

## 12. Plugin Marketplace

The marketplace lets users discover and install plugins from a curated registry hosted on GitHub Pages, without leaving the app.

### 12.1 Registry Format

A static JSON file hosted at `<repo>/plugins.json` on GitHub Pages:

```json
{
  "registry_version": "1",
  "updated_at": "2026-03-28T00:00:00Z",
  "signature": "a1b2c3d4e5f6...hex-encoded-ed25519-signature-of-canonical-body...",
  "public_key": "abcdef123456...hex-encoded-ed25519-public-key...",
  "plugins": [
    {
      "name": "email",
      "display_name": "Email",
      "icon": "📧",
      "version": "0.1.0",
      "description": "Gmail and Outlook integration with AI-powered triage and the 4R cognitive framework.",
      "repo_url": "https://github.com/saya-org/plugin-email",
      "verified": true,
      "manifest": {
        "columns": [
          { "name": "subject", "display": "Subject", "type": "main", "dtype": "text", "sortable": true }
        ]
      }
    }
  ]
}
```

**Top-level fields:**

| Field | Required | Description |
|---|---|---|
| `registry_version` | Yes | Schema version for forward compatibility |
| `updated_at` | Yes | ISO 8601 timestamp of last registry update |
| `signature` | Yes | Ed25519 signature of the canonical JSON body (hex-encoded). The signed payload is the JSON with `signature` and `public_key` fields removed. |
| `public_key` | Yes | Ed25519 public key (hex-encoded, 32 bytes). The app verifies the signature against this key before trusting any entry. |
| `plugins` | Yes | Array of plugin entries |

**Plugin entry fields:**

| Field | Required | Description |
|---|---|---|
| `verified` | Yes | `true` if the platform has reviewed and vouches for this plugin. Set by the registry maintainer when merging the PR. |

Each entry includes the plugin's manifest metadata inline so the card view can render without fetching the repo. The `repo_url` is used for README display and zip download.

### 12.2 User Flow

```
User clicks "Add Plugin" (PluginSidebar.vue:138)
        │
        ▼
┌──────────────────────────────┐
│  Plugin Marketplace Modal    │
│                              │
│  ┌────────┐  ┌────────┐     │
│  │ 📧     │  │ ✅     │     │
│  │ Email  │  │ Tasks  │     │
│  │ desc.. │  │ desc.. │     │
│  └────────┘  └────────┘     │
│  ┌────────┐  ┌────────┐     │
│  │ 📝     │  │ 📅     │     │
│  │ Notes  │  │Calendar│     │
│  │ desc.. │  │ desc.. │     │
│  └────────┘  └────────┘     │
└──────────────────────────────┘
        │
    User clicks a card
        │
        ▼
┌──────────────────────────────┐
│  Plugin Detail Modal         │
│  [  Install Plugin  ]  ←──  │
│                              │
│  ┌────────────────────────┐  │
│  │                        │  │
│  │   README rendered as   │  │
│  │   markdown from the    │  │
│  │   GitHub repo          │  │
│  │                        │  │
│  └────────────────────────┘  │
└──────────────────────────────┘
        │
    User clicks "Install Plugin"
        │
        ▼
  Tauri command: install_plugin
    1. Download repo zipball from GitHub API
    2. Extract to plugins/<name>/
    3. Run discover_plugins → register_manifest
    4. Return success → refresh plugin list
```

### 12.3 New Files

#### Frontend

| File | Purpose |
|---|---|
| `src/components/PluginMarketplace.vue` | Modal with plugin cards grid. Fetches registry JSON, renders cards with icon/name/description. |
| `src/components/PluginDetail.vue` | Sub-modal showing README (rendered markdown) and install button. |
| `src/stores/marketplace.ts` | Pinia store: `fetchRegistry()`, `fetchReadme()`, `installPlugin()`. |
| `src/lib/markdown.ts` | Lightweight markdown-to-HTML converter (or use `marked` dependency). |

#### Rust Backend

| File | Purpose |
|---|---|
| `src-tauri/src/plugins/marketplace.rs` | `fetch_registry(url)` — HTTP GET to GitHub Pages URL. `install_plugin(repo_url, plugins_dir)` — download zip, extract. |

### 12.4 Tauri Commands

| Command | Input | Output | Description |
|---|---|---|---|
| `fetch_plugin_registry` | `{ url: string }` | Registry JSON | HTTP GET to GitHub Pages. Returns parsed JSON. |
| `verify_registry` | `{ json: string }` | `{ valid: bool, plugins: PluginEntry[] }` | Strips `signature`/`public_key`, computes Ed25519 verification. Returns entries only if signature is valid. |
| `fetch_plugin_readme` | `{ owner: string, repo: string }` | `{ content: string }` | GET `https://api.github.com/repos/{owner}/{repo}/readme`. Returns decoded markdown. |
| `install_plugin_from_repo` | `{ repo_url: string }` | `{ success: bool }` | Downloads zipball, extracts to plugins dir, runs discovery. |

### 12.5 Dependencies to Add

**Rust (Cargo.toml):**
- `reqwest` with `json` and `stream` features — HTTP client for fetching registry, README, and zipballs
- `zip` — Extract downloaded plugin archives
- `base64` — Decode GitHub API README response
- `ed25519-dalek` — Ed25519 signature verification for the registry
- `hex` — Encode/decode hex strings for signatures and public keys

**Frontend (package.json):**
- `marked` — Markdown rendering for plugin READMEs (4KB gzipped, no dependencies)

### 12.6 Tauri HTTP Allowlist

Update `tauri.conf.json` to allow outbound HTTP:

```json
{
  "app": {
    "security": {
      "csp": null
    }
  },
  "plugins": {
    "shell": {
      "open": true
    }
  }
}
```

Or add `reqwest` directly to Rust and expose commands — preferred since it avoids giving plugins network access.

### 12.7 Registry Hosting (GitHub Pages)

The plugin registry is a static JSON file served from this repo's GitHub Pages site.

#### Directory Layout

```
saya/
├── plugins.json              # The registry (served at root for GitHub Pages)
├── .github/
│   └── workflows/
│       └── pages.yml         # GitHub Actions workflow to deploy Pages
├── public/
│   └── plugins.json          # Dev fallback (copied at build time so local dev works)
├── saya-core/                # The Tauri app
└── Documentation/
```

#### `plugins.json` (registry)

```json
{
  "registry_version": "1",
  "updated_at": "2026-03-28T00:00:00Z",
  "plugins": [
    {
      "name": "email",
      "display_name": "Email",
      "icon": "📧",
      "version": "0.1.0",
      "description": "Gmail and Outlook integration with AI-powered triage and the 4R cognitive framework.",
      "repo_url": "https://github.com/saya-org/plugin-email",
      "manifest": {
        "columns": [
          { "name": "subject", "display": "Subject", "type": "main", "dtype": "text", "sortable": true }
        ]
      }
    }
  ]
}
```

#### GitHub Actions Workflow (`.github/workflows/pages.yml`)

```yaml
name: Deploy Plugin Registry
on:
  push:
    branches: [main]
    paths:
      - 'plugins.json'
  workflow_dispatch:

permissions:
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: true

jobs:
  deploy:
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with:
          path: '.'
          # Only uploads plugins.json since it's at the root
      - id: deployment
        uses: actions/deploy-pages@v4
```

#### GitHub Pages Settings

1. Go to repo Settings → Pages
2. Source: **GitHub Actions**
3. The workflow deploys the repo root, so `plugins.json` is served at:
   ```
   https://<org>.github.io/saya/plugins.json
   ```

#### Development Fallback

For local development without deploying, copy `plugins.json` into the Tauri app's `public/` directory:

```bash
cp plugins.json saya-core/public/plugins.json
```

The app fetches from a configurable URL, defaulting to the production GitHub Pages URL. During development, override via an environment variable or config:

```typescript
// src/stores/marketplace.ts
const REGISTRY_URL = import.meta.env.VITE_REGISTRY_URL
  ?? "https://<org>.github.io/saya/plugins.json";
```

#### Adding a New Plugin to the Registry

1. Create the plugin repo (e.g. `plugin-tasks`)
2. Add an entry to `plugins.json` in this repo
3. Push to `main` — the workflow auto-deploys
4. The app picks up the new plugin on next registry fetch

### 12.8 Edge Cases

| Case | Handling |
|---|---|
| Plugin already installed | Show "Installed" badge instead of "Install" button |
| Network failure fetching registry | Show error state with retry button |
| GitHub API rate limit (60/hr unauthenticated) | Cache README responses in memory; consider adding a `GITHUB_TOKEN` for authenticated requests (15k/hr) |
| Zipball extraction fails | Show error, clean up partial extraction |
| Plugin name collision | Abort install, show conflict message |
| Signature invalid | Show "Registry integrity check failed" error. Do not display any plugins. |
| Public key mismatch | Treat same as invalid signature — possible tampering or wrong registry. |
| `verified: false` plugins | Show with a neutral badge. User can still install, but the app does not claim platform endorsement. |

### 12.9 Signing & Verification

The registry JSON is signed so the app can verify it hasn't been tampered with and can distinguish platform-vouched plugins from community submissions.

#### Why Sign?

- **Integrity** — Proves the registry content wasn't modified after the maintainer published it (e.g., MITM, compromised CDN)
- **Trust signals** — The `verified` field is only meaningful if the registry itself is authenticated
- **User safety** — The app can warn users before installing unverified plugins

#### How It Works

**Signing (one-time setup + on each update):**

1. Generate an Ed25519 keypair:
   ```bash
   openssl genpkey -algorithm Ed25519 -out registry_private.pem
   openssl pkey -in registry_private.pem -pubout -out registry_public.pem
   ```
2. Extract the raw 32-byte public key and embed it as hex in `plugins.json` → `public_key`
3. To sign an updated registry:
   - Remove `signature` and `public_key` fields from the JSON
   - Serialize the remaining JSON with deterministic key ordering (no whitespace variation)
   - Sign the raw bytes with the private key
   - Embed the 64-byte signature as hex in `plugins.json` → `signature`

**Verification (on every app launch / registry fetch):**

1. App fetches `plugins.json` via `fetch_plugin_registry`
2. App calls `verify_registry` which:
   - Strips `signature` and `public_key` from the JSON
   - Re-serializes the remaining JSON identically
   - Verifies the Ed25519 signature against the embedded public key
   - If valid: returns the plugin list with `verified` flags respected
   - If invalid: returns an error, app shows "Registry integrity check failed"

#### Key Management

| Aspect | Detail |
|---|---|
| Algorithm | Ed25519 (fast, small keys, no parameter negotiation) |
| Private key | Stored offline / in GitHub Actions secret (`REGISTRY_PRIVATE_KEY`) |
| Public key | Embedded in `plugins.json` and hardcoded as a fallback in the app binary |
| Key rotation | Update `public_key` in the JSON and re-sign. Old app versions will reject the new key (they update via app update, not registry). |
| Fallback public key | The app ships with a compiled-in public key. If the registry's `public_key` matches, verification proceeds. If it doesn't match (possible rotation), the app warns the user. |

#### Signing Workflow Automation

Extend `.github/workflows/pages.yml` to auto-sign on deploy:

```yaml
- name: Sign registry
  env:
    REGISTRY_PRIVATE_KEY: ${{ secrets.REGISTRY_PRIVATE_KEY }}
  run: |
    # Strip signature fields, sign, inject back
    python3 scripts/sign_registry.py \
      --input plugins.json \
      --key-env REGISTRY_PRIVATE_KEY \
      --output plugins.json
```

A `scripts/sign_registry.py` helper handles the canonical serialization and signing. This keeps the private key out of the repo and ensures every deploy is signed.

#### Verified vs Unverified

```
┌─────────────────────────────────────┐
│  Plugin Marketplace                 │
│                                     │
│  ┌────────┐  ┌────────┐            │
│  │ 📧     │  │ ✅     │            │
│  │ Email  │  │ Tasks  │            │
│  │ ✓ Verified    │ ✓ Verified     │
│  └────────┘  └────────┘            │
│  ┌────────┐  ┌────────┐            │
│  │ 📝     │  │ 📅     │            │
│  │ Notes  │  │Calendar│            │
│  │ Community    │ Community        │
│  └────────┘  └────────┘            │
└─────────────────────────────────────┘
```

| State | UI Treatment | `verified` field | Description |
|---|---|---|---|
| Verified | Checkmark + "Verified" label | `true` | Core team has reviewed; shown first in discovery |
| Community | Neutral badge + "Community" label | `false` | Listed but not reviewed; shown after verified plugins |

**Sort order in discovery:** Verified plugins always appear before unverified plugins. Within each group, plugins are sorted alphabetically.

**How a plugin becomes verified:** There is no request process. The core maintainers proactively review plugins in the registry. When a plugin is deemed valuable, secure, and privacy-focused, the maintainer sets `verified: true` in a subsequent update to `plugins.json`.

Both verified and community plugins are fully installable. The only differences are the badge and sort order — there are no technical restrictions or warnings for community plugins.

---

## 13. Out of Scope (v1)

- P2P sync
- Mobile clients
- PII sanitization (deferred until rust-presidio is ready)
