//! 环境分组 / 筛选视图状态（在 [`EnvListViewState`] 之上扩展）。
//!
//! 本模块**不重写**列表逻辑：条目解析、健康态、选中态仍由
//! [`EnvListViewState`] 负责；这里只增加三件事：
//!
//! - **分组归属**：读取服务端下发的 `group_id` / `group_name`；
//! - **分组健康**：每个分组内有多少条目解析失败（fail-closed 可见性）；
//! - **筛选**：按分组、按关键词过滤，且「筛选后为空」与「原始为空」可区分。
//!
//! 关键约定：
//! - **未分组是合法状态**，不是错误；
//! - `group_id` 缺失时**不猜归属**——即使带了 `group_name` 也归入未分组，
//!   因为没有稳定 id 就无法确认它们属于同一分组；
//! - 解析失败的条目按它**自带的** `group_id` 计入分组健康；没有 `group_id`
//!   的坏条目计入「未分组」，绝不按名字猜测。

use serde_json::Value;

use crate::state::env_list::{EnvEntry, EnvListViewState, EnvStatus};

/// 分组筛选目标。`All` 与 `Ungrouped` 都是合法选择。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GroupFilter {
    /// 不按分组筛选。
    #[default]
    All,
    /// 只显示没有分组归属的条目。
    Ungrouped,
    /// 只显示指定 `group_id` 的条目。
    Group(String),
}

impl GroupFilter {
    /// 某条目（按其分组 id，`None` 表示未分组）是否命中该筛选。
    #[must_use]
    pub fn matches(&self, group: Option<&str>) -> bool {
        match self {
            Self::All => true,
            Self::Ungrouped => group.is_none(),
            Self::Group(id) => group == Some(id.as_str()),
        }
    }
}

/// 一个分组的汇总。`name` 缺失时由渲染层显示明确占位，不拿 id 冒充名字。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupSummary {
    /// 服务端 `group_id`（字符串或数字的字符串形式）。
    pub id: String,
    /// 服务端 `group_name`；缺失/空白时为 `None`。
    pub name: Option<String>,
    /// 该分组内可展示的条目数。
    pub env_count: usize,
    /// 该分组内解析失败的条目数。
    pub invalid_count: usize,
}

/// 原始数据为空、还是筛选把数据全部排除了？两者必须能区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyReason {
    /// 原始结果一条都没有（含解析失败的条目）。
    NoData,
    /// 原始有数据，但当前分组 / 关键词筛选把可展示条目全部排除。
    FilteredOut,
}

/// 环境分组视图状态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvGroupState {
    list: EnvListViewState,
    groups: Vec<GroupSummary>,
    ungrouped_count: usize,
    ungrouped_invalid: usize,
    raw_count: usize,
    filter: GroupFilter,
    keyword: String,
}

impl EnvGroupState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 底层列表状态（只读）。
    #[must_use]
    pub fn list(&self) -> &EnvListViewState {
        &self.list
    }

    /// 全部可展示条目（不受筛选影响）。
    #[must_use]
    pub fn entries(&self) -> &[EnvEntry] {
        self.list.entries()
    }

    /// 最近一次注入的原始条目数（含解析失败的条目）。
    #[must_use]
    pub fn raw_count(&self) -> usize {
        self.raw_count
    }

    /// 解析失败条目总数，直接沿用 [`EnvListViewState::invalid_count`]。
    #[must_use]
    pub fn invalid_count(&self) -> usize {
        self.list.invalid_count()
    }

    #[must_use]
    pub fn status(&self) -> EnvStatus {
        self.list.status()
    }

    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.list.selected()
    }

    #[must_use]
    pub fn last_error(&self) -> Option<&str> {
        self.list.last_error()
    }

    /// 选中条目；不存在时不改变选中态（复用列表状态的行为）。
    pub fn select(&mut self, id: &str) {
        self.list.select(id);
    }

    /// 标记开始加载：不清空已有条目与分组汇总（避免界面闪空）。
    pub fn begin_load(&mut self) {
        self.list.begin_load();
    }

    /// 记录加载失败：清空条目与分组健康数据。
    pub fn fail(&mut self, message: impl Into<String>) {
        self.list.fail(message);
        self.groups.clear();
        self.ungrouped_count = 0;
        self.ungrouped_invalid = 0;
        self.raw_count = 0;
    }

    /// 注入一批原始条目：先统计分组，再交给 [`EnvListViewState::apply_entries`]。
    pub fn apply_entries(&mut self, raw_entries: Vec<Value>) {
        let mut groups: Vec<GroupSummary> = Vec::new();
        let mut ungrouped_invalid = 0;

        for raw in &raw_entries {
            let valid = has_id(raw);
            match group_of(raw) {
                Some(group) => {
                    note_group(&mut groups, &group);
                    if !valid {
                        if let Some(summary) = groups.iter_mut().find(|g| g.id == group.id) {
                            summary.invalid_count += 1;
                        }
                    }
                }
                // group_id 缺失：不猜归属，坏条目同样计入未分组。
                None if !valid => ungrouped_invalid += 1,
                None => {}
            }
        }

        self.raw_count = raw_entries.len();
        self.list.apply_entries(raw_entries);

        // 可展示条目按分组计数。分组元数据已由上面的原始遍历建立。
        let mut ungrouped_count = 0;
        for entry in self.list.entries() {
            match group_id_of(&entry.raw) {
                Some(id) => {
                    if let Some(summary) = groups.iter_mut().find(|g| g.id == id) {
                        summary.env_count += 1;
                    }
                }
                None => ungrouped_count += 1,
            }
        }

        self.groups = groups;
        self.ungrouped_count = ungrouped_count;
        self.ungrouped_invalid = ungrouped_invalid;
        self.reconcile_filter();
    }

    /// 左侧分组栏的数据源（按服务端首次出现顺序）。
    #[must_use]
    pub fn groups(&self) -> &[GroupSummary] {
        &self.groups
    }

    /// 按 id 查分组（不做模糊匹配）。
    #[must_use]
    pub fn group(&self, id: &str) -> Option<&GroupSummary> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// 未分组且可展示的条目数。
    #[must_use]
    pub fn ungrouped_count(&self) -> usize {
        self.ungrouped_count
    }

    /// 未分组且解析失败的条目数。
    #[must_use]
    pub fn ungrouped_invalid_count(&self) -> usize {
        self.ungrouped_invalid
    }

    /// 条目所属分组；`None` 表示未分组（或分组信息缺失，已按未分组处理）。
    #[must_use]
    pub fn group_of_entry(&self, entry: &EnvEntry) -> Option<&GroupSummary> {
        let id = group_id_of(&entry.raw)?;
        self.groups.iter().find(|g| g.id == id)
    }

    #[must_use]
    pub fn group_filter(&self) -> &GroupFilter {
        &self.filter
    }

    /// 设置分组筛选。指向不存在的分组时保持原样：刷新后该分组若仍不存在，
    /// [`Self::apply_entries`] 会回退到 `All`，不会留下永久空列表的幽灵筛选。
    pub fn set_group_filter(&mut self, filter: GroupFilter) {
        self.filter = filter;
    }

    #[must_use]
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// 设置关键词；比较时忽略首尾空白与大小写。
    pub fn set_keyword(&mut self, keyword: impl Into<String>) {
        self.keyword = keyword.into();
    }

    /// 清空关键词（保留分组筛选）。
    pub fn clear_keyword(&mut self) {
        self.keyword.clear();
    }

    /// 清空全部筛选条件。
    pub fn clear_filter(&mut self) {
        self.filter = GroupFilter::All;
        self.keyword.clear();
    }

    /// 是否存在有效筛选（纯空白关键词不算）。
    #[must_use]
    pub fn is_filter_active(&self) -> bool {
        !matches!(self.filter, GroupFilter::All) || !self.keyword.trim().is_empty()
    }

    /// 当前筛选下的可展示条目。
    #[must_use]
    pub fn visible_entries(&self) -> Vec<&EnvEntry> {
        let needle = self.keyword.trim().to_lowercase();
        self.list
            .entries()
            .iter()
            .filter(|entry| {
                self.filter.matches(group_id_of(&entry.raw).as_deref())
                    && (needle.is_empty()
                        || entry
                            .name
                            .as_deref()
                            .is_some_and(|name| name.to_lowercase().contains(needle.as_str())))
            })
            .collect()
    }

    /// 当前筛选下的条目数。
    #[must_use]
    pub fn visible_count(&self) -> usize {
        self.visible_entries().len()
    }

    /// 空结果的原因；`None` 表示还有可展示条目，或当前不应显示空态
    /// （加载中 / 失败 / 全部条目解析失败——这些情况由状态与降级提示表达）。
    #[must_use]
    pub fn empty_reason(&self) -> Option<EmptyReason> {
        if !self.visible_entries().is_empty() {
            return None;
        }
        if matches!(self.list.status(), EnvStatus::Loading | EnvStatus::Failed) {
            return None;
        }
        if self.raw_count == 0 {
            Some(EmptyReason::NoData)
        } else if self.is_filter_active() {
            Some(EmptyReason::FilteredOut)
        } else {
            // 原始有条目、但没有一条能展示，且不是筛选造成：这是降级，
            // 不能谎称「没有数据」。
            None
        }
    }

    /// 分组在刷新后消失时回退到 `All`，避免筛选永远匹配不到任何条目。
    fn reconcile_filter(&mut self) {
        if let GroupFilter::Group(id) = &self.filter {
            if !self.groups.iter().any(|g| &g.id == id) {
                self.filter = GroupFilter::All;
            }
        }
    }
}

/// 分组原始信息：id 必有，name 可能缺失。
struct GroupRef {
    id: String,
    name: Option<String>,
}

/// 记录分组元数据；同名 id 只保留**首个非空**名字，不覆盖已有名字。
fn note_group(groups: &mut Vec<GroupSummary>, group: &GroupRef) {
    if let Some(summary) = groups.iter_mut().find(|g| g.id == group.id) {
        if summary.name.is_none() {
            summary.name = group.name.clone();
        }
        return;
    }
    groups.push(GroupSummary {
        id: group.id.clone(),
        name: group.name.clone(),
        env_count: 0,
        invalid_count: 0,
    });
}

/// 与 [`crate::state::env_list`] 的 id 抽取规则保持一致，仅用于判断条目
/// 是否会计入 `EnvListViewState::invalid_count()`。
fn has_id(raw: &Value) -> bool {
    ["id", "env_id", "uuid"].iter().any(|key| {
        matches!(
            raw.get(*key),
            Some(Value::String(s)) if !s.is_empty()
        ) || matches!(raw.get(*key), Some(Value::Number(_)))
    })
}

fn group_id_of(raw: &Value) -> Option<String> {
    raw.get("group_id").and_then(scalar_id)
}

fn scalar_id(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn group_of(raw: &Value) -> Option<GroupRef> {
    let id = group_id_of(raw)?;
    let name = raw
        .get("group_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned);
    Some(GroupRef { id, name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::env_list::EnvStatus;
    use serde_json::json;

    #[test]
    fn ungrouped_is_a_legal_state_not_an_error() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"id": "e1", "name": "测试机"})]);
        assert_eq!(s.status(), EnvStatus::Ready);
        assert_eq!(s.invalid_count(), 0);
        assert_eq!(s.ungrouped_count(), 1);
        assert_eq!(s.ungrouped_invalid_count(), 0);
        assert!(s.groups().is_empty());
        assert!(s.group_of_entry(&s.entries()[0]).is_none());

        s.set_group_filter(GroupFilter::Ungrouped);
        assert_eq!(s.visible_count(), 1);
        assert_eq!(s.empty_reason(), None);
    }

    #[test]
    fn missing_group_id_does_not_guess_ownership_from_name() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({
            "id": "e1",
            "group_name": "华东组",
        })]);
        assert!(
            s.groups().is_empty(),
            "没有 group_id 时不得凭 group_name 猜测分组"
        );
        assert_eq!(s.ungrouped_count(), 1);
        assert!(s.group_of_entry(&s.entries()[0]).is_none());
    }

    #[test]
    fn group_id_and_name_are_read_from_server_fields() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![
            json!({"id": "e1", "group_id": "g1", "group_name": "华东组"}),
            json!({"id": "e2", "group_id": 7, "group_name": "华南组"}),
            json!({"id": "e3", "group_id": "g1"}),
        ]);
        let groups = s.groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, "g1");
        assert_eq!(groups[0].name.as_deref(), Some("华东组"));
        assert_eq!(groups[0].env_count, 2);
        assert_eq!(groups[1].id, "7");
        assert_eq!(groups[1].name.as_deref(), Some("华南组"));
        assert_eq!(groups[1].env_count, 1);
        assert_eq!(s.ungrouped_count(), 0);
    }

    #[test]
    fn group_without_name_keeps_identity_and_placeholder_is_ui_concern() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"id": "e1", "group_id": "g1"})]);
        let group = &s.groups()[0];
        assert_eq!(group.id, "g1");
        assert_eq!(group.name, None);
        assert_eq!(group.env_count, 1);
    }

    #[test]
    fn invalid_entries_count_toward_their_group_health() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![
            json!({"id": "e1", "group_id": "g1"}),
            json!({"name": "坏条目", "group_id": "g1"}),
            json!({"name": "无分组的坏条目"}),
            json!({"name": "只有组名的坏条目", "group_name": "猜不出来的组"}),
        ]);
        assert_eq!(s.status(), EnvStatus::Degraded);
        assert_eq!(s.invalid_count(), 3);
        let g1 = s.group("g1").unwrap();
        assert_eq!(g1.env_count, 1);
        assert_eq!(g1.invalid_count, 1);
        assert_eq!(s.ungrouped_invalid_count(), 2);
        assert_eq!(s.groups().len(), 1, "坏条目不得凭 group_name 造出分组");
    }

    #[test]
    fn per_group_invalid_counts_add_up_to_list_invalid_count() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![
            json!({"id": "a", "group_id": "g1"}),
            json!({"group_id": "g1"}),
            json!({"group_id": "g2"}),
            json!({}),
            json!({"env_id": 3, "group_id": "g2"}),
        ]);
        let grouped: usize = s.groups().iter().map(|g| g.invalid_count).sum();
        assert_eq!(
            grouped + s.ungrouped_invalid_count(),
            s.invalid_count(),
            "分组健康数据不得与列表的 invalid_count 脱节"
        );
    }

    #[test]
    fn group_filter_and_ungrouped_filter_select_disjoint_sets() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![
            json!({"id": "e1", "name": "甲", "group_id": "g1"}),
            json!({"id": "e2", "name": "乙", "group_id": "g2"}),
            json!({"id": "e3", "name": "丙"}),
            json!({"id": "e4", "name": "丁"}),
        ]);
        s.set_group_filter(GroupFilter::Group("g1".into()));
        let ids: Vec<&str> = s.visible_entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["e1"]);

        s.set_group_filter(GroupFilter::Ungrouped);
        let ids: Vec<&str> = s.visible_entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["e3", "e4"]);

        s.set_group_filter(GroupFilter::All);
        assert_eq!(s.visible_count(), 4);
    }

    #[test]
    fn keyword_is_trimmed_and_case_insensitive() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![
            json!({"id": "e1", "name": "Build-Win"}),
            json!({"id": "e2", "name": "build-linux"}),
            json!({"id": "e3", "name": "测试机"}),
        ]);
        s.set_keyword("  BUILD ");
        let ids: Vec<&str> = s.visible_entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["e1", "e2"]);

        s.set_keyword("测试");
        let ids: Vec<&str> = s.visible_entries().iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["e3"]);

        s.set_keyword("   ");
        assert!(!s.is_filter_active(), "纯空白关键词不构成筛选");
        assert_eq!(s.visible_count(), 3);
    }

    #[test]
    fn keyword_never_matches_the_id_or_an_absent_name() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"id": "needle-id"})]);
        s.set_keyword("needle");
        assert_eq!(
            s.visible_count(),
            0,
            "名称缺失时不得拿 id 当搜索目标或展示名"
        );
        assert_eq!(s.empty_reason(), Some(EmptyReason::FilteredOut));
    }

    #[test]
    fn filtered_empty_is_distinguishable_from_original_empty() {
        let mut empty = EnvGroupState::new();
        empty.apply_entries(vec![]);
        assert_eq!(empty.empty_reason(), Some(EmptyReason::NoData));

        let mut filtered = EnvGroupState::new();
        filtered.apply_entries(vec![json!({"id": "e1", "name": "甲"})]);
        filtered.set_keyword("不存在的关键词");
        assert_eq!(filtered.visible_count(), 0);
        assert_eq!(
            filtered.empty_reason(),
            Some(EmptyReason::FilteredOut),
            "筛选后为空不得与原始为空混为一谈"
        );
        assert_eq!(filtered.entries().len(), 1, "原始数据仍在");
    }

    #[test]
    fn all_invalid_data_is_degraded_not_a_plain_empty_state() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"name": "坏条目"})]);
        assert_eq!(s.status(), EnvStatus::Degraded);
        assert_eq!(s.raw_count(), 1);
        assert_eq!(s.visible_count(), 0);
        assert_eq!(
            s.empty_reason(),
            None,
            "全部解析失败时由降级提示表达，不得谎称没有数据"
        );
    }

    #[test]
    fn a_group_with_only_invalid_entries_is_still_listed() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"group_id": "g1", "group_name": "华东组"})]);
        let group = s.group("g1").unwrap();
        assert_eq!(group.env_count, 0);
        assert_eq!(group.invalid_count, 1);
    }

    #[test]
    fn group_filter_falls_back_when_the_group_disappears() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"id": "e1", "group_id": "g1"})]);
        s.set_group_filter(GroupFilter::Group("g1".into()));
        s.apply_entries(vec![json!({"id": "e2"})]);
        assert_eq!(
            s.group_filter(),
            &GroupFilter::All,
            "分组消失后不得留下永久匹配不到的幽灵筛选"
        );
        assert_eq!(s.visible_count(), 1);
    }

    #[test]
    fn group_filter_is_kept_while_the_group_still_exists() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"id": "e1", "group_id": "g1"})]);
        s.set_group_filter(GroupFilter::Group("g1".into()));
        s.apply_entries(vec![
            json!({"id": "e2", "group_id": "g1"}),
            json!({"id": "e3"}),
        ]);
        assert_eq!(s.group_filter(), &GroupFilter::Group("g1".into()));
        assert_eq!(s.visible_count(), 1);
    }

    #[test]
    fn fail_clears_group_health_and_raw_count() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![
            json!({"id": "e1", "group_id": "g1"}),
            json!({"group_id": "g1"}),
        ]);
        s.fail("gateway unreachable");
        assert_eq!(s.status(), EnvStatus::Failed);
        assert!(s.groups().is_empty());
        assert_eq!(s.ungrouped_count(), 0);
        assert_eq!(s.ungrouped_invalid_count(), 0);
        assert_eq!(s.raw_count(), 0);
        assert_eq!(s.empty_reason(), None);
    }

    #[test]
    fn begin_load_keeps_entries_and_group_summaries() {
        let mut s = EnvGroupState::new();
        s.apply_entries(vec![json!({"id": "e1", "group_id": "g1"})]);
        s.begin_load();
        assert_eq!(s.status(), EnvStatus::Loading);
        assert_eq!(s.entries().len(), 1);
        assert_eq!(s.groups().len(), 1);
        assert_eq!(s.empty_reason(), None);
    }
}
