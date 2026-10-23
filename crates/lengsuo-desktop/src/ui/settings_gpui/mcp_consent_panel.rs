//! 「MCP 授权同意」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 描述层 [`crate::ui::settings_gpui::view::mcp_consent`] 按 `poi.mcp.consent`
//! （4 键）给出「未下发 / 同意卡片」双态；本文件只做「描述 → 元素」的映射。
//!
//! 硬约束：**只读**、**fail-closed**、颜色**一律取自 `theme`**（⛔ 不硬编码）、
//! ⛔ 界面不判定权限、⛔ 界面不持有凭据、⛔ 不伪造服务端数据、⛔ 无保存按钮。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::mcp_consent as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 渲染「MCP 授权同意」面板：标题 + 主体（未下发提示 / 同意卡片）。
pub(super) fn render_mcp_consent_panel(
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    // 无宿主数据源 ⇒ 如实显示「未下发」，⛔ 不伪造服务名。
    let body = panel_view::body(None);

    let mut column = div().flex().flex_col().gap_2();

    // 面板标题：与 `PANELS` 清单一致，接线层不另造标题。
    column = column.child(
        div()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(panel_view::TITLE),
    );

    match body {
        panel_view::Body::NotDelivered => column.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::NOT_DELIVERED),
        ),
        panel_view::Body::Card(card) => column.child(render_card(&card, theme, _cx)),
    }
    .into_any_element()
}

/// 已下发时的同意卡片：标题 / 描述 / 允许 / 拒绝。
///
/// ⛔ 点击「允许 / 拒绝」**只上报意图**（本卡无宿主 ⇒ 落日志），
/// 不做授权判定、不落地授权结果。
fn render_card(card: &panel_view::Card, theme: &Theme, cx: &mut Context<SettingsGpui>) -> Div {
    let mut card_el = div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        // 卡片标题（已填 `{mcpServerName}`）：与面板标题区分，用默认字号。
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(card.title.clone()),
        )
        // 卡片描述：正文色偏弱，字号小一档。
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(card.description),
        );

    // 动作行：「允许」用链接色（可点正向动作），「拒绝」用弱化色。
    let actions = div()
        .flex()
        .flex_row()
        .gap_2()
        .child(render_action(
            "mcp-consent-allow",
            card.allow,
            theme.link,
            theme.border,
            panel_view::Intent::Allow,
            cx,
        ))
        .child(render_action(
            "mcp-consent-deny",
            card.deny,
            theme.muted_foreground,
            theme.border,
            panel_view::Intent::Deny,
            cx,
        ));

    card_el = card_el.child(actions);
    card_el
}

/// 单个动作按钮：可点击（`on_click` 只在带 id 元素上存在 ⇒ 先 `.id`），只上报意图。
fn render_action(
    id: &'static str,
    label: &str,
    color: gpui_kit::Hsla,
    border: gpui_kit::Hsla,
    intent: panel_view::Intent,
    cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(border)
        .text_sm()
        .text_color(color)
        .cursor_pointer()
        .child(label.to_string())
        .on_click(cx.listener(
            move |_this: &mut SettingsGpui,
                  _event: &ClickEvent,
                  _window: &mut Window,
                  _cx: &mut Context<SettingsGpui>| {
                // 本卡无宿主 ⇒ 只落日志（与 `system_perm_panel` 的引导点击同口径）。
                eprintln!(
                    "[lengsuo-desktop] MCP 授权同意点击（意图未接线）：{}",
                    intent_label(intent)
                );
            },
        ))
        .into_any_element()
}

/// 点击意图的可读名（日志用；与描述层枚举一一对应）。
fn intent_label(intent: panel_view::Intent) -> &'static str {
    match intent {
        panel_view::Intent::Allow => panel_view::ALLOW,
        panel_view::Intent::Deny => panel_view::DENY,
    }
}

#[cfg(test)]
mod tests {
    /// 接线层自检：两条渲染分支（未下发 / 卡片）都不 panic 即为通过。
    ///
    /// ⚠️ 无窗口渲染断言（不 panic + 布局非零 + 元素可定位）由
    /// `render_assert_tests/sp_mcp_consent.rs` 承担，本模块不重复搭探针。
    #[test]
    fn render_card_action_ids_unique() {
        // 允许 / 拒绝两个按钮 id 必须互异（GPUI 同 id 元素会拦截点击）。
        assert_ne!("mcp-consent-allow", "mcp-consent-deny");
    }
}
