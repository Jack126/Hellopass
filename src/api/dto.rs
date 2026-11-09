use chrono::Utc;
use serde::{Serialize, Deserialize};

/// 统一的 API 响应结构（与前端 ApiResponse<T> 对应）。
/// timestamp 使用 RFC3339 字符串，便于前端直接显示/序列化。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: String,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    pub fn err_message(msg: impl Into<String>) -> Self {
        ApiResponse {
            success: false,
            data: None,
            error: Some(msg.into()),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// 保险库未解锁时返回的便捷错误响应（泛型，便于直接用作任意 ApiResponse<T>）。
pub fn locked<T>() -> ApiResponse<T> {
    ApiResponse::err_message("保险库已锁定，请先解锁主密码")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: Option<i64>,
    pub site_name: String,
    pub site_url: String,
    pub username: String,
    pub encrypted_password: String,
    pub encrypted_notes: Option<String>,
    /// 明文密码（仅在解锁会话内由后端解密后返回，数据库只存密文）
    pub password: Option<String>,
    /// 明文备注（同上）
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_accessed: Option<String>,
    pub access_count: i32,
    pub tags: Vec<String>,
    pub category: String,
    pub priority: i32,
    pub metadata: Option<String>,
    pub checksum: Option<String>,
}

/// 启动器列表用的轻量摘要：只含检索和展示所需字段，不含任何密文/明文密码。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySummary {
    pub id: i64,
    pub site_name: String,
    pub site_url: String,
    pub username: String,
    pub tags: Vec<String>,
    pub category: String,
    pub updated_at: String,
    pub last_accessed: Option<String>,
    pub access_count: i32,
    /// 站点名 + 用户名 + 标签的拼音首字母（后端生成，前端直接匹配）
    pub pinyin_initials: String,
}

/// 详情面板用：含备注，但不含密码明文/密文；密码只在显式 reveal 时单独取。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryDetail {
    pub id: i64,
    pub site_name: String,
    pub site_url: String,
    pub username: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_accessed: Option<String>,
    pub access_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPassword {
    pub site_name: String,
    pub site_url: String,
    pub username: String,
    pub password: String,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePassword {
    pub id: i64,
    pub site_name: Option<String>,
    pub site_url: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStats {
    pub total_count: i64,
    pub category_counts: std::collections::HashMap<String, i64>,
    pub tag_counts: std::collections::HashMap<String, i32>,
    pub recent_access_count: i64,
}

/// 审计日志条目（对应 audit_logs 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub timestamp: String,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// 导入结果统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported_count: i64,
    pub skipped_count: i64,
    pub updated_count: i64,
    pub error_count: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicy {
    pub max_age_days: i64,
    pub history_limit: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordExpiryInfo {
    pub id: Option<i64>,
    pub site_name: String,
    pub days_until_expiry: i64,
    pub expired: bool,
}

/// 完整性检查报告：SQLite 自检 + 数据库文件 SHA-256 + 主密码配置 SHA-256。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub sqlite_ok: bool,
    pub db_checksum: String,
    pub db_size: u64,
    pub config_checksum: Option<String>,
    /// 与上次正常锁定写入的基线是否一致（无基线时为 true）
    pub baseline_ok: bool,
}
