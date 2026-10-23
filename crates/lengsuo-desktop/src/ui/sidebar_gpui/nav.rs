//! 边栏导航入口的 GPUI 渲染：频道入口 + 智能体管理入口（+ 环境 / 组织）。
//!
//! 对齐 A15 的**界面部分**：入口可点、有计数角标、选中态明确。
//! ⛔ 不实现频道收发、不实现智能体创建、不实现多代理角色模型 ——
//! 点击只改描述层里的选中态，不产生任何运行时行为。
//!
//! **不引入任何组件**：全部用 `div()` 自搭 ⇒ 零新依赖、零 API 不确定性。

#![cfg(feature = "ui")]

use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::sidebar::nav::NavEntry;

use super::render::render_count_badge;
use super::SidebarViewGpui;

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

/// 边栏顶部的导航入口列表（顺序与条目由描述层决定）。
pub(super) fn render_nav(
    entries: &[NavEntry],
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    div().flex().flex_col().px_1().py_1().children(
        entries
            .iter()
            .map(|entry| render_nav_entry(entry, entity, theme)),
    )
}

#[cfg(test)]
mod tests {
    //! A15 导航入口保真断言：① 标题可定位 ② 计数角标空/非空两态 ③ 选中分支。
    //!
    //! 显式导入、不写 `use super::*`（同 render_assert_tests 的约定，
    //! 防 GPUI 的 `test` 属性宏遮蔽标准库 `#[test]`）。

    use gpui_kit::component::Theme;
    use gpui_kit::{AppContext as _, IntoElement as _};

    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use crate::ui::sidebar::nav::{agents_entry, channels_entry, nav_entries, NavKind};
    use crate::ui::sidebar_gpui::SidebarViewGpui;

    use crate::ui::sidebar_gpui::render::HostedElement;

    use super::{render_nav, render_nav_entry};

    /// ① 标题可定位：频道 / 智能体管理入口都在，标题逐字取自描述层。
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

    /// ①（元素级）：整条导航画得出来且布局非零。
    #[test]
    fn nav_entries_render_nonzero() {
        let entries = nav_entries(2, 3, 0, 0, None);
        let bounds = draw_probe(|ctx| {
            let entity = ctx.new(|_| SidebarViewGpui::demo());
            let nav = render_nav(&entries, &entity, &Theme::default()).into_any_element();
            ctx.new(|_| HostedElement(nav)).into_any_element()
        });
        assert_rendered(bounds, "导航入口（频道 + 智能体管理）");
    }

    /// ② 计数角标两态：计数为 0 不渲染角标（更窄），非 0 渲染（更宽）。
    ///
    /// 宽度判别能抓住「角标分支没走到 / 空态也画了个 0」两类回归。
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

    /// ③ 选中分支：selected 入口走高亮分支，画得出来且布局非零。
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
