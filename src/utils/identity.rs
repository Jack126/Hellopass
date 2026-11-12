//! 条目身份标识：导入时判断两条记录是否指向同一个账号。
//!
//! 使用「名称 + 网址 + 用户名」三项作为标识，这样同一站点可以保存多个账号；
//! 名称与用户名忽略大小写和首尾空白，网址忽略协议、www. 前缀、大小写与末尾斜杠。

fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn normalize_username(username: &str) -> String {
    username.trim().to_lowercase()
}

/// 网址归一化：去掉协议、www. 前缀与末尾斜杠，其余（主机、端口、路径、参数）保留。
fn normalize_url(url: &str) -> String {
    let mut s = url.trim().to_lowercase();
    if let Some(idx) = s.find("://") {
        s = s[idx + 3..].to_string();
    }
    if let Some(rest) = s.strip_prefix("www.") {
        s = rest.to_string();
    }
    while s.ends_with('/') {
        s.pop();
    }
    s
}

/// 返回条目的身份标识：(名称, 网址, 用户名)。
pub fn entry_identity(site_url: &str, site_name: &str, username: &str) -> (String, String, String) {
    (
        normalize_name(site_name),
        normalize_url(site_url),
        normalize_username(username),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_is_normalized() {
        assert_eq!(normalize_url("https://www.Example.com/"), "example.com");
        assert_eq!(normalize_url(" http://example.com/login/ "), "example.com/login");
        assert_eq!(normalize_url("example.com"), "example.com");
        assert_eq!(normalize_url(""), "");
    }

    #[test]
    fn identity_uses_name_url_and_username() {
        assert_eq!(
            entry_identity("https://www.github.com/", "GitHub", " Alice "),
            (
                "github".to_string(),
                "github.com".to_string(),
                "alice".to_string()
            )
        );
        // 同一站点、不同账号：身份不同
        assert_ne!(
            entry_identity("https://github.com", "GitHub", "alice"),
            entry_identity("https://github.com", "GitHub", "bob")
        );
        // 同一站点同一用户名、不同名称：视为不同条目
        assert_ne!(
            entry_identity("https://github.com", "GitHub 工作", "alice"),
            entry_identity("https://github.com", "GitHub 私人", "alice")
        );
        // 路径不同也视为不同条目
        assert_ne!(
            entry_identity("https://example.com/a", "Example", "alice"),
            entry_identity("https://example.com/b", "Example", "alice")
        );
    }
}
