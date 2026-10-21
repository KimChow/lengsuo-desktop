//! 发送 / 停止按钮（A5）。
//!
//! 语义由描述层的 [`InputAreaState::send_button`] 决定：等待确认期间按钮形态是
//! 「停止」，点击只**产出停止意图**（[`StopIntent`]），**不执行** `agent:stop`
//! —— 执行停止属运行时职责，界面层不得越界。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::input_area::SendButtonKind;

use super::InputAreaGpui;

/// 渲染发送 / 停止按钮。
pub(super) fn render(
    view: &InputAreaGpui,
    theme: &Theme,
    cx: &mut Context<InputAreaGpui>,
) -> AnyElement {
    let send = view.state.send_button();
    match send.kind {
        SendButtonKind::Stop => render_stop(theme, cx),
        SendButtonKind::Send => render_send(send.label, send.enabled, theme, cx),
    }
}

/// 停止形态：danger 底色，点击产出停止意图。
fn render_stop(theme: &Theme, cx: &mut Context<InputAreaGpui>) -> AnyElement {
    let listener = cx.listener(
        |this: &mut InputAreaGpui, _: &ClickEvent, _window: &mut Window, cx| {
            if let Some(intent) = this.state.take_stop() {
                this.stop_outbox.push(intent);
            }
            cx.notify();
        },
    );
    div()
        .id("input-stop-button")
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm()
        .font_bold()
        .bg(theme.button_danger)
        .text_color(theme.button_danger_foreground)
        .cursor_pointer()
        .on_click(listener)
        .child("停止")
        .into_any_element()
}

/// 发送形态：可用时是主色按钮，不可用时置灰且不注册点击。
fn render_send(
    label: &'static str,
    enabled: bool,
    theme: &Theme,
    cx: &mut Context<InputAreaGpui>,
) -> AnyElement {
    if !enabled {
        return div()
            .px_3()
            .py_1()
            .rounded_sm()
            .text_sm()
            .bg(theme.button_secondary)
            .text_color(theme.muted_foreground)
            .child(label)
            .into_any_element();
    }
    let listener = cx.listener(
        |this: &mut InputAreaGpui, _: &ClickEvent, window: &mut Window, cx| {
            this.submit_and_queue(window, cx);
        },
    );
    div()
        .id("input-send-button")
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm()
        .font_bold()
        .bg(theme.button_primary)
        .text_color(theme.button_primary_foreground)
        .cursor_pointer()
        .on_click(listener)
        .child(label)
        .into_any_element()
}
