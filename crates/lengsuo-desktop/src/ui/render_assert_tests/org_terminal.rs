//! C + D 段：组织域三分区各画一遍；终端抽屉收起 / 展开两条分支。

use super::{assert_rendered, draw_probe};
use gpui_kit::{AppContext as _, IntoElement as _};

use crate::ui::org_gpui::{OrgSection, OrgViewGpui};
use crate::ui::org_view::OrgViewState;
use crate::ui::terminal_gpui::TerminalDrawerGpui;

fn org_view(section: OrgSection) -> OrgViewGpui {
    let mut state = OrgViewState::default();
    state
        .departments_mut()
        .apply(vec![serde_json::json!({"id": "d1", "name": "研发部"})]);
    state.members_mut().apply(vec![
        serde_json::json!({"id": "u1", "name": "甲", "email": "a@example.com"}),
    ]);
    state
        .roles_mut()
        .apply(vec![serde_json::json!({"id": "r1", "name": "管理员"})]);

    let mut view = OrgViewGpui::new(state);
    view.set_section(section);
    view
}

#[test]
fn org_view_every_section_renders() {
    for section in OrgSection::ALL {
        let bounds = draw_probe(|ctx| ctx.new(|_cx| org_view(section)).into_any_element());
        assert_rendered(bounds, section.label());
    }
}

#[test]
fn terminal_drawer_collapsed_renders() {
    let bounds = draw_probe(|ctx| ctx.new(|_cx| TerminalDrawerGpui::new()).into_any_element());
    assert_rendered(bounds, "终端抽屉（收起）");
}

/// 展开态多了 tab 条 + 220px 固定内容区 + 输入行 ⇒ 一定比收起态高。
///
/// 这条能抓到「条件分支走错 / 内容区没渲染」——只断言不 panic 抓不到。
#[test]
fn terminal_drawer_expanded_is_taller_than_collapsed() {
    let collapsed = draw_probe(|ctx| ctx.new(|_cx| TerminalDrawerGpui::new()).into_any_element());
    let expanded = draw_probe(|ctx| {
        ctx.new(|_cx| {
            let mut view = TerminalDrawerGpui::new();
            view.state_mut().toggle_expanded();
            view
        })
        .into_any_element()
    });

    let collapsed = assert_rendered(collapsed, "终端抽屉（收起）");
    let expanded = assert_rendered(expanded, "终端抽屉（展开）");
    assert!(
        expanded.size.height > collapsed.size.height,
        "展开态没有比收起态高 ⇒ 内容区没渲染出来（expanded={:?} collapsed={:?}）",
        expanded.size.height,
        collapsed.size.height
    );
}
