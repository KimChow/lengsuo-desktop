//! 环境视图的 GPUI 接线层：把 [`crate::ui::env_view`] 的渲染描述接到元素树。
//!
//! **这是本仓接线层的黄金范例** —— 其他 `*_gpui.rs` 照这个文件的结构写。
//!
//! 分层约定：
//! - **描述层**（`env_view`）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（本文件）：把描述变成 GPUI 元素，**不重新实现**任何业务判断。
//!
//! 本文件内部再分两半（与 [`crate::ui::sidebar_gpui`] 同构）：
//! - **映射层**（无 `feature = "ui"` 门控）：描述字段 → 界面语义（状态色槽位、
//!   主区模式、空态文案、分组行标识、选中态写回），纯数据，随 `cargo test` 一起跑。
//! - **渲染层**（`feature = "ui"` 门控）：GPUI 元素树，只能编译验证。
//!
//! 硬约束：
//! - fail-closed：加载失败必须显示错误，**不得**退化成空列表。
//! - 「真实为空」与「筛选无结果」用不同文案（描述层已区分，此处不得合并）。
//! - 描述层说「不显示空态」（`empty_text == None`）时，界面**不得**补默认文案。
//! - 文案只有一个来源：状态 / 空态 / 分组名都取自描述层，本文件不另造一套。
//! - 颜色一律取自 `cx.theme()`，不硬编码。

use crate::state::env_group::{EnvGroupState, GroupFilter};
use crate::state::env_list::EnvStatus;
use crate::ui::env_view::{CardStatus, EnvView, GroupRow};

#[cfg(feature = "ui")]
use crate::ui::env_view::EnvCard;
#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::ActiveTheme as _;
#[cfg(feature = "ui")]
use gpui_kit::*;

// === 映射层（无 feature 门控）：描述字段 → 界面语义 ===

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

// === 渲染层（feature = "ui"）：GPUI 元素树 ===

/// 环境视图的 GPUI 视图。
///
/// 持有**环境分组状态**（[`EnvGroupState`]）的副本：点击卡片只改这份副本的
/// 选中态，点击分组行只改这份副本的筛选；宿主可用 [`Self::state_mut`] 回写共享状态。
#[cfg(feature = "ui")]
pub struct EnvViewGpui {
    state: EnvGroupState,
}

#[cfg(feature = "ui")]
impl EnvViewGpui {
    /// 用一份环境分组状态构造视图。
    #[must_use]
    pub fn new(state: EnvGroupState) -> Self {
        Self { state }
    }

    /// 从只读引用构造视图（内部克隆一份状态）。
    #[must_use]
    pub fn from_state(state: &EnvGroupState) -> Self {
        Self::new(state.clone())
    }

    /// 演示用视图：手工构造的环境条目，**不接真实数据源**。
    #[must_use]
    pub fn demo() -> Self {
        Self::new(demo_state())
    }

    /// 当前环境分组状态（只读）。
    #[must_use]
    pub fn state(&self) -> &EnvGroupState {
        &self.state
    }

    /// 当前环境分组状态（可写）：宿主据此回写共享状态。
    pub fn state_mut(&mut self) -> &mut EnvGroupState {
        &mut self.state
    }
}

/// 状态色槽位 → 主题里的背景色 / 前景色。
#[cfg(feature = "ui")]
fn tone_colors(
    tone: StatusTone,
    theme: &gpui_kit::component::Theme,
) -> (gpui_kit::Hsla, gpui_kit::Hsla) {
    match tone {
        StatusTone::Success => (theme.button_success, theme.button_success_foreground),
        StatusTone::Warning => (theme.button_warning, theme.button_warning_foreground),
        StatusTone::Danger => (theme.button_danger, theme.button_danger_foreground),
    }
}

/// 状态标识（就绪 / 降级 / 异常）：三种状态三种颜色。
#[cfg(feature = "ui")]
fn render_status_badge(status: CardStatus, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let (background, foreground) = tone_colors(status_tone(status), theme);
    div()
        .rounded_full()
        .px_2()
        .text_xs()
        .bg(background)
        .text_color(foreground)
        .child(status.label().to_owned())
}

/// 单张环境卡片：名称 + 分组 + 状态标识；点击写入选中态。
#[cfg(feature = "ui")]
fn render_card(
    card: &EnvCard,
    selected: bool,
    entity: &Entity<EnvViewGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    // 名称原样取描述层：缺失时描述层已给占位文案，这里**不补 id**。
    let mut title = div().text_sm().font_bold().child(card.name.clone());
    if card.name_is_placeholder {
        title = title.text_color(theme.muted_foreground);
    }
    if selected {
        title = title.text_color(theme.accent);
    }

    let element = div()
        .id(format!("env-card-{}", card.id))
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .bg(theme.background)
        .w(px(220.))
        .cursor_pointer()
        .child(title)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(card.group.name.clone()),
                )
                .child(render_status_badge(card.status, theme)),
        );

    let entity = entity.clone();
    let id = card.id.clone();
    element.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            select_card(&mut view.state, &id);
            cx.notify();
        });
    })
}

/// 左侧分组栏的一行：点击切换分组筛选。
#[cfg(feature = "ui")]
fn render_group_row(
    row: &GroupRow,
    entity: &Entity<EnvViewGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let mut label = div().text_sm().child(row.label.clone());
    if row.selected {
        label = label.font_bold().text_color(theme.accent);
    }

    let mut meta = div().flex().items_center().gap_1().child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(format!("{} 项", row.env_count)),
    );
    if row.invalid_count > 0 {
        meta = meta.child(
            div()
                .text_xs()
                .text_color(theme.button_danger)
                .child(format!("!{}", row.invalid_count)),
        );
    }

    let element = div()
        .id(format!("env-group-{}", filter_key(&row.filter)))
        .flex()
        .justify_between()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_sm()
        .cursor_pointer()
        .child(label)
        .child(meta);

    let entity = entity.clone();
    let filter = row.filter.clone();
    element.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            set_group_filter(&mut view.state, filter.clone());
            cx.notify();
        });
    })
}

/// 主区标题行：当前分组的标题 + 条目数 + 坏条目数。
#[cfg(feature = "ui")]
fn render_group_title(
    row: &GroupRow,
    total_count: usize,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let mut title = div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().text_sm().font_bold().child(row.label.clone()))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(format!("{} 项", row.env_count)),
        );
    if row.invalid_count > 0 {
        title = title.child(
            div()
                .text_xs()
                .text_color(theme.button_danger)
                .child(format!("{} 条异常", row.invalid_count)),
        );
    }
    title.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(format!("共 {total_count} 项")),
    )
}

/// 降级提示条（描述层保证 `invalid_count > 0` 时必有）。
#[cfg(feature = "ui")]
fn render_notice(text: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(theme.button_warning)
        .text_color(theme.button_warning_foreground)
        .text_xs()
        .child(text.to_owned())
}

/// 加载中提示条：与卡片并存（描述层加载时不清空卡片）。
#[cfg(feature = "ui")]
fn render_loading(theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.muted_foreground)
        .child("正在加载环境……")
}

#[cfg(feature = "ui")]
impl Render for EnvViewGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = cx.entity();
        // 描述层产出渲染描述；本文件只做「描述 → 元素」的映射。
        let view = crate::ui::env_view::render(&self.state);
        let pane = main_pane(&view);

        let root = div()
            .flex()
            .h_full()
            .w_full()
            .bg(theme.background)
            .text_color(theme.foreground);

        // 失败：fail-closed —— 显示错误与「异常」标识，绝不退化成空列表。
        if pane == MainPane::Failed {
            let error = view.error.as_deref().unwrap_or("未知错误");
            return root
                .flex_col()
                .gap_2()
                .p_4()
                .child(render_status_badge(CardStatus::Failed, theme))
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.button_danger)
                        .child("环境列表加载失败"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(error.to_owned()),
                );
        }

        // 左侧分组栏（「未分组」是其中一行，不得隐藏）。
        let sidebar = div()
            .w(px(200.))
            .h_full()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .border_r_1()
            .border_color(theme.border)
            .children(
                view.sidebar
                    .iter()
                    .map(|row| render_group_row(row, &entity, theme)),
            );

        // 右侧主区
        let mut main = div().flex_1().h_full().flex().flex_col().gap_2().p_3();

        // 降级提示（描述层保证 invalid_count > 0 时必有）
        if let Some(notice) = &view.degraded_notice {
            main = main.child(render_notice(notice, theme));
        }

        // 加载中：给提示，但**保留**已有卡片（描述层加载时不清空，避免闪空）。
        if pane == MainPane::Loading {
            main = main.child(render_loading(theme));
        }

        // 分组标题行
        if let Some(row) = active_group_row(&view) {
            main = main.child(render_group_title(row, view.total_count, theme));
        }

        if view.cards.is_empty() {
            // 只有描述层说「该显示空态」时才显示。
            // 描述层给 None 的情形（如全部条目解析失败）只显示降级提示，
            // 不得补一句「暂无环境」——那是谎称没有数据。
            if let Some(text) = empty_state_text(&view) {
                main = main.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(text.to_owned()),
                );
            }
        } else {
            let grid = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(view.cards.iter().map(|card| {
                    let selected = view.selected.as_deref() == Some(card.id.as_str());
                    render_card(card, selected, &entity, theme)
                }));
            main = main.child(grid);
        }

        root.child(sidebar).child(main)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_group_row, demo_state, empty_state_text, filter_key, main_pane, select_card,
        set_group_filter, status_tone, MainPane, StatusTone,
    };
    use crate::state::env_group::{EnvGroupState, GroupFilter};
    use crate::ui::env_view::{self, CardStatus};
    use serde_json::json;

    fn state_with(entries: Vec<serde_json::Value>) -> EnvGroupState {
        let mut state = EnvGroupState::new();
        state.apply_entries(entries);
        state
    }

    fn active_label(view: &env_view::EnvView) -> Option<&str> {
        active_group_row(view).map(|row| row.label.as_str())
    }

    #[test]
    fn three_states_map_to_three_distinct_colors() {
        assert_eq!(status_tone(CardStatus::Ready), StatusTone::Success);
        assert_eq!(status_tone(CardStatus::Degraded), StatusTone::Warning);
        assert_eq!(status_tone(CardStatus::Failed), StatusTone::Danger);
        assert_ne!(
            status_tone(CardStatus::Ready),
            status_tone(CardStatus::Degraded)
        );
        assert_ne!(
            status_tone(CardStatus::Degraded),
            status_tone(CardStatus::Failed)
        );
    }

    #[test]
    fn ready_state_renders_cards_without_a_notice() {
        let view = env_view::render(&state_with(vec![json!({"id": "e1", "name": "甲"})]));
        assert_eq!(main_pane(&view), MainPane::Ready);
        assert_eq!(view.degraded_notice, None);
        assert_eq!(empty_state_text(&view), None, "有卡片时不得显示空态文案");
        assert_eq!(view.cards[0].status, CardStatus::Ready);
    }

    #[test]
    fn degraded_state_keeps_a_notice_and_cards_are_marked_degraded() {
        let view = env_view::render(&state_with(vec![
            json!({"id": "e1", "name": "甲"}),
            json!({"name": "坏条目"}),
        ]));
        assert_eq!(main_pane(&view), MainPane::Ready);
        assert_eq!(view.degraded_notice.as_deref(), Some("有 1 条数据读取异常"));
        assert_eq!(view.cards[0].status, CardStatus::Degraded);
        assert_eq!(status_tone(view.cards[0].status), StatusTone::Warning);
    }

    #[test]
    fn failed_state_is_visible_and_never_becomes_an_empty_state() {
        let mut state = state_with(vec![json!({"id": "e1", "name": "甲"})]);
        state.fail("gateway unreachable");
        let view = env_view::render(&state);

        assert_eq!(main_pane(&view), MainPane::Failed);
        assert_eq!(view.error.as_deref(), Some("gateway unreachable"));
        assert_eq!(
            empty_state_text(&view),
            None,
            "失败态不得显示空态文案（那是谎称没有数据）"
        );
        assert!(view.cards.is_empty());
    }

    #[test]
    fn loading_state_is_flagged_and_keeps_the_existing_cards() {
        let mut state = state_with(vec![json!({"id": "e1", "name": "甲"})]);
        state.begin_load();
        let view = env_view::render(&state);

        assert_eq!(main_pane(&view), MainPane::Loading);
        assert_eq!(view.cards.len(), 1, "加载中不清空卡片，避免闪空");
        assert_eq!(empty_state_text(&view), None);
    }

    #[test]
    fn all_invalid_data_shows_a_notice_but_no_empty_state() {
        let view = env_view::render(&state_with(vec![json!({"name": "坏条目"})]));
        assert_eq!(main_pane(&view), MainPane::Ready);
        assert_eq!(view.degraded_notice.as_deref(), Some("有 1 条数据读取异常"));
        assert_eq!(
            empty_state_text(&view),
            None,
            "全部解析失败时只能显示降级提示，不得显示「暂无环境」"
        );
    }

    #[test]
    fn filtered_empty_and_real_empty_use_different_texts() {
        let empty = env_view::render(&state_with(vec![]));
        assert_eq!(empty_state_text(&empty), Some("暂无环境"));

        let mut filtered = state_with(vec![json!({"id": "e1", "name": "甲"})]);
        filtered.set_keyword("不存在的关键词");
        let view = env_view::render(&filtered);
        assert_eq!(empty_state_text(&view), Some("没有符合筛选条件的环境"));
        assert_ne!(empty_state_text(&view), empty_state_text(&empty));
    }

    #[test]
    fn the_active_group_row_follows_the_filter() {
        let mut state = state_with(vec![
            json!({"id": "e1", "name": "甲", "group_id": "g1", "group_name": "华东组"}),
            json!({"id": "e2", "name": "乙"}),
        ]);

        let view = env_view::render(&state);
        assert_eq!(active_label(&view), Some("全部环境"));

        state.set_group_filter(GroupFilter::Group("g1".into()));
        let view = env_view::render(&state);
        assert_eq!(active_label(&view), Some("华东组"));

        state.set_group_filter(GroupFilter::Ungrouped);
        let view = env_view::render(&state);
        assert_eq!(
            active_label(&view),
            Some("未分组"),
            "「未分组」是合法分组，必须能成为主区标题"
        );
    }

    #[test]
    fn the_sidebar_never_hides_the_ungrouped_row() {
        let view = env_view::render(&state_with(vec![json!({"id": "e1", "name": "甲"})]));
        let labels: Vec<&str> = view.sidebar.iter().map(|row| row.label.as_str()).collect();
        assert!(labels.contains(&"未分组"), "「未分组」不得被隐藏");
        assert!(labels.contains(&"全部环境"));
    }

    #[test]
    fn filter_keys_are_stable_and_distinct() {
        assert_eq!(filter_key(&GroupFilter::All), "all");
        assert_eq!(filter_key(&GroupFilter::Ungrouped), "ungrouped");
        assert_eq!(filter_key(&GroupFilter::Group("g1".into())), "group-g1");
        assert_ne!(
            filter_key(&GroupFilter::Group("g1".into())),
            filter_key(&GroupFilter::Group("g2".into()))
        );
    }

    #[test]
    fn clicking_a_card_writes_the_selection() {
        let mut state = demo_state();
        assert!(select_card(&mut state, "demo-3"));
        assert_eq!(state.selected(), Some("demo-3"));
        assert_eq!(env_view::render(&state).selected.as_deref(), Some("demo-3"));
    }

    #[test]
    fn clicking_an_unknown_card_changes_nothing() {
        let mut state = demo_state();
        assert!(!select_card(&mut state, "不存在的 id"));
        assert_eq!(state.selected(), None);
    }

    #[test]
    fn clicking_a_group_row_switches_the_filter_once() {
        let mut state = demo_state();
        assert!(set_group_filter(&mut state, GroupFilter::Ungrouped));
        assert_eq!(state.group_filter(), &GroupFilter::Ungrouped);
        assert!(
            !set_group_filter(&mut state, GroupFilter::Ungrouped),
            "点同一行不应重复触发"
        );

        let view = env_view::render(&state);
        assert_eq!(view.cards.len(), 1, "未分组下只剩未分组的条目");
        assert_eq!(view.cards[0].id, "demo-4");
    }

    #[test]
    fn demo_state_has_grouped_and_ungrouped_entries() {
        let state = demo_state();
        assert_eq!(state.entries().len(), 4);
        assert_eq!(state.groups().len(), 2);
        assert_eq!(state.ungrouped_count(), 1);
        assert_eq!(state.invalid_count(), 0);

        let view = env_view::render(&state);
        assert_eq!(view.cards.len(), 4);
        assert_eq!(view.total_count, 4);
    }
}
