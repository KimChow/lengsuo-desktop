//! 任务发起浮层（波次 6，卡 `ui-overlay-task-starter`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `taskStarter`。
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ============================================================
// 纯数据层（无 GPUI 依赖；视图是它的纯函数）
// ============================================================

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "任务发起";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 未授权代码库时的提示（`taskStarter.noConnectorHint`，逐字）。
pub const HINT_NO_CONNECTOR: &str =
    "尚未授权代码库权限，如需从您的代码库创建工作空间，请先进行授权";
/// 仓库为空且需单独授权时的提示（`taskStarter.githubNoRepos`，逐字）。
pub const HINT_NO_REPOS: &str = "暂无仓库，Github 需要单独为仓库授权，请点击添加仓库。";
/// 新建工作空间失败的 toast 文案（`taskStarter.createWorkspace.toast.failed`，逐字）。
pub const TOAST_CREATE_FAILED: &str = "新建工作空间失败，请重试";
/// 新建工作空间说明（`taskStarter.createWorkspaceDesc`，逐字）。
pub const CREATE_WORKSPACE_DESC: &str =
    "为工作空间命名，本地将自动创建同名文件夹，命名后不可随意更改";
/// 新建工作空间输入占位（`taskStarter.createWorkspacePlaceholder`，逐字）。
pub const CREATE_WORKSPACE_PLACEHOLDER: &str = "输入工作空间名称";
/// 工作空间存储路径提示前缀（`taskStarter.workspacePathHint.prefix`，逐字）。
pub const PATH_HINT_PREFIX: &str = "可前往";
/// 工作空间存储路径提示中的设置入口（`taskStarter.workspacePathHint.settingsLink`，逐字）。
pub const PATH_HINT_SETTINGS_LINK: &str = "「系统设置」";
/// 工作空间存储路径提示后缀（`taskStarter.workspacePathHint.suffix`，逐字）。
pub const PATH_HINT_SUFFIX: &str = "设置默认工作空间存储路径";
/// 工作空间清单的固定行（`taskStarter.noWorkspace`，逐字）。
pub const NO_WORKSPACE_ROW: &str = "不使用工作空间";
/// 页脚标签（`taskStarter.startIn`，逐字；数值未下发，⛔ 不伪造）。
pub const FOOTER_START_IN: &str = "任务启动于：";

/// 浮层状态机的可枚举状态（fail-closed：数据未下发时停在 [`StarterState::NotDelivered`]）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StarterState {
    /// 数据未下发（默认；如实显示「未下发」，⛔ 不得伪造清单）。
    #[default]
    NotDelivered,
    /// 未授权代码库（授权提示 + 「添加仓库授权」）。
    NoConnector,
    /// 仓库清单加载中。
    LoadingRepos,
    /// 仓库清单为空（需单独授权后添加）。
    NoRepos,
    /// 分支清单加载中。
    LoadingBranches,
    /// 分支清单为空。
    NoBranches,
    /// 工作空间清单为空。
    NoWorkspaceFound,
    /// 通用清单空态。
    NoData,
    /// 清单就绪（仓库/工作空间均已下发且非空）。
    Ready,
    /// 新建工作空间失败（toast）。
    CreateFailed,
}

impl StarterState {
    /// 状态主文案（逐字取自 `taskStarter` 命名空间；就绪态无文案，改渲染清单行）。
    #[must_use]
    pub fn status_text(self) -> Option<&'static str> {
        match self {
            Self::NotDelivered => Some(NOT_DELIVERED),
            Self::NoConnector => Some(HINT_NO_CONNECTOR),
            Self::LoadingRepos => Some("加载仓库中..."),
            Self::NoRepos => Some(HINT_NO_REPOS),
            Self::LoadingBranches => Some("加载分支中..."),
            Self::NoBranches => Some("未找到分支"),
            Self::NoWorkspaceFound => Some("未找到工作空间"),
            Self::NoData => Some("暂无数据"),
            Self::Ready => None,
            Self::CreateFailed => Some(TOAST_CREATE_FAILED),
        }
    }

    /// 状态主操作按钮文案（无操作的状态返回 `None`；逐字取自命名空间）。
    #[must_use]
    pub fn primary_action(self) -> Option<&'static str> {
        match self {
            Self::NoConnector => Some("添加仓库授权"),
            Self::NoRepos => Some("添加仓库"),
            Self::NoWorkspaceFound | Self::CreateFailed => Some("新建工作空间"),
            Self::NotDelivered
            | Self::LoadingRepos
            | Self::LoadingBranches
            | Self::NoBranches
            | Self::NoData
            | Self::Ready => None,
        }
    }
}

/// 选择区静态文案（区标签 + 搜索占位；逐字取自 `taskStarter` 命名空间）。
pub struct SectionCopy {
    /// 区标签。
    pub label: &'static str,
    /// 搜索占位。
    pub search: &'static str,
}

/// 仓库选择区（`taskStarter.selectRepo` / `taskStarter.searchRepo`）。
pub const SECTION_REPO: SectionCopy = SectionCopy {
    label: "选择仓库",
    search: "搜索仓库",
};
/// 工作空间选择区（`taskStarter.selectWorkspace` / `taskStarter.searchWorkspace`）。
pub const SECTION_WORKSPACE: SectionCopy = SectionCopy {
    label: "选择工作空间",
    search: "搜索工作空间",
};

/// 拼接工作空间存储路径提示（prefix + settingsLink + suffix，逐字）。
#[must_use]
pub fn workspace_path_hint() -> String {
    format!("{PATH_HINT_PREFIX}{PATH_HINT_SETTINGS_LINK}{PATH_HINT_SUFFIX}")
}

/// 浮层纯数据模型（无 GPUI 依赖；视图是它的纯函数）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskStarterModel {
    /// 当前状态。
    pub state: StarterState,
    /// 已下发的仓库名（未下发时为空；⛔ 不伪造）。
    pub repos: Vec<String>,
    /// 已下发的工作空间名（未下发时为空；⛔ 不伪造）。
    pub workspaces: Vec<String>,
}

impl TaskStarterModel {
    /// 未下发时的诚实模型（fail-closed 默认）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self::default()
    }

    /// 由下发的授权位与清单推导状态：空输入 ⇒ 空态（⛔ 不是空列表）。
    /// 授权位优先：未授权时即便有清单也停在授权提示。
    #[must_use]
    pub fn from_delivered(authorized: bool, repos: &[String], workspaces: &[String]) -> Self {
        let state = if !authorized {
            StarterState::NoConnector
        } else if repos.is_empty() {
            StarterState::NoRepos
        } else if workspaces.is_empty() {
            StarterState::NoWorkspaceFound
        } else {
            StarterState::Ready
        };
        Self {
            state,
            repos: repos.to_vec(),
            workspaces: workspaces.to_vec(),
        }
    }

    /// toast 文案（仅新建工作空间失败时有，其余 `None`）。
    #[must_use]
    pub fn toast_text(&self) -> Option<&'static str> {
        match self.state {
            StarterState::CreateFailed => Some(TOAST_CREATE_FAILED),
            _ => None,
        }
    }

    /// 当前可渲染的清单行数（未下发/未授权 ⇒ 0；⛔ 不伪造）。
    #[must_use]
    pub fn visible_row_count(&self) -> usize {
        match self.state {
            StarterState::NotDelivered | StarterState::NoConnector => 0,
            _ => self.repos.len() + self.workspaces.len(),
        }
    }
}

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
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
