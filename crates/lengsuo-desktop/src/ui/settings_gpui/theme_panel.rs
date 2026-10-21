//! 「外观」面板：主题三档（跟随系统 / 浅色 / 深色）。
//!
//! 三档用 `div()` 自搭成单选行（**不引入 `Select` / `Radio` 组件**）。
//! 点中即调 [`SettingsGpui::set_theme`]：**改即生效，无保存按钮**。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::settings_gpui::view::{theme_label, SettingsView, THEME_CHOICES};

use super::window::SettingsGpui;

/// 渲染主题三档选择器。
pub(super) fn render_theme_picker(
    view: &SettingsView,
    theme: &Theme,
    cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    for choice in THEME_CHOICES {
        let selected = *choice == view.theme;
        let label = theme_label(*choice);
        let choice = *choice;

        let row = div()
            .id(format!("settings-theme-{}", choice.slug()))
            .px_3()
            .py_2()
            .rounded_sm()
            .text_sm()
            .cursor_pointer()
            .border_1()
            .border_color(if selected { theme.accent } else { theme.border });
        let row = if selected {
            row.bg(theme.accent)
                .text_color(theme.accent_foreground)
                .font_bold()
        } else {
            row.text_color(theme.foreground)
        };

        column = column.child(row.child(label).on_click(cx.listener(
            move |this: &mut SettingsGpui,
                  _event: &ClickEvent,
                  window: &mut Window,
                  cx: &mut Context<SettingsGpui>| {
                this.set_theme(choice, window, cx);
            },
        )));
    }

    // 提示文案：说明「改即生效」与「跟随系统」的当前含义。
    let hint = div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child("改主题即时生效，无需保存。「跟随系统」取当前系统外观。");

    column.child(hint).into_any_element()
}
