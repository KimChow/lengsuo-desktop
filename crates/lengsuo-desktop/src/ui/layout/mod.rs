//! 工作台三区布局骨架：左边栏 / 会话流 / 右侧面板，外加底部输入区。
//!
//! 本模块只负责**版式状态**（尺寸、折叠、tab 选中）与三区组合，
//! 不实现业务语义、不持有协议数据：
//!
//! - 会话内容来自 [`crate::state::session_stream::SessionStreamState`]（调用方传入只读引用）；
//! - 边栏会话列表来自 [`crate::ui::sidebar::SidebarState`]；
//! - 输入草稿 / 模式来自 [`crate::ui::input_area::InputAreaState`]；
//! - 右侧面板内容由 [`crate::ui::right_panel`] 生成。
//!
//! 与其余 `ui` 模块一致，[`WorkbenchLayout::render`] 产出的是**渲染描述**
//! （纯数据），不依赖 GPUI，便于在无窗口环境下测试。GPUI 组件接线见
//! `docs/workbench-layout-status.md` 的缺口说明。
//!
//! 尺寸单位统一为逻辑像素（与 `theme::tokens` 一致）。默认值对齐任务书：
//! 边栏 **260px**，右侧面板可折叠，三区宽度可调。
//!
//! ⚠️ 本文件由 `ui/layout.rs` 拆分而来（`e33-layout-split`）：
//! - [`view`]：渲染描述类型（`StreamView` / `ModeButton` / `InputView` / `WorkbenchView`）
//!   与 [`WorkbenchLayout::render`]；
//! - `tests.rs`：版式纯逻辑单测；
//! - 拆分的理由：原文件 696 行，距门禁 700 行硬上限只差 4 行 ⇒ 任何编辑都可能顶爆门禁。

use crate::state::layout_prefs::LayoutPrefs;

pub mod view;

pub use view::{InputView, ModeButton, StreamView, WorkbenchView};

#[cfg(test)]
mod tests;

/// 边栏默认宽度（任务书硬约束：260px）。
///
/// **同源常量**：值住在状态层（[`crate::state::layout_prefs`]），避免「描述层默认值」
/// 与「偏好回落默认值」两处各写一份而漂移。
pub const SIDEBAR_DEFAULT_WIDTH: f32 = crate::state::layout_prefs::DEFAULT_SIDEBAR_WIDTH;
/// 边栏最小宽度；再窄会把会话标题挤没。
pub const SIDEBAR_MIN_WIDTH: f32 = 200.0;
/// 边栏最大宽度；避免把会话流压到不可用。
pub const SIDEBAR_MAX_WIDTH: f32 = 420.0;
/// 会话流最小宽度。窗口过窄时优先保住会话流，允许三区总宽超出。
pub const STREAM_MIN_WIDTH: f32 = 360.0;
/// 右侧面板默认宽度（**同源常量**，理由同 [`SIDEBAR_DEFAULT_WIDTH`]）。
pub const RIGHT_PANEL_DEFAULT_WIDTH: f32 = crate::state::layout_prefs::DEFAULT_RIGHT_WIDTH;
/// 右侧面板最小宽度（展开态）。
pub const RIGHT_PANEL_MIN_WIDTH: f32 = 240.0;
/// 右侧面板最大宽度（展开态）。
pub const RIGHT_PANEL_MAX_WIDTH: f32 = 560.0;
/// 右侧面板折叠后的窄条宽度：只保留 tab 入口。
pub const RIGHT_PANEL_COLLAPSED_WIDTH: f32 = 44.0;

/// 输入区占位文案（多行输入）。
pub const INPUT_PLACEHOLDER: &str = "输入消息，可换行；Enter 发送，Shift+Enter 换行";
/// 会话流没有任何可渲染块时的占位文案。
pub const STREAM_EMPTY_TEXT: &str = "还没有消息，开始一段对话吧";

/// 右侧面板的三个 tab。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPanelTab {
    /// 任务清单。
    Tasks,
    /// 文件预览。
    Preview,
    /// 审批待办。
    Approvals,
}

impl RightPanelTab {
    /// 界面上的 tab 顺序。
    pub const ALL: [Self; 3] = [Self::Tasks, Self::Preview, Self::Approvals];

    /// tab 展示文案。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tasks => "任务清单",
            Self::Preview => "文件预览",
            Self::Approvals => "审批待办",
        }
    }

    /// 稳定标识（持久化选中 tab 时使用，不用文案派发）。
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Tasks => "tasks",
            Self::Preview => "preview",
            Self::Approvals => "approvals",
        }
    }

    /// 按 id 查找；未知值返回 `None`，不静默回落到某个 tab。
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.id() == id)
    }
}

/// 边栏版式状态：只有宽度（当前卡的折叠项在右侧面板）。
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarLayout {
    width: f32,
}

impl Default for SidebarLayout {
    fn default() -> Self {
        Self {
            width: SIDEBAR_DEFAULT_WIDTH,
        }
    }
}

impl SidebarLayout {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// 设置宽度并夹到 [`SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`]。
    /// 非有限值（NaN / ∞）被忽略，保持原宽度。
    pub fn set_width(&mut self, width: f32) {
        if width.is_finite() {
            self.width = width.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
        }
    }

    /// 按增量调整宽度（拖拽分隔条）。
    pub fn resize_by(&mut self, delta: f32) {
        self.set_width(self.width + delta);
    }
}

/// 右侧面板版式状态：宽度 + 折叠 + 选中 tab。
#[derive(Debug, Clone, PartialEq)]
pub struct RightPanelLayout {
    width: f32,
    collapsed: bool,
    tab: RightPanelTab,
}

impl Default for RightPanelLayout {
    fn default() -> Self {
        Self {
            width: RIGHT_PANEL_DEFAULT_WIDTH,
            collapsed: false,
            tab: RightPanelTab::Tasks,
        }
    }
}

impl RightPanelLayout {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 展开态宽度（折叠时仍保留，用于展开后恢复）。
    #[must_use]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// 当前实际占位宽度：折叠时为窄条宽度。
    #[must_use]
    pub fn effective_width(&self) -> f32 {
        if self.collapsed {
            RIGHT_PANEL_COLLAPSED_WIDTH
        } else {
            self.width
        }
    }

    #[must_use]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// 设置展开态宽度并夹到合法区间；非有限值被忽略。
    pub fn set_width(&mut self, width: f32) {
        if width.is_finite() {
            self.width = width.clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH);
        }
    }

    /// 拖拽调整宽度。折叠态下不调整（窄条不可拖拽），也不自动展开。
    pub fn resize_by(&mut self, delta: f32) {
        if !self.collapsed {
            self.set_width(self.width + delta);
        }
    }

    pub fn collapse(&mut self) {
        self.collapsed = true;
    }

    pub fn expand(&mut self) {
        self.collapsed = false;
    }

    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    #[must_use]
    pub fn tab(&self) -> RightPanelTab {
        self.tab
    }

    /// 选中 tab。折叠态下选中 tab 会展开面板 —— 折叠窄条只用于恢复入口，
    /// 不展示 tab 内容。
    pub fn select_tab(&mut self, tab: RightPanelTab) {
        self.tab = tab;
        self.collapsed = false;
    }

    /// 按 id 选中 tab；未知 id 返回 `false` 且不改变任何状态。
    pub fn select_tab_id(&mut self, id: &str) -> bool {
        match RightPanelTab::from_id(id) {
            Some(tab) => {
                self.select_tab(tab);
                true
            }
            None => false,
        }
    }
}

/// 三区实际占位宽度。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionWidths {
    pub sidebar: f32,
    pub stream: f32,
    pub right_panel: f32,
}

/// 完整的工作台版式状态。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkbenchLayout {
    sidebar: SidebarLayout,
    right_panel: RightPanelLayout,
}

impl WorkbenchLayout {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn sidebar(&self) -> &SidebarLayout {
        &self.sidebar
    }

    pub fn sidebar_mut(&mut self) -> &mut SidebarLayout {
        &mut self.sidebar
    }

    #[must_use]
    pub fn right_panel(&self) -> &RightPanelLayout {
        &self.right_panel
    }

    pub fn right_panel_mut(&mut self) -> &mut RightPanelLayout {
        &mut self.right_panel
    }

    #[must_use]
    pub fn sidebar_width(&self) -> f32 {
        self.sidebar.width()
    }

    pub fn set_sidebar_width(&mut self, width: f32) {
        self.sidebar.set_width(width);
    }

    pub fn resize_sidebar_by(&mut self, delta: f32) {
        self.sidebar.resize_by(delta);
    }

    /// 用偏好覆盖宽度与折叠态（偏好非法时保持默认值）。
    ///
    /// 宽度**一律经 `set_width` 落地** ⇒ 越界值自动被夹到合法区间、非有限值被忽略
    /// ⇒ 描述层仍是**夹取逻辑的唯一实现处**。tab 选中态**不在**偏好里 ⇒ 不动它。
    pub fn apply_prefs(&mut self, prefs: LayoutPrefs) {
        self.sidebar.set_width(prefs.sidebar_width);
        let panel = &mut self.right_panel;
        panel.set_width(prefs.right_width);
        if prefs.right_collapsed {
            panel.collapse();
        } else {
            panel.expand();
        }
    }

    /// 导出当前版式为偏好值（宽度与折叠态；tab 选中态**不持久化**）。
    #[must_use]
    pub fn export_prefs(&self) -> LayoutPrefs {
        LayoutPrefs {
            sidebar_width: self.sidebar.width(),
            right_width: self.right_panel.width(),
            right_collapsed: self.right_panel.is_collapsed(),
        }
    }

    /// 在给定窗口宽度下计算三区宽度。
    ///
    /// 会话流拿走剩余宽度；低于 [`STREAM_MIN_WIDTH`] 时按最小值算
    /// （宁可整体横向溢出，也不把会话流压到不可用）。
    #[must_use]
    pub fn regions(&self, available_width: f32) -> RegionWidths {
        let sidebar = self.sidebar.width();
        let right_panel = self.right_panel.effective_width();
        let stream = (available_width - sidebar - right_panel).max(STREAM_MIN_WIDTH);
        RegionWidths {
            sidebar,
            stream,
            right_panel,
        }
    }
}
