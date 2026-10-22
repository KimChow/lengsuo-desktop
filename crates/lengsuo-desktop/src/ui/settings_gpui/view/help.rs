//! 「帮助与反馈」面板的描述层：把版本号、快捷键表与命令表转成面板要渲染的描述。
//!
//! **只做界面**：只读展示本仓**已有**的信息；
//! ⛔ 不实现反馈提交、不实现网络检查 / 自检 / 资源监控（那些要网络与运行时）。
//!
//! 硬约束：
//! - **四态严格区分**：加载中 / 加载失败 / 真实空集 / 有内容，
//!   失败**不得**退化成空列表（fail-closed）。
//! - **不编造标题**：命令的中文名取自本仓已有的命令表与界面级命令说明，
//!   查不到就退回命令 id 本身（⛔ 不用空串、不用杜撰的名字冒充）。
//!
//! 分层约定同 [`super::data`]：接线层只映射这里的判定结果。

use crate::state::hotkeys::Binding;
use crate::theme::palette::PaletteCommand;

use super::hotkeys::{command_label, format_hotkey, UNKNOWN_COMMAND_LABEL};

/// 面板的数据来源状态。
///
/// ⚠️ 与 [`HelpPanelBody`] 的区别：这里说的是**数据源**的状态，
/// 而 [`HelpPanelBody`] 是**面板要显示什么**的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpStatus {
    /// 数据尚未就绪（正在加载）。
    Loading,
    /// 数据已就绪（可能为空 —— 空是**真实空集**，不是失败）。
    Ready,
    /// 读取失败（fail-closed：必须显示错误，**不得**退化成空列表）。
    Failed(String),
}

/// 「帮助与反馈」面板的正文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelpPanelBody {
    /// 正在加载。
    Loading,
    /// 加载失败（fail-closed：必须显示错误，**不得**退化成空列表）。
    Failed(String),
    /// 真实空集（没有可展示的信息 —— 与「加载失败」是两件事）。
    Empty,
    /// 有内容。
    Content(HelpContent),
}

/// 面板要展示的内容（全部来自本仓已有数据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpContent {
    /// 应用版本。
    pub version: String,
    /// 快捷键条目（人话标题 + 组合键文本）。
    pub shortcuts: Vec<ShortcutRow>,
    /// 命令表条目（标题 + 稳定 id）。
    pub commands: Vec<CommandRow>,
}

/// 一条快捷键。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutRow {
    /// 命令的**人话标题**（查本仓已有的命令表 / 界面级命令说明；查不到就退回命令 id）。
    pub title: String,
    /// 组合键文本（`Hotkey` 的 `Display`）。
    pub keys: String,
}

/// 一条命令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRow {
    /// 人话标题。
    pub title: String,
    /// 稳定 id（派发用）。
    pub id: String,
}

/// 从快捷键表与命令表算出面板正文。
///
/// - 数据源未就绪 ⇒ [`HelpPanelBody::Loading`]；
/// - 数据源失败 ⇒ [`HelpPanelBody::Failed`]（**失败优先于空集判定**）；
/// - 快捷键与命令**都**为空 ⇒ [`HelpPanelBody::Empty`]（真实为空，不是失败）；
/// - 否则 ⇒ [`HelpPanelBody::Content`]。
#[must_use]
pub fn help_body(
    bindings: &[Binding],
    commands: &[PaletteCommand],
    version: &str,
    status: HelpStatus,
) -> HelpPanelBody {
    match status {
        HelpStatus::Loading => HelpPanelBody::Loading,
        HelpStatus::Failed(reason) => HelpPanelBody::Failed(reason),
        HelpStatus::Ready => {
            if bindings.is_empty() && commands.is_empty() {
                HelpPanelBody::Empty
            } else {
                HelpPanelBody::Content(HelpContent {
                    version: version.to_owned(),
                    shortcuts: bindings.iter().map(shortcut_row).collect(),
                    commands: commands.iter().map(command_row).collect(),
                })
            }
        }
    }
}

/// 一条绑定 → 一行快捷键。
fn shortcut_row(binding: &Binding) -> ShortcutRow {
    ShortcutRow {
        title: command_title(binding.command_id),
        // ⚠️ 复用 `Hotkey` 已有的 `Display`（`format_hotkey` 就是它的薄封装），
        // 不自己拼字符串。
        keys: format_hotkey(&binding.hotkey),
    }
}

/// 命令的人话标题。
///
/// 查本仓已有的界面级命令说明与命令表；**查不到就退回命令 id 本身**
/// —— ⛔ 不编造标题，也不用空串冒充。
fn command_title(command_id: &str) -> String {
    let label = command_label(command_id);
    if label == UNKNOWN_COMMAND_LABEL {
        command_id.to_owned()
    } else {
        label.to_owned()
    }
}

/// 一条命令 → 一行命令。
fn command_row(command: &PaletteCommand) -> CommandRow {
    CommandRow {
        title: command.title.to_owned(),
        id: command.id.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    // 显式导入符号（不用 `use super::*`）：与仓内约定保持一致。
    use super::{help_body, HelpPanelBody, HelpStatus};
    use crate::state::hotkeys::HotkeyRegistry;
    use crate::theme::palette::COMMANDS;

    #[test]
    fn default_bindings_produce_content() {
        let registry = HotkeyRegistry::with_defaults().expect("默认快捷键表应可构造");
        let body = help_body(registry.bindings(), COMMANDS, "0.1.0", HelpStatus::Ready);
        assert!(matches!(body, HelpPanelBody::Content(_)));
    }

    #[test]
    fn no_bindings_and_no_commands_is_a_real_empty_set() {
        let body = help_body(&[], &[], "0.1.0", HelpStatus::Ready);
        assert!(matches!(body, HelpPanelBody::Empty));
    }

    #[test]
    fn failure_is_not_reported_as_empty() {
        let body = help_body(&[], &[], "0.1.0", HelpStatus::Failed("读取失败".to_owned()));
        assert!(matches!(body, HelpPanelBody::Failed(_)));
        assert!(
            !matches!(body, HelpPanelBody::Empty),
            "失败态不得显示成空集，否则故障会被伪装成没有数据"
        );
    }

    #[test]
    fn loading_is_not_reported_as_empty() {
        let body = help_body(&[], &[], "0.1.0", HelpStatus::Loading);
        assert!(matches!(body, HelpPanelBody::Loading));
        assert!(!matches!(body, HelpPanelBody::Empty));
    }

    #[test]
    fn content_carries_version_and_human_titles() {
        let registry = HotkeyRegistry::with_defaults().expect("默认快捷键表应可构造");
        let body = help_body(registry.bindings(), COMMANDS, "1.2.3", HelpStatus::Ready);
        match body {
            HelpPanelBody::Content(content) => {
                assert_eq!(content.version, "1.2.3");
                assert!(!content.shortcuts.is_empty());
                assert!(
                    content
                        .shortcuts
                        .iter()
                        .any(|row| row.title == "打开命令面板"),
                    "界面级命令不在命令表里，但本仓已有中文说明 ⇒ 用它，不退回 id"
                );
                assert!(content
                    .commands
                    .iter()
                    .any(|row| row.id == "app.settings" && row.title == "设置"));
            }
            other => panic!("期望有内容，实际 {other:?}"),
        }
    }
}
