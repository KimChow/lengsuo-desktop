//! 文件版本浮层（波次 7，卡 `ui-overlay-file-version`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `fileVersion`。
//! 数据源在运行时侧：本模块当前按「未下发」如实渲染，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字：`fileVersion.title`）。
pub const TITLE: &str = text::TITLE;
/// 数据未下发时的说明（仓内 fail-closed 约定文案；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 界面文案（逐字取自文案全表 `fileVersion` 命名空间；⛔ 不自编、不含品牌词）。
pub mod text {
    /// `fileVersion.binaryNotSupported`
    pub const BINARY_NOT_SUPPORTED: &str = "此文件为二进制，不支持 diff 查看";
    /// `fileVersion.cancel`
    pub const CANCEL: &str = "取消";
    /// `fileVersion.changeType.created`
    pub const CHANGE_TYPE_CREATED: &str = "新建";
    /// `fileVersion.changeType.deleted`
    pub const CHANGE_TYPE_DELETED: &str = "删除";
    /// `fileVersion.changeType.modified`
    pub const CHANGE_TYPE_MODIFIED: &str = "修改";
    /// `fileVersion.changeType.renamed`
    pub const CHANGE_TYPE_RENAMED: &str = "重命名";
    /// `fileVersion.confirmReset`
    pub const CONFIRM_RESET: &str = "确认重置";
    /// `fileVersion.detailEmpty`
    pub const DETAIL_EMPTY: &str = "选择左侧任一版本以查看文件明细";
    /// `fileVersion.diffLoading`
    pub const DIFF_LOADING: &str = "加载 diff...";
    /// `fileVersion.empty`
    pub const EMPTY: &str = "暂无版本记录";
    /// `fileVersion.filesChanged`（模板）
    pub const FILES_CHANGED: &str = "{count} 个文件变更";
    /// `fileVersion.headBadge`
    pub const HEAD_BADGE: &str = "当前版本";
    /// `fileVersion.loading`
    pub const LOADING: &str = "加载中...";
    /// `fileVersion.opFailed`（模板）
    pub const OP_FAILED: &str = "操作失败：{message}";
    /// `fileVersion.oversize`（模板）
    pub const OVERSIZE: &str = "文件过大（{size}），不展示 diff";
    /// `fileVersion.refresh`
    pub const REFRESH: &str = "刷新";
    /// `fileVersion.reset`
    pub const RESET: &str = "重置到此版本";
    /// `fileVersion.resetConfirmBody`
    pub const RESET_CONFIRM_BODY: &str = "将丢弃该 commit 之后的全部变更，且不可恢复。";
    /// `fileVersion.resetConfirmTitle`
    pub const RESET_CONFIRM_TITLE: &str = "重置到此版本？";
    /// `fileVersion.resetSuccess`
    pub const RESET_SUCCESS: &str = "已重置到所选版本";
    /// `fileVersion.resetTip`
    pub const RESET_TIP: &str =
        "把当前文件状态恢复到这个版本，并丢弃此版本之后的全部提交，不可恢复。";
    /// `fileVersion.revert`
    pub const REVERT: &str = "回滚到此版本";
    /// `fileVersion.revertSuccess`
    pub const REVERT_SUCCESS: &str = "回滚成功";
    /// `fileVersion.revertTip`
    pub const REVERT_TIP: &str =
        "把当前文件状态恢复到这个版本，保留此版本之后的所有提交记录（可再次回滚）。";
    /// `fileVersion.title`
    pub const TITLE: &str = "文件版本管理";
    /// `fileVersion.unknownError`
    pub const UNKNOWN_ERROR: &str = "未知错误";
    /// `fileVersion.versionMissing`
    pub const VERSION_MISSING: &str = "原始版本已被回收，无法查看 diff";

    /// `fileVersion.filesChanged`：替换 `{count}`。
    #[must_use]
    pub fn files_changed(count: u32) -> String {
        let n = count.to_string();
        FILES_CHANGED.replace("{count}", &n)
    }

    /// `fileVersion.opFailed`：替换 `{message}`。
    #[must_use]
    pub fn op_failed(message: &str) -> String {
        OP_FAILED.replace("{message}", message)
    }

    /// `fileVersion.oversize`：替换 `{size}`。
    #[must_use]
    pub fn oversize(size: &str) -> String {
        OVERSIZE.replace("{size}", size)
    }
}

// ---------------------------------------------------------------------------
// 纯数据层（无 GPUI 依赖；视图是它的纯函数）
// ---------------------------------------------------------------------------

/// 浮层整体状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayState {
    /// 版本记录加载中（`loading`）。
    Loading,
    /// 已拿到数据（可能是空集合 ⇒ 空态文案）。
    Normal,
    /// 操作/加载失败（`opFailed`，缺省消息 `unknownError`）。
    Failed,
    /// 数据未下发（fail-closed，如实显示）。
    NotDelivered,
}

/// 单个版本的变更类型（`changeType.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeType {
    Created,
    Deleted,
    Modified,
    Renamed,
}

impl ChangeType {
    /// 变更类型文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Created => text::CHANGE_TYPE_CREATED,
            Self::Deleted => text::CHANGE_TYPE_DELETED,
            Self::Modified => text::CHANGE_TYPE_MODIFIED,
            Self::Renamed => text::CHANGE_TYPE_RENAMED,
        }
    }
}

/// 时间线上的一个版本条目。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionEntry {
    pub change_type: ChangeType,
    /// 该版本涉及的文件数（渲染为 `filesChanged` 模板）。
    pub files_changed: u32,
    /// 是否当前版本（渲染 `headBadge`）。
    pub is_head: bool,
}

/// 右侧明细/对比面板状态（各态均为如实占位，⛔ 不伪造 diff 内容）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffState {
    /// 未选中任何版本（`detailEmpty`）。
    NotSelected,
    /// diff 加载中（`diffLoading`）。
    Loading,
    /// 二进制不支持 diff（`binaryNotSupported`）。
    Binary,
    /// 文件过大（`oversize`，携带原样 size 文案）。
    Oversize(String),
    /// 原始版本已被回收（`versionMissing`）。
    VersionMissing,
}

/// 最近一次恢复动作的结果反馈。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback {
    None,
    /// 回滚成功（`revertSuccess`）。
    RevertDone,
    /// 重置成功（`resetSuccess`）。
    ResetDone,
}

impl Feedback {
    /// 反馈文案；`None` 表示无反馈。
    #[must_use]
    pub fn label(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::RevertDone => Some(text::REVERT_SUCCESS),
            Self::ResetDone => Some(text::RESET_SUCCESS),
        }
    }
}

/// 浮层纯数据模型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileVersionOverlay {
    pub state: OverlayState,
    /// 版本时间线（新旧顺序由运行时侧保证）。
    pub versions: Vec<VersionEntry>,
    /// 选中的版本下标。
    pub selected: Option<usize>,
    /// 明细面板状态。
    pub diff: DiffState,
    /// 待确认的重置目标下标（`Some` ⇒ 确认面板开启）。
    pub pending_reset: Option<usize>,
    /// 失败消息；`None` ⇒ 显示 `unknownError`。
    pub failure_message: Option<String>,
    /// 结果反馈条。
    pub feedback: Feedback,
}

impl FileVersionOverlay {
    fn base(state: OverlayState) -> Self {
        Self {
            state,
            versions: Vec::new(),
            selected: None,
            diff: DiffState::NotSelected,
            pending_reset: None,
            failure_message: None,
            feedback: Feedback::None,
        }
    }

    /// 数据未下发（fail-closed；⛔ 不许用空列表冒充）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self::base(OverlayState::NotDelivered)
    }

    /// 加载中。
    #[must_use]
    pub fn loading() -> Self {
        Self::base(OverlayState::Loading)
    }

    /// 失败；`message` 为 `None` 时回落 `unknownError`。
    #[must_use]
    pub fn failed(message: Option<&str>) -> Self {
        Self {
            failure_message: message.map(str::to_owned),
            ..Self::base(OverlayState::Failed)
        }
    }

    /// 空态判定：正常态 + 无版本记录 ⇒ 显示 `empty` 文案，不是空列表。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.state == OverlayState::Normal && self.versions.is_empty()
    }

    /// 当前选中的版本条目。
    #[must_use]
    pub fn selected_entry(&self) -> Option<&VersionEntry> {
        self.selected.and_then(|idx| self.versions.get(idx))
    }

    /// 失败文案（`opFailed` 模板，缺省消息 `unknownError`）。
    #[must_use]
    pub fn failure_label(&self) -> String {
        text::op_failed(
            self.failure_message
                .as_deref()
                .unwrap_or(text::UNKNOWN_ERROR),
        )
    }

    /// 明细面板占位文案（按 `DiffState` 逐字取表）。
    #[must_use]
    pub fn diff_note(&self) -> String {
        match &self.diff {
            DiffState::NotSelected => text::DETAIL_EMPTY.to_owned(),
            DiffState::Loading => text::DIFF_LOADING.to_owned(),
            DiffState::Binary => text::BINARY_NOT_SUPPORTED.to_owned(),
            DiffState::Oversize(size) => text::oversize(size),
            DiffState::VersionMissing => text::VERSION_MISSING.to_owned(),
        }
    }
}

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
///
/// 无数据源 ⇒ 按未下发如实渲染（fail-closed）；数据源就绪后由运行时侧喂真实状态。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&FileVersionOverlay::not_delivered(), theme)
}

/// 按纯数据层渲染整层：标题行 + 状态分支（未下发/加载/失败/空态/双栏主体）。
fn render(overlay: &FileVersionOverlay, theme: &Theme) -> Div {
    let shell = div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col();
    let body = match overlay.state {
        OverlayState::NotDelivered => centered_note(theme, NOT_DELIVERED),
        OverlayState::Loading => centered_note(theme, text::LOADING),
        OverlayState::Failed => centered_note(theme, &overlay.failure_label()),
        OverlayState::Normal if overlay.is_empty() => centered_note(theme, text::EMPTY),
        OverlayState::Normal => div()
            .flex_1()
            .flex()
            .min_h_0()
            .child(timeline(overlay, theme))
            .child(detail(overlay, theme)),
    };
    shell.child(header_row(theme)).child(body)
}

/// 居中提示（未下发/加载中/失败/空态共用）。
fn centered_note(theme: &Theme, note: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(note.to_owned())
}

/// 标题行：标题 + 刷新动作。
fn header_row(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(pill(text::REFRESH, false, theme))
}

/// 左栏：版本时间线（清单行）。
fn timeline(overlay: &FileVersionOverlay, theme: &Theme) -> Div {
    let rows = overlay.versions.iter().enumerate().fold(
        div().flex().flex_col().gap_1().p_2(),
        |list, (idx, entry)| list.child(version_row(entry, overlay.selected == Some(idx), theme)),
    );
    div()
        .w(px(288.))
        .h_full()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(theme.border)
        .child(rows)
}

/// 单条版本行：「当前版本」徽标 + 变更类型 + 文件变更数。
fn version_row(entry: &VersionEntry, selected: bool, theme: &Theme) -> Div {
    let mut first_line = div().flex().items_center().gap_1();
    if entry.is_head {
        first_line = first_line.child(head_badge(theme));
    }
    first_line = first_line.child(
        div()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(entry.change_type.label()),
    );
    let mut row = div().flex().flex_col().gap_1().px_2().py_1().rounded_sm();
    if selected {
        row = row.bg(theme.muted);
    } else {
        row = row.border_1().border_color(theme.border);
    }
    row.child(first_line).child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(text::files_changed(entry.files_changed)),
    )
}

/// 「当前版本」徽标。
fn head_badge(theme: &Theme) -> Div {
    div()
        .px_1()
        .rounded_sm()
        .bg(theme.accent)
        .text_xs()
        .text_color(theme.accent_foreground)
        .child(text::HEAD_BADGE)
}

/// 右栏：结果反馈 + 明细/对比占位 + 恢复动作 + 重置确认面板。
fn detail(overlay: &FileVersionOverlay, theme: &Theme) -> Div {
    let mut pane = div().flex_1().flex().flex_col().gap_2().p_3().min_w_0();
    if let Some(label) = overlay.feedback.label() {
        pane = pane.child(div().text_sm().text_color(theme.success).child(label));
    }
    pane = pane.child(
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(overlay.diff_note()),
    );
    if overlay.selected_entry().is_some() {
        pane = pane.child(actions(theme));
    }
    if overlay.pending_reset.is_some() {
        pane = pane.child(reset_confirm(theme));
    }
    pane
}

/// 恢复动作组：回滚（保留后续提交）/ 重置（丢弃后续提交）。
fn actions(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(action(text::REVERT, text::REVERT_TIP, false, theme))
        .child(action(text::RESET, text::RESET_TIP, true, theme))
}

/// 单个动作：按钮 + 说明。
fn action(label: &'static str, tip: &'static str, danger: bool, theme: &Theme) -> Div {
    let mut button = div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(160.))
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm();
    if danger {
        button = button
            .bg(theme.button_danger)
            .text_color(theme.button_danger_foreground);
    } else {
        button = button.bg(theme.accent).text_color(theme.accent_foreground);
    }
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(button.child(label))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(tip),
        )
}

/// 「重置到此版本」确认面板：标题 + 后果说明 + 确认/取消。
fn reset_confirm(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(text::RESET_CONFIRM_TITLE),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(text::RESET_CONFIRM_BODY),
        )
        .child(
            div()
                .flex()
                .gap_2()
                .child(pill(text::CONFIRM_RESET, true, theme))
                .child(pill(text::CANCEL, false, theme)),
        )
}

/// 小按钮（刷新/确认/取消）：纯文案胶囊。
fn pill(label: &'static str, danger: bool, theme: &Theme) -> Div {
    let mut p = div()
        .px_3()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_sm();
    if danger {
        p = p.text_color(theme.danger);
    } else {
        p = p.text_color(theme.muted_foreground);
    }
    p.child(label)
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
