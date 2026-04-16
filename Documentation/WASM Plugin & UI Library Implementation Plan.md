# WASM Plugin & UI Library Implementation Plan

## Table of Contents

- [1. Overview](#1-overview)
- [2. Prerequisite: Fix Existing Plugin System Gaps](#2-prerequisite-fix-existing-plugin-system-gaps)
- [3. Architecture](#3-architecture)
- [4. Phase 1: Core WASM Runtime](#4-phase-1-core-wasm-runtime)
- [5. Phase 2: saya-plugin SDK Crate](#5-phase-2-saya-plugin-sdk-crate)
- [6. Phase 3: Shared UI Component Library](#6-phase-3-shared-ui-component-library)
- [7. Phase 4: saya-ui Rust Crate](#7-phase-4-saya-ui-rust-crate)
- [8. Phase 5: Build Tooling](#8-phase-5-build-tooling)
- [9. Phase 6: Migration & Docs](#9-phase-6-migration--docs)
- [10. Design System Decision](#10-design-system-decision)
- [11. Future: Cross-Plugin Agent (Phase 7)](#11-future-cross-plugin-agent-phase-7)
- [12. Todo List](#12-todo-list)

---

## 1. Overview

### Goals

1. **Rust plugins via WASM as MCP Servers** -- Plugin authors write Rust, compile to `.wasm`, core executes it in a sandboxed wasmtime runtime. These plugins act as Model Context Protocol (MCP) servers, providing tools, resources, and prompts to the central Saya Agent.
2. **Shared UI library** -- A set of web components + CSS served by the core, so all plugins look cohesive without reimplementing UI primitives.
3. **Rust UI crate** -- Typed wrappers around the web components for Rust plugin authors using a frontend framework (Leptos, Dioxus, or Yew).
4. **Mobile Integration** -- Plugins extend the Agent's capabilities which are exposed to the user's mobile device via Iroh Gossip.

### What Changes

| Layer | Today | After |
|-------|-------|-------|
| Plugin logic | Declarative / Some Rust | **MCP Server** (tools, resources, prompts) |
| Core Role | Framework Host | **Agent Host & MCP Client** |
| Plugin UI | Raw HTML/CSS/JS per plugin | Shared web components via `saya-ui` |
| Asset serving | `saya-plugin://` per-plugin only | New `saya-core://` scheme for shared assets |
| Core backend | Limited plugin execution | wasmtime loads and calls MCP-compliant `.wasm` modules |

---

## 2. Prerequisite: Fix Existing Plugin System Gaps

An audit of the current codebase against the documentation revealed several gaps that must be fixed before (or alongside) the WASM work. These affect the existing JS plugin system and would carry forward into WASM plugins if left unaddressed.

### 2.1 CRITICAL: Plugin `schema.sql` Never Executed

**Problem:** Plugins ship a `schema.sql` defining their data table (e.g., `CREATE TABLE chat_items (...)`). The Plugin Development Guide states this is "automatically executed when the plugin is registered." In reality, `discover_plugins()` in `lib.rs` only reads `manifest.json` and registers column metadata -- it never reads or executes `schema.sql`. Plugin data tables are never created.

**Fix:** During plugin discovery/registration, read and execute `schema.sql` from the plugin directory.

File: `saya-core/src-tauri/src/plugins/registry.rs`

Add a new function:

```rust
pub fn execute_plugin_schema(conn: &Connection, plugin_name: &str, plugin_dir: &Path) -> Result<(), String> {
    let schema_path = plugin_dir.join("schema.sql");
    if !schema_path.exists() {
        return Ok(()); // No schema file, nothing to do
    }

    let table_name = safe_table_name(plugin_name)?;

    // Check if table already exists (don't re-run on subsequent discoveries)
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            [&table_name],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to check table existence: {e}"))?;

    if table_exists {
        return Ok(());
    }

    let schema_sql = std::fs::read_to_string(&schema_path)
        .map_err(|e| format!("Failed to read schema.sql for '{}': {e}", plugin_name))?;

    // Validate: schema must only create the expected table name
    let sql_lower = schema_sql.to_lowercase();
    if !sql_lower.contains(&format!("create table {}", table_name.to_lowercase())) {
        return Err(format!(
            "Plugin '{}' schema.sql must create table '{}', but does not contain the expected CREATE TABLE statement",
            plugin_name, table_name
        ));
    }

    conn.execute_batch(&schema_sql)
        .map_err(|e| format!("Failed to execute schema.sql for '{}': {e}", plugin_name))?;

    info!("Executed schema.sql for plugin '{}'", plugin_name);
    Ok(())
}
```

File: `saya-core/src-tauri/src/lib.rs` -- call it from `discover_plugins()`:

```rust
if is_valid {
    plugins::registry::register_manifest(&conn, &manifest)?;
    let plugin_dir = plugins_dir.join(&manifest.name);
    plugins::registry::execute_plugin_schema(&conn, &manifest.name, &plugin_dir)?;
}
```

The marketplace install flow (`marketplace.rs`) should also call this after extracting the plugin.

### 2.2 CRITICAL: Plugin Settings Storage Not Implemented

**Problem:** The API defines `api.saveSettings()` and `api.loadSettings()`. The core message handler accepts `save_settings` and `load_settings` operations. But `mutate_item()` in `registry.rs` only handles `create`, `update`, `delete` -- settings operations fall through to "Unknown operation." There is no `plugin_settings` table in the database.

**Fix:**

1. Add a `plugin_settings` table to the schema:

```sql
CREATE TABLE IF NOT EXISTS plugin_settings (
    plugin_name TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT,
    PRIMARY KEY (plugin_name, key)
);
```

2. Add to `registry.rs`:

```rust
pub fn save_plugin_settings(
    conn: &Connection,
    plugin_name: &str,
    data: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    validate_plugin_name(plugin_name)?;
    let obj = data.as_object().ok_or("Settings must be a JSON object")?;

    for (key, value) in obj {
        conn.execute(
            "INSERT OR REPLACE INTO plugin_settings (plugin_name, key, value) VALUES (?1, ?2, ?3)",
            (plugin_name, key, &serde_json::to_string(value).unwrap_or_default()),
        ).map_err(|e| format!("Failed to save setting '{}': {e}", key))?;
    }
    Ok(serde_json::json!({"saved": true}))
}

pub fn load_plugin_settings(
    conn: &Connection,
    plugin_name: &str,
) -> Result<serde_json::Value, String> {
    validate_plugin_name(plugin_name)?;
    let mut stmt = conn
        .prepare("SELECT key, value FROM plugin_settings WHERE plugin_name = ?1")
        .map_err(|e| format!("Failed to prepare settings query: {e}"))?;

    let mut settings = serde_json::Map::new();
    let rows = stmt.query_map([plugin_name], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| format!("Failed to query settings: {e}"))?;

    for row in rows {
        let (key, value) = row.map_err(|e| format!("Failed to read setting: {e}"))?;
        let parsed = serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value));
        settings.insert(key, parsed);
    }
    Ok(serde_json::Value::Object(settings))
}
```

3. Wire into `mutate_item()` to handle the `save_settings` operation, and add `load_settings` as a query operation in `handleQuery` / `query_items`.

### 2.3 MAJOR: Event Subscriptions Never Fire

**Problem:** `emitToSubscriptions()` exists in `core-message-handler.ts` (line 340) but is never called from anywhere. Plugins can subscribe to `items_changed`, `item_created`, etc., but the events never fire because mutation operations don't trigger emission.

**Fix:** After a successful mutation response in `core-message-handler.ts`, emit events to subscribers:

```typescript
case "mutate": {
    // ... existing validation and handleMutation call ...
    handleMutation(pluginName, message.payload as MutationOptions)
        .then(data => {
            sendResponse({ success: true, data });
            // Emit events to subscribers
            const op = (message.payload as MutationOptions).operation;
            emitToSubscriptions(pluginName, "items_changed", { operation: op, data });
            if (op === "create") emitToSubscriptions(pluginName, "item_created", { item: data });
            if (op === "update") emitToSubscriptions(pluginName, "item_updated", { item: data });
            if (op === "delete") emitToSubscriptions(pluginName, "item_deleted", { item: data });
        })
        .catch(error => sendResponse({ success: false, error: String(error) }));
    break;
}
```

### 2.4 MAJOR: Cross-Plugin Actions Not Implemented

**Problem:** Plugins can declare `provides_actions` in their manifest, and validation checks the references, but there is no code to execute cross-plugin actions. No Tauri command, no handler, no field mapping expression evaluation.

**Status:** This is documented as a feature but has zero runtime implementation. This is a larger feature -- flag it for implementation but don't block WASM work on it. The WASM `on_action` hook is designed to eventually handle this.

### 2.5 MAJOR: `saya://` Protocol Not Implemented

**Problem:** The Plugin Development Guide and Saya API docs reference a `saya://` protocol for whitelisted network requests from plugins. The network isolation scanner checks for `saya://` as an allowlist string. But no `saya://` protocol handler is registered -- only `saya-plugin://` exists.

**Status:** Either implement the protocol or remove references from documentation. The `saya-core://` URI scheme planned in Phase 3 partially addresses this (for shared assets), but the `saya://` protocol was meant for proxied network requests, which is a different feature.

### 2.6 MAJOR: Bedrock LLM Provider Stubbed

**Problem:** `llm.rs` accepts `bedrock` as a provider but returns `Err("Bedrock provider not yet implemented")` at runtime.

**Status:** Either implement or remove from the accepted provider list and document as planned.

### 2.7 MODERATE: Network Isolation Scanner False Positive

**Problem:** The scanner in `mod.rs` allows `fetch()` if the file contains the string `saya://` anywhere -- including in comments. A plugin could write `// saya://` in a comment and use unrestricted `fetch()` elsewhere.

**Fix:** Check that fetch calls specifically use the `saya://` scheme, not just that the string appears somewhere in the file.

### 2.8 MODERATE: AI Classification Prompt Uses Non-Standard Axes

**Problem:** The system prompt in `registry.rs` hardcodes cognitive axes as: require, review, delegate, schedule, call, meeting, delete. The Implementation Plan and Product Overview define the 4R framework: Require, Review, Retain, Relieve. The LLM response parser in `llm.rs` uses keyword matching rather than parsing the documented structured format.

**Fix:** Align the hardcoded axes with the 4R framework (or make them configurable). Parse LLM responses using the documented `cognitive_axis: <value>\ncontext_axis: <value>` format instead of substring matching.

---

## 3. Architecture

### Two WASM Contexts (MCP-Ready)

There are two distinct WASM execution environments. This distinction is critical for the MCP-based architecture:

```
+------------------------------------------------------------------+
|  Tauri Core (Rust) - MCP Host / Agent Host                       |
|                                                                  |
|  +---------------------------+    +--------------------------+   |
|  | wasmtime runtime          |    | Saya Agent               |   |
|  | (MCP Servers)             |    | (MCP Client)             |   |
|  |                           |    |                          |   |
|  | plugin.wasm (MCP)         |    | +----------------------+ |   |
|  |   list_tools()            |<-->| | Iroh Gossip          | |   |
|  |   call_tool()             |    | | (Mobile Gateway)     | |   |
|  |   read_resource()         |    | +----------------------+ |   |
|  +---------------------------+    +--------------------------+   |
|         Backend WASM                        |                    |
|         (wasm32-wasip1)                     | postMessage        |
|         runs in wasmtime                    |                    |
+------------------------------------------------------------------+
                                              |
                                              v
+------------------------------------------------------------------+
|  Plugin iframe (Browser)                                         |
|                                                                  |
|  +---------------------------+    +--------------------------+   |
|  | saya-ui web components    |    | Plugin UI                |   |
|  | <saya-button>             |    |                          |   |
|  | <saya-input>              |    | Option A: Plain HTML/JS  |   |
|  | <saya-card>               |    | Option B: Rust via       |   |
|  | <saya-toast>              |    |   Leptos/Dioxus → WASM   |   |
|  +---------------------------+    +--------------------------+   |
|       Served from saya-core://         Frontend WASM             |
|       Shared across all plugins        (wasm32-unknown-unknown)  |
|                                        runs in browser           |
+------------------------------------------------------------------+
```

**Backend WASM** (`plugin.wasm`):
- Compiled for `wasm32-wasip1`.
- Acts as an **MCP Server** providing tools, resources, and prompts to the host.
- Has access to host-imported functions (query DB, log, etc.).
- Cannot access filesystem, network, or UI directly.

**Frontend WASM** (optional, for Rust-based UIs):
- Compiled for `wasm32-unknown-unknown`.
- Uses `saya-ui` web components for a unified look.
- Communicates with the core Agent via postMessage.
- Communicates with core via postMessage (same as JS plugins)

### Plugin Directory Structure (New Format)

```
chat/
├── manifest.json          # Plugin metadata (unchanged)
├── schema.sql             # Database schema (unchanged)
├── plugin.wasm            # NEW: Backend logic (optional)
└── ui/
    ├── index.html          # Entry point (unchanged)
    ├── saya-api/           # DEPRECATED: replaced by saya-core:// imports
    │   └── index.js
    └── pkg/                # NEW: Rust UI compiled output (optional)
        ├── plugin_ui.js
        ├── plugin_ui_bg.wasm
        └── plugin_ui.d.ts
```

Plugins can be:
1. **Declarative only** (today) -- manifest + schema + HTML/JS UI
2. **Declarative + backend WASM** -- adds `plugin.wasm` for hooks
3. **Full Rust** -- backend WASM + Rust-compiled frontend UI
4. **JS + shared UI** -- HTML/JS using `saya-ui` web components (no Rust needed)

---

## 4. Phase 1: Core WASM Runtime

### 4.1 Add wasmtime Dependency

File: `saya-core/src-tauri/Cargo.toml`

```toml
[dependencies]
wasmtime = "29"
```

wasmtime 29 is the latest stable release. It supports WASI preview 1 (`wasm32-wasip1`) and the component model. Binary size impact: ~5-8 MB to the final app.

### 4.2 Define Host-Guest Interface

The interface between core and plugin WASM uses JSON serialization over linear memory. This avoids the complexity of WIT/component model tooling while remaining practical.

**Memory Protocol:**

1. Plugin WASM exports:
   - `alloc(size: i32) -> i32` -- allocate bytes in WASM memory, return pointer
   - `dealloc(ptr: i32, size: i32)` -- free allocated bytes
   - `on_before_mutate(ptr: i32, len: i32) -> i64` -- hook, returns packed (ptr, len)
   - `on_after_mutate(ptr: i32, len: i32) -> i64` -- hook
   - `on_ai_action(ptr: i32, len: i32) -> i64` -- hook
   - `on_action(ptr: i32, len: i32) -> i64` -- cross-plugin action hook

2. Host provides imports:
   - `host.query_items(ptr: i32, len: i32) -> i64` -- query plugin's DB table
   - `host.log(level: i32, ptr: i32, len: i32)` -- structured logging
   - `host.get_setting(key_ptr: i32, key_len: i32) -> i64` -- read plugin setting
   - `host.complete(ptr: i32, len: i32) -> i64` -- call LLM

Return values use `i64` packed as `(ptr << 32) | len` to return both pointer and length in one value. An error is signaled by setting the high bit of the length.

**JSON Schemas for Exchanged Data:**

Mutation hook input:
```json
{
  "operation": "create",
  "id": "abc-123",
  "data": {
    "title": "New chat",
    "messages": "[]",
    "cognitive_axis": "review"
  }
}
```

Mutation hook output (same shape, plugin may modify fields):
```json
{
  "operation": "create",
  "id": "abc-123",
  "data": {
    "title": "New chat",
    "messages": "[]",
    "cognitive_axis": "task"
  }
}
```

AI action input:
```json
{
  "action_id": "classify",
  "item_ids": ["abc-123", "def-456"],
  "items": [
    {"id": "abc-123", "title": "Bug report", "messages": "[...]"},
    {"id": "def-456", "title": "Feature request", "messages": "[...]"}
  ],
  "context": {}
}
```

AI action output:
```json
{
  "results": [
    {"item_id": "abc-123", "cognitive_axis": "task", "context_axis": "Work"},
    {"item_id": "def-456", "cognitive_axis": "review", "context_axis": "General"}
  ]
}
```

### 4.3 Plugin WASM Loader

Create new file: `saya-core/src-tauri/src/plugins/wasm_runtime.rs`

```rust
use std::path::Path;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use wasmtime::*;

pub struct PluginWasm {
    store: Store<PluginHostState>,
    instance: Instance,
    alloc_fn: TypedFunc<i32, i32>,
    dealloc_fn: TypedFunc<(i32, i32), ()>,
    memory: Memory,
}

struct PluginHostState {
    db: Arc<Mutex<Connection>>,
    plugin_name: String,
    memory: Option<Memory>,
}

impl PluginWasm {
    pub fn load(
        wasm_path: &Path,
        plugin_name: &str,
        db: Arc<Mutex<Connection>>,
    ) -> Result<Self, String> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, wasm_path)
            .map_err(|e| format!("Failed to load WASM module: {e}"))?;

        let mut store = Store::new(&engine, PluginHostState {
            db: db.clone(),
            plugin_name: plugin_name.to_string(),
            memory: None,
        });

        let mut linker = Linker::new(&engine);
        Self::register_host_functions(&mut linker)?;

        let instance = linker.instantiate(&mut store, &module)
            .map_err(|e| format!("Failed to instantiate WASM: {e}"))?;

        let memory = instance.get_memory(&mut store, "memory")
            .ok_or("Plugin WASM must export 'memory'")?;
        store.data_mut().memory = Some(memory);

        let alloc_fn = instance.get_typed_func::<i32, i32>(&mut store, "alloc")
            .map_err(|e| format!("Plugin must export 'alloc': {e}"))?;
        let dealloc_fn = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
            .map_err(|e| format!("Plugin must export 'dealloc': {e}"))?;

        Ok(Self { store, instance, alloc_fn, dealloc_fn, memory })
    }

    fn register_host_functions(linker: &mut Linker<PluginHostState>) -> Result<(), String> {
        // host.log(level, ptr, len)
        linker.func_wrap("host", "log",
            |caller: Caller<'_, PluginHostState>, level: i32, ptr: i32, len: i32| {
                let mem = caller.data().memory.unwrap();
                let data = &mem.data(&caller)[ptr as usize..(ptr + len) as usize];
                let msg = String::from_utf8_lossy(data);
                match level {
                    0 => tracing::debug!("[plugin:{}] {}", caller.data().plugin_name, msg),
                    1 => tracing::info!("[plugin:{}] {}", caller.data().plugin_name, msg),
                    2 => tracing::warn!("[plugin:{}] {}", caller.data().plugin_name, msg),
                    _ => tracing::error!("[plugin:{}] {}", caller.data().plugin_name, msg),
                }
            },
        ).map_err(|e| format!("Failed to register host.log: {e}"))?;

        // host.query_items(ptr, len) -> i64
        linker.func_wrap("host", "query_items",
            |mut caller: Caller<'_, PluginHostState>, ptr: i32, len: i32| -> i64 {
                let mem = caller.data().memory.unwrap();
                let data = &mem.data(&caller)[ptr as usize..(ptr + len) as usize];
                let options_json = String::from_utf8_lossy(data).to_string();
                let plugin_name = caller.data().plugin_name.clone();
                let db = caller.data().db.clone();

                // Execute query using existing registry::query_items
                let result = crate::plugins::registry::query_items_internal(
                    &db.lock().unwrap(), &plugin_name, &options_json
                );

                let result_json = match result {
                    Ok(items) => serde_json::to_string(&items).unwrap_or_default(),
                    Err(e) => format!("{{\"error\":\"{e}\"}}"),
                };

                // Write result back to WASM memory
                Self::write_to_guest_memory(&mut caller, result_json.as_bytes())
            },
        ).map_err(|e| format!("Failed to register host.query_items: {e}"))?;

        // host.complete(ptr, len) -> i64
        // ... similar pattern for LLM completion

        Ok(())
    }

    /// Write bytes into WASM guest memory by calling its alloc function.
    /// Returns packed i64: (ptr << 32) | len
    fn write_to_guest_memory(
        caller: &mut Caller<'_, PluginHostState>,
        data: &[u8],
    ) -> i64 {
        // Implementation calls guest alloc, copies data, returns packed ptr+len
        // ...
        0 // placeholder
    }

    /// Call a plugin hook, passing JSON in, getting JSON out.
    pub fn call_hook(
        &mut self,
        hook_name: &str,
        input_json: &str,
    ) -> Result<String, String> {
        let func = self.instance
            .get_typed_func::<(i32, i32), i64>(&mut self.store, hook_name)
            .map_err(|_| format!("Plugin does not export hook '{hook_name}'"))?;

        // Write input to WASM memory
        let input_bytes = input_json.as_bytes();
        let ptr = self.alloc_fn.call(&mut self.store, input_bytes.len() as i32)
            .map_err(|e| format!("alloc failed: {e}"))?;
        self.memory.write(&mut self.store, ptr as usize, input_bytes)
            .map_err(|e| format!("memory write failed: {e}"))?;

        // Call hook
        let packed = func.call(&mut self.store, (ptr, input_bytes.len() as i32))
            .map_err(|e| format!("Hook '{hook_name}' trapped: {e}"))?;

        // Unpack result pointer and length
        let result_ptr = (packed >> 32) as i32;
        let result_len = (packed & 0xFFFFFFFF) as i32;

        // Check error bit
        if result_len < 0 {
            let err_len = result_len & 0x7FFFFFFF;
            let err_data = &self.memory.data(&self.store)
                [result_ptr as usize..(result_ptr + err_len) as usize];
            return Err(String::from_utf8_lossy(err_data).to_string());
        }

        let result_data = &self.memory.data(&self.store)
            [result_ptr as usize..(result_ptr + result_len) as usize];
        let result = String::from_utf8_lossy(result_data).to_string();

        // Free WASM memory
        self.dealloc_fn.call(&mut self.store, (result_ptr, result_len))
            .map_err(|e| format!("dealloc failed: {e}"))?;

        Ok(result)
    }

    /// Check if a hook is exported by this plugin
    pub fn has_hook(&mut self, hook_name: &str) -> bool {
        self.instance
            .get_typed_func::<(i32, i32), i64>(&mut self.store, hook_name)
            .is_ok()
    }
}
```

### 4.4 Integrate WASM into Plugin Lifecycle

File: `saya-core/src-tauri/src/plugins/mod.rs`

During `discover_plugins()`, after validating the manifest, check for `plugin.wasm`:

```rust
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub wasm: Option<PluginWasm>,
}

pub fn load_plugin(plugin_dir: &Path, db: Arc<Mutex<Connection>>) -> Result<LoadedPlugin, String> {
    let manifest = load_manifest(&plugin_dir.join("manifest.json"))?;
    let wasm_path = plugin_dir.join("plugin.wasm");

    let wasm = if wasm_path.exists() {
        Some(PluginWasm::load(&wasm_path, &manifest.name, db)?)
    } else {
        None
    };

    Ok(LoadedPlugin { manifest, wasm })
}
```

### 4.5 Hook into Existing Mutation & AI Action Paths

File: `saya-core/src-tauri/src/plugins/registry.rs`

Modify `mutate_item()` (currently at line ~399) to call WASM hooks:

```rust
pub fn mutate_item(
    conn: &Connection,
    plugin_name: &str,
    operation: &str,
    id: Option<&str>,
    data: serde_json::Value,
    wasm: Option<&mut PluginWasm>,  // NEW parameter
) -> Result<serde_json::Value, String> {
    let mutation_json = serde_json::json!({
        "operation": operation,
        "id": id,
        "data": data,
    });

    // Call on_before_mutate if plugin has WASM
    let final_mutation = if let Some(wasm) = wasm.as_mut() {
        if wasm.has_hook("on_before_mutate") {
            let result = wasm.call_hook(
                "on_before_mutate",
                &serde_json::to_string(&mutation_json).unwrap(),
            )?;
            serde_json::from_str(&result)
                .map_err(|e| format!("Invalid mutation from plugin: {e}"))?
        } else {
            mutation_json
        }
    } else {
        mutation_json
    };

    // Execute mutation (existing code)
    let result = execute_mutation(conn, plugin_name, &final_mutation)?;

    // Call on_after_mutate if plugin has WASM
    if let Some(wasm) = wasm.as_mut() {
        if wasm.has_hook("on_after_mutate") {
            let after_json = serde_json::json!({
                "mutation": final_mutation,
                "result": result,
            });
            let _ = wasm.call_hook(
                "on_after_mutate",
                &serde_json::to_string(&after_json).unwrap(),
            );
        }
    }

    Ok(result)
}
```

Similarly modify `execute_ai_action()` to delegate to WASM `on_ai_action` hook when present, falling back to the existing LLM-based classification.

### 4.6 WASM State Management

Loaded WASM plugins are stored in Tauri app state alongside the DB:

File: `saya-core/src-tauri/src/lib.rs`

```rust
pub struct PluginWasmState {
    pub plugins: Mutex<HashMap<String, PluginWasm>>,
}

// In setup:
app.manage(PluginWasmState {
    plugins: Mutex::new(HashMap::new()),
});
```

The `discover_plugins` command loads WASM for each plugin that has a `plugin.wasm` file and stores it in this state. Tauri commands that need WASM access extract it from state:

```rust
#[tauri::command]
fn mutate_plugin_item(
    state: tauri::State<'_, db::DbState>,
    wasm_state: tauri::State<'_, PluginWasmState>,
    plugin_name: String,
    operation: String,
    id: Option<String>,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut wasm_plugins = wasm_state.plugins.lock().map_err(|e| e.to_string())?;
    let wasm = wasm_plugins.get_mut(&plugin_name);

    registry::mutate_item(&conn, &plugin_name, &operation, id.as_deref(), data, wasm)
}
```

### 4.7 Hot Reload for WASM

Extend `hot_reload.rs` to detect `plugin.wasm` changes and reload the WASM module:

```rust
// When plugin.wasm changes:
if path.file_name() == Some("plugin.wasm") {
    // Reload WASM module in PluginWasmState
    app_handle.emit("plugin-wasm-reloaded", plugin_name);
}
```

---

## 5. Phase 2: saya-plugin SDK Crate

### 5.1 Crate Structure

Published as `saya-plugin` on crates.io.

```
saya-plugin/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Re-exports, prelude
│   ├── api.rs           # Host function wrappers (query, mutate, log, complete)
│   ├── types.rs         # Item, Mutation, AiAction, etc.
│   ├── memory.rs        # alloc/dealloc exports, memory helpers
│   └── macros.rs        # #[saya_plugin::export] proc macro
└── examples/
    └── minimal/
        └── src/lib.rs
```

### 5.2 Cargo.toml

```toml
[package]
name = "saya-plugin"
version = "0.1.0"
edition = "2021"
description = "SDK for building Saya plugins in Rust"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[features]
default = []
ui = ["saya-ui"]  # Optional: UI component wrappers

[dev-dependencies]
```

### 5.3 Types (`src/types.rs`)

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub plugin_name: String,
    pub cognitive_axis: Option<String>,
    pub context_axis: Option<String>,
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub operation: MutationOp,
    pub id: Option<String>,
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOp {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAction {
    pub action_id: String,
    pub item_ids: Vec<String>,
    pub items: Vec<Item>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiActionResult {
    pub item_id: String,
    pub cognitive_axis: Option<String>,
    pub context_axis: Option<String>,
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOptions {
    pub columns: Option<Vec<String>>,
    pub filters: Option<HashMap<String, String>>,
    pub sort_column: Option<String>,
    pub sort_direction: Option<SortDirection>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub system: Option<String>,
    pub user: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}
```

### 5.4 Host Function Wrappers (`src/api.rs`)

These wrap the raw WASM imports behind ergonomic Rust APIs:

```rust
use crate::types::*;

// Raw host imports (provided by wasmtime)
extern "C" {
    fn host_query_items(ptr: i32, len: i32) -> i64;
    fn host_log(level: i32, ptr: i32, len: i32);
    fn host_complete(ptr: i32, len: i32) -> i64;
    fn host_get_setting(ptr: i32, len: i32) -> i64;
}

/// Query items from this plugin's database table.
pub fn query(options: QueryOptions) -> Result<Vec<Item>, String> {
    let json = serde_json::to_string(&options).map_err(|e| e.to_string())?;
    let result = unsafe { call_host(host_query_items, json.as_bytes()) }?;
    serde_json::from_str(&result).map_err(|e| format!("Failed to parse query result: {e}"))
}

/// Log a message to the core's structured logger.
pub fn log(level: LogLevel, message: &str) {
    let bytes = message.as_bytes();
    unsafe { host_log(level as i32, bytes.as_ptr() as i32, bytes.len() as i32) };
}

/// Call the configured LLM endpoint.
pub fn complete(request: CompletionRequest) -> Result<CompletionResponse, String> {
    let json = serde_json::to_string(&request).map_err(|e| e.to_string())?;
    let result = unsafe { call_host(host_complete, json.as_bytes()) }?;
    serde_json::from_str(&result).map_err(|e| format!("Failed to parse completion: {e}"))
}

/// Read a plugin setting by key.
pub fn get_setting(key: &str) -> Result<Option<String>, String> {
    let result = unsafe { call_host(host_get_setting, key.as_bytes()) }?;
    if result.is_empty() { Ok(None) } else { Ok(Some(result)) }
}

/// Convenience logging macros
#[macro_export]
macro_rules! info { ($($arg:tt)*) => { $crate::api::log($crate::types::LogLevel::Info, &format!($($arg)*)) }; }
#[macro_export]
macro_rules! warn { ($($arg:tt)*) => { $crate::api::log($crate::types::LogLevel::Warn, &format!($($arg)*)) }; }
#[macro_export]
macro_rules! error { ($($arg:tt)*) => { $crate::api::log($crate::types::LogLevel::Error, &format!($($arg)*)) }; }
```

### 5.5 Memory Management (`src/memory.rs`)

```rust
use std::alloc::{alloc, dealloc, Layout};

/// Exported: allocate memory for host to write into
#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { alloc(layout) as i32 }
}

/// Exported: free memory
#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, size: i32) {
    let layout = Layout::from_size_align(size as usize, 1).unwrap();
    unsafe { dealloc(ptr as *mut u8, layout) }
}

/// Pack a pointer and length into i64 for return
pub fn pack_result(ptr: i32, len: i32) -> i64 {
    ((ptr as i64) << 32) | (len as i64 & 0xFFFFFFFF)
}

/// Pack an error result (sets high bit of length)
pub fn pack_error(ptr: i32, len: i32) -> i64 {
    ((ptr as i64) << 32) | ((len | 0x80000000u32 as i32) as i64 & 0xFFFFFFFF)
}

/// Helper: call a host function with bytes, decode result
pub unsafe fn call_host(
    func: unsafe extern "C" fn(i32, i32) -> i64,
    input: &[u8],
) -> Result<String, String> {
    let packed = func(input.as_ptr() as i32, input.len() as i32);
    let ptr = (packed >> 32) as i32;
    let len = (packed & 0xFFFFFFFF) as i32;

    if len < 0 {
        let err_len = len & 0x7FFFFFFF;
        let slice = std::slice::from_raw_parts(ptr as *const u8, err_len as usize);
        Err(String::from_utf8_lossy(slice).to_string())
    } else {
        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        Ok(String::from_utf8_lossy(slice).to_string())
    }
}
```

### 5.6 Plugin Trait & Export Macro (`src/lib.rs`)

```rust
pub mod api;
pub mod types;
pub mod memory;

pub use types::*;

pub mod prelude {
    pub use crate::types::*;
    pub use crate::api::{query, complete, get_setting, log};
    pub use crate::{info, warn, error};
}

/// Trait that plugins implement. All methods are optional with default no-ops.
pub trait Plugin {
    /// Called before a mutation is applied. Return modified mutation or error.
    fn on_before_mutate(&self, mutation: Mutation) -> Result<Mutation, String> {
        Ok(mutation) // pass-through by default
    }

    /// Called after a mutation is applied. For side effects (logging, notifications).
    fn on_after_mutate(&self, mutation: &Mutation, result: &Item) -> Result<(), String> {
        let _ = (mutation, result);
        Ok(())
    }

    /// Custom AI action handler. Return classification results.
    fn on_ai_action(&self, action: AiAction) -> Result<Vec<AiActionResult>, String> {
        let _ = action;
        Err("AI action not implemented".to_string())
    }

    /// Handle a cross-plugin action.
    fn on_action(&self, action: serde_json::Value) -> Result<serde_json::Value, String> {
        let _ = action;
        Err("Action not implemented".to_string())
    }
}
```

The `#[saya_plugin::export]` proc macro generates the WASM exports:

```rust
// Usage:
use saya_plugin::prelude::*;

struct ChatPlugin;

#[saya_plugin::export]
impl Plugin for ChatPlugin {
    fn on_before_mutate(&self, mut mutation: Mutation) -> Result<Mutation, String> {
        // Auto-set cognitive_axis based on content analysis
        if mutation.operation == MutationOp::Create {
            if let Some(title) = mutation.data.get("title") {
                if title.as_str().unwrap_or("").contains("urgent") {
                    mutation.data.insert(
                        "cognitive_axis".to_string(),
                        serde_json::Value::String("require".to_string()),
                    );
                }
            }
        }
        Ok(mutation)
    }

    fn on_ai_action(&self, action: AiAction) -> Result<Vec<AiActionResult>, String> {
        // Custom classification using LLM with plugin-specific prompt
        let mut results = Vec::new();
        for item in &action.items {
            let messages = item.data.get("messages")
                .and_then(|v| v.as_str())
                .unwrap_or("[]");

            let response = saya_plugin::api::complete(CompletionRequest {
                system: Some("Classify this chat by topic and urgency.".to_string()),
                user: format!("Title: {}\nMessages: {}", item.id, messages),
                temperature: Some(0.3),
                max_tokens: Some(256),
            })?;

            results.push(AiActionResult {
                item_id: item.id.clone(),
                cognitive_axis: Some(parse_axis(&response.content)),
                context_axis: Some(parse_context(&response.content)),
                data: Default::default(),
            });
        }
        Ok(results)
    }
}
```

The macro expands to:

```rust
// Generated by #[saya_plugin::export]
static PLUGIN: ChatPlugin = ChatPlugin;

#[no_mangle]
pub extern "C" fn on_before_mutate(ptr: i32, len: i32) -> i64 {
    let input = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let mutation: Mutation = match serde_json::from_slice(input) {
        Ok(m) => m,
        Err(e) => return memory::pack_error_string(&e.to_string()),
    };
    match PLUGIN.on_before_mutate(mutation) {
        Ok(result) => memory::pack_json(&result),
        Err(e) => memory::pack_error_string(&e),
    }
}

#[no_mangle]
pub extern "C" fn on_after_mutate(ptr: i32, len: i32) -> i64 { /* similar */ }

#[no_mangle]
pub extern "C" fn on_ai_action(ptr: i32, len: i32) -> i64 { /* similar */ }

#[no_mangle]
pub extern "C" fn on_action(ptr: i32, len: i32) -> i64 { /* similar */ }
```

### 5.7 Plugin Author Cargo.toml

```toml
[package]
name = "chat-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
saya-plugin = "0.1"
serde_json = "1"

# Build:
# cargo build --target wasm32-wasip1 --release
# cp target/wasm32-wasip1/release/chat_plugin.wasm ../plugin.wasm
```

---

## 6. Phase 3: Shared UI Component Library (Fluent 2)

### 6.1 Design System: Fluent 2

**Decision: Microsoft Fluent 2** via `@fluentui/web-components` (v3, built on FAST).

Rationale:
- Native web components -- works in sandboxed plugin iframes, no framework dependency
- Designed for all-day use (Teams, Outlook) -- eye strain reduction is core to the design
- Built-in light, dark, and high contrast themes via design tokens
- Full theme customization through CSS custom properties -- enterprise customers can brand Saya
- 47+ components covering all plugin UI needs
- Accessibility built in (ARIA, keyboard nav, high contrast) -- required for enterprise sales
- Active Microsoft backing with regular releases

### 6.2 Component Mapping

Plugins use `fluent-*` components directly. No `saya-*` wrapper layer -- this avoids maintenance burden and lets developers use existing Fluent 2 knowledge.

| Plugin Need | Fluent 2 Component | Notes |
|-------------|-------------------|-------|
| Primary/secondary/danger buttons | `<fluent-button appearance="primary\|outline\|subtle">` | Also: `<fluent-toggle-button>`, `<fluent-menu-button>` |
| Text input | `<fluent-text-input>` | Wrapped with `<fluent-field>` for labels |
| Multi-line text | `<fluent-textarea>` | Auto-resize support |
| Dropdown | `<fluent-select>` + `<fluent-option>` | Also: `<fluent-combobox>` for search |
| Checkbox / toggle | `<fluent-checkbox>`, `<fluent-switch>` | |
| Radio group | `<fluent-radio-group>` + `<fluent-radio>` | |
| Dialog / modal | `<fluent-dialog>` + `<fluent-dialog-body>` | |
| Side panel | `<fluent-drawer>` + `<fluent-drawer-body>` | For settings, detail views |
| Notifications | `<fluent-message-bar>` | Intent: success, warning, error, info |
| Tabs | `<fluent-tablist>` + `<fluent-tab>` | |
| Badge / tag | `<fluent-badge>`, `<fluent-counter-badge>` | |
| Avatar | `<fluent-avatar>` | For user/contact display |
| Progress | `<fluent-progress-bar>`, `<fluent-spinner>` | |
| Tooltip | `<fluent-tooltip>` | |
| Menu | `<fluent-menu>` + `<fluent-menu-item>` | Context menus, dropdowns |
| Tree | `<fluent-tree>` + `<fluent-tree-item>` | For hierarchical data |
| Accordion | `<fluent-accordion>` + `<fluent-accordion-item>` | Collapsible sections |
| Divider | `<fluent-divider>` | |
| Text / label | `<fluent-text>`, `<fluent-label>` | Typography |
| Slider | `<fluent-slider>` | For settings like font size |
| Image | `<fluent-image>` | With fit, shadow, border options |

Components NOT in Fluent 2 that plugins may need (implement as Saya additions in `saya-ui.js`):
- `<saya-list>` + `<saya-list-item>` -- Selectable item list with highlight (Fluent has `<fluent-listbox>` but it's for form selects, not content lists)
- `<saya-empty-state>` -- Placeholder for empty views (icon + message + action)
- `<saya-toast>` -- Timed notification toasts (Fluent's `<fluent-message-bar>` is static, not auto-dismissing)
- `<saya-layout>` -- Flex row/column/grid with gap shorthand

### 6.3 Theming Architecture

#### Theme Structure

Each Saya theme is a Fluent 2 base theme + Saya-specific token overrides:

```
Saya Theme = Fluent base theme + brand color overrides + Saya additions
```

#### Built-in Themes

| Theme | Base | Brand Color | Target Use |
|-------|------|-------------|------------|
| Saya Warm Light | `webLightTheme` | `#d97706` (amber) | Default, warm minimal aesthetic |
| Saya Warm Dark | `webDarkTheme` | `#d97706` (amber) | Dark mode with warm accent |
| Saya Neutral Light | `webLightTheme` | `#0078d4` (Fluent blue) | Familiar to Microsoft users |
| Saya Neutral Dark | `webDarkTheme` | `#0078d4` (Fluent blue) | Dark mode, neutral |
| High Contrast | `teamsHighContrastTheme` | System | Accessibility compliance |
| Custom | `createLightTheme()` / `createDarkTheme()` | Configurable | Enterprise branding |

#### Token Mapping: Saya Design → Fluent 2 Tokens

The current Saya design language maps to Fluent tokens as follows:

```javascript
// saya-warm-light.js
import { webLightTheme } from '@fluentui/tokens';

export const sayaWarmLight = {
    ...webLightTheme,

    // Brand color: amber instead of Fluent blue
    colorBrandBackground: '#d97706',
    colorBrandBackgroundHover: '#b45309',
    colorBrandBackgroundPressed: '#92400e',
    colorBrandBackgroundSelected: '#b45309',
    colorBrandForeground1: '#d97706',
    colorBrandForeground2: '#b45309',
    colorBrandForegroundLink: '#d97706',
    colorBrandForegroundLinkHover: '#b45309',
    colorBrandForegroundLinkPressed: '#92400e',
    colorBrandStroke1: '#d97706',

    // Warm neutral backgrounds (instead of Fluent's cool grays)
    colorNeutralBackground1: '#faf9f7',       // --bg-primary
    colorNeutralBackground1Hover: '#f0efec',   // --bg-hover
    colorNeutralBackground2: '#f5f4f1',        // --bg-sidebar
    colorNeutralBackground3: '#e8e7e4',        // --bg-badge
    colorNeutralBackground4: '#ffffff',         // --bg-card

    // Warm neutral foregrounds
    colorNeutralForeground1: '#1a1a1a',        // --text-primary
    colorNeutralForeground2: '#6b6a67',        // --text-secondary
    colorNeutralForeground3: '#a09f9c',        // --text-muted

    // Warm neutral strokes
    colorNeutralStroke1: '#e5e4e1',            // --border

    // Status colors (kept standard for clarity)
    colorStatusDangerBackground1: '#fef2f2',
    colorStatusDangerForeground1: '#dc2626',
    colorStatusWarningBackground1: '#fffbeb',
    colorStatusWarningForeground1: '#d97706',
    colorStatusSuccessBackground1: '#f0fdf4',
    colorStatusSuccessForeground1: '#16a34a',

    // Border radius: slightly softer than Fluent default
    borderRadiusMedium: '6px',                 // --radius
    borderRadiusLarge: '10px',                 // --radius-lg

    // Typography: system fonts (same as current Saya)
    fontFamilyBase: 'system-ui, -apple-system, sans-serif',
};
```

```javascript
// saya-warm-dark.js
import { webDarkTheme } from '@fluentui/tokens';

export const sayaWarmDark = {
    ...webDarkTheme,

    // Same amber brand
    colorBrandBackground: '#d97706',
    colorBrandBackgroundHover: '#f59e0b',
    colorBrandForeground1: '#f59e0b',
    colorBrandForegroundLink: '#f59e0b',

    // Dark warm backgrounds
    colorNeutralBackground1: '#1c1b19',
    colorNeutralBackground2: '#242320',
    colorNeutralBackground3: '#2d2c28',
    colorNeutralBackground4: '#1c1b19',

    // Light warm foregrounds
    colorNeutralForeground1: '#e8e7e4',
    colorNeutralForeground2: '#a09f9c',
    colorNeutralForeground3: '#6b6a67',

    // Warm strokes
    colorNeutralStroke1: '#3d3c38',

    borderRadiusMedium: '6px',
    borderRadiusLarge: '10px',
    fontFamilyBase: 'system-ui, -apple-system, sans-serif',
};
```

#### Enterprise Custom Themes

Enterprise customers configure a custom theme by providing a brand color. The core generates a full theme from it:

```javascript
import { createLightTheme, createDarkTheme } from '@fluentui/tokens';

// Enterprise provides: { brandColor: '#0052CC', companyName: 'Acme Corp' }
function createEnterpriseTheme(brandColor) {
    return {
        light: createLightTheme({ colorBrandBackground: brandColor }),
        dark: createDarkTheme({ colorBrandBackground: brandColor }),
    };
}
```

This is exposed in Settings → Appearance → Custom Theme with a color picker for the brand color. The Fluent `createLightTheme` / `createDarkTheme` factories generate the full token ramp (hover, pressed, selected states) from a single brand color.

#### Theme Propagation to Plugin Iframes

When the user changes theme, it must propagate into every plugin iframe:

1. Core stores the active theme name in app state and localStorage
2. On theme change, core posts a message to all plugin iframes:
   ```javascript
   iframe.contentWindow.postMessage({
       type: 'theme-change',
       source: 'core',
       theme: themeTokens  // full token object
   }, '*');
   ```
3. `saya-ui.js` (loaded in every plugin iframe) listens for this message and calls `setTheme(themeTokens)`
4. On initial load, `saya-ui.js` requests the current theme from the parent:
   ```javascript
   window.parent.postMessage({ type: 'theme-request', source: 'plugin' }, '*');
   ```
5. Core responds with the current theme tokens

This ensures plugins always match the core app's theme, including mid-session changes.

#### CSS Custom Property Bridge

For plugins that don't import `saya-ui.js` (legacy or minimal plugins), Saya also injects a CSS stylesheet with `--saya-*` variables mapped from the active Fluent theme. This maintains backward compatibility:

```css
/* Auto-generated from active Fluent theme, injected into plugin iframes */
:root {
    --saya-bg-primary: var(--colorNeutralBackground1);
    --saya-bg-card: var(--colorNeutralBackground4);
    --saya-bg-hover: var(--colorNeutralBackground1Hover);
    --saya-text-primary: var(--colorNeutralForeground1);
    --saya-text-secondary: var(--colorNeutralForeground2);
    --saya-text-muted: var(--colorNeutralForeground3);
    --saya-border: var(--colorNeutralStroke1);
    --saya-accent: var(--colorBrandBackground);
    --saya-accent-hover: var(--colorBrandBackgroundHover);
    --saya-radius: var(--borderRadiusMedium);
    --saya-radius-lg: var(--borderRadiusLarge);
}
```

### 6.4 Build Pipeline for UI Assets

Fluent 2 components are npm packages that need bundling into a single file for serving via `saya-core://`. This is a **build-time step** in the saya-core project, not a runtime operation.

#### Directory Structure

```
saya-core/
├── src-tauri/
│   └── assets/              # Build output (gitignored, generated)
│       ├── saya-ui.js       # Bundled Fluent components + Saya additions + theme setup
│       ├── saya-ui.css      # Base reset + Saya-specific component styles
│       ├── saya-api.js      # Compiled SayaApi SDK
│       └── themes/
│           ├── warm-light.json
│           ├── warm-dark.json
│           ├── neutral-light.json
│           ├── neutral-dark.json
│           └── high-contrast.json
└── ui-lib/                  # Source for the UI library build
    ├── package.json
    ├── vite.config.ts       # Bundles into assets/
    ├── src/
    │   ├── index.ts         # Imports & registers Fluent components + Saya additions
    │   ├── themes.ts        # Theme definitions (warm, dark, enterprise factory)
    │   ├── theme-bridge.ts  # Listens for theme-change messages, applies themes
    │   └── components/      # Saya-specific additions (not in Fluent)
    │       ├── saya-list.ts
    │       ├── saya-empty-state.ts
    │       ├── saya-toast.ts
    │       └── saya-layout.ts
    └── styles/
        └── saya-ui.css      # Base reset, Saya component styles, --saya-* bridge
```

#### Build Script

```json
// ui-lib/package.json
{
    "scripts": {
        "build": "vite build",
        "watch": "vite build --watch"
    },
    "dependencies": {
        "@fluentui/web-components": "^3.0.0",
        "@fluentui/tokens": "^1.0.0"
    },
    "devDependencies": {
        "vite": "^6"
    }
}
```

```typescript
// ui-lib/vite.config.ts
import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
    build: {
        lib: {
            entry: resolve(__dirname, 'src/index.ts'),
            formats: ['es'],
            fileName: 'saya-ui',
        },
        outDir: '../src-tauri/assets',
        rollupOptions: {
            output: {
                // Single file output for saya-core:// serving
                inlineDynamicImports: true,
            }
        }
    }
});
```

#### Entry Point (`ui-lib/src/index.ts`)

```typescript
// Register only the Fluent components plugins actually need.
// Each import registers the custom element.
import '@fluentui/web-components/button.js';
import '@fluentui/web-components/text-input.js';
import '@fluentui/web-components/textarea.js';
import '@fluentui/web-components/select.js';
import '@fluentui/web-components/option.js';
import '@fluentui/web-components/checkbox.js';
import '@fluentui/web-components/switch.js';
import '@fluentui/web-components/radio-group.js';
import '@fluentui/web-components/radio.js';
import '@fluentui/web-components/dialog.js';
import '@fluentui/web-components/dialog-body.js';
import '@fluentui/web-components/drawer.js';
import '@fluentui/web-components/drawer-body.js';
import '@fluentui/web-components/message-bar.js';
import '@fluentui/web-components/tablist.js';
import '@fluentui/web-components/tab.js';
import '@fluentui/web-components/badge.js';
import '@fluentui/web-components/counter-badge.js';
import '@fluentui/web-components/avatar.js';
import '@fluentui/web-components/progress-bar.js';
import '@fluentui/web-components/spinner.js';
import '@fluentui/web-components/tooltip.js';
import '@fluentui/web-components/menu.js';
import '@fluentui/web-components/menu-item.js';
import '@fluentui/web-components/divider.js';
import '@fluentui/web-components/text.js';
import '@fluentui/web-components/label.js';
import '@fluentui/web-components/field.js';
import '@fluentui/web-components/slider.js';
import '@fluentui/web-components/accordion.js';
import '@fluentui/web-components/accordion-item.js';
import '@fluentui/web-components/image.js';

// Saya-specific components (not in Fluent)
import './components/saya-list.js';
import './components/saya-empty-state.js';
import './components/saya-toast.js';
import './components/saya-layout.js';

// Theme bridge: listens for theme-change messages from core
import './theme-bridge.js';
```

#### Theme Bridge (`ui-lib/src/theme-bridge.ts`)

```typescript
import { setTheme } from '@fluentui/web-components';
import { sayaWarmLight } from './themes';

// Apply default theme immediately
setTheme(sayaWarmLight);

// Listen for theme changes from core
window.addEventListener('message', (event) => {
    const msg = event.data;
    if (msg?.type === 'theme-change' && msg?.source === 'core') {
        setTheme(msg.theme);
    }
});

// Request current theme from core on load
window.parent.postMessage({ type: 'theme-request', source: 'plugin' }, '*');
```

### 6.5 Serving Shared Assets

File: `saya-core/src-tauri/src/lib.rs`

```rust
.register_uri_scheme_protocol("saya-core", |_ctx, request| {
    let uri = request.uri().to_string();
    let path = uri
        .strip_prefix("saya-core://localhost/")
        .or_else(|| uri.strip_prefix("saya-core://localhost"))
        .unwrap_or("");

    // Serve from embedded assets or a known directory
    let content = match path {
        "ui/saya-ui.css" => include_str!("../assets/saya-ui.css"),
        "ui/saya-ui.js" => include_str!("../assets/saya-ui.js"),
        "ui/saya-api.js" => include_str!("../assets/saya-api.js"),
        _ => return not_found_response(),
    };

    let mime = mime_from_extension(
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
    );

    http::Response::builder()
        .status(200)
        .header("Content-Type", mime)
        .header("Access-Control-Allow-Origin", "*")
        .body(content.as_bytes().to_vec())
        .unwrap()
})
```

This also serves `saya-api.js` -- the compiled JavaScript SDK -- so plugins no longer need to copy `saya-api/` into their `ui/` directory.

### 6.3 Plugin Usage

Plugins import from the core URI scheme. They use `fluent-*` components directly:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <link rel="stylesheet" href="saya-core://localhost/ui/saya-ui.css">
    <script type="module" src="saya-core://localhost/ui/saya-ui.js"></script>
    <script type="module" src="saya-core://localhost/ui/saya-api.js"></script>
</head>
<body>
    <div style="display: flex; flex-direction: column; height: 100vh; padding: 16px; gap: 12px;">
        <fluent-text size="400" weight="semibold">My Plugin</fluent-text>

        <saya-list id="itemList">
            <saya-empty-state>No items yet</saya-empty-state>
        </saya-list>

        <div style="display: flex; gap: 8px;">
            <fluent-field style="flex: 1;">
                <fluent-text-input id="input" placeholder="Type here..."></fluent-text-input>
            </fluent-field>
            <fluent-button appearance="primary" id="submitBtn">Send</fluent-button>
        </div>
    </div>

    <script type="module">
        import { SayaApi } from "saya-core://localhost/ui/saya-api.js";
        const api = new SayaApi("my-plugin");
        api.connect(window.parent);

        document.getElementById("submitBtn").addEventListener("click", async () => {
            const input = document.getElementById("input");
            await api.mutate({ operation: "create", data: { title: input.value } });
            input.value = "";
        });
    </script>
</body>
</html>
```

Plugins get theming for free. When the user switches to dark mode in Settings, the core posts a `theme-change` message, `saya-ui.js`'s theme bridge applies the new tokens, and all `fluent-*` components re-render with the new colors. Plugin authors don't write any theme code.

### 6.4 Design Tokens (`saya-ui.css`)

File: `saya-core/src-tauri/assets/saya-ui.css`

These mirror the core app's design language and will be adapted to the chosen design system (see Section 9).

```css
:root {
    /* Colors */
    --saya-bg-primary: #faf9f7;
    --saya-bg-card: #ffffff;
    --saya-bg-hover: #f0efec;
    --saya-bg-input: #ffffff;
    --saya-bg-badge: #e8e7e4;
    --saya-text-primary: #1a1a1a;
    --saya-text-secondary: #6b6a67;
    --saya-text-muted: #a09f9c;
    --saya-text-on-accent: #ffffff;
    --saya-border: #e5e4e1;
    --saya-accent: #d97706;
    --saya-accent-hover: #b45309;
    --saya-error: #dc2626;
    --saya-warning: #d97706;
    --saya-info: #2563eb;
    --saya-success: #16a34a;

    /* Spacing */
    --saya-space-xs: 4px;
    --saya-space-sm: 8px;
    --saya-space-md: 12px;
    --saya-space-lg: 16px;
    --saya-space-xl: 24px;

    /* Radii */
    --saya-radius: 6px;
    --saya-radius-lg: 10px;

    /* Typography */
    --saya-font-sans: system-ui, -apple-system, sans-serif;
    --saya-font-mono: ui-monospace, monospace;
    --saya-text-xs: 11px;
    --saya-text-sm: 13px;
    --saya-text-base: 14px;
    --saya-text-lg: 16px;
    --saya-text-xl: 18px;

    /* Shadows */
    --saya-shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
    --saya-shadow-md: 0 4px 12px rgba(0, 0, 0, 0.1);
    --saya-shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.15);

    /* Transitions */
    --saya-transition-fast: 150ms ease;
    --saya-transition-normal: 200ms ease;
}

/* Base reset applied to all plugin iframes */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
body {
    font-family: var(--saya-font-sans);
    font-size: var(--saya-text-base);
    line-height: 1.5;
    color: var(--saya-text-primary);
    background: var(--saya-bg-primary);
    -webkit-font-smoothing: antialiased;
}
```

### 6.5 Web Components (`saya-ui.js`)

File: `saya-core/src-tauri/assets/saya-ui.js`

Each component is a Custom Element. Below are the core components with their full implementations:

#### Component List

| Component | Tag | Purpose |
|-----------|-----|---------|
| Button | `<saya-button>` | Primary, secondary, danger, ghost variants |
| Input | `<saya-input>` | Text input with label, placeholder, validation |
| TextArea | `<saya-textarea>` | Multi-line text input |
| Select | `<saya-select>` | Dropdown select with options |
| Card | `<saya-card>` | Content container with optional title |
| Toast | `<saya-toast>` | Notification toasts (error, warning, info, success) |
| Modal | `<saya-modal>` | Dialog overlay |
| Badge | `<saya-badge>` | Small label/tag |
| Spinner | `<saya-spinner>` | Loading indicator |
| List | `<saya-list>` | Scrollable item list with selection |
| ListItem | `<saya-list-item>` | Individual list entry |
| Divider | `<saya-divider>` | Horizontal rule |
| Text | `<saya-text>` | Typography (heading, body, caption) |
| Icon | `<saya-icon>` | Icon rendering (emoji or SVG) |
| Layout | `<saya-layout>` | Flex row/column/grid with gap |
| Form | `<saya-form>` | Form container with submission |
| EmptyState | `<saya-empty-state>` | Placeholder for empty lists |

#### Example Implementation: `<saya-button>`

```javascript
class SayaButton extends HTMLElement {
    static get observedAttributes() {
        return ["variant", "size", "disabled", "loading"];
    }

    constructor() {
        super();
        this.attachShadow({ mode: "open" });
    }

    connectedCallback() {
        this.render();
    }

    attributeChangedCallback() {
        this.render();
    }

    render() {
        const variant = this.getAttribute("variant") || "secondary";
        const size = this.getAttribute("size") || "md";
        const disabled = this.hasAttribute("disabled");
        const loading = this.hasAttribute("loading");

        this.shadowRoot.innerHTML = `
            <style>
                :host { display: inline-flex; }
                button {
                    font-family: var(--saya-font-sans);
                    font-size: var(--saya-text-sm);
                    font-weight: 500;
                    border: none;
                    border-radius: var(--saya-radius);
                    cursor: pointer;
                    display: inline-flex;
                    align-items: center;
                    justify-content: center;
                    gap: var(--saya-space-xs);
                    transition: all var(--saya-transition-fast);
                    white-space: nowrap;
                }
                button:disabled { opacity: 0.5; cursor: not-allowed; }

                /* Sizes */
                button.sm { padding: 4px 10px; font-size: var(--saya-text-xs); }
                button.md { padding: 8px 16px; }
                button.lg { padding: 10px 20px; font-size: var(--saya-text-base); }

                /* Variants */
                button.primary {
                    background: var(--saya-accent);
                    color: var(--saya-text-on-accent);
                }
                button.primary:hover:not(:disabled) { background: var(--saya-accent-hover); }

                button.secondary {
                    background: var(--saya-bg-hover);
                    color: var(--saya-text-primary);
                }
                button.secondary:hover:not(:disabled) { background: var(--saya-border); }

                button.danger {
                    background: var(--saya-error);
                    color: white;
                }
                button.danger:hover:not(:disabled) { background: #b91c1c; }

                button.ghost {
                    background: transparent;
                    color: var(--saya-text-secondary);
                }
                button.ghost:hover:not(:disabled) { background: var(--saya-bg-hover); }

                .spinner {
                    width: 14px; height: 14px;
                    border: 2px solid currentColor;
                    border-top-color: transparent;
                    border-radius: 50%;
                    animation: spin 600ms linear infinite;
                }
                @keyframes spin { to { transform: rotate(360deg); } }
            </style>
            <button class="${variant} ${size}" ${disabled || loading ? "disabled" : ""}>
                ${loading ? '<span class="spinner"></span>' : ""}
                <slot></slot>
            </button>
        `;
    }
}

customElements.define("saya-button", SayaButton);
```

#### Example Implementation: `<saya-toast>` (Imperative API)

```javascript
class SayaToastContainer extends HTMLElement {
    constructor() {
        super();
        this.attachShadow({ mode: "open" });
        this.shadowRoot.innerHTML = `
            <style>
                :host {
                    position: fixed;
                    top: var(--saya-space-lg);
                    right: var(--saya-space-lg);
                    z-index: 9999;
                    display: flex;
                    flex-direction: column;
                    gap: var(--saya-space-sm);
                    max-width: 360px;
                    pointer-events: none;
                }
                ::slotted(*) { pointer-events: auto; }
            </style>
            <slot></slot>
        `;
    }

    show(message, type = "error", duration = 4000) {
        const toast = document.createElement("saya-toast-item");
        toast.setAttribute("type", type);
        toast.textContent = message;
        this.appendChild(toast);
        setTimeout(() => toast.dismiss(), duration);
    }
}

customElements.define("saya-toast-container", SayaToastContainer);

// Global helper function for plugins
window.SayaToast = {
    _container: null,
    _getContainer() {
        if (!this._container) {
            this._container = document.createElement("saya-toast-container");
            document.body.appendChild(this._container);
        }
        return this._container;
    },
    error(msg) { this._getContainer().show(msg, "error"); },
    warning(msg) { this._getContainer().show(msg, "warning"); },
    info(msg) { this._getContainer().show(msg, "info"); },
    success(msg) { this._getContainer().show(msg, "success"); },
};
```

### 6.6 Compiled saya-api.js

The existing `SayaApi` class (currently copied per-plugin) is instead served from `saya-core://localhost/ui/saya-api.js`. This is the canonical JavaScript SDK. It is the compiled-to-JS version of the TypeScript source in `saya-core/src/lib/saya-api/index.ts`.

When the core app builds (via `npm run build`), a Vite config or simple script compiles the TypeScript to JavaScript and copies it to `saya-core/src-tauri/assets/saya-api.js`.

---

## 7. Phase 4: saya-ui Rust Crate

### 7.1 Purpose

For plugin authors who write their UI in Rust (using Leptos, Dioxus, or Yew), `saya-ui` provides typed wrappers around the web components so they integrate naturally with the framework's component model.

### 7.2 Crate Structure

```
saya-ui/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── button.rs
│   ├── input.rs
│   ├── card.rs
│   ├── toast.rs
│   ├── layout.rs
│   └── ...
```

### 7.3 Example: Leptos Wrappers

```rust
// saya-ui/src/button.rs
use leptos::*;

#[derive(Clone, Copy, Default)]
pub enum ButtonVariant { Primary, #[default] Secondary, Danger, Ghost }

#[component]
pub fn Button(
    #[prop(default = ButtonVariant::Secondary)] variant: ButtonVariant,
    #[prop(default = false)] disabled: bool,
    #[prop(default = false)] loading: bool,
    #[prop(optional)] on_click: Option<Callback<()>>,
    children: Children,
) -> impl IntoView {
    let variant_str = match variant {
        ButtonVariant::Primary => "primary",
        ButtonVariant::Secondary => "secondary",
        ButtonVariant::Danger => "danger",
        ButtonVariant::Ghost => "ghost",
    };

    view! {
        <saya-button
            variant=variant_str
            disabled=disabled
            loading=loading
            on:click=move |_| { if let Some(cb) = on_click { cb.call(()) } }
        >
            {children()}
        </saya-button>
    }
}
```

### 7.4 Usage in a Rust Plugin UI

```rust
use leptos::*;
use saya_ui::prelude::*;

#[component]
fn ChatView() -> impl IntoView {
    let (input, set_input) = create_signal(String::new());
    let (loading, set_loading) = create_signal(false);

    let send = move |_| {
        set_loading.set(true);
        // ... call api.complete(), api.mutate(), etc.
    };

    view! {
        <Layout direction="column" gap="md" style="height: 100vh">
            <Card>
                <Text slot="title" variant="heading">"Messages"</Text>
                // message list here
            </Card>
            <Layout direction="row" gap="sm">
                <Input
                    placeholder="Type a message..."
                    value=input
                    on_input=move |v| set_input.set(v)
                />
                <Button variant=ButtonVariant::Primary loading=loading on_click=send>
                    "Send"
                </Button>
            </Layout>
        </Layout>
    }
}
```

### 7.5 Cargo.toml

```toml
[package]
name = "saya-ui"
version = "0.1.0"
edition = "2021"
description = "UI components for Saya plugins (web component wrappers)"

[features]
leptos = ["dep:leptos"]
dioxus = ["dep:dioxus"]

[dependencies]
leptos = { version = "0.7", optional = true }
dioxus = { version = "0.6", optional = true }
```

---

## 8. Phase 5: Build Tooling

### 8.1 `saya-cli` (Optional, Quality-of-Life)

A CLI tool for plugin authors to scaffold, build, and package plugins:

```bash
# Scaffold a new plugin
saya-cli new my-plugin --template rust-full
saya-cli new my-plugin --template js-only

# Build WASM backend
saya-cli build

# Package for distribution
saya-cli package

# Install locally for testing
saya-cli dev-install
```

### 8.2 Template: Rust Full Plugin

Generated by `saya-cli new my-plugin --template rust-full`:

```
my-plugin/
├── manifest.json
├── schema.sql
├── plugin-logic/                # Backend WASM
│   ├── Cargo.toml               # depends on saya-plugin
│   └── src/
│       └── lib.rs
├── plugin-ui/                   # Frontend WASM (Leptos)
│   ├── Cargo.toml               # depends on saya-ui
│   ├── src/
│   │   └── lib.rs
│   └── index.html               # trunk entry point
├── build.sh                     # Build script
└── README.md
```

`build.sh`:

```bash
#!/bin/bash
set -e

# Build backend WASM
cd plugin-logic
cargo build --target wasm32-wasip1 --release
cp target/wasm32-wasip1/release/plugin_logic.wasm ../plugin.wasm
cd ..

# Build frontend WASM (using trunk)
cd plugin-ui
trunk build --release --dist ../ui
cd ..

echo "Build complete. Files:"
echo "  plugin.wasm  (backend logic)"
echo "  ui/          (frontend)"
```

### 8.3 Template: JS-Only Plugin (Updated)

```
my-plugin/
├── manifest.json
├── schema.sql
└── ui/
    └── index.html    # imports from saya-core:// (no local saya-api copy)
```

---

## 9. Phase 6: Migration & Docs

### 9.1 Backward Compatibility

All changes are additive. Existing plugins work unchanged:

| Feature | Existing plugins | New plugins |
|---------|-----------------|-------------|
| `plugin.wasm` | Not present, no hooks called | Optional, hooks called when present |
| `ui/saya-api/` | Local copy, still works | Can switch to `saya-core://` import |
| `saya-ui` components | Not used | Available via `saya-core://` |
| Manifest format | Unchanged | Unchanged |
| `schema.sql` | Unchanged | Unchanged |

### 9.2 Migration Path for Existing Plugins

1. **Optional:** Replace local `ui/saya-api/` with import from `saya-core://localhost/ui/saya-api.js`
2. **Optional:** Add `<link rel="stylesheet" href="saya-core://localhost/ui/saya-ui.css">` for design tokens
3. **Optional:** Use `<saya-button>`, `<saya-toast>`, etc. instead of custom HTML
4. **Optional:** Add `plugin.wasm` for backend hooks

None of these steps are required. Plugins can adopt incrementally.

### 9.3 Documentation Updates

- Update `Plugin Development Guide.md`: Add "Rust Plugins" section, update "UI Development" section
- Update `Saya API.md`: Document `saya-core://` imports, web component reference
- Create `saya-plugin` crate documentation (rustdoc)
- Create `saya-ui` component catalog (visual reference of all components)

### 9.4 Marketplace Changes

The marketplace zip download already supports arbitrary files. A `plugin.wasm` file in the zip is automatically installed alongside the manifest and UI. No marketplace changes needed.

The registry `plugins.json` can optionally add a `type` field for filtering:

```json
{
    "name": "chat",
    "type": "rust",
    "version": "v0.2.0"
}
```

---

## 10. Design System Decision

**Decided: Microsoft Fluent 2** (`@fluentui/web-components` v3, built on FAST).

### Why Fluent 2

| Requirement | How Fluent 2 Meets It |
|-------------|----------------------|
| Works in plugin iframes | Native web components, no framework dependency |
| All-day use / eye comfort | Designed for Teams/Outlook -- eye strain reduction is core to the design |
| Dark mode | Built-in `webDarkTheme` with proper token ramps, not CSS hacks |
| High contrast / accessibility | `teamsHighContrastTheme` ships out of the box, ARIA built into every component |
| Enterprise theming | `createLightTheme(brandColor)` generates a full theme from one color -- customers brand Saya without touching code |
| Component coverage | 47+ components covering forms, navigation, dialogs, data display |
| Active maintenance | Microsoft-backed, regular releases, approaching v3 GA |

### What Was Considered and Rejected

| System | Reason for rejection |
|--------|---------------------|
| PatternFly | Red Hat enterprise DNA, too dense for personal productivity, fights Saya's warm aesthetic |
| Carbon | IBM-heavy, overkill component library, enterprise-only flavor |
| Radix | React-only, won't work in sandboxed iframes without a React runtime |
| Preline | Tailwind-based HTML/CSS utilities, not web components, needs build step in every plugin |
| Material Web | Viable alternative, but Fluent 2 has better all-day-use design and enterprise theming story |
| Shoelace | Viable alternative, lighter, but less enterprise polish and no built-in high contrast theme |
| Custom | Full control but massive effort; Fluent 2 gives 90% of what's needed out of the box |

### Theming Details

See Section 6.3 for full theming architecture including:
- Built-in themes (Warm Light, Warm Dark, Neutral Light, Neutral Dark, High Contrast, Custom)
- Fluent token mapping to Saya's current design language
- Enterprise custom theme factory
- Theme propagation to plugin iframes via postMessage
- Backward-compatible `--saya-*` CSS variable bridge

---

## 11. Future: Cross-Plugin Agent (Phase 7)

> This phase is deliberately kept high-level. It depends on Phases 0-2 being complete (cross-plugin actions working, WASM hooks stable, LLM integration solid). Design decisions should be revisited once the foundation is in place.

### Intent

A core-level AI agent that operates across all installed plugins to handle natural-language requests like:
- "Remind me about this email on Friday"
- "Summarize my unread items across all inboxes"
- "Move anything from this sender to Relieve"

The agent composes existing plugin actions -- it does not introduce parallel data paths. Every action the agent takes is something the user could do manually with buttons. AI is a shortcut, not a separate system.

### Why rig

[rig](https://github.com/0xPlaygrounds/rig) is a Rust-native LLM agent framework. It fits Saya because:
- The core is already Rust -- no FFI or subprocess boundary to cross
- rig provides tool-use / function-calling patterns out of the box
- Agent tools map directly to existing Tauri commands (`query_plugin_items`, `mutate_plugin_item`, `execute_ai_action`) and the cross-plugin action system
- Supports multiple LLM providers, aligning with Saya's existing multi-provider LLM endpoint system
- Keeps the agent in-process, so it has direct access to the DB and plugin WASM state

### High-Level Architecture

```
User input (natural language)
    │
    v
┌─────────────────────────────────────────┐
│  Saya Agent (rig)                       │
│                                         │
│  Tools:                                 │
│   - query_items(plugin, filters)        │
│   - mutate_item(plugin, operation, data)│
│   - cross_plugin_action(source, target) │
│   - classify_item(plugin, item_ids)     │
│   - list_plugins()                      │
│                                         │
│  Context:                               │
│   - Installed plugins + their manifests │
│   - Cognitive axis definitions          │
│   - Context axis definitions            │
│   - User preferences (opt-in settings)  │
└─────────────────────────────────────────┘
    │
    v
Existing plugin system (unchanged)
```

The agent's tools are thin wrappers around the same functions that the frontend calls. The agent has no privileged access -- it operates within the same validation, sandboxing, and permission boundaries as manual actions.

### UI Surface

The chat plugin is the natural home for this. It already has a conversation UI and LLM completion support. The agent would be an evolution of the chat plugin's AI capabilities -- from simple completion to multi-step tool use across plugins.

Alternatively, a command palette (Ctrl+K) could provide a lightweight entry point for quick agent commands without opening a full chat.

### Opt-In Design

- Agent features are off by default
- User explicitly enables them in settings
- Every agent action is shown to the user before or as it executes (no silent mutations)
- The system works fully without the agent -- buttons and manual workflows remain the primary interface

### Prerequisites Before Starting

- [ ] Cross-plugin actions fully implemented (Phase 0 gap + WASM `on_action` hook)
- [ ] Plugin settings working (Phase 0)
- [ ] Event subscriptions firing (Phase 0)
- [ ] At least 2-3 plugins actively in use (email, tasks, notes) to make cross-plugin actions meaningful
- [ ] Stable WASM plugin system (Phases 1-2)

### Open Questions (Revisit Later)

- Should the agent run as a core module or as a special privileged plugin?
- How to handle agent actions that span multiple plugins in a single transaction (rollback semantics)?
- How to present agent action plans to the user for approval before execution?
- Should rig's tool definitions be auto-generated from plugin manifests?
- Rate limiting / cost control for LLM calls triggered by the agent

---

## 12. Todo List

> **Read before starting:** Tasks within a phase are ordered. Complete each before moving to the next. Tasks marked with `[SHARED]` create foundations reused by later phases -- implement them with the later phase in mind as described, so you don't rewrite them.

### Phase 0: Fix Existing Plugin System Gaps

> These fix bugs in the current system. Some also lay groundwork reused in Phase 1.

- [ ] `[SHARED]` **Refactor `discover_plugins()` to return plugin directory paths alongside manifests.** Currently it only returns `Vec<Result<PluginManifest, String>>`. Change it to return `Vec<Result<(PluginManifest, PathBuf), String>>` so the caller in `lib.rs` has access to the plugin directory. Phase 1 needs this same path to find `plugin.wasm`. Do this refactor once now. Files: `plugins/mod.rs`, `lib.rs`.
- [ ] **schema.sql execution**: Add `execute_plugin_schema()` to `registry.rs` -- reads `schema.sql`, validates it creates `{plugin_name}_items`, runs it if the table doesn't exist yet. See Section 2.1 for implementation.
- [ ] **schema.sql execution**: Call `execute_plugin_schema()` from `discover_plugins()` in `lib.rs` after `register_manifest()`, using the directory path from the refactored return value.
- [ ] **schema.sql execution**: Call `execute_plugin_schema()` from marketplace install flow in `marketplace.rs` after extracting the plugin.
- [ ] **Plugin settings**: Add `plugin_settings` table to `db/schema.sql` and add a v3 migration in `db/migrate.rs`.
- [ ] `[SHARED]` **Plugin settings**: Implement `save_plugin_settings()` and `load_plugin_settings()` as **standalone public functions** in `registry.rs` (not inside `mutate_item`/`query_items`). Phase 1's WASM `host.get_setting` import will call `load_plugin_settings()` directly. See Section 2.2 for implementation.
- [ ] **Plugin settings**: Wire the `save_settings` and `load_settings` operations. In `core-message-handler.ts`, route `save_settings` mutations to a new Tauri command `save_plugin_settings` and `load_settings` queries to a new Tauri command `load_plugin_settings`. **Do NOT add these operations to `mutate_item()`/`query_items()`** -- keep them as separate commands so the WASM host import can call the Rust functions directly without going through the mutation path. Files: `core-message-handler.ts`, `lib.rs`, `registry.rs`.
- [ ] **Event emission**: Call `emitToSubscriptions()` from the `mutate` case in `core-message-handler.ts` after successful mutations. Emit `items_changed` always, plus the specific event (`item_created`/`item_updated`/`item_deleted`) based on the operation. File: `core-message-handler.ts`.
- [ ] **AI classification prompt**: Align hardcoded cognitive axes in `registry.rs` with the 4R framework (Require, Review, Retain, Relieve) or make them configurable from context_axis table.
- [ ] **AI classification parsing**: Parse structured `cognitive_axis: <value>\ncontext_axis: <value>` format in `llm.rs` instead of loose keyword matching.
- [ ] **Network scanner**: Remove the `saya://` allowlist check from the network scanner in `mod.rs`. Plugins should not be making fetch() calls at all -- the scanner should flag all fetch/XHR/axios usage unconditionally. Remove `saya://` protocol references from documentation. The `saya-core://` URI scheme (Phase 3) serves assets via `<script src>` and `<link>` tags, not fetch().
- [ ] **Bedrock LLM**: Remove `bedrock` from the accepted provider list in `llm.rs` and document it as planned/future. Adding a non-functional provider that errors at runtime is worse than not listing it.
- [ ] **Cross-plugin actions**: No code changes now. Phase 1's `on_action` WASM hook prepares for this. Flag for post-Phase 1 implementation.

### Phase 1: Core WASM Runtime

> Builds on Phase 0. Uses the refactored `discover_plugins()` return value and the standalone settings functions.

- [ ] Add `wasmtime = "29"` to `saya-core/src-tauri/Cargo.toml`
- [ ] Create `saya-core/src-tauri/src/plugins/wasm_runtime.rs` with `PluginWasm` struct
- [ ] Implement memory protocol: `alloc`/`dealloc` handling, packed i64 returns
- [ ] Implement host imports: `host.log`, `host.query_items`, `host.complete`. For `host.get_setting`, call the `load_plugin_settings()` function created in Phase 0 -- do not reimplement settings storage.
- [ ] `host.query_items` needs a JSON-input wrapper: add `query_items_from_json()` in `registry.rs` that parses JSON into the existing `query_items()` parameters. Do NOT duplicate the SQL query logic.
- [ ] Add `PluginWasmState` to Tauri managed state in `lib.rs`
- [ ] Modify `discover_plugins()` in `lib.rs` to load `plugin.wasm` when present, using the directory path already available from Phase 0's refactor.
- [ ] **Modify `mutate_item()` signature** in `registry.rs` to accept `wasm: Option<&mut PluginWasm>`. Add `on_before_mutate` / `on_after_mutate` hook calls. The existing settings routing (Phase 0) is in separate functions and is NOT affected by this change.
- [ ] Modify `execute_ai_action()` in `registry.rs` to delegate to `on_ai_action` hook when present, falling back to existing LLM classification.
- [ ] Update Tauri commands (`mutate_plugin_item`, `execute_ai_action`) to extract WASM state and pass it through.
- [ ] Extend hot reload in `hot_reload.rs` to detect `plugin.wasm` changes and reload the WASM module in `PluginWasmState`.
- [ ] Write integration test: compile a minimal test `.wasm` plugin, load it in core, verify hooks are called and responses round-trip correctly.

### Phase 2: saya-plugin SDK Crate

> This is a new crate in a separate directory. No changes to saya-core. Must match the host-guest interface defined in Phase 1 exactly.

- [ ] Create `saya-plugin/` crate with `Cargo.toml` (target: `wasm32-wasip1`)
- [ ] Implement `src/types.rs`: `Item`, `Mutation`, `AiAction`, `QueryOptions`, etc. These MUST match the JSON schemas that Phase 1's host functions produce/consume.
- [ ] Implement `src/memory.rs`: `alloc`, `dealloc` exports, `pack_result`, `pack_error` -- must match Phase 1's `PluginWasm` memory protocol exactly.
- [ ] Implement `src/api.rs`: `query()`, `complete()`, `log()`, `get_setting()` host function wrappers. The `extern "C"` import names must match Phase 1's `linker.func_wrap("host", ...)` registrations.
- [ ] Implement `Plugin` trait with default no-op methods in `src/lib.rs`
- [ ] Implement `#[saya_plugin::export]` proc macro to generate WASM exports. The export function names (`on_before_mutate`, `on_after_mutate`, `on_ai_action`, `on_action`) must match what Phase 1's `call_hook()` and `has_hook()` look for.
- [ ] Create `examples/minimal/` -- a bare-bones plugin that logs on mutation
- [ ] Create `examples/chat-logic/` -- the chat plugin's AI action in Rust
- [ ] Test: compile example to `wasm32-wasip1`, load in saya-core, verify full round-trip
- [ ] Write crate documentation (rustdoc)

### Phase 3: Shared UI Component Library (Fluent 2)

> Independent of Phases 1-2. Can be done in parallel. No changes to WASM runtime or SDK.

**Build pipeline setup:**
- [ ] Create `saya-core/ui-lib/` directory with `package.json`, `vite.config.ts` (see Section 6.4)
- [ ] `npm install @fluentui/web-components@3 @fluentui/tokens` in `ui-lib/`
- [ ] Create `ui-lib/src/index.ts` -- import and register only the Fluent components listed in Section 6.2 (tree-shake the rest)
- [ ] Configure Vite to bundle into a single ES module output at `src-tauri/assets/saya-ui.js`
- [ ] Add `npm run build` step to the saya-core build process (before `cargo build`)

**Theme definitions:**
- [ ] Create `ui-lib/src/themes.ts` with Saya theme objects: `sayaWarmLight`, `sayaWarmDark`, `sayaNeutralLight`, `sayaNeutralDark` (token overrides from Section 6.3)
- [ ] Create enterprise theme factory: `createSayaTheme(brandColor)` wrapping Fluent's `createLightTheme` / `createDarkTheme`
- [ ] Export theme JSON files to `src-tauri/assets/themes/` for the Rust backend to serve

**Theme propagation:**
- [ ] Create `ui-lib/src/theme-bridge.ts` -- listens for `theme-change` postMessage from core, calls `setTheme()`. Requests current theme on load. See Section 6.3.
- [ ] Add theme message handling to `PluginHost.vue` -- when theme changes, post `theme-change` message to all plugin iframes
- [ ] Add theme setting to the Settings modal (dropdown: Warm Light, Warm Dark, Neutral Light, Neutral Dark, High Contrast, Custom)
- [ ] Store active theme in localStorage and Tauri app state
- [ ] Create `--saya-*` CSS variable bridge stylesheet (Section 6.3) for backward compatibility with legacy plugins

**Saya-specific components (not in Fluent):**
- [ ] Implement `<saya-list>` + `<saya-list-item>` -- selectable item list with highlight, using Fluent tokens for styling
- [ ] Implement `<saya-empty-state>` -- icon + message + optional action button
- [ ] Implement `<saya-toast>` -- auto-dismissing notification using Fluent status colors
- [ ] Implement `<saya-layout>` -- flex row/column/grid with gap shorthand

**Asset serving:**
- [ ] Register `saya-core://` URI scheme in `lib.rs` via `include_bytes!` / `include_str!`. Serve `saya-ui.js`, `saya-ui.css`, `saya-api.js`, and theme JSON files. This is a NEW protocol handler alongside `saya-plugin://`.
- [ ] Compile `saya-api.js` from TypeScript source. Add to Vite build or as a separate script. Output to `assets/saya-api.js`.
- [ ] Write `assets/saya-ui.css`: base reset + `--saya-*` bridge variables + Saya-specific component styles
- [ ] Update iframe `sandbox` attribute in `PluginHost.vue` if needed for cross-origin `saya-core://` access

**Verification:**
- [ ] Test: create a minimal JS plugin using `fluent-button`, `fluent-text-input`, `fluent-dialog` via `saya-core://` imports
- [ ] Test: verify theme switching (light → dark → high contrast) propagates into plugin iframe in real time
- [ ] Test: verify `--saya-*` bridge variables update when theme changes (backward compat for legacy plugins)

### Phase 4: saya-ui Rust Crate

> Depends on Phase 3 (Fluent components must be bundled and serving). Does NOT depend on Phases 1-2.

- [ ] Create `saya-ui/` crate with `Cargo.toml` (feature-gated: leptos, dioxus)
- [ ] Implement Leptos wrappers for Fluent components: `fluent-button`, `fluent-text-input`, `fluent-textarea`, `fluent-select`, `fluent-checkbox`, `fluent-switch`, `fluent-dialog`, `fluent-drawer`, `fluent-message-bar`, `fluent-tablist`, `fluent-badge`, `fluent-avatar`, `fluent-spinner`, `fluent-menu`, `fluent-accordion`, `fluent-field`, `fluent-divider`, `fluent-text`, `fluent-slider`, `fluent-image`
- [ ] Implement Leptos wrappers for Saya additions: `saya-list`, `saya-empty-state`, `saya-toast`, `saya-layout`
- [ ] Create example: full chat plugin UI in Leptos using `saya-ui` components with `fluent-*` elements
- [ ] Test: compile example with trunk, serve in iframe, verify components render, theme applies, events work
- [ ] Write crate documentation (rustdoc)

### Phase 5: Build Tooling

> Depends on Phases 1-4 being stable. Templates reference the SDK crate, UI crate, and build targets.

- [ ] Create `saya-cli/` crate
- [ ] Implement `saya-cli new` with templates: `rust-full`, `rust-logic-only`, `js-only`. Templates must use the final crate names and versions from Phases 2 and 4.
- [ ] Implement `saya-cli build` (compiles backend WASM via `cargo build --target wasm32-wasip1` + frontend via trunk)
- [ ] Implement `saya-cli dev-install` (symlinks plugin directory to `~/.local/share/saya-core/plugins/`)
- [ ] Implement `saya-cli package` (creates distributable zip matching marketplace format)
- [ ] Test: scaffold → build → install → verify end-to-end

### Phase 6: Migration & Documentation

> Final phase. All features are stable.

- [ ] Update `Plugin Development Guide.md`: add Rust plugin section, update UI section to reference `saya-core://` imports and web components.
- [ ] Update `Saya API.md`: document `saya-core://` scheme, web component reference, WASM host functions.
- [ ] Migrate plugin-chat to use `saya-core://` imports -- remove local `ui/saya-api/` directory (the hand-written `.js` files from the earlier stopgap fix).
- [ ] Optionally migrate plugin-chat UI to use `<saya-*>` web components.
- [ ] Create component visual catalog (HTML page showing all components with live examples).
- [ ] Update `plugins.json` schema to include optional `type` field for Rust vs JS plugins.
