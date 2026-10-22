//! 底部终端抽屉的容器：收起 ⇄ 展开。
//!
//! 形态：
//!
//! ```text
//! 收起：  ┌────────────────────────────────┐
//!         │ 终端                        展开 │  ← 标题条（可点 ⇒ 展开）
//!         └────────────────────────────────┘
//! 展开：  ┌────────────────────────────────┐
//!         │ 终端                        收起 │  ← 标题条（可点 ⇒ 收起）
//!         │ [输出] [历史]                   │  ← tab 条
//!         │ …等宽输出…                      │  ← 内容区（固定高度）
//!         │ > ____________      [运行][中断] │  ← 输入行
//!         └────────────────────────────────┘
//! ```
//!
//! 硬约束：
//! - **默认收起**（对齐 13 册「底部抽屉式默认收起」）⇒ 收起态只占一条标题条。
//! - 点击只改**本视图持有的副本**（`TerminalState`），⛔ 不直接 spawn 进程。
//! - 颜色一律取自主题，⛔ 不硬编码。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::terminal::TerminalTab;
use crate::ui::terminal::TerminalView;

use super::{input_row, output_view, tabs, TerminalDrawerGpui};

/// 展开态内容区的固定高度（px）。
///
/// ⛔ 本卡**不做滚动条**（`overflow_y_scrollbar` 需要额外导入 trait，
/// `-D warnings` 下 unused import 就整卡失败）⇒ 溢出由内容区自己截断
/// （只显示最近若干行，见 [`output_view::MAX_VISIBLE_LINES`]）。
pub(super) const CONTENT_HEIGHT: f32 = 220.;

/// 收起态标题条上的展开提示。
pub(super) const EXPAND_LABEL: &str = "展开";
/// 展开态标题条上的收起提示。
pub(super) const COLLAPSE_LABEL: &str = "收起";

/// 渲染抽屉。
pub(super) fn render_drawer(
    view: &TerminalView,
    textarea: Option<&Entity<TextareaState>>,
    theme: &Theme,
    cx: &mut Context<TerminalDrawerGpui>,
) -> impl IntoElement {
    let hint = if view.expanded {
        COLLAPSE_LABEL
    } else {
        EXPAND_LABEL
    };
    // 标题条：点一下展开 / 收起（⛔ 只改副本，随后 `cx.notify()` 让本视图重绘）。
    let header = {
        let toggle = cx.listener(
            |this: &mut TerminalDrawerGpui, _: &ClickEvent, _window: &mut Window, cx| {
                this.state_mut().toggle_expanded();
                cx.notify();
            },
        );
        div()
            .id(format!("terminal-toggle-{}", TerminalTab::Output.id()))
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .px_3()
            .py_2()
            .cursor_pointer()
            .on_click(toggle)
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(view.title.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(hint),
            )
    };

    let shell = div()
        .w_full()
        .flex_none()
        .flex_col()
        .bg(theme.background)
        .border_t_1()
        .border_color(theme.border)
        .child(header);

    // 收起态：只一条标题条（⛔ 不渲染内容区，避免占位）。
    if !view.expanded {
        return shell.into_any_element();
    }

    // 展开态：标题条 + tab 条 + 内容区 + 输入行。
    let content: AnyElement = match view.tab {
        TerminalTab::Output => output_view::render_output(&view.output, theme).into_any_element(),
        TerminalTab::History => {
            output_view::render_history(&view.history, theme).into_any_element()
        }
    };

    let body = div()
        .h(px(CONTENT_HEIGHT))
        .flex_none()
        .overflow_hidden()
        .px_3()
        .py_2()
        .child(content);

    shell
        .child(tabs::render_tabs(view, theme, cx))
        .child(body)
        .child(input_row::render_input_row(view, textarea, theme, cx))
        .into_any_element()
}
