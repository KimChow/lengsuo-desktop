//! 命令面板状态：开关、关键词、过滤结果与选中项。
//!
//! 纯逻辑：不渲染、不派发。执行时只返回命令 id（来自
//! [`crate::theme::palette::COMMANDS`]），由调用方按 id 派发。
//!
//! 约定：
//! - **打开即清空**上次关键词，选中项回到第一项。
//! - 上下选择到边界不越界：首项上移不变、末项下移不变。
//! - 没有选中项（过滤为空或面板关闭）时执行返回 `None`，不猜一个命令。

use crate::theme::palette::{filter_commands, PaletteCommand, COMMANDS};

/// 命令面板状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteState {
    open: bool,
    keyword: String,
    filtered: Vec<PaletteCommand>,
    selected: Option<usize>,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            open: false,
            keyword: String::new(),
            filtered: COMMANDS.to_vec(),
            selected: None,
        }
    }
}

impl PaletteState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// 打开面板：清空上次关键词，回到全量列表并选中第一项。
    pub fn open(&mut self) {
        self.open = true;
        self.keyword.clear();
        self.refilter();
    }

    /// 关闭面板。关键词保留到下次 [`Self::open`] 时再清空。
    pub fn close(&mut self) {
        self.open = false;
        self.selected = None;
    }

    /// 开 / 关切换。
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open();
        }
    }

    #[must_use]
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// 当前过滤结果（顺序与 [`COMMANDS`] 一致）。
    #[must_use]
    pub fn filtered(&self) -> &[PaletteCommand] {
        &self.filtered
    }

    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    #[must_use]
    pub fn selected_command(&self) -> Option<PaletteCommand> {
        self.selected
            .and_then(|index| self.filtered.get(index).copied())
    }

    /// 当前选中项的命令 id（派发依据，不是标题）。
    #[must_use]
    pub fn selected_id(&self) -> Option<&'static str> {
        self.selected_command().map(|command| command.id)
    }

    /// 更新关键词并重新过滤；选中项回到第一项，避免索引悬空。
    pub fn set_keyword(&mut self, keyword: impl Into<String>) {
        self.keyword = keyword.into();
        self.refilter();
    }

    /// 追加一个字符（视图的文本输入）。
    pub fn insert_char(&mut self, c: char) {
        self.keyword.push(c);
        self.refilter();
    }

    /// 删除最后一个字符（按字符而非字节，中文安全）。返回是否发生变化。
    pub fn backspace(&mut self) -> bool {
        if self.keyword.pop().is_some() {
            self.refilter();
            true
        } else {
            false
        }
    }

    fn refilter(&mut self) {
        self.filtered = filter_commands(&self.keyword, COMMANDS);
        self.selected = (self.open && !self.filtered.is_empty()).then_some(0);
    }

    /// 下移一项；末项不动。
    pub fn select_next(&mut self) {
        if let Some(index) = self.selected {
            if index + 1 < self.filtered.len() {
                self.selected = Some(index + 1);
            }
        }
    }

    /// 上移一项；首项不动。
    pub fn select_previous(&mut self) {
        if let Some(index) = self.selected {
            if index > 0 {
                self.selected = Some(index - 1);
            }
        }
    }

    /// 执行选中命令：返回命令 id 并关闭面板。
    ///
    /// 无选中项（面板关闭 / 过滤为空）时返回 `None` 且保持原状态。
    pub fn execute(&mut self) -> Option<&'static str> {
        if !self.open {
            return None;
        }
        let id = self.selected_id()?;
        self.close();
        Some(id)
    }

    /// Escape：关闭面板。返回是否确实关闭了一个打开的面板。
    pub fn handle_escape(&mut self) -> bool {
        if self.open {
            self.close();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened() -> PaletteState {
        let mut state = PaletteState::new();
        state.open();
        state
    }

    #[test]
    fn new_palette_is_closed() {
        let mut state = PaletteState::new();
        assert!(!state.is_open());
        assert_eq!(state.keyword(), "");
        assert_eq!(state.selected_index(), None);
        assert_eq!(state.execute(), None);
    }

    #[test]
    fn opening_selects_first_and_shows_all_commands() {
        let state = opened();
        assert!(state.is_open());
        assert_eq!(state.filtered().len(), COMMANDS.len());
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_id(), Some(COMMANDS[0].id));
    }

    #[test]
    fn opening_clears_previous_keyword() {
        let mut state = opened();
        state.set_keyword("会话");
        assert_eq!(state.filtered().len(), 2);
        state.close();
        state.open();
        assert_eq!(state.keyword(), "", "打开时必须清空上次关键词");
        assert_eq!(state.filtered().len(), COMMANDS.len());
        assert_eq!(state.selected_index(), Some(0));
    }

    #[test]
    fn keyword_filters_commands() {
        let mut state = opened();
        state.set_keyword("org.");
        assert_eq!(state.filtered().len(), 2);
        state.set_keyword("设置");
        assert_eq!(state.filtered().len(), 1);
        assert_eq!(state.selected_id(), Some("app.settings"));
    }

    #[test]
    fn selection_stops_at_first_and_last() {
        let mut state = opened();
        assert_eq!(state.selected_index(), Some(0));
        state.select_previous();
        assert_eq!(state.selected_index(), Some(0), "首项上移不变");

        for _ in 0..COMMANDS.len() + 3 {
            state.select_next();
        }
        assert_eq!(
            state.selected_index(),
            Some(COMMANDS.len() - 1),
            "末项下移不变"
        );
        state.select_next();
        assert_eq!(state.selected_index(), Some(COMMANDS.len() - 1));
    }

    #[test]
    fn empty_result_has_no_selection_and_cannot_execute() {
        let mut state = opened();
        state.set_keyword("zzz-nothing");
        assert!(state.filtered().is_empty());
        assert_eq!(state.selected_index(), None);
        assert_eq!(state.selected_id(), None);
        state.select_next();
        state.select_previous();
        assert_eq!(state.selected_index(), None);
        assert_eq!(state.execute(), None);
        assert!(state.is_open(), "没有可执行项时不得关闭或猜命令");
    }

    #[test]
    fn execute_returns_id_and_closes() {
        let mut state = opened();
        state.set_keyword("设置");
        let id = state.execute().expect("应能执行");
        assert_eq!(id, "app.settings", "派发依据是 id，不是标题");
        assert!(!state.is_open());
        assert_eq!(state.execute(), None, "关闭后不得再次执行");
    }

    #[test]
    fn escape_closes_only_when_open() {
        let mut state = opened();
        assert!(state.handle_escape());
        assert!(!state.is_open());
        assert!(!state.handle_escape());
    }

    #[test]
    fn toggle_switches_open_state() {
        let mut state = PaletteState::new();
        state.toggle();
        assert!(state.is_open());
        state.toggle();
        assert!(!state.is_open());
    }

    #[test]
    fn changing_keyword_resets_selection_to_first() {
        let mut state = opened();
        state.select_next();
        state.select_next();
        assert_eq!(state.selected_index(), Some(2));
        state.set_keyword("org.");
        assert_eq!(
            state.selected_index(),
            Some(0),
            "结果集变化后选中项必须落在有效范围内"
        );
    }

    #[test]
    fn backspace_edits_by_character() {
        let mut state = opened();
        state.set_keyword("设置");
        assert!(state.backspace());
        assert_eq!(state.keyword(), "设");
        assert!(state.backspace());
        assert_eq!(state.keyword(), "");
        assert!(!state.backspace(), "空关键词时不得声称有变化");
    }
}
