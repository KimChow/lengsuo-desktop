//! 边栏折叠态的 GPUI 渲染分支：折叠 / 展开按钮 + 折叠窄条。
//!
//! 本文件是 [`super`](crate::ui::sidebar_gpui) 的子模块，只放**渲染分支**。
//! 类型定义、视图主体与映射层在 `mod.rs`。
//!
//! 硬约束：颜色一律取自 `cx.theme()`，不硬编码；文案一律取自本文件的常量
//! （与右栏的折叠按钮同款：窄条里只放一个字，放不下「展开侧栏」）。

#![cfg(feature = "ui")]

use gpui_kit::component::Theme;
use gpui_kit::*;

use super::SidebarViewGpui;

/// 折叠窄条里的展开按钮文案（窄条只有几十像素宽，放不下「展开侧栏」）。
pub(super) const EXPAND_LABEL: &str = "展";
/// 展开态的折叠按钮文案。
pub(super) const COLLAPSE_LABEL: &str = "收起";

/// 折叠 / 展开按钮。
///
/// ⛔ `on_click` 只在 `Stateful<E>` 上存在 ⇒ 必须先 `.id(...)`（速查表 §12.1）。
pub(super) fn render_toggle(
    collapsed: bool,
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    let label = if collapsed {
        EXPAND_LABEL
    } else {
        COLLAPSE_LABEL
    };
    let color = if collapsed {
        theme.muted_foreground
    } else {
        theme.sidebar_foreground
    };

    let entity = entity.clone();
    div()
        .id("sidebar-collapse-toggle")
        .px_2()
        .py_1()
        .rounded_sm()
        .text_xs()
        .text_color(color)
        .border_1()
        .border_color(theme.sidebar_border)
        .cursor_pointer()
        .child(label)
        .on_click(move |_event, _window, app| {
            entity.update(app, |view, cx| {
                view.toggle_collapsed();
                cx.notify();
            });
        })
}

/// 折叠态窄条的内容：只保留展开入口，**不渲染会话列表**。
///
/// 背景 / 右边框由调用方的外框负责（与展开态共用同一层外框）。
pub(super) fn render_collapsed_strip(
    entity: &Entity<SidebarViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .p_1()
        .child(render_toggle(true, entity, theme))
}
