-- SecurePass 数据库结构
-- 与 src/database/mod.rs::initialize_database 保持一致

CREATE TABLE IF NOT EXISTS passwords (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_name TEXT NOT NULL,
    site_url TEXT NOT NULL,
    username TEXT NOT NULL,
    encrypted_password TEXT NOT NULL,
    encrypted_notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_accessed TEXT,
    access_count INTEGER DEFAULT 0,
    tags TEXT,
    priority INTEGER DEFAULT 0,
    metadata TEXT,
    checksum TEXT
);

CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    encrypted BOOLEAN DEFAULT FALSE,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    user_id TEXT,
    action TEXT NOT NULL,
    resource_type TEXT,
    resource_id TEXT,
    details TEXT,
    ip_address TEXT,
    user_agent TEXT,
    success BOOLEAN NOT NULL,
    error_message TEXT
);

CREATE TABLE IF NOT EXISTS backups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    backup_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    checksum TEXT NOT NULL,
    encrypted BOOLEAN NOT NULL,
    retention_until TEXT
);

CREATE TABLE IF NOT EXISTS password_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    password_id INTEGER NOT NULL,
    changed_at TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    FOREIGN KEY (password_id) REFERENCES passwords(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_passwords_site ON passwords(site_name);
CREATE INDEX IF NOT EXISTS idx_passwords_username ON passwords(username);
CREATE INDEX IF NOT EXISTS idx_passwords_tags ON passwords(tags);
CREATE INDEX IF NOT EXISTS idx_passwords_created_at ON passwords(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_backups_timestamp ON backups(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_password_history_pid ON password_history(password_id);
