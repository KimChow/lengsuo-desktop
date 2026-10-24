//! 任务状态徽标浮层（波次 6，卡 `ui-overlay-task-status`，对齐 ui-parity-spec §8 A36）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `taskStatus`。
//! 数据源在运行时侧：[`TaskStatusState::initial`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "任务状态徽标";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 任务状态徽标（label + description 成对，逐字取自命名空间 `taskStatus`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    /// 执行中。
    Working,
    /// 规划中。
    Planning,
    /// 等待输入（唯一带 tag 的状态：待确认）。
    Pending,
    /// 已完成。
    Completed,
    /// 执行失败（error）。
    Error,
    /// 执行失败（failed）。
    Failed,
    /// 异常中断。
    Terminated,
    /// 已归档。
    Archived,
}

impl TaskStatus {
    /// 全部可枚举状态（按生命周期排序）。
    pub const ALL: &'static [Self] = &[
        Self::Working,
        Self::Planning,
        Self::Pending,
        Self::Completed,
        Self::Error,
        Self::Failed,
        Self::Terminated,
        Self::Archived,
    ];

    /// 徽标文案（label）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Working => "执行中",
            Self::Planning => "规划中",
            Self::Pending => "等待输入",
            Self::Completed => "已完成",
            Self::Error | Self::Failed => "执行失败",
            Self::Terminated => "异常中断",
            Self::Archived => "已归档",
        }
    }

    /// 徽标说明（description）。
    pub fn description(self) -> &'static str {
        match self {
            Self::Working => "Agent 正在执行计划中的步骤",
            Self::Planning => "Agent 正在分析任务并制定执行计划",
            Self::Pending => "需要你提供更多信息后才能继续",
            Self::Completed => "任务已结束（成功完成或被用户终止）",
            Self::Error | Self::Failed => "任务执行过程中发生错误",
            Self::Terminated => "任务异常退出",
            Self::Archived => "任务已被归档",
        }
    }

    /// 徽标附加标签（仅「等待输入」带「待确认」）。
    pub fn tag(self) -> Option<&'static str> {
        match self {
            Self::Pending => Some("待确认"),
            _ => None,
        }
    }
}

/// 浮层可枚举状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskStatusState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空集）。
    NotDelivered,
    /// 数据已下发：待展示的任务状态徽标清单。
    Ready(Vec<TaskStatus>),
}

impl TaskStatusState {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self::NotDelivered
    }

    /// 是否处于未下发态。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self, Self::NotDelivered)
    }

    /// 待展示徽标（未下发或空输入都为空切片 ⇒ 空态）。
    pub fn badges(&self) -> &[TaskStatus] {
        match self {
            Self::NotDelivered => &[],
            Self::Ready(list) => list,
        }
    }

    /// 是否落到空态（未下发 / 清单为空 ⇒ 如实显示「未下发」，⛔ 不渲染空列表）。
    pub fn is_empty_state(&self) -> bool {
        self.badges().is_empty()
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
/// 恒以 [`TaskStatusState::initial`]（未下发）渲染 —— fail-closed。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &TaskStatusState::initial())
}

/// 按状态渲染整层：标题行 + 徽标清单（空态 ⇒ 居中「未下发」）。
fn render(theme: &Theme, state: &TaskStatusState) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title_row(theme))
        .child(body(theme, state))
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

/// 主体：有徽标 ⇒ 清单行；空态 ⇒ 居中「未下发」（fail-closed，⛔ 不伪造条目）。
fn body(theme: &Theme, state: &TaskStatusState) -> Div {
    if state.is_empty_state() {
        return div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED);
    }
    div()
        .flex_1()
        .flex()
        .flex_col()
        .children(state.badges().iter().map(|status| badge_row(theme, *status)))
}

/// 徽标清单行：label 徽标 + 可选 tag 徽标 + description 说明。
fn badge_row(theme: &Theme, status: TaskStatus) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_full()
                .border_1()
                .border_color(theme.border)
                .text_sm()
                .text_color(theme.foreground)
                .child(status.label()),
        )
        .children(status.tag().map(|tag| {
            div()
                .px_2()
                .py_1()
                .rounded_full()
                .border_1()
                .border_color(theme.accent)
                .text_xs()
                .text_color(theme.accent)
                .child(tag)
        }))
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(status.description()),
        )
}
#[cfg(test)]
mod tests {
    use super::{main_area, TaskStatus, TaskStatusState, NOT_DELIVERED, TITLE};
    use gpui_kit::component::Theme;

    #[test]
    fn labels_are_verbatim() {
        assert_eq!(TaskStatus::Working.label(), "执行中");
        assert_eq!(TaskStatus::Planning.label(), "规划中");
        assert_eq!(TaskStatus::Pending.label(), "等待输入");
        assert_eq!(TaskStatus::Completed.label(), "已完成");
        assert_eq!(TaskStatus::Error.label(), "执行失败");
        assert_eq!(TaskStatus::Failed.label(), "执行失败");
        assert_eq!(TaskStatus::Terminated.label(), "异常中断");
        assert_eq!(TaskStatus::Archived.label(), "已归档");
    }

    #[test]
    fn descriptions_are_verbatim() {
        assert_eq!(
            TaskStatus::Working.description(),
            "Agent 正在执行计划中的步骤"
        );
        assert_eq!(
            TaskStatus::Planning.description(),
            "Agent 正在分析任务并制定执行计划"
        );
        assert_eq!(
            TaskStatus::Pending.description(),
            "需要你提供更多信息后才能继续"
        );
        assert_eq!(
            TaskStatus::Completed.description(),
            "任务已结束（成功完成或被用户终止）"
        );
        assert_eq!(
            TaskStatus::Error.description(),
            "任务执行过程中发生错误"
        );
        assert_eq!(
            TaskStatus::Failed.description(),
            "任务执行过程中发生错误"
        );
        assert_eq!(TaskStatus::Terminated.description(), "任务异常退出");
        assert_eq!(TaskStatus::Archived.description(), "任务已被归档");
    }

    #[test]
    fn pending_is_the_only_tagged_status() {
        assert_eq!(TaskStatus::Pending.tag(), Some("待确认"));
        for status in TaskStatus::ALL {
            if !matches!(status, TaskStatus::Pending) {
                assert_eq!(status.tag(), None);
            }
        }
    }

    #[test]
    fn catalog_covers_all_enumerable_statuses() {
        assert_eq!(TaskStatus::ALL.len(), 8);
        let mut labels: Vec<_> = TaskStatus::ALL.iter().map(|s| s.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 7, "仅 error/failed 共享文案，其余 label 唯一");
    }

    #[test]
    fn initial_state_is_fail_closed_not_delivered() {
        let state = TaskStatusState::initial();
        assert!(state.is_not_delivered());
        assert!(state.is_empty_state());
        assert!(state.badges().is_empty());
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(TITLE, "任务状态徽标");
    }

    #[test]
    fn empty_input_is_empty_state_not_blank_list() {
        let state = TaskStatusState::Ready(Vec::new());
        assert!(!state.is_not_delivered());
        assert!(state.is_empty_state());
        assert!(state.badges().is_empty());
    }

    #[test]
    fn ready_badges_pass_through_verbatim() {
        let state = TaskStatusState::Ready(vec![TaskStatus::Archived, TaskStatus::Pending]);
        assert!(!state.is_empty_state());
        assert_eq!(state.badges(), &[TaskStatus::Archived, TaskStatus::Pending]);
        assert_eq!(state.badges()[0].label(), "已归档");
        assert_eq!(state.badges()[1].tag(), Some("待确认"));
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    #[test]
    fn render_branches_build_without_panic() {
        let theme = Theme::default();
        let _ = super::render(&theme, &TaskStatusState::NotDelivered);
        let _ = super::render(&theme, &TaskStatusState::Ready(Vec::new()));
        let _ = super::render(
            &theme,
            &TaskStatusState::Ready(TaskStatus::ALL.to_vec()),
        );
    }

    #[test]
    fn empty_state_centers_not_delivered_hint() {
        // 空态 = 居中提示「未下发」，不是空白或空列表（fail-closed 契约由测试钉死）。
        for state in [
            TaskStatusState::NotDelivered,
            TaskStatusState::Ready(Vec::new()),
        ] {
            assert!(state.is_empty_state());
        }
        let ready = TaskStatusState::Ready(TaskStatus::ALL.to_vec());
        assert!(!ready.is_empty_state());
    }
}
