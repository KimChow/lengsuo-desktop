//! 「个性化」面板：展示本仓真实的快捷键绑定（**只读**）。
//!
//! ⛔ 本面板**不提供改键入口**：快捷键的持久化与冲突裁决不在本卡范围。
//! ⛔ 本面板**不做去重**：`Ctrl+K` 与 `Ctrl+Shift+P` 指向同一命令，但它们是
//! **两条不同的绑定**，都要列出来。
//!
//! 本文件只做「描述 → 元素」的映射：行清单与空态判定都在描述层
//! [`crate::ui::settings_gpui::view::hotkey_panel`]。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::hotkeys::Binding;
use crate::ui::settings_gpui::view::{hotkey_panel, HotkeyPanel, HotkeyRow, EMPTY_BINDINGS_TEXT};

use super::window::SettingsGpui;

/// 渲染个性化面板：逐条列出「命令 + 按键」。
pub(super) fn render_hotkey_panel(
    bindings: &[Binding],
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    // 说明文案：讲清楚「只读」，避免用户以为能改键。
    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("以下为本机已注册的快捷键，仅供查看；暂不支持在此修改。"),
    );

    match hotkey_panel(bindings) {
        // fail-closed：绑定表为空 ⇒ 明确说「没有可用的快捷键」，
        // 绝不渲染成空表或「一切正常」（那会让人以为有绑定却没显示）。
        HotkeyPanel::Empty => {
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
                            .child(EMPTY_BINDINGS_TEXT),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("未检测到已注册的快捷键，请检查应用是否正常加载。"),
                    ),
            );
        }
        HotkeyPanel::Rows(rows) => {
            for row in &rows {
                column = column.child(render_row(row, theme));
            }
        }
    }

    column.into_any_element()
}

/// 单行：左为命令说明，右为按键组合。
fn render_row(row: &HotkeyRow, theme: &Theme) -> Div {
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
                        .child(row.command_id),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.accent)
                .whitespace_nowrap()
                .child(row.keys.clone()),
        )
}
