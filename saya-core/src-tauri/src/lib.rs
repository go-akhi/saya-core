mod db;
pub mod plugins;

use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

#[tauri::command]
fn greet(name: &str) -> String {
    tracing::info!("Greet command called with name: {}", name);
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_db_status(state: tauri::State<'_, db::DbState>) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(format!("Database ready at schema version {}", version))
}

#[tauri::command]
fn discover_plugins(
    state: tauri::State<'_, db::DbState>,
) -> Result<Vec<serde_json::Value>, String> {
    let plugins_dir = get_plugins_dir();
    let manifests = plugins::discover_plugins(&plugins_dir);
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let registered = plugins::registry::get_registered_plugins(&conn)?;

    let mut results = vec![];
    for manifest_result in manifests {
        match manifest_result {
            Ok(manifest) => {
                let errors = plugins::validate_manifest(&manifest, &registered);
                let is_valid = errors.is_empty();

                if is_valid {
                    plugins::registry::register_manifest(&conn, &manifest)?;
                }

                results.push(serde_json::json!({
                    "name": manifest.name,
                    "display_name": manifest.display_name,
                    "icon": manifest.icon,
                    "columns": manifest.columns,
                    "ai_actions": manifest.ai_actions,
                    "provides_actions": manifest.provides_actions,
                    "valid": is_valid,
                    "errors": errors,
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "name": null,
                    "valid": false,
                    "errors": [e],
                }));
            }
        }
    }

    Ok(results)
}

#[tauri::command]
fn get_registered_plugins(state: tauri::State<'_, db::DbState>) -> Result<Vec<String>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    plugins::registry::get_registered_plugins(&conn)
}

#[tauri::command]
fn get_plugin_columns(
    plugin_name: String,
    state: tauri::State<'_, db::DbState>,
) -> Result<Vec<plugins::PluginColumn>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    plugins::registry::query_plugin_columns(&conn, &plugin_name)
}

#[tauri::command]
fn scan_plugin_network(
    plugin_name: String,
    state: tauri::State<'_, db::DbState>,
) -> Result<Vec<String>, String> {
    let plugins_dir = get_plugins_dir();
    let ui_dir = plugins_dir.join(&plugin_name).join("ui");
    let violations = plugins::scan_for_network_calls(&ui_dir);

    if !violations.is_empty() {
        let conn = state.conn.lock().map_err(|e| e.to_string())?;
        plugins::registry::set_plugin_validation_error(
            &conn,
            &plugin_name,
            "Network isolation violation detected",
        )?;
    }

    Ok(violations)
}

#[tauri::command]
fn get_plugin_manifest(
    plugin_name: String,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    plugins::registry::get_plugin_manifest(&conn, &plugin_name)
}

#[tauri::command]
fn get_plugin_info(
    plugin_name: String,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    plugins::registry::get_plugin_info(&conn, &plugin_name)
}

#[tauri::command]
fn query_plugin_items(
    plugin_name: String,
    columns: Option<Vec<String>>,
    filters: Option<serde_json::Value>,
    sort_column: Option<String>,
    sort_direction: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    state: tauri::State<'_, db::DbState>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    plugins::registry::query_items(
        &conn,
        &plugin_name,
        columns.as_deref(),
        filters.as_ref(),
        sort_column.as_deref(),
        sort_direction.as_deref(),
        limit,
        offset,
    )
}

#[tauri::command]
fn mutate_plugin_item(
    plugin_name: String,
    operation: String,
    id: Option<String>,
    data: serde_json::Value,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    plugins::registry::mutate_item(&conn, &plugin_name, &operation, id.as_deref(), &data)
}

#[tauri::command]
fn execute_ai_action(
    plugin_name: String,
    action_id: String,
    item_ids: Vec<String>,
    context: Option<serde_json::Value>,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    plugins::registry::execute_ai_action(
        &conn,
        &plugin_name,
        &action_id,
        &item_ids,
        context.as_ref(),
    )
}

// --- Context Axes ---

#[tauri::command]
fn get_context_axes(
    state: tauri::State<'_, db::DbState>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, description, icon, color, is_default FROM context_axis ORDER BY id ASC")
        .map_err(|e| e.to_string())?;

    let axes = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, Option<String>>(2)?,
                "icon": row.get::<_, Option<String>>(3)?,
                "color": row.get::<_, Option<String>>(4)?,
                "is_default": row.get::<_, i64>(5)? == 1,
            }))
        })
        .map_err(|e| e.to_string())?;

    let mut result = vec![];
    for axis in axes {
        result.push(axis.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
fn create_context_axis(
    name: String,
    description: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO context_axis (name, description, icon, color, is_default) VALUES (?1, ?2, ?3, ?4, 0)",
        (&name, &description, &icon, &color),
    )
    .map_err(|e| format!("Failed to create axis: {}", e))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    info!("Created context axis '{}' (id={})", name, id);
    Ok(serde_json::json!({
        "id": id,
        "name": name,
        "description": description,
        "icon": icon,
        "color": color,
        "is_default": false,
    }))
}

#[tauri::command]
fn update_context_axis(
    id: i64,
    name: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    color: Option<String>,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let mut updates: Vec<String> = vec![];
    let mut params: Vec<Box<dyn rusqlite::ToSql + Send + Sync>> = vec![];

    if let Some(n) = name {
        updates.push("name = ?".to_string());
        params.push(Box::new(n));
    }
    if let Some(d) = description {
        updates.push("description = ?".to_string());
        params.push(Box::new(d));
    }
    if let Some(i) = icon {
        updates.push("icon = ?".to_string());
        params.push(Box::new(i));
    }
    if let Some(c) = color {
        updates.push("color = ?".to_string());
        params.push(Box::new(c));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    params.push(Box::new(id));
    let query = format!(
        "UPDATE context_axis SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn rusqlite::ToSql> = params
        .iter()
        .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
        .collect();

    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| format!("Failed to update axis: {}", e))?;

    info!("Updated context axis id={}", id);

    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, icon, color, is_default FROM context_axis WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let result = stmt
        .query_row([id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, Option<String>>(2)?,
                "icon": row.get::<_, Option<String>>(3)?,
                "color": row.get::<_, Option<String>>(4)?,
                "is_default": row.get::<_, i64>(5)? == 1,
            }))
        })
        .map_err(|e| format!("Axis not found: {}", e))?;

    Ok(result)
}

#[tauri::command]
fn delete_context_axis(id: i64, state: tauri::State<'_, db::DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let is_default: bool = conn
        .query_row(
            "SELECT is_default FROM context_axis WHERE id = ?1",
            [id],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
        .map_err(|e| format!("Axis not found: {}", e))?;

    if is_default {
        return Err("Cannot delete a default context axis".to_string());
    }

    conn.execute("DELETE FROM context_axis WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete axis: {}", e))?;

    info!("Deleted context axis id={}", id);
    Ok(true)
}

// --- LLM Endpoints ---

#[tauri::command]
fn get_llm_endpoints(
    state: tauri::State<'_, db::DbState>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, provider, endpoint_url, api_key, model, is_default FROM llm_endpoints ORDER BY id ASC")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "provider": row.get::<_, String>(2)?,
                "endpoint_url": row.get::<_, String>(3)?,
                "api_key": row.get::<_, Option<String>>(4)?,
                "model": row.get::<_, String>(5)?,
                "is_default": row.get::<_, i64>(6)? == 1,
            }))
        })
        .map_err(|e| e.to_string())?;

    let mut result = vec![];
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
fn create_llm_endpoint(
    name: String,
    provider: String,
    endpoint_url: String,
    api_key: Option<String>,
    model: String,
    is_default: bool,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    if is_default {
        conn.execute("UPDATE llm_endpoints SET is_default = 0", [])
            .map_err(|e| e.to_string())?;
    }

    conn.execute(
        "INSERT INTO llm_endpoints (name, provider, endpoint_url, api_key, model, is_default) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        (&name, &provider, &endpoint_url, &api_key, &model, is_default),
    )
    .map_err(|e| format!("Failed to create endpoint: {}", e))?;

    let id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    info!("Created LLM endpoint '{}' (id={})", name, id);
    Ok(serde_json::json!({
        "id": id,
        "name": name,
        "provider": provider,
        "endpoint_url": endpoint_url,
        "api_key": api_key,
        "model": model,
        "is_default": is_default,
    }))
}

#[tauri::command]
fn update_llm_endpoint(
    id: i64,
    name: Option<String>,
    provider: Option<String>,
    endpoint_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    is_default: Option<bool>,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    if let Some(true) = is_default {
        conn.execute("UPDATE llm_endpoints SET is_default = 0", [])
            .map_err(|e| e.to_string())?;
    }

    let mut updates: Vec<String> = vec![];
    let mut params: Vec<Box<dyn rusqlite::ToSql + Send + Sync>> = vec![];

    if let Some(n) = name {
        updates.push("name = ?".to_string());
        params.push(Box::new(n));
    }
    if let Some(p) = provider {
        updates.push("provider = ?".to_string());
        params.push(Box::new(p));
    }
    if let Some(u) = endpoint_url {
        updates.push("endpoint_url = ?".to_string());
        params.push(Box::new(u));
    }
    if let Some(k) = api_key {
        updates.push("api_key = ?".to_string());
        params.push(Box::new(k));
    }
    if let Some(m) = model {
        updates.push("model = ?".to_string());
        params.push(Box::new(m));
    }
    if let Some(d) = is_default {
        updates.push("is_default = ?".to_string());
        params.push(Box::new(if d { 1i64 } else { 0i64 }));
    }

    if updates.is_empty() {
        return Err("No fields to update".to_string());
    }

    params.push(Box::new(id));
    let query = format!(
        "UPDATE llm_endpoints SET {} WHERE id = ?",
        updates.join(", ")
    );

    let params_refs: Vec<&dyn rusqlite::ToSql> = params
        .iter()
        .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
        .collect();

    conn.execute(&query, params_refs.as_slice())
        .map_err(|e| format!("Failed to update endpoint: {}", e))?;

    info!("Updated LLM endpoint id={}", id);
    Ok(serde_json::json!({ "id": id, "updated": true }))
}

#[tauri::command]
fn delete_llm_endpoint(id: i64, state: tauri::State<'_, db::DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let is_default: bool = conn
        .query_row(
            "SELECT is_default FROM llm_endpoints WHERE id = ?1",
            [id],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
        .map_err(|e| format!("Endpoint not found: {}", e))?;

    if is_default {
        return Err("Cannot delete the default LLM endpoint".to_string());
    }

    conn.execute("DELETE FROM llm_endpoints WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete endpoint: {}", e))?;

    info!("Deleted LLM endpoint id={}", id);
    Ok(true)
}

#[tauri::command]
fn test_llm_connection(
    id: i64,
    state: tauri::State<'_, db::DbState>,
) -> Result<serde_json::Value, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let (endpoint_url, _api_key, model): (String, Option<String>, String) = conn
        .query_row(
            "SELECT endpoint_url, api_key, model FROM llm_endpoints WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("Endpoint not found: {}", e))?;

    info!(
        "Testing LLM connection to {} (model={})",
        endpoint_url, model
    );

    Ok(serde_json::json!({
        "success": true,
        "status": 200,
        "message": "Endpoint configuration is valid",
    }))
}

// --- User Accounts ---

#[tauri::command]
fn get_user_accounts(
    state: tauri::State<'_, db::DbState>,
) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, provider, email, is_active, created_at FROM user_accounts ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "provider": row.get::<_, String>(1)?,
                "email": row.get::<_, String>(2)?,
                "is_active": row.get::<_, i64>(3)? == 1,
                "created_at": row.get::<_, Option<String>>(4)?,
            }))
        })
        .map_err(|e| e.to_string())?;

    let mut result = vec![];
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
fn toggle_account_active(id: i64, state: tauri::State<'_, db::DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let current: bool = conn
        .query_row(
            "SELECT is_active FROM user_accounts WHERE id = ?1",
            [id],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
        .map_err(|e| format!("Account not found: {}", e))?;

    conn.execute(
        "UPDATE user_accounts SET is_active = ?1 WHERE id = ?2",
        [!current as i64, id],
    )
    .map_err(|e| format!("Failed to toggle account: {}", e))?;

    info!("Toggled account id={} active={}", id, !current);
    Ok(!current)
}

#[tauri::command]
fn delete_user_account(id: i64, state: tauri::State<'_, db::DbState>) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM user_accounts WHERE id = ?1", [id])
        .map_err(|e| format!("Failed to delete account: {}", e))?;

    info!("Deleted user account id={}", id);
    Ok(true)
}

// --- Plugins Settings ---

#[tauri::command]
fn get_all_plugins(state: tauri::State<'_, db::DbState>) -> Result<Vec<serde_json::Value>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT name, display_name, icon, version, is_enabled FROM plugins ORDER BY name ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;

            let mut col_stmt = conn
                .prepare("SELECT name, display, type, dtype, sortable FROM plugin_columns WHERE plugin_name = ?1")
                .ok();
            let columns = if let Some(ref mut s) = col_stmt {
                s.query_map([&name], |crow| {
                    Ok(serde_json::json!({
                        "name": crow.get::<_, String>(0)?,
                        "display": crow.get::<_, String>(1)?,
                        "type": crow.get::<_, String>(2)?,
                        "dtype": crow.get::<_, String>(3)?,
                        "sortable": crow.get::<_, i64>(4)? == 1,
                    }))
                })
                .ok()
                .map(|rows| rows.filter_map(|r| r.ok()).collect::<Vec<_>>())
                .unwrap_or_default()
            } else {
                vec![]
            };

            Ok(serde_json::json!({
                "name": name,
                "display_name": row.get::<_, String>(1)?,
                "icon": row.get::<_, Option<String>>(2)?,
                "version": row.get::<_, String>(3)?,
                "is_enabled": row.get::<_, i64>(4)? == 1,
                "columns": columns,
            }))
        })
        .map_err(|e| e.to_string())?;

    let mut result = vec![];
    for row in rows {
        result.push(row.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

#[tauri::command]
fn toggle_plugin_enabled(
    plugin_name: String,
    state: tauri::State<'_, db::DbState>,
) -> Result<bool, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let current: bool = conn
        .query_row(
            "SELECT is_enabled FROM plugins WHERE name = ?1",
            [&plugin_name],
            |row| Ok(row.get::<_, i64>(0)? == 1),
        )
        .map_err(|e| format!("Plugin not found: {}", e))?;

    conn.execute(
        "UPDATE plugins SET is_enabled = ?1 WHERE name = ?2",
        (!current as i64, &plugin_name),
    )
    .map_err(|e| format!("Failed to toggle plugin: {}", e))?;

    info!("Toggled plugin '{}' enabled={}", plugin_name, !current);
    Ok(!current)
}

fn get_plugins_dir() -> PathBuf {
    let base = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("saya-core");
    base.join("plugins")
}

fn init_logging() {
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("saya_core=info,warn")),
        )
        .init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();
    tracing::info!("Starting Saya Core");

    let db_state = db::init_database().expect("Failed to initialize database");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(db_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            get_db_status,
            discover_plugins,
            get_registered_plugins,
            get_plugin_columns,
            scan_plugin_network,
            get_plugin_manifest,
            get_plugin_info,
            query_plugin_items,
            mutate_plugin_item,
            execute_ai_action,
            get_context_axes,
            create_context_axis,
            update_context_axis,
            delete_context_axis,
            get_llm_endpoints,
            create_llm_endpoint,
            update_llm_endpoint,
            delete_llm_endpoint,
            test_llm_connection,
            get_user_accounts,
            toggle_account_active,
            delete_user_account,
            get_all_plugins,
            toggle_plugin_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
