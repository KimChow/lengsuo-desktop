//! 终端输入行：提示符 + 单行输入 + 运行 / 中断按钮。
//!
//! 形态：
//!
//! ```text
//! > node -v                          [运行] [中断]
//! ```
//!
//! 输入行是**单行语义**：`Enter` = **提交命令**（不是换行）——
//! 照抄本仓 `ui/palette_gpui` 的搜索框先例（`Textarea` + `InputEvent::PressEnter`）。
//!
//! 硬约束：
//! - ⛔ **不 spawn 进程**：点「运行」只登记 `TerminalIntent::Run`。
//! - ⛔ **「中断」只在有命令在跑时可用**（由描述层的 `running` 决定）。
//! - 文本字符与退格**由编辑器消费**（编辑器自己发 `Change`）⇒ 本层不重复写入。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::terminal::{TerminalView, INPUT_PROMPT, INTERRUPT_LABEL, RUN_LABEL};

use super::TerminalDrawerGpui;

/// 「运行」按钮的元素 id（**同屏唯一**：全仓只有这一个终端抽屉）。
const RUN_BUTTON_ID: &str = "terminal-run-button";
/// 「中断」按钮的元素 id（**同屏唯一**）。
const INTERRUPT_BUTTON_ID: &str = "terminal-interrupt-button";

/// 输入框的高度（px）：单行。
const INPUT_HEIGHT: f32 = 28.;

/// 渲染输入行。
pub(super) fn render_input_row(
    view: &TerminalView,
    textarea: Option<&Entity<TextareaState>>,
    theme: &Theme,
    cx: &mut Context<TerminalDrawerGpui>,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(INPUT_PROMPT),
        )
        .child(div().flex_1().child(render_editor(textarea)))
        .child(render_run_button(theme, cx))
        .child(render_interrupt_button(view.running, theme, cx))
}

/// 输入框本体（`Textarea`，单行语义：Enter = 提交）。
fn render_editor(textarea: Option<&Entity<TextareaState>>) -> impl IntoElement {
    match textarea {
        Some(textarea) => Textarea::new(textarea)
            .appearance(false)
            .h(px(INPUT_HEIGHT))
            .into_any_element(),
        // 还没渲染过（理论不会发生）：留空位，不报错。
        None => div().h(px(INPUT_HEIGHT)).into_any_element(),
    }
}

/// 「运行」按钮：登记执行意图（⛔ 不执行）。
fn render_run_button(theme: &Theme, cx: &mut Context<TerminalDrawerGpui>) -> impl IntoElement {
    let listener = cx.listener(
        |this: &mut TerminalDrawerGpui, _: &ClickEvent, window: &mut Window, cx| {
            this.submit(window, cx);
        },
    );
    div()
        .id(RUN_BUTTON_ID)
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm()
        .font_bold()
        .bg(theme.button_primary)
        .text_color(theme.button_primary_foreground)
        .cursor_pointer()
        .on_click(listener)
        .child(RUN_LABEL)
}

/// 「中断」按钮（Ctrl+C）：登记中断意图。
///
/// 没有命令在跑时**置灰且不注册点击**（由描述层的 `running` 决定）。
fn render_interrupt_button(
    running: bool,
    theme: &Theme,
    cx: &mut Context<TerminalDrawerGpui>,
) -> impl IntoElement {
    if !running {
        return div()
            .id(INTERRUPT_BUTTON_ID)
            .px_3()
            .py_1()
            .rounded_sm()
            .text_sm()
            .bg(theme.button_secondary)
            .text_color(theme.muted_foreground)
            .child(INTERRUPT_LABEL)
            .into_any_element();
    }
    let listener = cx.listener(
        |this: &mut TerminalDrawerGpui, _: &ClickEvent, _window: &mut Window, cx| {
            this.state_mut().interrupt();
            cx.notify();
        },
    );
    div()
        .id(INTERRUPT_BUTTON_ID)
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm()
        .font_bold()
        .bg(theme.button_danger)
        .text_color(theme.button_danger_foreground)
        .cursor_pointer()
        .on_click(listener)
        .child(INTERRUPT_LABEL)
        .into_any_element()
}
