//! 上下文菜单浮层（波次 7，卡 `ui-overlay-context-menu`，对齐 ui-parity-spec §9 A43）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `contextMenu`
//! （空态一行取自文案全表通用键 `detailPanel.noContent`，命名空间内无空态键）。
//! 数据源在运行时侧：[`ContextMenuState::initial`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "上下文菜单";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 菜单快照为空时的提示（命名空间内无空态键，取自文案全表 `detailPanel.noContent`）。
pub const EMPTY_HINT: &str = "暂无内容";

/// 菜单项文案（逐字取自 `contextMenu` 命名空间）。
pub const OPEN_FILE: &str = "打开";
pub const OPEN_FOLDER: &str = "打开文件夹";
pub const ADD_TO_CHAT: &str = "添加到对话框";
pub const SHARE_FILE: &str = "分享给好友";
pub const SHARE_TASK: &str = "分享任务";
pub const SAVE_AS: &str = "另存为";
pub const RENAME: &str = "重命名";
pub const SHOW_SOURCE: &str = "显示源码";
pub const VIEW_TASK_DETAIL: &str = "查看任务详情";
pub const BATCH_OPERATIONS: &str = "批量操作";
pub const DELETE_TASK: &str = "删除任务";
pub const REMOVE_FROM_LIST: &str = "从列表中移除";
pub const MORE: &str = "更多";

/// 打开/另存为失败错误行文案（逐字取自 `contextMenu` 命名空间 `*.failed` 键）。
pub const OPEN_FILE_FAILED: &str = "打开失败，请检查该文件类型是否已关联默认应用";
pub const OPEN_FOLDER_FAILED: &str = "无法打开文件夹，该文件夹可能已被重命名或删除";
pub const SAVE_AS_FAILED: &str = "另存为失败，请重试";

/// 菜单项（纯数据，对齐 A43：打开/打开文件夹/批量操作/删除任务/更多 等）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    OpenFile,
    OpenFolder,
    AddToChat,
    ShareFile,
    ShareTask,
    SaveAs,
    Rename,
    ShowSource,
    ViewTaskDetail,
    BatchOperations,
    DeleteTask,
    RemoveFromList,
    More,
}

impl MenuItem {
    /// 全部菜单项（渲染顺序即枚举顺序）。
    pub const ALL: [Self; 13] = [
        Self::OpenFile,
        Self::OpenFolder,
        Self::AddToChat,
        Self::ShareFile,
        Self::ShareTask,
        Self::SaveAs,
        Self::Rename,
        Self::ShowSource,
        Self::ViewTaskDetail,
        Self::BatchOperations,
        Self::DeleteTask,
        Self::RemoveFromList,
        Self::More,
    ];

    /// 菜单项文案。
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenFile => OPEN_FILE,
            Self::OpenFolder => OPEN_FOLDER,
            Self::AddToChat => ADD_TO_CHAT,
            Self::ShareFile => SHARE_FILE,
            Self::ShareTask => SHARE_TASK,
            Self::SaveAs => SAVE_AS,
            Self::Rename => RENAME,
            Self::ShowSource => SHOW_SOURCE,
            Self::ViewTaskDetail => VIEW_TASK_DETAIL,
            Self::BatchOperations => BATCH_OPERATIONS,
            Self::DeleteTask => DELETE_TASK,
            Self::RemoveFromList => REMOVE_FROM_LIST,
            Self::More => MORE,
        }
    }
}

/// 打开/另存为失败错误行（纯数据）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuError {
    OpenFile,
    OpenFolder,
    SaveAs,
}

impl MenuError {
    /// 全部错误行变体。
    pub const ALL: [Self; 3] = [Self::OpenFile, Self::OpenFolder, Self::SaveAs];

    /// 错误行文案。
    pub fn text(self) -> &'static str {
        match self {
            Self::OpenFile => OPEN_FILE_FAILED,
            Self::OpenFolder => OPEN_FOLDER_FAILED,
            Self::SaveAs => SAVE_AS_FAILED,
        }
    }
}

/// 菜单快照（纯数据，Ⓐ 不持凭据、不判权限）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuModel {
    /// 菜单项（按展示顺序）。
    pub items: Vec<MenuItem>,
    /// 打开/另存为失败时追加在菜单尾部的错误行。
    pub error: Option<MenuError>,
}

impl MenuModel {
    /// 由菜单项构造（无错误行）。
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self { items, error: None }
    }

    /// 空快照（⇒ 空态提示，不是空白面板）。
    pub fn empty() -> Self {
        Self { items: Vec::new(), error: None }
    }

    /// 是否一项都没有（空输入 ⇒ 空态）。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 附加错误行（链式）。
    pub fn with_error(mut self, error: MenuError) -> Self {
        self.error = Some(error);
        self
    }
}

/// 浮层可枚举状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextMenuState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空集）。
    NotDelivered,
    /// 数据已下发：带菜单快照的正常态。
    Ready(MenuModel),
}

impl ContextMenuState {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self::NotDelivered
    }

    /// 是否处于未下发态。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self, Self::NotDelivered)
    }

    /// 中央提示行：未下发 / 空态；有菜单项时为空串（不显示提示行）。
    pub fn hint_line(&self) -> &'static str {
        match self {
            Self::NotDelivered => NOT_DELIVERED,
            Self::Ready(model) if model.is_empty() => EMPTY_HINT,
            Self::Ready(_) => "",
        }
    }

    /// 错误行文案：未下发时恒为 `None`（⛔ 不伪造）。
    pub fn error_line(&self) -> Option<&'static str> {
        match self {
            Self::NotDelivered => None,
            Self::Ready(model) => model.error.map(MenuError::text),
        }
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
/// 恒以 [`ContextMenuState::initial`]（未下发）渲染 —— fail-closed。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &ContextMenuState::initial())
}

/// 按状态渲染整层：标题行 + 中央区（未下发 / 空态 / 菜单面板）。
fn render(theme: &Theme, state: &ContextMenuState) -> Div {
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

/// 中央区：未下发 / 空态走居中提示行；有菜单项走面板。
fn body(theme: &Theme, state: &ContextMenuState) -> Div {
    match state {
        ContextMenuState::NotDelivered => hint_panel(theme, NOT_DELIVERED),
        ContextMenuState::Ready(model) if model.is_empty() => hint_panel(theme, EMPTY_HINT),
        ContextMenuState::Ready(model) => menu_panel(theme, model),
    }
}

/// 居中提示行（未下发 / 空态共用，弱色）。
fn hint_panel(theme: &Theme, text: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 菜单面板：菜单项行 + 尾部错误行（⛔ 未下发不伪造条目/数值）。
fn menu_panel(theme: &Theme, model: &MenuModel) -> Div {
    let mut panel = div()
        .flex()
        .flex_col()
        .m_4()
        .w_64()
        .rounded_md()
        .border_1()
        .border_color(theme.border);
    for (idx, item) in model.items.iter().enumerate() {
        let mut row = div().px_4().py_2().text_sm().text_color(theme.foreground);
        if idx > 0 {
            row = row.border_t_1().border_color(theme.border);
        }
        panel = panel.child(row.child(item.label()));
    }
    if let Some(error) = model.error {
        panel = panel.child(
            div()
                .px_4()
                .py_2()
                .text_xs()
                .border_t_1()
                .border_color(theme.border)
                .text_color(theme.muted_foreground)
                .child(error.text()),
        );
    }
    panel
}
#[cfg(test)]
mod tests {
    use super::{main_area, ContextMenuState, MenuModel, NOT_DELIVERED, TITLE};
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
    #[test]
    fn initial_state_is_fail_closed_not_delivered() {
        let state = ContextMenuState::initial();
        assert!(state.is_not_delivered());
        assert_eq!(state.hint_line(), NOT_DELIVERED);
        assert_eq!(state.error_line(), None);
    }
    #[test]
    fn empty_input_is_empty_state_not_blank() {
        let state = ContextMenuState::Ready(MenuModel::empty());
        assert_eq!(state.hint_line(), "暂无内容");
        assert!(!state.hint_line().is_empty());
    }
}
