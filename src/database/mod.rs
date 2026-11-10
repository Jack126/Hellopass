pub mod operations;

use std::path::{Path, PathBuf};
use rusqlite::{Connection, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("数据库连接失败: {0}")]
    ConnectionError(#[from] rusqlite::Error),
    #[error("数据库操作失败: {0}")]
    OperationError(String),
    #[error("数据不存在")]
    NotFound,
    #[error("数据验证失败: {0}")]
    ValidationError(String),
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),
}

/// 数据库管理器
#[derive(Debug)]
pub struct Database {
    conn: Connection,
    path: PathBuf,
}

impl Database {
    /// 创建新的数据库连接
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let conn = Connection::open(&path)?;

        // 初始化数据库
        Self::initialize_database(&conn)?;

        // 数据库文件权限收紧为 0600（仅属主可读写），避免凭据库被其他用户读取
        #[cfg(unix)]
        set_private_permissions(&path);

        Ok(Self {
            conn,
            path: path.as_ref().to_path_buf(),
        })
    }

    /// 初始化数据库表结构
    fn initialize_database(conn: &Connection) -> Result<(), DatabaseError> {
        // 创建密码表
        conn.execute(
            r#"
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
            )
            "#,
            [],
        )?;

        // 创建配置表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                encrypted BOOLEAN DEFAULT FALSE,
                updated_at TEXT NOT NULL
            )
            "#,
            [],
        )?;

        // 创建审计日志表
        conn.execute(
            r#"
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
            )
            "#,
            [],
        )?;

        // 创建备份记录表
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS backups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                backup_type TEXT NOT NULL,
                file_path TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                checksum TEXT NOT NULL,
                encrypted BOOLEAN NOT NULL,
                retention_until TEXT
            )
            "#,
            [],
        )?;

        // 创建密码历史表：存「用主密钥加密后的历史密码密文」，用于防止复用最近用过的密码。
        // 早期版本这里存的是明文口令的裸 SHA-256（可被离线爆破），该格式已废弃（见下方迁移）。
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS password_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                password_id INTEGER NOT NULL,
                changed_at TEXT NOT NULL,
                password_hash TEXT,
                encrypted_password TEXT,
                FOREIGN KEY (password_id) REFERENCES passwords(id) ON DELETE CASCADE
            )
            "#,
            [],
        )?;

        // 创建 schema 元信息表（记录迁移版本）
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )?;

        // 兼容旧库：补齐可能缺失的列
        Self::migrate_columns(conn)?;

        // v3：密码历史改为「主密钥加密」。旧版本存的是裸 SHA-256，无法反推也无法转换，
        // 只能丢弃，避免继续留在库里被离线爆破（幂等：每次打开都顺手清掉历史垃圾）。
        conn.execute(
            "DELETE FROM password_history WHERE encrypted_password IS NULL",
            [],
        )?;

        // 创建索引
        Self::create_indexes(conn)?;

        // 记录 schema 版本
        Self::set_schema_version(conn, 4)?;

        Ok(())
    }

    /// 旧库迁移：确保关键列存在（CREATE TABLE IF NOT EXISTS 不会补列）。
    fn migrate_columns(conn: &Connection) -> Result<(), DatabaseError> {
        let password_cols = [
            ("last_accessed", "last_accessed TEXT"),
            ("access_count", "access_count INTEGER DEFAULT 0"),
            ("tags", "tags TEXT"),
            ("priority", "priority INTEGER DEFAULT 0"),
            ("metadata", "metadata TEXT"),
            ("checksum", "checksum TEXT"),
        ];
        for (name, ddl) in password_cols.iter() {
            Self::ensure_column(conn, "passwords", name, ddl)?;
        }

        // v4：category 从未在界面使用，删列（SQLite 3.35+ 支持 DROP COLUMN）
        Self::drop_column_if_exists(conn, "passwords", "category")?;

        let audit_cols = [
            ("user_id", "user_id TEXT"),
            ("resource_type", "resource_type TEXT"),
            ("resource_id", "resource_id TEXT"),
            ("details", "details TEXT"),
            ("ip_address", "ip_address TEXT"),
            ("user_agent", "user_agent TEXT"),
            ("error_message", "error_message TEXT"),
        ];
        for (name, ddl) in audit_cols.iter() {
            Self::ensure_column(conn, "audit_logs", name, ddl)?;
        }

        // 旧库的 password_history 没有加密列；password_hash 上的 NOT NULL 无法直接去掉，
        // 新代码会写空串占位、只使用 encrypted_password。
        Self::ensure_column(conn, "password_history", "encrypted_password", "encrypted_password TEXT")?;
        Ok(())
    }

    fn ensure_column(
        conn: &Connection,
        table: &str,
        column: &str,
        ddl: &str,
    ) -> Result<(), DatabaseError> {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
            rusqlite::params![table, column],
            |r| r.get::<_, i64>(0),
        )? > 0;
        if !exists {
            let sql = format!("ALTER TABLE {} ADD COLUMN {}", table, ddl);
            conn.execute(&sql, [])?;
        }
        Ok(())
    }

    fn drop_column_if_exists(conn: &Connection, table: &str, column: &str) -> Result<(), DatabaseError> {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM pragma_table_info(?1) WHERE name = ?2",
            rusqlite::params![table, column],
            |r| r.get::<_, i64>(0),
        )? > 0;
        if exists {
            let sql = format!("ALTER TABLE {} DROP COLUMN {}", table, column);
            conn.execute(&sql, [])?;
        }
        Ok(())
    }

    fn set_schema_version(conn: &Connection, version: i64) -> Result<(), DatabaseError> {
        conn.execute(
            "INSERT INTO schema_meta (key, value) VALUES ('schema_version', ?1) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![version.to_string()],
        )?;
        Ok(())
    }

    /// 当前数据库 schema 版本。
    pub fn schema_version(&self) -> Result<i64, DatabaseError> {
        let v: String = self.conn.query_row(
            "SELECT value FROM schema_meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )?;
        Ok(v.parse().unwrap_or(0))
    }

    /// 创建数据库索引
    fn create_indexes(conn: &Connection) -> Result<(), DatabaseError> {
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_passwords_site ON passwords(site_name)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_passwords_username ON passwords(username)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_passwords_tags ON passwords(tags)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_passwords_created_at ON passwords(created_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_backups_timestamp ON backups(timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_password_history_pid ON password_history(password_id)",
            [],
        )?;

        Ok(())
    }

    /// 获取数据库连接（只读借用）
    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    /// 执行数据库事务
    pub fn transaction<T, F>(&mut self, f: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let tx = self.conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    /// 备份数据库（VACUUM INTO 需要目标路径字符串，而非文件句柄）
    pub fn backup(&self, backup_path: &str) -> Result<(), DatabaseError> {
        self.conn.execute("VACUUM INTO ?", [backup_path])?;
        Ok(())
    }

    /// 检查数据库完整性
    pub fn check_integrity(&self) -> Result<bool, DatabaseError> {
        let result: String = self.conn.query_row(
            "PRAGMA integrity_check",
            [],
            |row| row.get(0),
        )?;
        // integrity_check 在无问题时返回单行文本 "ok"
        Ok(result == "ok")
    }

    /// 计算数据库文件的 SHA-256 与字节大小（用于文件级完整性记录/比对）。
    pub fn file_checksum(&self) -> Result<(String, u64), DatabaseError> {
        use sha2::{Sha256, Digest};
        let data = std::fs::read(&self.path)?;
        let size = data.len() as u64;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok((format!("{:x}", hasher.finalize()), size))
    }

    /// 获取数据库统计信息
    pub fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let password_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM passwords",
            [],
            |row| row.get(0),
        )?;

        let config_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM config",
            [],
            |row| row.get(0),
        )?;

        let audit_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM audit_logs",
            [],
            |row| row.get(0),
        )?;

        let backup_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM backups",
            [],
            |row| row.get(0),
        )?;

        Ok(DatabaseStats {
            password_count,
            config_count,
            audit_count,
            backup_count,
        })
    }
}

/// 数据库统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub password_count: i64,
    pub config_count: i64,
    pub audit_count: i64,
    pub backup_count: i64,
}


/// 将文件权限收紧为 0600（仅属主可读写）。仅在 Unix 平台执行。
#[cfg(unix)]
fn set_private_permissions<P: AsRef<std::path::Path>>(path: P) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(mut perms) = std::fs::metadata(&path).map(|m| m.permissions()) {
        perms.set_mode(0o600);
        let _ = std::fs::set_permissions(&path, perms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_old_schema_and_sets_version() {
        let dir = std::env::temp_dir();
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = dir.join(format!("hellopass_migrate_{}.db", stamp));
        let _ = std::fs::remove_file(&db_path);

        // 手工创建一个旧版最小 passwords 表（缺少 priority/tags 等列）
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute(
                "CREATE TABLE passwords (id INTEGER PRIMARY KEY AUTOINCREMENT, site_name TEXT NOT NULL, site_url TEXT NOT NULL, username TEXT NOT NULL, encrypted_password TEXT NOT NULL, encrypted_notes TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, category TEXT DEFAULT 'general')",
                [],
            )
            .unwrap();
        }

        let db = Database::new(&db_path).expect("open old db");
        let has_priority: i64 = db
            .get_connection()
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('passwords') WHERE name = 'priority'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_priority, 1);
        let has_category: i64 = db
            .get_connection()
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('passwords') WHERE name = 'category'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_category, 0, "v4 迁移应删除 category 列");
        assert_eq!(db.schema_version().unwrap(), 4);

        let _ = std::fs::remove_file(&db_path);
    }
}
