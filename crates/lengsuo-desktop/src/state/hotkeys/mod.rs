//! 快捷键注册表：组合键 → 命令 id 的解析、匹配与冲突检测。
//!
//! 三条硬约定：
//! - **按 id 派发**：映射的值是 [`crate::theme::palette`] 的命令 id
//!   （或本模块声明的界面级命令 id），绝不按标题派发。
//! - **冲突必须报错**：同一组合键映射到两个不同命令时返回
//!   [`HotkeyError::Conflict`]，不静默取第一个。
//! - **覆盖后重新校验**：用户改键经 [`HotkeyRegistry::apply_overrides`]
//!   **原子**应用；整批覆盖完成后重新做一次冲突检测，有冲突则整批不生效。
//!
//! 本模块只做注册表与匹配逻辑；真实全局快捷键注册是打包时宿主的职责。
//!
//! ⚠️ 本模块由 `state/hotkeys.rs` 拆分而来（`e47-hotkey-split`）：
//! - [`keys`]：按键原语（`Modifiers` / `Key` / `Hotkey` / `HotkeyParseError`）；
//! - [`registry`]：绑定表与注册表（`Binding` / `HotkeyRegistry` / `HotkeyError` / 覆盖项）；
//! - `tests`：原 `hotkeys.rs` 内的 15 个单测，逻辑一字未改。
//! - 拆分理由：原文件 687 行，距门禁 700 行硬上限只差 13 行 ⇒ 任何编辑都可能顶爆门禁。
//! - 公开 API 完全不变：`crate::state::hotkeys::X` 全部继续可用（`mod.rs` 统一 re-export）。

/// 界面级命令 id（不属于可搜索命令表，由宿主 / 视图直接处理）。
pub mod command_ids {
    /// 打开命令面板（`Ctrl+K` / `Ctrl+Shift+P`）。
    pub const PALETTE_OPEN: &str = "palette.open";
    /// 保存文件（`Ctrl+S`）。
    pub const FILE_SAVE: &str = "file.save";
    /// 关闭浮层（`Escape`）。
    pub const OVERLAY_CLOSE: &str = "overlay.close";
}

/// 全部界面级命令 id，用于校验覆盖项。
pub const APP_COMMAND_IDS: &[&str] = &[
    command_ids::PALETTE_OPEN,
    command_ids::FILE_SAVE,
    command_ids::OVERLAY_CLOSE,
];

mod keys;
mod registry;
#[cfg(test)]
mod tests;

pub use keys::{Hotkey, HotkeyParseError, Key, Modifiers};
pub use registry::{
    resolve_command_id, Binding, HotkeyError, HotkeyOverride, HotkeyRegistry, DEFAULT_BINDINGS,
};
