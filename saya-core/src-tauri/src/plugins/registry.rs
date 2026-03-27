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
    let table_name = format!("{}_items", plugin_name.replace('-', "_"));

    let cols = if let Some(c) = columns {
        c.join(", ")
    } else {
        "*".to_string()
    };

    let mut query = format!("SELECT {} FROM {}", cols, table_name);
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(f) = filters {
        if let Some(obj) = f.as_object() {
            let conditions: Vec<String> = obj
                .iter()
                .map(|(k, v)| {
                    params.push(json_value_to_sql_param(v));
                    format!("{} = ?", k)
                })
                .collect();
            if !conditions.is_empty() {
                query.push_str(" WHERE ");
                query.push_str(&conditions.join(" AND "));
            }
        }
    }

    if let Some(col) = sort_column {
        query.push_str(&format!(" ORDER BY {}", col));
        if let Some(dir) = sort_direction {
            if dir.eq_ignore_ascii_case("desc") {
                query.push_str(" DESC");
            }
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
    let table_name = format!("{}_items", plugin_name.replace('-', "_"));

    match operation {
        "create" => {
            let new_id = uuid::Uuid::new_v4().to_string();
            let mut columns = vec!["id".to_string(), "created_at".to_string()];
            let mut placeholders = vec!["?1".to_string(), "datetime('now')".to_string()];
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(new_id.clone())];

            if let Some(obj) = data.as_object() {
                for (k, v) in obj {
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
                let mut updates = vec![];
                let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

                if let Some(obj) = data.as_object() {
                    for (k, v) in obj {
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

    let table_name = format!("{}_items", plugin_name.replace('-', "_"));

    let mut results = vec![];
    for item_id in item_ids {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {} WHERE id = ?1", table_name))
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let column_count = column_names.len();

        let result = stmt
            .query_row([item_id.as_str()], |row| {
                let mut item = serde_json::json!({});
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
                    item[name] = value;
                }
                item["plugin_name"] = Value::String(plugin_name.to_string());
                Ok(item)
            })
            .ok();

        if let Some(item) = result {
            results.push(item);
        }
    }

    Ok(serde_json::json!({
        "action_id": action_id,
        "processed": results.len(),
        "items": results,
        "status": "pending_ai_processing"
    }))
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
