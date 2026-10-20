//! 环境视图渲染模型：卡片网格 + 分组侧栏 + 降级提示。
//!
//! 本模块只把 [`EnvGroupState`] 转成可直接绘制的卡片 / 侧栏行，**不依赖 GPUI**，
//! 便于在无窗口环境下测试渲染语义。
//!
//! 约定：
//! - 环境名缺失时显示 [`UNNAMED_ENV`]，**绝不拿 id 猜名字**；
//! - 分组名缺失时显示 [`UNNAMED_GROUP`]，同样不拿 id 冒充；
//! - 未分组显示 [`UNGROUPED_LABEL`]，这是合法状态而非错误；
//! - `invalid_count() > 0` 时 [`EnvView::degraded_notice`] 必为 `Some`，
//!   渲染层必须原样展示，不得静默吞掉坏条目。

use crate::state::env_group::{EmptyReason, EnvGroupState, GroupFilter, GroupSummary};
use crate::state::env_list::{EnvEntry, EnvStatus};

/// 环境名缺失时的占位文案（不是 id）。
pub const UNNAMED_ENV: &str = "未命名环境";
/// 分组名缺失时的占位文案（不是 id）。
pub const UNNAMED_GROUP: &str = "未命名分组";
/// 未分组条目的显示名。
pub const UNGROUPED_LABEL: &str = "未分组";
/// 侧栏「全部环境」行。
pub const ALL_GROUPS_LABEL: &str = "全部环境";

/// 卡片上的环境状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardStatus {
    /// 就绪。
    Ready,
    /// 降级：列表中仍有可展示条目，但存在读取异常的数据。
    Degraded,
    /// 异常：最近一次加载失败。
    Failed,
}

impl CardStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "就绪",
            Self::Degraded => "降级",
            Self::Failed => "异常",
        }
    }
}

/// 卡片所属分组的展示信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupLabel {
    /// `None` 表示未分组。
    pub id: Option<String>,
    /// 分组显示名；未分组时为 [`UNGROUPED_LABEL`]，名字缺失时为 [`UNNAMED_GROUP`]。
    pub name: String,
    /// `name` 是否为占位文案（分组 id 存在但名字缺失）。
    pub is_placeholder: bool,
}

impl GroupLabel {
    #[must_use]
    fn ungrouped() -> Self {
        Self {
            id: None,
            name: UNGROUPED_LABEL.to_owned(),
            is_placeholder: false,
        }
    }

    #[must_use]
    fn from_summary(summary: &GroupSummary) -> Self {
        let name = summary
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty());
        Self {
            id: Some(summary.id.clone()),
            name: name.unwrap_or(UNNAMED_GROUP).to_owned(),
            is_placeholder: name.is_none(),
        }
    }
}

/// 单张环境卡片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvCard {
    pub id: String,
    /// 展示名；原始名称缺失/空白时为 [`UNNAMED_ENV`]。
    pub name: String,
    /// `name` 是否为占位文案。
    pub name_is_placeholder: bool,
    pub status: CardStatus,
    pub group: GroupLabel,
}

impl EnvCard {
    #[must_use]
    fn from_entry(entry: &EnvEntry, group: Option<&GroupSummary>, status: CardStatus) -> Self {
        let name = entry
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty());
        Self {
            id: entry.id.clone(),
            name: name.unwrap_or(UNNAMED_ENV).to_owned(),
            name_is_placeholder: name.is_none(),
            status,
            group: group.map_or_else(GroupLabel::ungrouped, GroupLabel::from_summary),
        }
    }
}

/// 分组侧栏中的一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRow {
    /// 点击该行时应设置的筛选。
    pub filter: GroupFilter,
    pub label: String,
    pub env_count: usize,
    pub invalid_count: usize,
    pub selected: bool,
}

/// 完整的环境视图模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvView {
    pub status: EnvStatus,
    pub loading: bool,
    /// 失败原因；仅失败态为 `Some`。
    pub error: Option<String>,
    /// 当前筛选下应展示的卡片。
    pub cards: Vec<EnvCard>,
    /// 左侧分组栏。
    pub sidebar: Vec<GroupRow>,
    /// 非空时必须原样展示（`invalid_count() > 0` 时必为 `Some`）。
    pub degraded_notice: Option<String>,
    /// 空态原因；`None` 表示不展示空态。
    pub empty: Option<EmptyReason>,
    /// 空态文案（与 `empty` 同时出现）。
    pub empty_text: Option<&'static str>,
    pub total_count: usize,
    pub visible_count: usize,
    pub invalid_count: usize,
    pub selected: Option<String>,
}

impl EnvView {
    /// 卡片网格的列数提示：0 张卡片时为 0，避免渲染层画空网格。
    #[must_use]
    pub fn card_count(&self) -> usize {
        self.cards.len()
    }
}

/// 渲染整个环境视图。
#[must_use]
pub fn render(state: &EnvGroupState) -> EnvView {
    let status = state.status();
    let cards: Vec<EnvCard> = state
        .visible_entries()
        .into_iter()
        .map(|entry| EnvCard::from_entry(entry, state.group_of_entry(entry), card_status(status)))
        .collect();
    let empty = state.empty_reason();
    EnvView {
        status,
        loading: status == EnvStatus::Loading,
        error: state.last_error().map(str::to_owned),
        cards,
        sidebar: group_rows(state),
        degraded_notice: degraded_notice(state),
        empty,
        empty_text: empty.map(empty_text),
        total_count: state.entries().len(),
        visible_count: state.visible_count(),
        invalid_count: state.invalid_count(),
        selected: state.selected().map(str::to_owned),
    }
}

/// 降级提示。`invalid_count() > 0` 时返回「有 N 条数据读取异常」。
#[must_use]
pub fn degraded_notice(state: &EnvGroupState) -> Option<String> {
    let invalid = state.invalid_count();
    (invalid > 0).then(|| format!("有 {invalid} 条数据读取异常"))
}

/// 空态文案：真实空数据与筛选无结果必须使用不同文案。
#[must_use]
pub const fn empty_text(reason: EmptyReason) -> &'static str {
    match reason {
        EmptyReason::NoData => "暂无环境",
        EmptyReason::FilteredOut => "没有符合筛选条件的环境",
    }
}

/// 分组侧栏：「全部环境」+ 服务端分组 + 「未分组」。
///
/// 分组名缺失时用 [`UNNAMED_GROUP`] 占位，不展示 id 冒充名字。
#[must_use]
pub fn group_rows(state: &EnvGroupState) -> Vec<GroupRow> {
    let current = state.group_filter();
    let mut rows = Vec::with_capacity(state.groups().len() + 2);
    rows.push(GroupRow {
        filter: GroupFilter::All,
        label: ALL_GROUPS_LABEL.to_owned(),
        env_count: state.entries().len(),
        invalid_count: state.invalid_count(),
        selected: matches!(current, GroupFilter::All),
    });
    rows.extend(state.groups().iter().map(|group| {
        GroupRow {
            filter: GroupFilter::Group(group.id.clone()),
            label: group
                .name
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or(UNNAMED_GROUP)
                .to_owned(),
            env_count: group.env_count,
            invalid_count: group.invalid_count,
            selected: matches!(current, GroupFilter::Group(id) if id == &group.id),
        }
    }));
    rows.push(GroupRow {
        filter: GroupFilter::Ungrouped,
        label: UNGROUPED_LABEL.to_owned(),
        env_count: state.ungrouped_count(),
        invalid_count: state.ungrouped_invalid_count(),
        selected: matches!(current, GroupFilter::Ungrouped),
    });
    rows
}

fn card_status(status: EnvStatus) -> CardStatus {
    match status {
        EnvStatus::Failed => CardStatus::Failed,
        EnvStatus::Degraded => CardStatus::Degraded,
        EnvStatus::Loading | EnvStatus::Ready => CardStatus::Ready,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::env_group::EnvGroupState;
    use serde_json::json;

    fn state_with(entries: Vec<serde_json::Value>) -> EnvGroupState {
        let mut state = EnvGroupState::new();
        state.apply_entries(entries);
        state
    }

    #[test]
    fn missing_name_shows_placeholder_never_the_id() {
        let state = state_with(vec![json!({"id": "env-123"})]);
        let card = &render(&state).cards[0];
        assert_eq!(card.name, UNNAMED_ENV);
        assert!(card.name_is_placeholder);
        assert_ne!(card.name, card.id);
        assert!(!card.name.contains("env-123"));
    }

    #[test]
    fn blank_name_is_treated_as_missing() {
        let state = state_with(vec![json!({"id": "e1", "name": "   "})]);
        let card = &render(&state).cards[0];
        assert_eq!(card.name, UNNAMED_ENV);
        assert!(card.name_is_placeholder);
    }

    #[test]
    fn card_status_labels_cover_ready_degraded_failed() {
        let ready = state_with(vec![json!({"id": "e1", "name": "甲"})]);
        assert_eq!(render(&ready).cards[0].status.label(), "就绪");

        let degraded = state_with(vec![
            json!({"id": "e1", "name": "甲"}),
            json!({"name": "坏条目"}),
        ]);
        assert_eq!(render(&degraded).cards[0].status.label(), "降级");

        let mut failed = ready;
        failed.fail("boom");
        assert_eq!(CardStatus::Failed.label(), "异常");
        assert_eq!(render(&failed).error.as_deref(), Some("boom"));
        assert_eq!(
            render(&failed).status,
            EnvStatus::Failed,
            "失败态由视图状态承载，不渲染成空数据"
        );
    }

    #[test]
    fn card_shows_group_name_and_ungrouped_is_legal() {
        let state = state_with(vec![
            json!({"id": "e1", "name": "甲", "group_id": "g1", "group_name": "华东组"}),
            json!({"id": "e2", "name": "乙"}),
            json!({"id": "e3", "name": "丙", "group_id": "g2"}),
        ]);
        let cards = render(&state).cards;
        assert_eq!(cards[0].group.name, "华东组");
        assert_eq!(cards[0].group.id.as_deref(), Some("g1"));
        assert!(!cards[0].group.is_placeholder);
        assert_eq!(cards[1].group.name, UNGROUPED_LABEL);
        assert_eq!(cards[1].group.id, None);
        assert_eq!(cards[2].group.name, UNNAMED_GROUP);
        assert!(cards[2].group.is_placeholder);
        assert!(!cards[2].group.name.contains("g2"), "不得用 id 冒充分组名");
    }

    #[test]
    fn degraded_list_makes_invalid_count_visible() {
        let state = state_with(vec![
            json!({"id": "e1", "name": "甲"}),
            json!({"name": "坏条目"}),
            json!({"name": "另一条坏条目"}),
        ]);
        let view = render(&state);
        assert_eq!(
            view.degraded_notice.as_deref(),
            Some("有 2 条数据读取异常"),
            "坏条目必须在界面可见，不得静默吞掉"
        );
        assert_eq!(view.invalid_count, 2);
    }

    #[test]
    fn healthy_list_has_no_degraded_notice() {
        let state = state_with(vec![json!({"id": "e1", "name": "甲"})]);
        assert_eq!(render(&state).degraded_notice, None);
    }

    #[test]
    fn filtered_empty_uses_a_different_text_than_no_data() {
        let empty = render(&state_with(vec![]));
        assert_eq!(empty.empty, Some(EmptyReason::NoData));
        assert_eq!(empty.empty_text, Some("暂无环境"));
        assert!(empty.cards.is_empty());

        let mut filtered = state_with(vec![json!({"id": "e1", "name": "甲"})]);
        filtered.set_keyword("不存在");
        let view = render(&filtered);
        assert_eq!(view.empty, Some(EmptyReason::FilteredOut));
        assert_eq!(view.empty_text, Some("没有符合筛选条件的环境"));
        assert_ne!(view.empty_text, empty.empty_text);
        assert_eq!(view.total_count, 1, "原始数据仍在，只是被筛掉");
        assert_eq!(view.visible_count, 0);
    }

    #[test]
    fn cards_contain_only_the_visible_filtered_entries() {
        let mut state = state_with(vec![
            json!({"id": "e1", "name": "甲", "group_id": "g1"}),
            json!({"id": "e2", "name": "乙", "group_id": "g2"}),
        ]);
        state.set_group_filter(GroupFilter::Group("g2".into()));
        let view = render(&state);
        assert_eq!(view.cards.len(), 1);
        assert_eq!(view.cards[0].id, "e2");
        assert_eq!(view.total_count, 2);
        assert_eq!(view.visible_count, 1);
    }

    #[test]
    fn sidebar_lists_all_groups_with_counts_and_ungrouped() {
        let state = state_with(vec![
            json!({"id": "e1", "name": "甲", "group_id": "g1", "group_name": "华东组"}),
            json!({"id": "e2", "name": "乙", "group_id": "g1", "group_name": "华东组"}),
            json!({"id": "e3", "name": "丙"}),
            json!({"group_id": "g2", "group_name": "华南组"}),
        ]);
        let view = render(&state);
        let rows = &view.sidebar;
        assert_eq!(rows.len(), 4, "全部环境 + 2 个分组 + 未分组");
        assert_eq!(rows[0].label, ALL_GROUPS_LABEL);
        assert_eq!(rows[0].env_count, 3);
        assert_eq!(rows[0].invalid_count, 1);
        assert_eq!(rows[1].label, "华东组");
        assert_eq!(rows[1].env_count, 2);
        assert_eq!(rows[2].label, "华南组");
        assert_eq!(rows[2].env_count, 0);
        assert_eq!(rows[2].invalid_count, 1);
        assert_eq!(rows[3].label, UNGROUPED_LABEL);
        assert_eq!(rows[3].env_count, 1);
        assert!(rows[0].selected);
    }

    #[test]
    fn unnamed_group_uses_placeholder_and_sidebar_selection_follows_filter() {
        let mut state = state_with(vec![json!({"id": "e1", "group_id": "g1"})]);
        state.set_group_filter(GroupFilter::Group("g1".into()));
        let rows = group_rows(&state);
        let group = rows
            .iter()
            .find(|row| row.filter == GroupFilter::Group("g1".into()))
            .unwrap();
        assert_eq!(group.label, UNNAMED_GROUP);
        assert!(!group.label.contains("g1"));
        assert!(group.selected);
        assert!(!rows[0].selected);

        state.set_group_filter(GroupFilter::Ungrouped);
        let rows = group_rows(&state);
        let ungrouped = rows
            .iter()
            .find(|row| row.filter == GroupFilter::Ungrouped)
            .unwrap();
        assert!(ungrouped.selected);
    }

    #[test]
    fn failed_state_keeps_error_and_does_not_show_empty_state() {
        let mut state = state_with(vec![json!({"id": "e1"})]);
        state.fail("network down");
        let view = render(&state);
        assert_eq!(view.error.as_deref(), Some("network down"));
        assert_eq!(view.empty, None, "失败态不得伪装成空数据");
        assert!(view.cards.is_empty());
        assert_eq!(view.degraded_notice, None);
    }

    #[test]
    fn loading_state_keeps_cards_and_is_flagged() {
        let mut state = state_with(vec![json!({"id": "e1", "name": "甲"})]);
        state.begin_load();
        let view = render(&state);
        assert!(view.loading);
        assert_eq!(view.cards.len(), 1, "加载中不清空已有卡片，避免闪空");
        assert_eq!(view.empty, None);
    }

    #[test]
    fn all_invalid_data_is_visible_as_degraded_not_plain_empty() {
        let state = state_with(vec![json!({"name": "坏条目"})]);
        let view = render(&state);
        assert_eq!(view.status, EnvStatus::Degraded);
        assert_eq!(view.empty, None);
        assert_eq!(view.degraded_notice.as_deref(), Some("有 1 条数据读取异常"));
        assert!(view.cards.is_empty());
    }
}
