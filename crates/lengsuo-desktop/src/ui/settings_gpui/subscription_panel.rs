//! 「订阅用量」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 描述层见 [`super::view::subscription`]：文案逐字取自文案全表
//! `subscriptionUsage` 命名空间；本文件只把描述层的判定映射成元素。
//!
//! ⚠️ 当前**没有订阅用量数据源**（服务端未下发 ⇒ fail-closed 渲染「未下发」
//! 分支）；`Content` 分支已完整落地，待数据层接入后把快照传给
//! [`super::view::subscription::body`] 即可切换。
//!
//! 硬约束：**只读**（行不可点、不给增删改）、**fail-closed**、颜色**一律取自
//! `theme.*`**（⛔ 不硬编码）、⛔ 界面不判定权限、⛔ 界面不持有凭据、
//! ⛔ 不伪造服务端数据、⛔ 不实现支付（操作区只展示，意图上报待交互通道接入）。
#![cfg(feature = "ui")]
use super::view::subscription as panel_view;
use super::window::SettingsGpui;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 渲染「订阅用量」面板（无数据源 ⇒ fail-closed 走「未下发」分支）。
pub(super) fn render_subscription_panel(
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    render_body(theme, &panel_view::body(None))
}

/// 把描述层判定映射成元素（两条分支都在这里落地，便于无窗口断言）。
fn render_body(theme: &Theme, body: &panel_view::SubscriptionBody) -> AnyElement {
    let panel = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(section_title(theme, panel_view::TITLE));
    match body {
        panel_view::SubscriptionBody::NotDelivered => panel
            .child(muted_line(theme, panel_view::NOT_DELIVERED))
            .into_any_element(),
        panel_view::SubscriptionBody::Content(content) => render_content(theme, panel, content),
    }
}

/// 有内容分支：汇总区 + 积分明细 + 到期提示 + 操作区。
fn render_content(theme: &Theme, panel: Div, content: &panel_view::Content) -> AnyElement {
    let mut panel = panel;
    // 汇总区。
    if content.auto_renew {
        panel = panel.child(
            div()
                .text_sm()
                .text_color(theme.accent)
                .child(panel_view::AUTO_RENEW_ON),
        );
    }
    panel = panel
        .child(muted_line(theme, &content.expire_time))
        .child(row(
            theme,
            content.total_credits.label,
            &content.total_credits.value,
        ))
        // 积分明细区。
        .child(section_title(theme, panel_view::BREAKDOWN_TITLE));
    if content.breakdown.is_empty() {
        // fail-closed：下发为空也必须显示出来，⛔ 不退化成空白面板。
        panel = panel.child(muted_line(theme, panel_view::DETAIL_EMPTY));
    } else {
        for line in &content.breakdown {
            panel = panel.child(render_breakdown_row(theme, line));
        }
        panel = panel.child(muted_line(theme, panel_view::VIEW_DETAIL));
    }
    if let Some(nearest) = &content.nearest_expire {
        panel = panel.child(muted_line(theme, nearest));
    }
    if let Some(refresh) = &content.refresh_time {
        panel = panel.child(muted_line(theme, refresh));
    }
    if let Some(hint) = content.expiry_hint {
        panel = panel.child(div().text_sm().text_color(theme.accent).child(hint));
    }
    // 操作区（⛔ 只展示：不实现支付、不判定权限、不可点）。
    let mut actions = div().flex().gap_2();
    for action in &content.actions {
        actions = actions.child(action_chip(theme, action));
    }
    panel.child(actions).into_any_element()
}

/// 积分明细单行：分组名 + 状态徽标，第二行为用量（失败行不渲染百分比）。
fn render_breakdown_row(theme: &Theme, line: &panel_view::BreakdownRow) -> Div {
    let mut row = div().flex().flex_col().gap_1().child(
        div()
            .flex()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(line.label),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(if line.active {
                        theme.accent
                    } else {
                        theme.muted_foreground
                    })
                    .child(if line.active {
                        panel_view::DETAIL_ACTIVE
                    } else {
                        panel_view::DETAIL_INACTIVE
                    }),
            ),
    );
    if !line.remain.is_empty() {
        row = row.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(format!("{} {}", line.used, line.remain)),
        );
    }
    row
}

/// 只读行：label 左、value 右（⛔ 不可点、不给增删改）。
fn row(theme: &Theme, label: &'static str, value: &str) -> Div {
    div()
        .flex()
        .justify_between()
        .gap_2()
        .child(div().text_sm().text_color(theme.foreground).child(label))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(value.to_owned()),
        )
}

/// 小节标题。
fn section_title(theme: &Theme, text: &'static str) -> Div {
    div()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(text)
}

/// 弱化说明行（`text_sm`）。
fn muted_line(theme: &Theme, text: &str) -> Div {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}

/// 操作区展示片（⛔ 只展示，不接交互）。
fn action_chip(theme: &Theme, text: &'static str) -> Div {
    div()
        .text_sm()
        .text_color(theme.foreground)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .px_2()
        .py_1()
        .child(text)
}
