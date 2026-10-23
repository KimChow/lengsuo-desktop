//! 接线层测试：描述层契约 + 接线层纯函数的回归。
//!
//! ⛔ 不在这里再包 `mod tests {}`（clippy `module_inception` 会拦）。
//! ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test` 属性宏
//! 带进作用域并遮蔽标准库的 `#[test]`。
//![cfg(all(test, feature = "ui"))]

use crate::ui::input_area::{InputAreaState, InputMode};
use crate::ui::input_gpui::chips::chip_row;
use crate::ui::input_gpui::logic::{mention_hits, mode_options, MAX_MENTION_HITS};
use crate::ui::input_gpui::{apply_mode, mode_buttons, take_outgoing};

#[test]
fn mode_bar_renders_five_modes_with_exactly_one_selected() {
    let buttons = mode_buttons(InputMode::Ask);
    assert_eq!(buttons.len(), 5);
    assert_eq!(buttons.map(|b| b.mode), InputMode::ALL);
    assert_eq!(
        buttons.map(|b| b.label),
        ["快速", "问答", "规划", "执行", "专家"]
    );
    let selected: Vec<InputMode> = buttons
        .iter()
        .filter(|b| b.selected)
        .map(|b| b.mode)
        .collect();
    assert_eq!(selected, vec![InputMode::Ask]);
}

#[test]
fn switching_mode_updates_state_and_highlight() {
    let mut state = InputAreaState::new();
    assert_eq!(state.mode(), InputMode::Ask);
    for mode in InputMode::ALL {
        assert!(apply_mode(&mut state, mode), "切换到 {mode:?} 应生效");
        assert_eq!(state.mode(), mode);
        let selected: Vec<InputMode> = mode_buttons(state.mode())
            .iter()
            .filter(|b| b.selected)
            .map(|b| b.mode)
            .collect();
        assert_eq!(selected, vec![mode], "高亮必须跟着切换");
    }
    assert!(
        !apply_mode(&mut state, InputMode::Expert),
        "重复点击不算变化"
    );
    assert_eq!(state.mode(), InputMode::Expert);
}

#[test]
fn blank_draft_is_never_produced_for_delivery() {
    let mut state = InputAreaState::new();
    assert!(take_outgoing(&mut state).is_none());
    state.set_draft("   \n\t ");
    assert!(take_outgoing(&mut state).is_none(), "纯空白也不得产出");
    assert_eq!(state.draft(), "   \n\t ", "未发送则草稿保持原样");
}

#[test]
fn in_flight_blocks_resending() {
    let mut state = InputAreaState::new();
    state.set_mode(InputMode::Plan);
    state.set_draft("你好\n世界");
    let outgoing = take_outgoing(&mut state).expect("非空草稿应产出");
    assert_eq!(outgoing.text, "你好\n世界");
    assert_eq!(outgoing.mode, InputMode::Plan);
    assert!(state.is_in_flight());
    assert!(
        take_outgoing(&mut state).is_none(),
        "等待确认期间不得重复产出"
    );
    state.finish_send();
    assert!(take_outgoing(&mut state).is_none(), "草稿已清空");
}

#[test]
fn stop_intent_is_produced_only_in_waiting_state() {
    let mut state = InputAreaState::new();
    state.set_mode(InputMode::Craft);
    assert!(state.take_stop().is_none(), "未发送时不得产出停止意图");
    state.set_draft("跑起来");
    take_outgoing(&mut state).expect("应可发送");
    let intent = state.take_stop().expect("等待态应产出停止意图");
    assert_eq!(intent.mode, InputMode::Craft);
    assert!(state.take_stop().is_none(), "停止意图只产出一次");
}

#[test]
fn mode_options_render_five_entries_with_unique_ids() {
    let options = mode_options(InputMode::Plan);
    assert_eq!(options.len(), 5);
    assert_eq!(
        options.iter().map(|o| o.label).collect::<Vec<_>>(),
        ["快速", "问答", "规划", "执行", "专家"]
    );
    let selected: Vec<InputMode> = options
        .iter()
        .filter(|o| o.selected)
        .map(|o| o.mode)
        .collect();
    assert_eq!(selected, vec![InputMode::Plan]);

    let mut ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "同屏 id 不得重复");
    assert!(ids.iter().all(|id| id.starts_with("input-mode-option-")));
}

#[test]
fn mention_hits_filter_by_at_query_and_cap_the_list() {
    let mut state = InputAreaState::new();
    state.set_candidates(
        (0..10)
            .map(|i| format!("src/file{i}.rs"))
            .chain(["docs/readme.md".to_owned()]),
    );

    // 没有 @ 标记 ⇒ 不出层。
    state.set_draft("普通输入");
    assert!(mention_hits(&state).is_empty(), "没有 @ 不得出层");

    // 有 @ 标记但没命中 ⇒ 不出层。
    state.set_draft("@不存在的路径");
    assert!(mention_hits(&state).is_empty(), "没命中不得出层");

    // 命中 ⇒ 最多 MAX_MENTION_HITS 条，id 唯一。
    state.set_draft("@file");
    let hits = mention_hits(&state);
    assert_eq!(hits.len(), MAX_MENTION_HITS, "命中数必须封顶");
    assert!(hits.iter().all(|h| h.path.starts_with("src/file")));
    let mut ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "同屏 id 不得重复");
}

#[test]
fn mention_hits_match_the_candidate_filter() {
    let mut state = InputAreaState::new();
    state.set_candidates([
        "src/Main.rs".to_owned(),
        "docs/readme.md".to_owned(),
        "src/lib.rs".to_owned(),
    ]);
    state.set_draft("@readme");
    let hits = mention_hits(&state);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "docs/readme.md");
    assert_eq!(hits[0].id, "input-mention-option-docs/readme.md");
}

// ==== 附件 chip（A12 附件族）====

#[test]
fn empty_attachments_render_no_chip_row() {
    let state = InputAreaState::new();
    assert!(
        chip_row(&state).is_none(),
        "空列表不得渲染 chip 行（不留空壳）"
    );
}

#[test]
fn chip_row_lists_labels_and_unique_ids_in_insertion_order() {
    let mut state = InputAreaState::new();
    state.set_attachments([
        "docs/wb/截图.png".to_owned(),
        "img/粘贴.png".to_owned(),
        "docs/wb/截图.png".to_owned(),
    ]);
    let chips = chip_row(&state).expect("有附件必须出 chip 行");
    assert_eq!(
        chips.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
        ["截图.png", "粘贴.png"],
        "标签取末段文件名，插入序不变，重复已去"
    );
    assert_eq!(chips[0].path, "docs/wb/截图.png", "移除回传完整路径");
    assert_eq!(chips[0].id, "input-attachment-chip-docs/wb/截图.png");
    assert_eq!(
        chips[0].remove_id,
        "input-attachment-remove-docs/wb/截图.png"
    );
    let mut ids: Vec<&str> = chips
        .iter()
        .flat_map(|c| [c.id.as_str(), c.remove_id.as_str()])
        .collect();
    ids.sort_unstable();
    let before = ids.len();
    ids.dedup();
    assert_eq!(before, ids.len(), "同屏 id 不得重复");
}

#[test]
fn removing_the_last_attachment_collapses_the_row() {
    let mut state = InputAreaState::new();
    state.set_attachments(["img/only.png".to_owned()]);
    assert!(chip_row(&state).is_some());
    assert!(state.remove_attachment("img/only.png"));
    assert!(
        chip_row(&state).is_none(),
        "全部移除后 chip 行必须从元素树消失"
    );
}
