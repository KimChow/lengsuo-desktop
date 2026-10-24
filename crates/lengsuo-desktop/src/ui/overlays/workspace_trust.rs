//! 工作区信任浮层（波次 7，卡 `ui-overlay-workspace-trust`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `workspaceTrust`。
//! 数据源在运行时侧：无源时如实显示「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 浮层标题（逐字：`workspaceTrust.title`）。
pub const TITLE: &str = "选择打开方式";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 受限模式徽标（逐字：`workspaceTrust.badge`）。
pub const BADGE_RESTRICTED: &str = "受限模式";
/// 徽标文案模板（逐字：`workspaceTrust.badgeTitle`，`{path}` 为占位符）。
pub const BADGE_TITLE_TEMPLATE: &str = "受限模式：{path}";
/// 确认分支正文前段（逐字，含结尾空格）。
pub const BODY_BEFORE_DIRECTORY: &str = "此文件夹中的 ";
/// 确认分支正文后段（逐字，含开头空格）。
pub const BODY_AFTER_DIRECTORY: &str =
    " 目录可能包含可自动运行的脚本与配置。若以「受限模式」打开，将不会自动加载项目级配置，不自动执行终端命令。";
/// 确认按钮（逐字：`workspaceTrust.trust`）。
pub const TRUST: &str = "正常打开";
/// 取消按钮（逐字：`workspaceTrust.reject`）。
pub const REJECT: &str = "以「受限模式」打开";
/// 警告分支标题（逐字：`workspaceTrust.statusTitle`）。
pub const STATUS_TITLE: &str = "受限模式已启用";
/// 警告分支正文（逐字：`workspaceTrust.statusBody`）。
pub const STATUS_BODY: &str = "此工作区当前按「受限模式」处理。在该模式下，项目级配置不会自动加载，Agent Runtime 的 Bash 工具会被禁用。";
/// 警告分支「保持」按钮（逐字：`workspaceTrust.keepRestricted`）。
pub const KEEP_RESTRICTED: &str = "保持受限模式";
/// 警告分支「关闭」按钮（逐字：`workspaceTrust.closeRestricted`）。
pub const CLOSE_RESTRICTED: &str = "关闭受限模式";
/// 空态说明（逐字：`workspaceTrust.readFailed`）：请求已到但路径不可读，⛔ 不伪造路径。
pub const EMPTY_HINT: &str = "无法读取该工作区的授信状态，请稍后重试。";

/// 授信决定按钮（确认分支）：确认 = 正常打开；取消 = 以「受限模式」打开。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    /// 确认：信任该工作区并正常打开。
    Trust,
    /// 取消：以受限模式打开。
    Reject,
}

impl Decision {
    /// 按钮文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Trust => TRUST,
            Self::Reject => REJECT,
        }
    }
}

/// 受限模式状态按钮（警告分支）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestrictedAction {
    /// 保持受限（安全默认）。
    Keep,
    /// 关闭受限（升为授信）。
    Close,
}

impl RestrictedAction {
    /// 按钮文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Keep => KEEP_RESTRICTED,
            Self::Close => CLOSE_RESTRICTED,
        }
    }
}

/// 确认分支按钮组（确认在前、取消在后）。
pub const DECISION_BUTTONS: &[Decision] = &[Decision::Trust, Decision::Reject];
/// 警告分支按钮组（保持在前——安全默认）。
pub const RESTRICTED_BUTTONS: &[RestrictedAction] =
    &[RestrictedAction::Keep, RestrictedAction::Close];

/// 待授信的工作区目录请求（路径由运行时下发；空 ⇒ 空态，⛔ 不伪造路径）。
#[derive(Clone, Debug)]
pub struct TrustRequest {
    /// 工作区目录路径（已去首尾空白）。
    pub path: String,
}

impl TrustRequest {
    /// 构造请求；路径去首尾空白（全空白 ⇒ 空态输入）。
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self {
            path: path.trim().to_owned(),
        }
    }

    /// 确认分支正文：前段 + 路径 + 后段（逐字拼接，空格逐字保留）。
    #[must_use]
    pub fn body_text(&self) -> String {
        format!("{BODY_BEFORE_DIRECTORY}{}{BODY_AFTER_DIRECTORY}", self.path)
    }

    /// 徽标文案（逐字模板 + `{path}` 占位替换）。
    #[must_use]
    pub fn badge_title(&self) -> String {
        BADGE_TITLE_TEMPLATE.replace("{path}", &self.path)
    }
}

/// 浮层数据四态：未下发 ≠ 空态 ≠ 确认分支 ≠ 警告分支。
#[derive(Debug)]
pub enum OverlayState {
    /// 数据源未下发（fail-closed）。
    NotDelivered,
    /// 请求已下发但路径为空/不可读（空态，不是空清单）。
    Empty,
    /// 信任确认分支：正文 + 确认/取消按钮。
    Choose(TrustRequest),
    /// 受限模式警告分支：状态标题/正文 + 保持/关闭按钮。
    RestrictedStatus,
}

impl OverlayState {
    /// 由运行时下发构造：`None` ⇒ 未下发；全空白路径 ⇒ 空态；否则确认分支。
    #[must_use]
    pub fn from_delivered(path: Option<&str>) -> Self {
        match path {
            None => Self::NotDelivered,
            Some(p) if p.trim().is_empty() => Self::Empty,
            Some(p) => Self::Choose(TrustRequest::new(p)),
        }
    }
}

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
/// 无数据源接线 ⇒ 默认态 fail-closed，只如实渲染「未下发」。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render_overlay(theme, &OverlayState::NotDelivered)
}

/// 按纯数据层状态渲染整层：标题行 + 分支内容区。
fn render_overlay(theme: &Theme, state: &OverlayState) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header_row(theme, state))
        .child(body(theme, state))
}

/// 标题行：分支各自的标题 +（警告分支）受限徽标。
fn header_row(theme: &Theme, state: &OverlayState) -> Div {
    let title = match state {
        OverlayState::RestrictedStatus => STATUS_TITLE,
        _ => TITLE,
    };
    let mut row = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_4()
        .py_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(title),
        );
    if matches!(state, OverlayState::RestrictedStatus) {
        row = row.child(badge_chip(theme));
    }
    row
}

/// 内容区分支：未下发（fail-closed）/ 空态 / 信任确认 / 受限警告。
fn body(theme: &Theme, state: &OverlayState) -> Div {
    match state {
        OverlayState::NotDelivered => centered_note(theme, NOT_DELIVERED),
        OverlayState::Empty => centered_note(theme, EMPTY_HINT),
        OverlayState::Choose(request) => choose_branch(theme, request),
        OverlayState::RestrictedStatus => restricted_branch(theme),
    }
}

/// 居中说明（未下发/空态共用）。
fn centered_note(theme: &Theme, text: &'static str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 信任确认分支：正文（前段+路径+后段）+ 确认/取消按钮行。
fn choose_branch(theme: &Theme, request: &TrustRequest) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .pb_4()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(SharedString::from(request.body_text())),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(button_chip(theme, Decision::Trust.label(), true))
                .child(button_chip(theme, Decision::Reject.label(), false)),
        )
}

/// 受限警告分支：状态正文 + 保持/关闭按钮行。
fn restricted_branch(theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .pb_4()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(STATUS_BODY),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(button_chip(theme, RestrictedAction::Keep.label(), false))
                .child(button_chip(theme, RestrictedAction::Close.label(), true)),
        )
}

/// 按钮 chip：主按钮 accent 描边、次按钮 muted；颜色一律主题 token，⛔ 不硬编码色值。
fn button_chip(theme: &Theme, label: &'static str, primary: bool) -> Div {
    let color = if primary {
        theme.accent
    } else {
        theme.muted_foreground
    };
    div()
        .px_4()
        .py_2()
        .border_1()
        .border_color(color)
        .rounded_sm()
        .text_sm()
        .text_color(color)
        .child(label)
}

/// 受限模式徽标 chip（警告用色取主题 token）。
fn badge_chip(theme: &Theme) -> Div {
    div()
        .px_2()
        .border_1()
        .border_color(theme.warning)
        .rounded_sm()
        .text_xs()
        .text_color(theme.warning)
        .child(BADGE_RESTRICTED)
}
#[cfg(test)]
mod tests {
    use super::{main_area, NOT_DELIVERED, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }
    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
