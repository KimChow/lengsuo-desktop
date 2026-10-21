//! 命令面板描述层的单元测试。
//!
//! 显式导入符号而**不用** `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的
//! `test` 属性宏带进来并遮蔽标准库的 `#[test]`（速查表 §12.3）。
#![cfg(test)]

use crate::state::hotkeys::Hotkey;
use crate::state::palette_state::{demo_items, PaletteState};
use crate::theme::palette::COMMANDS;
use crate::ui::command_palette::search::SearchSource;
use crate::ui::command_palette::{
    handle_hotkey, handle_key, render, PaletteAction, PaletteKey, EMPTY_TEXT, FOOTER_HINT,
    PLACEHOLDER,
};

fn opened() -> PaletteState {
    let mut state = PaletteState::new();
    state.open();
    state
}

#[test]
fn closed_palette_renders_nothing() {
    let state = PaletteState::new();
    assert_eq!(render(&state), None);
}

#[test]
fn open_palette_shows_input_and_all_groups() {
    let state = opened();
    let view = render(&state).expect("打开时应渲染浮层");
    assert_eq!(view.query, "");
    assert_eq!(view.placeholder, PLACEHOLDER);
    assert_eq!(view.footer, FOOTER_HINT);
    assert_eq!(view.empty_text, None);
    assert_eq!(view.groups.len(), SearchSource::ALL.len());
    let total: usize = view.groups.iter().map(|g| g.items.len()).sum();
    assert_eq!(total, demo_items().len());
}

#[test]
fn groups_show_titles_and_never_ids() {
    let mut state = opened();
    state.set_keyword("会话");
    let view = render(&state).unwrap();
    let titles: Vec<&str> = view
        .groups
        .iter()
        .flat_map(|g| g.items.iter())
        .map(|item| item.title.as_str())
        .collect();
    assert!(titles.contains(&"新建会话"));
    for title in &titles {
        assert!(
            !COMMANDS.iter().any(|command| command.id == *title),
            "渲染行不得显示命令 id：{}",
            title
        );
    }
}

#[test]
fn selection_is_reported_by_id() {
    let state = opened();
    let view = render(&state).unwrap();
    assert_eq!(
        view.selected_id.as_deref(),
        Some(demo_items()[0].id.as_str()),
        "打开时选中第一项，且按 id 报出"
    );
}

#[test]
fn typing_filters_and_backspace_restores() {
    let mut state = opened();
    assert_eq!(
        handle_key(&mut state, PaletteKey::Char('设')),
        PaletteAction::Redraw
    );
    handle_key(&mut state, PaletteKey::Char('置'));
    assert_eq!(state.keyword(), "设置");
    let view = render(&state).unwrap();
    assert_eq!(view.query, "设置");
    let total: usize = view.groups.iter().map(|g| g.items.len()).sum();
    assert_eq!(total, 1);
    assert_eq!(view.groups[0].items[0].title, "设置");

    assert_eq!(
        handle_key(&mut state, PaletteKey::Backspace),
        PaletteAction::Redraw
    );
    assert_eq!(
        handle_key(&mut state, PaletteKey::Backspace),
        PaletteAction::Redraw
    );
    assert_eq!(
        handle_key(&mut state, PaletteKey::Backspace),
        PaletteAction::Ignored,
        "空关键词时退格无变化"
    );
    let restored: usize = render(&state)
        .unwrap()
        .groups
        .iter()
        .map(|g| g.items.len())
        .sum();
    assert_eq!(restored, demo_items().len());
}

#[test]
fn arrow_keys_move_selection_within_bounds() {
    let mut state = opened();
    assert_eq!(
        handle_key(&mut state, PaletteKey::Up),
        PaletteAction::Redraw
    );
    assert_eq!(state.selected_index(), Some(0), "首项上移不变");
    let count = demo_items().len();
    for _ in 0..count + 2 {
        handle_key(&mut state, PaletteKey::Down);
    }
    assert_eq!(state.selected_index(), Some(count - 1), "末项下移不变");
}

#[test]
fn enter_executes_selected_id_and_closes() {
    let mut state = opened();
    handle_key(&mut state, PaletteKey::Char('设'));
    handle_key(&mut state, PaletteKey::Char('置'));
    let action = handle_key(&mut state, PaletteKey::Enter);
    assert_eq!(action, PaletteAction::Execute("app.settings".to_owned()));
    assert!(!state.is_open());
    assert_eq!(render(&state), None);
}

#[test]
fn enter_with_no_match_is_ignored_and_keeps_panel_open() {
    let mut state = opened();
    for c in "zzz".chars() {
        handle_key(&mut state, PaletteKey::Char(c));
    }
    let view_before = render(&state).unwrap();
    assert_eq!(view_before.empty_text, Some(EMPTY_TEXT));
    assert!(view_before.groups.is_empty());
    assert_eq!(
        handle_key(&mut state, PaletteKey::Enter),
        PaletteAction::Ignored
    );
    assert!(state.is_open(), "无可执行项时不得关闭面板");
}

#[test]
fn escape_closes_panel() {
    let mut state = opened();
    handle_key(&mut state, PaletteKey::Char('a'));
    assert_eq!(
        handle_key(&mut state, PaletteKey::Escape),
        PaletteAction::Close
    );
    assert!(!state.is_open());
    assert_eq!(
        handle_key(&mut state, PaletteKey::Escape),
        PaletteAction::Ignored,
        "关闭后不得吞掉按键"
    );
}

#[test]
fn closed_palette_ignores_all_keys() {
    let mut state = PaletteState::new();
    for key in [
        PaletteKey::Up,
        PaletteKey::Down,
        PaletteKey::Enter,
        PaletteKey::Escape,
        PaletteKey::Backspace,
        PaletteKey::Char('k'),
    ] {
        assert_eq!(handle_key(&mut state, key), PaletteAction::Ignored);
    }
    assert_eq!(state.keyword(), "");
}

#[test]
fn reopening_clears_query_in_view() {
    let mut state = opened();
    handle_key(&mut state, PaletteKey::Char('会'));
    handle_key(&mut state, PaletteKey::Char('话'));
    assert_eq!(render(&state).unwrap().query, "会话");
    state.close();
    state.open();
    let view = render(&state).unwrap();
    assert_eq!(view.query, "", "再次打开时输入框必须为空");
    assert_eq!(view.groups.len(), SearchSource::ALL.len());
}

#[test]
fn hotkeys_open_and_close_the_panel() {
    let registry =
        crate::state::hotkeys::HotkeyRegistry::with_defaults().expect("内置快捷键表必须无冲突");
    let mut state = PaletteState::new();

    let ctrl_k = Hotkey::parse("Ctrl+K").unwrap();
    assert_eq!(
        handle_hotkey(&mut state, &registry, ctrl_k),
        PaletteAction::Open
    );
    assert!(state.is_open());

    let escape = Hotkey::parse("Escape").unwrap();
    assert_eq!(
        handle_hotkey(&mut state, &registry, escape),
        PaletteAction::Close
    );
    assert!(!state.is_open());

    let ctrl_shift_p = Hotkey::parse("Ctrl+Shift+P").unwrap();
    assert_eq!(
        handle_hotkey(&mut state, &registry, ctrl_shift_p),
        PaletteAction::Open
    );
    assert!(state.is_open());
}

#[test]
fn unrelated_hotkeys_are_left_to_the_host() {
    let registry = crate::state::hotkeys::HotkeyRegistry::with_defaults().unwrap();
    let mut state = PaletteState::new();
    let ctrl_s = Hotkey::parse("Ctrl+S").unwrap();
    assert_eq!(
        handle_hotkey(&mut state, &registry, ctrl_s),
        PaletteAction::Ignored,
        "Ctrl+S 是保存文件，不属于命令面板"
    );
    assert!(!state.is_open());
}

#[test]
fn user_rebinding_is_reflected_in_hotkey_wiring() {
    let mut registry = crate::state::hotkeys::HotkeyRegistry::with_defaults().unwrap();
    registry
        .apply_overrides(&[
            crate::state::hotkeys::HotkeyOverride::parse("palette.open", "Ctrl+P").unwrap(),
        ])
        .unwrap();
    let mut state = PaletteState::new();

    let old = Hotkey::parse("Ctrl+K").unwrap();
    assert_eq!(
        handle_hotkey(&mut state, &registry, old),
        PaletteAction::Ignored,
        "改键后旧快捷键不应再打开面板"
    );
    let new = Hotkey::parse("Ctrl+P").unwrap();
    assert_eq!(
        handle_hotkey(&mut state, &registry, new),
        PaletteAction::Open
    );
    assert!(state.is_open());
}
