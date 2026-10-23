//! 「扩展」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 把描述层（`view::extensions`）的两分支映射成元素：
//! 未下发 ⇒ 标题 + 「未下发」提示（fail-closed）；已下发 ⇒ 标题 + 条目行清单。
//! 行**只读**：状态标签与动作文案是静态文本，⛔ 不渲染成可点按钮、不给增删改。
//! 颜色一律取自 `theme`（⛔ 不硬编码）、⛔ 界面不判定权限、⛔ 界面不持有凭据。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::extensions as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 渲染「扩展」面板：未下发显示提示，已下发显示只读行清单。
///
/// 服务端尚未接入下发通道 ⇒ 当前恒走未下发分支（⛔ 不伪造服务端数据）。
pub(super) fn render_extensions_panel(
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let b = panel_view::body(None);
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(panel_view::TITLE),
        )
        .child(match b {
            panel_view::Body::NotDelivered => div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::NOT_DELIVERED)
                .into_any_element(),
            panel_view::Body::Delivered(rows) => div()
                .flex()
                .flex_col()
                .gap_2()
                .children(rows.iter().map(|row| render_row(theme, row)))
                .into_any_element(),
        })
        .into_any_element()
}

/// 一条只读行：条目名 + 状态标签 + 当前可用动作文案。
///
/// ⛔ 纯展示：不给交互（行不可点、无增删改按钮）；参考的动作文案按状态展示为静态文本。
fn render_row(theme: &Theme, row: &panel_view::Row) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(row.name.clone()),
        )
        .children(row.tag().map(|tag| {
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(tag)
        }))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(row.action()),
        )
        .into_any_element()
}
