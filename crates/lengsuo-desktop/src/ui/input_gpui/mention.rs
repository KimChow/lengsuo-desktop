//! `@` 文件引用快选（A12）。
//!
//! 参照平台的行为是「弹 Glob 快选插入路径」；本仓**不读盘**、不扫描文件系统：
//! 候选路径**由调用方注入**（[`InputAreaGpui::set_candidates`]），
//! 界面层只做「检测 `@` 查询 → 弹层快选 → 把路径插入输入框」。
//!
//! 弹层用 `div().absolute()` 自搭，**不引入任何组件库**（零新依赖）。
#![cfg(feature = "ui")]

use gpui_kit::component::Theme;
use gpui_kit::*;

use super::InputAreaGpui;

/// 快选面板宽度（像素）。
const PANEL_WIDTH: f32 = 320.;

/// 渲染 `@` 快选弹层。
///
/// 只有草稿里存在 `@查询词`（[`InputAreaState::mention_query`]）且弹层已展开
/// 时才出层；没有 `@`、或没有命中候选时返回空元素，不占位。
pub(super) fn render(
    view: &InputAreaGpui,
    theme: &Theme,
    cx: &mut Context<InputAreaGpui>,
) -> AnyElement {
    if !view.mention_open {
        return div().into_any_element();
    }
    let hits = super::logic::mention_hits(view.state());
    if hits.is_empty() {
        return div().into_any_element();
    }

    let mut items: Vec<AnyElement> = Vec::with_capacity(hits.len());
    for hit in hits {
        let target = hit.path.clone();
        let listener = cx.listener(
            move |this: &mut InputAreaGpui, _: &ClickEvent, window: &mut Window, cx| {
                this.state.insert_mention(&target);
                this.mention_open = false;
                // 把新草稿写回编辑器，否则旧 `@词` 文本会被 `sync_draft` 写回描述层。
                this.write_back_draft(window, cx);
                cx.notify();
            },
        );
        let item = div()
            .id(hit.id)
            .px_2()
            .py_1()
            .rounded_sm()
            .text_sm()
            .bg(theme.button)
            .text_color(theme.button_foreground)
            .cursor_pointer()
            .on_click(listener)
            .child(hit.path)
            .into_any_element();
        items.push(item);
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
        .children(items)
        .into_any_element()
}
