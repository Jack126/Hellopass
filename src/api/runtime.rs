use std::path::PathBuf;
use std::collections::{HashMap, HashSet};
use chrono::{Duration, DateTime, Utc};
use serde::{Serialize, Deserialize};
use base64::Engine;
use zeroize::Zeroizing;
use crate::api::dto::*;
use crate::database::Database;
use crate::database::operations::{PasswordOperations, OperationError};
use crate::security::MasterPasswordManager;
use crate::security::crypto::SecureCryptoEngine;
use crate::security::policies::{validate_password_strength, PasswordStrengthResult};
use crate::utils::log;
use crate::error::{AppError, AppResult};

/// 把 AppResult 统一转换为 ApiResponse，并记录错误日志。
fn to_response<T>(result: AppResult<T>) -> ApiResponse<T> {
    match result {
        Ok(v) => ApiResponse::ok(v),
        Err(e) => {
            log::error("runtime", &e.to_string());
            ApiResponse::err_message(e.to_string())
        }
    }
}

/// 审计日志默认保留天数（启动器每次复制都会写一条，不宜留太久）。
const AUDIT_RETENTION_DAYS: i64 = 30;
/// 审计日志最大条数，超出后删最旧的。
const AUDIT_MAX_ROWS: i64 = 1000;
/// 导入文件大小上限（字节）：明文 JSON/CSV 与加密备份包都适用。
const MAX_IMPORT_BYTES: usize = 5 * 1024 * 1024;
/// 单次导入的条目数上限。
const MAX_IMPORT_ENTRIES: usize = 5000;

/// 失败计数与锁定截止时间：需要落盘，否则重启应用就能重置 5 次失败锁定。
#[derive(Serialize, Deserialize, Default)]
struct LockoutState {
    failed_attempts: u32,
    lockout_until: Option<String>,
}

/// 保险库运行时：管理主密码设置/解锁状态，并持有已解密的保险库（PasswordOperations）。
///
/// 该模块不依赖 Tauri，可直接在单元测试中用临时目录验证完整流程
/// （设置主密码 → 解锁 → 增删查改 → 修改主密码并重加密）。
pub struct VaultRuntime {
    vault: Option<PasswordOperations>,
    config_dir: PathBuf,
    db_path: PathBuf,
    // 会话安全状态
    failed_attempts: u32,
    lockout_until: Option<DateTime<Utc>>,
    last_activity: Option<DateTime<Utc>>,
    session_timeout: Duration,
    max_attempts: u32,
    lockout_duration: Duration,
    // 上次正常锁定写入的数据库基线校验结果（None 表示一致或无基线）
    integrity_warning: Option<String>,
}

impl VaultRuntime {
    pub fn new(config_dir: PathBuf, db_path: PathBuf) -> Self {
        Self {
            vault: None,
            config_dir,
            db_path,
            failed_attempts: 0,
            lockout_until: None,
            last_activity: None,
            // 启动器形态下默认不自动锁定；设置窗里可改为 5/15/30 分钟
            session_timeout: Duration::zero(),
            max_attempts: 5,
            lockout_duration: Duration::minutes(30),
            integrity_warning: None,
        }
    }

    fn dir(&self) -> &str {
        self.config_dir.to_str().unwrap_or(".")
    }

    fn lockout_state_path(&self) -> PathBuf {
        self.config_dir.join("lockout_state.json")
    }

    /// 从磁盘恢复失败计数与锁定截止时间（重启应用不能绕过锁定）。
    fn load_lockout_state(&mut self) {
        let Ok(content) = std::fs::read_to_string(self.lockout_state_path()) else {
            return;
        };
        let Ok(state) = serde_json::from_str::<LockoutState>(&content) else {
            return;
        };
        self.failed_attempts = state.failed_attempts;
        self.lockout_until = state
            .lockout_until
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));
    }

    fn save_lockout_state(&self) {
        let state = LockoutState {
            failed_attempts: self.failed_attempts,
            lockout_until: self.lockout_until.map(|t| t.to_rfc3339()),
        };
        if let Ok(json) = serde_json::to_string(&state) {
            let path = self.lockout_state_path();
            let tmp = path.with_extension("json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    fn clear_lockout_state(&self) {
        let _ = std::fs::remove_file(self.lockout_state_path());
    }

    /// 是否已设置主密码（依据配置文件是否存在）。
    pub fn is_set(&self) -> bool {
        MasterPasswordManager::new(self.dir()).is_set()
    }

    /// 首次设置主密码：派生密钥、打开数据库、建立保险库。
    pub fn setup(&mut self, master_password: &str) -> ApiResponse<()> {
        to_response(self.try_setup(master_password))
    }

    fn try_setup(&mut self, master_password: &str) -> AppResult<()> {
        if self.is_set() {
            return Err(AppError::Message("主密码已设置，无法重复设置".to_string()));
        }
        let mut mgr = MasterPasswordManager::new(self.dir());
        mgr.set_master_password(master_password)?;
        let crypto = mgr.get_crypto_engine()?.clone();
        let database = Database::new(&self.db_path)?;
        self.vault = Some(PasswordOperations::new(database, crypto));
        self.clear_lockout_state();
        Ok(())
    }

    /// 用主密码解锁：验证密码正确性，重建同一密钥的保险库。
    /// 失败次数达到上限后触发锁定（lockout_duration）；锁定期间拒绝解锁。
    pub fn unlock(&mut self, master_password: &str) -> ApiResponse<()> {
        if !self.is_set() {
            return ApiResponse::err_message("主密码尚未设置");
        }
        // 先从磁盘恢复失败计数/锁定截止时间，重启应用不能绕过锁定
        self.load_lockout_state();
        // 已处于锁定状态
        if let Some(t) = self.lockout_until {
            if Utc::now() < t {
                return ApiResponse::err_message(format!(
                    "账户已锁定，请于 {} 后重试",
                    t.to_rfc3339()
                ));
            }
            // 锁定已过期，重置失败计数允许重试
            self.lockout_until = None;
            self.failed_attempts = 0;
            self.clear_lockout_state();
        }
        // 完整性基线校验（在打开数据库前读取文件，避免受连接影响）
        let baseline_warning = self.verify_baseline();

        // 提前打开数据库，以便记录（含失败的）解锁尝试
        let db = match Database::new(&self.db_path) {
            Ok(d) => d,
            Err(e) => return ApiResponse::err_message(format!("打开数据库失败: {}", e)),
        };
        let mut mgr = MasterPasswordManager::new(self.dir());
        let verify = mgr.verify_password(master_password);
        match &verify {
            Ok(_) => PasswordOperations::write_audit_log(
                &db,
                "unlock",
                Some("session"),
                None,
                None,
                true,
                None,
            ),
            Err(e) => {
                let msg = e.to_string();
                PasswordOperations::write_audit_log(
                    &db,
                    "unlock",
                    Some("session"),
                    None,
                    None,
                    false,
                    Some(&msg),
                );
            },
        }
        match verify {
            Ok(_) => {
                let crypto = match mgr.get_crypto_engine() {
                    Ok(c) => c.clone(),
                    Err(e) => return ApiResponse::err_message(format!("获取密钥失败: {}", e)),
                };
                self.failed_attempts = 0;
                self.lockout_until = None;
                self.clear_lockout_state();
                self.vault = Some(PasswordOperations::new(db, crypto));
                self.last_activity = Some(Utc::now());
                // 解锁成功后按保留策略清理过期/超量审计日志（best-effort）
                if let Some(v) = &self.vault {
                    let _ = v.purge_old_audit_logs(AUDIT_RETENTION_DAYS);
                    let _ = v.prune_audit_logs(AUDIT_MAX_ROWS);
                }
                if let Some(w) = &baseline_warning {
                    log::error("integrity", w);
                }
                self.integrity_warning = baseline_warning;
                ApiResponse::ok(())
            }
            Err(e) => {
                self.failed_attempts += 1;
                if self.failed_attempts >= self.max_attempts {
                    self.lockout_until = Some(Utc::now() + self.lockout_duration);
                }
                self.save_lockout_state();
                log::warn("runtime", &format!("主密码验证失败（第 {} 次）", self.failed_attempts));
                ApiResponse::err_message(format!("主密码错误: {}", e))
            }
        }
    }

    /// 锁定：记录审计并丢弃内存中的保险库。
    pub fn lock(&mut self) {
        self.vault.as_ref().map(|v| {
            v.add_audit_log("lock", Some("session"), None, None, true, None)
        });
        let was_open = self.vault.is_some();
        self.vault = None;
        self.last_activity = None;
        if was_open {
            // 正常锁定后写入数据库文件基线，供下次解锁时比对
            self.write_baseline();
        }
    }

    /// 修改主密码：验证旧密码，设置新主密码（新 salt），并用旧密钥解密后
    /// 以新密钥重新加密全部条目，保证历史数据可被新密码解密。
    pub fn change_password(&mut self, old_password: &str, new_password: &str) -> ApiResponse<()> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.try_change_password(old_password, new_password);
        if result.is_ok() {
            self.refresh_session();
        }
        to_response(result)
    }

    fn try_change_password(&mut self, old_password: &str, new_password: &str) -> AppResult<()> {
        let dir = self.dir();
        let mut old_mgr = MasterPasswordManager::new(dir);
        old_mgr.verify_password(old_password)?;
        let old_crypto = old_mgr.get_crypto_engine()?.clone();

        if !MasterPasswordManager::password_is_strong(new_password) {
            return Err(AppError::Message(
                "主密码强度不足：至少 12 位，且需包含大小写字母、数字和特殊字符".to_string(),
            ));
        }

        // 新密钥先只派生在内存里（随机 salt），数据库全部重加密成功后才落盘配置。
        // 顺序很关键：配置一旦先写，旧密码立刻失效，任何中途失败都会把库锁死。
        let new_crypto = SecureCryptoEngine::new(new_password, None)?;
        let database = Database::new(&self.db_path)?;
        let ops = PasswordOperations::new(database, new_crypto.clone());
        ops.rekey(&old_crypto)?; // 单事务：失败则整体回滚，旧密码仍可打开

        // 最后一步原子替换配置；失败则把数据重加密回旧密钥（磁盘上仍是旧配置）
        if let Err(e) = MasterPasswordManager::new(dir).rotate_to(new_crypto.clone()) {
            let rollback = Database::new(&self.db_path)
                .map_err(AppError::from)
                .and_then(|db| {
                    PasswordOperations::new(db, old_crypto.clone())
                        .rekey(&new_crypto)
                        .map_err(AppError::from)
                });
            if let Err(rb) = rollback {
                log::error(
                    "runtime",
                    &format!("改密回滚失败，数据库可能处于混合密钥状态: {}", rb),
                );
            }
            return Err(e.into());
        }

        self.failed_attempts = 0;
        self.lockout_until = None;
        self.clear_lockout_state();
        self.vault = Some(ops);
        self.last_activity = Some(Utc::now());
        if let Some(v) = &self.vault {
            v.add_audit_log("change_master_password", Some("session"), None, None, true, None);
        }
        Ok(())
    }

    /// 校验当前会话是否有效：未解锁或空闲超时则重新锁定并返回错误响应。
    fn enforce_session(&mut self) -> bool {
        if self.vault.is_none() {
            return false;
        }
        if self.session_timeout > Duration::zero() {
            if let Some(t) = self.last_activity {
                if Utc::now() - t > self.session_timeout {
                    self.vault = None;
                    self.last_activity = None;
                    return false;
                }
            }
        }
        true
    }

    /// 成功操作后刷新最近活跃时间，保持会话存活。
    fn refresh_session(&mut self) {
        if self.vault.is_some() {
            self.last_activity = Some(Utc::now());
        }
    }

    /// 调整空闲超时（秒），由前端「自动锁定」设置同步进来。
    /// 0 表示不自动锁定；其余下限 10 秒、上限 300 秒，防止误设成过短或过长。
    pub fn set_session_timeout(&mut self, timeout_seconds: i64) {
        self.session_timeout = if timeout_seconds <= 0 {
            Duration::zero()
        } else {
            Duration::seconds(timeout_seconds.clamp(10, 300))
        };
    }

    /// 配置会话安全参数（用于测试或按策略调整）。
    pub fn set_session_limits(&mut self, timeout_minutes: i64, max_attempts: u32, lockout_minutes: i64) {
        self.session_timeout = Duration::minutes(timeout_minutes);
        self.max_attempts = max_attempts;
        self.lockout_duration = Duration::minutes(lockout_minutes);
    }

    /// 读取审计日志（需在解锁会话内）。
    pub fn get_audit_logs(&self) -> ApiResponse<Vec<AuditLogEntry>> {
        match &self.vault {
            Some(v) => match v.get_audit_logs(200) {
                Ok(logs) => ApiResponse::ok(logs),
                Err(e) => ApiResponse::err_message(e.to_string()),
            },
            None => locked(),
        }
    }

    /// 手动触发审计日志清理（保留最近 days 天），返回删除条数。
    pub fn purge_old_audit_logs(&mut self, days: i64) -> ApiResponse<usize> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.purge_old_audit_logs(days));
        if result.success {
            self.refresh_session();
        }
        result
    }

    /// 完整性检查：SQLite 自检 + 数据库/配置文件 SHA-256。
    pub fn check_integrity(&mut self) -> ApiResponse<IntegrityReport> {
        if !self.enforce_session() {
            return locked();
        }
        let config_checksum = self.config_checksum();
        // 基线只在解锁前比对一次：解锁本身会写审计日志，会话内再比对必然不一致。
        let baseline_ok = self.integrity_warning.is_none();
        let result = self.with_vault(|v| {
            let (sqlite_ok, db_checksum, db_size) = v.check_integrity()?;
            Ok(IntegrityReport {
                sqlite_ok,
                db_checksum,
                db_size,
                config_checksum,
                baseline_ok,
            })
        });
        if result.success {
            self.refresh_session();
        }
        result
    }

    fn config_checksum(&self) -> Option<String> {
        use sha2::{Sha256, Digest};
        let path = self.config_dir.join("master_password_config.json");
        let data = std::fs::read(path).ok()?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Some(format!("{:x}", hasher.finalize()))
    }

    fn baseline_path(&self) -> PathBuf {
        PathBuf::from(format!("{}.integrity", self.db_path.display()))
    }

    fn file_sha256(path: &std::path::Path) -> Option<String> {
        use sha2::{Sha256, Digest};
        let data = std::fs::read(path).ok()?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Some(format!("{:x}", hasher.finalize()))
    }

    fn write_baseline(&self) {
        if let Some(sum) = Self::file_sha256(&self.db_path) {
            let _ = std::fs::write(self.baseline_path(), sum);
        }
    }

    /// 返回基线不一致的告警；无基线或一致时返回 None。
    fn verify_baseline(&self) -> Option<String> {
        let expected = std::fs::read_to_string(self.baseline_path()).ok()?;
        let expected = expected.trim().to_string();
        if expected.is_empty() {
            return None;
        }
        let actual = Self::file_sha256(&self.db_path)?;
        if actual == expected {
            None
        } else {
            let e = &expected[..expected.len().min(12)];
            let a = &actual[..actual.len().min(12)];
            Some(format!("数据库文件完整性校验失败：基线 {}，当前 {}", e, a))
        }
    }

    /// 最近一次解锁时的完整性告警（None 表示一致或无基线）。
    pub fn integrity_warning(&self) -> Option<&str> {
        self.integrity_warning.as_deref()
    }

    /// 把当前数据库文件记为新的可信基线（用于确认「不一致」是正常改动后手动消除告警）。
    pub fn rebuild_baseline(&mut self) -> ApiResponse<()> {
        if !self.enforce_session() {
            return locked();
        }
        self.write_baseline();
        self.integrity_warning = None;
        if let Some(v) = &self.vault {
            v.add_audit_log("rebuild_baseline", Some("integrity"), None, None, true, None);
        }
        self.refresh_session();
        ApiResponse::ok(())
    }

    fn with_vault<T, F>(&self, f: F) -> ApiResponse<T>
    where
        F: FnOnce(&PasswordOperations) -> Result<T, OperationError>,
    {
        match &self.vault {
            Some(v) => match f(v) {
                Ok(data) => ApiResponse::ok(data),
                Err(e) => {
                    let app: AppError = e.into();
                    log::error("runtime", &app.to_string());
                    ApiResponse::err_message(app.to_string())
                }
            },
            None => locked(),
        }
    }

    pub fn add_password(&mut self, password: NewPassword) -> ApiResponse<PasswordEntry> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.add_password(password));
        if result.success {
            self.refresh_session();
            if let Some(entry) = &result.data {
                let rid = entry.id.map(|i| i.to_string());
                self.vault.as_ref().map(|v| {
                    v.add_audit_log("add_password", Some("password"), rid.as_deref(), None, true, None)
                });
            }
        }
        result
    }

    pub fn get_passwords(&mut self) -> ApiResponse<Vec<PasswordEntry>> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.get_passwords());
        if result.success {
            self.refresh_session();
        }
        result
    }

    pub fn get_password(&mut self, id: i64) -> ApiResponse<PasswordEntry> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.get_password(id));
        if result.success {
            self.refresh_session();
        }
        result
    }

    pub fn search_passwords(&mut self, query: SearchQuery) -> ApiResponse<Vec<PasswordEntry>> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.search_passwords(&query.query));
        if result.success {
            self.refresh_session();
        }
        result
    }

    /// 启动器检索用摘要：只含元数据，不含任何明文/密文密码。
    pub fn list_entry_summaries(&mut self) -> ApiResponse<Vec<EntrySummary>> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.list_summaries());
        if result.success {
            self.refresh_session();
        }
        result
    }

    /// 当前是否处于解锁会话（未解锁或空闲超时都算锁定）。
    pub fn is_unlocked(&mut self) -> bool {
        self.enforce_session()
    }

    /// 取出要写入剪贴板的字段；返回值由调用方写入剪贴板后立即丢弃。
    /// 这是唯一会产出密码明文的复制入口，明文不经过前端。
    pub fn secret_for_copy(&mut self, id: i64, field: &str) -> AppResult<Zeroizing<String>> {
        if !self.enforce_session() {
            return Err(AppError::Message("保险库已锁定，请先解锁主密码".to_string()));
        }
        let value = {
            let vault = self
                .vault
                .as_ref()
                .ok_or_else(|| AppError::Message("保险库已锁定，请先解锁主密码".to_string()))?;
            let entry = vault.get_password(id)?;
            let (value, action) = match field {
                "password" => (entry.password.clone().unwrap_or_default(), "copy_password"),
                "username" => (entry.username.clone(), "copy_username"),
                _ => return Err(AppError::Message("不支持的复制字段".to_string())),
            };
            vault.add_audit_log(action, Some("password"), Some(&id.to_string()), None, true, None);
            Zeroizing::new(value)
        };
        self.refresh_session();
        Ok(value)
    }

    /// 详情面板：返回备注等字段，但不含密码（密码走 reveal_password）。
    pub fn entry_detail(&mut self, id: i64) -> AppResult<EntryDetail> {
        if !self.enforce_session() {
            return Err(AppError::Message("保险库已锁定，请先解锁主密码".to_string()));
        }
        let detail = {
            let vault = self
                .vault
                .as_ref()
                .ok_or_else(|| AppError::Message("保险库已锁定，请先解锁主密码".to_string()))?;
            let e = vault.get_password(id)?;
            EntryDetail {
                id: e.id.unwrap_or(id),
                site_name: e.site_name,
                site_url: e.site_url,
                username: e.username,
                notes: e.notes,
                tags: e.tags,
                created_at: e.created_at,
                updated_at: e.updated_at,
                last_accessed: e.last_accessed,
                access_count: e.access_count,
            }
        };
        self.refresh_session();
        Ok(detail)
    }

    /// 详情页显式查看密码：唯一向界面返回明文密码的入口，调用方须在收起时丢弃。
    pub fn reveal_password(&mut self, id: i64) -> AppResult<Zeroizing<String>> {
        if !self.enforce_session() {
            return Err(AppError::Message("保险库已锁定，请先解锁主密码".to_string()));
        }
        let value = {
            let vault = self
                .vault
                .as_ref()
                .ok_or_else(|| AppError::Message("保险库已锁定，请先解锁主密码".to_string()))?;
            let entry = vault.get_password(id)?;
            vault.add_audit_log("reveal_password", Some("password"), Some(&id.to_string()), None, true, None);
            Zeroizing::new(entry.password.clone().unwrap_or_default())
        };
        self.refresh_session();
        Ok(value)
    }

    pub fn update_password(&mut self, password: UpdatePassword) -> ApiResponse<PasswordEntry> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.update_password(password));
        if result.success {
            self.refresh_session();
            if let Some(entry) = &result.data {
                let rid = entry.id.map(|i| i.to_string());
                self.vault.as_ref().map(|v| {
                    v.add_audit_log("update_password", Some("password"), rid.as_deref(), None, true, None)
                });
            }
        }
        result
    }

    pub fn delete_password(&mut self, id: i64) -> ApiResponse<()> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.delete_password(id));
        if result.success {
            self.refresh_session();
            self.vault.as_ref().map(|v| {
                v.add_audit_log("delete_password", Some("password"), Some(&id.to_string()), None, true, None)
            });
        }
        result
    }

    /// 清空所有密码条目（含历史密码），返回删除条数。
    /// 审计日志保留，并记录本次清空操作。
    pub fn clear_all_passwords(&mut self) -> ApiResponse<usize> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| v.delete_all_passwords());
        if result.success {
            self.refresh_session();
            if let Some(n) = result.data {
                let details = format!("清空 {} 条密码", n);
                self.vault.as_ref().map(|v| {
                    v.add_audit_log(
                        "clear_all_passwords",
                        Some("password"),
                        None,
                        Some(&details),
                        true,
                        None,
                    )
                });
            }
        }
        result
    }

    pub fn get_password_stats(&mut self) -> ApiResponse<PasswordStats> {
        if !self.enforce_session() {
            return locked();
        }
        self.with_vault(|v| {
            let stats = v.get_stats()?;
            Ok(PasswordStats {
                total_count: stats.password_count,
                tag_counts: Default::default(),
                recent_access_count: 0,
            })
        })
    }

    /// 评估给定密码的强度（无需解锁，供前端表单实时反馈）。
    pub fn check_password_strength(&self, password: &str) -> ApiResponse<PasswordStrengthResult> {
        ApiResponse::ok(validate_password_strength(password))
    }

    pub fn get_password_policy(&self) -> ApiResponse<PasswordPolicy> {
        ApiResponse::ok(PasswordOperations::policy())
    }

    pub fn get_expiring_passwords(&mut self, within_days: i64) -> ApiResponse<Vec<PasswordExpiryInfo>> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| {
            let entries = v.get_passwords()?;
            let policy = PasswordOperations::policy();
            let mut out: Vec<PasswordExpiryInfo> = Vec::new();
            for e in entries {
                let due = PasswordOperations::days_until_expiry(&e.updated_at, policy.max_age_days);
                if due <= within_days {
                    out.push(PasswordExpiryInfo {
                        id: e.id,
                        site_name: e.site_name,
                        days_until_expiry: due,
                        expired: due <= 0,
                    });
                }
            }
            Ok(out)
        });
        if result.success {
            self.refresh_session();
        }
        result
    }


    /// 导入密码（自动识别 JSON 或 CSV）。返回导入统计。
    pub fn import_passwords(&mut self, data: String) -> ApiResponse<ImportResult> {
        if !self.enforce_session() {
            return locked();
        }
        let vault = match &self.vault {
            Some(v) => v,
            None => return locked(),
        };
        let result = do_import(vault, &data);
        if result.success {
            self.refresh_session();
        }
        result
    }

    /// 导出加密备份：条目序列化为 JSON，用当前主密钥加密，并附带 salt/iterations
    /// 组成自包含备份包（可在任意安装上用同一主密码恢复）。
    pub fn export_backup(&mut self) -> ApiResponse<String> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.with_vault(|v| {
            let entries = v.get_passwords()?;
            let json = serde_json::to_vec(&entries)
                .map_err(|e| OperationError::DatabaseError(format!("序列化失败: {}", e)))?;
            let data = v.encrypt_blob(&json)?;
            let pkg = BackupPackage {
                version: "1.0".to_string(),
                iterations: v.get_iterations(),
                salt: base64::engine::general_purpose::STANDARD.encode(v.get_salt()),
                data,
            };
            serde_json::to_string(&pkg)
                .map_err(|e| OperationError::DatabaseError(format!("序列化失败: {}", e)))
        });
        if result.success {
            self.refresh_session();
        }
        result
    }

    /// 加密导出（单独导出密码）：用「导出密码」派生密钥（随机 salt，PBKDF2 600k 次），
    /// 把所有条目序列化为 JSON 后做 AES-256-GCM 加密，输出自包含加密包。
    /// 该文件与保险库主密码无关——只有知道导出密码的人才能解密，适合跨设备/外部分发。
    pub fn export_encrypted(
        &mut self,
        export_password: &str,
        ids: Option<Vec<i64>>,
    ) -> ApiResponse<String> {
        if !self.enforce_session() {
            return locked();
        }
        if export_password.len() < 8 {
            return ApiResponse::err_message("导出密码至少 8 位");
        }
        let result = self.with_vault(|v| {
            let entries = v.get_passwords()?;
            // ids = None 导出全部；Some(list) 只导出选中项
            let entries: Vec<PasswordEntry> = match ids {
                Some(list) => {
                    let set: HashSet<i64> = list.into_iter().collect();
                    entries
                        .into_iter()
                        .filter(|e| e.id.map(|i| set.contains(&i)).unwrap_or(false))
                        .collect()
                }
                None => entries,
            };
            if entries.is_empty() {
                return Err(OperationError::ValidationError(
                    "请至少选择一条要导出的条目".to_string(),
                ));
            }
            let json = serde_json::to_vec(&entries)
                .map_err(|e| OperationError::DatabaseError(format!("序列化失败: {}", e)))?;
            let engine = SecureCryptoEngine::new(export_password, None)
                .map_err(|_| OperationError::DatabaseError("密钥派生失败".to_string()))?;
            let blob = engine.encrypt(&json)
                .map_err(|_| OperationError::DatabaseError("加密失败".to_string()))?;
            let pkg = EncryptedExportPackage {
                version: "2.0".to_string(),
                kdf: "PBKDF2-HMAC-SHA256".to_string(),
                iterations: engine.get_iterations(),
                salt: base64::engine::general_purpose::STANDARD.encode(engine.get_salt()),
                data: base64::engine::general_purpose::STANDARD.encode(&blob),
            };
            serde_json::to_string(&pkg)
                .map_err(|e| OperationError::DatabaseError(format!("序列化失败: {}", e)))
        });
        if result.success {
            self.refresh_session();
        }
        result
    }

    /// 加密导入（单独导出密码）：用导出密码 + 备份内嵌的 salt/iterations 重新派生密钥解密，
    /// 得到条目 JSON 后按本地主密码重新加密写入数据库（数据库始终为密文）。
    pub fn import_encrypted(&mut self, data: String, export_password: String) -> ApiResponse<ImportResult> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.try_import_encrypted(data, export_password);
        if result.is_ok() {
            self.refresh_session();
        }
        to_response(result)
    }

    fn try_import_encrypted(&mut self, data: String, export_password: String) -> AppResult<ImportResult> {
        if data.len() > MAX_IMPORT_BYTES {
            return Err(AppError::Message(format!(
                "导入文件过大：上限 {} MB",
                MAX_IMPORT_BYTES / 1024 / 1024
            )));
        }
        let pkg: EncryptedExportPackage = serde_json::from_str(&data)
            .map_err(|e| AppError::Message(format!("备份格式错误: {}", e)))?;
        if pkg.version != "2.0" {
            return Err(AppError::Message("不支持的备份版本".to_string()));
        }
        let salt = base64::engine::general_purpose::STANDARD
            .decode(&pkg.salt)
            .map_err(|_| AppError::Message("备份 salt 损坏".to_string()))?;
        let engine = SecureCryptoEngine::new_with_iterations(
            &export_password,
            Some(&salt),
            pkg.iterations,
        )?;
        let blob = base64::engine::general_purpose::STANDARD
            .decode(&pkg.data)
            .map_err(|_| AppError::Message("备份数据损坏".to_string()))?;
        let plain = Zeroizing::new(
            engine
                .decrypt(&blob)
                .map_err(|_| AppError::Message("导出密码错误或数据被篡改".to_string()))?,
        );
        let entries: Vec<PasswordEntry> = serde_json::from_slice(plain.as_slice())?;
        let vault = self
            .vault
            .as_ref()
            .ok_or_else(|| AppError::Message("保险库已锁定".to_string()))?;
        let items: Vec<NewPassword> = entries.iter().map(entry_to_new).collect();
        apply_import(vault, items).map_err(AppError::Message)
    }

    /// 从加密备份恢复：用备份内嵌的 salt/iterations 与提供的主密码重新派生密钥，
    /// 解密后按 JSON 条目导入（按 站点名+用户名 去重）。
    pub fn restore_backup(&mut self, data: String, master_password: String) -> ApiResponse<ImportResult> {
        if !self.enforce_session() {
            return locked();
        }
        let result = self.try_restore_backup(data, master_password);
        if result.is_ok() {
            self.refresh_session();
        }
        to_response(result)
    }

    fn try_restore_backup(&mut self, data: String, master_password: String) -> AppResult<ImportResult> {
        if data.len() > MAX_IMPORT_BYTES {
            return Err(AppError::Message(format!(
                "备份文件过大：上限 {} MB",
                MAX_IMPORT_BYTES / 1024 / 1024
            )));
        }
        let pkg: BackupPackage = serde_json::from_str(&data)
            .map_err(|e| AppError::Message(format!("备份格式错误: {}", e)))?;
        if pkg.version != "1.0" {
            return Err(AppError::Message("不支持的备份版本".to_string()));
        }
        let salt = base64::engine::general_purpose::STANDARD
            .decode(&pkg.salt)
            .map_err(|_| AppError::Message("备份 salt 损坏".to_string()))?;
        let engine = SecureCryptoEngine::new_with_iterations(
            &master_password,
            Some(salt.as_slice()),
            pkg.iterations,
        )?;
        let ct = base64::engine::general_purpose::STANDARD
            .decode(&pkg.data)
            .map_err(|_| AppError::Message("备份数据损坏".to_string()))?;
        let plain = Zeroizing::new(
            engine
                .decrypt(&ct)
                .map_err(|_| AppError::Message("备份密码错误或数据被篡改".to_string()))?,
        );
        let entries: Vec<PasswordEntry> = serde_json::from_slice(plain.as_slice())?;
        let vault = self
            .vault
            .as_ref()
            .ok_or_else(|| AppError::Message("保险库已锁定".to_string()))?;
        let items: Vec<NewPassword> = entries.iter().map(entry_to_new).collect();
        apply_import(vault, items).map_err(AppError::Message)
    }
}


/// 单独密码加密的导出自包含包：版本 + KDF 信息 + 迭代次数 + salt(base64) + 密文(base64)。
/// 密文由 SecureCryptoEngine.encrypt 产出（nonce 已拼接到密文前）。
#[derive(Serialize, Deserialize)]
struct EncryptedExportPackage {
    version: String,
    kdf: String,
    iterations: u32,
    salt: String,
    data: String,
}

/// 自包含加密备份包：版本 + 迭代次数 + salt(base64) + 密文(base64)。
#[derive(Serialize, Deserialize)]
struct BackupPackage {
    version: String,
    iterations: u32,
    salt: String,
    data: String,
}

/// 导入辅助结构：CSV 解析后的单条记录。
struct ImportEntry {
    site_name: String,
    site_url: String,
    username: String,
    password: String,
    notes: Option<String>,
    tags: Option<String>,
}

/// 按 CSV 规则切分整个文档：支持引号包裹的逗号、换行与 "" 转义。
/// 浏览器导出的备注里可能带换行，不能按行切分。
fn split_csv_records(data: &str) -> Vec<Vec<String>> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = data.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes {
                    if chars.peek() == Some(&'"') {
                        field.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => record.push(std::mem::take(&mut field)),
            '\r' if !in_quotes => {
                // CRLF：吃掉 \n，并按换行结束当前记录
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            '\n' if !in_quotes => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

/// 在表头中查找列下标（忽略大小写），支持多个别名。
fn find_column(headers: &[String], names: &[&str]) -> Option<usize> {
    headers
        .iter()
        .position(|h| names.iter().any(|n| h.eq_ignore_ascii_case(n)))
}

/// 解析 CSV 导入数据（首行为表头）。
/// 兼容本程序导出（site_name / site_url / notes）与浏览器导出（name / url / note），
/// 表头忽略大小写与首尾空白，首行 BOM 会被去掉。
fn parse_csv_import(data: &str) -> Result<Vec<ImportEntry>, String> {
    let mut records = split_csv_records(data.trim_start_matches('\u{feff}'))
        .into_iter()
        .filter(|r| !r.iter().all(|f| f.trim().is_empty()));
    let header = match records.next() {
        Some(h) => h,
        None => return Err("CSV 内容为空".to_string()),
    };
    let headers: Vec<String> = header.iter().map(|h| h.trim().to_string()).collect();

    let name_col = find_column(&headers, &["site_name", "name", "title"])
        .ok_or_else(|| "CSV 缺少站点名列（name / site_name）".to_string())?;
    let password_col =
        find_column(&headers, &["password"]).ok_or_else(|| "CSV 缺少 password 列".to_string())?;
    let url_col = find_column(&headers, &["site_url", "url"]);
    let username_col = find_column(&headers, &["username"]);
    let notes_col = find_column(&headers, &["notes", "note"]);
    let tags_col = find_column(&headers, &["tags"]);

    let mut out: Vec<ImportEntry> = Vec::new();
    for (offset, record) in records.enumerate() {
        let get = |col: Option<usize>| -> Option<String> {
            col.and_then(|i| record.get(i).cloned())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let site_name = match get(Some(name_col)) {
            Some(v) => v,
            None => return Err(format!("CSV 第 {} 行缺少站点名称", offset + 2)),
        };
        // 浏览器导出里允许用户名为空；空密码保留，由导入流程按「密码为空」跳过。
        let username = get(username_col).unwrap_or_default();
        let password = get(Some(password_col)).unwrap_or_default();
        out.push(ImportEntry {
            site_name,
            site_url: get(url_col).unwrap_or_default(),
            username,
            password,
            notes: get(notes_col),
            tags: get(tags_col),
        });
    }
    Ok(out)
}

/// 将已解密的 PasswordEntry 转回 NewPassword 以便重新写入。
fn entry_to_new(p: &PasswordEntry) -> NewPassword {
    NewPassword {
        site_name: p.site_name.clone(),
        site_url: p.site_url.clone(),
        username: p.username.clone(),
        password: p.password.clone().unwrap_or_default(),
        notes: p.notes.clone(),
        tags: Some(p.tags.clone()),
        priority: Some(p.priority),
    }
}

/// 统一执行导入：以「名称 + 网址 + 用户名」作为身份标识（同一站点可以有多个账号），
/// 已存在的条目用导入内容覆盖（新密码替换旧密码），否则新增。
fn apply_import(vault: &PasswordOperations, items: Vec<NewPassword>) -> Result<ImportResult, String> {
    if items.len() > MAX_IMPORT_ENTRIES {
        return Err(format!(
            "单次最多导入 {} 条，本次 {} 条",
            MAX_IMPORT_ENTRIES,
            items.len()
        ));
    }
    let mut result = ImportResult {
        imported_count: 0,
        skipped_count: 0,
        updated_count: 0,
        error_count: 0,
        errors: Vec::new(),
    };
    let existing = vault.get_passwords().unwrap_or_default();
    let mut index: HashMap<(String, String, String), i64> = HashMap::new();
    for e in &existing {
        if let Some(id) = e.id {
            index.insert(
                crate::utils::identity::entry_identity(&e.site_url, &e.site_name, &e.username),
                id,
            );
        }
    }
    for np in items {
        // 空密码多半是源文件缺少 password 字段，写入会覆盖掉可用密码，直接跳过。
        if np.password.is_empty() {
            result.skipped_count += 1;
            result.errors.push(format!("跳过「{}」：密码为空", np.site_name));
            continue;
        }
        let key = crate::utils::identity::entry_identity(&np.site_url, &np.site_name, &np.username);
        match index.get(&key).copied() {
            Some(id) => match vault.overwrite_password(id, np) {
                Ok(_) => result.updated_count += 1,
                Err(err) => {
                    result.error_count += 1;
                    result.errors.push(format!("{}", err));
                }
            },
            None => {
                let label = np.site_name.clone();
                match vault.add_password(np) {
                    Ok(entry) => {
                        result.imported_count += 1;
                        if let Some(id) = entry.id {
                            index.insert(key, id);
                        }
                    }
                    Err(err) => {
                        result.error_count += 1;
                        result.errors.push(format!("{}: {}", label, err));
                    }
                }
            }
        }
    }
    Ok(result)
}

fn import_json(vault: &PasswordOperations, data: &str) -> Result<ImportResult, String> {
    let entries: Vec<PasswordEntry> = serde_json::from_str(data)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;
    let items: Vec<NewPassword> = entries.iter().map(entry_to_new).collect();
    apply_import(vault, items)
}

fn import_csv(vault: &PasswordOperations, data: &str) -> Result<ImportResult, String> {
    let entries = parse_csv_import(data)?;
    let items: Vec<NewPassword> = entries
        .into_iter()
        .map(|e| NewPassword {
            site_name: e.site_name,
            site_url: e.site_url,
            username: e.username,
            password: e.password,
            notes: e.notes,
            tags: e.tags.map(|t| t.split(';').map(|s| s.to_string()).collect()),
            priority: None,
        })
        .collect();
    apply_import(vault, items)
}

/// 自动识别 JSON / CSV 并执行导入，返回统计结果。
fn do_import(vault: &PasswordOperations, data: &str) -> ApiResponse<ImportResult> {
    if data.len() > MAX_IMPORT_BYTES {
        return ApiResponse::err_message(format!(
            "导入文件过大：上限 {} MB",
            MAX_IMPORT_BYTES / 1024 / 1024
        ));
    }
    let trimmed = data.trim();
    let res = if trimmed.starts_with('[') || trimmed.starts_with('{') {
        import_json(vault, data)
    } else {
        import_csv(vault, data)
    };
    match res {
        Ok(r) => ApiResponse::ok(r),
        Err(e) => ApiResponse::err_message(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db = dir.join(format!("hellopass_test_{}_{}_rt.db", pid, n));
        let cfg = dir.join(format!("hellopass_cfg_{}_{}_rt", pid, n));
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
        std::fs::create_dir_all(&cfg).unwrap();
        (db, cfg)
    }

    #[test]
    fn runtime_setup_unlock_change_rekey() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(!rt.is_set());
        // 尚未设置时解锁应失败
        assert!(!rt.unlock("whatever").success);

        // 设置主密码
        let setup = rt.setup("StrongPass123!");
        assert!(setup.success, "setup 失败: {:?}", setup.error);
        assert!(rt.is_set());

        // 用正确密码重新解锁（新实例，重新从磁盘加载 salt）
        let mut rt2 = VaultRuntime::new(cfg.clone(), db.clone());
        let unlock = rt2.unlock("StrongPass123!");
        assert!(unlock.success, "unlock 失败: {:?}", unlock.error);

        // 添加一条
        let add = rt2.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "bob".into(),
            password: "secret-value".into(),
            notes: Some("note".into()),
            tags: Some(vec!["work".into()]),
            priority: Some(2),
        });
        assert!(add.success);
        let id = add.data.as_ref().unwrap().id.unwrap();

        // 修改主密码（触发重加密）
        let ch = rt2.change_password("StrongPass123!", "NewStrongPass456!");
        assert!(ch.success, "change 失败: {:?}", ch.error);

        // 旧密码应无法解锁，新密码可以
        let mut rt_old = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(!rt_old.unlock("StrongPass123!").success, "旧密码不应解锁");
        let mut rt_new = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(rt_new.unlock("NewStrongPass456!").success, "新密码应解锁");

        // 数据仍可被新密钥正确解密
        let got = rt_new.get_password(id);
        assert!(got.success);
        assert_eq!(got.data.unwrap().password.as_deref(), Some("secret-value"));

        // 删除
        assert!(rt_new.delete_password(id).success);
        assert_eq!(rt_new.get_passwords().data.unwrap().len(), 0);

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn password_policy_and_reuse() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let add = rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "first-pass".into(),
            notes: None,
            tags: None,
            priority: None,
        });
        assert!(add.success);
        let id = add.data.as_ref().unwrap().id.unwrap();

        let same = rt.update_password(UpdatePassword {
            id,
            site_name: None,
            site_url: None,
            username: None,
            password: Some("first-pass".into()),
            notes: None,
            tags: None,
            priority: None,
        });
        // 原样提交当前密码 = 未修改，应当成功（否则“编辑其他字段”会永远保存失败）
        assert!(same.success, "same password should be treated as unchanged");

        let diff = rt.update_password(UpdatePassword {
            id,
            site_name: None,
            site_url: None,
            username: None,
            password: Some("second-pass".into()),
            notes: None,
            tags: None,
            priority: None,
        });
        assert!(diff.success);

        // 但真的改回历史密码必须被拦截
        let back = rt.update_password(UpdatePassword {
            id,
            site_name: None,
            site_url: None,
            username: None,
            password: Some("first-pass".into()),
            notes: None,
            tags: None,
            priority: None,
        });
        assert!(!back.success, "改回历史密码应被拦截");

        let pol = rt.get_password_policy();
        assert!(pol.success);
        assert_eq!(pol.data.unwrap().max_age_days, 90);

        let exp = rt.get_expiring_passwords(100000);
        assert!(exp.success);
        assert!(exp.data.unwrap().iter().any(|e| e.id == Some(id)));

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn integrity_baseline_detects_tamper() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let _ = rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "secret".into(),
            notes: None,
            tags: None,
            priority: None,
        });
        rt.lock();

        let baseline = format!("{}.integrity", db.display());
        assert!(std::path::Path::new(&baseline).exists());

        {
            let mut rt2 = VaultRuntime::new(cfg.clone(), db.clone());
            assert!(rt2.unlock("StrongPass123!").success);
            assert!(rt2.integrity_warning().is_none());
        }

        // 篡改基线后，下次解锁应产生告警
        std::fs::write(&baseline, "deadbeef").unwrap();
        let mut rt3 = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(rt3.unlock("StrongPass123!").success);
        assert!(rt3.integrity_warning().is_some());
        let rep = rt3.check_integrity();
        assert!(rep.success);
        assert!(!rep.data.unwrap().baseline_ok);

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(&baseline);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn integrity_report_ok() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let rep = rt.check_integrity();
        assert!(rep.success, "integrity: {:?}", rep.error);
        let r = rep.data.unwrap();
        assert!(r.sqlite_ok);
        assert_eq!(r.db_checksum.len(), 64);
        assert!(r.db_size > 0);
        assert!(r.config_checksum.is_some());
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn setup_twice_rejected() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(rt.setup("StrongPass123!").success);
        assert!(!rt.setup("StrongPass123!").success);
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn wrong_master_password_rejected() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(!rt.unlock("WrongPass456!").success);
        assert!(rt.unlock("StrongPass123!").success);
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn backup_wrong_password_rejected() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let _ = rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "secret".into(),
            notes: None,
            tags: None,
            priority: None,
        });
        let backup = rt.export_backup().data.unwrap();

        let (db2, cfg2) = tmp();
        let mut dst = VaultRuntime::new(cfg2.clone(), db2.clone());
        dst.setup("StrongPass123!");
        assert!(dst.unlock("StrongPass123!").success);
        assert!(!dst.restore_backup(backup, "WrongPass456!".into()).success);

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
        let _ = std::fs::remove_file(&db2);
        let _ = std::fs::remove_dir_all(&cfg2);
    }

    #[test]
    fn launcher_summaries_never_expose_secrets() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg, db);
        assert!(rt.setup("StrongPass123!").success);
        let add = rt.add_password(NewPassword {
            site_name: "微信".into(),
            site_url: "https://wx.qq.com".into(),
            username: "bob".into(),
            password: "secret-value".into(),
            notes: Some("private note".into()),
            tags: Some(vec!["聊天".into()]),
            priority: Some(1),
        });
        assert!(add.success);
        let id = add.data.as_ref().unwrap().id.unwrap();

        let res = rt.list_entry_summaries();
        assert!(res.success, "summaries 失败: {:?}", res.error);
        let list = res.data.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].site_name, "微信");
        assert!(list[0].pinyin_initials.contains("wx"), "拼音首字母缺失");

        // 摘要序列化后不能出现任何密码/备注明文或密文字段
        let json = serde_json::to_string(&list).unwrap();
        assert!(!json.contains("secret-value"));
        assert!(!json.contains("private note"));
        assert!(!json.contains("encrypted_password"));

        // 复制走独立入口；锁定后必须拒绝
        let copied = rt.secret_for_copy(id, "password").unwrap();
        assert_eq!(copied.as_str(), "secret-value");
        rt.lock();
        assert!(rt.list_entry_summaries().data.is_none());
        assert!(rt.secret_for_copy(id, "password").is_err());
        assert!(rt.reveal_password(id).is_err());
    }

    #[test]
    fn integrity_report_uses_unlock_time_baseline() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(rt.setup("StrongPass123!").success);
        assert!(rt.unlock("StrongPass123!").success);
        let rep = rt.check_integrity();
        assert!(rep.success);
        assert!(rep.data.unwrap().baseline_ok, "无基线时应视为一致");

        assert!(rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: String::new(),
            username: "a".into(),
            password: "secret".into(),
            notes: None,
            tags: None,
            priority: None,
        }).success);
        rt.lock();
        assert!(std::path::Path::new(&format!("{}.integrity", db.display())).exists());

        let mut rt2 = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(rt2.unlock("StrongPass123!").success);
        assert!(rt2.integrity_warning().is_none(), "正常锁定后不应告警");
        let rep2 = rt2.check_integrity();
        assert!(rep2.success);
        assert!(rep2.data.unwrap().baseline_ok);

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_file(format!("{}.integrity", db.display()));
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn full_lifecycle_integration() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(!rt.is_set());
        assert!(rt.setup("StrongPass123!").success);
        assert!(rt.is_set());
        assert!(rt.unlock("StrongPass123!").success);

        let a = rt.add_password(NewPassword {
            site_name: "a.com".into(),
            site_url: "https://a.com".into(),
            username: "alice".into(),
            password: "pass-a".into(),
            notes: None,
            tags: Some(vec!["work".into()]),
            priority: Some(1),
        }).data.unwrap();
        let b = rt.add_password(NewPassword {
            site_name: "b.com".into(),
            site_url: "https://b.com".into(),
            username: "bob".into(),
            password: "pass-b".into(),
            notes: None,
            tags: None,
            priority: None,
        }).data.unwrap();

        assert_eq!(rt.get_passwords().data.unwrap().len(), 2);
        let found = rt.search_passwords(SearchQuery {
            query: "a.com".into(),
            tags: None,
            limit: None,
            offset: None,
        });
        assert_eq!(found.data.unwrap().len(), 1);

        let upd = rt.update_password(UpdatePassword {
            id: a.id.unwrap(),
            site_name: None,
            site_url: None,
            username: None,
            password: Some("pass-a2".into()),
            notes: None,
            tags: None,
            priority: None,
        });
        assert!(upd.success);

        assert_eq!(rt.get_password_stats().data.unwrap().total_count, 2);
        assert!(rt.get_audit_logs().success);
        assert!(rt.check_integrity().success);

        // 备份 → 独立新库恢复
        let backup = rt.export_backup().data.unwrap();
        let (db2, cfg2) = tmp();
        let mut dst = VaultRuntime::new(cfg2.clone(), db2.clone());
        dst.setup("StrongPass123!");
        assert!(dst.unlock("StrongPass123!").success);
        assert!(dst.restore_backup(backup, "StrongPass123!".into()).success);
        assert_eq!(dst.get_passwords().data.unwrap().len(), 2);

        // 锁定后拒绝访问
        rt.lock();
        assert!(!rt.get_passwords().success);

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
        let _ = std::fs::remove_file(&db2);
        let _ = std::fs::remove_dir_all(&cfg2);
    }

    #[test]
    fn export_encrypted_uses_separate_password() {
        let (db1, cfg1) = tmp();
        let mut src = VaultRuntime::new(cfg1.clone(), db1.clone());
        src.setup("MasterPass123!");
        assert!(src.unlock("MasterPass123!").success);
        let add = src.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "secret-value".into(),
            notes: Some("note".into()),
            tags: Some(vec!["work".into()]),
            priority: Some(1),
        });
        assert!(add.success);

        let pkg = src.export_encrypted("ExportPass456!", None);
        assert!(pkg.success, "export_encrypted: {:?}", pkg.error);
        let blob = pkg.data.unwrap();
        assert!(!blob.contains("secret-value"), "encrypted export must be ciphertext");

        let (db2, cfg2) = tmp();
        let mut dst = VaultRuntime::new(cfg2.clone(), db2.clone());
        dst.setup("OtherMaster789!");
        assert!(dst.unlock("OtherMaster789!").success);
        let restored = dst.import_encrypted(blob.clone(), "ExportPass456!".into());
        assert!(restored.success, "import_encrypted: {:?}", restored.error);
        assert_eq!(restored.data.unwrap().imported_count, 1);

        let (db3, cfg3) = tmp();
        let mut dst3 = VaultRuntime::new(cfg3.clone(), db3.clone());
        dst3.setup("OtherMaster789!");
        assert!(dst3.unlock("OtherMaster789!").success);
        let bad = dst3.import_encrypted(blob, "WrongPass000!".into());
        assert!(bad.success == false, "wrong export password must be rejected");

        let all = dst.get_passwords();
        assert!(all.success);
        let entries = all.data.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].password.as_deref(), Some("secret-value"));

        let _ = std::fs::remove_file(&db1);
        let _ = std::fs::remove_dir_all(&cfg1);
        let _ = std::fs::remove_file(&db2);
        let _ = std::fs::remove_dir_all(&cfg2);
        let _ = std::fs::remove_file(&db3);
        let _ = std::fs::remove_dir_all(&cfg3);
    }

    #[test]
    fn export_encrypted_respects_selection() {
        let (db1, cfg1) = tmp();
        let mut src = VaultRuntime::new(cfg1.clone(), db1.clone());
        src.setup("MasterPass123!");
        assert!(src.unlock("MasterPass123!").success);
        let a = src
            .add_password(NewPassword {
                site_name: "a.com".into(),
                site_url: "https://a.com".into(),
                username: "alice".into(),
                password: "pass-a".into(),
                notes: None,
                tags: None,
                priority: Some(2),
            })
            .data
            .unwrap();
        let _b = src
            .add_password(NewPassword {
                site_name: "b.com".into(),
                site_url: "https://b.com".into(),
                username: "bob".into(),
                password: "pass-b".into(),
                notes: None,
                tags: None,
                priority: Some(2),
            })
            .data
            .unwrap();

        // 只导出 a.com
        let pkg = src.export_encrypted("ExportPass456!", Some(vec![a.id.unwrap()]));
        assert!(pkg.success, "export: {:?}", pkg.error);

        let (db2, cfg2) = tmp();
        let mut dst = VaultRuntime::new(cfg2.clone(), db2.clone());
        dst.setup("OtherMaster789!");
        assert!(dst.unlock("OtherMaster789!").success);
        let restored = dst.import_encrypted(pkg.data.unwrap(), "ExportPass456!".into());
        assert!(restored.success, "import: {:?}", restored.error);
        assert_eq!(restored.data.unwrap().imported_count, 1);
        let names: Vec<String> = dst
            .get_passwords()
            .data
            .unwrap()
            .into_iter()
            .map(|e| e.site_name)
            .collect();
        assert_eq!(names, vec!["a.com".to_string()]);

        // 空选择必须报错
        let empty = src.export_encrypted("ExportPass456!", Some(vec![]));
        assert!(!empty.success, "空选择不应导出全部");
        // 导出密码过短必须报错
        let weak = src.export_encrypted("short", None);
        assert!(!weak.success, "导出密码应至少 8 位");

        let _ = std::fs::remove_file(&db1);
        let _ = std::fs::remove_dir_all(&cfg1);
        let _ = std::fs::remove_file(&db2);
        let _ = std::fs::remove_dir_all(&cfg2);
    }

    #[test]
    fn backup_encrypted_roundtrip() {
        let (db1, cfg1) = tmp();
        let mut src = VaultRuntime::new(cfg1.clone(), db1.clone());
        src.setup("StrongPass123!");
        assert!(src.unlock("StrongPass123!").success);
        let add = src.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "secret-value".into(),
            notes: Some("note".into()),
            tags: Some(vec!["work".into()]),
            priority: Some(1),
        });
        assert!(add.success);

        let backup = src.export_backup();
        assert!(backup.success, "export_backup: {:?}", backup.error);
        let blob = backup.data.unwrap();
        assert!(!blob.contains("secret-value"), "备份必须是密文");

        // 目标保险库：独立 db/config，同一主密码但 salt 不同
        let (db2, cfg2) = tmp();
        let mut dst = VaultRuntime::new(cfg2.clone(), db2.clone());
        dst.setup("StrongPass123!");
        assert!(dst.unlock("StrongPass123!").success);
        let restored = dst.restore_backup(blob, "StrongPass123!".into());
        assert!(restored.success, "restore_backup: {:?}", restored.error);
        assert_eq!(restored.data.unwrap().imported_count, 1);

        let all = dst.get_passwords();
        assert!(all.success);
        let entries = all.data.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].site_name, "example.com");
        assert_eq!(entries[0].password.as_deref(), Some("secret-value"));

        let _ = std::fs::remove_file(&db1);
        let _ = std::fs::remove_dir_all(&cfg1);
        let _ = std::fs::remove_file(&db2);
        let _ = std::fs::remove_dir_all(&cfg2);
    }

    #[test]
    fn audit_logs_written_on_operations() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let _ = rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "secret".into(),
            notes: None,
            tags: None,
            priority: None,
        });
        let logs = rt.get_audit_logs();
        assert!(logs.success, "audit logs: {:?}", logs.error);
        let data = logs.data.as_ref().unwrap();
        assert!(data.iter().any(|l| l.action == "unlock"));
        assert!(data.iter().any(|l| l.action == "add_password"));
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn session_lockout_after_max_attempts() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        for _ in 0..5 {
            assert!(!rt.unlock("wrong-password").success);
        }
        assert!(rt.lockout_until.is_some(), "should be locked");
        assert!(!rt.unlock("StrongPass123!").success);
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn session_timeout_defaults_and_clamps() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        // 启动器默认不自动锁定
        assert_eq!(rt.session_timeout.num_seconds(), 0);
        rt.set_session_timeout(0);
        assert_eq!(rt.session_timeout.num_seconds(), 0);
        // 下限 10 秒
        rt.set_session_timeout(1);
        assert_eq!(rt.session_timeout.num_seconds(), 10);
        // 上限 5 分钟
        rt.set_session_timeout(3600);
        assert_eq!(rt.session_timeout.num_seconds(), 300);
        rt.set_session_timeout(45);
        assert_eq!(rt.session_timeout.num_seconds(), 45);

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn session_timeout_relocks() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        rt.set_session_timeout(10);
        rt.last_activity = Some(chrono::Utc::now() - chrono::Duration::hours(2));
        let resp = rt.get_passwords();
        assert!(!resp.success, "should require re-unlock after timeout");
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn import_export_roundtrip() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "secret".into(),
            notes: Some("note".into()),
            tags: Some(vec!["work".into()]),
            priority: Some(1),
        });
        // 用列表接口拿到的条目序列化成 JSON，再喂回导入（验证去重逻辑）
        let json = serde_json::to_string(&rt.get_passwords().data.unwrap()).unwrap();
        let imp = rt.import_passwords(json.clone());
        assert!(imp.success, "import: {:?}", imp.error);
        assert_eq!(imp.data.as_ref().unwrap().updated_count, 1);
        assert_eq!(imp.data.as_ref().unwrap().imported_count, 0);
        let csv = format!("site_name,site_url,username,password,notes,category,tags\n{}\n", "bank.com,https://bank.com,bob,bankpass,,work,bank");
        let imp2 = rt.import_passwords(csv);
        assert!(imp2.success, "csv import: {:?}", imp2.error);
        assert_eq!(imp2.data.as_ref().unwrap().imported_count, 1);
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn import_overwrites_by_name_url_and_username() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        rt.add_password(NewPassword {
            site_name: "Example".into(),
            site_url: "https://www.example.com/login".into(),
            username: "alice".into(),
            password: "old-pass".into(),
            notes: Some("旧备注".into()),
            tags: Some(vec!["work".into()]),
            priority: Some(2),
        });
        // 名称/网址写法/用户名大小写不同，仍应识别为同一条并覆盖
        let csv = "name,url,username,password\nexample,https://example.com/login,Alice,new-pass\n";
        let imp = rt.import_passwords(csv.to_string());
        assert!(imp.success, "import: {:?}", imp.error);
        let data = imp.data.as_ref().unwrap();
        assert_eq!(data.updated_count, 1);
        assert_eq!(data.imported_count, 0);
        assert_eq!(data.skipped_count, 0);

        let entries = rt.get_passwords().data.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].password.as_deref(), Some("new-pass"));
        assert_eq!(entries[0].username, "Alice");
        // 导入未提供的备注/标签沿用原值
        assert_eq!(entries[0].notes.as_deref(), Some("旧备注"));
        assert_eq!(entries[0].tags, vec!["work".to_string()]);
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn import_limits_are_enforced() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);

        // 文件大小上限：明文导入与加密包都在解析前拒绝
        let big = "x".repeat(MAX_IMPORT_BYTES + 1);
        let too_big = rt.import_passwords(big.clone());
        assert!(!too_big.success);
        assert!(too_big.error.unwrap().contains("过大"));
        let too_big_spb = rt.import_encrypted(big, "ExportPass456!".into());
        assert!(!too_big_spb.success);

        // 条数上限：超限时整批拒绝，不写入任何条目
        let mut csv = String::from("name,url,username,password\n");
        for i in 0..=MAX_IMPORT_ENTRIES {
            csv.push_str(&format!(
                "site{i},https://s{i}.com,u{i},pass{i}\n"
            ));
        }
        let too_many = rt.import_passwords(csv);
        assert!(!too_many.success);
        assert!(too_many.error.unwrap().contains("5000"));
        assert!(rt.get_passwords().data.unwrap().is_empty());

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn same_site_keeps_multiple_accounts() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let csv = "name,url,username,password\n\
GitHub,https://github.com,alice,pass1\n\
GitHub,https://github.com,bob,pass2\n";
        let imp = rt.import_passwords(csv.to_string());
        assert!(imp.success, "import: {:?}", imp.error);
        assert_eq!(imp.data.as_ref().unwrap().imported_count, 2);

        // 再导入时只覆盖同名同址同用户的那一条
        let csv2 = "name,url,username,password\nGitHub,https://github.com,bob,pass2-new\n";
        let imp2 = rt.import_passwords(csv2.to_string());
        assert!(imp2.success, "import: {:?}", imp2.error);
        assert_eq!(imp2.data.as_ref().unwrap().updated_count, 1);
        assert_eq!(imp2.data.as_ref().unwrap().imported_count, 0);

        let entries = rt.get_passwords().data.unwrap();
        assert_eq!(entries.len(), 2);
        let alice = entries.iter().find(|e| e.username == "alice").unwrap();
        let bob = entries.iter().find(|e| e.username == "bob").unwrap();
        assert_eq!(alice.password.as_deref(), Some("pass1"));
        assert_eq!(bob.password.as_deref(), Some("pass2-new"));
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn browser_csv_headers_are_supported() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        // 浏览器（Chrome/Edge）导出：BOM + CRLF + name,url,username,password,note，
        // 最后一行备注里带引号包裹的换行。
        let csv = "\u{feff}name,url,username,password,note\r\n\
Google,https://accounts.google.com,alice,chrome-pass,个人账号\r\n\
GitHub,https://github.com,,gh-pass,\r\n\
Bank,https://bank.com,bob,bank-pass,\"第一行\n第二行\"\r\n";
        let imp = rt.import_passwords(csv.to_string());
        assert!(imp.success, "import: {:?}", imp.error);
        assert_eq!(imp.data.as_ref().unwrap().imported_count, 3, "{:?}", imp.data);

        let entries = rt.get_passwords().data.unwrap();
        let google = entries.iter().find(|e| e.site_name == "Google").unwrap();
        assert_eq!(google.site_url, "https://accounts.google.com");
        assert_eq!(google.username, "alice");
        assert_eq!(google.password.as_deref(), Some("chrome-pass"));
        assert_eq!(google.notes.as_deref(), Some("个人账号"));

        let github = entries.iter().find(|e| e.site_name == "GitHub").unwrap();
        assert_eq!(github.username, "");
        assert_eq!(github.password.as_deref(), Some("gh-pass"));

        let bank = entries.iter().find(|e| e.site_name == "Bank").unwrap();
        assert_eq!(bank.notes.as_deref(), Some("第一行\n第二行"));
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn clear_all_passwords_removes_entries() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        for (site, user) in [("a.com", "alice"), ("b.com", "bob")] {
            rt.add_password(NewPassword {
                site_name: site.into(),
                site_url: format!("https://{}", site),
                username: user.into(),
                password: "secret1".into(),
                notes: None,
                tags: None,
                priority: None,
            });
        }
        let cleared = rt.clear_all_passwords();
        assert!(cleared.success, "clear: {:?}", cleared.error);
        assert_eq!(cleared.data, Some(2));
        assert!(rt.get_passwords().data.unwrap().is_empty());
        assert_eq!(rt.get_password_stats().data.unwrap().total_count, 0);
        let logs = rt.get_audit_logs().data.unwrap();
        assert!(logs.iter().any(|l| l.action == "clear_all_passwords"));
        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn history_survives_password_change() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let add = rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "hist-pass".into(),
            notes: None,
            tags: None,
            priority: None,
        });
        let id = add.data.as_ref().unwrap().id.unwrap();
        let upd = rt.update_password(UpdatePassword {
            id,
            site_name: None,
            site_url: None,
            username: None,
            password: Some("second-pass".into()),
            notes: None,
            tags: None,
            priority: None,
        });
        assert!(upd.success);

        let ch = rt.change_password("StrongPass123!", "NewStrongPass456!");
        assert!(ch.success, "change 失败: {:?}", ch.error);

        // 历史密文必须随主密钥一起重加密，改密后仍能拦截复用
        let reuse = rt.update_password(UpdatePassword {
            id,
            site_name: None,
            site_url: None,
            username: None,
            password: Some("hist-pass".into()),
            notes: None,
            tags: None,
            priority: None,
        });
        assert!(!reuse.success, "改密后仍应拦截历史密码复用");

        let mut rt_new = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(rt_new.unlock("NewStrongPass456!").success);
        let got = rt_new.get_password(id);
        assert_eq!(
            got.data.as_ref().and_then(|e| e.password.clone()).as_deref(),
            Some("second-pass")
        );

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn lockout_survives_restart() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        for i in 0..5 {
            let res = rt.unlock("WrongPass123!");
            assert!(!res.success, "第 {} 次错误尝试应失败", i + 1);
        }
        assert!(!rt.unlock("StrongPass123!").success, "达到上限后应锁定");

        // 新实例（模拟重启应用）不能再靠重置内存绕过锁定
        let mut rt2 = VaultRuntime::new(cfg.clone(), db.clone());
        let res = rt2.unlock("StrongPass123!");
        assert!(!res.success, "重启后锁定必须仍然生效");
        assert!(
            res.error.unwrap_or_default().contains("已锁定"),
            "应返回锁定提示"
        );

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }

    #[test]
    fn failed_rekey_keeps_old_password_valid() {
        let (db, cfg) = tmp();
        let mut rt = VaultRuntime::new(cfg.clone(), db.clone());
        rt.setup("StrongPass123!");
        assert!(rt.unlock("StrongPass123!").success);
        let add = rt.add_password(NewPassword {
            site_name: "example.com".into(),
            site_url: "https://example.com".into(),
            username: "alice".into(),
            password: "secret-value".into(),
            notes: None,
            tags: None,
            priority: None,
        });
        let id = add.data.as_ref().unwrap().id.unwrap();

        // 把密文改成能被 base64 解码、但 GCM 认证必然失败的内容
        {
            let conn = rusqlite::Connection::open(&db).unwrap();
            let garbage = base64::engine::general_purpose::STANDARD.encode([0u8; 40]);
            conn.execute(
                "UPDATE passwords SET encrypted_password=?1 WHERE id=?2",
                rusqlite::params![garbage, id],
            )
            .unwrap();
        }

        let ch = rt.change_password("StrongPass123!", "NewStrongPass456!");
        assert!(!ch.success, "重加密失败时改密必须报错");

        // 关键：配置没有被提前替换，旧密码仍能解锁，新密码不行
        let mut old = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(old.unlock("StrongPass123!").success, "回滚后旧密码必须仍可用");
        let mut new = VaultRuntime::new(cfg.clone(), db.clone());
        assert!(!new.unlock("NewStrongPass456!").success, "失败的新密码不应生效");

        let _ = std::fs::remove_file(&db);
        let _ = std::fs::remove_dir_all(&cfg);
    }
}
