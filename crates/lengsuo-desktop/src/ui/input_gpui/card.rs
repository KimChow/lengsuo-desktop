//! 居中大输入卡（波次 9，卡 `ui-input-card-polish`，差异 D4）。
//!
//! 参照形态：卡宽约主区 70%、圆角、浅描边；卡内多行占位文案；左下「+」附件钮、
//! 右下「⚡快捷 ⌄ + 麦克风 + 圆形发送钮」；卡外底行「选择工作空间 ⌄」「默认权限 ⌄」。
//! 硬约束：
//! - 点击只上报意图枚举 [`InputCardIntent`]，不实现行为；
//! - 颜色一律 `theme.*`，不硬编码；
//! - 空间等数据未下发时如实显示「未下发」（fail-closed），不伪造。
//!
//! 原文一律逐字取自文案全表命名空间 `chat.input`（见各常量文档）。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 占位符（逐字：`chat.input.placeholder.workbuddy`）。
pub const PLACEHOLDER: &str = "今天帮你做些什么？@ 添加上下文，/调用技能与指令";
/// 快速问答占位（逐字：`chat.input.placeholder.quickQa`）。
pub const PLACEHOLDER_QUICK_QA: &str = "快速问答模式不消耗积分，可解答简单问题但不支持复杂任务";
/// 通用占位（逐字：`chat.input.placeholder`）。
pub const PLACEHOLDER_GENERIC: &str = "输入消息...";
/// 发送提示模板：双快捷键（逐字：`chat.input.sendShortcutTooltip`）。
pub const SEND_TOOLTIP_TEMPLATE: &str = "{sendShortcut} 发送，{newlineShortcut} 换行";
/// 发送提示兜底：无任何快捷键（逐字：`chat.input.sendShortcutTooltip.fallback`）。
pub const SEND_TOOLTIP_FALLBACK: &str = "发送";
/// 发送提示模板：仅发送键（逐字：`chat.input.sendShortcutTooltip.sendOnly`）。
pub const SEND_TOOLTIP_SEND_ONLY: &str = "{sendShortcut} 发送";
/// 发送提示模板：仅换行键（逐字：`chat.input.sendShortcutTooltip.newlineOnly`）。
pub const SEND_TOOLTIP_NEWLINE_ONLY: &str = "{newlineShortcut} 换行";
/// 模板占位符：发送快捷键。
const SEND_KEY: &str = "{sendShortcut}";
/// 模板占位符：换行快捷键。
const NEWLINE_KEY: &str = "{newlineShortcut}";
/// 选择工作空间（语义标签；数据未下发时显示 [`NOT_DELIVERED`]）。
pub const WORKSPACE_LABEL: &str = "选择工作空间";
/// 默认权限（语义标签）。
pub const PERMISSION_LABEL: &str = "默认权限";
/// 未下发（fail-closed：数据缺失时如实显示，不伪造）。
pub const NOT_DELIVERED: &str = "未下发";
/// 附件钮「+」（参照形态：左下）。
pub const ATTACH_GLYPH: &str = "+";
/// 快捷下拉触发器「⚡快捷 ⌄」（参照形态：右下）。
pub const QUICK_LABEL: &str = "⚡快捷";
/// 下拉箭头。
pub const CARET_GLYPH: &str = "⌄";
/// 麦克风钮（语义标签；图标钮文案未在 `chat.input` 命名空间下发）。
pub const MIC_LABEL: &str = "麦克风";
/// 发送圆钮 glyph（参照形态：右下圆形发送钮）。
pub const SEND_GLYPH: &str = "↑";

/// 输入卡上可点击的控件 → 意图。点击**只上报**本枚举，行为由宿主实现。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputCardIntent {
    /// 左下「+」：添加附件。
    Attach,
    /// 右下「⚡快捷 ⌄」：快捷指令下拉。
    QuickMenu,
    /// 右下麦克风：语音输入。
    Microphone,
    /// 右下圆形发送钮。
    Send,
    /// 卡外底行「选择工作空间 ⌄」。
    WorkspaceMenu,
    /// 卡外底行「默认权限 ⌄」。
    PermissionMenu,
}

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

/// 发送提示（逐字模板 + 快捷键代入；fail-closed：什么键都不知道 → 兜底「发送」）。
#[must_use]
pub fn send_tooltip(send: Option<&str>, newline: Option<&str>) -> String {
    match (send, newline) {
        (Some(send), Some(newline)) => SEND_TOOLTIP_TEMPLATE
            .replace(SEND_KEY, send)
            .replace(NEWLINE_KEY, newline),
        (Some(send), None) => SEND_TOOLTIP_SEND_ONLY.replace(SEND_KEY, send),
        (None, Some(newline)) => SEND_TOOLTIP_NEWLINE_ONLY.replace(NEWLINE_KEY, newline),
        (None, None) => SEND_TOOLTIP_FALLBACK.to_string(),
    }
}

/// 工作空间展示文案：未下发时如实「未下发」，不伪造。
#[must_use]
pub fn workspace_display(workspace: Option<&str>) -> &str {
    workspace.unwrap_or(NOT_DELIVERED)
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
        .child(SharedString::from(SEND_TOOLTIP_FALLBACK.to_string()))
        .child(SharedString::from(format!("{WORKSPACE_LABEL}：{ws}")))
        .child(PERMISSION_LABEL)
}

#[cfg(test)]
mod tests {
    use super::{
        card, send_tooltip, workspace_display, InputMode, NOT_DELIVERED, PLACEHOLDER,
        PLACEHOLDER_GENERIC, PLACEHOLDER_QUICK_QA, SEND_TOOLTIP_FALLBACK,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn placeholder_verbatim() {
        assert_eq!(
            PLACEHOLDER,
            "今天帮你做些什么？@ 添加上下文，/调用技能与指令"
        );
        assert_eq!(
            PLACEHOLDER_QUICK_QA,
            "快速问答模式不消耗积分，可解答简单问题但不支持复杂任务"
        );
        assert_eq!(PLACEHOLDER_GENERIC, "输入消息...");
        assert_eq!(InputMode::Task.placeholder(), PLACEHOLDER);
        assert_eq!(InputMode::QuickQa.placeholder(), PLACEHOLDER_QUICK_QA);
    }

    #[test]
    fn send_tooltip_full_matrix() {
        assert_eq!(
            send_tooltip(Some("Ctrl+Enter"), Some("Shift+Enter")),
            "Ctrl+Enter 发送，Shift+Enter 换行"
        );
        assert_eq!(send_tooltip(Some("Ctrl+Enter"), None), "Ctrl+Enter 发送");
        assert_eq!(send_tooltip(None, Some("Shift+Enter")), "Shift+Enter 换行");
    }

    #[test]
    fn send_tooltip_fallback_when_no_shortcuts() {
        assert_eq!(send_tooltip(None, None), SEND_TOOLTIP_FALLBACK);
        assert_eq!(SEND_TOOLTIP_FALLBACK, "发送");
    }

    #[test]
    fn workspace_display_fail_closed() {
        assert_eq!(workspace_display(None), NOT_DELIVERED);
        assert_eq!(workspace_display(Some("默认空间")), "默认空间");
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
