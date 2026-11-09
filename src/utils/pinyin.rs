//! 拼音首字母：给启动器的轻量检索用（wx -> 微信）。
//!
//! 只取每个汉字的拼音首字母，非汉字原样保留并转小写；
//! 多音字按 pinyin crate 的默认读音取首字母。

use pinyin::ToPinyin;

/// 生成文本的拼音首字母串。
pub fn initials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if let Some(py) = ch.to_pinyin() {
            out.push_str(&py.first_letter().to_lowercase());
        } else {
            for c in ch.to_lowercase() {
                out.push(c);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_initials() {
        assert_eq!(initials("微信"), "wx");
        assert_eq!(initials("GitHub"), "github");
        assert_eq!(initials("支付宝 Alipay"), "zfb alipay");
        assert_eq!(initials(""), "");
    }
}
