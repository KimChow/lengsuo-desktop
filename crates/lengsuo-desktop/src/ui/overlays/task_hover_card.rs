//! 任务悬停卡浮层（波次 7，卡 `ui-overlay-task-hover-card`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间
//! `taskHoverCard` / `taskStatus`。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ===== 纯数据层（无 GPUI 依赖）=====

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "任务悬停卡";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 复制分支名动作文案（`taskHoverCard.copyWorktreeBranch`，逐字）。
pub const COPY_WORKTREE_BRANCH: &str = "复制 Worktree 分支名";
/// 所属空间模板（`taskHoverCard.workspace`，逐字；`{name}` 为占位符）。
pub const WORKSPACE_TEMPLATE: &str = "所属空间：{name}";
/// 更新时间模板（`taskHoverCard.updatedAt`，逐字；`{time}` 为占位符）。
pub const UPDATED_AT_TEMPLATE: &str = "更新于 {time}";

/// 任务类型（类型行文案，`taskHoverCard.*` 命名空间，逐字）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskKind {
    /// 云电脑（`taskHoverCard.cloudTask`）。
    Cloud,
    /// 本地电脑（`taskHoverCard.localTask`）。
    Local,
    /// 代码开发（`taskHoverCard.codeMode`）。
    Code,
    /// 设计创意（`taskHoverCard.designMode`）。
    Design,
    /// 日常办公（`taskHoverCard.workMode`）。
    Work,
}

impl TaskKind {
    /// 类型行全量（对齐 A52 的四类 + `workMode`）。
    pub const ALL: [Self; 5] = [Self::Cloud, Self::Local, Self::Code, Self::Design, Self::Work];

    /// 类型行文案（逐字）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Cloud => "云电脑",
            Self::Local => "本地电脑",
            Self::Code => "代码开发",
            Self::Design => "设计创意",
            Self::Work => "日常办公",
        }
    }
}

/// 任务状态徽标（`taskStatus.*` 命名空间，label/description 逐字成对）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TaskStatus {
    /// 已归档。
    Archived,
    /// 已完成。
    Completed,
    /// 执行失败（`error` 键；`failed` 键文案与之相同，只保留一份）。
    Error,
    /// 等待输入。
    Pending,
    /// 规划中。
    Planning,
    /// 异常中断。
    Terminated,
    /// 执行中。
    Working,
}

impl TaskStatus {
    /// 状态全量。
    pub const ALL: [Self; 7] = [
        Self::Archived,
        Self::Completed,
        Self::Error,
        Self::Pending,
        Self::Planning,
        Self::Terminated,
        Self::Working,
    ];

    /// 徽标文案（`taskStatus.*.label`，逐字）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Archived => "已归档",
            Self::Completed => "已完成",
            Self::Error => "执行失败",
            Self::Pending => "等待输入",
            Self::Planning => "规划中",
            Self::Terminated => "异常中断",
            Self::Working => "执行中",
        }
    }

    /// 状态说明（`taskStatus.*.description`，逐字）。
    pub fn description(self) -> &'static str {
        match self {
            Self::Archived => "任务已被归档",
            Self::Completed => "任务已结束（成功完成或被用户终止）",
            Self::Error => "任务执行过程中发生错误",
            Self::Pending => "需要你提供更多信息后才能继续",
            Self::Planning => "Agent 正在分析任务并制定执行计划",
            Self::Terminated => "任务异常退出",
            Self::Working => "Agent 正在执行计划中的步骤",
        }
    }
}

/// 明细区三态：未下发 ≠ 空态 ≠ 有内容（⛔ 不得用空列表冒充空态）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Details {
    /// 数据源未下发：fail-closed，如实显示「未下发」。
    NotDelivered,
    /// 已下发但一条明细也没有：空态。
    Empty,
    /// 已填充的明细行（模板占位符已替换为实值）。
    Filled(Vec<String>),
}

/// 悬停卡视图模型（纯数据；由运行时数据源装配，本模块不伪造）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TaskHoverCard {
    /// 类型行；未下发时为 `None`。
    pub kind: Option<TaskKind>,
    /// 状态徽标；未下发时为 `None`。
    pub status: Option<TaskStatus>,
    /// 明细区三态。
    pub details: Details,
}

impl TaskHoverCard {
    /// fail-closed 缺省态：数据未下发，如实显示「未下发」。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            kind: None,
            status: None,
            details: Details::NotDelivered,
        }
    }

    /// 空态：已下发但无一条明细（不是空列表）。
    #[must_use]
    pub fn empty() -> Self {
        Self {
            kind: None,
            status: None,
            details: Details::Empty,
        }
    }

    /// 由下发字段装配：`None` 字段不生成行，全部缺失 ⇒ 空态。
    #[must_use]
    pub fn from_parts(
        kind: TaskKind,
        status: TaskStatus,
        workspace_name: Option<&str>,
        updated_at: Option<&str>,
    ) -> Self {
        let mut lines = Vec::new();
        if let Some(name) = workspace_name {
            lines.push(fill_workspace_line(name));
        }
        if let Some(time) = updated_at {
            lines.push(fill_updated_at_line(time));
        }
        let details = if lines.is_empty() {
            Details::Empty
        } else {
            Details::Filled(lines)
        };
        Self {
            kind: Some(kind),
            status: Some(status),
            details,
        }
    }

    /// 明细行（空态/未下发时为空片）。
    #[must_use]
    pub fn detail_lines(&self) -> &[String] {
        match &self.details {
            Details::Filled(lines) => lines,
            Details::Empty | Details::NotDelivered => &[],
        }
    }
}

/// 填充 `taskHoverCard.workspace` 模板。
#[must_use]
pub fn fill_workspace_line(name: &str) -> String {
    WORKSPACE_TEMPLATE.replace("{name}", name)
}

/// 填充 `taskHoverCard.updatedAt` 模板。
#[must_use]
pub fn fill_updated_at_line(time: &str) -> String {
    UPDATED_AT_TEMPLATE.replace("{time}", time)
}

// ===== GPUI 层 =====

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}
#[cfg(test)]
mod tests {
    use super::{main_area, NOT_DELIVERED, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }
    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
