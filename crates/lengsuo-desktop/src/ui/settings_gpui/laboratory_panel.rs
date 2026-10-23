//! 「实验室」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 把描述层 [`panel_view::body`] 的两分支映射成只读元素：未下发 ⇒ 标题 +
//! 「未下发」提示；已下发 ⇒ 标题 + 区块（区块标题 + 只读行：标签 + 下方说明）。
//! 行⛔ 不可点、不给增删改；开关只展示状态文案，⛔ 不实现开关语义、不落盘。
//!
//! ⚠️ `SettingsGpui` 尚无实验室频道状态源（需改 `window.rs`，归主理人卡）⇒
//! 生产分发路径恒走「未下发」分支（fail-closed，⛔ 不伪造服务端数据）；
//! 「有内容」分支由 [`render_body`] 全量实现并被测试驱动，状态源落地后接线只差一行。
//!
//! 硬约束：颜色**一律取自 `theme.*`**（⛔ 不硬编码）、⛔ 界面不判定权限、
//! ⛔ 界面不持有凭据。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::laboratory as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 渲染「实验室」面板（只读：未下发 ⇒ 提示；已下发 ⇒ 区块行清单）。
pub(super) fn render_laboratory_panel(
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    // ponytail: 状态源落地前恒走「未下发」分支（fail-closed，不伪造服务端数据）
    let body = panel_view::body(None);
    render_body(theme, &body)
}

/// 把描述层主体映射成元素（「未下发」/「有内容」两条分支共用，测试直接驱动）。
fn render_body(theme: &Theme, body: &panel_view::Body) -> AnyElement {
    let mut panel = div().flex().flex_col().gap_2().child(
        div()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(panel_view::TITLE),
    );
    match body {
        panel_view::Body::NotDelivered => {
            panel = panel.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(panel_view::NOT_DELIVERED),
            );
        }
        panel_view::Body::Delivered(sections) => {
            for section in sections {
                panel = panel.child(render_section(theme, section));
            }
        }
    }
    panel.into_any_element()
}

/// 渲染一个区块：区块标题 + 只读行清单。
fn render_section(theme: &Theme, section: &panel_view::Section) -> AnyElement {
    let mut el = div().flex().flex_col().gap_1().child(
        div()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(section.title),
    );
    for row in &section.rows {
        el = el.child(render_row(theme, row));
    }
    el.into_any_element()
}

/// 渲染一条只读行：标签（前景色）+ 下方说明（弱化色；空说明只渲染标签）。
fn render_row(theme: &Theme, row: &panel_view::Row) -> AnyElement {
    let mut el = div().flex().flex_col().gap_1().pl_2().child(
        div()
            .text_sm()
            .text_color(theme.foreground)
            .child(row.label),
    );
    if !row.value.is_empty() {
        el = el.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(row.value),
        );
    }
    el.into_any_element()
}
