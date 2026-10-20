//! 工作台三区布局骨架：左边栏 / 会话流 / 右侧面板，外加底部输入区。
//!
//! 本模块只负责**版式状态**（尺寸、折叠、tab 选中）与三区组合，
//! 不实现业务语义、不持有协议数据：
//!
//! - 会话内容来自 [`SessionStreamState`]（调用方传入只读引用）；
//! - 边栏会话列表来自 [`SidebarState`]；
//! - 输入草稿 / 模式来自 [`InputAreaState`]；
//! - 右侧面板内容由 [`crate::ui::right_panel`] 生成。
//!
//! 与其余 `ui` 模块一致，[`WorkbenchLayout::render`] 产出的是**渲染描述**
//! （纯数据），不依赖 GPUI，便于在无窗口环境下测试。GPUI 组件接线见
//! `docs/workbench-layout-status.md` 的缺口说明。
//!
//! 尺寸单位统一为逻辑像素（与 `theme::tokens` 一致）。默认值对齐任务书：
//! 边栏 **260px**，右侧面板可折叠，三区宽度可调。

use crate::state::session_stream::SessionStreamState;
use crate::ui::input_area::{InputAreaState, InputMode, SendButton};
use crate::ui::right_panel::{self, RightPanelView};
use crate::ui::session_render::{self, Block};
use crate::ui::sidebar::{self, SidebarState, SidebarView};

/// 边栏默认宽度（任务书硬约束：260px）。
pub const SIDEBAR_DEFAULT_WIDTH: f32 = 260.0;
/// 边栏最小宽度；再窄会把会话标题挤没。
pub const SIDEBAR_MIN_WIDTH: f32 = 200.0;
/// 边栏最大宽度；避免把会话流压到不可用。
pub const SIDEBAR_MAX_WIDTH: f32 = 420.0;
/// 会话流最小宽度。窗口过窄时优先保住会话流，允许三区总宽超出。
pub const STREAM_MIN_WIDTH: f32 = 360.0;
/// 右侧面板默认宽度。
pub const RIGHT_PANEL_DEFAULT_WIDTH: f32 = 320.0;
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

    /// 生成完整工作台的渲染描述。
    ///
    /// 会话流块由 [`session_render::render`] 生成：断档提示、四类消息、
    /// 待审批卡片全部只读消费 [`SessionStreamState`]。
    #[must_use]
    pub fn render(
        &self,
        session: &SessionStreamState,
        sidebar_state: &SidebarState,
        input: &InputAreaState,
        available_width: f32,
    ) -> WorkbenchView {
        let widths = self.regions(available_width);
        let blocks = session_render::render(session);
        WorkbenchView {
            widths,
            sidebar: sidebar::render(sidebar_state, widths.sidebar),
            stream: StreamView {
                stale: session.is_stale(),
                empty_text: blocks.is_empty().then_some(STREAM_EMPTY_TEXT),
                blocks,
            },
            right_panel: right_panel::render(&self.right_panel, session),
            input: input_view(input),
        }
    }
}

/// 会话流区域视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamView {
    /// 消息 / 工具卡片 / 审批 / 断档提示（顺序即渲染顺序）。
    pub blocks: Vec<Block>,
    /// 是否处于事件流断档态（与 `blocks` 首块的断档提示一致）。
    pub stale: bool,
    /// 没有任何可渲染内容时的占位文案。
    pub empty_text: Option<&'static str>,
}

/// 输入区模式按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeButton {
    /// 稳定 id（`quick` / `ask` / ...）。
    pub id: &'static str,
    pub label: &'static str,
    pub selected: bool,
}

/// 输入区视图：多行草稿 + 5 模式 + 发送按钮。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputView {
    pub draft: String,
    pub line_count: usize,
    pub placeholder: &'static str,
    pub modes: Vec<ModeButton>,
    pub send: SendButton,
}

/// 完整工作台的渲染描述。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchView {
    pub widths: RegionWidths,
    pub sidebar: SidebarView,
    pub stream: StreamView,
    pub right_panel: RightPanelView,
    pub input: InputView,
}

fn input_view(input: &InputAreaState) -> InputView {
    let mode = input.mode();
    InputView {
        draft: input.draft().to_owned(),
        line_count: input.line_count(),
        placeholder: INPUT_PLACEHOLDER,
        modes: InputMode::ALL
            .iter()
            .map(|&candidate| ModeButton {
                id: candidate.id(),
                label: candidate.label(),
                selected: candidate == mode,
            })
            .collect(),
        send: input.send_button(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::session_stream::{MessageRole, PendingApproval, SessionMessage};
    use crate::ui::right_panel::RightPanelContent;
    use crate::ui::session_render::STALE_NOTICE;
    use crate::ui::sidebar::SessionEntry;

    fn session_with_message() -> SessionStreamState {
        let mut session = SessionStreamState::new();
        assert!(session.push_message(
            1,
            SessionMessage {
                role: MessageRole::User,
                content: "你好".into(),
                tool: None,
                ts_ms: 0,
            }
        ));
        session
    }

    fn sidebar_state() -> SidebarState {
        let mut state = SidebarState::new();
        state.set_entries(vec![SessionEntry::new("s1", "第一个会话")]);
        state
    }

    #[test]
    fn defaults_match_the_260px_three_region_spec() {
        let layout = WorkbenchLayout::new();
        assert_eq!(layout.sidebar_width(), SIDEBAR_DEFAULT_WIDTH);
        assert_eq!(SIDEBAR_DEFAULT_WIDTH, 260.0);
        let panel = layout.right_panel();
        assert!(!panel.is_collapsed());
        assert_eq!(panel.width(), RIGHT_PANEL_DEFAULT_WIDTH);
        assert_eq!(panel.effective_width(), RIGHT_PANEL_DEFAULT_WIDTH);
        assert_eq!(panel.tab(), RightPanelTab::Tasks, "默认选中任务清单");
    }

    #[test]
    fn widths_are_clamped_to_usable_ranges() {
        let mut sidebar = SidebarLayout::new();
        sidebar.set_width(10.0);
        assert_eq!(sidebar.width(), SIDEBAR_MIN_WIDTH);
        sidebar.set_width(10_000.0);
        assert_eq!(sidebar.width(), SIDEBAR_MAX_WIDTH);
        sidebar.set_width(f32::NAN);
        assert_eq!(sidebar.width(), SIDEBAR_MAX_WIDTH, "非有限值不得破坏宽度");

        let mut panel = RightPanelLayout::new();
        panel.set_width(1.0);
        assert_eq!(panel.width(), RIGHT_PANEL_MIN_WIDTH);
        panel.set_width(f32::INFINITY);
        assert_eq!(panel.width(), RIGHT_PANEL_MIN_WIDTH);
    }

    #[test]
    fn resize_by_moves_width_within_bounds() {
        let mut layout = WorkbenchLayout::new();
        layout.resize_sidebar_by(10.0);
        assert_eq!(layout.sidebar_width(), 270.0);
        layout.resize_sidebar_by(-1000.0);
        assert_eq!(layout.sidebar_width(), SIDEBAR_MIN_WIDTH);

        layout.right_panel_mut().resize_by(10.0);
        assert_eq!(
            layout.right_panel().width(),
            RIGHT_PANEL_DEFAULT_WIDTH + 10.0
        );
    }

    #[test]
    fn collapse_and_expand_keep_the_stored_width() {
        let mut panel = RightPanelLayout::new();
        panel.collapse();
        assert!(panel.is_collapsed());
        assert_eq!(panel.effective_width(), RIGHT_PANEL_COLLAPSED_WIDTH);
        assert_eq!(panel.width(), RIGHT_PANEL_DEFAULT_WIDTH, "展开宽度要保留");
        panel.resize_by(100.0);
        assert_eq!(
            panel.width(),
            RIGHT_PANEL_DEFAULT_WIDTH,
            "折叠态拖拽不得改动展开宽度"
        );
        panel.toggle_collapsed();
        assert!(!panel.is_collapsed());
        assert_eq!(panel.effective_width(), RIGHT_PANEL_DEFAULT_WIDTH);
    }

    #[test]
    fn tab_switch_rejects_unknown_and_expands_when_collapsed() {
        let mut panel = RightPanelLayout::new();
        assert_eq!(panel.tab(), RightPanelTab::Tasks);
        assert!(!panel.select_tab_id("bogus"));
        assert_eq!(panel.tab(), RightPanelTab::Tasks, "未知 id 不得静默切换");
        panel.collapse();
        assert!(panel.select_tab_id("approvals"));
        assert_eq!(panel.tab(), RightPanelTab::Approvals);
        assert!(!panel.is_collapsed(), "选中 tab 应展开面板");
    }

    #[test]
    fn all_three_tabs_have_unique_stable_ids() {
        assert_eq!(RightPanelTab::ALL.len(), 3);
        let mut ids: Vec<&str> = RightPanelTab::ALL.iter().map(|tab| tab.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 3);
        for tab in RightPanelTab::ALL {
            assert_eq!(RightPanelTab::from_id(tab.id()), Some(tab));
        }
        assert_eq!(RightPanelTab::from_id("history"), None);
    }

    #[test]
    fn regions_give_the_stream_the_remaining_width() {
        let mut layout = WorkbenchLayout::new();
        let widths = layout.regions(1200.0);
        assert_eq!(widths.sidebar, 260.0);
        assert_eq!(widths.right_panel, 320.0);
        assert_eq!(widths.stream, 620.0);
        assert_eq!(widths.sidebar + widths.stream + widths.right_panel, 1200.0);

        layout.right_panel_mut().collapse();
        let collapsed = layout.regions(1200.0);
        assert_eq!(collapsed.right_panel, RIGHT_PANEL_COLLAPSED_WIDTH);
        assert_eq!(collapsed.stream, 1200.0 - 260.0 - 44.0);
    }

    #[test]
    fn narrow_window_keeps_stream_minimum() {
        let layout = WorkbenchLayout::new();
        let widths = layout.regions(500.0);
        assert_eq!(widths.stream, STREAM_MIN_WIDTH);
        assert!(
            widths.sidebar + widths.stream + widths.right_panel > 500.0,
            "宁可横向溢出也不压垮会话流"
        );
    }

    #[test]
    fn render_composes_all_three_regions_and_input() {
        let mut input = InputAreaState::new();
        input.set_mode(InputMode::Plan);
        input.set_draft("第一行\n第二行");

        let layout = WorkbenchLayout::new();
        let view = layout.render(&session_with_message(), &sidebar_state(), &input, 1200.0);

        assert_eq!(view.widths.sidebar, 260.0);
        assert_eq!(view.sidebar.heading, sidebar::HEADING);
        assert_eq!(view.sidebar.sections.len(), 1);
        assert_eq!(view.sidebar.sections[0].rows[0].title, "第一个会话");

        assert_eq!(view.stream.blocks.len(), 1);
        assert!(!view.stream.stale);
        assert_eq!(view.stream.empty_text, None);

        assert_eq!(view.right_panel.tabs.len(), 3);
        assert!(matches!(
            view.right_panel.content,
            RightPanelContent::Placeholder { .. }
        ));

        assert_eq!(view.input.modes.len(), 5);
        assert_eq!(
            view.input.modes.iter().filter(|mode| mode.selected).count(),
            1
        );
        assert!(view
            .input
            .modes
            .iter()
            .any(|m| m.selected && m.id == "plan"));
        assert_eq!(view.input.line_count, 2);
        assert!(view.input.send.enabled);
    }

    #[test]
    fn empty_stream_and_collapsed_panel_render_placeholders() {
        let layout = {
            let mut layout = WorkbenchLayout::new();
            layout.right_panel_mut().collapse();
            layout
        };
        let view = layout.render(
            &SessionStreamState::new(),
            &SidebarState::new(),
            &InputAreaState::new(),
            900.0,
        );
        assert_eq!(view.stream.empty_text, Some(STREAM_EMPTY_TEXT));
        assert_eq!(view.right_panel.width, RIGHT_PANEL_COLLAPSED_WIDTH);
        assert!(matches!(
            view.right_panel.content,
            RightPanelContent::Collapsed
        ));
        assert!(!view.input.send.enabled, "空输入不得可发送");
        assert_eq!(view.sidebar.empty_text, Some(sidebar::EMPTY_TEXT));
    }

    #[test]
    fn stale_session_surfaces_notice_and_hides_empty_placeholder() {
        let mut session = session_with_message();
        session.mark_gap();
        let view = WorkbenchLayout::new().render(
            &session,
            &sidebar_state(),
            &InputAreaState::new(),
            1000.0,
        );
        assert!(view.stream.stale);
        assert_eq!(view.stream.empty_text, None);
        assert!(matches!(
            view.stream.blocks.first(),
            Some(Block::StaleNotice { text: STALE_NOTICE })
        ));
    }

    #[test]
    fn approvals_are_read_only_from_session_state() {
        let mut session = session_with_message();
        session.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "Bash".into(),
            summary: "rm -rf build".into(),
            resolved: None,
        });
        let mut layout = WorkbenchLayout::new();
        assert!(layout.right_panel_mut().select_tab_id("approvals"));
        let view = layout.render(&session, &sidebar_state(), &InputAreaState::new(), 1000.0);
        assert_eq!(view.right_panel.pending_approval_count, 1);
        match view.right_panel.content {
            RightPanelContent::Approvals(cards) => assert_eq!(cards.len(), 1),
            other => panic!("期望审批卡片，得到 {other:?}"),
        }
        assert_eq!(
            session.pending_approvals().count(),
            1,
            "布局渲染是只读的，不得替用户决策"
        );
    }
}
