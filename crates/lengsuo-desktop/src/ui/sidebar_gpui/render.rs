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

/// 测试侧宿主实体：把手工拼的元素树包进一个 `Render` 里画。
///
/// 带 `on_click` 的交互元素在绘制期要取「当前视图」（gpui 的
/// `rendered_entity_stack` 不能为空），裸画会 panic；包一层实体即可。
#[cfg(test)]
pub(in crate::ui::sidebar_gpui) struct HostedElement(pub AnyElement);

#[cfg(test)]
impl Render for HostedElement {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // 这个 gpui 版本的 AnyElement 不可 Clone ⇒ 取走元素，后续帧给空占位
        // （无窗口断言只画一帧，够用）。
        std::mem::replace(&mut self.0, div().into_any_element())
    }
}

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

/// 演示来源横幅：频道 / 智能体视图体顶部的一行弱化标注。
///
/// C9 运行时契约未就绪 ⇒ 视图体只有演示数据，来源必须如实标注在界面上。
pub(super) fn render_demo_banner(theme: &Theme) -> AnyElement {
    div()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .border_b_1()
        .border_color(theme.sidebar_border)
        .child(super::DEMO_SOURCE_BANNER)
        .into_any_element()
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

#[cfg(test)]
mod tests {
    //! A15 会话列表保真断言：① 置顶排序 ② 未读可见标记 ③ 分组整帧可渲染。
    //!
    //! 显式导入、不写 `use super::*`：避免 glob 带入的 GPUI `test` 属性宏
    //! 遮蔽标准库 `#[test]`（同 render_assert_tests 的约定）。
    //! 元素级断言复用 render_assert_tests 的无窗口绘制骨架。

    use gpui_kit::component::Theme;
    use gpui_kit::{div, AppContext as _, IntoElement as _, ParentElement as _, Styled as _};

    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use crate::ui::sidebar::{self, SessionEntry, SessionRow, SidebarState, SidebarView};
    use crate::ui::sidebar_gpui::{demo_state, row_marks, MarkKind, SidebarViewGpui};

    use super::{render_mark, render_section, HostedElement};

    /// 取一条会话在渲染描述里的行（找不到即测试失败）。
    fn row_of<'a>(view: &'a SidebarView, id: &str) -> &'a SessionRow {
        view.sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .find(|row| row.id == id)
            .unwrap_or_else(|| panic!("渲染描述里找不到会话行 {id}"))
    }

    /// ① 置顶排序：注入时置顶放在最后，摊平后仍必须排在所有非置顶之前。
    #[test]
    fn pinned_rows_sort_before_regular() {
        let mut state = SidebarState::new();
        state.set_entries(vec![
            SessionEntry::new("regular-1", "普通会话一"),
            SessionEntry::new("pinned", "置顶会话").with_pinned(true),
            SessionEntry::new("regular-2", "普通会话二"),
        ]);
        let view = sidebar::render(&state, 260.0);
        let order: Vec<&str> = view
            .sections
            .iter()
            .flat_map(|section| section.rows.iter().map(|row| row.id.as_str()))
            .collect();
        assert_eq!(
            order,
            vec!["pinned", "regular-1", "regular-2"],
            "置顶项必须排在非置顶之前"
        );
    }

    /// ①（demo 数据源同样成立）：第一条可见行就是置顶会话。
    #[test]
    fn demo_state_puts_pinned_row_first() {
        let view = sidebar::render(&demo_state(), 260.0);
        let first = &view.sections[0].rows[0];
        assert_eq!(first.id, "demo-pinned", "demo 的置顶会话必须排第一");
        assert!(first.pinned);
    }

    /// ② 未读可见标记：未读行恰好一个「未读」标记，非未读行不得带。
    #[test]
    fn unread_rows_carry_visible_mark() {
        let mut state = SidebarState::new();
        state.set_entries(vec![
            SessionEntry::new("unread", "有未读").with_unread(true),
            SessionEntry::new("plain", "无未读"),
        ]);
        let view = sidebar::render(&state, 260.0);
        let unread_row_marks = row_marks(row_of(&view, "unread"));
        let unread_marks: Vec<_> = unread_row_marks
            .iter()
            .filter(|mark| mark.kind == MarkKind::Unread)
            .collect();
        assert_eq!(unread_marks.len(), 1, "未读行必须恰好一个未读标记");
        assert_eq!(unread_marks[0].label, "未读", "未读标记文案必须逐字");
        assert!(
            row_marks(row_of(&view, "plain"))
                .iter()
                .all(|mark| mark.kind != MarkKind::Unread),
            "非未读行不得出现未读标记"
        );
    }

    /// ②（元素级）：未读标记走实心角标分支，真的画得出来且布局非零。
    #[test]
    fn unread_mark_element_renders_nonzero() {
        let view = sidebar::render(&demo_state(), 260.0);
        let mark = row_marks(row_of(&view, "demo-pinned"))
            .into_iter()
            .find(|mark| mark.kind == MarkKind::Unread)
            .expect("demo 置顶会话应带未读标记");
        let bounds = draw_probe(|_| render_mark(&mark, &Theme::default()));
        assert_rendered(bounds, "未读标记（实心角标）");
    }

    /// ③ 会话分组整帧不崩 + 布局非零（置顶 / 全部会话 / 归档按钮同屏）。
    #[test]
    fn demo_sections_render_nonzero() {
        let view = sidebar::render(&demo_state(), 260.0);
        let bounds = draw_probe(|ctx| {
            let entity = ctx.new(|_| SidebarViewGpui::demo());
            let theme = Theme::default();
            let mut list = div().flex().flex_col();
            for section in &view.sections {
                list = list.child(render_section(section, &entity, &theme));
            }
            ctx.new(|_| HostedElement(list.into_any_element()))
                .into_any_element()
        });
        assert_rendered(bounds, "会话分组");
    }
}
