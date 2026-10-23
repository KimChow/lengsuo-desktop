//! 边栏会话部分的 GPUI 渲染分支。
//!
//! 本文件是 [`super`]（`sidebar_gpui`）的子模块，只放**渲染分支**：
//! 计数徽标 / 行标记 / 会话行 / 分组 / 归档按钮 / 空态。
//! 类型定义、视图主体与映射层在 `mod.rs`。
//!
//! 硬约束：颜色一律取自 `cx.theme()`，不硬编码；文案一律取自描述层。

#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::sidebar::{SessionRow, SidebarSection, SidebarView};

use super::{row_marks, select_row, toggle_archived, MarkKind, SidebarViewGpui};

/// 实心小角标：警示底色 + 高对比前景（计数角标与未读标记共用同一档样式）。
fn solid_badge(text: &str, theme: &Theme) -> AnyElement {
    div()
        .rounded_full()
        .px_1()
        .text_xs()
        .text_color(theme.button_warning_foreground)
        .bg(theme.button_warning)
        .child(text.to_owned())
        .into_any_element()
}

/// 计数徽标（未读数、隐藏的归档数）。
pub(super) fn render_count_badge(count: usize, theme: &Theme) -> AnyElement {
    solid_badge(&count.to_string(), theme)
}

/// 行标记（置顶 / 归档 / 未读）。
///
/// 未读是行动信号 ⇒ 实心角标（最强一档）；置顶 / 归档只是属性 ⇒ 描边胶囊弱一档。
/// 三种状态在视觉上必须一眼可分，不能只存在枚举里。
pub(super) fn render_mark(mark: &super::RowMark, theme: &Theme) -> AnyElement {
    match mark.kind {
        MarkKind::Unread => solid_badge(mark.label, theme),
        _ => outlined_mark(mark, theme),
    }
}

/// 描边胶囊（置顶 / 归档）：弱一档的属性标记。
fn outlined_mark(mark: &super::RowMark, theme: &Theme) -> AnyElement {
    let color = match mark.kind {
        MarkKind::Pinned => theme.accent,
        _ => theme.muted_foreground,
    };
    div()
        .rounded_full()
        .px_1()
        .text_xs()
        .text_color(color)
        .border_1()
        .border_color(color)
        .child(mark.label)
        .into_any_element()
}

/// 一行会话：标题 + 标记；选中项高亮；点击回写选中态。
pub(super) fn render_row(
    row: &SessionRow,
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    // 标题颜色：占位标题弱化、选中项用强调色、其余用边栏前景色。
    let title_color = if row.title_is_placeholder {
        theme.muted_foreground
    } else if row.selected {
        theme.sidebar_accent_foreground
    } else {
        theme.sidebar_foreground
    };

    let mut marks = div().flex().items_center().gap_1();
    for mark in row_marks(row) {
        marks = marks.child(render_mark(&mark, theme));
    }

    let mut element = div()
        .id(format!("sidebar-row-{}", row.id))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_sm()
        .cursor_pointer();
    if row.selected {
        element = element.bg(theme.sidebar_accent);
    }
    let element = element
        .child(
            div()
                .text_sm()
                .text_color(title_color)
                .truncate()
                .child(row.title.clone()),
        )
        .child(marks);

    let entity = entity.clone();
    let id = row.id.clone();
    element.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            select_row(view.state_mut(), &id);
            cx.notify();
        });
    })
}

/// 一个分组：分组标题 + 组内会话行。
pub(super) fn render_section(
    section: &SidebarSection,
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .px_2()
                .pt_2()
                .pb_1()
                .text_xs()
                .font_bold()
                .text_color(theme.muted_foreground)
                .child(section.label),
        )
        .children(
            section
                .rows
                .iter()
                .map(|row| render_row(row, entity, theme)),
        )
}

/// 归档展开 / 收起按钮：文案与隐藏数量都取自描述层。
pub(super) fn render_archived_toggle(
    view: &SidebarView,
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    let button = div()
        .id("sidebar-archived-toggle")
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_sm()
        .cursor_pointer()
        .child(
            div()
                .text_xs()
                .text_color(theme.sidebar_foreground)
                .child(view.archived_toggle_label),
        );

    let button = if view.hidden_archived > 0 {
        button.child(render_count_badge(view.hidden_archived, theme))
    } else {
        button
    };

    let entity = entity.clone();
    button.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            toggle_archived(view.state_mut());
            cx.notify();
        });
    })
}

/// 空态占位文案（描述层已区分「真没有」与「只有归档」）。
pub(super) fn render_empty(text: &str, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .px_2()
        .py_4()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}
