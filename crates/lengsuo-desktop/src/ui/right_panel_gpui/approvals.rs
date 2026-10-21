//! 右侧面板「审批待办」tab 的**审批卡片**渲染。
//!
//! 硬约束：
//! - ⛔ **不伪造审批决策**：只有用户点击按钮才经 [`approval_card::decide`] 落决策，
//!   渲染本身绝不替用户放行。
//! - 颜色一律取自 `cx.theme()`，不硬编码。

use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use super::RightPanelGpui;
use crate::ui::approval_card::{self, ApprovalAction, ApprovalButton, ApprovalCardView};

/// 一个审批按钮。
///
/// 只有 `enabled` 的按钮才挂点击处理；点击经 [`approval_card::decide`] 落决策，
/// 接线层自己**不**判定「允许」。
fn render_approval_button(
    card: &ApprovalCardView,
    button: &ApprovalButton,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let enabled = button.enabled;
    let (background, foreground) = match button.action {
        ApprovalAction::Allow => (theme.button_success, theme.button_success_foreground),
        ApprovalAction::Deny => (theme.button_danger, theme.button_danger_foreground),
        ApprovalAction::AlwaysAllow => (theme.button_warning, theme.button_warning_foreground),
    };
    let mut element = div()
        .id(format!(
            "right-panel-approval-{}-{}",
            card.id,
            button.action.label()
        ))
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_xs();
    element = if enabled {
        element
            .bg(background)
            .text_color(foreground)
            .cursor_pointer()
    } else {
        element.text_color(theme.muted_foreground)
    };
    let element = element.child(button.label);

    if enabled {
        let entity = entity.clone();
        let id = card.id.clone();
        let action = button.action;
        element.on_click(move |_event, _window, app| {
            entity.update(app, |view, cx| {
                approval_card::decide(view.session_mut(), &id, action);
                cx.notify();
            });
        })
    } else {
        element
    }
}

/// 一张审批卡片。
pub(super) fn render_approval_card(
    card: &ApprovalCardView,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let (status_color, status_text) = if card.pending {
        (theme.button_warning, "等待你的决策")
    } else if card.resolved_allow == Some(true) {
        (theme.button_success, "已放行")
    } else {
        (theme.button_danger, "已拒绝")
    };
    let buttons = div().flex().flex_wrap().gap_2().children(
        card.buttons
            .iter()
            .map(|button| render_approval_button(card, button, entity, theme)),
    );

    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(card.title.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(card.summary.clone()),
        )
        .child(div().text_xs().text_color(status_color).child(status_text))
        .child(buttons)
}
