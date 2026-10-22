//! 「个性化」面板：展示本仓真实的快捷键绑定，**可改键**（改即生效）。
//!
//! ⛔ 改键**不落盘**：只在本次运行内有效（持久化不在本卡范围）。
//! ⛔ 本面板**不做去重**：`Ctrl+K` 与 `Ctrl+Shift+P` 指向同一命令，但它们是
//! **两条不同的绑定**，都要列出来。
//!
//! 本文件只做「描述 → 元素」的映射：行清单与空态判定都在描述层
//! [`crate::ui::settings_gpui::view::hotkey_panel`]，改键的裁决也在那里
//! （本面板**不自己实现冲突检测**）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::hotkeys::Binding;
use crate::ui::settings_gpui::view::{hotkey_panel, HotkeyPanel, HotkeyRow, EMPTY_BINDINGS_TEXT};

use super::window::SettingsGpui;

/// 面板顶部的说明文案。
const PANEL_HINT: &str = "点击某一行的按键组合即可改键，改完立即生效。";

/// 改键状态（接线层的投影）。
///
/// ⚠️ 描述层的 `HotkeyEdit` 住在 `view` 的私有子模块里 ⇒ **不可命名**
/// （`view/mod.rs` 的 `pub use` 清单不在本卡边界内）⇒ 这里只存
/// 「目标命令 id + 提示文案」两样，够接线层用。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct EditState {
    /// 正在改键的目标命令 id（`None` ⇒ 不在改键中）。
    pub(super) target: Option<&'static str>,
    /// 状态提示文案（`None` ⇒ 不渲染状态区）。
    pub(super) status: Option<String>,
}

/// 渲染个性化面板：逐条列出「命令 + 按键」，按键组合**可点**（点了开始改键）。
pub(super) fn render_hotkey_panel(
    bindings: &[Binding],
    edit: &EditState,
    theme: &Theme,
    cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    // 说明文案：讲清楚「怎么改、改完什么时候生效」。
    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(PANEL_HINT),
    );

    // 状态区：**只在有话可说时渲染**（`Idle` ⇒ 什么都不渲染，不留空占位）。
    if let Some(status) = &edit.status {
        column = column.child(
            div()
                .text_xs()
                .text_color(if edit.target.is_some() {
                    theme.accent
                } else {
                    theme.danger
                })
                .child(status.clone()),
        );
    }

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
                column = column.child(render_row(row, edit, theme, cx));
            }
        }
    }

    column.into_any_element()
}

/// 单行：左为命令说明，右为按键组合（**可点** ⇒ 点它开始改键）。
fn render_row(
    row: &HotkeyRow,
    edit: &EditState,
    theme: &Theme,
    cx: &mut Context<SettingsGpui>,
) -> Div {
    let command_id = row.command_id;
    // 正在改这一行 ⇒ 按键组合描一圈边（accent），让用户知道在改谁。
    let editing = edit.target == Some(command_id);

    let keys = div()
        // id 必须**同屏唯一** ⇒ 带上限键文本：
        // 同一个命令可能有**两条**绑定（`palette.open` 的 `Ctrl+K` 与
        // `Ctrl+Shift+P`）⇒ 只用 command_id 会撞 id。
        .id(format!("hotkey-row-{}-{}", row.command_id, row.keys))
        .text_xs()
        .text_color(theme.accent)
        .whitespace_nowrap()
        .cursor_pointer()
        .child(row.keys.clone());
    let keys = if editing {
        keys.border_1().border_color(theme.accent).rounded_sm()
    } else {
        keys
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
                        .child(row.command_id),
                ),
        )
        .child(keys.on_click(cx.listener(
            move |this: &mut SettingsGpui,
                  _event: &ClickEvent,
                  _window: &mut Window,
                  cx: &mut Context<SettingsGpui>| {
                this.begin_edit(command_id, cx);
            },
        )))
}
