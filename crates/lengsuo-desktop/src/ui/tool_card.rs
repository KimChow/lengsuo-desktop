//! 工具调用卡片（默认折叠）。
//!
//! 规则 R13-1：**工具名对用户不可见**。卡片标题一律经
//! [`crate::theme::tool_titles`] 生成（如「读取 src/main.rs」），
//! 不直接显示 `msg.tool` 原文；未知工具由映射层兜底为「执行操作」。
//!
//! 本模块只产出渲染描述，不依赖 GPUI。

use crate::state::session_stream::SessionMessage;
use crate::theme::tool_titles::{short_summary, tool_title};

/// 标题里参数摘要的最大字符数（按字符而非字节裁剪，避免切坏中文）。
const MAX_SUMMARY_CHARS: usize = 60;

/// 一张工具调用卡片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCard {
    /// 人话标题，含参数摘要（如「读取 src/main.rs」）。
    pub title: String,
    /// 已裁剪的参数摘要（供展开态或悬浮提示使用）。
    pub summary: String,
    /// 是否折叠。`tool_use` 默认折叠。
    pub folded: bool,
}

impl ToolCard {
    /// 由工具名与参数正文构建卡片。
    ///
    /// `tool` 为 `None` 或未知值时不会泄漏内部名：标题由映射层兜底。
    #[must_use]
    pub fn from_tool(tool: Option<&str>, content: &str) -> Self {
        let summary = short_summary(content.trim(), MAX_SUMMARY_CHARS);
        let label = tool_title(
            tool.unwrap_or_default(),
            (!summary.is_empty()).then_some(&summary),
        );
        Self {
            title: label,
            summary,
            folded: true,
        }
    }

    /// 从会话消息构建（`ToolCall` 角色）。
    #[must_use]
    pub fn from_message(msg: &SessionMessage) -> Self {
        Self::from_tool(msg.tool.as_deref(), &msg.content)
    }

    /// 展开 / 折叠。
    pub fn toggle(&mut self) {
        self.folded = !self.folded;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::session_stream::MessageRole;

    fn call(tool: Option<&str>, content: &str) -> SessionMessage {
        SessionMessage {
            role: MessageRole::ToolCall,
            content: content.into(),
            tool: tool.map(str::to_owned),
            ts_ms: 0,
        }
    }

    #[test]
    fn known_tool_title_is_human_readable() {
        let card = ToolCard::from_message(&call(Some("Read"), "src/main.rs"));
        assert_eq!(card.title, "读取 src/main.rs");
        assert!(card.folded, "工具调用卡片默认折叠");
    }

    #[test]
    fn unknown_tool_never_leaks_internal_name() {
        let card = ToolCard::from_message(&call(Some("SecretInternalTool"), "payload"));
        assert!(
            !card.title.contains("SecretInternalTool")
                && !card.summary.contains("SecretInternalTool"),
            "未知工具名不得出现在卡片上，实际：{}",
            card.title
        );
        assert_eq!(card.title, "执行操作 payload");
    }

    #[test]
    fn missing_tool_still_renders() {
        let card = ToolCard::from_message(&call(None, ""));
        assert_eq!(card.title, "执行操作");
        assert_eq!(card.summary, "");
    }

    #[test]
    fn long_arguments_are_truncated() {
        let card = ToolCard::from_tool(Some("Grep"), &"x".repeat(100));
        assert!(card.summary.ends_with('…'));
        assert_eq!(card.summary.chars().count(), MAX_SUMMARY_CHARS + 1);
    }

    #[test]
    fn toggle_switches_folded_state() {
        let mut card = ToolCard::from_tool(Some("Bash"), "ls");
        card.toggle();
        assert!(!card.folded);
        card.toggle();
        assert!(card.folded);
    }
}
