//! 映射层（无 `feature = "ui"` 门控）：描述字段 → 界面语义。
//! 纯数据，随 `cargo test` 一起跑；不依赖 GPUI。

use crate::state::datasource::LoadState;
use crate::state::env_group::{EmptyReason, EnvGroupState, GroupFilter};
use crate::state::env_list::EnvStatus;
use crate::ui::env_view::{CardStatus, EnvView, GroupRow};

/// 状态色的语义槽位（决定用主题里的哪个颜色）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTone {
    /// 就绪：成功色。
    Success,
    /// 降级：警告色。
    Warning,
    /// 异常：危险色。
    Danger,
}

/// 描述层状态 → 颜色槽位。
///
/// 状态**文案**仍取 [`CardStatus::label`]，本文件不另造一套。
#[must_use]
pub fn status_tone(status: CardStatus) -> StatusTone {
    match status {
        CardStatus::Ready => StatusTone::Success,
        CardStatus::Degraded => StatusTone::Warning,
        CardStatus::Failed => StatusTone::Danger,
    }
}

/// 主区要显示什么（纯数据，可在无窗口环境测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainPane {
    /// 加载中：显示加载提示，但**保留**已有卡片（描述层不清空，避免闪空）。
    Loading,
    /// 失败：显示错误，**绝不**退化成空列表。
    Failed,
    /// 正常：按描述层给的内容渲染（就绪 / 降级共用这一支）。
    Ready,
}

/// 主区模式：由描述层的健康态决定，接线层不自行判断「该不该空」。
#[must_use]
pub fn main_pane(view: &EnvView) -> MainPane {
    match view.status {
        EnvStatus::Loading => MainPane::Loading,
        EnvStatus::Failed => MainPane::Failed,
        EnvStatus::Ready | EnvStatus::Degraded => MainPane::Ready,
    }
}

/// 空态文案：描述层给什么就显示什么。
///
/// `None` 表示**不显示空态** —— 描述层在「全部条目解析失败」时故意给 `None`，
/// 此时界面只能显示降级提示，**不得**补一句「暂无环境」。
#[must_use]
pub fn empty_state_text(view: &EnvView) -> Option<&'static str> {
    view.empty_text
}

/// 主区标题对应哪一行分组。
///
/// 取当前被选中的那一行；描述层没标选中时回退到第一行（「全部环境」）。
#[must_use]
pub fn active_group_row(view: &EnvView) -> Option<&GroupRow> {
    view.sidebar
        .iter()
        .find(|row| row.selected)
        .or_else(|| view.sidebar.first())
}

/// 分组筛选 → 稳定的行标识（元素 id / 测试用）。
#[must_use]
pub fn filter_key(filter: &GroupFilter) -> String {
    match filter {
        GroupFilter::All => "all".to_owned(),
        GroupFilter::Ungrouped => "ungrouped".to_owned(),
        GroupFilter::Group(id) => format!("group-{id}"),
    }
}

/// 点击卡片：写入选中态（本卡不接真实启动逻辑）。
///
/// 返回是否真的选中了 —— 描述层的 `select` 对不存在的 id 不改变选中态。
pub fn select_card(state: &mut EnvGroupState, id: &str) -> bool {
    state.select(id);
    state.selected() == Some(id)
}

/// 点击分组行：切换分组筛选。
///
/// 返回筛选是否真的变了（点同一行不重复触发）。
pub fn set_group_filter(state: &mut EnvGroupState, filter: GroupFilter) -> bool {
    if state.group_filter() == &filter {
        return false;
    }
    state.set_group_filter(filter);
    true
}

/// 空态恢复动作：只有「筛选造成的空态」才给恢复入口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyRecovery {
    /// 清除分组筛选与关键词。
    ClearFilter,
}

/// 「清除筛选」按钮文案（唯一来源，界面不另写字面量）。
pub const CLEAR_FILTER_LABEL: &str = "清除筛选";
/// 搜索框占位文案（唯一来源）。
pub const SEARCH_PLACEHOLDER: &str = "按名称搜索环境";

/// 该不该显示恢复入口。
///
/// - `FilteredOut` 且确有筛选 ⇒ 显示（清筛选能恢复）；
/// - `NoData` ⇒ 不显示（本来就没数据，点了没用 = 假按钮）；
/// - `None`（不显示空态）⇒ 不显示。
#[must_use]
pub fn empty_recovery(empty: Option<EmptyReason>, filter_active: bool) -> Option<EmptyRecovery> {
    match empty {
        Some(EmptyReason::FilteredOut) if filter_active => Some(EmptyRecovery::ClearFilter),
        _ => None,
    }
}

/// 筛选栏要显示的摘要（关键词原样给界面，不 trim、不另造）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterSummary {
    /// 当前关键词（原值）。
    pub keyword: String,
    /// 是否存在有效筛选（纯空白关键词不算）。
    pub active: bool,
}

#[must_use]
pub fn filter_summary(state: &EnvGroupState) -> FilterSummary {
    FilterSummary {
        keyword: state.keyword().to_owned(),
        active: state.is_filter_active(),
    }
}

/// 点「清除筛选」：分组与关键词一起清。返回是否真的变了（已无筛选 ⇒ false）。
pub fn clear_filter(state: &mut EnvGroupState) -> bool {
    if !state.is_filter_active() {
        return false;
    }
    state.clear_filter();
    true
}

/// 演示用状态：手工构造若干环境条目，**不接真实数据源**。
#[must_use]
pub fn demo_state() -> EnvGroupState {
    use serde_json::json;
    let mut state = EnvGroupState::new();
    state.apply_entries(vec![
        json!({"id": "demo-1", "name": "构建机 A", "group_id": "g1", "group_name": "华东组"}),
        json!({"id": "demo-2", "name": "构建机 B", "group_id": "g1", "group_name": "华东组"}),
        json!({"id": "demo-3", "name": "测试机", "group_id": "g2", "group_name": "华南组"}),
        json!({"id": "demo-4", "name": "本地沙箱"}),
    ]);
    state
}

/// 把注入的三态快照水合成视图状态：**只换来源，不改行/分组结构**——
/// 行清单仍由 [`crate::ui::env_view::render`] 从状态产出，字段与分组语义原样。
///
/// - `Ready(snapshot)` ⇒ 原样采用快照（快照自带健康态，如实表达）；
/// - `Loading` ⇒ 加载态（未拉取过 ≠ 空集，不得报「暂无环境」）；
/// - `Failed(reason)` ⇒ fail-closed：记录原因，绝不退化成空列表。
#[must_use]
pub fn hydrate(load: LoadState<EnvGroupState>) -> EnvGroupState {
    match load {
        LoadState::Ready(snapshot) => snapshot,
        // 默认态即「尚未拉取」的加载态（见 `EnvListViewState::Default`）。
        LoadState::Loading => EnvGroupState::new(),
        LoadState::Failed(reason) => {
            let mut state = EnvGroupState::new();
            state.fail(reason);
            state
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{demo_state, hydrate, main_pane, MainPane};
    use crate::state::datasource::LoadState;
    use crate::state::env_group::GroupFilter;
    use crate::state::env_list::EnvStatus;
    use crate::ui::env_view;

    /// ① 注入 Ready 快照 ⇒ 行数 == 快照组数（每个分组恰好一行）。
    #[test]
    fn ready_snapshot_hydrates_one_row_per_group() {
        let snap = demo_state();
        let state = hydrate(LoadState::Ready(snap.clone()));
        assert_eq!(state, snap, "Ready 快照原样采用（只换来源，不改结构）");
        let view = env_view::render(&state);
        let group_rows = view
            .sidebar
            .iter()
            .filter(|row| matches!(row.filter, GroupFilter::Group(_)))
            .count();
        assert_eq!(group_rows, snap.groups().len(), "每个快照分组恰好一行");
        assert_eq!(view.cards.len(), snap.entries().len());
    }

    /// ② 注入 Failed ⇒ 原因保留、fail-closed，不退化成空列表。
    #[test]
    fn failed_snapshot_keeps_reason_fail_closed() {
        let state = hydrate(LoadState::Failed("环境服务不可达".to_owned()));
        assert_eq!(state.status(), EnvStatus::Failed);
        assert_eq!(state.last_error(), Some("环境服务不可达"));
        let view = env_view::render(&state);
        assert_eq!(main_pane(&view), MainPane::Failed);
        assert_eq!(view.error.as_deref(), Some("环境服务不可达"));
        assert_eq!(view.empty, None, "失败不得伪装成「暂无环境」");
    }

    /// ③ 注入 Loading ⇒ 加载态，不 panic、不报空数据。
    #[test]
    fn loading_snapshot_stays_loading_not_no_data() {
        let state = hydrate(LoadState::Loading);
        assert_eq!(state.status(), EnvStatus::Loading);
        let view = env_view::render(&state);
        assert_eq!(main_pane(&view), MainPane::Loading);
        assert!(view.loading);
        assert_eq!(view.empty_text, None, "未拉取过 ≠ 空集");
    }
}
