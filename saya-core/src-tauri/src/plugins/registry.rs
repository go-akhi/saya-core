use crate::llm::{self, LlmClient, LlmEndpoint};
use crate::plugins::{PluginColumn, PluginManifest};
use rusqlite::Connection;
use serde_json::Value;
use tracing::info;

fn json_value_to_sql_param(v: &Value) -> Box<dyn rusqlite::ToSql + Send + Sync> {
    match v {
        Value::Null => Box::new(rusqlite::types::Null),
        Value::Bool(b) => Box::new(if *b { 1i64 } else { 0i64 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else if let Some(f) = n.as_f64() {
                Box::new(f)
            } else {
                Box::new(n.to_string())
            }
        }
        Value::String(s) => Box::new(s.clone()),
        Value::Array(arr) => Box::new(serde_json::to_string(arr).unwrap_or_default()),
        Value::Object(obj) => Box::new(serde_json::to_string(obj).unwrap_or_default()),
    }
}

// SQL injection prevention: validate and sanitize identifiers
fn validate_plugin_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("Plugin name must be 1-64 characters".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(
            "Plugin name must contain only lowercase letters, digits, and hyphens".to_string(),
        );
    }
    Ok(())
}

fn safe_table_name(plugin_name: &str) -> Result<String, String> {
    validate_plugin_name(plugin_name)?;
    let table_name = format!("{}_items", plugin_name.replace('-', "_"));
    // Double-check the result is safe (only alphanumeric + underscore)
    if !table_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Generated table name contains invalid characters".to_string());
    }
    Ok(table_name)
}

fn validate_column_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 128 {
        return Err("Column name must be 1-128 characters".to_string());
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Column name must contain only letters, digits, and underscores".to_string());
    }
    Ok(())
}

fn validate_sort_direction(dir: &str) -> Result<&'static str, String> {
    match dir.to_lowercase().as_str() {
        "asc" => Ok("ASC"),
        "desc" => Ok("DESC"),
        _ => Err("Sort direction must be 'asc' or 'desc'".to_string()),
    }
}

fn validate_item_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 128 {
        return Err("Item ID must be 1-128 characters".to_string());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(
            "Item ID must contain only letters, digits, hyphens, and underscores".to_string(),
        );
    }
    Ok(())
}

fn safe_join_columns(columns: &[String]) -> Result<String, String> {
    let mut validated = Vec::with_capacity(columns.len());
    for col in columns {
        validate_column_name(col)?;
        validated.push(col.clone());
    }
    Ok(validated.join(", "))
}

pub fn register_plugin_columns(
    conn: &Connection,
    plugin_name: &str,
    columns: &[PluginColumn],
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM plugin_columns WHERE plugin_name = ?1",
        [plugin_name],
    )
    .map_err(|e| format!("Failed to clear old columns: {}", e))?;

    for col in columns {
        conn.execute(
            "INSERT INTO plugin_columns (plugin_name, name, display, type, dtype, sortable)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                plugin_name,
                &col.name,
                &col.display,
                &col.col_type,
                &col.dtype,
                col.sortable,
            ),
        )
        .map_err(|e| format!("Failed to insert column '{}': {}", col.name, e))?;
    }

    info!(
        "Registered {} columns for plugin '{}'",
        columns.len(),
        plugin_name
    );
    Ok(())
}

pub fn query_plugin_columns(
    conn: &Connection,
    plugin_name: &str,
) -> Result<Vec<PluginColumn>, String> {
    let mut stmt = conn
        .prepare("SELECT name, display, type, dtype, sortable FROM plugin_columns WHERE plugin_name = ?1")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let columns = stmt
        .query_map([plugin_name], |row| {
            Ok(PluginColumn {
                name: row.get(0)?,
                display: row.get(1)?,
                col_type: row.get(2)?,
                dtype: row.get(3)?,
                sortable: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query columns: {}", e))?;

    let mut result = vec![];
    for col in columns {
        result.push(col.map_err(|e| format!("Failed to read column: {}", e))?);
    }
    Ok(result)
}

pub fn register_manifest(conn: &Connection, manifest: &PluginManifest) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO plugins (name, display_name, icon, version, is_enabled)
         VALUES (?1, ?2, ?3, ?4, 1)",
        (
            &manifest.name,
            &manifest.display_name,
            &manifest.icon,
            "0.1.0",
        ),
    )
    .map_err(|e| format!("Failed to register plugin '{}': {}", manifest.name, e))?;

    register_plugin_columns(conn, &manifest.name, &manifest.columns)?;

    info!("Registered plugin '{}'", manifest.name);
    Ok(())
}

pub fn get_registered_plugins(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT name FROM plugins WHERE is_enabled = 1")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let names = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Failed to query plugins: {}", e))?;

    let mut result = vec![];
    for name in names {
        result.push(name.map_err(|e| format!("Failed to read plugin: {}", e))?);
    }
    Ok(result)
}

pub fn set_plugin_validation_error(
    conn: &Connection,
    plugin_name: &str,
    error: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE plugins SET is_enabled = 0 WHERE name = ?1",
        [plugin_name],
    )
    .map_err(|e| format!("Failed to disable plugin: {}", e))?;

    info!(
        "Plugin '{}' disabled due to validation error: {}",
        plugin_name, error
    );
    Ok(())
}

pub fn uninstall_plugin(conn: &Connection, plugin_name: &str) -> Result<(), String> {
    let table_name = safe_table_name(plugin_name)?;

    conn.execute(&format!("DROP TABLE IF EXISTS {}", table_name), [])
        .map_err(|e| format!("Failed to drop items table: {}", e))?;

    conn.execute(
        "DELETE FROM plugin_columns WHERE plugin_name = ?1",
        [plugin_name],
    )
    .map_err(|e| format!("Failed to delete plugin columns: {}", e))?;

    conn.execute("DELETE FROM plugins WHERE name = ?1", [plugin_name])
        .map_err(|e| format!("Failed to delete plugin: {}", e))?;

    info!("Uninstalled plugin '{}' from database", plugin_name);
    Ok(())
}

pub fn get_plugin_manifest(conn: &Connection, plugin_name: &str) -> Result<Value, String> {
    let mut stmt = conn
        .prepare(
            "SELECT display_name, icon, version FROM plugins WHERE name = ?1 AND is_enabled = 1",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let result = stmt
        .query_row([plugin_name], |row| {
            Ok(serde_json::json!({
                "display_name": row.get::<_, String>(0)?,
                "icon": row.get::<_, Option<String>>(1)?,
                "version": row.get::<_, String>(2)?,
            }))
        })
        .map_err(|e| format!("Plugin '{}' not found: {}", plugin_name, e))?;

    Ok(result)
}

pub fn get_plugin_info(conn: &Connection, plugin_name: &str) -> Result<Value, String> {
    let mut stmt = conn
        .prepare(
            "SELECT p.display_name, p.icon, p.version, p.is_enabled,
                    GROUP_CONCAT(pc.name || ':' || pc.display || ':' || pc.type || ':' || pc.dtype, ';') as columns
             FROM plugins p
             LEFT JOIN plugin_columns pc ON p.name = pc.plugin_name
             WHERE p.name = ?1
             GROUP BY p.name"
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let result = stmt
        .query_row([plugin_name], |row| {
            let columns_str: Option<String> = row.get(4)?;
            let columns: Vec<Value> = columns_str
                .map(|s| {
                    s.split(';')
                        .filter_map(|col| {
                            let parts: Vec<&str> = col.split(':').collect();
                            if parts.len() == 4 {
                                Some(serde_json::json!({
                                    "name": parts[0],
                                    "display": parts[1],
                                    "type": parts[2],
                                    "dtype": parts[3],
                                }))
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            Ok(serde_json::json!({
                "name": plugin_name,
                "display_name": row.get::<_, String>(0)?,
                "icon": row.get::<_, Option<String>>(1)?,
                "version": row.get::<_, String>(2)?,
                "is_enabled": row.get::<_, i32>(3)? == 1,
                "columns": columns,
            }))
        })
        .map_err(|e| format!("Plugin '{}' not found: {}", plugin_name, e))?;

    Ok(result)
}

pub fn query_items(
    conn: &Connection,
    plugin_name: &str,
    columns: Option<&[String]>,
    filters: Option<&Value>,
    sort_column: Option<&str>,
    sort_direction: Option<&str>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<Value>, String> {
    let table_name = safe_table_name(plugin_name)?;

    let cols = if let Some(c) = columns {
        safe_join_columns(c)?
    } else {
        "*".to_string()
    };

    let mut query = format!("SELECT {} FROM {}", cols, table_name);
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(f) = filters {
        if let Some(obj) = f.as_object() {
            let mut conditions: Vec<String> = vec![];
            for (k, v) in obj.iter() {
                validate_column_name(k)?;
                params.push(json_value_to_sql_param(v));
                conditions.push(format!("{} = ?", k));
            }
            if !conditions.is_empty() {
                query.push_str(" WHERE ");
                query.push_str(&conditions.join(" AND "));
            }
        }
    }

    if let Some(col) = sort_column {
        validate_column_name(col)?;
        query.push_str(&format!(" ORDER BY {}", col));
        if let Some(dir) = sort_direction {
            let dir = validate_sort_direction(dir)?;
            query.push_str(&format!(" {}", dir));
        }
    }

    if let Some(l) = limit {
        query.push_str(&format!(" LIMIT {}", l));
    }

    if let Some(off) = offset {
        query.push_str(&format!(" OFFSET {}", off));
    }

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let column_count = column_names.len();

    let params_refs: Vec<&dyn rusqlite::ToSql> = params
        .iter()
        .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
        .collect();

    let rows = stmt
        .query_map(params_refs.as_slice(), |row| {
            let mut result = serde_json::json!({});
            for i in 0..column_count {
                let name = &column_names[i];
                let value: Value = match row.get_ref(i) {
                    Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                    Ok(rusqlite::types::ValueRef::Integer(i)) => Value::Number(i.into()),
                    Ok(rusqlite::types::ValueRef::Real(f)) => serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                    Ok(rusqlite::types::ValueRef::Text(t)) => {
                        Value::String(String::from_utf8_lossy(t).to_string())
                    }
                    Ok(rusqlite::types::ValueRef::Blob(b)) => {
                        Value::String(format!("<blob {} bytes>", b.len()))
                    }
                    Err(_) => Value::Null,
                };
                result[name] = value;
            }
            result["plugin_name"] = Value::String(plugin_name.to_string());
            Ok(result)
        })
        .map_err(|e| format!("Failed to query items: {}", e))?;

    let mut items = vec![];
    for item in rows {
        items.push(item.map_err(|e| format!("Failed to read item: {}", e))?);
    }

    Ok(items)
}

pub fn mutate_item(
    conn: &Connection,
    plugin_name: &str,
    operation: &str,
    id: Option<&str>,
    data: &Value,
) -> Result<Value, String> {
    let table_name = safe_table_name(plugin_name)?;

    match operation {
        "create" => {
            let new_id = uuid::Uuid::new_v4().to_string();
            let mut columns = vec!["id".to_string(), "created_at".to_string()];
            let mut placeholders = vec!["?1".to_string(), "datetime('now')".to_string()];
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(new_id.clone())];

            if let Some(obj) = data.as_object() {
                for (k, v) in obj {
                    validate_column_name(k)?;
                    columns.push(k.clone());
                    placeholders.push(format!("?{}", params.len() + 1));
                    params.push(json_value_to_sql_param(v));
                }
            }

            let query = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                table_name,
                columns.join(", "),
                placeholders.join(", ")
            );

            let params_refs: Vec<&dyn rusqlite::ToSql> = params
                .iter()
                .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
                .collect();

            conn.execute(&query, params_refs.as_slice())
                .map_err(|e| format!("Failed to create item: {}", e))?;

            let mut stmt = conn
                .prepare(&format!("SELECT * FROM {} WHERE id = ?1", table_name))
                .map_err(|e| format!("Failed to prepare query: {}", e))?;

            let column_names: Vec<String> =
                stmt.column_names().iter().map(|s| s.to_string()).collect();
            let column_count = column_names.len();

            let result = stmt
                .query_row([new_id.as_str()], |row| {
                    let mut item = serde_json::json!({});
                    for i in 0..column_count {
                        let name = &column_names[i];
                        let value: Value = match row.get_ref(i) {
                            Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                            Ok(rusqlite::types::ValueRef::Integer(i)) => Value::Number(i.into()),
                            Ok(rusqlite::types::ValueRef::Real(f)) => {
                                serde_json::Number::from_f64(f)
                                    .map(Value::Number)
                                    .unwrap_or(Value::Null)
                            }
                            Ok(rusqlite::types::ValueRef::Text(t)) => {
                                Value::String(String::from_utf8_lossy(t).to_string())
                            }
                            Ok(rusqlite::types::ValueRef::Blob(b)) => {
                                Value::String(format!("<blob {} bytes>", b.len()))
                            }
                            Err(_) => Value::Null,
                        };
                        item[name] = value;
                    }
                    item["plugin_name"] = Value::String(plugin_name.to_string());
                    Ok(item)
                })
                .map_err(|e| format!("Failed to fetch created item: {}", e))?;

            Ok(result)
        }

        "update" => {
            if let Some(item_id) = id {
                validate_item_id(item_id)?;
                let mut updates = vec![];
                let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

                if let Some(obj) = data.as_object() {
                    for (k, v) in obj {
                        validate_column_name(k)?;
                        updates.push(format!("{} = ?", k));
                        params.push(json_value_to_sql_param(v));
                    }
                }

                if updates.is_empty() {
                    return Err("No fields to update".to_string());
                }

                params.push(Box::new(item_id.to_string()));

                let query = format!(
                    "UPDATE {} SET {} WHERE id = ?",
                    table_name,
                    updates.join(", ")
                );

                let params_refs: Vec<&dyn rusqlite::ToSql> = params
                    .iter()
                    .map(|b| b.as_ref() as &dyn rusqlite::ToSql)
                    .collect();

                conn.execute(&query, params_refs.as_slice())
                    .map_err(|e| format!("Failed to update item: {}", e))?;

                info!("Updated item {} in {}", item_id, plugin_name);
                Ok(serde_json::json!({ "id": item_id, "updated": true }))
            } else {
                Err("Item ID required for update".to_string())
            }
        }

        "delete" => {
            if let Some(item_id) = id {
                validate_item_id(item_id)?;
                conn.execute(
                    &format!("DELETE FROM {} WHERE id = ?", table_name),
                    [item_id],
                )
                .map_err(|e| format!("Failed to delete item: {}", e))?;

                info!("Deleted item {} from {}", item_id, plugin_name);
                Ok(serde_json::json!({ "id": item_id, "deleted": true }))
            } else {
                Err("Item ID required for delete".to_string())
            }
        }

        _ => Err(format!("Unknown operation: {}", operation)),
    }
}

pub fn execute_ai_action(
    conn: &Connection,
    plugin_name: &str,
    action_id: &str,
    item_ids: &[String],
    _context: Option<&Value>,
) -> Result<Value, String> {
    info!(
        "Executing AI action '{}' for plugin '{}' on {} items",
        action_id,
        plugin_name,
        item_ids.len()
    );

    let endpoint = get_default_llm_endpoint(conn)?;
    let llm_client = LlmClient::new();

    let table_name = safe_table_name(plugin_name)?;

    let column_names = get_plugin_column_names(conn, plugin_name)?;
    let main_column = column_names
        .iter()
        .find(|c| c.col_type == "main")
        .map(|c| c.name.clone());
    let secondary_columns: Vec<String> = column_names
        .iter()
        .filter(|c| c.col_type == "secondary")
        .map(|c| c.name.clone())
        .collect();

    let system_prompt = build_classification_system_prompt(conn, plugin_name)?;

    #[derive(Clone)]
    struct ItemData {
        id: String,
        content: String,
    }

    let items_to_process: Vec<ItemData> = {
        let mut items = vec![];
        for item_id in item_ids {
            let mut stmt = conn
                .prepare(&format!("SELECT * FROM {} WHERE id = ?1", table_name))
                .map_err(|e| format!("Failed to prepare query: {}", e))?;

            let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            let col_count = cols.len();

            let item_result: Option<Value> = stmt
                .query_row([item_id.as_str()], |row| {
                    let mut item = serde_json::json!({});
                    for i in 0..col_count {
                        let name = &cols[i];
                        let value: Value = match row.get_ref(i) {
                            Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                            Ok(rusqlite::types::ValueRef::Integer(i)) => Value::Number(i.into()),
                            Ok(rusqlite::types::ValueRef::Real(f)) => {
                                serde_json::Number::from_f64(f)
                                    .map(Value::Number)
                                    .unwrap_or(Value::Null)
                            }
                            Ok(rusqlite::types::ValueRef::Text(t)) => {
                                Value::String(String::from_utf8_lossy(t).to_string())
                            }
                            Ok(rusqlite::types::ValueRef::Blob(b)) => {
                                Value::String(format!("<blob {} bytes>", b.len()))
                            }
                            Err(_) => Value::Null,
                        };
                        item[name] = value;
                    }
                    Ok::<Value, rusqlite::Error>(item)
                })
                .ok();

            if let Some(item) = item_result {
                if let Some(item_map) = item.as_object() {
                    let user_prompt = build_user_prompt_for_item(
                        item_map,
                        main_column.as_deref(),
                        &secondary_columns,
                    );
                    items.push(ItemData {
                        id: item_id.clone(),
                        content: user_prompt,
                    });
                }
            }
        }
        items
    };

    let mut results = vec![];
    let mut updated_count = 0;

    for item in items_to_process {
        match llm_client.complete(&endpoint, &system_prompt, &item.content, 0.7, 1024) {
            Ok(ai_response) => {
                let (cognitive_axis, context_axis) = llm::parse_classification_response(
                    &ai_response,
                    "cognitive_axis",
                    Some("context_axis"),
                )
                .unwrap_or(("General".to_string(), None));

                conn.execute(
                    &format!(
                        "UPDATE {} SET cognitive_axis = ?1 WHERE id = ?2",
                        table_name
                    ),
                    [&cognitive_axis, &item.id],
                )
                .map_err(|e| format!("Failed to update cognitive_axis: {}", e))?;

                if let Some(ctx) = &context_axis {
                    let _ = conn.execute(
                        &format!("UPDATE {} SET context_axis = ?1 WHERE id = ?2", table_name),
                        [ctx, &item.id],
                    );
                }

                results.push(serde_json::json!({
                    "id": item.id,
                    "cognitive_axis": cognitive_axis,
                    "context_axis": context_axis,
                }));
                updated_count += 1;
            }
            Err(e) => {
                info!("LLM call failed for item {}: {}", item.id, e);
                results.push(serde_json::json!({
                    "id": item.id,
                    "error": e,
                }));
            }
        }
    }

    Ok(serde_json::json!({
        "action_id": action_id,
        "processed": results.len(),
        "updated": updated_count,
        "items": results,
        "status": if updated_count > 0 { "completed" } else { "failed" }
    }))
}

fn get_default_llm_endpoint(conn: &Connection) -> Result<LlmEndpoint, String> {
    let result = conn.query_row(
        "SELECT id, name, provider, endpoint_url, api_key, model, is_default FROM llm_endpoints WHERE is_default = 1 LIMIT 1",
        [],
        |row| {
            Ok(LlmEndpoint {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                endpoint_url: row.get(3)?,
                api_key: row.get(4)?,
                model: row.get(5)?,
                is_default: row.get(6)?,
            })
        },
    );

    if let Ok(endpoint) = result {
        return Ok(endpoint);
    }

    conn.query_row(
        "SELECT id, name, provider, endpoint_url, api_key, model, is_default FROM llm_endpoints LIMIT 1",
        [],
        |row| {
            Ok(LlmEndpoint {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                endpoint_url: row.get(3)?,
                api_key: row.get(4)?,
                model: row.get(5)?,
                is_default: row.get(6)?,
            })
        },
    ).map_err(|_| "No LLM endpoint configured. Please add an LLM endpoint in settings.".to_string())
}

fn get_plugin_column_names(
    conn: &Connection,
    plugin_name: &str,
) -> Result<Vec<PluginColumn>, String> {
    let mut stmt = conn.prepare(
        "SELECT name, display, type, dtype, sortable FROM plugin_columns WHERE plugin_name = ?1"
    ).map_err(|e| e.to_string())?;

    let columns = stmt
        .query_map([plugin_name], |row| {
            Ok(PluginColumn {
                name: row.get(0)?,
                display: row.get(1)?,
                col_type: row.get(2)?,
                dtype: row.get(3)?,
                sortable: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;

    columns
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn build_classification_system_prompt(
    conn: &Connection,
    plugin_name: &str,
) -> Result<String, String> {
    let mut stmt = conn
        .prepare("SELECT name FROM context_axis ORDER BY name")
        .map_err(|e| e.to_string())?;

    let axes: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let cognitive_axes = vec![
        "require (needs immediate attention)",
        "review (needs to be reviewed)",
        "delegate (should be forwarded to someone else)",
        "schedule (needs to be scheduled)",
        "call (needs a phone call)",
        "meeting (needs a meeting)",
        "delete (can be deleted)",
    ];

    Ok(format!(
        r#"You are a cognitive axis classifier for a plugin called '{}'.
Your task is to classify items based on their content.

COGNITIVE AXES (choose one):
{}

CONTEXT AXES (choose one or 'General' if none applies):
{}

Respond with ONLY the cognitive axis and context axis in this format:
cognitive_axis: <axis>
context_axis: <context>

For example:
cognitive_axis: review
context_axis: Work"#,
        plugin_name,
        cognitive_axes
            .iter()
            .map(|a| format!("- {}", a))
            .collect::<Vec<_>>()
            .join("\n"),
        axes.join("\n")
    ))
}

fn build_user_prompt_for_item(
    item: &serde_json::Map<String, Value>,
    main_column: Option<&str>,
    secondary_columns: &[String],
) -> String {
    let mut parts = vec![];

    if let Some(main) = main_column {
        if let Some(value) = item.get(main) {
            parts.push(format!("{}: {}", main, value));
        }
    }

    for col in secondary_columns.iter().take(5) {
        if let Some(value) = item.get(col) {
            parts.push(format!("{}: {}", col, value));
        }
    }

    if parts.is_empty() {
        "No content available for classification".to_string()
    } else {
        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        db::migrate::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_register_and_query_columns() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO plugins (name, display_name, icon, version, is_enabled) VALUES ('email', 'Email', NULL, '0.1.0', 1)",
            [],
        )
        .unwrap();

        let columns = vec![
            PluginColumn {
                name: "subject".into(),
                display: "Subject".into(),
                col_type: "main".into(),
                dtype: "text".into(),
                sortable: true,
            },
            PluginColumn {
                name: "cognitive_axis".into(),
                display: "Axis".into(),
                col_type: "filterable".into(),
                dtype: "enum".into(),
                sortable: true,
            },
            PluginColumn {
                name: "context_axis".into(),
                display: "Context".into(),
                col_type: "filterable".into(),
                dtype: "text".into(),
                sortable: false,
            },
        ];

        register_plugin_columns(&conn, "email", &columns).unwrap();
        let result = query_plugin_columns(&conn, "email").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].name, "subject");
        assert_eq!(result[1].name, "cognitive_axis");
        assert_eq!(result[2].name, "context_axis");
    }

    #[test]
    fn test_register_replaces_old_columns() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO plugins (name, display_name, icon, version, is_enabled) VALUES ('test', 'Test', NULL, '0.1.0', 1)",
            [],
        )
        .unwrap();

        let cols_v1 = vec![PluginColumn {
            name: "old_col".into(),
            display: "Old".into(),
            col_type: "main".into(),
            dtype: "text".into(),
            sortable: false,
        }];
        register_plugin_columns(&conn, "test", &cols_v1).unwrap();

        let cols_v2 = vec![PluginColumn {
            name: "new_col".into(),
            display: "New".into(),
            col_type: "main".into(),
            dtype: "text".into(),
            sortable: false,
        }];
        register_plugin_columns(&conn, "test", &cols_v2).unwrap();

        let result = query_plugin_columns(&conn, "test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "new_col");
    }

    #[test]
    fn test_register_manifest() {
        let conn = setup_db();
        let manifest = PluginManifest {
            name: "email".into(),
            display_name: "Email".into(),
            icon: Some("📧".into()),
            columns: vec![
                PluginColumn {
                    name: "cognitive_axis".into(),
                    display: "Axis".into(),
                    col_type: "filterable".into(),
                    dtype: "enum".into(),
                    sortable: true,
                },
                PluginColumn {
                    name: "context_axis".into(),
                    display: "Context".into(),
                    col_type: "filterable".into(),
                    dtype: "text".into(),
                    sortable: false,
                },
            ],
            ai_actions: vec![],
            provides_actions: vec![],
        };

        register_manifest(&conn, &manifest).unwrap();

        let plugins = get_registered_plugins(&conn).unwrap();
        assert_eq!(plugins, vec!["email"]);

        let columns = query_plugin_columns(&conn, "email").unwrap();
        assert_eq!(columns.len(), 2);
    }

    #[test]
    fn test_get_registered_plugins_empty() {
        let conn = setup_db();
        let plugins = get_registered_plugins(&conn).unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_query_columns_nonexistent_plugin() {
        let conn = setup_db();
        let result = query_plugin_columns(&conn, "nonexistent").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_set_plugin_validation_error_disables() {
        let conn = setup_db();
        let manifest = PluginManifest {
            name: "broken".into(),
            display_name: "Broken".into(),
            icon: None,
            columns: vec![],
            ai_actions: vec![],
            provides_actions: vec![],
        };
        register_manifest(&conn, &manifest).unwrap();

        set_plugin_validation_error(&conn, "broken", "missing column").unwrap();

        let plugins = get_registered_plugins(&conn).unwrap();
        assert!(!plugins.contains(&"broken".to_string()));
    }
}
