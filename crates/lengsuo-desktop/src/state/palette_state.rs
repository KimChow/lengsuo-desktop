//! 命令面板状态：开关、关键词、**多来源搜索结果**（按来源分组）与选中项。
//!
//! 纯逻辑：不渲染、不派发。执行时只返回条目 id（**按 id 派发**，不用标题），
//! 由调用方按 id 派发。
//!
//! 约定：
//! - **打开即清空**上次关键词，选中项回到第一项。
//! - 上下选择到边界不越界：首项上移不变、末项下移不变。
//! - 没有选中项（过滤为空或面板关闭）时执行返回 `None`，不猜一个条目。
//! - **选中项按 id 记**（不是按下标）：结果按来源分组后跨组按下标会错位。
//!
//! 搜索条目的形状在 [`search_state`]（同文件树下的子模块）；数据由**调用方注入**
//! —— 命令来自内置命令表；其余四类（技能 / MCP / 文件 / 会话）的真实数据源
//! 属运行时侧，⛔ 本仓不实现 ⇒ **没有注入就没有该分组**（不显示假条目）。

#[path = "search_state.rs"]
pub mod search_state;

pub use self::search_state::{
    command_items, filter_items, flatten, group_by_source, real_items, SearchGroup, SearchItem,
    SearchSource,
};

use self::search_state::SearchState;

/// 命令面板状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteState {
    open: bool,
    /// 全部可搜索条目 + 关键词（**关键词只存这一处**）。
    search: SearchState,
    /// 过滤 + 分组后的结果（空分组不出现）。
    groups: Vec<SearchGroup>,
    /// 当前选中项的 id（**按 id 记**，跨分组不会错位）。
    selected_id: Option<String>,
}

impl Default for PaletteState {
    /// **空条目**（不是演示数据）：面板默认什么都不显示，直到宿主注入真实条目。
    fn default() -> Self {
        Self::with_items(Vec::new())
    }
}

impl PaletteState {
    /// 建一个**空**状态：没有注入任何条目 ⇒ 面板打开后没有任何分组。
    ///
    /// ⛔ 不再灌演示条目：本仓没有真实数据源的类别 ⇒ **不显示该分组**，
    /// 而不是显示假条目。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 用**注入的条目**建状态：真实数据源接入时的入口。
    ///
    /// 注入什么就显示什么；**空分组不出现**（见 [`group_by_source`]）。
    #[must_use]
    pub fn with_items(items: Vec<SearchItem>) -> Self {
        Self {
            open: false,
            search: SearchState::new(items),
            groups: Vec::new(),
            selected_id: None,
        }
    }

    /// 换掉全部条目（宿主拿到新数据时用），并重新过滤。
    ///
    /// 换数据后**关键词保留**：「同一批关键词下换一批结果」，而不是清空输入。
    pub fn set_items(&mut self, items: Vec<SearchItem>) {
        self.search = SearchState::new(items);
        self.search.set_keyword(self.keyword().to_owned());
        self.refilter();
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// 打开面板：清空上次关键词，回到全量列表并选中第一项。
    pub fn open(&mut self) {
        self.open = true;
        self.search.set_keyword("");
        self.refilter();
    }

    /// 关闭面板。关键词保留到下次 [`Self::open`] 时再清空。
    pub fn close(&mut self) {
        self.open = false;
        self.selected_id = None;
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
        self.search.keyword()
    }

    /// 全部可搜索条目（注入的那一份，未过滤）。
    #[must_use]
    pub fn all_items(&self) -> &[SearchItem] {
        self.search.items()
    }

    /// 当前结果**按来源分组**（空分组不出现）。
    #[must_use]
    pub fn groups(&self) -> &[SearchGroup] {
        &self.groups
    }

    /// 当前结果的**展示顺序**摊平列表（分组顺序 → 组内顺序）。
    #[must_use]
    pub fn items(&self) -> Vec<SearchItem> {
        flatten(&self.groups)
    }

    /// 当前选中项的 id（派发依据，不是标题）。
    #[must_use]
    pub fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    /// 当前选中项（条目本身）。
    #[must_use]
    pub fn selected_item(&self) -> Option<&SearchItem> {
        let id = self.selected_id.as_deref()?;
        self.groups
            .iter()
            .flat_map(|group| group.items.iter())
            .find(|item| item.id == id)
    }

    /// 当前选中项在**展示顺序**里的下标（没有选中项时为 `None`）。
    #[must_use]
    pub fn selected_index(&self) -> Option<usize> {
        let id = self.selected_id.as_deref()?;
        self.flat_ids().iter().position(|x| *x == id)
    }

    /// 更新关键词并重新过滤；选中项回到第一项，避免选中悬空。
    pub fn set_keyword(&mut self, keyword: impl Into<String>) {
        self.search.set_keyword(keyword);
        self.refilter();
    }

    /// 追加一个字符（视图的文本输入）。
    pub fn insert_char(&mut self, c: char) {
        let mut keyword = self.keyword().to_owned();
        keyword.push(c);
        self.set_keyword(keyword);
    }

    /// 删除最后一个字符（按字符而非字节，中文安全）。返回是否发生变化。
    pub fn backspace(&mut self) -> bool {
        let mut keyword = self.keyword().to_owned();
        if keyword.pop().is_none() {
            return false;
        }
        self.set_keyword(keyword);
        true
    }

    fn refilter(&mut self) {
        self.groups = self.search.groups();
        self.selected_id = if self.open {
            self.flat_ids().first().map(|id| (*id).to_owned())
        } else {
            None
        };
    }

    /// 展示顺序里的 id 列表（分组顺序 → 组内顺序）。
    fn flat_ids(&self) -> Vec<&str> {
        self.groups
            .iter()
            .flat_map(|group| group.items.iter().map(|item| item.id.as_str()))
            .collect()
    }

    /// 下移一项；末项不动。
    pub fn select_next(&mut self) {
        self.move_selection(1);
    }

    /// 上移一项；首项不动。
    pub fn select_previous(&mut self) {
        self.move_selection(-1);
    }

    /// 在展示顺序里移动选中项；边界不越界。
    fn move_selection(&mut self, step: isize) {
        let ids = self.flat_ids();
        if ids.is_empty() {
            self.selected_id = None;
            return;
        }
        let current = self
            .selected_id
            .as_deref()
            .and_then(|id| ids.iter().position(|x| *x == id));
        let index = match current {
            Some(index) => {
                let next = index as isize + step;
                next.clamp(0, ids.len() as isize - 1) as usize
            }
            None => 0,
        };
        self.selected_id = Some(ids[index].to_owned());
    }

    /// 执行选中条目：返回条目 id 并关闭面板。
    ///
    /// 无选中项（面板关闭 / 过滤为空）时返回 `None` 且保持原状态。
    pub fn execute(&mut self) -> Option<String> {
        if !self.open {
            return None;
        }
        let id = self.selected_id.clone()?;
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
    // 显式导入（不用 `use super::*`）：保持符号来源清晰。
    use super::search_state::demo_items;
    use super::{PaletteState, SearchItem, SearchSource};

    /// 带演示条目的状态（测试夹具）：演示数据只在测试构建里存在。
    fn opened() -> PaletteState {
        let mut state = PaletteState::with_items(demo_items());
        state.open();
        state
    }

    /// 带**真实条目**（内置命令表）的已打开状态：生产路径的样子。
    fn opened_real() -> PaletteState {
        let mut state = PaletteState::with_items(super::real_items());
        state.open();
        state
    }

    fn total(state: &PaletteState) -> usize {
        state.groups().iter().map(|group| group.items.len()).sum()
    }

    #[test]
    fn new_palette_is_closed() {
        let mut state = PaletteState::new();
        assert!(!state.is_open());
        assert_eq!(state.keyword(), "");
        assert_eq!(state.selected_id(), None);
        assert_eq!(state.execute(), None);
    }

    #[test]
    fn new_palette_has_no_items_at_all() {
        let mut state = PaletteState::new();
        state.open();
        assert_eq!(total(&state), 0, "生产路径不得自带任何条目");
        assert!(state.groups().is_empty(), "没有注入就没有分组");
        assert_eq!(state.selected_id(), None);
    }

    #[test]
    fn opening_selects_first_and_shows_all_items() {
        let state = opened();
        assert!(state.is_open());
        assert_eq!(total(&state), demo_items().len());
        assert_eq!(state.selected_id(), Some(demo_items()[0].id.as_str()));
    }

    #[test]
    fn opening_clears_previous_keyword() {
        let mut state = opened();
        state.set_keyword("会话");
        assert!(total(&state) < demo_items().len());
        state.close();
        state.open();
        assert_eq!(state.keyword(), "", "打开时必须清空上次关键词");
        assert_eq!(total(&state), demo_items().len());
        assert_eq!(state.selected_id(), Some(demo_items()[0].id.as_str()));
    }

    #[test]
    fn keyword_filters_across_sources() {
        let mut state = opened();
        state.set_keyword("app.settings");
        assert_eq!(total(&state), 1);
        assert_eq!(state.selected_id(), Some("app.settings"));
        state.set_keyword("设置");
        assert_eq!(total(&state), 1);
        assert_eq!(state.selected_id(), Some("app.settings"));
    }

    #[test]
    fn selection_stops_at_first_and_last() {
        let mut state = opened();
        let count = total(&state);
        assert_eq!(state.selected_index(), Some(0));
        state.select_previous();
        assert_eq!(state.selected_index(), Some(0), "首项上移不变");

        for _ in 0..count + 3 {
            state.select_next();
        }
        assert_eq!(state.selected_index(), Some(count - 1), "末项下移不变");
        state.select_next();
        assert_eq!(state.selected_index(), Some(count - 1));
    }

    #[test]
    fn selection_is_tracked_by_id_across_groups() {
        let mut state = opened();
        // 走过整个命令分组 ⇒ 落到第二个分组（技能）里：按 id 记选中才不会错位。
        let command_count = state
            .groups()
            .iter()
            .find(|g| g.source == SearchSource::Command)
            .expect("命令分组必须存在")
            .items
            .len();
        for _ in 0..command_count {
            state.select_next();
        }
        let id = state.selected_id().expect("必须有选中项").to_owned();
        let group = state
            .groups()
            .iter()
            .find(|g| g.items.iter().any(|i| i.id == id))
            .expect("选中项必须落在某个分组里");
        assert_eq!(group.source, SearchSource::Skill);
        let flat = state.items();
        let position = flat.iter().position(|i| i.id == id).unwrap();
        assert_eq!(
            state.selected_index(),
            Some(position),
            "下标必须与展示顺序一致"
        );
    }

    #[test]
    fn empty_result_has_no_selection_and_cannot_execute() {
        let mut state = opened();
        state.set_keyword("zzz-nothing");
        assert_eq!(total(&state), 0);
        assert!(state.groups().is_empty());
        assert_eq!(state.selected_id(), None);
        state.select_next();
        state.select_previous();
        assert_eq!(state.selected_id(), None);
        assert_eq!(state.execute(), None);
        assert!(state.is_open(), "没有可执行项时不得关闭或猜条目");
    }

    #[test]
    fn execute_returns_id_and_closes() {
        let mut state = opened();
        state.set_keyword("app.settings");
        let id = state.execute().expect("应能执行");
        assert_eq!(id, "app.settings", "派发依据是 id，不是标题");
        assert!(!state.is_open());
        assert_eq!(state.execute(), None, "关闭后不得再次执行");
    }

    #[test]
    fn execute_returns_non_command_ids_too() {
        let mut state = PaletteState::with_items(vec![SearchItem {
            source: SearchSource::Skill,
            id: "demo.skill.review".to_owned(),
            title: "代码审查".to_owned(),
            hint: None,
        }]);
        state.open();
        assert_eq!(state.execute().as_deref(), Some("demo.skill.review"));
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
        state.set_keyword("app.settings");
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

    // —— 注入真实条目（e28）：没有注入的来源**不出现** ——

    #[test]
    fn injected_commands_show_only_the_command_group() {
        let state = opened_real();
        let sources: Vec<SearchSource> = state.groups().iter().map(|g| g.source).collect();
        assert_eq!(sources, vec![SearchSource::Command], "只注入了命令");
        for source in [
            SearchSource::Skill,
            SearchSource::Mcp,
            SearchSource::File,
            SearchSource::Session,
        ] {
            assert!(
                !state.groups().iter().any(|g| g.source == source),
                "未注入的来源不得出现分组：{:?}",
                source
            );
        }
        let titles: Vec<&str> = state
            .groups()
            .iter()
            .flat_map(|g| g.items.iter())
            .map(|item| item.title.as_str())
            .collect();
        assert!(titles.contains(&"新建会话"), "真实命令标题必须可见");
        assert!(
            titles.iter().all(|t| !t.contains("演示数据")),
            "面板里不得出现演示数据字样"
        );
    }

    #[test]
    fn set_items_replaces_the_injected_set() {
        let mut state = opened_real();
        state.set_items(vec![SearchItem {
            source: SearchSource::File,
            id: "file.readme".to_owned(),
            title: "README".to_owned(),
            hint: None,
        }]);
        let sources: Vec<SearchSource> = state.groups().iter().map(|g| g.source).collect();
        assert_eq!(sources, vec![SearchSource::File], "换数据后分组随之变化");
    }
}
