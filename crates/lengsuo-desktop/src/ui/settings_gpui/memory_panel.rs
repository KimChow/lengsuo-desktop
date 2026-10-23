//! 「记忆」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 布局按参考面板（MemorySettingsPanel）的形态自搭 `div()`：开场说明 →
//! 功能行区（对话记忆 / 管理记忆 / 查看和编辑记忆 / 本地记忆，右侧只读镜像
//! 参考面板的按钮文字）→「来自对话的记忆」区块（fail-closed 双分支）→ 隐私说明。
//! 行**只读**（不可点、不给增删改）；颜色一律 `theme.*`（⛔ 不硬编码）；
//! ⛔ 界面不判定权限、⛔ 界面不持有凭据、⛔ 不伪造服务端数据。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::memory as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 功能行：左列名 + 说明，右列只读镜像参考面板的按钮文字（无点击行为）。
fn feature_row(theme: &Theme, row: &panel_view::Row) -> Div {
    div()
        .flex()
        .items_start()
        .justify_between()
        .gap_4()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(row.label),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(row.value.clone()),
                ),
        )
        // Option<Div> 即可迭代 ⇒ 无 action 时不占位、不加空壳节点。
        .children(row.action.map(|action| {
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(action)
        }))
}

/// 「来自对话的记忆」区块（fail-closed 双分支）：
/// 有内容 ⇒ 标题 + 行清单；未下发 ⇒ 明示「未下发」，⛔ 不退化成空区块。
fn chat_section(theme: &Theme, body: panel_view::Body) -> Div {
    match body {
        panel_view::Body::NotDelivered => div()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(panel_view::NOT_DELIVERED),
        panel_view::Body::Rows(rows) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(panel_view::CHAT_MEMORY_TITLE),
            )
            .children(rows.into_iter().map(|row| feature_row(theme, &row))),
    }
}

/// 渲染「记忆」面板。
pub(super) fn render_memory_panel(theme: &Theme, _cx: &mut Context<SettingsGpui>) -> AnyElement {
    // ponytail: 服务端更新时间接口未接线 ⇒ 恒走未下发分支，接线后换成 Some(...)
    panel_element(theme, panel_view::body(None)).into_any_element()
}

/// 面板主体（`body` 由调用方决定 ⇒ 测试可覆盖「有内容」分支）。
fn panel_element(theme: &Theme, body: panel_view::Body) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        // 开场说明（intro 文案随客户端下发，{name} 槽位原样呈现）
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::INTRO),
        )
        // 功能行区：左名右说明 + 只读按钮镜像
        .children(
            panel_view::feature_rows()
                .into_iter()
                .map(|row| feature_row(theme, &row))
                .collect::<Vec<_>>(),
        )
        .child(chat_section(theme, body))
        // 隐私说明：三段逐字拼接（privacy.prefix + linkText + suffix）
        .child(
            div()
                .p_3()
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(panel_view::privacy_line()),
                ),
        )
}
