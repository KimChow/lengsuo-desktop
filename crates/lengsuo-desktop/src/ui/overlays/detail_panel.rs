//! 详情面板浮层（波次 7，卡 `ui-overlay-detail-panel`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（无 GPUI 依赖）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed —— 无数据源时如实显示「未下发」，⛔ 不伪造条目/数值；
//! 文案逐字取自界面文案全表命名空间 `detailPanel`（品牌词键已剔除）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ── 纯数据层（无 GPUI 依赖）────────────────────────────────────────────

/// detailPanel.overview
pub const OVERVIEW: &str = "概览";
/// detailPanel.review
pub const REVIEW: &str = "审查";
/// detailPanel.follow
pub const FOLLOW: &str = "关注";
/// detailPanel.hideSidebar
pub const HIDE_SIDEBAR: &str = "隐藏侧边栏";
/// detailPanel.showSidebar
pub const SHOW_SIDEBAR: &str = "显示侧边栏";
/// detailPanel.newTab
pub const NEW_TAB: &str = "新标签页";
/// detailPanel.newTab.openBrowser
pub const OPEN_BROWSER: &str = "打开浏览器";
/// detailPanel.newTab.openTerminal
pub const OPEN_TERMINAL: &str = "打开终端";
/// detailPanel.newTab.openWorkspaceFiles
pub const VIEW_WORKSPACE: &str = "查看工作空间";
/// detailPanel.newTab.viewChanges
pub const VIEW_CHANGES: &str = "查看变更";
/// detailPanel.artifacts
pub const ARTIFACTS: &str = "产物";
/// detailPanel.files
pub const FILES: &str = "文件";
/// detailPanel.taskProgress
pub const TASK_PROGRESS: &str = "任务进程";
/// detailPanel.design.newCanvas
pub const NEW_CANVAS: &str = "新画布";
/// detailPanel.noContent
pub const NO_CONTENT: &str = "暂无内容";
/// detailPanel.noDetails
pub const NO_DETAILS: &str = "暂无详情";
/// detailPanel.selectArtifact
pub const SELECT_ARTIFACT: &str = "请选择一个产物查看详情";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空清单或空态文案）。
pub const NOT_DELIVERED: &str = "未下发";

/// 详情面板浮层状态（可枚举；决定内容体分支）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelState {
    /// 已下发且有条目 ⇒ 分节渲染（仅真实计数）。
    Normal,
    /// 已下发但无条目 ⇒ 空态文案（不是空清单）。
    Empty,
    /// 未下发 ⇒ fail-closed，如实显示「未下发」。
    NotDelivered,
}

/// 内容分节（产物 / 文件 / 任务进程）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// 产物节。
    Artifacts,
    /// 文件节。
    Files,
    /// 任务进程节。
    TaskProgress,
}

impl Section {
    /// 节标题（逐字取自文案全表）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Artifacts => ARTIFACTS,
            Self::Files => FILES,
            Self::TaskProgress => TASK_PROGRESS,
        }
    }

    /// 空态文案（空输入 ⇒ 空态，不是空清单）。
    #[must_use]
    pub fn empty_text(self) -> &'static str {
        match self {
            Self::Artifacts => SELECT_ARTIFACT,
            Self::Files => NO_CONTENT,
            Self::TaskProgress => NO_DETAILS,
        }
    }
}

/// 详情面板浮层纯数据模型（fail-closed；⛔ 不伪造条目/数值）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetailPanelModel {
    /// 数据是否已下发（false ⇒ 一律 [`PanelState::NotDelivered`]）。
    delivered: bool,
    /// 已下发产物条目数（未下发 ⇒ 0）。
    artifact_count: usize,
    /// 已下发文件条目数（未下发 ⇒ 0）。
    file_count: usize,
    /// 侧栏是否已隐藏（决定标题行切换文案）。
    sidebar_hidden: bool,
}

impl DetailPanelModel {
    /// 无数据源的诚实模型（当前 [`main_area`] 即此状态）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            delivered: false,
            artifact_count: 0,
            file_count: 0,
            sidebar_hidden: false,
        }
    }

    /// 由真实下发计数构造（空输入 ⇒ [`PanelState::Empty`] 空态）。
    #[must_use]
    pub fn delivered(artifact_count: usize, file_count: usize) -> Self {
        Self {
            delivered: true,
            artifact_count,
            file_count,
            sidebar_hidden: false,
        }
    }

    /// 纯更新：切换侧栏隐藏态。
    #[must_use]
    pub fn with_sidebar_hidden(self, sidebar_hidden: bool) -> Self {
        Self {
            sidebar_hidden,
            ..self
        }
    }

    /// 推导渲染状态。
    #[must_use]
    pub fn state(&self) -> PanelState {
        if !self.delivered {
            PanelState::NotDelivered
        } else if self.total_count() == 0 {
            PanelState::Empty
        } else {
            PanelState::Normal
        }
    }

    /// 条目总数（未下发 ⇒ 0）。
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.artifact_count + self.file_count
    }

    /// 产物条目数（未下发 ⇒ 0）。
    #[must_use]
    pub fn artifact_count(&self) -> usize {
        self.artifact_count
    }

    /// 文件条目数（未下发 ⇒ 0）。
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.file_count
    }

    /// 标题行侧栏动作文案（已隐藏 ⇒ 显示侧边栏；未隐藏 ⇒ 隐藏侧边栏）。
    #[must_use]
    pub fn sidebar_toggle_text(&self) -> &'static str {
        if self.sidebar_hidden {
            SHOW_SIDEBAR
        } else {
            HIDE_SIDEBAR
        }
    }
}

// ── GPUI 层（颜色一律 theme.*，⛔ 不硬编码色值）────────────────────────

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
///
/// 无数据源 ⇒ [`DetailPanelModel::not_delivered`]，如实显示「未下发」。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&DetailPanelModel::not_delivered(), theme)
}

/// 具名模型渲染（接线/测试用；按纯数据层分支渲染）。
#[must_use]
pub fn render(model: &DetailPanelModel, theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .border_l_1()
        .border_color(theme.border)
        .child(header_row(model, theme))
        .child(new_tab_menu(theme))
        .child(content_body(model, theme))
}

/// 标题行：概览/审查页签 + 关注 / 侧栏切换动作。
fn header_row(model: &DetailPanelModel, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .py_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .text_sm()
                                .font_bold()
                                .text_color(theme.foreground)
                                .child(OVERVIEW),
                        )
                        .child(div().mt_1().h(px(2.)).w_full().bg(theme.accent)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(REVIEW),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(chip(theme, FOLLOW))
                .child(chip(theme, model.sidebar_toggle_text())),
        )
}

/// 新标签页菜单（含「打开浏览器」等子动作）。
fn new_tab_menu(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_4()
        .py_2()
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(NEW_TAB),
        )
        .child(menu_row(theme, OPEN_BROWSER))
        .child(menu_row(theme, OPEN_TERMINAL))
        .child(menu_row(theme, VIEW_WORKSPACE))
        .child(menu_row(theme, VIEW_CHANGES))
}

/// 菜单行（静态文案；接线前无交互）。
fn menu_row(theme: &Theme, label: &'static str) -> Div {
    div()
        .py_1()
        .text_sm()
        .text_color(theme.foreground)
        .child(label)
}

/// 动作胶囊（静态文案；接线前无交互）。
fn chip(theme: &Theme, label: &'static str) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(label)
}

/// 内容体：未下发 ⇒ fail-closed 说明；已下发 ⇒ 分节。
fn content_body(model: &DetailPanelModel, theme: &Theme) -> Div {
    let inner = match model.state() {
        PanelState::NotDelivered => div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED),
        PanelState::Empty | PanelState::Normal => sections(model, theme),
    };
    div().flex_1().flex().flex_col().child(inner)
}

/// 三节清单（产物 / 文件 / 任务进程）。
fn sections(model: &DetailPanelModel, theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .child(section(model, theme, Section::Artifacts))
        .child(section(model, theme, Section::Files))
        .child(section(model, theme, Section::TaskProgress))
}

/// 单节：标题 + 计数/空态体。
fn section(model: &DetailPanelModel, theme: &Theme, s: Section) -> Div {
    let mut head = div().flex().items_center().justify_between().child(
        div()
            .text_xs()
            .font_bold()
            .text_color(theme.foreground)
            .child(s.label()),
    );
    if let Some(count) = section_count(model, s) {
        head = head.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(format!("{count}")),
        );
    }
    let mut body = div();
    match model.state() {
        PanelState::Empty => {
            body = body
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(s.empty_text());
        }
        PanelState::NotDelivered => {
            // 防御：上层已按状态分流，正常不会走到。
            body = body
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED);
        }
        PanelState::Normal => {
            if s == Section::Artifacts {
                body = body.child(chip(theme, NEW_CANVAS));
            }
        }
    }
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_4()
        .py_2()
        .border_t_1()
        .border_color(theme.border)
        .child(head)
        .child(body)
}

/// Normal 态的节计数（仅真实下发数值；任务进程无计数）。
fn section_count(model: &DetailPanelModel, s: Section) -> Option<usize> {
    if model.state() != PanelState::Normal {
        return None;
    }
    match s {
        Section::Artifacts => Some(model.artifact_count()),
        Section::Files => Some(model.file_count()),
        Section::TaskProgress => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DetailPanelModel, PanelState, Section, ARTIFACTS, FILES, FOLLOW, HIDE_SIDEBAR, NEW_CANVAS,
        NEW_TAB, NOT_DELIVERED, OPEN_BROWSER, OPEN_TERMINAL, OVERVIEW, REVIEW, SHOW_SIDEBAR,
        TASK_PROGRESS, VIEW_CHANGES, VIEW_WORKSPACE,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn state_is_not_delivered_without_source() {
        let model = DetailPanelModel::not_delivered();
        assert_eq!(model.state(), PanelState::NotDelivered);
        assert_eq!(model.total_count(), 0);
    }

    #[test]
    fn empty_input_yields_empty_state_not_empty_list() {
        assert_eq!(DetailPanelModel::delivered(0, 0).state(), PanelState::Empty);
    }

    #[test]
    fn normal_state_counts_match_delivered_input() {
        let model = DetailPanelModel::delivered(3, 2);
        assert_eq!(model.state(), PanelState::Normal);
        assert_eq!(model.artifact_count(), 3);
        assert_eq!(model.file_count(), 2);
        assert_eq!(model.total_count(), 5);
    }

    #[test]
    fn section_labels_and_empty_texts_are_verbatim() {
        assert_eq!(Section::Artifacts.label(), "产物");
        assert_eq!(Section::Files.label(), "文件");
        assert_eq!(Section::TaskProgress.label(), "任务进程");
        assert_eq!(Section::Artifacts.empty_text(), "请选择一个产物查看详情");
        assert_eq!(Section::Files.empty_text(), "暂无内容");
        assert_eq!(Section::TaskProgress.empty_text(), "暂无详情");
    }

    #[test]
    fn chrome_copy_is_verbatim_from_namespace() {
        assert_eq!(OVERVIEW, "概览");
        assert_eq!(REVIEW, "审查");
        assert_eq!(FOLLOW, "关注");
        assert_eq!(HIDE_SIDEBAR, "隐藏侧边栏");
        assert_eq!(SHOW_SIDEBAR, "显示侧边栏");
        assert_eq!(NEW_TAB, "新标签页");
        assert_eq!(OPEN_BROWSER, "打开浏览器");
        assert_eq!(OPEN_TERMINAL, "打开终端");
        assert_eq!(VIEW_WORKSPACE, "查看工作空间");
        assert_eq!(VIEW_CHANGES, "查看变更");
        assert_eq!(NEW_CANVAS, "新画布");
        assert_eq!(ARTIFACTS, "产物");
        assert_eq!(FILES, "文件");
        assert_eq!(TASK_PROGRESS, "任务进程");
    }

    #[test]
    fn not_delivered_copy_is_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn sidebar_toggle_text_flips_with_state() {
        assert_eq!(
            DetailPanelModel::not_delivered().sidebar_toggle_text(),
            "隐藏侧边栏"
        );
        assert_eq!(
            DetailPanelModel::not_delivered()
                .with_sidebar_hidden(true)
                .sidebar_toggle_text(),
            "显示侧边栏"
        );
    }

    #[test]
    fn main_area_does_not_panic_without_source() {
        let _ = super::main_area(&Theme::default());
    }

    #[test]
    fn render_covers_all_state_branches() {
        let theme = Theme::default();
        let _ = super::render(&DetailPanelModel::not_delivered(), &theme);
        let _ = super::render(&DetailPanelModel::delivered(0, 0), &theme);
        let _ = super::render(&DetailPanelModel::delivered(2, 1), &theme);
    }
}
