//! 定时任务主区视图（波次 5，卡 `ui-automation-center`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，不 import GPUI）：条目 / 分组 / 空态 / 诚实占位文案，
//!   外加 [`定时任务View::nodes`] 作为元素树的**纯描述**（渲染层只翻译不计算）；
//! - **GPUI 层**（[`main_area`]）：把节点描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自界面文案全表命名空间 `automation`）。
//!
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后由
//! `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ===== 纯数据层（无 GPUI 依赖）=====

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "定时任务";

/// 面板描述（`automation.list.description`，逐字）。
pub const DESCRIPTION: &str = "配置定时任务，冷锁 将按计划定时执行任务";

/// 清单空态主文案（`automation.empty`，逐字）。
pub const EMPTY: &str = "暂无定时任务";

/// 清单空态引导文案（`automation.empty.firstTask`，逐字）。
pub const EMPTY_FIRST_TASK: &str = "开启你的第一个定时任务吧";

/// 归档区空态文案（`automation.empty.archived`，逐字）。
pub const EMPTY_ARCHIVED: &str = "暂无归档记录";

/// 分组头：当前（`automation.list.groupCurrent`，逐字）。
pub const GROUP_CURRENT: &str = "当前";

/// 分组头：已暂停（`automation.list.groupPaused`，逐字）。
pub const GROUP_PAUSED: &str = "已暂停";

/// 组头计数模板（`automation.list.pagination.total`，逐字，`{count}` 占位）。
pub const TOTAL_TEMPLATE: &str = "共 {count} 条";

/// 任务状态徽标：已启用（`automation.filter.statusActive`，逐字）。
pub const STATUS_ACTIVE: &str = "已启用";

/// 任务状态徽标：已暂停（`automation.filter.statusPaused`，逐字）。
pub const STATUS_PAUSED: &str = "已暂停";

/// 上次结果：成功（`automation.filter.success`，逐字）。
pub const RESULT_SUCCESS: &str = "成功";

/// 上次结果：失败（`automation.filter.failed`，逐字）。
pub const RESULT_FAILED: &str = "失败";

/// 上次结果：待执行（`automation.run.status.pending`，逐字）。
pub const RESULT_PENDING: &str = "待执行";

/// 行内动作：查看运行详情（`automation.action.viewRun`，逐字）。
pub const ACTION_VIEW_RUN: &str = "查看运行详情";

/// 行内动作：查看任务（`automation.action.viewTask`，逐字）。
pub const ACTION_VIEW_TASK: &str = "查看任务";

/// 行内动作：归档（`automation.action.archive`，逐字）。
pub const ACTION_ARCHIVE: &str = "归档";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 任务启停状态（运行态数据未下发，不伪造「运行中」组）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// 已启用（归入「当前」组）。
    Active,
    /// 已暂停（归入「已暂停」组）。
    Paused,
}

impl TaskStatus {
    /// 状态徽标文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Active => STATUS_ACTIVE,
            Self::Paused => STATUS_PAUSED,
        }
    }
}

/// 上次运行结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LastResult {
    /// 成功。
    Success,
    /// 失败。
    Failed,
    /// 待执行（尚无运行记录时的诚实占位，非成功非失败）。
    Pending,
}

impl LastResult {
    /// 结果徽标文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Success => RESULT_SUCCESS,
            Self::Failed => RESULT_FAILED,
            Self::Pending => RESULT_PENDING,
        }
    }
}

/// 一条定时任务（名称 + 状态 + 上次结果；调度字段未下发，不伪造）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTask {
    /// 任务名。
    pub name: String,
    /// 启停状态。
    pub status: TaskStatus,
    /// 上次运行结果。
    pub last_result: LastResult,
}

/// 清单分组（固定顺序：当前 → 已暂停；空组不产出）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGroup {
    /// 组标签文案。
    pub label: &'static str,
    /// 组内任务。
    pub tasks: Vec<ScheduledTask>,
}

/// 可枚举的视图状态（未下发 ≠ 空态，空态 ≠ 空清单）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewState {
    /// 数据未下发：fail-closed 显示「未下发」。
    NotDelivered,
    /// 数据已下发且为空：显示空态文案。
    Empty,
    /// 有条目：显示分组清单。
    Entries,
}

/// 定时任务视图的纯数据描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 定时任务View {
    /// 视图状态。
    pub state: ViewState,
    /// 分组清单（仅 `Entries` 态非空）。
    pub groups: Vec<TaskGroup>,
}

/// 视图构造器（纯函数）。
///
/// * `None` ⇒ 数据未下发（`NotDelivered`，显示「未下发」）；
/// * `Some(空)` ⇒ 空态（`Empty`，显示空态文案，**不是**空清单）；
/// * `Some(非空)` ⇒ 分组清单。
#[must_use]
pub fn build(tasks: Option<Vec<ScheduledTask>>) -> 定时任务View {
    match tasks {
        None => 定时任务View {
            state: ViewState::NotDelivered,
            groups: Vec::new(),
        },
        Some(list) if list.is_empty() => 定时任务View {
            state: ViewState::Empty,
            groups: Vec::new(),
        },
        Some(list) => 定时任务View {
            state: ViewState::Entries,
            groups: group_tasks(list),
        },
    }
}

/// 按状态分组；固定顺序「当前 → 已暂停」，空组不产出。
fn group_tasks(tasks: Vec<ScheduledTask>) -> Vec<TaskGroup> {
    let mut current = Vec::new();
    let mut paused = Vec::new();
    for task in tasks {
        match task.status {
            TaskStatus::Active => current.push(task),
            TaskStatus::Paused => paused.push(task),
        }
    }
    [(GROUP_CURRENT, current), (GROUP_PAUSED, paused)]
        .into_iter()
        .filter(|(_, tasks)| !tasks.is_empty())
        .map(|(label, tasks)| TaskGroup { label, tasks })
        .collect()
}

/// 把 `{count}` 占位模板格式化为计数文案。
#[must_use]
pub fn format_total(count: usize) -> String {
    TOTAL_TEMPLATE.replace("{count}", &count.to_string())
}

/// 元素树节点（渲染层的唯一信息源；测试据此定位标题 / 清单 / 空态节点）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewNode {
    /// 标题行。
    Title,
    /// 描述行。
    Description,
    /// 分组头（标签 + 计数文案）。
    GroupHeader {
        /// 组标签。
        label: &'static str,
        /// 计数文案（如「共 2 条」）。
        summary: String,
    },
    /// 任务行（名称 + 徽标 + 动作）。
    TaskRow {
        /// 任务名。
        name: String,
        /// 状态 / 结果徽标。
        badges: Vec<&'static str>,
        /// 行内动作。
        actions: Vec<&'static str>,
    },
    /// 归档区空态行（归档数据未下发，如实显示空态文案）。
    ArchivedEmpty,
    /// 清单空态块。
    EmptyState,
    /// 「未下发」fail-closed 块。
    NotDeliveredBlock,
}

impl 定时任务View {
    /// 展开为元素树节点序列（纯函数；渲染层只翻译不计算）。
    #[must_use]
    pub fn nodes(&self) -> Vec<ViewNode> {
        match self.state {
            ViewState::NotDelivered => vec![ViewNode::Title, ViewNode::NotDeliveredBlock],
            ViewState::Empty => vec![ViewNode::Title, ViewNode::Description, ViewNode::EmptyState],
            ViewState::Entries => {
                let mut nodes = vec![ViewNode::Title, ViewNode::Description];
                for group in &self.groups {
                    nodes.push(ViewNode::GroupHeader {
                        label: group.label,
                        summary: format_total(group.tasks.len()),
                    });
                    for task in &group.tasks {
                        nodes.push(ViewNode::TaskRow {
                            name: task.name.clone(),
                            badges: vec![task.status.label(), task.last_result.label()],
                            actions: vec![ACTION_VIEW_RUN, ACTION_VIEW_TASK, ACTION_ARCHIVE],
                        });
                    }
                }
                nodes.push(ViewNode::ArchivedEmpty);
                nodes
            }
        }
    }
}

// ===== GPUI 层 =====

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
///
/// 数据源未接线：当前固定 `build(None)`（fail-closed「未下发」）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&build(None), theme)
}

/// 按节点描述渲染（[`main_area`] 的渲染核；测试直接驱动各状态）。
fn render(view: &定时任务View, theme: &Theme) -> Div {
    let mut column = div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col();
    for node in view.nodes() {
        column = column.child(node_element(&node, theme));
    }
    column
}

/// 单个节点的元素树翻译（只翻译，不计算；颜色一律主题 token）。
fn node_element(node: &ViewNode, theme: &Theme) -> Div {
    match node {
        ViewNode::Title => div()
            .px_4()
            .py_2()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(TITLE),
        ViewNode::Description => div()
            .px_4()
            .pb_2()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(DESCRIPTION),
        ViewNode::GroupHeader { label, summary } => div()
            .px_4()
            .py_1()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .bg(theme.muted)
            .text_xs()
            .font_medium()
            .text_color(theme.muted_foreground)
            .child(*label)
            .child(summary.clone()),
        ViewNode::TaskRow {
            name,
            badges,
            actions,
        } => {
            let mut trailing = div().flex().flex_row().items_center().gap_2().text_xs();
            for badge in badges {
                trailing = trailing.child(badge_chip(badge, theme));
            }
            for (rank, action) in actions.iter().enumerate() {
                let color = if rank == 0 {
                    theme.accent
                } else {
                    theme.muted_foreground
                };
                trailing = trailing.child(div().py_1().text_color(color).child(*action));
            }
            div()
                .px_4()
                .py_2()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(name.clone()),
                )
                .child(trailing)
        }
        ViewNode::ArchivedEmpty => div()
            .px_4()
            .py_2()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(EMPTY_ARCHIVED),
        ViewNode::EmptyState => centered_block(theme, EMPTY, Some(EMPTY_FIRST_TASK)),
        ViewNode::NotDeliveredBlock => centered_block(theme, NOT_DELIVERED, None),
    }
}

/// 徽标（状态 / 结果），触摸目标按无障碍基线留足内边距。
fn badge_chip(label: &'static str, theme: &Theme) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded_sm()
        .bg(theme.muted)
        .text_color(theme.muted_foreground)
        .child(label)
}

/// 居中状态块：主文案 + 可选引导文案（空态 / 未下发共用）。
fn centered_block(theme: &Theme, primary: &'static str, secondary: Option<&'static str>) -> Div {
    let mut block = div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .py_6()
        .text_sm()
        .text_color(theme.muted_foreground);
    block = block.child(div().child(primary));
    if let Some(hint) = secondary {
        block = block.child(div().text_xs().child(hint));
    }
    block
}

#[cfg(test)]
mod tests {
    use super::{
        build, format_total, group_tasks, LastResult, ScheduledTask, TaskStatus, ViewState,
        ACTION_VIEW_RUN, EMPTY, EMPTY_ARCHIVED, EMPTY_FIRST_TASK, GROUP_CURRENT, GROUP_PAUSED,
        NOT_DELIVERED, RESULT_FAILED, RESULT_SUCCESS, STATUS_ACTIVE, STATUS_PAUSED, TITLE,
    };
    use gpui_kit::component::Theme;

    fn task(name: &str, status: TaskStatus, last_result: LastResult) -> ScheduledTask {
        ScheduledTask {
            name: name.to_string(),
            status,
            last_result,
        }
    }

    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert_eq!(TITLE, "定时任务");
    }

    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn no_data_falls_back_to_not_delivered_state() {
        let view = build(None);
        assert_eq!(view.state, ViewState::NotDelivered);
        assert!(view.groups.is_empty());
    }

    #[test]
    fn empty_input_is_empty_state_not_blank_entries() {
        let view = build(Some(Vec::new()));
        assert_eq!(view.state, ViewState::Empty);
        assert!(view.groups.is_empty());
    }

    #[test]
    fn groups_tasks_in_fixed_order_and_skips_empty_groups() {
        let groups = group_tasks(vec![
            task("晨报", TaskStatus::Active, LastResult::Success),
            task("周清", TaskStatus::Paused, LastResult::Failed),
            task("巡检", TaskStatus::Active, LastResult::Pending),
        ]);
        let labels: Vec<_> = groups.iter().map(|g| g.label).collect();
        assert_eq!(labels, vec![GROUP_CURRENT, GROUP_PAUSED]);
        assert_eq!(groups[0].tasks.len(), 2);
        assert_eq!(groups[1].tasks.len(), 1);

        let only_paused = group_tasks(vec![task("周清", TaskStatus::Paused, LastResult::Failed)]);
        assert_eq!(only_paused.len(), 1);
        assert_eq!(only_paused[0].label, GROUP_PAUSED);
    }

    #[test]
    fn total_template_formats_count_verbatim() {
        assert_eq!(format_total(2), "共 2 条");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = super::main_area(&theme);
    }

    #[test]
    fn copy_is_verbatim_from_namespace() {
        assert_eq!(
            super::DESCRIPTION,
            "配置定时任务，冷锁 将按计划定时执行任务"
        );
        assert_eq!(EMPTY, "暂无定时任务");
        assert_eq!(EMPTY_FIRST_TASK, "开启你的第一个定时任务吧");
        assert_eq!(EMPTY_ARCHIVED, "暂无归档记录");
        assert_eq!(STATUS_ACTIVE, "已启用");
        assert_eq!(STATUS_PAUSED, "已暂停");
        assert_eq!(RESULT_SUCCESS, "成功");
        assert_eq!(RESULT_FAILED, "失败");
        assert_eq!(super::RESULT_PENDING, "待执行");
        assert_eq!(ACTION_VIEW_RUN, "查看运行详情");
        assert_eq!(super::ACTION_VIEW_TASK, "查看任务");
        assert_eq!(super::ACTION_ARCHIVE, "归档");
    }

    #[test]
    fn status_and_result_labels_map_verbatim() {
        assert_eq!(TaskStatus::Active.label(), STATUS_ACTIVE);
        assert_eq!(TaskStatus::Paused.label(), STATUS_PAUSED);
        assert_eq!(LastResult::Success.label(), RESULT_SUCCESS);
        assert_eq!(LastResult::Failed.label(), RESULT_FAILED);
        assert_eq!(LastResult::Pending.label(), super::RESULT_PENDING);
    }

    #[test]
    fn nodes_locate_title_and_at_least_three_row_nodes() {
        let view = build(Some(vec![
            task("晨报", TaskStatus::Active, LastResult::Success),
            task("巡检", TaskStatus::Active, LastResult::Pending),
            task("周清", TaskStatus::Paused, LastResult::Failed),
        ]));
        let nodes = view.nodes();
        assert_eq!(nodes.first(), Some(&super::ViewNode::Title));
        let rows = nodes
            .iter()
            .filter(|n| {
                matches!(
                    n,
                    super::ViewNode::GroupHeader { .. } | super::ViewNode::TaskRow { .. }
                )
            })
            .count();
        assert!(rows >= 3, "group/row nodes = {rows}");
        assert!(nodes.contains(&super::ViewNode::ArchivedEmpty));
    }

    #[test]
    fn empty_and_not_delivered_nodes_stay_honest() {
        let empty_nodes = build(Some(Vec::new())).nodes();
        assert_eq!(empty_nodes.first(), Some(&super::ViewNode::Title));
        assert!(empty_nodes.contains(&super::ViewNode::EmptyState));
        assert!(!empty_nodes
            .iter()
            .any(|n| matches!(n, super::ViewNode::TaskRow { .. })));

        let nd_nodes = build(None).nodes();
        assert!(nd_nodes.contains(&super::ViewNode::NotDeliveredBlock));
        assert!(!nd_nodes
            .iter()
            .any(|n| matches!(n, super::ViewNode::TaskRow { .. })));
    }

    #[test]
    fn render_every_state_without_panic() {
        let theme = Theme::default();
        let _ = super::main_area(&theme);
        for view in [
            build(None),
            build(Some(Vec::new())),
            build(Some(vec![task(
                "晨报",
                TaskStatus::Active,
                LastResult::Success,
            )])),
        ] {
            let _ = super::render(&view, &theme);
        }
    }
}
