//! 命令面板接线层的单元测试。
//!
//! 显式导入符号而**不用** `use super::*`：`mod.rs` 的 `use gpui_kit::*` 会把 GPUI 的
//! `test` 属性宏带进来并遮蔽标准库的 `#[test]`（速查表 §12.3）。
#![cfg(test)]

use crate::state::palette_state::PaletteState;
use crate::ui::command_palette::{self, PaletteKey};
// 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
// `test` 属性宏带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
use crate::ui::palette_gpui::input_box::{self, PanelSync, PanelSyncState};

#[test]
fn panel_stays_closed_until_opened() {
    let mut state = PaletteState::new();
    assert!(!state.is_open());
    assert!(command_palette::render(&state).is_none());
    state.open();
    let view = command_palette::render(&state).expect("打开后应有描述");
    assert_eq!(view.query, "", "打开时关键词必须为空");
    assert_eq!(view.rows.len(), state.filtered().len());
}

#[test]
fn selection_stops_at_the_edges() {
    let mut state = PaletteState::new();
    state.open();
    command_palette::handle_key(&mut state, PaletteKey::Up);
    assert_eq!(state.selected_index(), Some(0), "首项上移不变");
    for _ in 0..state.filtered().len() + 2 {
        command_palette::handle_key(&mut state, PaletteKey::Down);
    }
    assert_eq!(
        state.selected_index(),
        Some(state.filtered().len() - 1),
        "末项下移不变"
    );
}

// —— 搜索框 ↔ 关键词 双向同步（纯逻辑，不建窗口）——

#[test]
fn search_box_text_reaches_the_keyword_character_by_character() {
    let mut state = PaletteState::new();
    state.open();
    let mut sync = PanelSyncState::default();

    // IME 提交后编辑器里一次出现的可能是一整段文字：按 char 追加，绝不按字节切。
    assert!(sync.accept_search_box(&mut state, "设"));
    assert!(sync.accept_search_box(&mut state, "设置"));
    assert_eq!(state.keyword(), "设置");

    // 一次粘进多字节字符串同样按字符追加。
    assert!(sync.accept_search_box(&mut state, "设置项"));
    assert_eq!(state.keyword(), "设置项");
}

#[test]
fn external_clear_is_written_back_to_the_search_box() {
    let mut state = PaletteState::new();
    state.open();
    let mut sync = PanelSyncState::default();
    sync.accept_search_box(&mut state, "设置");
    assert_eq!(
        sync.pull_into_search_box(&state, "设置"),
        None,
        "一致时不写回"
    );

    // 外部（宿主 / 打开面板）清空关键词 ⇒ 回写路径给出空串。
    state.open();
    assert_eq!(state.keyword(), "");
    assert_eq!(
        sync.pull_into_search_box(&state, "设置"),
        Some(String::new())
    );
    sync.accept_search_box(&mut state, "");
    assert_eq!(sync.pull_into_search_box(&state, ""), None);
}

#[test]
fn ime_input_filters_like_the_description_layer() {
    let mut state = PaletteState::new();
    state.open();
    let mut sync = PanelSyncState::default();
    // 真实路径：编辑器先拿到字符，再发 `Change`；关键词只能来自写回。
    let mut typed = String::new();
    for c in "设置".chars() {
        typed.push(c);
        assert!(sync.accept_search_box(&mut state, &typed));
        assert_eq!(state.keyword(), typed, "关键词必须逐字跟随编辑器");
    }
    let view = command_palette::render(&state).expect("面板打开");
    assert_eq!(view.query, "设置");
    assert_eq!(view.rows.len(), state.filtered().len());
    assert_eq!(state.selected_id(), Some("app.settings"));
}

#[test]
fn deleting_multibyte_text_never_leaves_half_a_character() {
    let mut state = PaletteState::new();
    state.open();
    let mut sync = PanelSyncState::default();
    for text in ["设置", "设"] {
        assert!(sync.accept_search_box(&mut state, text));
        assert!(state.keyword().chars().all(|c| c == '设' || c == '置'));
    }
    // 编辑器按 char 删除后给出的新值，关键词完全跟随。
    assert!(sync.accept_search_box(&mut state, ""));
    assert_eq!(state.keyword(), "");
    assert_eq!(
        input_box::diff_input("设置", "设"),
        input_box::TextEdit::Delete {
            start: 1,
            removed: "置".to_owned()
        }
    );
}

#[test]
fn diff_input_keeps_char_boundaries() {
    assert_eq!(
        input_box::diff_input("", "设"),
        input_box::TextEdit::Insert {
            at: 0,
            text: "设".to_owned()
        }
    );
    assert_eq!(
        input_box::diff_input("设置", "设置面板"),
        input_box::TextEdit::Insert {
            at: 2,
            text: "面板".to_owned()
        }
    );
}
