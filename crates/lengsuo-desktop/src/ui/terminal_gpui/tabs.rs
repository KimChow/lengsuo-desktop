//! 终端抽屉的 tab 条：输出 / 历史。
//!
//! 两个 tab **始终可见**（描述层的硬约束）⇒ 收起态也能看到它们。
//! 点击只改**本视图持有的副本**（`TerminalState::select_tab`）。

use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::terminal::{TerminalTabButton, TerminalView};

use super::TerminalDrawerGpui;

/// 渲染 tab 条。
pub(super) fn render_tabs(
    view: &TerminalView,
    theme: &Theme,
    cx: &mut Context<TerminalDrawerGpui>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_1()
        .border_b_1()
        .border_color(theme.border)
        .children(
            view.tabs
                .iter()
                .map(|tab| render_tab_button(tab, theme, cx)),
        )
}

/// 一个 tab 按钮。
fn render_tab_button(
    tab: &TerminalTabButton,
    theme: &Theme,
    cx: &mut Context<TerminalDrawerGpui>,
) -> impl IntoElement {
    let label_color = if tab.selected {
        theme.accent_foreground
    } else {
        theme.muted_foreground
    };
    let mut button = div()
        .id(format!("terminal-tab-{}", tab.tab.id()))
        .px_2()
        .py_1()
        .rounded_sm()
        .text_sm()
        .text_color(label_color)
        .cursor_pointer();
    if tab.selected {
        button = button.bg(theme.accent);
    }
    let button = button.child(tab.label);

    let selected_tab = tab.tab;
    let listener = cx.listener(
        move |this: &mut TerminalDrawerGpui, _: &ClickEvent, _window: &mut Window, cx| {
            this.state_mut().select_tab(selected_tab);
            cx.notify();
        },
    );
    button.on_click(listener)
}
