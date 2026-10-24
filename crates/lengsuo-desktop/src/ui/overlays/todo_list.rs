//! 任务清单浮层（波次 6，卡 `ui-overlay-todo-list`，对齐 ui-parity-spec §8 A35）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `todoList`。
//! 数据源在运行时侧：[`TodoOverlayState::initial`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "任务清单";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 新任务输入行标签。
pub const NEW_TASK: &str = "新任务";
/// 任务标题输入占位。
pub const TASK_TITLE: &str = "任务标题";
/// 任务描述输入占位（可选字段）。
pub const TASK_DESCRIPTION: &str = "任务描述（可选）";
/// 添加任务按钮文案。
pub const ADD_TASK: &str = "添加任务";
/// 编辑按钮文案。
pub const EDIT: &str = "编辑";
/// 保存编辑按钮文案。
pub const SAVE: &str = "保存";
/// 取消编辑按钮文案。
pub const CANCEL: &str = "取消";
/// 删除按钮文案。
pub const DELETE: &str = "删除";
/// 空态提示（已下发但零条目）。
pub const NO_TASKS: &str = "暂无任务";
/// 任务计数模板（占位符 `{count}`）。
pub const TASK_COUNT: &str = "{count} 个任务";

/// 任务状态徽标（`todoList.status.*`，文案见 [`TodoStatus::label`]）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TodoStatus {
    Waiting,
    InProgress,
    Completed,
    Error,
}

impl TodoStatus {
    /// 徽标文案（逐字取自 `todoList.status.*`）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Waiting => "等待",
            Self::InProgress => "进行中",
            Self::Completed => "已完成",
            Self::Error => "错误",
        }
    }
}

/// 单条任务（纯数据；Ⓐ 不持凭据、不判权限）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TodoItem {
    pub title: String,
    pub description: String,
    pub status: TodoStatus,
}

/// 编辑中的草稿（保存前不落回条目）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Editing {
    pub index: usize,
    pub title: String,
    pub description: String,
}

/// 已下发的清单快照 + 输入行草稿。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
    pub draft_title: String,
    pub draft_description: String,
    pub editing: Option<Editing>,
}

/// 浮层可枚举状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TodoOverlayState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空集）。
    NotDelivered,
    /// 数据已下发：清单 + 输入行草稿的正常态。
    Ready(TodoList),
}

/// 把模板里的 `{count}` 占位符替换成数值。
fn fill_count(tmpl: &str, count: usize) -> String {
    tmpl.replace("{count}", &count.to_string())
}

impl TodoOverlayState {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self::NotDelivered
    }

    /// 是否处于未下发态。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self, Self::NotDelivered)
    }

    /// 清单区居中文本：未下发 / 空态；有条目时返回 `None`（改渲染清单行）。
    pub fn body_text(&self) -> Option<String> {
        match self {
            Self::NotDelivered => Some(NOT_DELIVERED.to_string()),
            Self::Ready(list) if list.is_empty() => Some(NO_TASKS.to_string()),
            Self::Ready(_) => None,
        }
    }

    /// 底栏计数：仅有条目时出现（⛔ 未下发/空态不伪造数值）。
    pub fn footer_count(&self) -> Option<String> {
        match self {
            Self::Ready(list) if !list.is_empty() => Some(list.count_text()),
            Self::Ready(_) | Self::NotDelivered => None,
        }
    }
}

impl TodoList {
    /// 是否零条目（空输入 ⇒ 空态，不是空列表）。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 「{count} 个任务」。
    pub fn count_text(&self) -> String {
        fill_count(TASK_COUNT, self.items.len())
    }

    /// 已完成条数。
    pub fn completed_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.status == TodoStatus::Completed)
            .count()
    }

    /// 新任务可添加：标题草稿非空白（⛔ 空输入不成条）。
    pub fn can_add(&self) -> bool {
        !self.draft_title.trim().is_empty()
    }

    /// 添加草稿为新任务（默认「等待」），成功后清空草稿。
    pub fn add_task(&mut self) -> bool {
        if !self.can_add() {
            return false;
        }
        self.items.push(TodoItem {
            title: self.draft_title.trim().to_string(),
            description: self.draft_description.trim().to_string(),
            status: TodoStatus::Waiting,
        });
        self.draft_title.clear();
        self.draft_description.clear();
        true
    }

    /// 删除条目；同步取消或前移编辑态下标。
    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }
        self.items.remove(index);
        match self.editing.take() {
            Some(edit) if edit.index == index => {}
            Some(mut edit) if edit.index > index => {
                edit.index -= 1;
                self.editing = Some(edit);
            }
            edit => self.editing = edit,
        }
        true
    }

    /// 进入编辑（拷贝条目为草稿，保存前不落回）。
    pub fn begin_edit(&mut self, index: usize) -> bool {
        let Some(item) = self.items.get(index) else {
            return false;
        };
        self.editing = Some(Editing {
            index,
            title: item.title.clone(),
            description: item.description.clone(),
        });
        true
    }

    /// 保存编辑（标题非空白才落回条目）。
    pub fn save_edit(&mut self) -> bool {
        match self.editing.take() {
            None => false,
            Some(edit) if edit.title.trim().is_empty() => {
                self.editing = Some(edit);
                false
            }
            Some(edit) => {
                if let Some(item) = self.items.get_mut(edit.index) {
                    item.title = edit.title.trim().to_string();
                    item.description = edit.description.trim().to_string();
                }
                true
            }
        }
    }

    /// 取消编辑（草稿丢弃）。
    pub fn cancel_edit(&mut self) {
        self.editing = None;
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
/// 恒以 [`TodoOverlayState::initial`]（未下发）渲染 —— fail-closed。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &TodoOverlayState::initial())
}

/// 按状态渲染整层：标题行 + 新任务输入行 + 清单区 + 底栏计数行。
fn render(theme: &Theme, state: &TodoOverlayState) -> Div {
    let list = match state {
        TodoOverlayState::Ready(list) => Some(list),
        TodoOverlayState::NotDelivered => None,
    };
    let mut col = div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title_row(theme))
        .child(input_row(theme, list))
        .child(list_area(theme, state));
    if let Some(bar) = footer_row(theme, state) {
        col = col.child(bar);
    }
    col
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

/// 新任务输入行：标签 + 标题/描述输入位 + 添加任务（未下发时草稿恒空 ⇒ 只显示占位）。
fn input_row(theme: &Theme, list: Option<&TodoList>) -> Div {
    let (draft_title, draft_description) = match list {
        Some(list) => (list.draft_title.as_str(), list.draft_description.as_str()),
        None => ("", ""),
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NEW_TASK),
        )
        .child(input_box(theme, draft_title, TASK_TITLE))
        .child(input_box(theme, draft_description, TASK_DESCRIPTION))
        .child(text_button(theme, ADD_TASK, true))
}

/// 输入位样式：有值显示值，无值显示占位（占位用弱色）。
fn input_box(theme: &Theme, value: &str, placeholder: &'static str) -> Div {
    let (text, color) = if value.is_empty() {
        (placeholder.to_string(), theme.muted_foreground)
    } else {
        (value.to_string(), theme.foreground)
    };
    div()
        .flex_1()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(color)
        .child(text)
}

/// 清单区：未下发 / 空态居中提示，有条目时渲染清单行。
fn list_area(theme: &Theme, state: &TodoOverlayState) -> Div {
    if let Some(text) = state.body_text() {
        return div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(text);
    }
    let TodoOverlayState::Ready(list) = state else {
        unreachable!("body_text 为 None 时必然是 Ready");
    };
    let mut col = div().flex_1().flex().flex_col();
    for (idx, item) in list.items.iter().enumerate() {
        let editing = list.editing.as_ref().filter(|edit| edit.index == idx);
        col = col.child(task_row(theme, item, editing));
    }
    col
}

/// 清单行：状态徽标 + 标题/描述 + 行内操作；编辑中的行换成草稿位 + 保存/取消。
fn task_row(theme: &Theme, item: &TodoItem, editing: Option<&Editing>) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(badge(theme, item.status));
    if let Some(edit) = editing {
        return row
            .child(input_box(theme, &edit.title, TASK_TITLE))
            .child(text_button(theme, SAVE, true))
            .child(text_button(theme, CANCEL, false));
    }
    let mut text_col = div().flex_1().flex().flex_col().child(
        div()
            .text_sm()
            .text_color(theme.foreground)
            .child(item.title.clone()),
    );
    if !item.description.is_empty() {
        text_col = text_col.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(item.description.clone()),
        );
    }
    row.child(text_col)
        .child(text_button(theme, EDIT, false))
        .child(text_button(theme, DELETE, false))
}

/// 状态徽标（进行中/错误用强调描边，等待/已完成用弱样式）。
fn badge(theme: &Theme, status: TodoStatus) -> Div {
    let (border, text) = match status {
        TodoStatus::InProgress | TodoStatus::Error => (theme.accent, theme.foreground),
        TodoStatus::Waiting | TodoStatus::Completed => (theme.border, theme.muted_foreground),
    };
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(border)
        .text_xs()
        .text_color(text)
        .child(status.label())
}

/// 文本按钮（文案即无障碍标签）；primary 用强调描边。
fn text_button(theme: &Theme, label: &'static str, primary: bool) -> Div {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if primary { theme.accent } else { theme.border })
        .text_sm()
        .text_color(if primary {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .child(label)
}

/// 底栏计数行：仅有条目时出现（⛔ 未下发/空态不伪造数值）。
fn footer_row(theme: &Theme, state: &TodoOverlayState) -> Option<Div> {
    state.footer_count().map(|count| {
        div()
            .flex()
            .items_center()
            .px_4()
            .py_2()
            .border_t_1()
            .border_color(theme.border)
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(div().flex_1())
            .child(count)
    })
}
#[cfg(test)]
mod tests {
    use super::{
        fill_count, main_area, TodoItem, TodoList, TodoOverlayState, TodoStatus, ADD_TASK, CANCEL,
        DELETE, EDIT, NEW_TASK, NO_TASKS, NOT_DELIVERED, SAVE, TASK_COUNT, TASK_DESCRIPTION,
        TASK_TITLE, TITLE,
    };
    use gpui_kit::component::Theme;

    fn item(title: &str, status: TodoStatus) -> TodoItem {
        TodoItem {
            title: title.to_string(),
            description: String::new(),
            status,
        }
    }

    fn list_with(items: Vec<TodoItem>) -> TodoList {
        TodoList {
            items,
            ..TodoList::default()
        }
    }

    #[test]
    fn count_template_fills_placeholder() {
        assert_eq!(fill_count(TASK_COUNT, 3), "3 个任务");
        let list = list_with(vec![
            item("a", TodoStatus::Waiting),
            item("b", TodoStatus::Completed),
        ]);
        assert_eq!(list.count_text(), "2 个任务");
        assert_eq!(list.completed_count(), 1);
    }

    #[test]
    fn initial_state_is_fail_closed_not_delivered() {
        assert!(TodoOverlayState::initial().is_not_delivered());
        assert_eq!(
            TodoOverlayState::initial().body_text().as_deref(),
            Some(NOT_DELIVERED)
        );
        assert_eq!(TodoOverlayState::initial().footer_count(), None);
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn empty_items_is_empty_state_not_blank() {
        let state = TodoOverlayState::Ready(TodoList::default());
        assert_eq!(state.body_text().as_deref(), Some(NO_TASKS));
        assert!(!state.body_text().expect("上方已断言 Some").is_empty());
        assert_eq!(state.footer_count(), None);
    }

    #[test]
    fn footer_count_only_when_items_exist() {
        let state = TodoOverlayState::Ready(list_with(vec![item("a", TodoStatus::Waiting)]));
        assert_eq!(state.footer_count().as_deref(), Some("1 个任务"));
    }

    #[test]
    fn empty_draft_cannot_add_and_add_clears_draft() {
        let mut list = TodoList::default();
        assert!(!list.can_add());
        assert!(!list.add_task());
        list.draft_title = "   ".to_string();
        assert!(!list.can_add());
        list.draft_title = " 对接数据源 ".to_string();
        list.draft_description = " 备注去空白 ".to_string();
        assert!(list.add_task());
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].title, "对接数据源");
        assert_eq!(list.items[0].description, "备注去空白");
        assert_eq!(list.items[0].status, TodoStatus::Waiting);
        assert!(list.draft_title.is_empty() && list.draft_description.is_empty());
    }

    #[test]
    fn edit_flow_save_and_cancel() {
        let mut list = list_with(vec![
            item("标题一", TodoStatus::InProgress),
            item("标题二", TodoStatus::Waiting),
        ]);
        assert!(!list.begin_edit(2));
        assert!(list.begin_edit(0));
        list.editing.as_mut().expect("已进入编辑").title = "  ".to_string();
        assert!(!list.save_edit());
        assert!(list.editing.is_some());
        list.editing.as_mut().expect("仍处于编辑").title = "新标题".to_string();
        assert!(list.save_edit());
        assert_eq!(list.items[0].title, "新标题");
        assert!(list.editing.is_none());
        assert!(list.begin_edit(1));
        list.cancel_edit();
        assert_eq!(list.items[1].title, "标题二");
        assert!(list.editing.is_none());
    }

    #[test]
    fn remove_adjusts_or_cancels_editing() {
        let mut list = list_with(vec![
            item("一", TodoStatus::Waiting),
            item("二", TodoStatus::Waiting),
            item("三", TodoStatus::Error),
        ]);
        assert!(!list.remove(3));
        assert!(list.begin_edit(2));
        assert!(list.remove(0));
        assert_eq!(list.editing.as_ref().expect("编辑态保留").index, 1);
        assert!(list.remove(1));
        assert!(list.editing.is_none());
        assert_eq!(list.items.len(), 1);
    }

    #[test]
    fn copy_is_verbatim() {
        assert_eq!(TITLE, "任务清单");
        assert_eq!(NEW_TASK, "新任务");
        assert_eq!(TASK_TITLE, "任务标题");
        assert_eq!(TASK_DESCRIPTION, "任务描述（可选）");
        assert_eq!(ADD_TASK, "添加任务");
        assert_eq!(EDIT, "编辑");
        assert_eq!(SAVE, "保存");
        assert_eq!(CANCEL, "取消");
        assert_eq!(DELETE, "删除");
        assert_eq!(NO_TASKS, "暂无任务");
        assert_eq!(TASK_COUNT, "{count} 个任务");
        assert_eq!(TodoStatus::Waiting.label(), "等待");
        assert_eq!(TodoStatus::InProgress.label(), "进行中");
        assert_eq!(TodoStatus::Completed.label(), "已完成");
        assert_eq!(TodoStatus::Error.label(), "错误");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
