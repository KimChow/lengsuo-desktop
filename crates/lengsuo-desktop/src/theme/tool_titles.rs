//! 工具名 → 人话标题映射。
//!
//! 依据：参考客户端基线 —— **工具名对用户不可见**，卡片标题写「读取文件」而非 `Read`。
//! 本模块是纯映射，不依渲染；未知工具走「可读化」兜底而非原样暴露内部名。

/// 已知工具的人话标题前缀。
const KNOWN: &[(&str, &str)] = &[
    ("Read", "读取"),
    ("Write", "创建"),
    ("Edit", "修改"),
    ("Bash", "执行命令"),
    ("Grep", "搜索"),
    ("Glob", "查找文件"),
    ("WebFetch", "查看网页"),
    ("WebSearch", "搜索网页"),
];

/// 生成工具卡片标题（人话）。
///
/// - 已知工具：`前缀 + 参数摘要`（如 `读取 src/main.rs`）
/// - 未知工具：标题统一为「执行操作」，**不暴露内部工具名**
#[must_use]
pub fn tool_title(tool: &str, summary: Option<&str>) -> String {
    let label = title_of(tool);
    match summary {
        Some(s) if !s.is_empty() => format!("{label} {s}"),
        _ => label.to_owned(),
    }
}

/// 只取工具的中文类别名（用于分组、徽标等只需要类别的场景）。
#[must_use]
pub fn title_of(tool: &str) -> String {
    KNOWN
        .iter()
        .find(|(name, _)| *name == tool)
        .map_or_else(|| "执行操作".to_owned(), |(_, label)| (*label).to_owned())
}

/// 参数摘要裁剪：超长截断并加省略号，避免卡片被撑破。
#[must_use]
pub fn short_summary(s: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max_chars {
            out.push('…');
            return out;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_tools_get_human_titles() {
        assert_eq!(tool_title("Read", Some("src/main.rs")), "读取 src/main.rs");
        assert_eq!(tool_title("Bash", None), "执行命令");
        assert_eq!(tool_title("Grep", Some("fn main")), "搜索 fn main");
    }

    #[test]
    fn unknown_tool_does_not_leak_internal_name() {
        let t = tool_title("SecretInternalTool", Some("x"));
        assert!(
            !t.contains("SecretInternalTool"),
            "未知工具不得把内部名暴露给用户，实际得到：{t}"
        );
        assert_eq!(title_of("SecretInternalTool"), "执行操作");
    }

    #[test]
    fn empty_summary_falls_back_to_label_only() {
        assert_eq!(tool_title("Read", Some("")), "读取");
        assert_eq!(tool_title("Read", None), "读取");
    }

    #[test]
    fn short_summary_truncates_by_chars_not_bytes() {
        assert_eq!(short_summary("abcdef", 3), "abc…");
        assert_eq!(short_summary("abc", 3), "abc");
        assert_eq!(short_summary("中文测试", 2), "中文…");
        assert_eq!(short_summary("abc", 0), "");
    }
}
