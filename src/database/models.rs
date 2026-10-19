use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 密码条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: Option<i64>,
    pub site_name: String,
    pub site_url: String,
    pub username: String,
    pub encrypted_password: String,
    pub encrypted_notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_accessed: Option<String>,
    pub access_count: i32,
    pub tags: Option<String>,
    pub category: String,
    pub priority: i32,
    pub metadata: Option<String>,
    pub checksum: Option<String>,
}

impl PasswordEntry {
    /// 创建新的密码条目
    pub fn new(
        site_name: String,
        site_url: String,
        username: String,
        encrypted_password: String,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        
        Self {
            id: None,
            site_name,
            site_url,
            username,
            encrypted_password,
            encrypted_notes: None,
            created_at: now.clone(),
            updated_at: now,
            last_accessed: None,
            access_count: 0,
            tags: None,
            category: "general".to_string(),
            priority: 0,
            metadata: None,
            checksum: None,
        }
    }

    /// 更新访问信息
    pub fn update_access(&mut self) {
        self.last_accessed = Some(Utc::now().to_rfc3339());
        self.access_count += 1;
        self.updated_at = Utc::now().to_rfc3339();
    }

    /// 设置标签
    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = if tags.is_empty() {
            None
        } else {
            Some(tags.join(","))
        };
        self.updated_at = Utc::now().to_rfc3339();
    }

    /// 获取标签列表
    pub fn get_tags(&self) -> Vec<String> {
        match &self.tags {
            Some(tags_str) => tags_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            None => Vec::new(),
        }
    }

    /// 验证数据完整性
    pub fn verify_integrity(&self, expected_checksum: &str) -> bool {
        // 这里可以实现具体的校验逻辑
        // 目前简单实现
        self.checksum.as_deref() == Some(expected_checksum)
    }

    /// 生成元数据JSON
    pub fn to_metadata_json(&self) -> Result<String, serde_json::Error> {
        let metadata = serde_json::json!({
            "id": self.id,
            "category": self.category,
            "priority": self.priority,
            "access_count": self.access_count,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "last_accessed": self.last_accessed,
        });
        serde_json::to_string(&metadata)
    }

    /// 从元数据JSON创建
    pub fn from_metadata_json(
        &mut self,
        metadata_json: &str,
    ) -> Result<(), serde_json::Error> {
        let metadata: serde_json::Value = serde_json::from_str(metadata_json)?;
        
        if let Some(category) = metadata.get("category").and_then(|v| v.as_str()) {
            self.category = category.to_string();
        }
        
        if let Some(priority) = metadata.get("priority").and_then(|v| v.as_i64()) {
            self.priority = priority as i32;
        }
        
        if let Some(access_count) = metadata.get("access_count").and_then(|v| v.as_i64()) {
            self.access_count = access_count as i32;
        }
        
        if let Some(updated_at) = metadata.get("updated_at").and_then(|v| v.as_str()) {
            self.updated_at = updated_at.to_string();
        }
        
        if let Some(last_accessed) = metadata.get("last_accessed").and_then(|v| v.as_str()) {
            self.last_accessed = Some(last_accessed.to_string());
        }
        
        Ok(())
    }
}

/// 审计日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Option<i64>,
    pub timestamp: String,
    pub user_id: Option<String>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

impl AuditLog {
    /// 创建新的审计日志
    pub fn new(action: String, user_id: Option<String>) -> Self {
        Self {
            id: None,
            timestamp: Utc::now().to_rfc3339(),
            user_id,
            action,
            resource_type: None,
            resource_id: None,
            details: None,
            ip_address: None,
            user_agent: None,
            success: true,
            error_message: None,
        }
    }

    /// 设置资源信息
    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
        self
    }

    /// 设置详细信息
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(serde_json::to_string(&details).unwrap_or_default());
        self
    }

    /// 标记为失败
    pub fn with_failure(mut self, error_message: String) -> Self {
        self.success = false;
        self.error_message = Some(error_message);
        self
    }

    /// 获取详细信息
    pub fn get_details(&self) -> Option<serde_json::Value> {
        self.details.as_ref().and_then(|details| {
            serde_json::from_str(details).ok()
        })
    }
}

/// 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub key: String,
    pub value: String,
    pub encrypted: bool,
    pub updated_at: String,
}

impl Config {
    /// 创建新的配置项
    pub fn new(key: String, value: String, encrypted: bool) -> Self {
        Self {
            key,
            value,
            encrypted,
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    /// 更新配置值
    pub fn update_value(&mut self, value: String) {
        self.value = value;
        self.updated_at = Utc::now().to_rfc3339();
    }
}

/// 备份记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRecord {
    pub id: Option<i64>,
    pub timestamp: String,
    pub backup_type: String,
    pub file_path: String,
    pub file_size: i64,
    pub checksum: String,
    pub encrypted: bool,
    pub retention_until: Option<String>,
}

impl BackupRecord {
    /// 创建新的备份记录
    pub fn new(
        backup_type: String,
        file_path: String,
        file_size: i64,
        checksum: String,
        encrypted: bool,
    ) -> Self {
        Self {
            id: None,
            timestamp: Utc::now().to_rfc3339(),
            backup_type,
            file_path,
            file_size,
            checksum,
            encrypted,
            retention_until: None,
        }
    }

    /// 设置保留时间
    pub fn with_retention(mut self, retention_until: String) -> Self {
        self.retention_until = Some(retention_until);
        self
    }

    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        match &self.retention_until {
            Some(retention_until) => {
                if let Ok(retention_time) = chrono::DateTime::parse_from_rfc3339(retention_until) {
                    Utc::now() > retention_time
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

/// 搜索过滤器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub query: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub priority_min: Option<i32>,
    pub priority_max: Option<i32>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub has_notes: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

impl SearchFilter {
    /// 创建空过滤器
    pub fn new() -> Self {
        Self {
            query: None,
            category: None,
            tags: None,
            priority_min: None,
            priority_max: None,
            date_from: None,
            date_to: None,
            has_notes: None,
            limit: None,
            offset: None,
            sort_by: None,
            sort_order: None,
        }
    }

    /// 设置搜索查询
    pub fn with_query(mut self, query: String) -> Self {
        self.query = Some(query);
        self
    }

    /// 设置分类过滤
    pub fn with_category(mut self, category: String) -> Self {
        self.category = Some(category);
        self
    }

    /// 设置标签过滤
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    /// 设置优先级范围
    pub fn with_priority_range(mut self, min: i32, max: i32) -> Self {
        self.priority_min = Some(min);
        self.priority_max = Some(max);
        self
    }

    /// 设置日期范围
    pub fn with_date_range(mut self, from: String, to: String) -> Self {
        self.date_from = Some(from);
        self.date_to = Some(to);
        self
    }

    /// 设置分页
    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }

    /// 设置排序
    pub fn with_sort(mut self, by: String, order: String) -> Self {
        self.sort_by = Some(by);
        self.sort_order = Some(order);
        self
    }

    /// 生成SQL查询条件
    pub fn build_conditions(&self) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        // 查询条件
        if let Some(query) = &self.query {
            conditions.push("(site_name LIKE ? OR username LIKE ? OR tags LIKE ?)");
            let query_param = format!("%{}%", query);
            params.push(Box::new(query_param.clone()) as Box<dyn rusqlite::ToSql>);
            params.push(Box::new(query_param.clone()) as Box<dyn rusqlite::ToSql>);
            params.push(Box::new(query_param) as Box<dyn rusqlite::ToSql>);
        }

        // 分类条件
        if let Some(category) = &self.category {
            conditions.push("category = ?");
            params.push(Box::new(category.clone()) as Box<dyn rusqlite::ToSql>);
        }

        // 标签条件
        if let Some(tags) = &self.tags {
            for tag in tags {
                conditions.push("tags LIKE ?");
                params.push(Box::new(format!("%{}%", tag)) as Box<dyn rusqlite::ToSql>);
            }
        }

        // 优先级条件
        if let Some(min) = self.priority_min {
            conditions.push("priority >= ?");
            params.push(Box::new(min) as Box<dyn rusqlite::ToSql>);
        }

        if let Some(max) = self.priority_max {
            conditions.push("priority <= ?");
            params.push(Box::new(max) as Box<dyn rusqlite::ToSql>);
        }

        // 日期条件
        if let Some(date_from) = &self.date_from {
            conditions.push("created_at >= ?");
            params.push(Box::new(date_from.clone()) as Box<dyn rusqlite::ToSql>);
        }

        if let Some(date_to) = &self.date_to {
            conditions.push("created_at <= ?");
            params.push(Box::new(date_to.clone()) as Box<dyn rusqlite::ToSql>);
        }

        // 备注条件
        if let Some(has_notes) = self.has_notes {
            if has_notes {
                conditions.push("encrypted_notes IS NOT NULL");
            } else {
                conditions.push("encrypted_notes IS NULL");
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        (where_clause, params)
    }

    /// 生成排序条件
    pub fn build_sort(&self) -> String {
        if let Some(sort_by) = &self.sort_by {
            let order = self.sort_order.as_deref().unwrap_or("DESC");
            format!("ORDER BY {} {}", sort_by, order)
        } else {
            String::from("ORDER BY created_at DESC")
        }
    }

    /// 生成限制条件
    pub fn build_limit(&self) -> String {
        if let Some(limit) = self.limit {
            if let Some(offset) = self.offset {
                format!("LIMIT {} OFFSET {}", limit, offset)
            } else {
                format!("LIMIT {}", limit)
            }
        } else {
            String::new()
        }
    }
}

/// 密码强度评估结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrength {
    pub score: u8,
    pub level: String,
    pub feedback: Vec<String>,
    pub common_password: bool,
}

impl PasswordStrength {
    /// 评估密码强度
    pub fn assess(password: &str) -> Self {
        let mut score = 0u8;
        let mut feedback = Vec::new();
        let mut common_password = false;

        // 检查常见密码
        let common_passwords = [
            "password", "123456", "qwerty", "letmein", "welcome",
            "admin", "password1", "12345678", "abc123", "password123",
            "123456789", "12345", "1234", "1234567", "1234567890",
            "qwerty123", "iloveyou", "admin123", "password123"
        ];

        if common_passwords.contains(&password.to_lowercase().as_str()) {
            common_password = true;
            feedback.push("这是一个常见密码，容易被猜测");
            score = 0;
        } else {
            // 长度评分
            if password.len() >= 12 {
                score += 25;
            } else if password.len() >= 8 {
                score += 15;
            } else {
                feedback.push("密码长度应至少为12个字符");
            }

            // 复杂度评分
            if password.chars().any(|c| c.is_uppercase()) {
                score += 15;
            } else {
                feedback.push("应包含大写字母");
            }

            if password.chars().any(|c| c.is_lowercase()) {
                score += 15;
            } else {
                feedback.push("应包含小写字母");
            }

            if password.chars().any(|c| c.is_numeric()) {
                score += 15;
            } else {
                feedback.push("应包含数字");
            }

            if password.chars().any(|c| !c.is_alphanumeric()) {
                score += 20;
            } else {
                feedback.push("应包含特殊字符");
            }

            // 多样性评分
            let unique_chars: std::collections::HashSet<char> = password.chars().collect();
            if unique_chars.len() >= password.len() * 3 / 4 {
                score += 10;
            } else {
                feedback.push("应使用更多样化的字符");
            }
        }

        // 限制分数范围
        score = std::cmp::min(score, 100);

        // 确定强度等级
        let level = if score < 30 {
            "弱"
        } else if score < 70 {
            "中"
        } else {
            "强"
        };

        // 如果分数很高但没有反馈，添加正面反馈
        if feedback.is_empty() && score >= 70 {
            feedback.push("这是一个强密码");
        }

        Self {
            score,
            level: level.to_string(),
            feedback: feedback.iter().map(|s| s.to_string()).collect(),
            common_password,
        }
    }
}

/// 导入导出数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,
    pub timestamp: String,
    pub format: String,
    pub encrypted: bool,
    pub count: usize,
    pub passwords: Vec<PasswordEntry>,
    pub metadata: Option<serde_json::Value>,
}

impl ExportData {
    /// 创建导出数据
    pub fn new(passwords: Vec<PasswordEntry>, encrypted: bool) -> Self {
        Self {
            version: "1.0".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            format: "json".to_string(),
            encrypted,
            count: passwords.len(),
            passwords,
            metadata: None,
        }
    }

    /// 设置元数据
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// 验证导出数据
    pub fn validate(&self) -> Result<(), String> {
        if self.passwords.is_empty() {
            return Err("导出数据为空".to_string());
        }

        if self.count != self.passwords.len() {
            return Err("记录数量不匹配".to_string());
        }

        // 检查每条记录的完整性
        for (index, password) in self.passwords.iter().enumerate() {
            if password.site_name.is_empty() {
                return Err(format!("第{}条记录的网站名称为空", index + 1));
            }
            if password.site_url.is_empty() {
                return Err(format!("第{}条记录的网站URL为空", index + 1));
            }
            if password.username.is_empty() {
                return Err(format!("第{}条记录的用户名为空", index + 1));
            }
            if password.encrypted_password.is_empty() {
                return Err(format!("第{}条记录的密码为空", index + 1));
            }
        }

        Ok(())
    }
}