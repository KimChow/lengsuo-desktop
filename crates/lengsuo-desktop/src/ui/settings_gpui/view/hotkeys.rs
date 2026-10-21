//! 「个性化」面板的描述层：命令 id 的中文说明、按键格式化、面板行清单。
//!
//! 纯数据、无 GPUI 依赖 ⇒ 不开 `ui` feature 也能测。
//!
//! ⛔ 本层只做**展示**：不提供改键入口（快捷键的持久化与冲突裁决不在本卡范围）。
//!
//! 数据源是本仓真实的绑定表 [`crate::state::hotkeys`]：
//! - `Binding` 只有命令 id（英文点分 id，如 `palette.open`）⇒ 中文说明在此补齐；
//! - `Hotkey` 已有 `Display` ⇒ 按键文本直接复用，不另造一套格式。

use crate::state::hotkeys::{command_ids, Binding, Hotkey};
use crate::theme::palette::COMMANDS;

/// 命令 id → 中文说明（界面级命令）。
///
/// 命令面板里的命令（`session.new` 等）复用 [`COMMANDS`] 已有的中文标题，
/// 不在这里重复维护一份。
const COMMAND_LABELS: &[(&str, &str)] = &[
    (command_ids::PALETTE_OPEN, "打开命令面板"),
    (command_ids::FILE_SAVE, "保存文件"),
    (command_ids::OVERLAY_CLOSE, "关闭当前浮层"),
];

/// 绑定表为空时的提示文案。
///
/// ⛔ 不得退化成空表或「一切正常」——那会让人以为有绑定却没显示。
pub const EMPTY_BINDINGS_TEXT: &str = "没有可用的快捷键";

/// 未知命令 id 的兜底文案。
///
/// 单独公开：测试要能断言「内置绑定**没有**落到这个兜底上」。
pub const UNKNOWN_COMMAND_LABEL: &str = "未知命令";

/// 命令 id 的中文说明（设置窗口「个性化」面板用）。
///
/// 界面级命令取自 [`COMMAND_LABELS`]；命令面板的命令复用 [`COMMANDS`] 的标题。
/// 未知命令 id ⇒ 「未知命令」（**不编造**说明，也不返回空串）。
#[must_use]
pub fn command_label(command_id: &str) -> &'static str {
    if let Some((_, label)) = COMMAND_LABELS.iter().find(|(id, _)| *id == command_id) {
        return label;
    }
    COMMANDS
        .iter()
        .find(|command| command.id == command_id)
        .map_or(UNKNOWN_COMMAND_LABEL, |command| command.title)
}

/// 把一个快捷键格式化成展示文本（例：`Ctrl+Shift+P`）。
///
/// 行为定死：
/// - 修饰键顺序 `Ctrl` → `Shift` → `Alt` → `Meta`，再接键名；
/// - 没有修饰键时**不**多出 `+`（是 `Escape`，不是 `+Escape`）；
/// - 键名用键本身的名字（`k` ⇒ `K`、`Escape` ⇒ `Escape`、`F5` ⇒ `F5`）。
///
/// ⛔ 不自己拼字符串：复用 [`Hotkey`] 已有的 `Display`（行为同上），
/// 且 [`crate::state::hotkeys::Key`] 的每个变体都有名字 ⇒ 非 ASCII 键也不会 panic。
#[must_use]
pub fn format_hotkey(hotkey: &Hotkey) -> String {
    hotkey.to_string()
}

/// 「个性化」面板的一行：命令 id + 中文说明 + 按键组合。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeyRow {
    /// 命令 id（稳定，派发用）。
    pub command_id: &'static str,
    /// 中文说明。
    pub label: &'static str,
    /// 按键组合的展示文本（例：`Ctrl+Shift+P`）。
    pub keys: String,
}

/// 「个性化」面板该显示什么（纯逻辑判定；渲染层只映射结果）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyPanel {
    /// 绑定表为空 ⇒ 明确说「没有可用的快捷键」。
    Empty,
    /// 逐条列出绑定。
    ///
    /// ⛔ **不去重**：同一个命令的两个键（`Ctrl+K` 与 `Ctrl+Shift+P`）是
    /// **两条不同的绑定**，都要列出来。
    Rows(Vec<HotkeyRow>),
}

/// 构造「个性化」面板的展示结论。
#[must_use]
pub fn hotkey_panel(bindings: &[Binding]) -> HotkeyPanel {
    if bindings.is_empty() {
        return HotkeyPanel::Empty;
    }
    HotkeyPanel::Rows(
        bindings
            .iter()
            .map(|binding| HotkeyRow {
                command_id: binding.command_id,
                label: command_label(binding.command_id),
                keys: format_hotkey(&binding.hotkey),
            })
            .collect(),
    )
}
