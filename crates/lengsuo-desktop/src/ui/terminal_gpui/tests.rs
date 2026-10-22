//! 终端抽屉接线层的单元测试（无窗口可测）。
//!
//! ⛔ 不包一层 `mod tests {}`（clippy `module_inception` ⇒ `-D warnings` 整卡失败）；
//! ⛔ 不用 `use super::*`（父模块的 `use gpui_kit::*` 会把 GPUI 的 `test` 属性宏
//! 带进作用域并遮蔽标准库的 `#[test]`，见速查表 §12.3）⇒ 显式列出需要的符号。
#![cfg(test)]

use crate::state::terminal::{TerminalIntent, TerminalTab};
use crate::ui::terminal_gpui::TerminalDrawerGpui;

#[test]
fn drawer_starts_collapsed() {
    let drawer = TerminalDrawerGpui::new();
    assert!(!drawer.state().expanded, "抽屉必须默认收起");
    assert!(!drawer.view().expanded, "描述层必须与状态一致");
}

#[test]
fn toggling_expands_the_drawer() {
    let mut drawer = TerminalDrawerGpui::new();
    drawer.state_mut().toggle_expanded();
    assert!(drawer.state().expanded);
    assert!(drawer.view().expanded, "描述层必须跟着状态走");
}

#[test]
fn the_view_lists_both_tabs_whatever_the_state_is() {
    let drawer = TerminalDrawerGpui::new();
    let view = drawer.view();
    assert_eq!(view.tabs.len(), 2, "两个 tab 始终可见");
    let selected: Vec<TerminalTab> = view
        .tabs
        .iter()
        .filter(|tab| tab.selected)
        .map(|tab| tab.tab)
        .collect();
    assert_eq!(selected, vec![TerminalTab::Output], "默认选中输出 tab");
}

#[test]
fn submitting_a_command_registers_a_run_intent() {
    let mut drawer = TerminalDrawerGpui::new();
    drawer.state_mut().set_input("node -v");
    drawer.state_mut().submit_input();
    assert_eq!(
        drawer.state_mut().take_intents(),
        vec![TerminalIntent::Run("node -v".to_string())],
        "提交必须登记运行意图（⛔ 但不执行）"
    );
}

#[test]
fn the_interrupt_button_registers_an_interrupt_intent() {
    let mut drawer = TerminalDrawerGpui::new();
    drawer.state_mut().interrupt();
    assert_eq!(
        drawer.state_mut().take_intents(),
        vec![TerminalIntent::Interrupt]
    );
}

#[test]
fn the_view_follows_the_selected_tab() {
    let mut drawer = TerminalDrawerGpui::new();
    drawer.state_mut().select_tab(TerminalTab::History);
    assert_eq!(drawer.view().tab, TerminalTab::History);
}
