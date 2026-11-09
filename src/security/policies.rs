use serde::{Serialize, Deserialize};

/// 密码强度评估结果（供 UI 展示评分与改进建议）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordStrengthResult {
    pub is_valid: bool,
    pub score: i32,
    pub feedback: Vec<String>,
    pub suggestions: Vec<String>,
}

/// 评估密码强度：长度 + 大小写/数字/特殊字符 + 常见弱口令检测。
/// 说明：主密码的最终准入由 MasterPasswordManager 校验，本函数用于前端反馈。
pub fn validate_password_strength(password: &str) -> PasswordStrengthResult {
    let mut score = 0;
    let mut feedback = Vec::new();
    let mut suggestions = Vec::new();

    if password.len() < 12 {
        feedback.push("密码长度至少需要12个字符".to_string());
        suggestions.push("使用更长的密码".to_string());
    } else {
        score += 25;
    }

    let has_upper = password.chars().any(|c| c.is_uppercase());
    let has_lower = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
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

    let common_passwords = [
        "password", "123456", "qwerty", "letmein", "welcome",
        "admin", "password1", "12345678", "abc123", "password123",
    ];
    if common_passwords.contains(&password.to_lowercase().as_str()) {
        feedback.push("这是一个常见密码，容易被猜测".to_string());
        suggestions.push("使用更独特的密码".to_string());
        score = 0;
    }

    let is_valid = score >= 75;
    PasswordStrengthResult {
        is_valid,
        score,
        feedback,
        suggestions,
    }
}
