//! 渲染层（`feature = "ui"` 门控）：GPUI 元素树。
//! 只能编译验证，不随 `cargo test` 跑（无窗口环境不可达）。

#[cfg(feature = "ui")]
use crate::ui::env_view::EnvCard;
#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::ActiveTheme as _;
#[cfg(feature = "ui")]
use gpui_kit::*;

use super::mapping::{
    active_group_row, demo_state, empty_state_text, filter_key, main_pane, select_card,
    set_group_filter, status_tone, MainPane, StatusTone,
};
use crate::state::env_group::EnvGroupState;
use crate::ui::env_view::{CardStatus, GroupRow};

/// 环境视图的 GPUI 视图.
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
