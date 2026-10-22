//! 集成终端抽屉的**描述层**：把「终端会话态」转成要渲染的描述。
//!
//! **只做界面**：不 spawn 进程、不实现 PTY、不解析终端协议。
//! 数据只来自 [`crate::state::terminal`]，⛔ 不直连运行时。
//!
//! 分层：描述层纯数据（可在无窗口环境测），接线层在
//! `ui/terminal_gpui/`（把描述变成 GPUI 元素）。
//!
//! 对齐规格 `ui-parity-spec.md` §1 条目 **A6**（终端输出 / Ctrl+C / 历史保留）
//! 与 **A13**（集成终端抽屉）。

use crate::state::terminal::{TerminalState, TerminalTab};

/// 抽屉收起时标题条上的文案。
pub const TERMINAL_TAB_LABEL: &str = "终端";

/// 抽屉展开时标题条上的文案。
pub const TERMINAL_TITLE: &str = "终端";

/// 输出区为空时的文案（真实为空，⛔ 不是加载失败）。
pub const OUTPUT_EMPTY_TEXT: &str = "暂无输出";

/// 历史区为空时的文案（真实为空，⛔ 不是加载失败）。
pub const HISTORY_EMPTY_TEXT: &str = "暂无历史命令";

/// 输入行的提示符。
pub const INPUT_PROMPT: &str = "> ";

/// 运行按钮文案。
pub const RUN_LABEL: &str = "运行";

/// 中断按钮文案（Ctrl+C）。
pub const INTERRUPT_LABEL: &str = "中断";

/// 一个 tab 按钮。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalTabButton {
    /// tab 身份（点击时回写状态）。
    pub tab: TerminalTab,
    /// tab 标签。
    pub label: &'static str,
    /// 是否选中。
    pub selected: bool,
}

/// 终端抽屉的渲染描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalView {
    /// 抽屉是否展开。
    pub expanded: bool,
    /// 抽屉标题。
    pub title: String,
    /// 当前选中的 tab（输出 / 历史）。
    pub tab: TerminalTab,
    /// 全部 tab（始终可见）。
    pub tabs: Vec<TerminalTabButton>,
    /// 输出区内容（等宽渲染；⛔ 不做任何 ANSI 解析）。
    pub output: Vec<String>,
    /// 命令历史（**倒序**：最近的在最前）。
    pub history: Vec<String>,
    /// 当前输入行内容。
    pub input: String,
    /// 是否有正在运行的命令（决定「中断」按钮是否可用）。
    pub running: bool,
}

/// 生成终端抽屉的渲染描述。
///
/// - 两个 tab **始终可见**（与右栏 tab 的做法一致）。
/// - 历史**倒序**（最近输入的在最前）。
/// - 本函数**不修改**传入的状态。
#[must_use]
pub fn terminal_view(state: &TerminalState) -> TerminalView {
    TerminalView {
        expanded: state.expanded,
        title: TERMINAL_TITLE.to_owned(),
        tab: state.tab,
        tabs: TerminalTab::ALL
            .iter()
            .map(|&tab| TerminalTabButton {
                tab,
                label: tab.label(),
                selected: tab == state.tab,
            })
            .collect(),
        output: state.output.clone(),
        history: state.history.iter().rev().cloned().collect(),
        input: state.input.clone(),
        running: state.running,
    }
}

#[cfg(test)]
mod tests {
    use crate::state::terminal::{TerminalState, TerminalTab};
    use crate::ui::terminal::terminal_view;

    #[test]
    fn collapsed_view_lists_both_tabs() {
        let state = TerminalState::default();
        let view = terminal_view(&state);
        assert!(!view.expanded);
        assert_eq!(view.tabs.len(), 2, "两个 tab 始终可见");
    }

    #[test]
    fn output_lines_are_kept_in_order() {
        let mut state = TerminalState::default();
        state.push_output("第一行");
        state.push_output("第二行");
        let view = terminal_view(&state);
        assert_eq!(
            view.output,
            vec!["第一行".to_string(), "第二行".to_string()]
        );
    }

    #[test]
    fn history_is_newest_first() {
        let mut state = TerminalState::default();
        state.set_input("第一条");
        state.submit_input();
        state.set_input("第二条");
        state.submit_input();
        let view = terminal_view(&state);
        assert_eq!(
            view.history,
            vec!["第二条".to_string(), "第一条".to_string()],
            "历史必须倒序：最近输入的在最前"
        );
    }

    #[test]
    fn the_selected_tab_marks_only_itself() {
        let mut state = TerminalState::default();
        state.select_tab(TerminalTab::History);
        let view = terminal_view(&state);
        let selected: Vec<TerminalTab> = view
            .tabs
            .iter()
            .filter(|tab| tab.selected)
            .map(|tab| tab.tab)
            .collect();
        assert_eq!(selected, vec![TerminalTab::History]);
    }
}
