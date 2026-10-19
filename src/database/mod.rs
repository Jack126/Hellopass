pub mod models;
pub mod connection;
pub mod operations;

use std::path::Path;
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
}

/// 数据库管理器
pub struct Database {
    conn: Connection,
}

impl Database {
    /// 创建新的数据库连接
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let conn = Connection::open(path)?;
        
        // 初始化数据库
        Self::initialize_database(&conn)?;
        
        Ok(Self { conn })
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
                category TEXT DEFAULT 'general',
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

        // 创建索引
        Self::create_indexes(conn)?;

        Ok(())
    }

    /// 创建数据库索引
    fn create_indexes(conn: &Connection) -> Result<(), DatabaseError> {
        // 密码表索引
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

        // 审计日志索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp DESC)",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_logs_severity ON audit_logs(severity)",
            [],
        )?;

        // 备份记录索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_backups_timestamp ON backups(timestamp DESC)",
            [],
        )?;

        Ok(())
    }

    /// 获取数据库连接
    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    /// 执行数据库事务
    pub fn transaction<T, F>(&self, f: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let tx = self.conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    /// 备份数据库
    pub fn backup(&self, backup_path: &str) -> Result<(), DatabaseError> {
        use std::fs::File;
        use std::io::Write;
        
        let backup_file = File::create(backup_path)?;
        let mut backup = rusqlite::backup::Backup::new(&self.conn, &backup_file)?;
        
        while backup.step(-1)? != rusqlite::backup::BackupStep::Done {
            // 继续备份
        }
        
        Ok(())
    }

    /// 检查数据库完整性
    pub fn check_integrity(&self) -> Result<bool, DatabaseError> {
        let result: Result<i32, _> = self.conn.query_row(
            "PRAGMA integrity_check",
            [],
            |row| row.get(0)
        );
        
        match result {
            Ok(0) => Ok(true),  // 完整性检查通过
            Ok(_) => Ok(false), // 发现问题
            Err(_) => Ok(false),
        }
    }

    /// 获取数据库统计信息
    pub fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let password_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM passwords",
            [],
            |row| row.get(0)
        )?;

        let config_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM config",
            [],
            |row| row.get(0)
        )?;

        let audit_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM audit_logs",
            [],
            |row| row.get(0)
        )?;

        let backup_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM backups",
            [],
            |row| row.get(0)
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

// 重新导出常用模块
pub use models::*;
pub use connection::*;
pub use operations::*;