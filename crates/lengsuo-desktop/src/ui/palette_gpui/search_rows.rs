//! 命令面板的**多来源搜索结果**渲染：按来源分组，每组一个标题 + 若干条目。
//!
//! 分组顺序 = [`crate::ui::command_palette::SearchSource::ALL`] 的顺序
//! （命令 / 技能 / MCP / 文件 / 会话）；空分组不出现。
//!
//! **不引入任何组件**：全部用 `div()` 自搭 ⇒ 零新依赖、零 API 不确定性。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
// `overflow_y_scrollbar` 来自 gpui-component 的 `ScrollableElement` trait
// （不在 gpui-pre 里）⇒ 必须显式导入，否则 E0599。
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::*;

use crate::ui::command_palette::{CommandPaletteView, SearchItem};

use super::layout::LIST_MAX_HEIGHT;

/// 渲染整个结果列表：分组标题 + 各组条目；无结果时给出显式提示。
pub(super) fn render_results(view: &CommandPaletteView, theme: &Theme) -> impl IntoElement {
    let mut list = div()
        .flex()
        .flex_col()
        .max_h(px(LIST_MAX_HEIGHT))
        .overflow_y_scrollbar();
    if view.groups.is_empty() {
        // 空白结果给出显式提示，不静默显示空列表（文案取自描述层）。
        let empty = view
            .empty_text
            .unwrap_or(crate::ui::command_palette::EMPTY_TEXT);
        list = list.child(
            div()
                .px_3()
                .py_4()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(empty),
        );
    } else {
        for group in &view.groups {
            list = list.child(render_group_title(group.title, theme));
            for item in &group.items {
                let selected = view.selected_id.as_deref() == Some(item.id.as_str());
                list = list.child(render_item(item, selected, theme));
            }
        }
    }
    list
}

/// 渲染一个来源分组的标题。
pub(super) fn render_group_title(title: &str, theme: &Theme) -> impl IntoElement {
    div()
        .px_3()
        .pt_2()
        .pb_1()
        .text_xs()
        .font_bold()
        .text_color(theme.muted_foreground)
        .child(title.to_owned())
}

/// 渲染一个分组下的条目：标题 + 右侧提示。
///
/// ⚠️ `on_click` 只在 `Stateful<E>` 上存在 ⇒ 必须先 `.id("唯一id")`；
/// id 由**业务 id** 派生 ⇒ 同屏唯一。
pub(super) fn render_item(item: &SearchItem, selected: bool, theme: &Theme) -> impl IntoElement {
    let mut title = div().text_sm().child(item.title.clone());
    let mut line = div()
        .id(format!("palette-search-item-{}", item.id))
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .py_2()
        .rounded_sm()
        .cursor_pointer();
    let hint_color = if selected {
        theme.accent_foreground
    } else {
        theme.muted_foreground
    };
    if selected {
        line = line.bg(theme.accent);
        title = title.font_bold().text_color(theme.accent_foreground);
    } else {
        title = title.text_color(theme.foreground);
    }
    line = line.child(title);
    if let Some(hint) = &item.hint {
        line = line.child(div().text_xs().text_color(hint_color).child(hint.clone()));
    }
    line
}
