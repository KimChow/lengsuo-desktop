//! 搜索筛选浮层（波次 7，卡 `ui-overlay-search-filter`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `search + filter`。
//! 数据源在运行时侧：数据未下发时如实显示「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "搜索筛选";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 搜索输入占位（search.placeholder）。
pub const PLACEHOLDER: &str = "查找";
/// 空态说明（search.noResults；空态 ≠ 空列表）。
pub const NO_RESULTS: &str = "无结果";
/// 搜索选项（search.caseSensitive / search.wholeWord / search.useRegex）。
pub const SEARCH_TOGGLES: [&str; 3] = [
    "区分大小写 (Alt+C)",
    "全词匹配 (Alt+W)",
    "使用正则表达式 (Alt+R)",
];
/// 匹配导航（search.previousMatch / search.nextMatch / search.close）。
pub const MATCH_NAV: [&str; 3] = [
    "上一个匹配 (Shift+Enter)",
    "下一个匹配 (Enter)",
    "关闭 (Escape)",
];
/// 搜索范围标题（search.scope）。
pub const SCOPE_LABEL: &str = "搜索范围：";
/// 搜索范围选项（search.scope.both / modified / original）。
pub const SCOPE_OPTIONS: [&str; 3] = ["两侧", "修改文件", "原始文件"];
/// 筛选面板标题（filter.title）。
pub const FILTER_TITLE: &str = "任务筛选";
/// 状态组多选提示（filter.multi）。
pub const FILTER_MULTI: &str = "多选";
/// 时间筛选标题（filter.date）。
pub const DATE_FILTER_LABEL: &str = "筛选时间";
/// 时间筛选项（filter.date.today / yesterday / last7days / last30days / all / customRange）。
pub const DATE_FILTER_OPTIONS: [&str; 6] = [
    "今天",
    "昨天",
    "最近 7 天",
    "最近 30 天",
    "全部时间",
    "自定义范围...",
];
/// 状态筛选标题（filter.status）。
pub const STATUS_FILTER_LABEL: &str = "筛选状态";
/// 状态筛选项（filter.status.all / pending / planning / working / completed / failed / archived）。
pub const STATUS_FILTER_OPTIONS: [&str; 7] = [
    "全部状态",
    "待处理",
    "规划中",
    "进行中",
    "已完成",
    "失败",
    "已归档",
];
/// 重置按钮（filter.clearAll）。
pub const CLEAR_ALL: &str = "重置筛选条件";

/// 数据送达状态：无数据源时如实停在「未下发」，不伪装成正常态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryState {
    /// 数据已送达。
    Normal,
    /// 数据未下发（fail-closed）。
    NotDelivered,
}

/// 内容区形态：条目 / 空态 / 未下发 —— 空态不是空列表。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntriesView {
    /// 命中条目。
    Items(Vec<&'static str>),
    /// 空态（无结果）。
    Empty,
    /// 数据未下发。
    NotDelivered,
}

/// 搜索筛选浮层纯数据模型（无 GPUI 依赖）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFilterModel {
    /// 数据送达状态。
    pub state: DeliveryState,
    /// 搜索词；为空 ⇒ 输入框显示占位，清除后回到空态。
    pub query: String,
    /// 已下发的命中条目；无数据源 ⇒ 恒为空。
    pub entries: Vec<&'static str>,
}

impl SearchFilterModel {
    /// 未下发模型（当前唯一真实形态：数据源在运行时侧，尚未接线）。
    pub fn not_delivered() -> Self {
        Self {
            state: DeliveryState::NotDelivered,
            query: String::new(),
            entries: Vec::new(),
        }
    }

    /// 清除搜索词：输入框回到占位，内容区回到空态。
    pub fn clear_query(&mut self) {
        self.query.clear();
    }

    /// 输入框是否显示占位（搜索词为空）。
    pub fn shows_placeholder(&self) -> bool {
        self.query.is_empty()
    }

    /// 内容区形态：未下发优先（fail-closed）；正常态无条目 ⇒ 空态，不是空列表。
    pub fn entries_view(&self) -> EntriesView {
        match self.state {
            DeliveryState::NotDelivered => EntriesView::NotDelivered,
            DeliveryState::Normal if self.entries.is_empty() => EntriesView::Empty,
            DeliveryState::Normal => EntriesView::Items(self.entries.clone()),
        }
    }
}
/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    let model = SearchFilterModel::not_delivered();
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(search_panel(theme))
        .child(filter_panel(theme))
        .child(content_area(theme, &model))
}

/// 搜索区：占位输入行 + 搜索选项 + 匹配导航 + 搜索范围。
fn search_panel(theme: &Theme) -> Div {
    div()
        .mx_4()
        .mt_2()
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                // fail-closed：搜索词恒为空 ⇒ 输入框如实只显示占位，不伪造已输入态。
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(PLACEHOLDER),
                ),
        )
        .child(option_row(theme, &SEARCH_TOGGLES))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(MATCH_NAV.iter().map(|nav| muted_text(theme, nav))),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .child(muted_text(theme, SCOPE_LABEL))
                .child(option_row(theme, &SCOPE_OPTIONS)),
        )
}

/// 筛选区：面板标题 + 重置按钮 + 时间/状态两组筛选项。
fn filter_panel(theme: &Theme) -> Div {
    div()
        .mx_4()
        .mt_2()
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(FILTER_TITLE),
                )
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .text_xs()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_md()
                        .text_color(theme.accent)
                        .child(CLEAR_ALL),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(muted_text(theme, DATE_FILTER_LABEL))
                .child(option_row(theme, &DATE_FILTER_OPTIONS)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(muted_text(theme, STATUS_FILTER_LABEL))
                        .child(muted_text(theme, FILTER_MULTI)),
                )
                .child(option_row(theme, &STATUS_FILTER_OPTIONS)),
        )
}

/// 内容区：按纯数据层分支 —— 未下发 / 空态 / 条目。
fn content_area(theme: &Theme, model: &SearchFilterModel) -> Div {
    let area = div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm();
    match model.entries_view() {
        EntriesView::NotDelivered => area.text_color(theme.muted_foreground).child(NOT_DELIVERED),
        EntriesView::Empty => area.text_color(theme.muted_foreground).child(NO_RESULTS),
        EntriesView::Items(items) => area
            .flex_col()
            .text_color(theme.foreground)
            .children(items.iter().map(|entry| div().px_4().py_2().child(*entry))),
    }
}

/// 可选项胶囊（选项无选中态数据 ⇒ 一律中性样式，不伪造选中）。
fn chip(theme: &Theme, text: &'static str) -> Div {
    div()
        .px_2()
        .py_1()
        .text_xs()
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .text_color(theme.foreground)
        .child(text)
}

/// 弱化说明文字（组标题/导航/提示）。
fn muted_text(theme: &Theme, text: &'static str) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 胶囊行（自动换行）。
fn option_row(theme: &Theme, options: &'static [&'static str]) -> Div {
    div()
        .flex()
        .flex_wrap()
        .gap_1()
        .children(options.iter().map(|option| chip(theme, option)))
}
#[cfg(test)]
mod tests {
    use super::{
        main_area, DeliveryState, EntriesView, SearchFilterModel, DATE_FILTER_OPTIONS, MATCH_NAV,
        NOT_DELIVERED, SCOPE_OPTIONS, SEARCH_TOGGLES, STATUS_FILTER_OPTIONS, TITLE,
    };
    use gpui_kit::component::Theme;
    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }
    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
    #[test]
    fn model_is_not_delivered_and_counts_option_groups() {
        let model = SearchFilterModel::not_delivered();
        assert_eq!(model.state, DeliveryState::NotDelivered);
        assert!(model.entries.is_empty());
        assert_eq!(SEARCH_TOGGLES.len(), 3);
        assert_eq!(MATCH_NAV.len(), 3);
        assert_eq!(SCOPE_OPTIONS.len(), 3);
        assert_eq!(DATE_FILTER_OPTIONS.len(), 6);
        assert_eq!(STATUS_FILTER_OPTIONS.len(), 7);
    }
    #[test]
    fn empty_input_yields_empty_state_not_empty_list() {
        let mut model = SearchFilterModel {
            state: DeliveryState::Normal,
            ..SearchFilterModel::not_delivered()
        };
        model.clear_query();
        assert!(model.shows_placeholder());
        assert_eq!(model.entries_view(), EntriesView::Empty);
        assert_ne!(model.entries_view(), EntriesView::Items(Vec::new()));
    }
    #[test]
    fn not_delivered_state_stays_fail_closed_even_with_entries() {
        let model = SearchFilterModel {
            state: DeliveryState::NotDelivered,
            entries: vec![STATUS_FILTER_OPTIONS[1]],
            ..SearchFilterModel::not_delivered()
        };
        assert_eq!(model.entries_view(), EntriesView::NotDelivered);
    }
}
