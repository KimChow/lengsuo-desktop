//! 集成终端抽屉的状态层（纯逻辑，无 IO、无渲染、无 async）。
//!
//! ⛔ **不 spawn 进程、不实现 PTY、不做终端协议**：本层只保存
//! 「界面要显示什么」与「用户想执行什么意图」。
//!
//! 意图（用户点了运行 / 中断）由宿主消费（`AppShell`），
//! 本层**只登记不执行** ⇒ 界面层不做业务语义。
//!
//! 对齐规格 `ui-parity-spec.md` §1 条目 **A6**（终端输出 / Ctrl+C / 历史保留）
//! 与 **A13**（集成终端抽屉）。

/// 终端抽屉的 tab。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalTab {
    /// 输出。
    #[default]
    Output,
    /// 历史。
    History,
}

impl TerminalTab {
    /// 全部 tab（始终可见）。
    pub const ALL: [Self; 2] = [Self::Output, Self::History];

    /// tab 的中文标签。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Output => "输出",
            Self::History => "历史",
        }
    }

    /// tab 的稳定标识（用于拼同屏唯一的元素 id）。
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Output => "output",
            Self::History => "history",
        }
    }
}

/// 用户对终端的操作意图（**只登记，不执行**）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalIntent {
    /// 执行一条命令。
    Run(String),
    /// 中断当前命令（Ctrl+C）。
    Interrupt,
}

/// 终端会话态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalState {
    /// 抽屉是否展开。
    pub expanded: bool,
    /// 当前选中的 tab。
    pub tab: TerminalTab,
    /// 输出行（⛔ 不做 ANSI 解析，原样显示）。
    pub output: Vec<String>,
    /// 命令历史（按输入顺序；渲染时倒序）。
    pub history: Vec<String>,
    /// 当前输入行。
    pub input: String,
    /// 是否有正在运行的命令。
    pub running: bool,
    /// 待宿主消费的命令意图（运行 / 中断）。
    pub pending: Vec<TerminalIntent>,
}

impl TerminalState {
    /// 展开 / 收起抽屉。
    pub fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    /// 直接设置展开态。
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    /// 切换选中的 tab。
    pub fn select_tab(&mut self, tab: TerminalTab) {
        self.tab = tab;
    }

    /// 追加一行输出（**不覆盖**已有内容）。
    pub fn push_output(&mut self, line: impl Into<String>) {
        self.output.push(line.into());
    }

    /// 写入输入行内容。
    pub fn set_input(&mut self, input: impl Into<String>) {
        self.input = input.into();
    }

    /// 设置「是否有正在运行的命令」。
    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    /// 提交输入行。
    ///
    /// - **空白输入 ⇒ 不产生任何意图**（fail-closed，也不动历史与输入行）。
    /// - 非空 ⇒ 记入历史、清空输入行、登记 [`TerminalIntent::Run`] 意图。
    pub fn submit_input(&mut self) {
        let command = self.input.trim().to_owned();
        if command.is_empty() {
            return;
        }
        self.history.push(command.clone());
        self.input.clear();
        self.pending.push(TerminalIntent::Run(command));
    }

    /// 登记一次中断（Ctrl+C）意图。
    pub fn interrupt(&mut self) {
        self.pending.push(TerminalIntent::Interrupt);
    }

    /// 取走并清空待消费的意图（宿主消费一次）。
    pub fn take_intents(&mut self) -> Vec<TerminalIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use crate::state::terminal::{TerminalIntent, TerminalState, TerminalTab};

    #[test]
    fn empty_input_does_not_produce_an_intent() {
        let mut state = TerminalState::default();
        state.set_input("   ");
        state.submit_input();
        assert!(state.take_intents().is_empty(), "空白输入不得产生执行意图");
        assert!(state.history.is_empty(), "空白输入不得进历史");
    }

    #[test]
    fn submitting_a_command_records_history_and_clears_input() {
        let mut state = TerminalState::default();
        state.set_input("node -v");
        state.submit_input();
        assert_eq!(state.history, vec!["node -v".to_string()]);
        assert!(state.input.is_empty());
        assert_eq!(
            state.take_intents(),
            vec![TerminalIntent::Run("node -v".to_string())]
        );
    }

    #[test]
    fn interrupt_produces_an_interrupt_intent() {
        let mut state = TerminalState::default();
        state.interrupt();
        assert_eq!(state.take_intents(), vec![TerminalIntent::Interrupt]);
    }

    #[test]
    fn toggling_expanded_flips_the_drawer() {
        let mut state = TerminalState::default();
        assert!(!state.expanded, "抽屉必须默认收起");
        state.toggle_expanded();
        assert!(state.expanded);
        state.toggle_expanded();
        assert!(!state.expanded);
    }

    #[test]
    fn taking_intents_clears_the_queue() {
        let mut state = TerminalState::default();
        state.interrupt();
        assert_eq!(state.take_intents().len(), 1);
        assert!(state.take_intents().is_empty(), "意图只能被消费一次");
    }

    #[test]
    fn output_lines_are_appended_not_overwritten() {
        let mut state = TerminalState::default();
        state.push_output("第一行");
        state.push_output("第二行");
        assert_eq!(
            state.output,
            vec!["第一行".to_string(), "第二行".to_string()]
        );
    }

    #[test]
    fn selecting_a_tab_switches_the_active_tab() {
        let mut state = TerminalState::default();
        assert_eq!(state.tab, TerminalTab::Output);
        state.select_tab(TerminalTab::History);
        assert_eq!(state.tab, TerminalTab::History);
    }
}
