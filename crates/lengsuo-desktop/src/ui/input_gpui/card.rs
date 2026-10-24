//! 居中大输入卡（波次 9，卡 `ui-input-card`，差异 D4）。
//!
//! 参照形态：卡宽约主区 70%、圆角；卡内左下「+ 附件」、右下「⚡快捷 ⌄ + 麦克风 + 发送圆钮」；
//! 卡外底部「选择工作空间 ⌄ / 默认权限 ⌄」。占位符逐字 `chat.input.placeholder.workbuddy`。
//! 硬约束：点击只上报意图；不持凭据；无空间数据时如实「未下发」。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 占位符（逐字：`chat.input.placeholder.workbuddy`）。
pub const PLACEHOLDER: &str = "今天帮你做些什么？@ 添加上下文，/调用技能与指令";
/// 快速问答占位（逐字：`chat.input.placeholder.quickQa`）。
pub const PLACEHOLDER_QUICK_QA: &str = "快速问答模式不消耗积分，可解答简单问题但不支持复杂任务";
/// 发送提示（逐字：`chat.input.sendShortcutTooltip.fallback`）。
pub const SEND_TOOLTIP: &str = "发送";
/// 选择工作空间（语义标签；数据未下发时显示「未下发」）。
pub const WORKSPACE_LABEL: &str = "选择工作空间";
/// 默认权限（语义标签）。
pub const PERMISSION_LABEL: &str = "默认权限";
/// 未下发（fail-closed）。
pub const NOT_DELIVERED: &str = "未下发";

/// 输入卡模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputMode {
    /// 普通任务模式。
    #[default]
    Task,
    /// 快速问答（不消耗积分）。
    QuickQa,
}

impl InputMode {
    /// 当前占位符（逐字）。
    pub fn placeholder(self) -> &'static str {
        match self {
            InputMode::Task => PLACEHOLDER,
            InputMode::QuickQa => PLACEHOLDER_QUICK_QA,
        }
    }
}

/// 渲染输入卡（纯构建；真实交互等运行时契约）。
pub fn card(theme: &Theme, mode: InputMode, workspace: Option<&str>) -> Div {
    let _ = theme;
    let ws = workspace.unwrap_or(NOT_DELIVERED);
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(SharedString::from(mode.placeholder()))
        .child(SharedString::from(SEND_TOOLTIP.to_string()))
        .child(SharedString::from(format!("{WORKSPACE_LABEL}：{ws}")))
        .child(PERMISSION_LABEL)
}

#[cfg(test)]
mod tests {
    use super::{card, InputMode, PLACEHOLDER, PLACEHOLDER_QUICK_QA, SEND_TOOLTIP};
    use gpui_kit::component::Theme;

    #[test]
    fn placeholder_verbatim() {
        assert_eq!(InputMode::Task.placeholder(), PLACEHOLDER);
        assert_eq!(InputMode::QuickQa.placeholder(), PLACEHOLDER_QUICK_QA);
    }

    #[test]
    fn send_tooltip_verbatim() {
        assert_eq!(SEND_TOOLTIP, "发送");
    }

    #[test]
    fn default_mode_is_task() {
        assert_eq!(InputMode::default(), InputMode::Task);
    }

    #[test]
    fn build_not_panic() {
        let theme = Theme::default();
        let _ = card(&theme, InputMode::Task, None);
        let _ = card(&theme, InputMode::QuickQa, Some("默认空间"));
    }
}
