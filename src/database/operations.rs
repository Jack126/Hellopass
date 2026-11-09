use crate::api::{NewPassword, PasswordEntry, UpdatePassword, AuditLogEntry, PasswordPolicy};
use crate::database::Database;
use crate::security::SecureCryptoEngine;
use base64::Engine;
use chrono::Utc;
use thiserror::Error;
use crate::utils::log;

/// 记住的历史密码条数：更新密码时不能与最近这么多次使用过的密码相同。
const PASSWORD_HISTORY_LIMIT: usize = 5;
/// 密码最长使用年限（天），超过即视为过期。
const PASSWORD_MAX_AGE_DAYS: i64 = 90;

#[derive(Error, Debug)]
pub enum OperationError {
    #[error("数据库操作失败: {0}")]
    DatabaseError(String),
    #[error("数据不存在")]
    NotFound,
    #[error("数据验证失败: {0}")]
    ValidationError(String),
    #[error("解密失败（密码错误或数据被篡改）")]
    DecryptError,
}

/// 密码操作：持有数据库连接与主密钥派生的加密引擎。
/// 明文密码在落库前用 AES-256-GCM 加密（密文以 base64 存储），
/// 读取时解密并以明文形式返回给调用方（解锁会话内）。
pub struct PasswordOperations {
    database: Database,
    crypto: SecureCryptoEngine,
}

/// 数据库原始行（密文形式），先映射再解密，避免闭包捕获 self。
struct RawRow {
    id: i64,
    site_name: String,
    site_url: String,
    username: String,
    encrypted_password: String,
    encrypted_notes: Option<String>,
    created_at: String,
    updated_at: String,
    last_accessed: Option<String>,
    access_count: i32,
    tags: Option<String>,
    category: String,
    priority: i32,
    metadata: Option<String>,
    checksum: Option<String>,
}

impl PasswordOperations {
    pub fn new(database: Database, crypto: SecureCryptoEngine) -> Self {
        Self { database, crypto }
    }

    fn conn(&self) -> &rusqlite::Connection {
        self.database.get_connection()
    }

    fn encrypt_to_b64(&self, plain: &str) -> Result<String, OperationError> {
        let ct = self
            .crypto
            .encrypt(plain.as_bytes())
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(ct))
    }

    fn decrypt_from_b64(&self, b64: &str) -> Result<String, OperationError> {
        let ct = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|_| OperationError::DecryptError)?;
        let plain = self
            .crypto
            .decrypt(&ct)
            .map_err(|_| OperationError::DecryptError)?;
        String::from_utf8(plain).map_err(|_| OperationError::DecryptError)
    }

    /// 用当前主密钥加密任意字节并返回 base64（用于加密备份）。
    pub fn encrypt_blob(&self, plain: &[u8]) -> Result<String, OperationError> {
        let ct = self
            .crypto
            .encrypt(plain)
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        Ok(base64::engine::general_purpose::STANDARD.encode(ct))
    }

    /// 解密 base64 密文（用于加密备份恢复）。
    pub fn decrypt_blob(&self, b64: &str) -> Result<Vec<u8>, OperationError> {
        let ct = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|_| OperationError::DecryptError)?;
        self.crypto
            .decrypt(&ct)
            .map_err(|_| OperationError::DecryptError)
    }

    /// 添加密码
    pub fn add_password(&self, password: NewPassword) -> Result<PasswordEntry, OperationError> {
        let now = Utc::now().to_rfc3339();
        let enc_pwd = self.encrypt_to_b64(&password.password)?;
        let enc_notes = match &password.notes {
            Some(n) => Some(self.encrypt_to_b64(n)?),
            None => None,
        };
        let tags_str = password.tags.clone().unwrap_or_default().join(",");
        let category = password.category.clone().unwrap_or_else(|| "general".to_string());
        let priority = password.priority.unwrap_or(0);

        let id = self.conn().execute(
            "INSERT INTO passwords              (site_name, site_url, username, encrypted_password, encrypted_notes, created_at, updated_at, access_count, tags, category, priority)              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10)",
            rusqlite::params![
                password.site_name,
                password.site_url,
                password.username,
                enc_pwd,
                enc_notes,
                now,
                now,
                tags_str,
                category,
                priority
            ],
        ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        // SQLite 的 rowid 即插入后的自增 id
        let _ = id;
        let row_id = self.conn().query_row(
            "SELECT last_insert_rowid()",
            [],
            |r| r.get::<_, i64>(0),
        ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        let _ = self.record_password_history(row_id, &password.password);

        self.get_password(row_id)
    }

    /// 获取所有密码（解密后返回明文）
    pub fn get_passwords(&self) -> Result<Vec<PasswordEntry>, OperationError> {
        let mut stmt = self.conn().prepare(
            "SELECT id, site_name, site_url, username, encrypted_password, encrypted_notes,              created_at, updated_at, last_accessed, access_count, tags, category, priority, metadata, checksum              FROM passwords ORDER BY created_at DESC",
        ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map([], Self::map_row)
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?
            .collect::<rusqlite::Result<Vec<RawRow>>>()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(|r| self.raw_to_entry(r)).collect()
    }

    /// 按 id 获取单条
    pub fn get_password(&self, id: i64) -> Result<PasswordEntry, OperationError> {
        let mut stmt = self.conn().prepare(
            "SELECT id, site_name, site_url, username, encrypted_password, encrypted_notes,              created_at, updated_at, last_accessed, access_count, tags, category, priority, metadata, checksum              FROM passwords WHERE id = ?1",
        ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        let row = stmt
            .query_row(rusqlite::params![id], Self::map_row)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => OperationError::NotFound,
                _ => OperationError::DatabaseError(e.to_string()),
            })?;

        self.raw_to_entry(row)
    }

    /// 搜索密码（按站点名/用户名/标签模糊匹配）
    pub fn search_passwords(&self, query: &str) -> Result<Vec<PasswordEntry>, OperationError> {
        let like = format!("%{}%", query);
        let mut stmt = self.conn().prepare(
            "SELECT id, site_name, site_url, username, encrypted_password, encrypted_notes,              created_at, updated_at, last_accessed, access_count, tags, category, priority, metadata, checksum              FROM passwords              WHERE site_name LIKE ?1 OR username LIKE ?1 OR tags LIKE ?1              ORDER BY created_at DESC",
        ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![like], Self::map_row)
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?
            .collect::<rusqlite::Result<Vec<RawRow>>>()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        rows.into_iter().map(|r| self.raw_to_entry(r)).collect()
    }

    /// 更新密码（仅更新提供的字段）
    pub fn update_password(&self, p: UpdatePassword) -> Result<PasswordEntry, OperationError> {
        let current =         self.get_password(p.id)?;

        let site_name = p.site_name.unwrap_or(current.site_name);
        let site_url = p.site_url.unwrap_or(current.site_url);
        let username = p.username.unwrap_or(current.username);
        let category = p.category.unwrap_or(current.category);
        let priority = p.priority.unwrap_or(current.priority);
        let tags = p.tags.unwrap_or(current.tags);
        // 空字符串视为“不修改密码”；与当前密码相同也视为未修改（表单原样提交不应被当成复用）
        let submitted_password = p.password.clone().unwrap_or_default();
        let password_changed = !submitted_password.is_empty()
            && Some(submitted_password.as_str()) != current.password.as_deref();
        // 防复用：只拦截“改回一个历史密码”
        if password_changed && self.is_password_reused(p.id, &submitted_password)? {
            return Err(OperationError::ValidationError(format!(
                "新密码不能与最近 {} 次使用过的密码相同",
                PASSWORD_HISTORY_LIMIT
            )));
        }
        let password_plain = if password_changed {
            submitted_password
        } else {
            current.password.unwrap_or_default()
        };
        let notes_plain = p.notes.or(current.notes);

        let enc_pwd = self.encrypt_to_b64(&password_plain)?;
        let enc_notes = match notes_plain {
            Some(n) => Some(self.encrypt_to_b64(&n)?),
            None => None,
        };
        let now = Utc::now().to_rfc3339();
        let tags_str = tags.join(",");

        self.conn().execute(
            "UPDATE passwords SET site_name=?1, site_url=?2, username=?3, encrypted_password=?4,              encrypted_notes=?5, updated_at=?6, tags=?7, category=?8, priority=?9 WHERE id=?10",
            rusqlite::params![
                site_name, site_url, username, enc_pwd, enc_notes, now, tags_str, category, priority, p.id
            ],
        ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        if password_changed {
            let _ = self.record_password_history(p.id, &password_plain);
        }

        self.get_password(p.id)
    }

    /// 删除密码
    pub fn delete_password(&self, id: i64) -> Result<(), OperationError> {
        // 未开启 PRAGMA foreign_keys，需要显式清理历史记录，避免残留已删除条目的密文
        let _ = self
            .conn()
            .execute("DELETE FROM password_history WHERE password_id = ?1", rusqlite::params![id]);
        let affected = self.conn().execute("DELETE FROM passwords WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        if affected == 0 {
            return Err(OperationError::NotFound);
        }
        Ok(())
    }

    /// 数据库统计
    pub fn get_stats(&self) -> Result<crate::database::DatabaseStats, OperationError> {
        self.database.get_stats().map_err(|e| OperationError::DatabaseError(e.to_string()))
    }

    /// 完整性检查：SQLite 自检 + 数据库文件 SHA-256/大小。
    pub fn check_integrity(&self) -> Result<(bool, String, u64), OperationError> {
        let ok = self
            .database
            .check_integrity()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        let (checksum, size) = self
            .database
            .file_checksum()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        Ok((ok, checksum, size))
    }

    /// 新密码是否命中该条目最近的历史密码。
    ///
    /// 历史密码用主密钥加密存储，只有解锁后才能比对；即使拿到 db 文件，
    /// 也无法像旧的裸 SHA-256 那样离线爆破。
    fn is_password_reused(&self, password_id: i64, plaintext: &str) -> Result<bool, OperationError> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT encrypted_password FROM password_history \
                 WHERE password_id = ?1 AND encrypted_password IS NOT NULL",
            )
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![password_id], |r| r.get::<_, String>(0))
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        for row in rows {
            let stored = row.map_err(|e| OperationError::DatabaseError(e.to_string()))?;
            if self.decrypt_from_b64(&stored)? == plaintext {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn record_password_history(&self, password_id: i64, plaintext: &str) {
        let ts = Utc::now().to_rfc3339();
        // password_hash 是旧版本遗留列（NOT NULL），只写空串占位，不再存任何哈希。
        if let Ok(enc) = self.encrypt_to_b64(plaintext) {
            let _ = self.conn().execute(
                "INSERT INTO password_history (password_id, changed_at, password_hash, encrypted_password) \
                 VALUES (?1,?2,'',?3)",
                rusqlite::params![password_id, ts, enc],
            );
        }
        self.prune_password_history(password_id);
    }

    fn prune_password_history(&self, password_id: i64) {
        let _ = self.conn().execute(
            "DELETE FROM password_history WHERE password_id = ?1 AND id NOT IN (SELECT id FROM password_history WHERE password_id = ?1 ORDER BY changed_at DESC, id DESC LIMIT ?2)",
            rusqlite::params![password_id, PASSWORD_HISTORY_LIMIT as i64],
        );
    }

    pub fn password_age_days(updated_at: &str) -> Option<i64> {
        let dt = chrono::DateTime::parse_from_rfc3339(updated_at).ok()?;
        let now = Utc::now();
        let dur = now.signed_duration_since(dt.with_timezone(&Utc));
        Some(dur.num_days())
    }

    pub fn days_until_expiry(updated_at: &str, max_age_days: i64) -> i64 {
        let age = Self::password_age_days(updated_at).unwrap_or(0);
        max_age_days - age
    }

    pub fn is_password_expired(updated_at: &str, max_age_days: i64) -> bool {
        Self::password_age_days(updated_at).unwrap_or(0) > max_age_days
    }

    /// 当前主密钥的 salt（用于构造自包含的加密备份包）。
    pub fn get_salt(&self) -> Vec<u8> {
        self.crypto.get_salt().to_vec()
    }

    /// 当前主密钥派生迭代次数。
    pub fn get_iterations(&self) -> u32 {
        self.crypto.get_iterations()
    }

    pub fn policy() -> PasswordPolicy {
        PasswordPolicy {
            max_age_days: PASSWORD_MAX_AGE_DAYS,
            history_limit: PASSWORD_HISTORY_LIMIT as i64,
        }
    }

    /// 写入一条审计日志（best-effort，失败仅忽略，不阻断主流程）。
    pub fn add_audit_log(
        &self,
        action: &str,
        resource_type: Option<&str>,
        resource_id: Option<&str>,
        details: Option<&str>,
        success: bool,
        error_message: Option<&str>,
    ) {
        Self::write_audit_log(
            &self.database,
            action,
            resource_type,
            resource_id,
            details,
            success,
            error_message,
        );
    }

    /// 查询最近的审计日志（按时间倒序）。
    pub fn get_audit_logs(&self, limit: i64) -> Result<Vec<AuditLogEntry>, OperationError> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, timestamp, action, resource_type, resource_id, details, success, error_message \
                 FROM audit_logs ORDER BY timestamp DESC LIMIT ?1",
            )
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![limit], |r| {
                Ok(AuditLogEntry {
                    id: r.get("id")?,
                    timestamp: r.get("timestamp")?,
                    action: r.get("action")?,
                    resource_type: r.get("resource_type")?,
                    resource_id: r.get("resource_id")?,
                    details: r.get("details")?,
                    success: r.get("success")?,
                    error_message: r.get("error_message")?,
                })
            })
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?
            .collect::<rusqlite::Result<Vec<AuditLogEntry>>>()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        Ok(rows)
    }

    /// 删除早于 `days` 天的审计日志，返回删除条数（用于保留策略）。
    pub fn purge_old_audit_logs(&self, days: i64) -> Result<usize, OperationError> {
        let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        self.conn()
            .execute(
                "DELETE FROM audit_logs WHERE timestamp < ?1",
                rusqlite::params![cutoff],
            )
            .map_err(|e| OperationError::DatabaseError(e.to_string()))
    }

/// 直接向数据库写入审计日志（供尚未解锁、没有 `PasswordOperations` 的场景使用，如记录解锁失败）。
    pub fn write_audit_log(
    db: &Database,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    details: Option<&str>,
    success: bool,
    error_message: Option<&str>,
) {
    let ts = Utc::now().to_rfc3339();
    if let Err(e) = db.get_connection().execute(
        "INSERT INTO audit_logs (timestamp, action, resource_type, resource_id, details, success, error_message) \
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params![ts, action, resource_type, resource_id, details, success, error_message],
    ) {
        log::error("audit", &format!("写入审计日志失败: {}", e));
    }
}

    /// 读取全部条目的密文（不解密），用于主密码更换时的重加密。
    pub fn get_encrypted_rows(&self) -> Result<Vec<(i64, String, Option<String>)>, OperationError> {
        let mut stmt = self.conn().prepare(
            "SELECT id, encrypted_password, encrypted_notes FROM passwords",
        ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>("id")?,
                    r.get::<_, String>("encrypted_password")?,
                    r.get::<_, Option<String>>("encrypted_notes")?,
                ))
            })
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?
            .collect::<rusqlite::Result<Vec<(i64, String, Option<String>)>>>()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        Ok(rows)
    }

    /// 主密码更换时，用旧密钥解密每条目、用当前（新）密钥重新加密并写回。
    ///
    /// 整个重加密在单个事务里完成：任一条目失败都会整体回滚，
    /// 不会留下「一半旧密钥、一半新密钥」的库。
    pub fn rekey(&self, old_crypto: &SecureCryptoEngine) -> Result<(), OperationError> {
        let tx = self
            .conn()
            .unchecked_transaction()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;

        for (id, enc_pwd, enc_notes) in self.get_encrypted_rows()? {
            let old_ct = base64::engine::general_purpose::STANDARD
                .decode(&enc_pwd)
                .map_err(|_| OperationError::DecryptError)?;
            let plain = old_crypto
                .decrypt(&old_ct)
                .map_err(|_| OperationError::DecryptError)?;
            let new_ct = self
                .crypto
                .encrypt(&plain)
                .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
            let new_enc = base64::engine::general_purpose::STANDARD.encode(new_ct);

            let new_enc_notes = match enc_notes {
                Some(e) => {
                    let ct = base64::engine::general_purpose::STANDARD
                        .decode(&e)
                        .map_err(|_| OperationError::DecryptError)?;
                    let p = old_crypto
                        .decrypt(&ct)
                        .map_err(|_| OperationError::DecryptError)?;
                    let n = self
                        .crypto
                        .encrypt(p.as_slice())
                        .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
                    Some(base64::engine::general_purpose::STANDARD.encode(n))
                }
                None => None,
            };

            tx.execute(
                "UPDATE passwords SET encrypted_password=?1, encrypted_notes=?2 WHERE id=?3",
                rusqlite::params![new_enc, new_enc_notes, id],
            ).map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        }

        // 历史密码同样由主密钥加密，必须一起换成新密钥
        let history: Vec<(i64, String)> = {
            let mut stmt = tx
                .prepare(
                    "SELECT id, encrypted_password FROM password_history \
                     WHERE encrypted_password IS NOT NULL",
                )
                .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
                .map_err(|e| OperationError::DatabaseError(e.to_string()))?
                .collect::<rusqlite::Result<Vec<(i64, String)>>>()
                .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
            rows
        };
        for (id, enc) in history {
            let ct = base64::engine::general_purpose::STANDARD
                .decode(&enc)
                .map_err(|_| OperationError::DecryptError)?;
            let plain = old_crypto
                .decrypt(&ct)
                .map_err(|_| OperationError::DecryptError)?;
            let new_ct = self
                .crypto
                .encrypt(&plain)
                .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
            tx.execute(
                "UPDATE password_history SET encrypted_password=?1 WHERE id=?2",
                rusqlite::params![
                    base64::engine::general_purpose::STANDARD.encode(new_ct),
                    id
                ],
            )
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| OperationError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<RawRow> {
        Ok(RawRow {
            id: row.get("id")?,
            site_name: row.get("site_name")?,
            site_url: row.get("site_url")?,
            username: row.get("username")?,
            encrypted_password: row.get("encrypted_password")?,
            encrypted_notes: row.get("encrypted_notes")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            last_accessed: row.get("last_accessed")?,
            access_count: row.get("access_count")?,
            tags: row.get("tags")?,
            category: row.get("category")?,
            priority: row.get("priority")?,
            metadata: row.get("metadata")?,
            checksum: row.get("checksum")?,
        })
    }

    fn raw_to_entry(&self, r: RawRow) -> Result<PasswordEntry, OperationError> {
        let password = self.decrypt_from_b64(&r.encrypted_password)?;
        let notes = match r.encrypted_notes {
            Some(ref e) => Some(self.decrypt_from_b64(e)?),
            None => None,
        };
        let tags = r.tags
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        Ok(PasswordEntry {
            id: Some(r.id),
            site_name: r.site_name,
            site_url: r.site_url,
            username: r.username,
            encrypted_password: r.encrypted_password,
            encrypted_notes: r.encrypted_notes,
            created_at: r.created_at,
            updated_at: r.updated_at,
            last_accessed: r.last_accessed,
            access_count: r.access_count,
            tags,
            category: r.category,
            priority: r.priority,
            metadata: r.metadata,
            checksum: r.checksum,
            // 明文仅在解锁会话内返回给调用方
            password: Some(password),
            notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::MasterPasswordManager;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_path = dir.join(format!("securepass_test_{}_{}_ops.db", pid, n));
        let config_dir = dir.join(format!("securepass_cfg_{}_{}_ops", pid, n));
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&config_dir);
        std::fs::create_dir_all(&config_dir).expect("create config dir");
        (db_path, config_dir)
    }

    #[test]
    fn vault_flow_add_search_delete() {
        let (db_path, config_dir) = tmp();

        // 设置主密码
        let mut mgr = MasterPasswordManager::new(config_dir.to_str().unwrap());
        mgr.set_master_password("StrongPass123!").expect("set master password");

        let database = Database::new(&db_path).expect("open db");
        let crypto = mgr.get_crypto_engine().expect("crypto engine").clone();
        let ops = PasswordOperations::new(database, crypto);

        // 添加
        let added = ops.add_password(NewPassword {
            site_name: "github.com".into(),
            site_url: "https://github.com".into(),
            username: "alice".into(),
            password: "super-secret".into(),
            notes: Some("2FA app".into()),
            tags: Some(vec!["work".into(), "dev".into()]),
            category: Some("general".into()),
            priority: Some(1),
        }).expect("add password");

        assert_eq!(added.password.as_deref(), Some("super-secret"));
        assert_eq!(added.notes.as_deref(), Some("2FA app"));
        assert_eq!(added.username, "alice");

        // 列出
        let all = ops.get_passwords().expect("get all");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].password.as_deref(), Some("super-secret"));
        assert_eq!(all[0].tags, vec!["work".to_string(), "dev".to_string()]);

        // 搜索
        let found = ops.search_passwords("git").expect("search");
        assert_eq!(found.len(), 1);

        // 更新
        let updated = ops.update_password(UpdatePassword {
            id: added.id.unwrap(),
            site_name: None,
            site_url: None,
            username: Some("alice_new".into()),
            password: Some("new-secret".into()),
            notes: None,
            tags: None,
            category: None,
            priority: None,
        }).expect("update");
        assert_eq!(updated.username, "alice_new");
        assert_eq!(updated.password.as_deref(), Some("new-secret"));

        // 删除
        ops.delete_password(added.id.unwrap()).expect("delete");
        let after = ops.get_passwords().expect("get after delete");
        assert_eq!(after.len(), 0);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&config_dir);
    }

    #[test]
    fn password_history_blocks_reuse() {
        let (db_path, config_dir) = tmp();
        let mut mgr = MasterPasswordManager::new(config_dir.to_str().unwrap());
        mgr.set_master_password("StrongPass123!").expect("set");
        let database = Database::new(&db_path).expect("open");
        let crypto = mgr.get_crypto_engine().expect("crypto").clone();
        let ops = PasswordOperations::new(database, crypto);

        let added = ops.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "first-pass".into(),
            notes: None,
            tags: None,
            category: None,
            priority: None,
        }).expect("add");
        let id = added.id.unwrap();

        let ok = ops.update_password(UpdatePassword {
            id,
            site_name: None,
            site_url: None,
            username: None,
            password: Some("second-pass".into()),
            notes: None,
            tags: None,
            category: None,
            priority: None,
        });
        assert!(ok.is_ok(), "update to new password should succeed: {:?}", ok.err());

        let reuse = ops.update_password(UpdatePassword {
            id,
            site_name: None,
            site_url: None,
            username: None,
            password: Some("first-pass".into()),
            notes: None,
            tags: None,
            category: None,
            priority: None,
        });
        assert!(
            matches!(reuse, Err(OperationError::ValidationError(_))),
            "reusing a historical password must be blocked, got {:?}",
            reuse.map(|e| e.id)
        );

        // 历史密码必须以密文存储：不能再是可被离线爆破的裸 SHA-256
        let stored: String = ops
            .conn()
            .query_row(
                "SELECT encrypted_password FROM password_history WHERE password_id = ?1 LIMIT 1",
                rusqlite::params![id],
                |r| r.get(0),
            )
            .expect("history row");
        assert!(
            stored.chars().any(|c| !c.is_ascii_hexdigit()),
            "历史密码不能以裸哈希存储: {}",
            stored
        );
    }

    #[test]
    fn audit_log_retention_purges_old() {
        let (db_path, config_dir) = tmp();
        let mut mgr = MasterPasswordManager::new(config_dir.to_str().unwrap());
        mgr.set_master_password("StrongPass123!").expect("set");
        let database = Database::new(&db_path).expect("open");
        let crypto = mgr.get_crypto_engine().expect("crypto").clone();
        let ops = PasswordOperations::new(database, crypto);

        let old = (Utc::now() - chrono::Duration::days(120)).to_rfc3339();
        let recent = Utc::now().to_rfc3339();
        ops.conn()
            .execute(
                "INSERT INTO audit_logs (timestamp, action, success) VALUES (?1, 'old_action', 1)",
                rusqlite::params![old],
            )
            .unwrap();
        ops.conn()
            .execute(
                "INSERT INTO audit_logs (timestamp, action, success) VALUES (?1, 'recent_action', 1)",
                rusqlite::params![recent],
            )
            .unwrap();

        let removed = ops.purge_old_audit_logs(90).expect("purge");
        assert_eq!(removed, 1);
        let logs = ops.get_audit_logs(100).expect("logs");
        assert!(logs.iter().all(|l| l.action != "old_action"));
        assert!(logs.iter().any(|l| l.action == "recent_action"));
    }

    #[test]
    fn expiry_helpers() {
        let now = Utc::now().to_rfc3339();
        assert!(!PasswordOperations::is_password_expired(&now, 90));
        let old = (Utc::now() - chrono::Duration::days(120)).to_rfc3339();
        assert!(PasswordOperations::is_password_expired(&old, 90));
        assert!(PasswordOperations::days_until_expiry(&old, 90) <= 0);
        let policy = PasswordOperations::policy();
        assert_eq!(policy.max_age_days, 90);
        assert_eq!(policy.history_limit, 5);
    }

    #[cfg(unix)]
    #[test]
    fn db_file_permissions_are_private() {
        let (db_path, config_dir) = tmp();
        let _ = Database::new(&db_path).expect("open db");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&db_path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "db file should be 0600");
        }
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir_all(&config_dir);
    }
}
