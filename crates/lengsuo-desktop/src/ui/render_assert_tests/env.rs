//! B 段：环境视图 Entity 形态 —— 就绪 / 空态 / 失败态三条渲染分支。

use super::{assert_rendered, draw_probe};
use gpui_kit::{AppContext as _, IntoElement as _};

use crate::state::env_group::EnvGroupState;
use crate::ui::env_gpui::EnvViewGpui;

fn env_state_with_cards() -> EnvGroupState {
    let mut state = EnvGroupState::new();
    state.apply_entries(vec![
        serde_json::json!({"id": "e1", "name": "构建机 A", "group_id": "g1", "group_name": "华东组"}),
        serde_json::json!({"id": "e2", "name": "构建机 B"}),
    ]);
    state
}

#[test]
fn env_view_ready_state_renders() {
    let bounds = draw_probe(|ctx| {
        let state = env_state_with_cards();
        ctx.new(|_cx| EnvViewGpui::new(state)).into_any_element()
    });
    assert_rendered(bounds, "环境视图（就绪）");
}

#[test]
fn env_view_empty_state_renders() {
    let bounds = draw_probe(|ctx| {
        ctx.new(|_cx| EnvViewGpui::new(EnvGroupState::new()))
            .into_any_element()
    });
    assert_rendered(bounds, "环境视图（空态）");
}

#[test]
fn env_view_failed_state_renders_error_banner() {
    let bounds = draw_probe(|ctx| {
        let mut state = EnvGroupState::new();
        state.fail("gateway unreachable");
        ctx.new(|_cx| EnvViewGpui::new(state)).into_any_element()
    });
    assert_rendered(bounds, "环境视图（失败态）");
}
