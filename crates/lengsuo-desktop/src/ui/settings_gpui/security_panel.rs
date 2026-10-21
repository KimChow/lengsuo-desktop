//! 「安全中心」面板：展示服务端下发的能力标记（**只读**）。
//!
//! ⛔ 本面板**不做授权推理**：可用性完全来自服务端下发的 `can_*` 标记。
//! 判断全在描述层 [`crate::ui::settings_gpui::view::capability_rows`]，
//! 本文件只做「描述 → 元素」的映射。
//!
//! 两条硬约束：
//! - **只读清单**：行不可点击、不可提交任何意图（本卡范围内不实现点击上报）。
//! - **缺失 ≠ 不可用**：服务端没下发标记 ⇒ 显示「服务端未下发」，
//!   绝不显示成「所有能力都不可用」或干脆空白（会误导用户）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::org_perm::CapabilityFlags;
use crate::ui::settings_gpui::view::{
    capability_rows, has_delivered_marks, CapabilityRow, CapabilityStatus,
};

use super::window::SettingsGpui;

/// 渲染安全中心面板：9 个能力逐行列出可用性。
pub(super) fn render_security_panel(
    flags: &CapabilityFlags,
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    // 说明文案：讲清楚「只读」与「数据来自服务端」，避免用户误以为能改。
    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("以下状态由服务端下发，仅供查看；界面禁用不等于授权结果。"),
    );

    let rows = capability_rows(flags);
    if !has_delivered_marks(flags) {
        // fail-closed 的反面：**没有任何标记下发** ⇒ 明确说「未下发」，
        // 而不是让 9 行都显示「不可用」（那会让用户以为服务端判了拒绝）。
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

    for row in &rows {
        column = column.child(render_row(row, theme));
    }

    column.into_any_element()
}

/// 单行：左为能力中文名，右为可用性标记。
fn render_row(row: &CapabilityRow, theme: &Theme) -> Div {
    // 颜色一律取自主题：可用 = 成功色，不可用/未下发 = 弱化色。
    let (badge_color, badge_text) = match row.status {
        CapabilityStatus::Available => (theme.success, row.status.text()),
        CapabilityStatus::Denied(_) | CapabilityStatus::NotDelivered => {
            (theme.muted_foreground, row.status.text())
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
                .child(row.label)
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(row.capability.key()),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(badge_color)
                .whitespace_nowrap()
                .child(badge_text),
        )
}
