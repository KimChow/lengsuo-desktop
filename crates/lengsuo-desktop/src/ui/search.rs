//! 全局搜索的**描述层门面**：多来源条目、来源种类、过滤与分组。
//!
//! 参照平台的命令面板是**命令 / 技能 / MCP / 文件 / 会话**五类来源的统一搜索。
//! 本模块只定义**条目形状 + 过滤 + 分组**；**数据由调用方注入**
//! （技能 / MCP / 文件 / 会话的真实数据源在运行时侧，⛔ 本仓不实现）。
//!
//! 数据形状住在**状态层**（[`crate::state::palette_state::search_state`]），
//! 本模块是它的门面：描述层（`ui`）的调用方不必知道形状定义在哪个层。
//!
//! ⚠️ 模块路径：文件物理落在 `ui/search.rs`，但 `ui/mod.rs` 是**共享区**
//! （本卡不得改）⇒ 由 `ui/command_palette/mod.rs` 用 `#[path]` 引入，
//! 模块路径是 `crate::ui::command_palette::search`。

pub use crate::state::palette_state::search_state::{
    command_items, demo_items, filter_items, flatten, group_by_source, SearchGroup, SearchItem,
    SearchSource, SearchState,
};

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：保持符号来源清晰。
    use super::{demo_items, filter_items, flatten, group_by_source, SearchItem, SearchSource};

    fn item(source: SearchSource, id: &str, title: &str) -> SearchItem {
        SearchItem {
            source,
            id: id.to_owned(),
            title: title.to_owned(),
            hint: None,
        }
    }

    #[test]
    fn the_facade_re_exports_the_state_layer_shapes() {
        let item: SearchItem = SearchItem {
            source: SearchSource::Command,
            id: "app.settings".to_owned(),
            title: "设置".to_owned(),
            hint: None,
        };
        let groups = group_by_source(&[item]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].source, SearchSource::Command);
        assert_eq!(groups[0].title, "命令");
    }

    #[test]
    fn flatten_keeps_group_then_item_order() {
        let items = vec![
            item(SearchSource::Session, "s1", "会话一"),
            item(SearchSource::Command, "c1", "命令一"),
            item(SearchSource::Command, "c2", "命令二"),
        ];
        let flat = flatten(&group_by_source(&items));
        let ids: Vec<&str> = flat.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["c1", "c2", "s1"]);
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

    #[test]
    fn filter_is_case_insensitive_on_title_or_id() {
        let items = vec![
            item(SearchSource::Command, "app.settings", "设置"),
            item(SearchSource::File, "demo.file.readme", "README"),
        ];
        assert_eq!(filter_items("readme", &items).len(), 1);
        assert_eq!(filter_items("APP.SETTINGS", &items).len(), 1);
        assert!(filter_items("zzz-nothing", &items).is_empty());
    }
}
