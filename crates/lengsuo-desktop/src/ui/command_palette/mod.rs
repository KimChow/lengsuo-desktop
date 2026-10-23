//! 命令面板浮层：输入框 + **多来源搜索结果**（按来源分组）+ 键盘交互。
//!
//! 与其余 `ui` 模块一致，本模块只产出**渲染描述**（[`CommandPaletteView`]），
//! 不依赖 GPUI，便于在无窗口环境下测试交互语义。
//!
//! 硬约定：
//! - 搜索来源有五类：**命令 / 技能 / MCP / 文件 / 会话**（[`SearchSource`]）。
//!   ⛔ **数据只来自宿主注入**：命令来自内置命令表（真实数据）；其余四类的
//!   **真实数据源在运行时侧**，本仓不实现 ⇒ **没注入就没有该分组**，
//!   **绝不显示假条目**（fail-closed）。
//! - 结果**按来源分组**展示，空分组不出现。
//! - 列表只展示标题，**不展示 id**；派发由状态层按 id 完成。
//! - 上下选择、回车执行、Escape 关闭；边界不越界（由 [`PaletteState`] 保证）。
//! - 空白结果给出显式提示，不静默显示空列表。
//!
//! 真实全局快捷键注册是打包时宿主的职责：宿主用
//! [`crate::state::hotkeys::HotkeyRegistry`] 匹配按键，命中
//! `palette.open` 后调用 [`PaletteState::open`]。

/// 描述层门面的**测试视图**（e20 留下的 `ui/search.rs`）。
///
/// ⛔ 只在测试构建里存在：它无条件 re-export 了演示条目集合，而演示条目是
/// `#[cfg(test)]` 符号 ⇒ 生产构建不能引它。生产路径的形状入口在
/// [`crate::state::palette_state::search_state`]。
#[cfg(test)]
#[path = "../search.rs"]
pub mod search;

use crate::state::hotkeys::{self, Hotkey, HotkeyRegistry};
use crate::state::palette_state::PaletteState;

pub use crate::state::palette_state::search_state::{SearchGroup, SearchItem, SearchSource};

/// 输入框占位文案。
pub const PLACEHOLDER: &str = "搜索命令 / 技能 / MCP / 文件 / 会话";
/// 无匹配结果时的提示。
pub const EMPTY_TEXT: &str = "没有匹配的结果";
/// 底部键盘提示。
pub const FOOTER_HINT: &str = "↑↓ 选择 · Enter 执行 · Esc 关闭";

/// 命令面板浮层的渲染描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteView {
    /// 顶部输入框内容。
    pub query: String,
    pub placeholder: &'static str,
    /// 过滤后的多来源分组结果（**空分组不出现**）。
    pub groups: Vec<SearchGroup>,
    /// 当前选中项的 id。
    ///
    /// ⚠️ **按 id 记选中**（不是按下标）：结果按来源分组后，跨组按下标会错位。
    pub selected_id: Option<String>,
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
    let groups = state.groups().to_vec();
    Some(CommandPaletteView {
        query: state.keyword().to_owned(),
        placeholder: PLACEHOLDER,
        empty_text: groups.is_empty().then_some(EMPTY_TEXT),
        groups,
        selected_id: state.selected_id().map(str::to_owned),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    /// 按键被忽略（面板关闭时，或没有可执行项时按 Enter）。
    Ignored,
    /// 状态已改变，需要重绘。
    Redraw,
    /// 执行条目：按 **id** 派发，面板已关闭。
    ///
    /// id 是 `String`（非命令来源的 id 在运行时才有，不是 `'static`）。
    Execute(String),
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
            Some(id) => PaletteAction::Execute(id),
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
mod tests;

/// A7 对齐断言：五类来源统一搜索 + 按来源分组（纯描述层，无 GPUI 依赖）。
///
/// 与 `tests.rs` 同约定：显式导入，不用 `use super::*`（glob 会把 GPUI 的
/// `test` 属性宏带进来，遮蔽标准库的 `#[test]`）。
#[cfg(test)]
mod sources_tests {
    use crate::state::palette_state::search_state::{SearchItem, SearchSource};
    use crate::state::palette_state::PaletteState;
    use crate::ui::command_palette::{handle_key, render, PaletteAction, PaletteKey, EMPTY_TEXT};

    /// 造一条最小条目（hint 缺省为空）。
    fn item(source: SearchSource, id: &str, title: &str) -> SearchItem {
        SearchItem {
            source,
            id: id.to_owned(),
            title: title.to_owned(),
            hint: None,
        }
    }

    /// ① 五类来源各注入 ≥1 项 ⇒ 恰好 5 组、组标题非空、组顺序 = `SearchSource::ALL`、
    /// 组内顺序按 id 稳定（来源故意乱序注入，组间仍按固定来源顺序输出）。
    #[test]
    fn five_sources_yield_five_titled_groups_in_all_order() {
        let items = vec![
            item(SearchSource::Session, "sess-1", "昨天的会话"),
            item(SearchSource::File, "file-1", "季度报表"),
            item(SearchSource::Mcp, "mcp-1", "数据库工具"),
            item(SearchSource::Skill, "skill-1", "代码审查"),
            item(SearchSource::Command, "cmd-open", "打开设置"),
            item(SearchSource::Command, "cmd-save", "保存"),
        ];
        let mut state = PaletteState::with_items(items);
        state.open();
        let view = render(&state).expect("打开时必须渲染浮层");
        assert_eq!(view.groups.len(), 5, "五类各注入 ≥1 项 ⇒ 恰好 5 组");
        for (group, source) in view.groups.iter().zip(SearchSource::ALL.iter()) {
            assert_eq!(group.source, *source, "分组顺序必须等于 SearchSource::ALL");
            assert!(!group.title.is_empty(), "每个分组必须有非空标题");
            assert!(!group.items.is_empty(), "注入过的来源不得整组消失");
        }
        let cmd = view
            .groups
            .iter()
            .find(|g| g.source == SearchSource::Command)
            .expect("命令组必须在场");
        let ids: Vec<&str> = cmd.items.iter().map(|it| it.id.as_str()).collect();
        assert_eq!(ids, vec!["cmd-open", "cmd-save"], "组内顺序按 id 保持稳定");
    }

    /// ② 同标题不同 id：选中与派发**按 id** 区分——选第 2 个不选中第 1 个，
    /// 跨组同名条目也按 id 步进（不按标题、不按下标）。
    #[test]
    fn same_title_different_ids_select_and_dispatch_by_id() {
        let items = vec![
            item(SearchSource::Command, "dup-cmd-1", "重名条目"),
            item(SearchSource::Command, "dup-cmd-2", "重名条目"),
            item(SearchSource::Session, "dup-sess", "重名条目"),
        ];
        let mut state = PaletteState::with_items(items);
        state.open();
        // 打开即选中首项；若实现延后选中，先步进到位（有界，防死循环）。
        for _ in 0..4 {
            if state.selected_id().is_some() {
                break;
            }
            state.select_next();
        }
        assert_eq!(state.selected_id(), Some("dup-cmd-1"), "初始选中应为第一项");

        state.select_next();
        assert_eq!(
            state.selected_id(),
            Some("dup-cmd-2"),
            "同组同标题的第 2 项必须能单独选中"
        );
        assert_ne!(
            state.selected_id(),
            Some("dup-cmd-1"),
            "选第 2 个不得选中第 1 个"
        );

        // 渲染描述同样按 id：两个同标题条目都在场，选中 id 精确指向第二个。
        let view = render(&state).expect("打开时必须渲染浮层");
        let cmd = view
            .groups
            .iter()
            .find(|g| g.source == SearchSource::Command)
            .expect("命令组必须在场");
        assert_eq!(cmd.items.len(), 2);
        assert_eq!(
            cmd.items[0].title, cmd.items[1].title,
            "用例前提：两条目标题相同"
        );
        assert_eq!(view.selected_id.as_deref(), Some("dup-cmd-2"));

        // 跨组同名：向下按 id 进入会话组，退回第 2 项后回车派发。
        state.select_next();
        assert_eq!(
            state.selected_id(),
            Some("dup-sess"),
            "跨组步进也按 id 定位"
        );
        state.select_previous();
        assert_eq!(state.selected_id(), Some("dup-cmd-2"));
        let action = handle_key(&mut state, PaletteKey::Enter);
        assert_eq!(
            action,
            PaletteAction::Execute("dup-cmd-2".to_owned()),
            "派发必须按 id 返回第 2 项，而非标题或第 1 项"
        );
    }

    /// ③ 空查询且无数据 / 有数据但无匹配 ⇒ 不 panic，且给出**显式空态**
    /// （fail-closed，不静默退化成空列表）；无可执行项时 Enter 不派发。
    #[test]
    fn empty_or_no_match_shows_explicit_empty_state() {
        // 空查询 + 无任何注入：显式空态提示，而非什么都不渲染。
        let mut state = PaletteState::with_items(Vec::new());
        state.open();
        let view = render(&state).expect("打开时必须渲染浮层");
        assert_eq!(view.query, "");
        assert!(view.groups.is_empty());
        assert_eq!(view.empty_text, Some(EMPTY_TEXT), "必须给出显式空态提示");
        assert_eq!(
            handle_key(&mut state, PaletteKey::Enter),
            PaletteAction::Ignored,
            "无可执行项时 Enter 不得派发"
        );

        // 有数据但关键词全不匹配：同样显式空态，不 panic。
        let mut state =
            PaletteState::with_items(vec![item(SearchSource::Command, "cmd-x", "唯一命令")]);
        state.open();
        state.set_keyword("绝不匹配的词");
        let view = render(&state).expect("打开时必须渲染浮层");
        assert_eq!(view.query, "绝不匹配的词");
        assert!(view.groups.is_empty(), "无匹配 ⇒ 无分组");
        assert_eq!(
            view.empty_text,
            Some(EMPTY_TEXT),
            "无匹配也必须显示显式空态"
        );
    }
}
