//! 停止意图的**宿主侧**接线：消费 + 记账。
//!
//! 输入区点「停止」时，`input_gpui/send_button.rs` 把 [`StopIntent`] 塞进
//! `InputAreaGpui` 的 `stop_outbox`；**此前没有任何消费者** ⇒ 意图只进不出。
//! 本文件只做**宿主该做的事**：取走意图、清掉「本地回显等待态」、如实记账日志。
//!
//! 硬约束：
//! - ⛔ **不执行** `agent:stop`：执行停止属运行时职责（本仓没有运行时）。
//! - ⛔ **不写网络、不落盘**。
//! - 与 `terminal_drawer.rs`（终端意图的宿主侧接线）同构。
use gpui_kit::*;

use crate::ui::input_area::StopIntent;

use super::AppShell;

/// 消费一批停止意图：清掉「本地回显等待态」。
///
/// 返回状态是否真的变了（供调用方决定要不要 `notify()`）。
/// 停止意味着「这条回显不必再等运行时确认」⇒ `pending_seq` 归零。
pub(super) fn apply_stop_intents(pending_seq: &mut Option<u64>, intents: &[StopIntent]) -> bool {
    if intents.is_empty() {
        return false;
    }
    let had_pending = pending_seq.is_some();
    *pending_seq = None;
    had_pending
}

/// 停止意图的**可读**说法（给日志用，⛔ 不参与业务判断）。
#[must_use]
pub(super) fn describe_stop_intent(intent: &StopIntent) -> String {
    format!("请求停止（模式 {}）", intent.mode.id())
}

impl AppShell {
    /// 取走输入区登记的停止意图。
    ///
    /// ⛔ **不执行**：本仓没有运行时 ⇒ 意图只被取走并**如实记账**到日志。
    pub(super) fn drain_stop_intents(&mut self, cx: &mut Context<Self>) {
        let input = self.input.clone();
        let mut intents: Vec<StopIntent> = Vec::new();
        input.update(cx, |input, _| {
            while let Some(intent) = input.take_stop() {
                intents.push(intent);
            }
        });
        if intents.is_empty() {
            return;
        }
        if apply_stop_intents(&mut self.pending_seq, &intents) {
            cx.notify();
        }
        for intent in &intents {
            eprintln!(
                "[lengsuo-desktop] 停止意图（未执行，本仓无运行时）：{}",
                describe_stop_intent(intent)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（**不用 `use super::*`**）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进来并遮蔽标准库的 `#[test]`。
    use super::{apply_stop_intents, describe_stop_intent};
    use crate::ui::input_area::{InputMode, StopIntent};

    fn intent(mode: InputMode) -> StopIntent {
        StopIntent { mode }
    }

    #[test]
    fn stopping_clears_the_pending_echo() {
        let mut pending = Some(7u64);
        assert!(apply_stop_intents(
            &mut pending,
            &[intent(InputMode::Craft)]
        ));
        assert_eq!(pending, None, "停止后不再等运行时确认");
    }

    #[test]
    fn no_intents_change_nothing() {
        let mut pending = Some(7u64);
        assert!(!apply_stop_intents(&mut pending, &[]));
        assert_eq!(pending, Some(7), "没有意图就不许动等待态");
    }

    #[test]
    fn stop_without_pending_echo_reports_no_change() {
        let mut pending = None;
        assert!(!apply_stop_intents(&mut pending, &[intent(InputMode::Ask)]));
        assert_eq!(pending, None);
    }

    #[test]
    fn intents_are_described_in_chinese() {
        let text = describe_stop_intent(&intent(InputMode::Quick));
        assert!(text.starts_with("请求停止"), "必须是人话中文，实际: {text}");
        assert!(!text.contains("agent:stop"), "界面文案不得出现内部命令名");
    }
}
