use rusqlite::Connection;
use tracing::info;

const SCHEMA_SQL: &str = include_str!("schema.sql");

pub fn get_schema_version(conn: &Connection) -> rusqlite::Result<i64> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(0);
    }
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let count: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = '{}'",
            table, column
        ),
        [],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn migrate_v1_to_v2(conn: &Connection) -> rusqlite::Result<()> {
    info!("Applying schema version 2: add description to context_axis, seed General");

    if !column_exists(conn, "context_axis", "description")? {
        conn.execute_batch("ALTER TABLE context_axis ADD COLUMN description TEXT;")?;
    }

    conn.execute_batch(
        "INSERT OR IGNORE INTO context_axis (name, description, icon, color, is_default)
             VALUES ('General', 'Catch-all for unclassified items', X'1F4CB', '#6B7280', 1);
         INSERT OR REPLACE INTO schema_version (version) VALUES (2);",
    )?;

    info!("Schema version 2 applied");
    Ok(())
}

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    let current = get_schema_version(conn)?;
    info!("Current schema version: {}", current);

    if current < 1 {
        info!("Applying schema version 1: initial schema");
        conn.execute_batch(SCHEMA_SQL)?;
        info!("Schema version 1 applied");
        return Ok(());
    }

    if current < 2 {
        migrate_v1_to_v2(conn)?;
    }

    Ok(())
}
