//! 居中大输入卡（波次 9，卡 `ui-input-card-polish`，差异 D4）。
//!
//! 参照形态：卡宽约主区 70%、大圆角、浅描边；卡内多行占位文案；左下「+」附件钮、
//! 右下「⚡快捷 ⌄ + 麦克风 + 圆形发送钮」；卡外底行「选择工作空间 ⌄」「默认权限 ⌄」。
//! 硬约束：
//! - 点击只上报意图枚举 [`InputCardIntent`]，不实现行为；
//! - 颜色/圆角一律 `theme.*` token，不硬编码；
//! - 空间等数据未下发时如实显示「未下发」（fail-closed），不伪造。
//!
//! 原文一律逐字取自文案全表（`chat.input.*` 及 `taskStarter.*`，见各常量文档）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
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
/// 选择工作空间（逐字：`taskStarter.selectWorkspace`；数据未下发时显示 [`NOT_DELIVERED`]）。
pub const WORKSPACE_LABEL: &str = "选择工作空间";
/// 默认权限（逐字：`automation.permission.default`）。
pub const PERMISSION_LABEL: &str = "默认权限";
/// 未下发（fail-closed：数据缺失时如实显示，不伪造）。
pub const NOT_DELIVERED: &str = "未下发";
/// 附件钮「+」（参照形态：卡内左下）。
pub const ATTACH_GLYPH: &str = "+";
/// 快捷下拉触发器文案（参照形态：卡内右下「⚡快捷 ⌄」）。
pub const QUICK_LABEL: &str = "⚡快捷";
/// 下拉箭头（参照形态）。
pub const CARET_GLYPH: &str = "⌄";
/// 麦克风钮（语义标签；图标钮文案未在全表下发，参照形态为麦克风图标）。
pub const MIC_LABEL: &str = "麦克风";
/// 发送圆钮 glyph（参照形态：卡内右下圆形发送钮）。
pub const SEND_GLYPH: &str = "↑";
/// 触摸/命中目标边长（无障碍：≥48）。
const HIT: f32 = 48.;

/// 输入卡上可点击的控件 → 意图。点击**只上报**本枚举，行为由宿主实现。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputCardIntent {
    /// 卡内左下「+」：添加附件。
    Attach,
    /// 卡内右下「⚡快捷 ⌄」：快捷指令下拉。
    QuickMenu,
    /// 卡内右下麦克风：语音输入。
    Microphone,
    /// 卡内右下圆形发送钮。
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

/// 渲染输入卡：卡面 + 卡外底行。纯构建，点击经 `on_intent` 上报。
pub fn card(
    theme: &Theme,
    mode: InputMode,
    workspace: Option<&str>,
    on_intent: impl Fn(InputCardIntent) + Clone + 'static,
) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .w_full()
        .child(card_face(theme, mode, on_intent.clone()))
        .child(under_row(theme, workspace, on_intent))
}

/// 卡面：70% 宽、大圆角、浅描边；占位文案 + 底部左右操作行。
fn card_face(
    theme: &Theme,
    mode: InputMode,
    on_intent: impl Fn(InputCardIntent) + Clone + 'static,
) -> AnyElement {
    div()
        .id("input-card-face")
        .w(relative(0.7))
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .text_color(theme.foreground)
        // 多行占位文案：min 高度撑出多行空间，muted 色按模式逐字显示。
        .child(
            div()
                .id("input-card-placeholder")
                .min_h(px(96.))
                .line_height(relative(1.6))
                .text_color(theme.muted_foreground)
                .child(mode.placeholder()),
        )
        // 底行：左下「+」，右下「⚡快捷 ⌄ + 麦克风 + 发送圆钮」。
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(round_icon(
                    "input-card-attach",
                    ATTACH_GLYPH,
                    theme,
                    on_intent.clone(),
                    InputCardIntent::Attach,
                ))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(pill_trigger(
                            "input-card-quick",
                            QUICK_LABEL,
                            theme,
                            on_intent.clone(),
                            InputCardIntent::QuickMenu,
                        ))
                        .child(pill_trigger(
                            "input-card-mic",
                            MIC_LABEL,
                            theme,
                            on_intent.clone(),
                            InputCardIntent::Microphone,
                        ))
                        .child(send_circle(theme, on_intent)),
                ),
        )
        .into_any_element()
}

/// 卡外底行：「选择工作空间：X ⌄」「默认权限 ⌄」（X 未下发时如实「未下发」）。
fn under_row(
    theme: &Theme,
    workspace: Option<&str>,
    on_intent: impl Fn(InputCardIntent) + Clone + 'static,
) -> Div {
    let ws = SharedString::from(format!(
        "{WORKSPACE_LABEL}：{}",
        workspace_display(workspace)
    ));
    let permission = PERMISSION_LABEL;
    div()
        .w(relative(0.7))
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap_6()
        .text_color(theme.muted_foreground)
        .child(pill_trigger(
            "input-card-workspace",
            &ws,
            theme,
            on_intent.clone(),
            InputCardIntent::WorkspaceMenu,
        ))
        .child(pill_trigger(
            "input-card-permission",
            permission,
            theme,
            on_intent,
            InputCardIntent::PermissionMenu,
        ))
}

/// 圆形图标钮（命中区 ≥48）：+ / 麦克风 / 发送等。
fn round_icon(
    id: &'static str,
    glyph: &'static str,
    theme: &Theme,
    on_intent: impl Fn(InputCardIntent) + Clone + 'static,
    intent: InputCardIntent,
) -> AnyElement {
    div()
        .id(id)
        .size(px(HIT))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme.button_secondary)
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .on_click(move |_, _, _| on_intent(intent))
        .child(glyph)
        .into_any_element()
}

/// 胶囊触发器（命中区 ≥48）：⚡快捷 ⌄ / 工作空间 ⌄ / 权限 ⌄。
fn pill_trigger(
    id: &'static str,
    label: &str,
    theme: &Theme,
    on_intent: impl Fn(InputCardIntent) + Clone + 'static,
    intent: InputCardIntent,
) -> AnyElement {
    div()
        .id(id)
        .h(px(HIT))
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .rounded_full()
        .bg(theme.button_secondary)
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .on_click(move |_, _, _| on_intent(intent))
        .child(SharedString::from(label))
        .child(CARET_GLYPH)
        .into_any_element()
}

/// 圆形发送钮：主色底、≥48 命中区，点击只上报 [`InputCardIntent::Send`]。
fn send_circle(theme: &Theme, on_intent: impl Fn(InputCardIntent) + Clone + 'static) -> AnyElement {
    div()
        .id("input-card-send")
        .size(px(HIT))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(theme.button_primary)
        .text_color(theme.button_primary_foreground)
        .font_bold()
        .cursor_pointer()
        .on_click(move |_, _, _| on_intent(InputCardIntent::Send))
        .child(SEND_GLYPH)
        .into_any_element()
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
        let noop = |_: super::InputCardIntent| ();
        let _ = card(&theme, InputMode::Task, None, noop);
        let _ = card(&theme, InputMode::QuickQa, Some("默认空间"), noop);
    }
}
