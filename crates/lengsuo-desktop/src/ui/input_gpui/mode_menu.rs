//! 模式下拉（A11）：一个触发器 + 展开的 5 个选项。
//!
//! 弹层用 `div().absolute()` 自搭，**不引入任何组件库**（零新依赖）。
//! 模式项文案取自描述层的 [`InputMode::label`]，接线层不另造一套。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::input_area::InputMode;

use super::InputAreaGpui;

/// 下拉面板宽度（像素）。
const PANEL_WIDTH: f32 = 96.;

/// 渲染模式条：触发器（显示当前模式）+ 展开时的 5 项下拉列表。
pub(super) fn render(
    view: &InputAreaGpui,
    theme: &Theme,
    cx: &mut Context<InputAreaGpui>,
) -> AnyElement {
    let current = view.state.mode();
    let trigger_text = format!("{} ▾", current.label());

    let trigger = div()
        .id("input-mode-trigger")
        .px_2()
        .py_1()
        .rounded_sm()
        .text_sm()
        .bg(theme.button)
        .text_color(theme.button_foreground)
        .cursor_pointer()
        .on_click(cx.listener(
            |this: &mut InputAreaGpui, _: &ClickEvent, _window: &mut Window, cx| {
                this.mode_menu_open = !this.mode_menu_open;
                // 两个弹层互斥：打开模式下拉时收起 @ 快选。
                this.mention_open = false;
                cx.notify();
            },
        ))
        .child(trigger_text);

    // 锚点：紧跟触发器之后的零尺寸 relative 容器；绝对定位的面板以它为坐标系。
    //
    // 面板用 `bottom_0` ⇒ 贴着触发器下沿**向上**展开（触发器在底部行，
    // 面板就该往上弹）。gpui 没有 z-index，绘制顺序 = 树序，
    // 所以这一行必须排在多行输入框**之后**，面板才不会被输入框盖住。
    //
    // 外层用 `items_start`：锚点不拉伸成整行宽 ⇒ 面板的 `left_0`
    // 正好落在触发器的左沿（否则会跑到输入区最左边）。
    let anchor = div().relative();

    let mut elements: Vec<AnyElement> = Vec::with_capacity(2);
    elements.push(trigger.into_any_element());
    if view.mode_menu_open {
        elements.push(
            anchor
                .child(render_options(current, theme, cx))
                .into_any_element(),
        );
    }

    div()
        .flex()
        .flex_col()
        .items_start()
        .children(elements)
        .into_any_element()
}

/// 展开态的面板：5 个模式选项，当前模式高亮。
fn render_options(
    current: InputMode,
    theme: &Theme,
    cx: &mut Context<InputAreaGpui>,
) -> AnyElement {
    let mut options: Vec<AnyElement> = Vec::with_capacity(InputMode::ALL.len());
    for option in super::logic::mode_options(current) {
        let mode = option.mode;
        let listener = cx.listener(
            move |this: &mut InputAreaGpui, _: &ClickEvent, _window: &mut Window, cx| {
                this.mode_menu_open = false;
                // `select_mode` 内部会 notify；此处再补一次，覆盖「模式没变」的情形。
                this.select_mode(mode, &mut *cx);
                cx.notify();
            },
        );
        let element = if option.selected {
            div()
                .id(option.id)
                .px_2()
                .py_1()
                .rounded_sm()
                .text_sm()
                .font_bold()
                .bg(theme.accent)
                .text_color(theme.accent_foreground)
                .cursor_pointer()
                .on_click(listener)
                .child(option.label)
                .into_any_element()
        } else {
            div()
                .id(option.id)
                .px_2()
                .py_1()
                .rounded_sm()
                .text_sm()
                .bg(theme.button)
                .text_color(theme.button_foreground)
                .cursor_pointer()
                .on_click(listener)
                .child(option.label)
                .into_any_element()
        };
        options.push(element);
    }

    div()
        .absolute()
        .bottom_0()
        .left_0()
        .w(px(PANEL_WIDTH))
        .flex()
        .flex_col()
        .p_1()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg(theme.popover)
        .shadow_lg()
        .children(options)
        .into_any_element()
}
