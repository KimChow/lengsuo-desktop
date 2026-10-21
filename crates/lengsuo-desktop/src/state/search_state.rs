//! 全局搜索：条目形状、过滤、分组与状态（纯逻辑，不依赖 GPUI）。
//!
//! 参照平台的命令面板是**命令 / 技能 / MCP / 文件 / 会话**五类来源的统一搜索。
//! 本模块定义**条目形状 + 过滤 + 分组 + 状态**；**数据由调用方注入**
//! （技能 / MCP / 文件 / 会话的真实数据源在运行时侧，⛔ 本仓不实现）。
//!
//! ⚠️ 模块路径：文件物理落在 `state/search_state.rs`，但 `state/mod.rs` 是**共享区**
//! （本卡不得改）⇒ 由 `state/palette_state.rs` 用 `#[path]` 引入，
//! 模块路径是 `crate::state::palette_state::search_state`。
//!
//! 分层：数据形状放在**状态层**（本模块）而不是 `ui` 层 ⇒ `state` 不反向依赖
//! `ui`（`ui → state` 是仓库既有方向，见 `ui/layout.rs` 对 `state::session_stream` 的引用）。

use crate::theme::palette::{matches_keyword, COMMANDS};

/// 搜索来源种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SearchSource {
    /// 命令（内置命令表）。
    Command,
    /// 技能。
    Skill,
    /// MCP 服务器 / 工具。
    Mcp,
    /// 文件。
    File,
    /// 会话。
    Session,
}

impl SearchSource {
    /// 全部来源（固定顺序 = 展示分组顺序）。
    pub const ALL: [SearchSource; 5] = [
        SearchSource::Command,
        SearchSource::Skill,
        SearchSource::Mcp,
        SearchSource::File,
        SearchSource::Session,
    ];

    /// 来源分组的展示标题（中文）。
    #[must_use]
    pub const fn group_label(self) -> &'static str {
        match self {
            SearchSource::Command => "命令",
            SearchSource::Skill => "技能",
            SearchSource::Mcp => "MCP",
            SearchSource::File => "文件",
            SearchSource::Session => "会话",
        }
    }
}

/// 一条可搜索的条目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchItem {
    /// 来源种类（决定分组）。
    pub source: SearchSource,
    /// 稳定标识（**派发用 id，不用标题派发**）。
    pub id: String,
    /// 展示标题。
    pub title: String,
    /// 右侧提示（可选，如快捷键、路径）。
    pub hint: Option<String>,
}

/// 一个来源分组（渲染时一个分组一个标题）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchGroup {
    /// 分组所属来源。
    pub source: SearchSource,
    /// 分组标题（中文）。
    pub title: &'static str,
    /// 分组内的条目。
    pub items: Vec<SearchItem>,
}

/// 按关键词过滤多来源条目：大小写不敏感的子串匹配（匹配标题或 id）。
///
/// 匹配规则与命令过滤**同一条**（[`crate::theme::palette::matches_keyword`]）
/// ⇒ 命令与其它来源不会出现两套口径。
#[must_use]
pub fn filter_items(keyword: &str, items: &[SearchItem]) -> Vec<SearchItem> {
    items
        .iter()
        .filter(|i| matches_keyword(keyword, &i.title, &i.id))
        .cloned()
        .collect()
}

/// 把过滤后的条目按来源分组；**空分组不出现**。
#[must_use]
pub fn group_by_source(items: &[SearchItem]) -> Vec<SearchGroup> {
    let mut groups: Vec<SearchGroup> = Vec::new();
    for source in SearchSource::ALL {
        let group_items: Vec<SearchItem> = items
            .iter()
            .filter(|i| i.source == source)
            .cloned()
            .collect();
        if group_items.is_empty() {
            continue;
        }
        groups.push(SearchGroup {
            source,
            title: source.group_label(),
            items: group_items,
        });
    }
    groups
}

/// 把分组摊平成**展示顺序**的条目列表（分组顺序 → 组内顺序）。
///
/// 选中项的上下移动按这个顺序走，与屏幕上看到的顺序一致。
#[must_use]
pub fn flatten(groups: &[SearchGroup]) -> Vec<SearchItem> {
    groups
        .iter()
        .flat_map(|g| g.items.iter().cloned())
        .collect()
}

/// 命令来源的条目：来自**内置命令表**（真实数据）。
#[must_use]
pub fn command_items() -> Vec<SearchItem> {
    COMMANDS
        .iter()
        .map(|command| SearchItem {
            source: SearchSource::Command,
            id: command.id.to_owned(),
            title: command.title.to_owned(),
            hint: None,
        })
        .collect()
}

/// 演示条目集合：命令来自内置命令表，其余四类是**演示数据**。
///
/// 技能 / MCP / 文件 / 会话的真实数据源在运行时侧，⛔ 本仓不实现；
/// 这里只提供**形状正确**的占位条目，让面板能按五类来源分组展示。
#[must_use]
pub fn demo_items() -> Vec<SearchItem> {
    let mut items = command_items();
    items.extend([
        SearchItem {
            source: SearchSource::Skill,
            id: "demo.skill.review".to_owned(),
            title: "代码审查".to_owned(),
            hint: Some("演示数据".to_owned()),
        },
        SearchItem {
            source: SearchSource::Skill,
            id: "demo.skill.commit-message".to_owned(),
            title: "生成提交信息".to_owned(),
            hint: Some("演示数据".to_owned()),
        },
        SearchItem {
            source: SearchSource::Mcp,
            id: "demo.mcp.filesystem".to_owned(),
            title: "filesystem".to_owned(),
            hint: Some("演示数据".to_owned()),
        },
        SearchItem {
            source: SearchSource::Mcp,
            id: "demo.mcp.git".to_owned(),
            title: "git".to_owned(),
            hint: Some("演示数据".to_owned()),
        },
        SearchItem {
            source: SearchSource::File,
            id: "demo.file.search-rs".to_owned(),
            title: "src/ui/search.rs".to_owned(),
            hint: Some("演示数据".to_owned()),
        },
        SearchItem {
            source: SearchSource::Session,
            id: "demo.session.palette-refactor".to_owned(),
            title: "重构命令面板".to_owned(),
            hint: Some("演示数据".to_owned()),
        },
    ]);
    items
}

/// 全局搜索状态：条目集合 + 关键词 + 过滤分组结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchState {
    /// 全部可搜索条目（注入）。
    items: Vec<SearchItem>,
    /// 当前关键词。
    keyword: String,
}

impl SearchState {
    /// 用注入的条目建状态（关键词为空 ⇒ 全部命中）。
    #[must_use]
    pub fn new(items: Vec<SearchItem>) -> Self {
        Self {
            items,
            keyword: String::new(),
        }
    }

    /// 全部条目（只读）。
    #[must_use]
    pub fn items(&self) -> &[SearchItem] {
        &self.items
    }

    /// 当前关键词。
    #[must_use]
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// 设置关键词。
    pub fn set_keyword(&mut self, keyword: impl Into<String>) {
        self.keyword = keyword.into();
    }

    /// 过滤 + 分组后的结果（空分组不出现）。
    #[must_use]
    pub fn groups(&self) -> Vec<SearchGroup> {
        group_by_source(&filter_items(&self.keyword, &self.items))
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：保持符号来源清晰。
    use super::{
        command_items, demo_items, filter_items, group_by_source, SearchItem, SearchSource,
        SearchState,
    };

    fn item(source: SearchSource, id: &str, title: &str) -> SearchItem {
        SearchItem {
            source,
            id: id.to_owned(),
            title: title.to_owned(),
            hint: None,
        }
    }

    #[test]
    fn search_state_starts_with_all_items() {
        let items = vec![
            item(SearchSource::Command, "app.settings", "设置"),
            item(SearchSource::File, "demo.file.readme", "README"),
        ];
        let state = SearchState::new(items);
        assert_eq!(state.keyword(), "");
        assert_eq!(state.items().len(), 2);
        let groups = state.groups();
        let total: usize = groups.iter().map(|g| g.items.len()).sum();
        assert_eq!(total, 2, "空关键词时全部条目都在结果里");
    }

    #[test]
    fn search_state_groups_reflect_keyword() {
        let items = vec![
            item(SearchSource::Command, "app.settings", "设置"),
            item(SearchSource::File, "demo.file.readme", "README"),
        ];
        let mut state = SearchState::new(items);
        state.set_keyword("readme");
        let groups = state.groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].source, SearchSource::File);
        assert_eq!(groups[0].items[0].id, "demo.file.readme");

        state.set_keyword("zzz-nothing");
        assert!(state.groups().is_empty(), "无命中时没有任何分组");
    }

    #[test]
    fn command_items_come_from_the_builtin_table() {
        let items = command_items();
        assert_eq!(items.len(), crate::theme::palette::COMMANDS.len());
        assert!(items.iter().all(|i| i.source == SearchSource::Command));
        assert!(items.iter().all(|i| i.hint.is_none()));
    }

    #[test]
    fn filter_keeps_items_matching_title_or_id() {
        let items = vec![
            item(SearchSource::Command, "app.settings", "设置"),
            item(SearchSource::File, "demo.file.readme", "README"),
        ];
        assert_eq!(filter_items("设置", &items).len(), 1);
        assert_eq!(filter_items("app.", &items).len(), 1);
        assert_eq!(filter_items("", &items).len(), 2);
    }

    #[test]
    fn grouping_drops_empty_sources() {
        let items = vec![item(SearchSource::Session, "s1", "会话一")];
        let groups = group_by_source(&items);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].source, SearchSource::Session);
    }

    #[test]
    fn groups_keep_source_order_and_drop_empty() {
        let items = vec![
            item(SearchSource::Session, "s1", "会话一"),
            item(SearchSource::Command, "c1", "命令一"),
            item(SearchSource::File, "f1", "文件一"),
        ];
        let groups = group_by_source(&items);
        let sources: Vec<SearchSource> = groups.iter().map(|g| g.source).collect();
        assert_eq!(
            sources,
            vec![
                SearchSource::Command,
                SearchSource::File,
                SearchSource::Session
            ],
            "分组顺序必须等于 SearchSource::ALL 的顺序"
        );
        assert!(groups.iter().all(|g| !g.items.is_empty()), "空分组不出现");
    }

    #[test]
    fn group_labels_are_chinese() {
        let labels: Vec<&str> = SearchSource::ALL.iter().map(|s| s.group_label()).collect();
        assert_eq!(
            labels,
            vec!["命令", "技能", "MCP", "文件", "会话"],
            "五个来源的分组标题必须非空且互不相同"
        );
        let mut unique = labels.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), labels.len(), "分组标题必须互不相同");
    }

    #[test]
    fn empty_keyword_returns_all_items() {
        let items = demo_items();
        assert_eq!(filter_items("", &items).len(), items.len());
        assert_eq!(
            filter_items("   ", &items).len(),
            items.len(),
            "全是空白等同空关键词"
        );
    }

    #[test]
    fn filter_matches_title_or_id_case_insensitively() {
        let items = vec![
            item(SearchSource::Command, "app.settings", "设置"),
            item(SearchSource::File, "demo.file.readme", "README"),
        ];
        // 按标题命中（大小写不敏感）。
        let hit = filter_items("readme", &items);
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].id, "demo.file.readme");
        // 按 id 命中。
        let by_id = filter_items("APP.SETTINGS", &items);
        assert_eq!(by_id.len(), 1);
        assert_eq!(by_id[0].id, "app.settings");
        // 都不命中。
        assert!(filter_items("zzz-nothing", &items).is_empty());
    }

    #[test]
    fn demo_items_cover_all_five_sources() {
        let items = demo_items();
        for source in SearchSource::ALL {
            assert!(
                items.iter().any(|i| i.source == source),
                "演示数据必须覆盖来源：{:?}",
                source
            );
        }
    }
}
