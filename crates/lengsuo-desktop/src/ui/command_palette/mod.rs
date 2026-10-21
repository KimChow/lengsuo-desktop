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
