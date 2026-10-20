//! 边栏的 GPUI 接线层：把 [`crate::ui::sidebar`] 的渲染描述接到元素树。
//!
//! 分层约定（同 [`crate::ui::env_gpui`]）：
//! - **描述层**（`sidebar`）：纯数据、无 GPUI 依赖，可在无窗口环境测。
//! - **接线层**（本文件）：把描述变成 GPUI 元素，**不重新实现**任何业务判断。
//!
//! 本文件内部再分两半：
//! - **映射层**（无 `feature = "ui"` 门控）：描述字段 → 界面标记 / 交互契约，
//!   纯数据，随 `cargo test` 一起跑。
//! - **渲染层**（`feature = "ui"` 门控）：GPUI 元素树，只能编译验证。
//!
//! 硬约束：
//! - **文案只有一个来源**：分组名、标题、空态、归档按钮文案全部取自描述层
//!   （[`crate::ui::sidebar`]），本文件不另造一套。
//! - **归档默认隐藏**：哪些分组可见由描述层决定，本文件不自行过滤条目。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
//!
//! **接线层不实现业务语义**：置顶 / 归档 / 未读 / 选中 / 空态的判定都在
//! 描述层，本文件只做「描述字段 → 样式与标记」的映射。

use crate::ui::sidebar::{SessionRow, SidebarState};

// GPUI 相关导入：只有渲染层需要。
// 三个 import 缺一不可：`StyledExt`（`font_bold` / `truncate` 等扩展）、
// `ActiveTheme`（`cx.theme()`）、`gpui_kit::*`（`div` / `px` / `Entity` / `Render`）。
#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::ActiveTheme as _;
#[cfg(feature = "ui")]
use gpui_kit::*;

/// 置顶标记文案。
pub const PINNED_MARK: &str = "置顶";
/// 归档标记文案。
pub const ARCHIVED_MARK: &str = "归档";
/// 未读标记文案。
pub const UNREAD_MARK: &str = "未读";

/// 行标记的种类（描述层布尔字段 → 界面标记）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    /// 置顶会话。
    Pinned,
    /// 已归档会话。
    Archived,
    /// 有未读消息。
    Unread,
}

/// 行标记的展示信息（纯数据，可在无窗口环境测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowMark {
    /// 标记种类（决定颜色）。
    pub kind: MarkKind,
    /// 标记文案（中文）。
    pub label: &'static str,
}

/// 描述层的一行 → 界面上的标记集合。
///
/// 这是「描述 → 元素」映射里唯一带判断的一步：描述层只给布尔字段，
/// 标记的**种类与文案**在这里定型，方便无窗口环境测。
#[must_use]
pub fn row_marks(row: &SessionRow) -> Vec<RowMark> {
    let mut marks = Vec::new();
    if row.pinned {
        marks.push(RowMark {
            kind: MarkKind::Pinned,
            label: PINNED_MARK,
        });
    }
    if row.archived {
        marks.push(RowMark {
            kind: MarkKind::Archived,
            label: ARCHIVED_MARK,
        });
    }
    if row.unread {
        marks.push(RowMark {
            kind: MarkKind::Unread,
            label: UNREAD_MARK,
        });
    }
    marks
}

/// 点击一行：写入选中态。
///
/// 接线层的点击处理只调这个入口，不自己判断「这条能不能选」——
/// 能不能选由描述层的 [`SidebarState::select`] 说了算（归档隐藏时不可选）。
pub fn select_row(state: &mut SidebarState, id: &str) -> bool {
    state.select(id)
}

/// 点击「显示 / 隐藏归档」：翻转归档可见性。
pub fn toggle_archived(state: &mut SidebarState) {
    state.toggle_archived();
}

/// 演示用状态：手工构造若干会话条目，**不接真实数据源**。
#[must_use]
pub fn demo_state() -> SidebarState {
    let mut state = SidebarState::new();
    state.set_entries(vec![
        crate::ui::sidebar::SessionEntry::new("demo-pinned", "接口联调")
            .with_pinned(true)
            .with_unread(true),
        crate::ui::sidebar::SessionEntry::new("demo-plain", "打包脚本排查"),
        crate::ui::sidebar::SessionEntry::new("demo-archived", "旧版设计稿").with_archived(true),
    ]);
    state
}

// === 渲染层：GPUI 元素树（feature = "ui"） ===

/// 边栏的 GPUI 视图。
///
/// 持有**会话列表状态**（[`SidebarState`]）的副本：点击行 / 归档切换按钮
/// 只改这份副本，宿主可用 [`Self::state_mut`] 回写共享状态。
#[cfg(feature = "ui")]
pub struct SidebarViewGpui {
    state: SidebarState,
}

#[cfg(feature = "ui")]
impl SidebarViewGpui {
    /// 用一份会话列表状态构造视图。
    #[must_use]
    pub fn new(state: SidebarState) -> Self {
        Self { state }
    }

    /// 从只读引用构造视图（内部克隆一份状态）。
    #[must_use]
    pub fn from_state(state: &SidebarState) -> Self {
        Self::new(state.clone())
    }

    /// 演示用视图：手工构造的会话条目，**不接真实数据源**。
    #[must_use]
    pub fn demo() -> Self {
        Self::new(demo_state())
    }

    /// 当前会话列表状态（只读）。
    #[must_use]
    pub fn state(&self) -> &SidebarState {
        &self.state
    }

    /// 当前会话列表状态（可写）：宿主据此回写共享状态。
    pub fn state_mut(&mut self) -> &mut SidebarState {
        &mut self.state
    }
}

/// 计数徽标（未读数、隐藏的归档数）。
#[cfg(feature = "ui")]
fn render_count_badge(count: usize, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .rounded_full()
        .px_1()
        .text_xs()
        .text_color(theme.button_warning_foreground)
        .bg(theme.button_warning)
        .child(count.to_string())
}

/// 行标记（置顶 / 归档 / 未读）。
#[cfg(feature = "ui")]
fn render_mark(mark: &RowMark, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let color = match mark.kind {
        MarkKind::Pinned => theme.accent,
        MarkKind::Archived => theme.muted_foreground,
        MarkKind::Unread => theme.button_warning,
    };
    div()
        .rounded_full()
        .px_1()
        .text_xs()
        .text_color(color)
        .border_1()
        .border_color(color)
        .child(mark.label)
}

/// 一行会话：标题 + 标记；选中项高亮；点击回写选中态。
#[cfg(feature = "ui")]
fn render_row(
    row: &SessionRow,
    entity: &Entity<SidebarViewGpui>,
    theme: &gpui_kit::component::Theme,
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
            select_row(&mut view.state, &id);
            cx.notify();
        });
    })
}

/// 一个分组：分组标题 + 组内会话行。
#[cfg(feature = "ui")]
fn render_section(
    section: &crate::ui::sidebar::SidebarSection,
    entity: &Entity<SidebarViewGpui>,
    theme: &gpui_kit::component::Theme,
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
#[cfg(feature = "ui")]
fn render_archived_toggle(
    view: &crate::ui::sidebar::SidebarView,
    entity: &Entity<SidebarViewGpui>,
    theme: &gpui_kit::component::Theme,
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
            toggle_archived(&mut view.state);
            cx.notify();
        });
    })
}

/// 空态占位文案（描述层已区分「真没有」与「只有归档」）。
#[cfg(feature = "ui")]
fn render_empty(text: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
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

#[cfg(feature = "ui")]
impl Render for SidebarViewGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = cx.entity();
        // 描述层产出渲染描述；本文件只做「描述 → 元素」的映射。
        let view =
            crate::ui::sidebar::render(&self.state, crate::ui::layout::SIDEBAR_DEFAULT_WIDTH);

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_2()
            .border_b_1()
            .border_color(theme.sidebar_border)
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.sidebar_foreground)
                    .child(view.heading),
            );
        let header = if view.unread_count > 0 {
            header.child(render_count_badge(view.unread_count, theme))
        } else {
            header
        };

        let body = if let Some(text) = view.empty_text {
            render_empty(text, theme).into_any_element()
        } else {
            div()
                .id("sidebar-body")
                .flex()
                .flex_col()
                .flex_1()
                .overflow_y_scroll()
                .children(
                    view.sections
                        .iter()
                        .map(|section| render_section(section, &entity, theme)),
                )
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .h_full()
            .w(px(view.width))
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(header)
            .child(body)
            .child(render_archived_toggle(&view, &entity, theme))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        demo_state, row_marks, select_row, toggle_archived, MarkKind, RowMark, ARCHIVED_MARK,
        PINNED_MARK, UNREAD_MARK,
    };
    use crate::ui::sidebar::{self, SessionEntry, SessionRow, SidebarState};

    fn row(pinned: bool, archived: bool, unread: bool) -> SessionRow {
        let mut state = SidebarState::new();
        state.set_entries(vec![SessionEntry::new("x", "标题")
            .with_pinned(pinned)
            .with_archived(archived)
            .with_unread(unread)]);
        // 归档默认隐藏，展开后才有一行可断言。
        if archived {
            toggle_archived(&mut state);
        }
        let view = sidebar::render(&state, 260.0);
        view.sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .next()
            .expect("应当有一行")
            .clone()
    }

    #[test]
    fn marks_map_description_flags_to_chinese_labels() {
        assert_eq!(
            row_marks(&row(true, false, false)),
            vec![RowMark {
                kind: MarkKind::Pinned,
                label: PINNED_MARK
            }]
        );
        assert_eq!(
            row_marks(&row(false, false, true)),
            vec![RowMark {
                kind: MarkKind::Unread,
                label: UNREAD_MARK
            }]
        );
        assert_eq!(
            row_marks(&row(true, true, true)),
            vec![
                RowMark {
                    kind: MarkKind::Pinned,
                    label: PINNED_MARK
                },
                RowMark {
                    kind: MarkKind::Archived,
                    label: ARCHIVED_MARK
                },
                RowMark {
                    kind: MarkKind::Unread,
                    label: UNREAD_MARK
                },
            ]
        );
        assert!(row_marks(&row(false, false, false)).is_empty());
    }

    #[test]
    fn archived_mark_only_appears_when_archived_is_visible() {
        let mut state = SidebarState::new();
        state.set_entries(vec![SessionEntry::new("a", "旧会话").with_archived(true)]);
        // 归档默认隐藏：描述层不给出这一行，界面也就无从标注。
        let view = sidebar::render(&state, 260.0);
        assert!(view.sections.is_empty());
        assert_eq!(view.hidden_archived, 1);

        toggle_archived(&mut state);
        let view = sidebar::render(&state, 260.0);
        let archived_row = view.sections[0].rows[0].clone();
        assert_eq!(
            row_marks(&archived_row),
            vec![RowMark {
                kind: MarkKind::Archived,
                label: ARCHIVED_MARK
            }]
        );
    }

    #[test]
    fn demo_state_covers_pinned_archived_and_unread() {
        let state = demo_state();
        assert_eq!(state.entries().len(), 3);
        assert_eq!(state.unread_count(), 1);
        assert_eq!(state.archived_count(), 1);

        let description = sidebar::render(&state, 260.0);
        let labels: Vec<&str> = description
            .sections
            .iter()
            .map(|section| section.label)
            .collect();
        assert_eq!(labels, ["置顶", "全部会话"]);
    }

    #[test]
    fn clicking_a_row_writes_the_selection_back() {
        let mut state = demo_state();
        assert!(select_row(&mut state, "demo-plain"));
        assert_eq!(state.selected(), Some("demo-plain"));

        let description = sidebar::render(&state, 260.0);
        let selected: Vec<&str> = description
            .sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .filter(|row| row.selected)
            .map(|row| row.id.as_str())
            .collect();
        assert_eq!(selected, ["demo-plain"]);
    }

    #[test]
    fn clicking_an_archived_row_while_hidden_changes_nothing() {
        let mut state = demo_state();
        assert!(!select_row(&mut state, "demo-archived"), "归档隐藏时不可选");
        assert_eq!(state.selected(), None);

        toggle_archived(&mut state);
        assert!(select_row(&mut state, "demo-archived"), "展开后可选");
        assert_eq!(state.selected(), Some("demo-archived"));
    }

    #[test]
    fn toggling_archived_reveals_the_archived_section() {
        let mut state = demo_state();
        assert!(!state.show_archived());
        toggle_archived(&mut state);
        assert!(state.show_archived());
        let description = sidebar::render(&state, 260.0);
        assert_eq!(description.hidden_archived, 0);
        assert!(description
            .sections
            .iter()
            .any(|section| section.label == sidebar::ARCHIVED_SECTION));
    }
}
