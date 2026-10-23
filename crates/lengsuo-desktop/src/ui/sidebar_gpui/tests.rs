//! 接线层测试：描述层契约 + 接线层纯函数的回归。
//!
//! ⛔ 不在这里再包 `mod tests {}`（clippy `module_inception` 会拦）。
//! ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test` 属性宏
//! 带进作用域并遮蔽标准库的 `#[test]` ⇒ 显式列出符号。

use crate::ui::sidebar::{self, NavKind, SessionEntry, SessionRow, SidebarState};
use crate::ui::sidebar_gpui::{
    demo_state, row_marks, select_nav, select_row, toggle_archived, MarkKind, RowMark,
    ARCHIVED_MARK, PINNED_MARK, UNREAD_MARK,
};

fn row(pinned: bool, archived: bool, unread: bool) -> SessionRow {
    let mut state = SidebarState::new();
    state.set_entries(vec![SessionEntry::new("x", "标题")
        .with_pinned(pinned)
        .with_archived(archived)
        .with_unread(unread)]);
    // 归档默认隐藏，展开后才有一行可断言。
    if archived {
        toggle_archived(&mut state);
    }
    let view = sidebar::render(&state, 260.0);
    view.sections
        .iter()
        .flat_map(|section| section.rows.iter())
        .next()
        .expect("应当有一行")
        .clone()
}

#[test]
fn marks_map_description_flags_to_chinese_labels() {
    assert_eq!(
        row_marks(&row(true, false, false)),
        vec![RowMark {
            kind: MarkKind::Pinned,
            label: PINNED_MARK
        }]
    );
    assert_eq!(
        row_marks(&row(false, false, true)),
        vec![RowMark {
            kind: MarkKind::Unread,
            label: UNREAD_MARK
        }]
    );
    assert_eq!(
        row_marks(&row(true, true, true)),
        vec![
            RowMark {
                kind: MarkKind::Pinned,
                label: PINNED_MARK
            },
            RowMark {
                kind: MarkKind::Archived,
                label: ARCHIVED_MARK
            },
            RowMark {
                kind: MarkKind::Unread,
                label: UNREAD_MARK
            },
        ]
    );
    assert!(row_marks(&row(false, false, false)).is_empty());
}

#[test]
fn archived_mark_only_appears_when_archived_is_visible() {
    let mut state = SidebarState::new();
    state.set_entries(vec![SessionEntry::new("a", "旧会话").with_archived(true)]);
    // 归档默认隐藏：描述层不给出这一行，界面也就无从标注。
    let view = sidebar::render(&state, 260.0);
    assert!(view.sections.is_empty());
    assert_eq!(view.hidden_archived, 1);

    toggle_archived(&mut state);
    let view = sidebar::render(&state, 260.0);
    let archived_row = view.sections[0].rows[0].clone();
    assert_eq!(
        row_marks(&archived_row),
        vec![RowMark {
            kind: MarkKind::Archived,
            label: ARCHIVED_MARK
        }]
    );
}

#[test]
fn demo_state_covers_pinned_archived_and_unread() {
    let state = demo_state();
    assert_eq!(state.entries().len(), 3);
    assert_eq!(state.unread_count(), 1);
    assert_eq!(state.archived_count(), 1);

    let description = sidebar::render(&state, 260.0);
    let labels: Vec<&str> = description
        .sections
        .iter()
        .map(|section| section.label)
        .collect();
    assert_eq!(labels, ["置顶", "全部会话"]);
}

#[test]
fn clicking_a_row_writes_the_selection_back() {
    let mut state = demo_state();
    assert!(select_row(&mut state, "demo-plain"));
    assert_eq!(state.selected(), Some("demo-plain"));

    let description = sidebar::render(&state, 260.0);
    let selected: Vec<&str> = description
        .sections
        .iter()
        .flat_map(|section| section.rows.iter())
        .filter(|row| row.selected)
        .map(|row| row.id.as_str())
        .collect();
    assert_eq!(selected, ["demo-plain"]);
}

#[test]
fn clicking_an_archived_row_while_hidden_changes_nothing() {
    let mut state = demo_state();
    assert!(!select_row(&mut state, "demo-archived"), "归档隐藏时不可选");
    assert_eq!(state.selected(), None);

    toggle_archived(&mut state);
    assert!(select_row(&mut state, "demo-archived"), "展开后可选");
    assert_eq!(state.selected(), Some("demo-archived"));
}

#[test]
fn toggling_archived_reveals_the_archived_section() {
    let mut state = demo_state();
    assert!(!state.show_archived());
    toggle_archived(&mut state);
    assert!(state.show_archived());
    let description = sidebar::render(&state, 260.0);
    assert_eq!(description.hidden_archived, 0);
    assert!(description
        .sections
        .iter()
        .any(|section| section.label == sidebar::ARCHIVED_SECTION));
}

// === A15：频道入口 + 智能体管理入口 ===

#[test]
fn demo_state_carries_channel_and_agent_counts() {
    let state = demo_state();
    assert_eq!(state.channel_count(), 2);
    assert_eq!(state.agent_count(), 3);

    let description = sidebar::render(&state, 260.0);
    assert_eq!(description.nav.len(), 14);
    assert_eq!(description.nav[0].title, "频道");
    assert_eq!(description.nav[0].count, 2);
    assert_eq!(description.nav[1].title, "智能体管理");
    assert_eq!(description.nav[1].count, 3);
    // A17：第三个入口是浏览器环境管理（角标 0 ⇒ 不显示）。
    assert_eq!(description.nav[2].kind, NavKind::Environments);
    assert_eq!(description.nav[2].title, "环境");
    assert_eq!(description.nav[2].count, 0);
    // A18：第四个入口是组织与权限可见性（角标 0 ⇒ 不显示）。
    assert_eq!(description.nav[3].kind, NavKind::Organization);
    assert_eq!(description.nav[3].title, "组织");
    assert_eq!(description.nav[3].count, 0);
}

#[test]
fn clicking_a_nav_entry_writes_the_selection_back() {
    let mut state = demo_state();
    assert!(select_nav(&mut state, NavKind::Channels));
    assert_eq!(state.selected_nav(), Some(NavKind::Channels));

    let description = sidebar::render(&state, 260.0);
    let selected: Vec<NavKind> = description
        .nav
        .iter()
        .filter(|entry| entry.selected)
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(selected, [NavKind::Channels]);
}

/// e41：边栏视图的折叠态可被宿主读取（回收 / 下发都靠这个入口）。
#[cfg(feature = "ui")]
#[test]
fn sidebar_view_exposes_its_collapsed_state() {
    use crate::ui::sidebar_gpui::SidebarViewGpui;

    let mut view = SidebarViewGpui::new(demo_state());
    assert!(!view.is_collapsed(), "默认展开");
    view.toggle_collapsed();
    assert!(view.is_collapsed(), "点收起 ⇒ 折叠");
    view.set_collapsed(false);
    assert!(!view.is_collapsed(), "宿主下发展开态应生效");
}

/// A17：点「环境」入口 ⇒ 描述层选中态变成 `NavKind::Environments`。
#[test]
fn clicking_the_environment_entry_writes_the_selection_back() {
    let mut state = demo_state();
    assert!(select_nav(&mut state, NavKind::Environments));
    assert_eq!(state.selected_nav(), Some(NavKind::Environments));

    let description = sidebar::render(&state, 260.0);
    let selected: Vec<NavKind> = description
        .nav
        .iter()
        .filter(|entry| entry.selected)
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(selected, [NavKind::Environments]);
}
