//! 批量操作条浮层（波次 6，卡 `ui-overlay-batch-bar`，对齐 ui-parity-spec §8 A40）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `batch`。
//! 数据源在运行时侧：[`BatchBarState::initial`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 层标题 / 无障碍标签（逐字取自界面文案全表）。
pub const TITLE: &str = "批量操作";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空选态提示：没有可执行的任务，需重新选择。
pub const EMPTY_HINT: &str = "没有可执行的任务，请重新选择";
/// 退出按钮的无障碍文案。
pub const EXIT: &str = "退出批量操作";
/// 退出按钮的悬浮提示文案。
pub const EXIT_TOOLTIP: &str = "取消";
/// 已选计数模板（占位符 `{count}`）。
pub const SELECTED_COUNT: &str = "已选 {count} 项";
/// 全选标签。
pub const SELECT_ALL: &str = "全选";
/// 全选计数后缀模板（占位符 `{count}`）。
pub const SELECT_ALL_COUNT: &str = "({count})";
/// 归档动作按钮文案。
pub const ACTION_ARCHIVE: &str = "归档";
/// 删除动作按钮文案。
pub const ACTION_DELETE: &str = "删除";
/// 含不支持归档条目（助理/项目任务）时的提示。
pub const ARCHIVE_UNSUPPORTED: &str = "暂不支持助理和项目任务归档，请重新选择";

/// 浮层可枚举状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchBarState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空集）。
    NotDelivered,
    /// 数据已下发：带选中快照的正常态。
    Ready(Selection),
}

/// 选中快照（纯数据，Ⓐ 不持凭据、不判权限）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    /// 已选条数。
    pub selected: usize,
    /// 可选总数（驱动「全选({count})」后缀）。
    pub total: usize,
    /// 是否含不支持归档的条目（助理/项目任务）。
    pub archive_unsupported: bool,
}

/// 把模板里的 `{count}` 占位符替换成数值。
fn fill_count(tmpl: &str, count: usize) -> String {
    tmpl.replace("{count}", &count.to_string())
}

impl BatchBarState {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self::NotDelivered
    }

    /// 是否处于未下发态。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self, Self::NotDelivered)
    }

    /// 中央状态行：未下发 / 空选提示 / 已选计数。
    pub fn status_line(&self) -> String {
        match self {
            Self::NotDelivered => NOT_DELIVERED.to_string(),
            Self::Ready(sel) if sel.is_empty() => EMPTY_HINT.to_string(),
            Self::Ready(sel) => sel.selected_count_text(),
        }
    }
}

impl Selection {
    /// 是否一项都没选（空输入 ⇒ 空态，不是空列表）。
    pub fn is_empty(&self) -> bool {
        self.selected == 0
    }

    /// 「已选 {count} 项」。
    pub fn selected_count_text(&self) -> String {
        fill_count(SELECTED_COUNT, self.selected)
    }

    /// 「全选({count})」。
    pub fn select_all_text(&self) -> String {
        format!("{SELECT_ALL}{}", fill_count(SELECT_ALL_COUNT, self.total))
    }

    /// 归档是否可执行：有选中且不含不支持归档的条目。
    pub fn can_archive(&self) -> bool {
        !self.is_empty() && !self.archive_unsupported
    }

    /// 删除是否可执行：有选中即可。
    pub fn can_delete(&self) -> bool {
        !self.is_empty()
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
/// 恒以 [`BatchBarState::initial`]（未下发）渲染 —— fail-closed。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &BatchBarState::initial())
}

/// 按状态渲染整层：标题行 + 中央状态行 + 底部操作条。
fn render(theme: &Theme, state: &BatchBarState) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title_row(theme))
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(state.status_line()),
        )
        .child(bar(theme, state))
}

/// 标题行（文案即无障碍标签）。
fn title_row(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(TITLE)
}

/// 底部操作条：有选中时才出现计数/全选/归档/删除（⛔ 未下发不伪造数值）。
fn bar(theme: &Theme, state: &BatchBarState) -> Div {
    let selection = match state {
        BatchBarState::Ready(sel) if !sel.is_empty() => Some(sel),
        _ => None,
    };
    let mut col = div()
        .flex()
        .flex_col()
        .border_t_1()
        .border_color(theme.border)
        .child(bar_row(theme, selection));
    if selection.is_some_and(|sel| sel.archive_unsupported) {
        col = col.child(
            div()
                .px_4()
                .pb_2()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(ARCHIVE_UNSUPPORTED),
        );
    }
    col
}

/// 操作条主行。
fn bar_row(theme: &Theme, selection: Option<&Selection>) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(exit_button(theme));
    if let Some(sel) = selection {
        row = row
            .child(div().flex_1())
            .child(
                div()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(sel.selected_count_text()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(sel.select_all_text()),
            )
            .child(archive_button(theme, sel))
            .child(delete_button(theme));
    }
    row
}

/// 退出按钮（悬浮提示文案见 [`EXIT_TOOLTIP`]）。
fn exit_button(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(EXIT)
}

/// 归档按钮：不可归档时降为弱样式（提示行由 [`bar`] 补充）。
fn archive_button(theme: &Theme, sel: &Selection) -> Div {
    let can = sel.can_archive();
    div()
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if can { theme.accent } else { theme.border })
        .text_sm()
        .text_color(if can {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .child(ACTION_ARCHIVE)
}

/// 删除按钮。
fn delete_button(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.foreground)
        .child(ACTION_DELETE)
}
#[cfg(test)]
mod tests {
    use super::{
        fill_count, main_area, BatchBarState, Selection, ACTION_ARCHIVE, ACTION_DELETE,
        ARCHIVE_UNSUPPORTED, EMPTY_HINT, EXIT, EXIT_TOOLTIP, NOT_DELIVERED, SELECTED_COUNT,
        SELECT_ALL, SELECT_ALL_COUNT, TITLE,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn count_templates_fill_placeholder() {
        assert_eq!(fill_count(SELECTED_COUNT, 3), "已选 3 项");
        assert_eq!(fill_count(SELECT_ALL_COUNT, 5), "(5)");
        let sel = Selection {
            selected: 3,
            total: 12,
            archive_unsupported: false,
        };
        assert_eq!(sel.selected_count_text(), "已选 3 项");
        assert_eq!(sel.select_all_text(), "全选(12)");
    }

    #[test]
    fn initial_state_is_fail_closed_not_delivered() {
        assert!(BatchBarState::initial().is_not_delivered());
        assert_eq!(BatchBarState::initial().status_line(), NOT_DELIVERED);
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn empty_selection_is_empty_state_not_blank() {
        let sel = Selection {
            selected: 0,
            total: 5,
            archive_unsupported: false,
        };
        let state = BatchBarState::Ready(sel.clone());
        assert!(sel.is_empty());
        assert_eq!(state.status_line(), EMPTY_HINT);
        // 空态是提示文案，不是空白或空列表。
        assert!(!state.status_line().is_empty());
    }

    #[test]
    fn action_copy_is_verbatim() {
        assert_eq!(ACTION_ARCHIVE, "归档");
        assert_eq!(ACTION_DELETE, "删除");
    }

    #[test]
    fn bar_copy_is_verbatim() {
        assert_eq!(TITLE, "批量操作");
        assert_eq!(EXIT, "退出批量操作");
        assert_eq!(EXIT_TOOLTIP, "取消");
        assert_eq!(SELECT_ALL, "全选");
        assert_eq!(SELECTED_COUNT, "已选 {count} 项");
        assert_eq!(SELECT_ALL_COUNT, "({count})");
        assert_eq!(ARCHIVE_UNSUPPORTED, "暂不支持助理和项目任务归档，请重新选择");
        assert_eq!(EMPTY_HINT, "没有可执行的任务，请重新选择");
    }

    #[test]
    fn archive_guard_blocks_unsupported_and_empty() {
        let ok = Selection {
            selected: 2,
            total: 4,
            archive_unsupported: false,
        };
        let blocked = Selection {
            archive_unsupported: true,
            ..ok.clone()
        };
        let empty = Selection {
            selected: 0,
            total: 4,
            archive_unsupported: false,
        };
        assert!(ok.can_archive());
        assert!(ok.can_delete());
        assert!(!blocked.can_archive());
        assert!(blocked.can_delete());
        assert!(!empty.can_archive());
        assert!(!empty.can_delete());
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    #[test]
    fn render_ready_branches_build_without_panic() {
        let theme = Theme::default();
        let ok = Selection {
            selected: 2,
            total: 4,
            archive_unsupported: false,
        };
        let unsupported = Selection {
            archive_unsupported: true,
            ..ok.clone()
        };
        let empty = Selection {
            selected: 0,
            total: 4,
            archive_unsupported: false,
        };
        let _ = super::render(&theme, &BatchBarState::Ready(ok));
        let _ = super::render(&theme, &BatchBarState::Ready(unsupported));
        let _ = super::render(&theme, &BatchBarState::Ready(empty));
        let _ = super::render(&theme, &BatchBarState::NotDelivered);
    }
}
