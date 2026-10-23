//! 版式纯逻辑单测（不依赖窗口）。
//!
//! 显式导入（不做父模块通配导入）：父模块的导入会带进 GPUI 自己的 `test`
//! 属性宏并遮蔽标准库的 `#[test]`（速查表 §12.3）。

use crate::state::layout_prefs::LayoutPrefs;
use crate::state::session_stream::{
    MessageRole, PendingApproval, SessionMessage, SessionStreamState,
};
use crate::ui::input_area::{InputAreaState, InputMode};
use crate::ui::layout::{
    RightPanelLayout, RightPanelTab, SidebarLayout, WorkbenchLayout, RIGHT_PANEL_COLLAPSED_WIDTH,
    RIGHT_PANEL_DEFAULT_WIDTH, RIGHT_PANEL_MIN_WIDTH, SIDEBAR_COLLAPSED_WIDTH,
    SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, STREAM_EMPTY_TEXT,
    STREAM_MIN_WIDTH,
};
use crate::ui::right_panel::RightPanelContent;
use crate::ui::session_render::{Block, STALE_NOTICE};
use crate::ui::sidebar::{self, SessionEntry, SidebarState};

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
    assert!(!layout.sidebar().is_collapsed(), "边栏默认不折叠");
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
fn sidebar_collapse_keeps_the_stored_width_and_narrows_the_region() {
    let mut layout = WorkbenchLayout::new();
    layout.sidebar_mut().collapse();
    assert!(layout.sidebar().is_collapsed());
    assert_eq!(
        layout.sidebar().effective_width(),
        SIDEBAR_COLLAPSED_WIDTH,
        "折叠时占位宽度是固定窄条"
    );
    assert_eq!(
        layout.sidebar().width(),
        SIDEBAR_DEFAULT_WIDTH,
        "展开宽度要保留"
    );

    let collapsed = layout.regions(1200.0);
    assert_eq!(collapsed.sidebar, SIDEBAR_COLLAPSED_WIDTH);
    assert_eq!(
        collapsed.stream,
        1200.0 - SIDEBAR_COLLAPSED_WIDTH - RIGHT_PANEL_DEFAULT_WIDTH,
        "边栏折叠后会话流变宽"
    );

    layout.sidebar_mut().resize_by(100.0);
    assert_eq!(
        layout.sidebar().width(),
        SIDEBAR_DEFAULT_WIDTH,
        "折叠态拖拽不得改动展开宽度"
    );
}

#[test]
fn sidebar_toggle_collapsed_switches_both_ways() {
    let mut sidebar = SidebarLayout::new();
    assert!(!sidebar.is_collapsed());
    sidebar.toggle_collapsed();
    assert!(sidebar.is_collapsed());
    assert_eq!(sidebar.effective_width(), SIDEBAR_COLLAPSED_WIDTH);
    sidebar.toggle_collapsed();
    assert!(!sidebar.is_collapsed());
    assert_eq!(sidebar.effective_width(), SIDEBAR_DEFAULT_WIDTH);
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
        RightPanelContent::Tasks
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
    let view =
        WorkbenchLayout::new().render(&session, &sidebar_state(), &InputAreaState::new(), 1000.0);
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

// ---- e22：版式偏好导入 / 导出 ----

/// 偏好恢复：宽度与折叠态按偏好恢复；越界宽度被夹取、非有限宽度被忽略；
/// 导出 → 导入 → 导出幂等。
#[test]
fn apply_prefs_restores_clamps_and_round_trips() {
    let mut layout = WorkbenchLayout::new();
    layout.apply_prefs(LayoutPrefs {
        sidebar_width: 300.0,
        sidebar_collapsed: true,
        right_width: 400.0,
        right_collapsed: true,
    });
    assert_eq!(layout.sidebar_width(), 300.0);
    assert!(layout.sidebar().is_collapsed());
    assert_eq!(
        layout.sidebar().effective_width(),
        SIDEBAR_COLLAPSED_WIDTH,
        "偏好里的折叠态要恢复"
    );
    assert_eq!(layout.right_panel().width(), 400.0);
    assert!(layout.right_panel().is_collapsed());
    assert_eq!(
        layout.right_panel().effective_width(),
        RIGHT_PANEL_COLLAPSED_WIDTH,
        "折叠时占位宽度是固定窄条"
    );

    // 越界 ⇒ 夹取；非有限 ⇒ 忽略（描述层是夹取逻辑的唯一实现处）。
    layout.apply_prefs(LayoutPrefs {
        sidebar_width: 10.0,
        sidebar_collapsed: false,
        right_width: f32::NAN,
        right_collapsed: false,
    });
    assert_eq!(layout.sidebar_width(), SIDEBAR_MIN_WIDTH);
    assert!(!layout.sidebar().is_collapsed(), "偏好里的展开态要恢复");
    assert_eq!(
        layout.right_panel().width(),
        400.0,
        "非有限宽度不得破坏版式"
    );
    assert!(!layout.right_panel().is_collapsed());

    // 导出 → 导入 → 导出：幂等。
    layout.set_sidebar_width(280.0);
    layout.sidebar_mut().collapse();
    layout.right_panel_mut().set_width(360.0);
    layout.right_panel_mut().collapse();
    let exported = layout.export_prefs();
    assert_eq!(exported.sidebar_width, 280.0);
    assert!(exported.sidebar_collapsed, "边栏折叠态要导出");
    assert_eq!(exported.right_width, 360.0, "导出的是展开态宽度");
    assert!(exported.right_collapsed);
    let mut restored = WorkbenchLayout::new();
    restored.apply_prefs(exported);
    assert_eq!(restored.export_prefs(), exported, "往返必须幂等");
}
