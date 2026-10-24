//! 归档任务确认浮层（波次 8，卡 `ui-overlay-archive`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（[`copy`] 全量文案 + [`ArchiveOverlay`]
//! 状态模型）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `archive`。
//! 数据源在运行时侧：未接入前 [`main_area`] 渲染诚实默认态（未下发）。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 命名空间 `archive` 的界面文案（逐字，共 17 键）。
pub mod copy {
    /// `archive.dialog.cancel`
    pub const DIALOG_CANCEL: &str = "取消";
    /// `archive.dialog.confirm`
    pub const DIALOG_CONFIRM: &str = "确认归档";
    /// `archive.dialog.content`
    pub const DIALOG_CONTENT: &str =
        "确认将该任务归档吗？归档后可在「系统设置-数据管理」中查看已归档任务。";
    /// `archive.dialog.content.cloud`
    pub const DIALOG_CONTENT_CLOUD: &str = "确认将该任务归档吗？";
    /// `archive.dialog.title`
    pub const DIALOG_TITLE: &str = "归档任务";
    /// `archive.disabled.reason`
    pub const DISABLED_REASON: &str = "任务进行中，无法归档";
    /// `archive.inline.confirm`
    pub const INLINE_CONFIRM: &str = "确认";
    /// `archive.page.empty`
    pub const PAGE_EMPTY: &str = "暂无归档内容";
    /// `archive.page.title`
    pub const PAGE_TITLE: &str = "已归档任务";
    /// `archive.toast.failed`
    pub const TOAST_FAILED: &str = "归档失败，请重试";
    /// `archive.toast.success`（键值尾随半角空格，原样保留）
    pub const TOAST_SUCCESS: &str = "任务已归档，可在 ";
    /// `archive.toast.success.cloud`
    pub const TOAST_SUCCESS_CLOUD: &str = "任务已归档";
    /// `archive.toast.successLink`
    pub const TOAST_SUCCESS_LINK: &str = "设置";
    /// `archive.toast.successSuffix`（键值前导半角空格，原样保留）
    pub const TOAST_SUCCESS_SUFFIX: &str = " 中查看";
    /// `archive.unarchive`
    pub const UNARCHIVE: &str = "取消归档";
    /// `archive.unarchive.failed`
    pub const UNARCHIVE_FAILED: &str = "恢复失败，请重试";
    /// `archive.unarchive.success`
    pub const UNARCHIVE_SUCCESS: &str = "任务已恢复";
    /// 全量键位数组（防丢键：长度恒为 17）。
    pub const ALL: [&str; 17] = [
        DIALOG_CANCEL,
        DIALOG_CONFIRM,
        DIALOG_CONTENT,
        DIALOG_CONTENT_CLOUD,
        DIALOG_TITLE,
        DISABLED_REASON,
        INLINE_CONFIRM,
        PAGE_EMPTY,
        PAGE_TITLE,
        TOAST_FAILED,
        TOAST_SUCCESS,
        TOAST_SUCCESS_CLOUD,
        TOAST_SUCCESS_LINK,
        TOAST_SUCCESS_SUFFIX,
        UNARCHIVE,
        UNARCHIVE_FAILED,
        UNARCHIVE_SUCCESS,
    ];
}

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    /// 拉取中（不显示条目，也不宣称空集）。
    Loading,
    /// 下发/操作失败（提示失败文案）。
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 归档任务确认浮层的纯数据模型（无 GPUI 依赖；视图是它的纯函数）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArchiveOverlay {
    /// 浮层状态。
    pub state: State,
    /// 云端环境：正文与成功 toast 用短文案（无「设置」链接段）。
    pub cloud: bool,
    /// 任务进行中：确认入口禁用，正文替换为禁用原因。
    pub task_running: bool,
    /// 已归档任务名（数据源未接入前恒为空 ⇒ 归档页空态）。
    pub archived_tasks: Vec<String>,
}

impl ArchiveOverlay {
    /// 确认对话框正文：任务进行中 ⇒ 禁用原因；云端 ⇒ 短文案；否则完整文案。
    pub fn dialog_content(&self) -> &'static str {
        if self.task_running {
            copy::DISABLED_REASON
        } else if self.cloud {
            copy::DIALOG_CONTENT_CLOUD
        } else {
            copy::DIALOG_CONTENT
        }
    }

    /// 成功 toast：本地版由三键拼装（含「设置」链接段），云端版无链接段。
    pub fn toast_success(&self) -> String {
        if self.cloud {
            copy::TOAST_SUCCESS_CLOUD.to_string()
        } else {
            format!(
                "{}{}{}",
                copy::TOAST_SUCCESS,
                copy::TOAST_SUCCESS_LINK,
                copy::TOAST_SUCCESS_SUFFIX
            )
        }
    }

    /// 状态提示行：未下发/失败时向用户言明，其余为 `None`。
    pub fn status_notice(&self) -> Option<&'static str> {
        match self.state {
            State::NotDelivered => Some(NOT_DELIVERED),
            State::Failed => Some(copy::TOAST_FAILED),
            State::Loading | State::Normal => None,
        }
    }

    /// 确认按钮是否可用（任务进行中禁用）。
    pub fn confirm_enabled(&self) -> bool {
        !self.task_running
    }

    /// 归档页已归档条数。
    pub fn archived_count(&self) -> usize {
        self.archived_tasks.len()
    }

    /// 归档页是否渲染空态：已下发且为空集才算空态；
    /// 未下发 ≠ 空集（三态契约），走 fail-closed 提示而不是空态。
    pub fn archived_page_empty(&self) -> bool {
        self.state == State::Normal && self.archived_tasks.is_empty()
    }
}

/// 操作行：取消 / 确认归档（任务进行中降为次级配色，颜色一律取主题 token）。
fn button_row(theme: &Theme, overlay: &ArchiveOverlay) -> Div {
    let confirm = if overlay.confirm_enabled() {
        div()
            .bg(theme.button_primary)
            .text_color(theme.button_primary_foreground)
    } else {
        div()
            .bg(theme.button_secondary)
            .text_color(theme.button_secondary_foreground)
    };
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(
            div()
                .px_3()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .text_color(theme.foreground)
                .child(copy::DIALOG_CANCEL),
        )
        .child(
            confirm
                .px_3()
                .py_1()
                .rounded_md()
                .child(copy::DIALOG_CONFIRM),
        )
}

/// 归档页区：标题 + 状态化条目区（未下发/拉取中不宣称空集，⛔ 不伪造条目）。
fn page_section(theme: &Theme, overlay: &ArchiveOverlay) -> Div {
    let mut section = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_color(theme.foreground).child(copy::PAGE_TITLE));
    match overlay.state {
        // 数据未下发：fail-closed 提示，⛔ 不得替换为空列表或空态。
        State::NotDelivered => {
            section = section.child(
                div()
                    .text_color(theme.muted_foreground)
                    .child(NOT_DELIVERED),
            );
        }
        // 拉取中：不渲染条目，也不宣称空集。
        State::Loading => {}
        State::Failed => {
            section = section.child(
                div()
                    .text_color(theme.danger_foreground)
                    .child(copy::TOAST_FAILED),
            );
        }
        State::Normal if overlay.archived_tasks.is_empty() => {
            section = section.child(
                div()
                    .text_color(theme.muted_foreground)
                    .child(copy::PAGE_EMPTY),
            );
        }
        State::Normal => {
            for name in &overlay.archived_tasks {
                section =
                    section.child(div().text_color(theme.muted_foreground).child(name.clone()));
            }
            section = section.child(
                div()
                    .text_color(theme.accent_foreground)
                    .child(copy::UNARCHIVE),
            );
        }
    }
    section
}

/// 按纯数据模型渲染浮层（标题行 + 正文 + 操作行 + 归档页区）。
fn render(theme: &Theme, overlay: &ArchiveOverlay) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(copy::DIALOG_TITLE))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(overlay.dialog_content()),
        )
        .child(button_row(theme, overlay))
        .child(page_section(theme, overlay))
}

/// 主区渲染（签名固定；数据源未接入前用诚实默认态，fail-closed）。
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &ArchiveOverlay::default())
}

#[cfg(test)]
mod tests {
    use super::copy;
    use super::{main_area, ArchiveOverlay, State, NOT_DELIVERED};
    use gpui_kit::component::Theme;

    /// ① 纯函数层：默认态（fail-closed）与计数。
    #[test]
    fn default_state_is_not_delivered() {
        let overlay = ArchiveOverlay::default();
        assert_eq!(overlay.state, State::NotDelivered);
        assert_eq!(overlay.status_notice(), Some(NOT_DELIVERED));
        assert_eq!(overlay.archived_count(), 0);
    }

    /// ① 纯函数层：计数随输入变化。
    #[test]
    fn archived_count_tracks_input() {
        let mut overlay = ArchiveOverlay {
            state: State::Normal,
            archived_tasks: vec!["任务甲".into(), "任务乙".into()],
            ..ArchiveOverlay::default()
        };
        assert_eq!(overlay.archived_count(), 2);
        overlay.archived_tasks.clear();
        assert_eq!(overlay.archived_count(), 0);
    }

    /// ② 空输入 ⇒ 空态（不是空列表；未下发 ≠ 空态）。
    #[test]
    fn empty_input_yields_empty_state_not_list() {
        let delivered_empty = ArchiveOverlay {
            state: State::Normal,
            ..ArchiveOverlay::default()
        };
        assert!(delivered_empty.archived_page_empty());
        // 未拉取过 ≠ 空集：默认态不宣称空态。
        assert!(!ArchiveOverlay::default().archived_page_empty());
    }

    /// ③ 文案逐字一致：全 17 键逐一断言（含两处空格边界键）。
    #[test]
    fn copy_verbatim_all_17_keys() {
        assert_eq!(copy::DIALOG_CANCEL, "取消");
        assert_eq!(copy::DIALOG_CONFIRM, "确认归档");
        assert_eq!(
            copy::DIALOG_CONTENT,
            "确认将该任务归档吗？归档后可在「系统设置-数据管理」中查看已归档任务。"
        );
        assert_eq!(copy::DIALOG_CONTENT_CLOUD, "确认将该任务归档吗？");
        assert_eq!(copy::DIALOG_TITLE, "归档任务");
        assert_eq!(copy::DISABLED_REASON, "任务进行中，无法归档");
        assert_eq!(copy::INLINE_CONFIRM, "确认");
        assert_eq!(copy::PAGE_EMPTY, "暂无归档内容");
        assert_eq!(copy::PAGE_TITLE, "已归档任务");
        assert_eq!(copy::TOAST_FAILED, "归档失败，请重试");
        assert_eq!(copy::TOAST_SUCCESS, "任务已归档，可在 ");
        assert_eq!(copy::TOAST_SUCCESS_CLOUD, "任务已归档");
        assert_eq!(copy::TOAST_SUCCESS_LINK, "设置");
        assert_eq!(copy::TOAST_SUCCESS_SUFFIX, " 中查看");
        assert_eq!(copy::UNARCHIVE, "取消归档");
        assert_eq!(copy::UNARCHIVE_FAILED, "恢复失败，请重试");
        assert_eq!(copy::UNARCHIVE_SUCCESS, "任务已恢复");
        assert_eq!(copy::ALL.len(), 17);
    }

    /// ③ 成功 toast 三段拼装（尾随/前导空格落在正确位置）。
    #[test]
    fn toast_success_assembles_link_parts() {
        let local = ArchiveOverlay::default();
        assert_eq!(local.toast_success(), "任务已归档，可在 设置 中查看");
        let cloud = ArchiveOverlay {
            cloud: true,
            ..ArchiveOverlay::default()
        };
        assert_eq!(cloud.toast_success(), "任务已归档");
    }

    /// ④ `main_area(&Theme::default())` 不 panic。
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }

    /// ⑤ 状态分支 Normal：正文三态 + 确认按钮禁用。
    #[test]
    fn branch_normal_dialog_content_and_disable() {
        let mut overlay = ArchiveOverlay {
            state: State::Normal,
            ..ArchiveOverlay::default()
        };
        assert_eq!(overlay.dialog_content(), copy::DIALOG_CONTENT);
        assert!(overlay.confirm_enabled());
        overlay.cloud = true;
        assert_eq!(overlay.dialog_content(), copy::DIALOG_CONTENT_CLOUD);
        overlay.task_running = true;
        assert_eq!(overlay.dialog_content(), copy::DISABLED_REASON);
        assert!(!overlay.confirm_enabled());
    }

    /// ⑤ 状态分支 NotDelivered：提示「未下发」。
    #[test]
    fn branch_not_delivered_shows_notice() {
        assert_eq!(
            ArchiveOverlay::default().status_notice(),
            Some(NOT_DELIVERED)
        );
    }

    /// ⑤ 状态分支 Failed：提示失败文案。
    #[test]
    fn branch_failed_shows_toast_failed() {
        let overlay = ArchiveOverlay {
            state: State::Failed,
            ..ArchiveOverlay::default()
        };
        assert_eq!(overlay.status_notice(), Some(copy::TOAST_FAILED));
        assert!(!overlay.archived_page_empty());
    }

    /// ⑤ 状态分支 Loading：不提示、不宣称空态。
    #[test]
    fn branch_loading_no_notice_no_empty_claim() {
        let overlay = ArchiveOverlay {
            state: State::Loading,
            ..ArchiveOverlay::default()
        };
        assert_eq!(overlay.status_notice(), None);
        assert!(!overlay.archived_page_empty());
    }
}
