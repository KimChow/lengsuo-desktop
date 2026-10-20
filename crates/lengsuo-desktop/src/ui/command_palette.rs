//! 命令面板浮层：输入框 + 过滤结果列表 + 键盘交互。
//!
//! 与其余 `ui` 模块一致，本模块只产出**渲染描述**（[`CommandPaletteView`]），
//! 不依赖 GPUI，便于在无窗口环境下测试交互语义。
//!
//! 硬约定：
//! - 列表只展示命令的 `title`，**不展示、不渲染 id**；派发由状态层按 id 完成。
//! - 上下选择、回车执行、Escape 关闭；边界不越界（由 [`PaletteState`] 保证）。
//! - 空白结果给出显式提示，不静默显示空列表。
//!
//! 真实全局快捷键注册是打包时宿主的职责：宿主用
//! [`crate::state::hotkeys::HotkeyRegistry`] 匹配按键，命中
//! `palette.open` 后调用 [`PaletteState::open`]。

use crate::state::hotkeys::{self, Hotkey, HotkeyRegistry};
use crate::state::palette_state::PaletteState;

/// 输入框占位文案。
pub const PLACEHOLDER: &str = "搜索命令";
/// 无匹配结果时的提示。
pub const EMPTY_TEXT: &str = "没有匹配的命令";
/// 底部键盘提示。
pub const FOOTER_HINT: &str = "↑↓ 选择 · Enter 执行 · Esc 关闭";

/// 列表中的一行。只承载展示所需字段：**没有 id**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteRow {
    /// 命令标题（展示用）。
    pub title: &'static str,
    /// 是否为当前选中项。
    pub selected: bool,
}

/// 命令面板浮层的渲染描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteView {
    /// 顶部输入框内容。
    pub query: String,
    pub placeholder: &'static str,
    /// 过滤后的命令标题列表。
    pub rows: Vec<PaletteRow>,
    /// 结果为空时的提示文案；非空为 `None`。
    pub empty_text: Option<&'static str>,
    pub footer: &'static str,
}

/// 由状态生成浮层描述；面板关闭时返回 `None`（不渲染浮层）。
#[must_use]
pub fn render(state: &PaletteState) -> Option<CommandPaletteView> {
    if !state.is_open() {
        return None;
    }
    let selected = state.selected_index();
    let rows: Vec<PaletteRow> = state
        .filtered()
        .iter()
        .enumerate()
        .map(|(index, command)| PaletteRow {
            title: command.title,
            selected: selected == Some(index),
        })
        .collect();
    Some(CommandPaletteView {
        query: state.keyword().to_owned(),
        placeholder: PLACEHOLDER,
        empty_text: rows.is_empty().then_some(EMPTY_TEXT),
        rows,
        footer: FOOTER_HINT,
    })
}

/// 面板打开时接收的按键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteKey {
    Up,
    Down,
    Enter,
    Escape,
    Backspace,
    Char(char),
}

/// 一次按键处理的结果，供宿主决定重绘 / 派发 / 关闭。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    /// 按键被忽略（面板关闭时，或没有可执行项时按 Enter）。
    Ignored,
    /// 状态已改变，需要重绘。
    Redraw,
    /// 执行命令：按 **id** 派发，面板已关闭。
    Execute(&'static str),
    /// 关闭浮层。
    Close,
    /// 经全局快捷键打开面板。
    Open,
}

/// 处理一次按键。
///
/// 面板关闭时一律返回 [`PaletteAction::Ignored`]，不吞掉全局按键。
pub fn handle_key(state: &mut PaletteState, key: PaletteKey) -> PaletteAction {
    if !state.is_open() {
        return PaletteAction::Ignored;
    }
    match key {
        PaletteKey::Up => {
            state.select_previous();
            PaletteAction::Redraw
        }
        PaletteKey::Down => {
            state.select_next();
            PaletteAction::Redraw
        }
        PaletteKey::Enter => match state.execute() {
            Some(command_id) => PaletteAction::Execute(command_id),
            None => PaletteAction::Ignored,
        },
        PaletteKey::Escape => {
            state.handle_escape();
            PaletteAction::Close
        }
        PaletteKey::Backspace => {
            if state.backspace() {
                PaletteAction::Redraw
            } else {
                PaletteAction::Ignored
            }
        }
        PaletteKey::Char(c) => {
            state.insert_char(c);
            PaletteAction::Redraw
        }
    }
}

/// 处理一次**全局**快捷键，完成快捷键注册表与命令面板的接线。
///
/// 只有 `palette.open` 与 `overlay.close` 归本面板处理；其余命令返回
/// [`PaletteAction::Ignored`]，由宿主继续按 id 派发。
#[must_use]
pub fn handle_hotkey(
    state: &mut PaletteState,
    registry: &HotkeyRegistry,
    event: Hotkey,
) -> PaletteAction {
    match registry.command_for(event) {
        Some(hotkeys::command_ids::PALETTE_OPEN) => {
            state.open();
            PaletteAction::Open
        }
        Some(hotkeys::command_ids::OVERLAY_CLOSE) => {
            if state.handle_escape() {
                PaletteAction::Close
            } else {
                PaletteAction::Ignored
            }
        }
        _ => PaletteAction::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::palette::COMMANDS;

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
    fn open_palette_shows_input_and_all_titles() {
        let state = opened();
        let view = render(&state).expect("打开时应渲染浮层");
        assert_eq!(view.query, "");
        assert_eq!(view.placeholder, PLACEHOLDER);
        assert_eq!(view.rows.len(), COMMANDS.len());
        assert_eq!(view.empty_text, None);
        assert_eq!(view.footer, FOOTER_HINT);
        assert!(view.rows[0].selected, "打开时选中第一项");
        assert!(view.rows[1..].iter().all(|row| !row.selected));
    }

    #[test]
    fn rows_show_titles_and_never_ids() {
        let mut state = opened();
        state.set_keyword("会话");
        let view = render(&state).unwrap();
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].title, "新建会话");
        for row in &view.rows {
            assert!(
                !COMMANDS.iter().any(|command| command.id == row.title),
                "渲染行不得显示命令 id：{}",
                row.title
            );
        }
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
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].title, "设置");

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
        assert_eq!(render(&state).unwrap().rows.len(), COMMANDS.len());
    }

    #[test]
    fn arrow_keys_move_selection_within_bounds() {
        let mut state = opened();
        assert_eq!(
            handle_key(&mut state, PaletteKey::Up),
            PaletteAction::Redraw
        );
        assert_eq!(state.selected_index(), Some(0), "首项上移不变");
        for _ in 0..COMMANDS.len() + 2 {
            handle_key(&mut state, PaletteKey::Down);
        }
        assert_eq!(
            state.selected_index(),
            Some(COMMANDS.len() - 1),
            "末项下移不变"
        );
        let view = render(&state).unwrap();
        assert!(view.rows.last().unwrap().selected);
        assert!(view.rows[..COMMANDS.len() - 1]
            .iter()
            .all(|row| !row.selected));
    }

    #[test]
    fn enter_executes_selected_id_and_closes() {
        let mut state = opened();
        handle_key(&mut state, PaletteKey::Char('设'));
        handle_key(&mut state, PaletteKey::Char('置'));
        let action = handle_key(&mut state, PaletteKey::Enter);
        assert_eq!(action, PaletteAction::Execute("app.settings"));
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
        assert!(view_before.rows.is_empty());
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
        assert_eq!(view.rows.len(), COMMANDS.len());
    }

    #[test]
    fn hotkeys_open_and_close_the_panel() {
        let registry = HotkeyRegistry::with_defaults().expect("内置快捷键表必须无冲突");
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
        let registry = HotkeyRegistry::with_defaults().unwrap();
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
        let mut registry = HotkeyRegistry::with_defaults().unwrap();
        registry
            .apply_overrides(&[crate::state::hotkeys::HotkeyOverride::parse(
                "palette.open",
                "Ctrl+P",
            )
            .unwrap()])
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
}
