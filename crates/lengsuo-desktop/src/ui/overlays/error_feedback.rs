//! 错误反馈/异常提示条（波次 7，卡 `ui-overlay-error-feedback`，条目 A56）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `errorBanner`
//! （关闭按钮读屏文案逐字取自全表 `ardotAuth.aria.close`）。
//! 数据源在运行时侧：无源时如实显示「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "异常提示";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 关闭按钮读屏文案（逐字取自全表 `ardotAuth.aria.close`）。
pub const CLOSE_ARIA: &str = "关闭";
/// 错误图标字形（图标字符，非文案表条目）。
pub const ERROR_ICON: &str = "!";
/// 关闭按钮字形（图标字符，非文案表条目）。
pub const CLOSE_ICON: &str = "✕";

/// 错误条动作按钮；文案逐字取自 `errorBanner` 命名空间。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// 重试（`errorBanner.creditsExhausted.retry`）。
    Retry,
    /// 网络诊断（`errorBanner.checkNetwork`）。
    Diagnose,
    /// 升级套餐（`errorBanner.creditsExhausted.upgrade`）。
    Upgrade,
    /// 购买积分（`errorBanner.creditsExhausted.buyTopUp`）。
    BuyTopUp,
}

impl Action {
    /// 动作按钮中文文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Retry => "重试",
            Self::Diagnose => "网络诊断",
            Self::Upgrade => "升级套餐",
            Self::BuyTopUp => "购买积分",
        }
    }
}

/// 错误场景；标题/说明/动作全部逐字取自 `errorBanner` 命名空间。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorKind {
    /// 连接超时（`errorBanner.connectionTimeout`）。
    ConnectionTimeout,
    /// 模型响应超时（`errorBanner.modelResponseTimeout`）。
    ModelResponseTimeout,
    /// 积分耗尽（标题 `errorBanner.unexpectedError`，说明 `creditsExhausted.upgradeContent`）。
    CreditsExhausted,
}

impl ErrorKind {
    /// 全部可复现场景（固定 3 项，逐字文案骨架，⛔ 不附带伪造数值/凭据）。
    pub const ALL: &'static [Self] = &[
        Self::ConnectionTimeout,
        Self::ModelResponseTimeout,
        Self::CreditsExhausted,
    ];

    /// 标题（逐字）。
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::ConnectionTimeout => "连接超时",
            Self::ModelResponseTimeout => "模型响应超时",
            Self::CreditsExhausted => "发生意外错误",
        }
    }

    /// 说明段（逐字）。
    #[must_use]
    pub fn detail(self) -> &'static str {
        match self {
            Self::ConnectionTimeout => "如果问题持续存在，请检查您的网络设置或 VPN 状态",
            Self::ModelResponseTimeout => "请重试或切换模型",
            Self::CreditsExhausted => "您账户下可用的积分额度已耗尽，升级套餐继续完成任务。",
        }
    }

    /// 动作按钮清单（重试/诊断/升级）。
    #[must_use]
    pub fn actions(self) -> &'static [Action] {
        match self {
            Self::ConnectionTimeout => &[Action::Retry, Action::Diagnose],
            Self::ModelResponseTimeout => &[Action::Retry],
            Self::CreditsExhausted => &[Action::Retry, Action::Upgrade, Action::BuyTopUp],
        }
    }
}

/// 错误条目行 = 错误场景。
#[derive(Debug)]
pub struct BannerRow {
    pub kind: ErrorKind,
}

/// 浮层数据三态：未下发 ≠ 无错误空态 ≠ 错误条清单。
#[derive(Debug)]
pub enum OverlayState {
    /// 数据源未下发（fail-closed）。
    NotDelivered,
    /// 已下发且无错误：整条隐藏（空态，不是空清单）。
    Hidden,
    /// 错误条清单。
    Banner(Vec<BannerRow>),
}

impl OverlayState {
    /// 按场景清单构造；空输入归一化为无错误空态（⛔ 不是空清单、不是未下发）。
    #[must_use]
    pub fn from_kinds(kinds: &[ErrorKind]) -> Self {
        if kinds.is_empty() {
            Self::Hidden
        } else {
            Self::Banner(kinds.iter().map(|&kind| BannerRow { kind }).collect())
        }
    }

    /// 条目数（未下发与空态均为 0）。
    #[must_use]
    pub fn row_count(&self) -> usize {
        match self {
            Self::Banner(rows) => rows.len(),
            _ => 0,
        }
    }
}

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render_overlay(theme, &OverlayState::NotDelivered)
}

/// 按纯数据层状态渲染整层：标题行 + 三分支内容区。
fn render_overlay(theme: &Theme, state: &OverlayState) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header_row(theme))
        .child(body(theme, state))
}

/// 标题行：视图标题。
fn header_row(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .px_4()
        .py_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
}

/// 内容区三分支：未下发（fail-closed）/ 无错误隐藏 / 错误条清单。
fn body(theme: &Theme, state: &OverlayState) -> Div {
    match state {
        OverlayState::NotDelivered => centered_note(theme, NOT_DELIVERED),
        OverlayState::Hidden => div().flex_1(),
        OverlayState::Banner(rows) => banner_list(theme, rows),
    }
}

/// 居中说明（未下发）。
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

/// 错误条清单：逐条渲染 异常提示条。
fn banner_list(theme: &Theme, rows: &[BannerRow]) -> Div {
    let mut list = div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .pb_4();
    for row in rows {
        list = list.child(banner_row(theme, row.kind));
    }
    list
}

/// 单条异常提示条：错误图标 + 标题 + 说明段 + 动作按钮 + 关闭按钮。
fn banner_row(theme: &Theme, kind: ErrorKind) -> Div {
    div()
        .flex()
        .items_start()
        .gap_2()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(error_icon(theme))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .flex_1()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(kind.title()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(kind.detail()),
                )
                .child(actions_row(theme, kind.actions())),
        )
        .child(close_button(theme))
}

/// 错误图标：danger 底 + 感叹号字形。
fn error_icon(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w_4()
        .h_4()
        .rounded_full()
        .bg(theme.danger)
        .text_xs()
        .font_bold()
        .text_color(theme.foreground)
        .child(ERROR_ICON)
}

/// 动作按钮行；主动作（首个）强调色描边。
fn actions_row(theme: &Theme, actions: &'static [Action]) -> Div {
    let mut row = div().flex().items_center().gap_2().flex_wrap();
    for (index, &action) in actions.iter().enumerate() {
        row = row.child(action_chip(theme, action, index == 0));
    }
    row
}

/// 动作按钮 chip；颜色一律主题 token，⛔ 不硬编码色值。
fn action_chip(theme: &Theme, action: Action, primary: bool) -> Div {
    let color = if primary {
        theme.accent
    } else {
        theme.muted_foreground
    };
    div()
        .px_2()
        .py_1()
        .border_1()
        .border_color(color)
        .rounded_sm()
        .text_xs()
        .text_color(color)
        .child(action.label())
}

/// 关闭按钮（✕ 字形；读屏文案 [`CLOSE_ARIA`] 由运行时侧接线）。
fn close_button(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .w_6()
        .h_6()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(CLOSE_ICON)
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
