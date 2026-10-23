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

use super::SidebarViewGpui;
use super::render::render_count_badge;

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
