//! 后台任务胶囊与面板（波次 8，卡 `ui-overlay-background-task`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（无 GPUI 依赖）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `backgroundTask`（9 键）。
//! 数据源在运行时侧：本模块当前以 [`Panel::not_delivered`] 诚实占位，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 界面文案（键与中文均逐字取自文案全表，命名空间 `backgroundTask`，共 9 键）。
pub mod copy {
    pub const CANCELLED: (&str, &str) = ("backgroundTask.cancelled", "已取消");
    pub const CAPSULE_LABEL: (&str, &str) =
        ("backgroundTask.capsuleLabel", "{count} 个后台任务运行中");
    pub const CAPSULE_LABEL_COMPLETED: (&str, &str) = (
        "backgroundTask.capsuleLabelCompleted",
        "{count} 个后台任务已完成",
    );
    pub const COMPLETED: (&str, &str) = ("backgroundTask.completed", "已完成");
    pub const FAILED: (&str, &str) = ("backgroundTask.failed", "任务失败");
    pub const PANEL_TITLE: (&str, &str) = ("backgroundTask.panelTitle", "{count} 个后台任务运行中");
    pub const PANEL_TITLE_WITH_COMPLETED: (&str, &str) = (
        "backgroundTask.panelTitleWithCompleted",
        "{count} 个后台任务",
    );
    pub const PENDING: (&str, &str) = ("backgroundTask.pending", "等待中");
    pub const RUNNING: (&str, &str) = ("backgroundTask.running", "已运行");
}
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "后台任务胶囊与面板";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    Loading,
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}
/// 单条后台任务的执行状态（对应命名空间下 5 个状态标签键）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Pending,
    Failed,
    Cancelled,
    Completed,
}
impl TaskStatus {
    /// 面板状态行的既定顺序（运行中 → 等待中 → 失败 → 已取消 → 已完成）。
    pub const ORDER: [TaskStatus; 5] = [
        TaskStatus::Running,
        TaskStatus::Pending,
        TaskStatus::Failed,
        TaskStatus::Cancelled,
        TaskStatus::Completed,
    ];
    /// 状态标签（逐字取自文案全表）。
    pub fn label(self) -> &'static str {
        match self {
            TaskStatus::Running => copy::RUNNING.1,
            TaskStatus::Pending => copy::PENDING.1,
            TaskStatus::Failed => copy::FAILED.1,
            TaskStatus::Cancelled => copy::CANCELLED.1,
            TaskStatus::Completed => copy::COMPLETED.1,
        }
    }
}
/// 把模板里的 `{count}` 占位替换为计数（模板逐字来自文案全表，其余字符不动）。
fn with_count(template: &str, count: usize) -> String {
    template.replace("{count}", &count.to_string())
}
/// 后台任务面板的纯数据快照（运行中/已完成计数 + 各状态标签的唯一来源）。
#[derive(Debug, Default)]
pub struct Panel {
    /// 数据源状态；仅 `Normal` 视为已下发。
    pub state: State,
    /// 已下发的任务状态列表（空 ⇒ 空态，≠ 未下发）。
    pub tasks: Vec<TaskStatus>,
}
impl Panel {
    /// fail-closed 默认：无数据源 ⇒ 未下发（⛔ 不是空列表）。
    pub fn not_delivered() -> Self {
        Self::default()
    }
    /// 已下发的任务集（空集 ⇒ 空态「0 个后台任务」）。
    pub fn delivered(tasks: Vec<TaskStatus>) -> Self {
        Self {
            state: State::Normal,
            tasks,
        }
    }
    /// 指定状态的任务数。
    pub fn count(&self, status: TaskStatus) -> usize {
        self.tasks.iter().filter(|t| **t == status).count()
    }
    /// 运行中计数（胶囊/标题首选）。
    pub fn running_count(&self) -> usize {
        self.count(TaskStatus::Running)
    }
    /// 已完成计数（胶囊次选）。
    pub fn completed_count(&self) -> usize {
        self.count(TaskStatus::Completed)
    }
    /// 已下发但一个任务都没有（空态）。
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
    /// 出现过的状态（按既定顺序去重）⇒ 面板状态行。
    pub fn statuses_present(&self) -> Vec<TaskStatus> {
        TaskStatus::ORDER
            .iter()
            .copied()
            .filter(|s| self.count(*s) > 0)
            .collect()
    }
    /// 胶囊文案：优先「N 个运行中」，否则「N 个已完成」；皆无 ⇒ 无胶囊（不伪造）。
    pub fn capsule_text(&self) -> Option<String> {
        if self.running_count() > 0 {
            Some(with_count(copy::CAPSULE_LABEL.1, self.running_count()))
        } else if self.completed_count() > 0 {
            Some(with_count(
                copy::CAPSULE_LABEL_COMPLETED.1,
                self.completed_count(),
            ))
        } else {
            None
        }
    }
    /// 面板标题：有运行中 ⇒ 运行数；否则 ⇒ 任务总数（0 ⇒ 空态标题）。
    pub fn panel_title(&self) -> String {
        if self.running_count() > 0 {
            with_count(copy::PANEL_TITLE.1, self.running_count())
        } else {
            with_count(copy::PANEL_TITLE_WITH_COMPLETED.1, self.tasks.len())
        }
    }
}
/// 主区渲染（浮层不占主区，签名不变；无数据源 ⇒ fail-closed「未下发」）。
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &Panel::not_delivered())
}
/// 面板渲染：非 `Normal`（未拉取/加载中/失败）一律 fail-closed「未下发」；
/// `Normal` 按快照渲染标题行 + 胶囊 + 各状态标签行（空集 ⇒ 仅空态标题，不渲染空列表）。
fn render(theme: &Theme, panel: &Panel) -> Div {
    if panel.state != State::Normal {
        return div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_color(theme.foreground).child(TITLE))
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child(NOT_DELIVERED),
            );
    }
    let mut view = div().flex().flex_col().gap_2().child(
        div()
            .text_color(theme.foreground)
            .child(SharedString::from(panel.panel_title())),
    );
    if let Some(capsule) = panel.capsule_text() {
        view = view.child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(capsule)),
        );
    }
    for status in panel.statuses_present() {
        view = view.child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(div().text_color(theme.foreground).child(status.label()))
                .child(
                    div()
                        .text_color(theme.muted_foreground)
                        .child(SharedString::from(panel.count(status).to_string())),
                ),
        );
    }
    view
}
#[cfg(test)]
mod tests {
    use super::copy;
    use super::{main_area, Panel, State, TaskStatus, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(Panel::not_delivered().state, State::NotDelivered);
    }
    #[test]
    fn title_not_empty() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
    #[test]
    fn counts_by_status() {
        let panel = Panel::delivered(vec![
            TaskStatus::Running,
            TaskStatus::Running,
            TaskStatus::Completed,
            TaskStatus::Pending,
        ]);
        assert_eq!(panel.running_count(), 2);
        assert_eq!(panel.completed_count(), 1);
        assert_eq!(panel.count(TaskStatus::Pending), 1);
        assert_eq!(panel.count(TaskStatus::Cancelled), 0);
        assert_eq!(
            panel.statuses_present(),
            vec![
                TaskStatus::Running,
                TaskStatus::Pending,
                TaskStatus::Completed
            ]
        );
    }
    #[test]
    fn empty_input_is_empty_state_not_undelivered() {
        let empty = Panel::delivered(Vec::new());
        assert!(empty.is_empty());
        assert_eq!(empty.state, State::Normal);
        assert_eq!(empty.capsule_text(), None);
        assert_eq!(empty.panel_title(), "0 个后台任务");
        assert!(empty.statuses_present().is_empty());
    }
    #[test]
    fn copy_keys_verbatim() {
        assert_eq!(
            copy::CAPSULE_LABEL,
            ("backgroundTask.capsuleLabel", "{count} 个后台任务运行中")
        );
        assert_eq!(
            copy::CAPSULE_LABEL_COMPLETED,
            (
                "backgroundTask.capsuleLabelCompleted",
                "{count} 个后台任务已完成"
            )
        );
        assert_eq!(
            copy::PANEL_TITLE_WITH_COMPLETED,
            (
                "backgroundTask.panelTitleWithCompleted",
                "{count} 个后台任务"
            )
        );
        assert_eq!(copy::PENDING, ("backgroundTask.pending", "等待中"));
    }
    #[test]
    fn status_labels_verbatim() {
        assert_eq!(TaskStatus::Running.label(), copy::RUNNING.1);
        assert_eq!(TaskStatus::Running.label(), "已运行");
        assert_eq!(TaskStatus::Pending.label(), copy::PENDING.1);
        assert_eq!(TaskStatus::Pending.label(), "等待中");
        assert_eq!(TaskStatus::Failed.label(), copy::FAILED.1);
        assert_eq!(TaskStatus::Failed.label(), "任务失败");
        assert_eq!(TaskStatus::Cancelled.label(), copy::CANCELLED.1);
        assert_eq!(TaskStatus::Cancelled.label(), "已取消");
        assert_eq!(TaskStatus::Completed.label(), copy::COMPLETED.1);
        assert_eq!(TaskStatus::Completed.label(), "已完成");
    }
    #[test]
    fn fail_closed_state_branches() {
        for state in [State::Loading, State::Failed, State::NotDelivered] {
            let panel = Panel {
                state,
                tasks: Vec::new(),
            };
            assert_ne!(panel.state, State::Normal);
            assert_eq!(panel.capsule_text(), None);
        }
        assert_eq!(Panel::not_delivered().state, State::default());
    }
    #[test]
    fn capsule_prefers_running_and_counts_templates() {
        let mixed = Panel::delivered(vec![
            TaskStatus::Completed,
            TaskStatus::Completed,
            TaskStatus::Running,
        ]);
        assert_eq!(mixed.capsule_text().as_deref(), Some("1 个后台任务运行中"));
        assert_eq!(mixed.panel_title(), "1 个后台任务运行中");
        let done_only = Panel::delivered(vec![TaskStatus::Completed, TaskStatus::Completed]);
        assert_eq!(
            done_only.capsule_text().as_deref(),
            Some("2 个后台任务已完成")
        );
        assert_eq!(done_only.panel_title(), "2 个后台任务");
    }
}
