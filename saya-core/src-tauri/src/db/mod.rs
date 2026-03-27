pub mod migrate;

use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::info;

pub struct DbState {
    pub conn: Mutex<Connection>,
}

fn get_db_path() -> PathBuf {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("saya-core");
    fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    data_dir.join("saya.db")
}

pub fn init_database() -> rusqlite::Result<DbState> {
    let db_path = get_db_path();
    info!("Opening database at: {:?}", db_path);

    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    migrate::run_migrations(&conn)?;

    let version = migrate::get_schema_version(&conn)?;
    info!("Database ready at version {}", version);

    Ok(DbState {
        conn: Mutex::new(conn),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate::run_migrations(&conn).unwrap();

        let version = migrate::get_schema_version(&conn).unwrap();
        assert_eq!(version, 2);

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            table_count >= 5,
            "Expected at least 5 tables, got {}",
            table_count
        );
    }

    #[test]
    fn test_context_axis_seeded() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate::run_migrations(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM context_axis", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_general_axis_is_first() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate::run_migrations(&conn).unwrap();

        let name: String = conn
            .query_row(
                "SELECT name FROM context_axis ORDER BY id ASC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "General");
    }

    #[test]
    fn test_general_axis_has_description() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate::run_migrations(&conn).unwrap();

        let desc: Option<String> = conn
            .query_row(
                "SELECT description FROM context_axis WHERE name = 'General'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(desc.is_some());
        assert!(desc.unwrap().contains("Catch-all"));
    }
}
