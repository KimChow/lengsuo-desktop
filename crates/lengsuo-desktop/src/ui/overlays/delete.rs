//! 删除任务确认浮层（波次 8，卡 `ui-overlay-delete`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `delete`。
//! 数据源在运行时侧：浮层数据未接入前，保持「未下发」的诚实默认态。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ── 纯数据层（无 GPUI 依赖） ─────────────────────────────────────────

/// 界面文案全表：命名空间 `delete` 全 27 键（`key`、中文），逐字取自文案全表。
pub const TEXTS: [(&str, &str); 27] = [
    ("delete.dialog.cancel", "取消"),
    ("delete.dialog.confirm", "确认删除"),
    ("delete.dialog.content", "确认后将从列表中删除任务，请确认是否删除？"),
    ("delete.dialog.title", "删除任务"),
    ("delete.inline.confirm", "确认"),
    (
        "delete.inline.content",
        "删除后，这些任务将从你的所有设备中移除，且无法恢复。",
    ),
    (
        "delete.inline.running.confirm",
        "停止并删除",
    ),
    (
        "delete.inline.running.content",
        "该任务正在执行中，删除将立即终止运行，且无法恢复。",
    ),
    ("delete.inline.running.title", "删除任务？"),
    ("delete.inline.title", "删除任务？"),
    ("delete.toast.failed", "删除失败，请重试"),
    ("delete.toast.loading", "正在删除..."),
    ("delete.toast.success", "删除成功"),
    ("delete.workspace.dialog.confirm", "确认移除"),
    (
        "delete.workspace.dialog.content",
        "确认后将从列表中移除工作空间，请确认是否移除？",
    ),
    (
        "delete.workspace.dialog.contentWithPinnedTasks",
        "该工作空间下有 {count} 个任务（含置顶任务），移除工作空间后这些任务将被同时删除且无法恢复，确认移除？",
    ),
    (
        "delete.workspace.dialog.contentWithTasks",
        "该工作空间下有 {count} 个任务，移除工作空间后这些任务将被同时删除且无法恢复，确认移除？",
    ),
    ("delete.workspace.dialog.title", "移除工作空间"),
    ("delete.workspace.toast.failed", "移除失败，请重试"),
    ("delete.workspace.toast.success", "移除成功"),
    ("delete.worktree.dialog.confirm", "确认"),
    (
        "delete.worktree.dialog.content",
        "确认删除后，该任务将从列表中彻底移除，并清理关联的任务详情与执行记录。",
    ),
    (
        "delete.worktree.dialog.deleteFiles",
        "同时删除 Worktree 分支和文件",
    ),
    (
        "delete.worktree.dialog.dirtyWarningSummary",
        "Uncommitted +{stagedAdditions} -{stagedDeletions}，Unstaged +{unstagedAdditions} -{unstagedDeletions}，可取消删除后回到对话中处理。",
    ),
    (
        "delete.worktree.dialog.dirtyWarningTitle",
        "删除 Worktree 会丢失未提交改动",
    ),
    ("delete.worktree.dialog.title", "确认删除任务？"),
    (
        "delete.worktree.toast.failed",
        "任务已删除，Worktree 分支和文件删除失败",
    ),
];

/// 按全表 key 取文案；未命中 ⇒ `None`（fail-closed，⛔ 不回退到自编文案）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 视图标题（浮层名）。
pub const TITLE: &str = "删除任务确认浮层";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 浮层状态：三条业务分支 + 提交反馈 + 未下发默认态。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 删除确认分支（`delete.dialog.*`）。
    Dialog,
    /// 行内删除分支（`delete.inline.*`）。
    Inline,
    /// 运行中删除分支（`delete.inline.running.*`）。
    Running,
    /// 提交删除中（`delete.toast.loading`）。
    Loading,
    /// 删除成功（`delete.toast.success`）。
    Success,
    /// 删除失败（`delete.toast.failed`）。
    Failed,
    /// 数据未下发（诚实默认态：未拉取过 ≠ 空集，⛔ 不伪造）。
    #[default]
    NotDelivered,
}

/// 三条业务分支：删除确认 / 行内删除 / 运行中删除。
pub const BRANCHES: [State; 3] = [State::Dialog, State::Inline, State::Running];

/// 浮层状态载体（纯数据层；视图是它的纯函数）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeleteOverlay {
    /// 当前状态分支。
    pub state: State,
}

impl DeleteOverlay {
    /// 由已下发的分支序列构造；空输入 ⇒ `NotDelivered` 空态（⛔ 不是空列表）。
    pub fn from_delivered(states: &[State]) -> Self {
        states
            .first()
            .copied()
            .map_or(Self::default(), |state| Self { state })
    }

    /// 分支标题；非业务分支 ⇒ `None`（fail-closed）。
    pub fn title(&self) -> Option<&'static str> {
        match self.state {
            State::Dialog => text("delete.dialog.title"),
            State::Inline => text("delete.inline.title"),
            State::Running => text("delete.inline.running.title"),
            _ => None,
        }
    }

    /// 分支正文；非业务分支 ⇒ `None`。
    pub fn content(&self) -> Option<&'static str> {
        match self.state {
            State::Dialog => text("delete.dialog.content"),
            State::Inline => text("delete.inline.content"),
            State::Running => text("delete.inline.running.content"),
            _ => None,
        }
    }

    /// 确认动作文案；非业务分支 ⇒ `None`。
    pub fn confirm_label(&self) -> Option<&'static str> {
        match self.state {
            State::Dialog => text("delete.dialog.confirm"),
            State::Inline => text("delete.inline.confirm"),
            State::Running => text("delete.inline.running.confirm"),
            _ => None,
        }
    }

    /// 取消动作文案（仅删除确认分支提供）；其余 ⇒ `None`。
    pub fn cancel_label(&self) -> Option<&'static str> {
        match self.state {
            State::Dialog => text("delete.dialog.cancel"),
            _ => None,
        }
    }

    /// 提交反馈文案（进行中/成功/失败 toast）；其余 ⇒ `None`。
    pub fn toast_text(&self) -> Option<&'static str> {
        match self.state {
            State::Loading => text("delete.toast.loading"),
            State::Success => text("delete.toast.success"),
            State::Failed => text("delete.toast.failed"),
            _ => None,
        }
    }
}

// ── GPUI 层 ─────────────────────────────────────────────────────────

/// 单条分支卡片：标题 / 正文 / 动作行（颜色一律 theme token，⛔ 不硬编码色值）。
fn branch_card(theme: &Theme, overlay: &DeleteOverlay) -> Div {
    let confirm = div()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.danger)
        .text_color(theme.danger)
        .child(overlay.confirm_label().unwrap_or(NOT_DELIVERED));
    let cancel = match overlay.cancel_label() {
        Some(label) => div()
            .px_3()
            .py_2()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .text_color(theme.muted_foreground)
            .child(label),
        None => div(),
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(
            div()
                .text_color(theme.foreground)
                .child(overlay.title().unwrap_or(NOT_DELIVERED)),
        )
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(overlay.content().unwrap_or(NOT_DELIVERED)),
        )
        .child(div().flex().items_center().gap_2().child(confirm).child(cancel))
}

/// 提交反馈 chip（进行中/成功/失败 toast 文案，语义色取 theme token）。
fn toast_chip(theme: &Theme, state: State) -> Div {
    let overlay = DeleteOverlay { state };
    let text_color = match state {
        State::Success => theme.success,
        State::Failed => theme.danger,
        _ => theme.muted_foreground,
    };
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_color(text_color)
        .child(overlay.toast_text().unwrap_or(NOT_DELIVERED))
}

/// 空态节点：无数据源时如实展示「未下发」（fail-closed，⛔ 不伪造条目/数值）。
fn not_delivered_card(theme: &Theme) -> Div {
    let empty = DeleteOverlay::from_delivered(&[]);
    div()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(empty.title().unwrap_or(NOT_DELIVERED)),
        )
}

/// 主区渲染：标题行 + 三条分支卡片 + 反馈行 + 空态（未下发 fail-closed 分支）。
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(TITLE))
        .child(branch_card(theme, &DeleteOverlay { state: State::Dialog }))
        .child(branch_card(theme, &DeleteOverlay { state: State::Inline }))
        .child(branch_card(theme, &DeleteOverlay { state: State::Running }))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(toast_chip(theme, State::Loading))
                .child(toast_chip(theme, State::Success))
                .child(toast_chip(theme, State::Failed)),
        )
        .child(not_delivered_card(theme))
}
#[cfg(test)]
mod tests {
    use super::{main_area, State, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
    }
    #[test]
    fn title_not_empty() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
