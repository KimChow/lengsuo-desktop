//! 描述层测试：会话分组契约 + 导航入口契约。
//!
//! ⛔ 不在这里再包 `mod tests {}`（clippy `module_inception` 会拦）。
//! ⛔ 不用 `use super::*`：显式列出符号，测试才读得出依赖。

use crate::ui::sidebar::nav::{
    agents_entry, channels_entry, environments_entry, nav_entries, organization_entry, NavKind,
    AGENTS_TITLE, CHANNELS_TITLE, ENVIRONMENTS_TITLE, ORGANIZATION_TITLE,
};
use crate::ui::sidebar::{
    render, SessionEntry, SessionRow, SidebarState, ARCHIVED_SECTION, ARCHIVED_TOGGLE_HIDE,
    ARCHIVED_TOGGLE_SHOW, EMPTY_ACTIVE_TEXT, EMPTY_TEXT, HEADING, PINNED_SECTION, SESSIONS_SECTION,
    UNTITLED_SESSION,
};

fn sample() -> SidebarState {
    let mut state = SidebarState::new();
    state.set_entries(vec![
        SessionEntry::new("a", "普通一").with_unread(true),
        SessionEntry::new("b", "置顶一").with_pinned(true),
        SessionEntry::new("c", "归档一").with_archived(true),
        SessionEntry::new("d", "普通二"),
    ]);
    state
}

#[test]
fn entries_keep_injection_order_and_selection_starts_empty() {
    let state = sample();
    assert_eq!(state.entries().len(), 4);
    assert_eq!(state.selected(), None);
    assert_eq!(state.unread_count(), 1);
    assert_eq!(state.archived_count(), 1);
}

#[test]
fn pinned_sessions_are_sorted_first() {
    let state = sample();
    let visible: Vec<&str> = state
        .visible_entries()
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(visible, ["b", "a", "d"], "置顶在前，归档隐藏，其余保持顺序");
}

#[test]
fn archived_are_hidden_by_default_and_toggleable() {
    let mut state = sample();
    let view = render(&state, 260.0);
    assert_eq!(view.hidden_archived, 1);
    assert_eq!(view.archived_toggle_label, ARCHIVED_TOGGLE_SHOW);
    assert!(view
        .sections
        .iter()
        .all(|section| section.label != ARCHIVED_SECTION));

    state.toggle_archived();
    let view = render(&state, 260.0);
    assert_eq!(view.hidden_archived, 0);
    assert_eq!(view.archived_toggle_label, ARCHIVED_TOGGLE_HIDE);
    let archived = view
        .sections
        .iter()
        .find(|section| section.label == ARCHIVED_SECTION)
        .expect("展开后应有归档分组");
    assert_eq!(archived.rows.len(), 1);
    assert_eq!(archived.rows[0].title, "归档一");
}

#[test]
fn sections_show_pinned_then_regular() {
    let view = render(&sample(), 260.0);
    let labels: Vec<&str> = view.sections.iter().map(|s| s.label).collect();
    assert_eq!(labels, [PINNED_SECTION, SESSIONS_SECTION]);
    assert_eq!(view.sections[0].rows.len(), 1);
    assert_eq!(view.sections[1].rows.len(), 2);
    assert_eq!(view.width, 260.0);
    assert_eq!(view.heading, HEADING);
    assert_eq!(view.unread_count, 1);
    assert_eq!(view.empty_text, None);
}

#[test]
fn select_requires_a_visible_entry() {
    let mut state = sample();
    assert!(state.select("a"));
    assert_eq!(state.selected(), Some("a"));
    assert!(!state.select("missing"));
    assert_eq!(state.selected(), Some("a"), "未知 id 不得改选中");
    assert!(!state.select("c"), "归档隐藏时不可选中");
    assert_eq!(state.selected(), Some("a"));
}

#[test]
fn hiding_archived_clears_archived_selection() {
    let mut state = sample();
    state.set_show_archived(true);
    assert!(state.select("c"));
    assert_eq!(state.selected(), Some("c"));
    state.set_show_archived(false);
    assert_eq!(state.selected(), None, "归档隐藏后选中项不得悬空");
}

#[test]
fn replacing_entries_clears_stale_selection() {
    let mut state = sample();
    assert!(state.select("a"));
    state.set_entries(vec![SessionEntry::new("x", "新会话")]);
    assert_eq!(state.selected(), None);
    assert_eq!(state.entries().len(), 1);
}

#[test]
fn empty_state_and_missing_titles_are_explicit() {
    let view = render(&SidebarState::new(), 260.0);
    assert!(view.sections.is_empty());
    assert_eq!(view.empty_text, Some(EMPTY_TEXT));

    let mut only_archived = SidebarState::new();
    only_archived.set_entries(vec![
        SessionEntry::new("archived", "旧会话").with_archived(true)
    ]);
    let view = render(&only_archived, 260.0);
    assert_eq!(view.empty_text, Some(EMPTY_ACTIVE_TEXT));
    assert_eq!(view.hidden_archived, 1);

    let mut state = SidebarState::new();
    state.set_entries(vec![
        SessionEntry::new("blank", "   "),
        SessionEntry::new("archived", "旧会话").with_archived(true),
    ]);
    let view = render(&state, 260.0);
    assert_eq!(view.empty_text, None, "有进行中会话时不显示空态");
    let row = &view.sections[0].rows[0];
    assert_eq!(row.title, UNTITLED_SESSION);
    assert!(row.title_is_placeholder);
    assert!(!row.title.contains("blank"), "不得拿 id 冒充标题");
}

#[test]
fn selected_row_is_marked() {
    let mut state = sample();
    assert!(state.select("b"));
    let view = render(&state, 260.0);
    let selected: Vec<&SessionRow> = view
        .sections
        .iter()
        .flat_map(|section| section.rows.iter())
        .filter(|row| row.selected)
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, "b");
    assert_eq!(view.selected.as_deref(), Some("b"));
}

#[test]
fn unread_count_ignores_archived_sessions() {
    let mut state = SidebarState::new();
    state.set_entries(vec![
        SessionEntry::new("a", "进行中").with_unread(true),
        SessionEntry::new("b", "已归档")
            .with_archived(true)
            .with_unread(true),
    ]);
    assert_eq!(state.unread_count(), 1);
}

// === A15：频道入口 + 智能体管理入口 ===

#[test]
fn nav_entries_expose_channels_and_agents_in_order() {
    let entries = nav_entries(2, 3, 0, 0, None);
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].kind, NavKind::Channels);
    assert_eq!(entries[0].count, 2);
    assert_eq!(entries[0].title, CHANNELS_TITLE);
    assert_eq!(entries[1].kind, NavKind::Agents);
    assert_eq!(entries[1].count, 3);
    assert_eq!(entries[1].title, AGENTS_TITLE);
    // A17：第三个入口是浏览器环境管理（顺序在频道 / 智能体管理之后）。
    assert_eq!(entries[2].kind, NavKind::Environments);
    assert_eq!(entries[2].count, 0);
    assert_eq!(entries[2].title, ENVIRONMENTS_TITLE);
    // A18：第四个入口是组织与权限可见性（顺序在环境之后）。
    assert_eq!(entries[3].kind, NavKind::Organization);
    assert_eq!(entries[3].count, 0);
    assert_eq!(entries[3].title, ORGANIZATION_TITLE);
}

#[test]
fn nav_entry_count_zero_is_not_rendered() {
    assert_eq!(channels_entry(0, false).count, 0);
    assert_eq!(agents_entry(0, false).count, 0);
    assert_eq!(environments_entry(0, false).count, 0);
    assert_eq!(organization_entry(0, false).count, 0);
}

/// A18：组织角标恒 0（成员数在运行时侧，本仓**不伪造**）⇒ 界面上不显示角标。
#[test]
fn organization_count_is_zero_so_no_fake_badge() {
    let state = SidebarState::new();
    assert_eq!(
        state.organization_count(),
        0,
        "组织成员数在运行时侧，本仓不伪造 ⇒ 恒 0（角标不显示）"
    );
}

#[test]
fn select_nav_marks_only_the_target_entry() {
    let mut state = SidebarState::new();
    assert!(state.select_nav(NavKind::Channels));
    assert_eq!(state.selected_nav(), Some(NavKind::Channels));

    let entries = nav_entries(2, 3, 0, 0, state.selected_nav());
    let selected: Vec<NavKind> = entries
        .iter()
        .filter(|entry| entry.selected)
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(selected, [NavKind::Channels], "只有被点中的入口才是选中态");
}

#[test]
fn select_nav_twice_reports_no_change() {
    let mut state = SidebarState::new();
    assert!(state.select_nav(NavKind::Agents));
    assert!(!state.select_nav(NavKind::Agents));
    assert_eq!(state.selected_nav(), Some(NavKind::Agents));
}

#[test]
fn nav_counts_come_from_state_and_reach_the_view() {
    let mut state = SidebarState::new();
    state.set_channel_count(2);
    state.set_agent_count(3);
    assert_eq!(state.channel_count(), 2);
    assert_eq!(state.agent_count(), 3);

    let view = render(&state, 260.0);
    assert_eq!(view.nav.len(), 4);
    assert_eq!(view.nav[0].count, 2);
    assert_eq!(view.nav[1].count, 3);
    assert!(!view.nav[0].selected);
    assert!(!view.nav[1].selected);
}

#[test]
fn selecting_a_nav_entry_marks_it_in_the_view() {
    let mut state = SidebarState::new();
    state.select_nav(NavKind::Agents);
    let view = render(&state, 260.0);
    assert!(!view.nav[0].selected);
    assert!(view.nav[1].selected);
}

// === A17：浏览器环境管理入口 ===

#[test]
fn environment_count_is_zero_so_no_badge_is_faked() {
    let state = SidebarState::new();
    assert_eq!(state.environment_count(), 0);

    let view = render(&state, 260.0);
    let entry = view
        .nav
        .iter()
        .find(|entry| entry.kind == NavKind::Environments)
        .expect("应当有环境入口");
    assert_eq!(entry.count, 0, "环境条目来自运行时侧，界面层不伪造角标");
    assert_eq!(entry.title, ENVIRONMENTS_TITLE);
}

#[test]
fn selecting_the_environment_entry_marks_only_it() {
    let mut state = SidebarState::new();
    assert!(state.select_nav(NavKind::Environments));
    assert_eq!(state.selected_nav(), Some(NavKind::Environments));

    let view = render(&state, 260.0);
    let selected: Vec<NavKind> = view
        .nav
        .iter()
        .filter(|entry| entry.selected)
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(selected, [NavKind::Environments]);
}

#[test]
fn clear_nav_returns_to_sessions_and_is_idempotent() {
    use crate::ui::main_view::{resolve, MainView};
    use crate::ui::sidebar_gpui::demo_state;

    let mut state = demo_state();
    assert!(state.select_nav(NavKind::Channels));
    assert_eq!(resolve(state.selected_nav()), MainView::Channels);

    assert!(state.clear_nav(), "有选中时清空必须报告变化");
    assert_eq!(state.selected_nav(), None);
    assert_eq!(resolve(state.selected_nav()), MainView::Sessions);
    assert!(!state.clear_nav(), "已经空时再清必须是幂等的 false");
}
