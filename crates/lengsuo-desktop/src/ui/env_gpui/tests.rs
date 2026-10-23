//! `ui::env_gpui` 的单测（原 `env_gpui.rs` 第 477–668 行，逻辑一字未改）。

use super::mapping::{
    active_group_row, demo_state, empty_state_text, filter_key, main_pane, select_card,
    set_group_filter, status_tone, MainPane, StatusTone,
};
use crate::state::env_group::{EnvGroupState, GroupFilter};
use crate::ui::env_view::{self, CardStatus};
use serde_json::json;

fn state_with(entries: Vec<serde_json::Value>) -> EnvGroupState {
    let mut state = EnvGroupState::new();
    state.apply_entries(entries);
    state
}

fn active_label(view: &env_view::EnvView) -> Option<&str> {
    active_group_row(view).map(|row| row.label.as_str())
}

#[test]
fn three_states_map_to_three_distinct_colors() {
    assert_eq!(status_tone(CardStatus::Ready), StatusTone::Success);
    assert_eq!(status_tone(CardStatus::Degraded), StatusTone::Warning);
    assert_eq!(status_tone(CardStatus::Failed), StatusTone::Danger);
    assert_ne!(
        status_tone(CardStatus::Ready),
        status_tone(CardStatus::Degraded)
    );
    assert_ne!(
        status_tone(CardStatus::Degraded),
        status_tone(CardStatus::Failed)
    );
}

#[test]
fn ready_state_renders_cards_without_a_notice() {
    let view = env_view::render(&state_with(vec![json!({"id": "e1", "name": "甲"})]));
    assert_eq!(main_pane(&view), MainPane::Ready);
    assert_eq!(view.degraded_notice, None);
    assert_eq!(empty_state_text(&view), None, "有卡片时不得显示空态文案");
    assert_eq!(view.cards[0].status, CardStatus::Ready);
}

#[test]
fn degraded_state_keeps_a_notice_and_cards_are_marked_degraded() {
    let view = env_view::render(&state_with(vec![
        json!({"id": "e1", "name": "甲"}),
        json!({"name": "坏条目"}),
    ]));
    assert_eq!(main_pane(&view), MainPane::Ready);
    assert_eq!(view.degraded_notice.as_deref(), Some("有 1 条数据读取异常"));
    assert_eq!(view.cards[0].status, CardStatus::Degraded);
    assert_eq!(status_tone(view.cards[0].status), StatusTone::Warning);
}

#[test]
fn failed_state_is_visible_and_never_becomes_an_empty_state() {
    let mut state = state_with(vec![json!({"id": "e1", "name": "甲"})]);
    state.fail("gateway unreachable");
    let view = env_view::render(&state);

    assert_eq!(main_pane(&view), MainPane::Failed);
    assert_eq!(view.error.as_deref(), Some("gateway unreachable"));
    assert_eq!(
        empty_state_text(&view),
        None,
        "失败态不得显示空态文案（那是谎称没有数据）"
    );
    assert!(view.cards.is_empty());
}

#[test]
fn loading_state_is_flagged_and_keeps_the_existing_cards() {
    let mut state = state_with(vec![json!({"id": "e1", "name": "甲"})]);
    state.begin_load();
    let view = env_view::render(&state);

    assert_eq!(main_pane(&view), MainPane::Loading);
    assert_eq!(view.cards.len(), 1, "加载中不清空卡片，避免闪空");
    assert_eq!(empty_state_text(&view), None);
}

#[test]
fn all_invalid_data_shows_a_notice_but_no_empty_state() {
    let view = env_view::render(&state_with(vec![json!({"name": "坏条目"})]));
    assert_eq!(main_pane(&view), MainPane::Ready);
    assert_eq!(view.degraded_notice.as_deref(), Some("有 1 条数据读取异常"));
    assert_eq!(
        empty_state_text(&view),
        None,
        "全部解析失败时只能显示降级提示，不得显示「暂无环境」"
    );
}

#[test]
fn filtered_empty_and_real_empty_use_different_texts() {
    let empty = env_view::render(&state_with(vec![]));
    assert_eq!(empty_state_text(&empty), Some("暂无环境"));

    let mut filtered = state_with(vec![json!({"id": "e1", "name": "甲"})]);
    filtered.set_keyword("不存在的关键词");
    let view = env_view::render(&filtered);
    assert_eq!(empty_state_text(&view), Some("没有符合筛选条件的环境"));
    assert_ne!(empty_state_text(&view), empty_state_text(&empty));
}

#[test]
fn the_active_group_row_follows_the_filter() {
    let mut state = state_with(vec![
        json!({"id": "e1", "name": "甲", "group_id": "g1", "group_name": "华东组"}),
        json!({"id": "e2", "name": "乙"}),
    ]);

    let view = env_view::render(&state);
    assert_eq!(active_label(&view), Some("全部环境"));

    state.set_group_filter(GroupFilter::Group("g1".into()));
    let view = env_view::render(&state);
    assert_eq!(active_label(&view), Some("华东组"));

    state.set_group_filter(GroupFilter::Ungrouped);
    let view = env_view::render(&state);
    assert_eq!(
        active_label(&view),
        Some("未分组"),
        "「未分组」是合法分组，必须能成为主区标题"
    );
}

#[test]
fn the_sidebar_never_hides_the_ungrouped_row() {
    let view = env_view::render(&state_with(vec![json!({"id": "e1", "name": "甲"})]));
    let labels: Vec<&str> = view.sidebar.iter().map(|row| row.label.as_str()).collect();
    assert!(labels.contains(&"未分组"), "「未分组」不得被隐藏");
    assert!(labels.contains(&"全部环境"));
}

#[test]
fn filter_keys_are_stable_and_distinct() {
    assert_eq!(filter_key(&GroupFilter::All), "all");
    assert_eq!(filter_key(&GroupFilter::Ungrouped), "ungrouped");
    assert_eq!(filter_key(&GroupFilter::Group("g1".into())), "group-g1");
    assert_ne!(
        filter_key(&GroupFilter::Group("g1".into())),
        filter_key(&GroupFilter::Group("g2".into()))
    );
}

#[test]
fn clicking_a_card_writes_the_selection() {
    let mut state = demo_state();
    assert!(select_card(&mut state, "demo-3"));
    assert_eq!(state.selected(), Some("demo-3"));
    assert_eq!(env_view::render(&state).selected.as_deref(), Some("demo-3"));
}

#[test]
fn clicking_an_unknown_card_changes_nothing() {
    let mut state = demo_state();
    assert!(!select_card(&mut state, "不存在的 id"));
    assert_eq!(state.selected(), None);
}

#[test]
fn clicking_a_group_row_switches_the_filter_once() {
    let mut state = demo_state();
    assert!(set_group_filter(&mut state, GroupFilter::Ungrouped));
    assert_eq!(state.group_filter(), &GroupFilter::Ungrouped);
    assert!(
        !set_group_filter(&mut state, GroupFilter::Ungrouped),
        "点同一行不应重复触发"
    );

    let view = env_view::render(&state);
    assert_eq!(view.cards.len(), 1, "未分组下只剩未分组的条目");
    assert_eq!(view.cards[0].id, "demo-4");
}

#[test]
fn demo_state_has_grouped_and_ungrouped_entries() {
    let state = demo_state();
    assert_eq!(state.entries().len(), 4);
    assert_eq!(state.groups().len(), 2);
    assert_eq!(state.ungrouped_count(), 1);
    assert_eq!(state.invalid_count(), 0);

    let view = env_view::render(&state);
    assert_eq!(view.cards.len(), 4);
    assert_eq!(view.total_count, 4);
}
