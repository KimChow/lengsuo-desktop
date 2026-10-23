//! 「Claw 设置」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 渲染分支由描述层 [`body`](crate::ui::settings_gpui::view::claw::body) 决定：
//! 服务端未下发通道状态 ⇒ 整体显示「未下发」提示；有内容 ⇒ 只读行清单。
//!
//! 硬约束：**只读**（行不可点、不给增删改）、**fail-closed**、
//! 颜色**一律取自 `cx.theme()`**（⛔ 不硬编码）、⛔ 界面不判定权限、
//! ⛔ 界面不持有凭据、⛔ 不伪造服务端数据。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::claw as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 渲染「Claw 设置」面板。
///
/// ponytail: 服务端通道状态下发后接进 `body()`；当前无后端 ⇒ 恒走「未下发」分支。
pub(super) fn render_claw_panel(theme: &Theme, _cx: &mut Context<SettingsGpui>) -> AnyElement {
    panel(theme, panel_view::body(&[])).into_any_element()
}
/// 标题 + 主体（两条渲染分支都在这里，便于无窗口单测）。
fn panel(theme: &Theme, body: panel_view::Body) -> Div {
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
        .child(body_element(theme, body))
}
/// 主体：未下发 ⇒ 「未下发」提示；有内容 ⇒ 区块标题/说明 + 行清单。
fn body_element(theme: &Theme, body: panel_view::Body) -> Div {
    match body {
        panel_view::Body::NotDelivered => div().flex().flex_col().gap_2().child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::NOT_DELIVERED),
        ),
        panel_view::Body::Rows(rows) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(panel_view::SECTION_TITLE),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(panel_view::SECTION_DESC),
            )
            .children(rows.into_iter().map(|row| row_element(theme, row))),
    }
}
/// 一行通道：上行「标题 + 状态」，下行集成说明。
fn row_element(theme: &Theme, row: panel_view::Row) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(row.label),
                )
                .child(div().text_sm().text_color(theme.accent).child(row.value)),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(row.desc),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 占位/内容两条渲染分支构树不 panic（未下发分支即生产当前分支）。
    // 全局导入 `gpui_kit::*` 会把 gpui 的 `#[test]` 宏引入作用域盖住内建属性，
    // 这里用全路径点名内建 `test`（与 gpui 自身宏展开同款写法）。
    #[::core::prelude::v1::test]
    fn both_branches_build_without_panic() {
        let theme = Theme::default();
        let _ = panel(&theme, panel_view::body(&[]));
        let _ = panel(
            &theme,
            panel_view::body(&[
                (
                    panel_view::ChannelId::WeCom,
                    panel_view::ChannelState::Connected,
                ),
                (
                    panel_view::ChannelId::Dingtalk,
                    panel_view::ChannelState::ViewOnly,
                ),
            ]),
        );
    }
}
