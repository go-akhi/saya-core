CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY,
    applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS plugins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    icon TEXT,
    version TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS plugin_columns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_name TEXT NOT NULL,
    name TEXT NOT NULL,
    display TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('main', 'secondary', 'filterable', 'hidden')),
    dtype TEXT NOT NULL CHECK(dtype IN ('text', 'datetime', 'integer', 'boolean', 'binary', 'enum')),
    sortable BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (plugin_name) REFERENCES plugins(name)
);

CREATE TABLE IF NOT EXISTS context_axis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    icon TEXT,
    color TEXT,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    email TEXT NOT NULL,
    access_token TEXT,
    refresh_token TEXT,
    token_expiry DATETIME,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS llm_endpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    endpoint_url TEXT NOT NULL,
    api_key TEXT,
    model TEXT NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO schema_version (version) VALUES (2);

INSERT OR IGNORE INTO context_axis (name, description, icon, color, is_default) VALUES
    ('General', 'Catch-all for unclassified items', '\1F4CB', '#6B7280', 1),
    ('Work', NULL, '\1F4BC', '#3B82F6', 1),
    ('Personal', NULL, '\1F3E0', '#10B981', 1);
