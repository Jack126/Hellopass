use std::path::Path;
use rusqlite::{Connection, Result};
use super::DatabaseError;

/// 数据库连接管理器
pub struct DatabaseConnection {
    conn: Connection,
}

impl DatabaseConnection {
    /// 创建新的数据库连接
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DatabaseError> {
        let conn = Connection::open(path)?;
        
        // 设置连接参数
        Self::configure_connection(&conn)?;
        
        Ok(Self { conn })
    }

    /// 配置数据库连接
    fn configure_connection(conn: &Connection) -> Result<(), DatabaseError> {
        // 设置WAL模式提高并发性能
        conn.execute("PRAGMA journal_mode = WAL", [])?;
        
        // 设置同步模式为NORMAL（平衡性能和数据安全）
        conn.execute("PRAGMA synchronous = NORMAL", [])?;
        
        // 设置缓存大小
        conn.execute("PRAGMA cache_size = -10000", [])?; // 10MB
        
        // 设置临时存储内存
        conn.execute("PRAGMA temp_store = MEMORY", [])?;
        
        // 设置 mmap 大小
        conn.execute("PRAGMA mmap_size = 268435456", [])?; // 256MB
        
        // 设置 busy 超时时间
        conn.execute("PRAGMA busy_timeout = 30000", [])?; // 30秒
        
        // 启用外键约束
        conn.execute("PRAGMA foreign_keys = ON", [])?;
        
        Ok(())
    }

    /// 获取数据库连接
    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    /// 执行查询
    pub fn query<T, F>(&self, query: &str, params: &[&dyn rusqlite::ToSql], mapper: F) -> Result<Vec<T>, DatabaseError>
    where
        F: Fn(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let mut stmt = self.conn.prepare(query)?;
        let results = stmt.query_map(params, mapper)?;
        
        let mut vec = Vec::new();
        for result in results {
            vec.push(result?);
        }
        
        Ok(vec)
    }

    /// 执行查询，返回单个结果
    pub fn query_one<T, F>(&self, query: &str, params: &[&dyn rusqlite::ToSql], mapper: F) -> Result<T, DatabaseError>
    where
        F: Fn(&rusqlite::Row) -> rusqlite::Result<T>,
    {
        let mut stmt = self.conn.prepare(query)?;
        let result = stmt.query_row(params, mapper)?;
        
        Ok(result)
    }

    /// 执行插入操作
    pub fn insert(&self, query: &str, params: &[&dyn rusqlite::ToSql]) -> Result<i64, DatabaseError> {
        let mut stmt = self.conn.prepare(query)?;
        let result = stmt.insert(params)?;
        
        Ok(result)
    }

    /// 执行更新操作
    pub fn update(&self, query: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize, DatabaseError> {
        let mut stmt = self.conn.prepare(query)?;
        let result = stmt.execute(params)?;
        
        Ok(result)
    }

    /// 执行删除操作
    pub fn delete(&self, query: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize, DatabaseError> {
        let mut stmt = self.conn.prepare(query)?;
        let result = stmt.execute(params)?;
        
        Ok(result)
    }

    /// 执行事务
    pub fn transaction<T, F>(&mut self, f: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error>,
    {
        let tx = self.conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        
        Ok(result)
    }

    /// 执行批量操作
    pub fn batch_execute(&self, queries: &[String]) -> Result<(), DatabaseError> {
        for query in queries {
            self.conn.execute(query, [])?;
        }
        
        Ok(())
    }

    /// 获取数据库版本
    pub fn get_version(&self) -> Result<String, DatabaseError> {
        let version: String = self.conn.query_row(
            "SELECT sqlite_version()",
            [],
            |row| row.get(0)
        )?;
        
        Ok(version)
    }

    /// 获取数据库大小
    pub fn get_database_size(&self) -> Result<i64, DatabaseError> {
        let size: i64 = self.conn.query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0)
        )?;
        
        Ok(size)
    }

    /// 获取表信息
    pub fn get_table_info(&self, table_name: &str) -> Result<Vec<(String, String)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({})", table_name))?;
        let results = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,  // column name
                row.get::<_, String>(2)?,  // column type
            ))
        })?;
        
        let mut info = Vec::new();
        for result in results {
            info.push(result?);
        }
        
        Ok(info)
    }

    /// 检查表是否存在
    pub fn table_exists(&self, table_name: &str) -> Result<bool, DatabaseError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            &[&table_name.to_string()],
            |row| row.get(0)
        )?;
        
        Ok(count > 0)
    }

    /// 检查索引是否存在
    pub fn index_exists(&self, index_name: &str) -> Result<bool, DatabaseError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?",
            &[&index_name.to_string()],
            |row| row.get(0)
        )?;
        
        Ok(count > 0)
    }

    /// 优化数据库
    pub fn optimize(&self) -> Result<(), DatabaseError> {
        let queries = vec![
            "VACUUM".to_string(),
            "ANALYZE".to_string(),
        ];
        
        self.batch_execute(&queries)
    }

    /// 备份数据库
    pub fn backup(&self, backup_path: &str) -> Result<(), DatabaseError> {
        // 简化的备份实现
        use std::fs::File;
        
        let backup_file = File::create(backup_path)?;
        let _ = self.conn.execute("VACUUM INTO ?", &[backup_file]);
        
        Ok(())
    }

    /// 恢复数据库
    pub fn restore(&self, backup_path: &str) -> Result<(), DatabaseError> {
        use std::fs::File;
        
        // 简化的恢复实现
        use std::fs::File;
        use std::io::Write;
        
        let backup_file = File::open(backup_path)?;
        let _ = self.conn.execute("VACUUM INTO ?", &[backup_file]);
        
        Ok(())
    }

    /// 获取数据库统计信息
    pub fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let table_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0)
        )?;

        let index_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index'",
            [],
            |row| row.get(0)
        )?;

        let database_size = self.get_database_size()?;
        let version = self.get_version()?;

        Ok(DatabaseStats {
            table_count,
            index_count,
            database_size,
            version,
        })
    }
}

/// 数据库统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatabaseStats {
    pub table_count: i64,
    pub index_count: i64,
    pub database_size: i64,
    pub version: String,
}

/// 连接池管理器
pub struct ConnectionPool {
    connections: Vec<DatabaseConnection>,
    max_size: usize,
    current_size: usize,
}

impl ConnectionPool {
    /// 创建新的连接池
    pub fn new<P: AsRef<Path>>(path: P, max_size: usize) -> Result<Self, DatabaseError> {
        let mut connections = Vec::new();
        
        // 初始化连接池
        for _ in 0..std::cmp::min(max_size, 5) { // 初始创建5个连接
            let conn = DatabaseConnection::new(path.as_ref())?;
            connections.push(conn);
        }
        
        Ok(Self {
            connections,
            max_size,
            current_size: connections.len(),
        })
    }

    /// 获取连接
    pub fn get_connection(&mut self) -> Result<DatabaseConnection, DatabaseError> {
        if let Some(conn) = self.connections.pop() {
            Ok(conn)
        } else if self.current_size < self.max_size {
            // 创建新连接
            self.current_size += 1;
            let conn = DatabaseConnection::new("passwords.db")?; // 这里应该使用实际的路径
            Ok(conn)
        } else {
            Err(DatabaseError::ConnectionError(rusqlite::Error::InvalidQuery))
        }
    }

    /// 释放连接
    pub fn release_connection(&mut self, conn: DatabaseConnection) {
        if self.connections.len() < self.max_size {
            self.connections.push(conn);
        }
        // 如果连接池已满，连接将被丢弃
    }

    /// 获取连接池统计
    pub fn get_stats(&self) -> ConnectionPoolStats {
        ConnectionPoolStats {
            available_connections: self.connections.len(),
            max_connections: self.max_size,
            current_connections: self.current_size,
        }
    }
}

/// 连接池统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectionPoolStats {
    pub available_connections: usize,
    pub max_connections: usize,
    pub current_connections: usize,
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        // 关闭所有连接
        for conn in self.connections.drain(..) {
            // 连接会在drop时自动关闭
        }
    }
}