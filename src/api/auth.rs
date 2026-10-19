use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use chrono::serde::{ts_seconds, ts_seconds_option};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub master_password: String,
    pub remember_me: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub session_id: Option<String>,
    pub error: Option<String>,
    #[serde(with = "ts_seconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrengthResult {
    pub is_valid: bool,
    pub score: i32,
    pub feedback: Vec<String>,
    pub suggestions: Vec<String>,
}

// 认证相关函数
pub async fn login(request: LoginRequest) -> LoginResponse {
    // 这里应该调用实际的认证逻辑
    LoginResponse {
        success: true,
        session_id: Some("session_id_123".to_string()),
        error: None,
        timestamp: Utc::now(),
    }
}

pub async fn logout(request: LogoutRequest) -> LoginResponse {
    // 这里应该调用实际的登出逻辑
    LoginResponse {
        success: true,
        session_id: None,
        error: None,
        timestamp: Utc::now(),
    }
}

pub async fn change_password(request: ChangePasswordRequest) -> LoginResponse {
    // 这里应该调用实际的密码修改逻辑
    LoginResponse {
        success: true,
        session_id: None,
        error: None,
        timestamp: Utc::now(),
    }
}

pub fn validate_password_strength(password: &str) -> PasswordStrengthResult {
    let mut score = 0;
    let mut feedback = Vec::new();
    let mut suggestions = Vec::new();

    // 检查长度
    if password.len() < 12 {
        feedback.push("密码长度至少需要12个字符".to_string());
        suggestions.push("使用更长的密码".to_string());
    } else {
        score += 25;
    }

    // 检查复杂度
    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_digit(10));
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    if !has_upper {
        feedback.push("密码需要包含大写字母".to_string());
        suggestions.push("添加至少一个大写字母".to_string());
    } else {
        score += 25;
    }

    if !has_lower {
        feedback.push("密码需要包含小写字母".to_string());
        suggestions.push("添加至少一个小写字母".to_string());
    } else {
        score += 25;
    }

    if !has_digit {
        feedback.push("密码需要包含数字".to_string());
        suggestions.push("添加至少一个数字".to_string());
    } else {
        score += 25;
    }

    if !has_special {
        feedback.push("密码需要包含特殊字符".to_string());
        suggestions.push("添加至少一个特殊字符".to_string());
    } else {
        score += 25;
    }

    // 检查常见密码
    let common_passwords = [
        "password", "123456", "qwerty", "letmein", "welcome",
        "admin", "password1", "12345678", "abc123", "password123"
    ];

    if common_passwords.contains(&password.to_lowercase().as_str()) {
        feedback.push("这是一个常见密码，容易被猜测".to_string());
        suggestions.push("使用更独特的密码".to_string());
        score = 0;
    }

    let is_valid = score >= 75; // 至少75分

    PasswordStrengthResult {
        is_valid,
        score,
        feedback,
        suggestions,
    }
}