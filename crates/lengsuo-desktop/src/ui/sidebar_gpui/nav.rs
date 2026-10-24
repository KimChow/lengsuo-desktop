//! 边栏导航的 GPUI 渲染：波次 9 左栏分组改版（D1/D2）。
//!
//! 形态对齐 docs/ui-parity-ref-snapshot.md §三 D1/D2（桌面 2 实拍）：
//! - **上区**：6 个主入口（图标 + 文字行）：新建任务 / 助理(BETA) /
//!   专家·技能·连接器 / 定时任务 / 资料库 / 更多。
//! - **中区**：「任务」「空间」分组（组名 + 计数徽标），组内条目行。
//! - **「更多」组**：收纳现有 14 项旧入口（展开 / 收起）。
//! - **底部**：活动卡位 + 用户行（D8）。
//!
//! 硬约束：
//! - 文案逐字：主入口 / 分组名取自实拍 D1/D2，「更多」= `home.practiceCases.more`，
//!   「未下发」复用 [`crate::ui::panel_views::home::NOT_DELIVERED`]，⛔ 本文件不造词。
//! - 颜色一律 `theme.*`，不硬编码；布局用 gpui flex（div / flex_col / items_center / gap_*）。
//! - fail-closed：任务 / 空间 / 用户 / 活动卡的数据源未接 ⇒ 如实「未下发」，不伪造计数。
//! - ⛔ 点击只上报意图：主入口暂无运行时接线，只有手型光标；旧入口点击只改选中态
//!   （A15 约定），不产生任何运行时行为。

#![cfg(feature = "ui")]

use std::cell::Cell;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::panel_views::home::NOT_DELIVERED;
use crate::ui::sidebar::nav::NavEntry;

use super::render::render_count_badge;
use super::SidebarViewGpui;

// === 文案常量（⛔ 逐字，来源见各注释） ===

/// 「更多」组标签（逐字：文案全表 `home.practiceCases.more` → 更多）。
pub(super) const MORE_LABEL: &str = "更多";
/// 上区 6 主入口的标签（顺序即展示顺序；逐字取自实拍 D1）。
pub(super) const MAIN_ENTRY_LABELS: [&str; 6] = [
    "新建任务",
    "助理(BETA)",
    "专家·技能·连接器",
    "定时任务",
    "资料库",
    MORE_LABEL,
];
/// 主入口的图标字形（实拍为图标资产；本卡无图标资产 ⇒ 等宽字符占位，差异见报告）。
const MAIN_ENTRY_GLYPHS: [&str; 6] = ["＋", "◆", "◎", "▤", "☰", "⋯"];
/// 「任务」分组名（逐字，实拍 D2：「任务 (N)」）。
pub(super) const TASKS_GROUP_LABEL: &str = "任务";
/// 「空间」分组名（逐字，实拍 D2：「空间 (N)」）。
pub(super) const SPACES_GROUP_LABEL: &str = "空间";
/// 「更多」组收起态的指示符（实拍为 chevron 图标资产 ⇒ 字形占位）。
const COLLAPSED_CHEVRON: &str = "▸";
/// 「更多」组展开态的指示符。
const EXPANDED_CHEVRON: &str = "▾";

thread_local! {
    /// 「更多」组的展开态。
    ///
    /// ponytail: 本卡只许改本文件，视图结构体（`SidebarViewGpui`）加不了字段 ⇒
    /// 收进线程局部；GPUI 应用单线程渲染，跨线程无诉求。宿主接线后迁进视图字段。
    static MORE_OPEN: Cell<bool> = const { Cell::new(false) };
}

/// 「更多」组是否展开。
fn more_open() -> bool {
    MORE_OPEN.with(Cell::get)
}

/// 翻转「更多」组展开态（点击头行时调用，随后 `cx.notify()` 重渲染）。
fn toggle_more_open() {
    MORE_OPEN.with(|open| open.set(!open.get()));
}

/// 测试用：直接设定展开态（避免用例间串扰）。
#[cfg(test)]
fn set_more_open(open: bool) {
    MORE_OPEN.with(|cell| cell.set(open));
}

/// 上区主入口行：图标位 + 标签（实拍 D1 的 6 项，图标 + 文字行）。
///
/// ⛔ 点击只上报意图：运行时接线不在本卡，这里只给手型光标，不改状态。
fn render_main_entries(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(
            MAIN_ENTRY_LABELS
                .iter()
                .zip(MAIN_ENTRY_GLYPHS)
                .map(|(label, glyph)| {
                    div()
                        .id(format!("sidebar-main-{label}"))
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .child(
                            div()
                                .flex()
                                .w_4()
                                .h_4()
                                .items_center()
                                .justify_center()
                                .text_xs()
                                .text_color(theme.sidebar_foreground)
                                .child(glyph),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.sidebar_foreground)
                                .truncate()
                                .child(*label),
                        )
                }),
        )
}

/// 中区分组：组名行 + 组内条目行（实拍 D2 的「任务 (N)」/「空间 (N)」形态）。
///
/// 组内条目与计数的数据源未接 ⇒ fail-closed：组名照画、不伪造计数徽标
/// （计数到 0 也不显示，同旧入口角标约定），条目行如实「未下发」。
fn render_group(label: &'static str, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .pt_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .text_xs()
                .font_bold()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(
            div()
                .px_2()
                .py_1()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}

/// 底部：活动卡位（D8：图 + 文 + CTA 钮）+ 用户行（头像 + 名称）。
///
/// 两处数据源均未接 ⇒ fail-closed「未下发」。钉到边栏最底需宿主把本列
/// 放进 `mt_auto` 布局（本卡不改 mod.rs，见报告）。
fn render_footer(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .px_2()
        .py_2()
        .mt_2()
        .border_t_1()
        .border_color(theme.sidebar_border)
        // 活动卡位：图（色块位）+ 文。
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .p_2()
                .rounded_md()
                .border_1()
                .border_color(theme.sidebar_border)
                .child(div().w_6().h_6().rounded_sm().bg(theme.sidebar_accent))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .truncate()
                        .child(NOT_DELIVERED),
                ),
        )
        // 用户行：头像位（圆形）+ 名称。
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .py_1()
                .child(div().w_6().h_6().rounded_full().bg(theme.sidebar_accent))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.sidebar_foreground)
                        .truncate()
                        .child(NOT_DELIVERED),
                ),
        )
}

/// 渲染一个导航入口：标题 + 计数角标。
///
/// ⚠️ `on_click` 只在 `Stateful<E>` 上存在 ⇒ 必须先 `.id("唯一id")`。
pub(super) fn render_nav_entry(
    entry: &NavEntry,
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    let title_color = if entry.selected {
        theme.sidebar_accent_foreground
    } else {
        theme.sidebar_foreground
    };

    let mut element = div()
        .id(format!("sidebar-nav-{:?}", entry.kind))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_sm()
        .cursor_pointer();
    if entry.selected {
        element = element.bg(theme.sidebar_accent);
    }

    let mut element = element.child(
        div()
            .text_sm()
            .text_color(title_color)
            .truncate()
            .child(entry.title),
    );

    // 计数角标：0 不显示（不是「显示 0」）。复用会话区的徽标样式，同一条边栏里
    // 「角标」长一个样，不另造一套。
    if entry.count > 0 {
        element = element.child(render_count_badge(entry.count, theme));
    }

    let entity = entity.clone();
    let kind = entry.kind;
    element.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            if view.state_mut().select_nav(kind) {
                cx.notify();
            }
        });
    })
}

/// 「更多」组：头行（图标位 + 「更多」+ 计数徽标 + 展开 / 收起钮）+ 收纳的旧入口。
///
/// 头行点击只翻转展开态（`cx.notify()` 重渲染）；旧入口点击仍走 A15 的选中态，
/// ⛔ 都不触发运行时行为。
fn render_more_group(
    entries: &[NavEntry],
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    let open = more_open();
    let chevron = if open {
        EXPANDED_CHEVRON
    } else {
        COLLAPSED_CHEVRON
    };
    let count = entries.len();

    let entity_for_toggle = entity.clone();
    let mut header = div()
        .id("sidebar-nav-more-toggle")
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_2()
        .rounded_sm()
        .cursor_pointer()
        .child(
            div()
                .flex()
                .w_4()
                .h_4()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(theme.sidebar_foreground)
                .child(MAIN_ENTRY_GLYPHS[5]),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.sidebar_foreground)
                .truncate()
                .child(MORE_LABEL),
        );
    // 计数徽标只在非空时出现（同旧入口角标约定：0 不显示）。
    if count > 0 {
        header = header.child(render_count_badge(count, theme));
    }
    let header = header
        .child(
            div()
                .ml_auto()
                .flex()
                .items_center()
                .gap_1()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(chevron),
        )
        .on_click(move |_event, _window, app| {
            let entity = entity_for_toggle.clone();
            entity.update(app, |_, cx| {
                toggle_more_open();
                cx.notify();
            });
        });

    let mut group = div().flex().flex_col().gap_1().pt_2().child(header);
    if open {
        group = group.children(
            entries
                .iter()
                .map(|entry| render_nav_entry(entry, entity, theme)),
        );
    }
    group
}

/// 左栏导航列（D1/D2）：上区主入口 + 「任务 / 空间」分组 + 「更多」组 + 底部卡位。
///
/// 顺序与条目由描述层决定（`entries` = 现有 14 项旧入口，「更多」组收纳）。
pub(super) fn render_nav(
    entries: &[NavEntry],
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .id("sidebar-nav-rail")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px_1()
        .py_1()
        .child(render_main_entries(theme))
        .child(render_group(TASKS_GROUP_LABEL, theme))
        .child(render_group(SPACES_GROUP_LABEL, theme))
        .child(render_more_group(entries, entity, theme))
        .child(render_footer(theme))
}

#[cfg(test)]
mod tests {
    //! D1/D2 分组导航保真断言 + A15 旧入口回归。
    //!
    //! 显式导入、不写 `use super::*`（gpui_kit 的 `test` 宏会遮蔽标准库 `#[test]`
    //! ⇒ recursion limit，同 render_assert_tests 的约定）。

    use gpui_kit::component::Theme;
    use gpui_kit::{AppContext as _, IntoElement as _};

    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use crate::ui::sidebar::nav::{agents_entry, channels_entry, nav_entries, NavKind};
    use crate::ui::sidebar_gpui::render::HostedElement;
    use crate::ui::sidebar_gpui::SidebarViewGpui;

    use super::{render_nav, render_nav_entry, set_more_open};

    /// ①（元素级）整列导航画得出来且布局非零（主入口 + 两组 + 更多 + 底部）。
    #[test]
    fn nav_rail_renders_nonzero() {
        set_more_open(true);
        let entries = nav_entries(2, 3, 0, 0, None);
        let bounds = draw_probe(|ctx| {
            let entity = ctx.new(|_| SidebarViewGpui::demo());
            let nav = render_nav(&entries, &entity, &Theme::default()).into_any_element();
            ctx.new(|_| HostedElement(nav)).into_any_element()
        });
        assert_rendered(bounds, "左栏分组导航（收起态）");
    }

    /// ① 标题可定位（A15 回归）：频道 / 智能体管理入口，标题逐字取自描述层。
    #[test]
    fn channel_and_agent_titles_are_locatable() {
        let entries = nav_entries(2, 3, 0, 0, None);
        let channel = entries
            .iter()
            .find(|entry| entry.kind == NavKind::Channels)
            .expect("必须有频道入口");
        let agent = entries
            .iter()
            .find(|entry| entry.kind == NavKind::Agents)
            .expect("必须有智能体管理入口");
        assert_eq!(channel.title, "频道", "频道入口标题必须逐字");
        assert_eq!(agent.title, "智能体管理", "智能体管理入口标题必须逐字");
    }

    /// ②（元素级）旧入口行画得出来且布局非零（A15 回归）。
    #[test]
    fn nav_entries_render_nonzero() {
        let entries = nav_entries(2, 3, 0, 0, None);
        let bounds = draw_probe(|ctx| {
            let entity = ctx.new(|_| SidebarViewGpui::demo());
            let nav = render_nav(&entries, &entity, &Theme::default()).into_any_element();
            ctx.new(|_| HostedElement(nav)).into_any_element()
        });
        assert_rendered(bounds, "左栏分组导航（含旧入口行）");
    }

    /// ③ 计数角标两态（A15 回归）：计数为 0 不渲染角标（更窄），非 0 渲染（更宽）。
    #[test]
    fn nav_count_badge_has_both_states() {
        let draw = |count: usize| {
            draw_probe(|ctx| {
                let entity = ctx.new(|_| SidebarViewGpui::demo());
                let entry =
                    render_nav_entry(&channels_entry(count, false), &entity, &Theme::default())
                        .into_any_element();
                ctx.new(|_| HostedElement(entry)).into_any_element()
            })
        };
        let with_badge = assert_rendered(draw(2), "频道入口（角标非空）");
        let without_badge = assert_rendered(draw(0), "频道入口（角标空）");
        assert!(
            with_badge.size.width > without_badge.size.width,
            "角标非空的入口必须比角标空的宽（角标没画出来）：{:?} vs {:?}",
            with_badge.size.width,
            without_badge.size.width
        );
    }

    /// ④ 选中分支（A15 回归）：selected 入口走高亮分支，画得出来且布局非零。
    #[test]
    fn nav_selected_entry_renders_nonzero() {
        let bounds = draw_probe(|ctx| {
            let entity = ctx.new(|_| SidebarViewGpui::demo());
            let entry = render_nav_entry(&agents_entry(3, true), &entity, &Theme::default())
                .into_any_element();
            ctx.new(|_| HostedElement(entry)).into_any_element()
        });
        assert_rendered(bounds, "智能体管理入口（选中态）");
    }
}
