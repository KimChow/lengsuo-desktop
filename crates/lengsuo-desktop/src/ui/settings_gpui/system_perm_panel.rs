//! 「系统权限」面板：按拒绝原因分开展示服务端下发的能力位（**只读**）。
//!
//! ⛔ 本面板**不做授权推理**：可用性与拒绝原因完全来自服务端下发的 `can_*` 标记。
//! 判定全在描述层 [`crate::ui::settings_gpui::view::permission_display`]，
//! 本文件只做「描述 → 元素」的映射。
//!
//! 与「安全中心」的分工：
//! - 安全中心 = 总览（可用 / 不可用 / 未下发）；
//! - 本面板 = 原因细分 + 引导路径（套餐不含 / 角色无权限 / 企业策略禁止）。
//!
//! 硬约束：
//! - **只读**：可用性来自服务端；点「申请权限 / 升级购买」只**上报意图**，
//!   不实现任何申请 / 升级业务逻辑（宿主 / 运行时处理）。
//! - **策略拒绝只提示联系管理员**，不提供自助入口（`ContactAdmin` 无按钮）。
//! - **缺失 ≠ 不可用**：服务端没下发标记 ⇒ 显示「服务端未下发」；
//!   服务端拒绝但没给原因 ⇒ 显示「服务端未说明原因」（不得编一个原因）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::org_perm::{Capability, CapabilityFlags, Guidance};
use crate::ui::settings_gpui::view::{
    capability_label, has_delivered_marks, permission_display, PermissionDisplay,
};

use super::window::SettingsGpui;

/// 渲染系统权限面板：逐项列出能力 + 拒绝原因 + 引导路径。
pub(super) fn render_system_perm_panel(
    flags: &CapabilityFlags,
    theme: &Theme,
    cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    // 说明文案：讲清楚「只读」与「数据来自服务端」。
    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("以下状态由服务端下发，仅供查看；点击引导入口只上报意图，不直接生效。"),
    );

    if !has_delivered_marks(flags) {
        // fail-closed 的反面：**没有任何标记下发** ⇒ 明确说「未下发」，
        // 而不是让 9 行都显示「不可用 / 未说明原因」。
        column = column.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_4()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child("服务端未下发能力标记"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("无法判断当前组织的能力范围，请联系企业管理员确认。"),
                ),
        );
    }

    for cap in Capability::ALL {
        column = column.child(render_row(flags, cap, theme, cx));
    }

    column.into_any_element()
}

/// 单行：左为能力中文名，右为可用性 + 拒绝原因 + 引导路径。
fn render_row(
    flags: &CapabilityFlags,
    cap: Capability,
    theme: &Theme,
    cx: &mut Context<SettingsGpui>,
) -> Div {
    let right = match permission_display(flags, cap) {
        PermissionDisplay::Allowed => div()
            .text_xs()
            .text_color(theme.success)
            .whitespace_nowrap()
            .child("可用"),
        PermissionDisplay::NotProvided => div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .whitespace_nowrap()
            .child("服务端未下发"),
        PermissionDisplay::DeniedWithoutReason => div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .whitespace_nowrap()
            .child("服务端未说明原因"),
        PermissionDisplay::Denied { reason } => {
            // 拒绝原因（三种文案不可合并）+ 引导路径。
            let mut column = div().flex().flex_col().items_end().gap_1();
            column = column.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .whitespace_nowrap()
                    .child(reason.message()),
            );
            column = column.child(render_guidance(cap, reason.guidance(), theme, cx));
            column
        }
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .py_2()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(capability_label(cap))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(cap.key()),
                ),
        )
        .child(right)
}

/// 引导路径：自助入口渲染成可点击按钮，`ContactAdmin` 只提示文字。
///
/// ⚠️ 点击**只上报意图**：不实现申请 / 升级的业务逻辑，由宿主 / 运行时处理。
/// 本卡未接派发器 ⇒ 点击先把意图写进日志（与 `window.rs` 的偏好落盘日志同口径）。
fn render_guidance(
    cap: Capability,
    guidance: Guidance,
    theme: &Theme,
    cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    if guidance.is_self_service() {
        // 自助路径（升级购买 / 申请权限）：可点击，但只上报意图。
        div()
            .id(format!("perm-guide-{}", cap.key()))
            .px_2()
            .py_1()
            .rounded_sm()
            .text_xs()
            .text_color(theme.link)
            .cursor_pointer()
            .child(guidance.label())
            .on_click(cx.listener(
                move |_this: &mut SettingsGpui,
                      _event: &ClickEvent,
                      _window: &mut Window,
                      _cx: &mut Context<SettingsGpui>| {
                    eprintln!(
                        "[lengsuo-desktop] 系统权限引导点击（意图未接线）：{} → {}",
                        capability_label(cap),
                        guidance.label()
                    );
                },
            ))
            .into_any_element()
    } else {
        // 策略拒绝只提示联系管理员，不接自助申请入口。
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .whitespace_nowrap()
            .child(guidance.label())
            .into_any_element()
    }
}
