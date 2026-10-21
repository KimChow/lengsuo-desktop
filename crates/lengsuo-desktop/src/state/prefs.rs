//! 主题偏好的**纯逻辑**序列化层：枚举 ⇄ JSON 文本。
//!
//! ⛔ 本模块**不碰文件**：层约定是「纯逻辑，无 IO」。
//! 文件读写由 [`crate::ui::gpui_app::launch`] 负责（那是唯一做 IO 的地方）。
//!
//! 格式刻意保持**极简且可读**：一个 JSON 对象，键 `theme`，值是主题的 slug。
//! 例：`{"theme":"dark"}`

use crate::ui::settings_gpui::view::ThemeChoice;

/// 偏好 JSON 的键名。
pub const THEME_KEY: &str = "theme";

/// 把主题取值序列化成偏好 JSON 文本。
#[must_use]
pub fn encode_theme(choice: ThemeChoice) -> String {
    format!("{{\"{THEME_KEY}\":\"{}\"}}", choice.slug())
}

/// 从偏好 JSON 文本解析主题取值。
///
/// **解析失败一律回落到 [`ThemeChoice::System`]**（跟随系统 = 不挑暗色 = 最保守），
/// 但**不 panic**、**不静默吞错**：本函数返回 `Result`，
/// 调用方决定是记日志还是提示。
pub fn decode_theme(text: &str) -> Result<ThemeChoice, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("偏好 JSON 解析失败：{e}"))?;
    let raw = value
        .get(THEME_KEY)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("偏好 JSON 缺键 {THEME_KEY}"))?;
    match raw {
        "light" => Ok(ThemeChoice::Light),
        "dark" => Ok(ThemeChoice::Dark),
        "system" => Ok(ThemeChoice::System),
        other => Err(format!("未知主题取值：{other}")),
    }
}

#[cfg(test)]
mod tests {
    // 显式列出符号：不用 `use super::*`（父模块的 `use` 会把测试属性宏带进来遮蔽 `#[test]`）。
    use super::{decode_theme, encode_theme, THEME_KEY};
    use crate::ui::settings_gpui::view::ThemeChoice;

    #[test]
    fn encode_theme_produces_readable_json() {
        let text = encode_theme(ThemeChoice::Dark);
        assert!(text.contains("\"dark\""), "编码结果应含 dark：{text}");
        assert!(
            text.contains(THEME_KEY),
            "编码结果应含键名 {THEME_KEY}：{text}"
        );
    }

    #[test]
    fn decode_theme_round_trips_all_three_choices() {
        for choice in [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark] {
            let text = encode_theme(choice);
            let decoded = decode_theme(&text).expect("自己编码的文本应能解回来");
            assert_eq!(decoded, choice, "往返应相等：{text}");
        }
    }

    #[test]
    fn decode_theme_rejects_unknown_slug() {
        let result = decode_theme("{\"theme\":\"neon\"}");
        assert!(result.is_err(), "未知取值应报错而不是 panic");
    }

    #[test]
    fn decode_theme_rejects_malformed_json() {
        let result = decode_theme("not json at all");
        assert!(result.is_err(), "坏 JSON 应报错而不是 panic");
    }

    #[test]
    fn decode_theme_rejects_missing_key() {
        let result = decode_theme("{\"other\":\"dark\"}");
        assert!(result.is_err(), "缺键应报错而不是 panic");
    }
}
