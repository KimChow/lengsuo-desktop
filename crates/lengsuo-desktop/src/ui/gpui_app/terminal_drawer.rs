//! 终端抽屉的**宿主侧**接线：意图消费 + 本地回显。
//!
//! 抽屉本体在 [`crate::ui::terminal_gpui`]；本文件只做**宿主该做的事**：
//! 取走抽屉登记的 [`TerminalIntent`]（运行 / 中断），并让根视图重排。
//!
//! 硬约束：
//! - ⛔ **不 spawn 进程、不实现 PTY**：本仓**没有运行时**可执行终端命令 ⇒
//!   意图只被**取走并如实记账**（日志），⛔ 不伪造**运行结果**。
//! - ⛔ **不落盘**：抽屉开合与命令历史都只在内存里（不像右栏折叠进 `layout.json`）。
//! - 颜色与字体一律取自主题。
//!
//! 「运行中」与「回显」的口径（与会话流的「本地回显 / 待运行时确认」一致）：
//! - 提交一条命令 ⇒ 记 `running = true`（**已提交、尚未收到结果**）
//!   ⇒ 「中断」按钮随之可用。
//! - 提交的命令**原样回显**进输出区（带提示符）—— 这是**本地回显**，
//!   ⛔ 不是运行结果；真实输出不在本仓产生。

use gpui_kit::*;

use crate::state::terminal::{TerminalIntent, TerminalState};
use crate::ui::terminal::INPUT_PROMPT;

use super::AppShell;

/// 把终端意图落到状态上。
///
/// - [`TerminalIntent::Run`] ⇒ 记「运行中」（已提交、尚未收到结果）
///   + 把命令**原样回显**进输出区（⛔ 本地回显，不是运行结果）。
/// - [`TerminalIntent::Interrupt`] ⇒ 取消「运行中」。
///
/// 返回状态是否真的变了（供调用方决定要不要 `notify()`）。
pub(super) fn apply_terminal_intents(
    state: &mut TerminalState,
    intents: &[TerminalIntent],
) -> bool {
    let mut changed = false;
    for intent in intents {
        match intent {
            TerminalIntent::Run(command) => {
                state.set_running(true);
                state.push_output(format!("{INPUT_PROMPT}{command}"));
                changed = true;
            }
            TerminalIntent::Interrupt => {
                if state.running {
                    state.set_running(false);
                    changed = true;
                }
            }
        }
    }
    changed
}

/// 终端意图的**可读**说法（给日志用，⛔ 不参与业务判断）。
#[must_use]
pub(super) fn describe_intent(intent: &TerminalIntent) -> String {
    match intent {
        TerminalIntent::Run(command) => format!("运行 `{command}`"),
        TerminalIntent::Interrupt => "中断（Ctrl+C）".to_owned(),
    }
}

impl AppShell {
    /// 取走抽屉登记的终端意图（运行 / 中断）。
    ///
    /// ⛔ **不执行**：本仓没有运行时（无 PTY、无进程 spawn）⇒ 意图只被取走并
    /// **如实记账**到日志，⛔ **绝不伪造运行结果**。
    ///
    /// 取走即清空（状态层的 `take_intents` 语义）⇒ 意图不会被重复消费。
    pub(super) fn drain_terminal_intents(&mut self, cx: &mut Context<Self>) {
        let intents = self.terminal_drawer.update(cx, |drawer, cx| {
            let state = drawer.state_mut();
            let intents = state.take_intents();
            if apply_terminal_intents(state, &intents) {
                cx.notify();
            }
            intents
        });

        for intent in intents {
            eprintln!(
                "[lengsuo-desktop] 终端意图（未执行，本仓无运行时）：{}",
                describe_intent(&intent)
            );
        }
    }

    /// 抽屉变化 ⇒ 消费意图 + 让根视图重排。
    ///
    /// ⚠️ **必须重排**：抽屉展开后高度从一条标题条变成「内容区 + 输入行」，
    /// 而根视图是 `flex_col` ⇒ 高度变了就必须让**根视图**重算布局
    /// （只重绘子实体不会让父级重排，同 `sync_layout` 的教训）。
    pub(super) fn sync_terminal_drawer(&mut self, cx: &mut Context<Self>) {
        self.drain_terminal_intents(cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
    use super::{apply_terminal_intents, describe_intent};
    use crate::state::terminal::{TerminalIntent, TerminalState};

    #[test]
    fn running_a_command_marks_the_terminal_busy_and_echoes_it() {
        let mut state = TerminalState::default();
        let changed = apply_terminal_intents(&mut state, &[TerminalIntent::Run("node -v".into())]);
        assert!(changed);
        assert!(state.running, "已提交、尚未收到结果 ⇒ 运行中");
        assert_eq!(
            state.output,
            vec!["> node -v".to_string()],
            "提交的命令要原样回显（本地回显，不是运行结果）"
        );
    }

    #[test]
    fn interrupting_clears_the_running_flag() {
        let mut state = TerminalState::default();
        state.set_running(true);
        let changed = apply_terminal_intents(&mut state, &[TerminalIntent::Interrupt]);
        assert!(changed);
        assert!(!state.running, "中断后不再算运行中");
    }

    #[test]
    fn no_intents_change_nothing() {
        let mut state = TerminalState::default();
        assert!(!apply_terminal_intents(&mut state, &[]));
        assert!(state.output.is_empty());
        assert!(!state.running);
    }

    #[test]
    fn intents_are_described_in_chinese() {
        assert_eq!(
            describe_intent(&TerminalIntent::Run("node -v".into())),
            "运行 `node -v`"
        );
        assert_eq!(
            describe_intent(&TerminalIntent::Interrupt),
            "中断（Ctrl+C）"
        );
    }
}
