//! 「个性化」面板的描述层：命令 id 的中文说明、按键格式化、面板行清单、改键裁决。
//!
//! 纯数据、无 GPUI 依赖 ⇒ 不开 `ui` feature 也能测。
//!
//! 本层做两件事：
//! - **展示**：行清单 + 空态判定（[`hotkey_panel`]）；
//! - **改键裁决**：按下的键 → 改键意图（成功 / 取消 / 冲突 / 失败），
//!   走既有的 [`crate::state::hotkeys::HotkeyRegistry::apply_overrides`]，
//!   **不自己实现冲突检测**。
//!
//! ⛔ 裁决**不落盘**：改键只在本次运行内有效（持久化不在本卡范围）。
//!
//! 数据源是本仓真实的绑定表 [`crate::state::hotkeys`]：
//! - `Binding` 只有命令 id（英文点分 id，如 `palette.open`）⇒ 中文说明在此补齐；
//! - `Hotkey` 已有 `Display` ⇒ 按键文本直接复用，不另造一套格式。
//!
//! ⚠️ **本模块不门控**（`view/` 的约定：不开 `ui` feature 也能测）⇒ 改键裁决
//! （`HotkeyEdit` 等）在**不开** `ui` 时没有任何调用者（唯一的调用者是
//! `ui` 接线层）⇒ 用 `allow(dead_code)` 静音，**只在不开 `ui` 时生效**；
//! 开了 `ui` 时本属性不生效，死代码照旧被 `-D warnings` 抓到。

#![cfg_attr(not(feature = "ui"), allow(dead_code))]

use crate::state::hotkeys::{
    command_ids, Binding, Hotkey, HotkeyError, HotkeyOverride, HotkeyRegistry, Key, Modifiers,
};
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

/// 「个性化」面板的**改键**状态（纯数据）。
///
/// ⛔ 可见性收到 `crate::ui::settings_gpui`：`view` 的子模块清单
/// （`view/mod.rs`）**不在本卡边界内**，不能加 `pub use` ⇒ 接线层经
/// [`HotkeyPanel::Edit`] 取用本类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui::settings_gpui) enum HotkeyEdit {
    /// 不在改键中。
    Idle,
    /// 正在等用户按下一个组合键（目标命令 id）。
    Awaiting(&'static str),
    /// 改键失败：**冲突**（该组合键已被别的命令占用）。
    Conflict {
        /// 被改的命令 id。
        command_id: &'static str,
        /// 造成冲突的组合键文本。
        hotkey: String,
    },
    /// 改键失败：其它原因（未知命令 / 格式错）。
    Failed {
        /// 被改的命令 id。
        command_id: &'static str,
        /// 中文原因说明。
        reason: String,
    },
}

/// 裸键（没有修饰键）被拒时的原因文案。
const NO_MODIFIER_REASON: &str = "至少需要一个修饰键（Ctrl/Shift/Alt/Meta）";

/// 把「用户按下的键」转成改键意图。
///
/// ⛔ **单键不算**：没有修饰键的裸键（如 `A`）**拒绝** —— 会和输入框抢键。
/// ⛔ **Esc 取消**：Esc 本身是 `overlay.close` 的绑定 ⇒ 捕获时**排除** Esc。
///
/// 判定顺序（Esc 先判，因为 Esc 本身也没有修饰键）：
/// 1. 裸 Esc ⇒ [`HotkeyEdit::Idle`]（取消，不算失败）；
/// 2. 其它无修饰键的键 ⇒ [`HotkeyEdit::Failed`]；
/// 3. 有修饰键 ⇒ 交给既有的 [`HotkeyRegistry::apply_overrides`] 裁决：
///    成功 ⇒ [`HotkeyEdit::Idle`]；冲突 ⇒ [`HotkeyEdit::Conflict`]；
///    其它错误 ⇒ [`HotkeyEdit::Failed`]。
///
/// ⚠️ [`HotkeyEdit::Idle`] 同时表示「取消」与「成功」：调用方要区分二者，
/// 须先自己判「是不是裸 Esc」。
#[must_use]
pub(in crate::ui::settings_gpui) fn edit_intent(
    bindings: &[Binding],
    target: &'static str,
    pressed: Hotkey,
) -> HotkeyEdit {
    // 1. 裸 Esc ⇒ 取消（不算失败）。
    if pressed.key == Key::Escape && pressed.modifiers == Modifiers::NONE {
        return HotkeyEdit::Idle;
    }
    // 2. 裸键 ⇒ 拒绝。
    if pressed.modifiers == Modifiers::NONE {
        return HotkeyEdit::Failed {
            command_id: target,
            reason: NO_MODIFIER_REASON.to_owned(),
        };
    }
    // 3. 交给既有的注册表裁决（原子：任一条出错 ⇒ 整批不生效）。
    let mut registry = match HotkeyRegistry::from_bindings(bindings.iter().copied()) {
        Ok(registry) => registry,
        Err(reason) => {
            return HotkeyEdit::Failed {
                command_id: target,
                reason: reason.to_string(),
            }
        }
    };
    match registry.apply_overrides(&[HotkeyOverride {
        command_id: target.to_owned(),
        hotkey: pressed,
    }]) {
        Ok(()) => HotkeyEdit::Idle,
        Err(HotkeyError::Conflict { .. }) => HotkeyEdit::Conflict {
            command_id: target,
            hotkey: format_hotkey(&pressed),
        },
        Err(reason) => HotkeyEdit::Failed {
            command_id: target,
            reason: reason.to_string(),
        },
    }
}

/// 改键状态的中文提示。
///
/// [`HotkeyEdit::Idle`] ⇒ **没有提示**（`None`）⇒ 面板不渲染状态区
/// （不留空占位）。
#[must_use]
pub(in crate::ui::settings_gpui) fn edit_status_text(edit: &HotkeyEdit) -> Option<String> {
    match edit {
        HotkeyEdit::Idle => None,
        HotkeyEdit::Awaiting(target) => Some(format!(
            "请按下新的组合键（目标：{}）… Esc 取消",
            command_label(target)
        )),
        HotkeyEdit::Conflict { .. } => Some("该组合键已被其他命令占用，改键未生效。".to_owned()),
        HotkeyEdit::Failed { reason, .. } => Some(format!("改键未生效：{reason}")),
    }
}

impl HotkeyEdit {
    /// 状态是否为空闲。
    ///
    /// ⚠️ `Idle` 同时表示「取消」与「成功」⇒ **只有接线层能区分**：
    /// 它先判「是不是裸 Esc」，判过了才轮到这里的 `Idle` ⇒ 此时 `Idle` 即成功。
    pub(in crate::ui::settings_gpui) fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// 失败详情（给日志用；成功/取消时没有详情）。
    ///
    /// ⛔ 面板文案是固定的（见 [`edit_status_text`]）⇒ 命令 id 与组合键不进文案，
    /// 但**不能丢** ⇒ 落到日志里，便于事后查「谁跟谁撞了」。
    pub(in crate::ui::settings_gpui) fn failure_detail(&self) -> Option<String> {
        match self {
            Self::Conflict { command_id, hotkey } => Some(format!(
                "{command_id} 想占用 {hotkey}，但该键已被别的命令绑定"
            )),
            Self::Failed { command_id, reason } => Some(format!("{command_id}：{reason}")),
            _ => None,
        }
    }
}

impl HotkeyPanel {
    /// 构造「正在等键」状态（接线层点行时用）。
    pub(in crate::ui::settings_gpui) fn edit_awaiting(target: &'static str) -> HotkeyEdit {
        HotkeyEdit::Awaiting(target)
    }

    /// 裁决一次改键（见 [`edit_intent`]）。
    pub(in crate::ui::settings_gpui) fn edit_intent(
        bindings: &[Binding],
        target: &'static str,
        pressed: Hotkey,
    ) -> HotkeyEdit {
        edit_intent(bindings, target, pressed)
    }

    /// 改键状态的中文提示（见 [`edit_status_text`]）。
    pub(in crate::ui::settings_gpui) fn edit_status_text(edit: &HotkeyEdit) -> Option<String> {
        edit_status_text(edit)
    }
}

#[cfg(test)]
mod tests {
    // 显式列出符号（不用 `use super::*`）：见速查表 §12.3。
    use super::{edit_intent, hotkey_panel, HotkeyEdit, HotkeyPanel, EMPTY_BINDINGS_TEXT};
    use crate::state::hotkeys::{command_ids, Binding, Hotkey, HotkeyRegistry, Key};

    /// 解析一个测试用快捷键。
    fn hotkey(spec: &str) -> Hotkey {
        Hotkey::parse(spec).expect("测试快捷键应可解析")
    }

    /// 内置绑定表（真实数据，不是编造的）。
    fn default_bindings() -> Vec<Binding> {
        HotkeyRegistry::with_defaults()
            .expect("内置表不该冲突")
            .bindings()
            .to_vec()
    }

    /// 裸键（没有修饰键）必须被**拒绝**：不能和输入框抢键。
    #[test]
    fn a_bare_key_is_rejected() {
        let bindings = default_bindings();
        let edit = edit_intent(&bindings, command_ids::FILE_SAVE, hotkey("A"));

        assert_eq!(
            edit,
            HotkeyEdit::Failed {
                command_id: command_ids::FILE_SAVE,
                reason: "至少需要一个修饰键（Ctrl/Shift/Alt/Meta）".to_owned(),
            },
            "裸键必须判失败（不是取消、也不是冲突）"
        );
    }

    /// Esc 是取消，不是失败。
    #[test]
    fn escape_cancels_the_edit() {
        let bindings = default_bindings();
        let edit = edit_intent(
            &bindings,
            command_ids::FILE_SAVE,
            Hotkey::plain(Key::Escape),
        );

        assert_eq!(edit, HotkeyEdit::Idle, "Esc 只能取消改键");
    }

    /// 与别的命令冲突的组合键必须**报冲突**，且原绑定表**一个字节都不变**。
    #[test]
    fn a_conflicting_combo_is_reported_and_changes_nothing() {
        let bindings = default_bindings();
        // `Ctrl+S` 已经是 `file.save` 的键 ⇒ 拿它改 `palette.open` ⇒ 冲突。
        let edit = edit_intent(&bindings, command_ids::PALETTE_OPEN, hotkey("Ctrl+S"));

        assert_eq!(
            edit,
            HotkeyEdit::Conflict {
                command_id: command_ids::PALETTE_OPEN,
                hotkey: "Ctrl+S".to_owned(),
            },
            "冲突必须显式报出来（fail-closed：不能静默忽略）"
        );
        // 原绑定表不变：`Ctrl+S` 仍归 `file.save`，`palette.open` 的两个键也还在。
        let registry =
            HotkeyRegistry::from_bindings(bindings.iter().copied()).expect("内置表不该冲突");
        assert_eq!(
            registry.command_for(hotkey("Ctrl+S")),
            Some(command_ids::FILE_SAVE),
            "冲突不能把别人的键抢走"
        );
        assert_eq!(
            registry.command_for(hotkey("Ctrl+K")),
            Some(command_ids::PALETTE_OPEN),
            "冲突不能动目标命令原有的键"
        );
    }

    /// 新组合键 ⇒ 改键成功，且**旧键被释放**（按命令覆盖）。
    #[test]
    fn a_new_combo_replaces_the_old_one() {
        let bindings = default_bindings();
        let target = command_ids::FILE_SAVE;

        assert_eq!(
            edit_intent(&bindings, target, hotkey("Ctrl+J")),
            HotkeyEdit::Idle,
            "没冲突的组合键必须判成功"
        );

        // 落表：设置窗口的写回走的是同一条路（`HotkeyRegistry::rebind`）。
        let mut registry =
            HotkeyRegistry::from_bindings(bindings.iter().copied()).expect("内置表不该冲突");
        registry
            .rebind(target, hotkey("Ctrl+J"))
            .expect("裁决通过后落表必成功");

        assert_eq!(
            registry.command_for(hotkey("Ctrl+S")),
            None,
            "旧键必须被释放"
        );
        assert_eq!(
            registry.command_for(hotkey("Ctrl+J")),
            Some(target),
            "新键必须挂上"
        );
        assert_eq!(
            registry.command_for(hotkey("Ctrl+K")),
            Some(command_ids::PALETTE_OPEN),
            "别的命令的键不受影响"
        );
    }

    /// 改键逻辑不许把空表吃成「有内容」：空表仍要说「没有可用的快捷键」。
    #[test]
    fn an_empty_binding_table_says_there_is_no_shortcut() {
        assert_eq!(hotkey_panel(&[]), HotkeyPanel::Empty);
        assert_eq!(EMPTY_BINDINGS_TEXT, "没有可用的快捷键");
    }
}
