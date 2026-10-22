//! 主区（中间那列）的渲染：按侧栏选中的导航入口切换显示哪一块。
//!
//! 对齐 A15 的**缺口部分**：侧栏的频道入口 / 智能体管理入口已就位（e19），
//! 但主区不会跟着切 —— 本模块是主区唯一的消费者。
//!
//! e35（A17）补上第三个入口：环境管理渲染**真实视图**（`EnvViewGpui`），
//! 不是占位。
//!
//! e38（A18）补上第四个入口：组织与权限可见性渲染**真实视图**（`OrgViewGpui`），
//! 不是占位。
//!
//! ⛔ 频道 / 智能体管理**只显示占位说明**：数据源在运行时侧，本仓不实现，
//! 绝不伪造列表。

#![cfg(feature = "ui")]

use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::env_gpui::EnvViewGpui;
use crate::ui::layout::STREAM_MIN_WIDTH;
use crate::ui::main_view::{self, MainView};
use crate::ui::org_gpui::OrgViewGpui;
use crate::ui::session_gpui::SessionStreamGpui;

/// 渲染主区。
///
/// 五个分支**结构完全一样**（都是 `div()`）⇒ 返回类型必然一致（都是 `Div`），
/// 不需要 `into_any_element()`。
pub(super) fn render_main_area(
    view: MainView,
    session: &Entity<SessionStreamGpui>,
    env: &Entity<EnvViewGpui>,
    org: &Entity<OrgViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    match view {
        MainView::Sessions => div()
            .flex_1()
            .min_w(px(STREAM_MIN_WIDTH))
            .h_full()
            .child(session.clone()),
        MainView::Channels => placeholder(main_view::CHANNELS_PLACEHOLDER, theme),
        MainView::Agents => placeholder(main_view::AGENTS_PLACEHOLDER, theme),
        // 环境管理是**真实视图**（挂在 `AppShell.env_view` 上）：
        // ⚠️ 条目内容当前是 `EnvViewGpui::demo()` 的演示数据，真实来源在运行时侧。
        MainView::Environments => div()
            .flex_1()
            .min_w(px(STREAM_MIN_WIDTH))
            .h_full()
            .child(env.clone()),
        // 组织与权限可见性是**真实视图**（挂在 `AppShell.org_view` 上）：
        // ⚠️ 条目内容当前是 `demo_org_view()` 的演示数据，真实来源在运行时侧。
        MainView::Organization => div()
            .flex_1()
            .min_w(px(STREAM_MIN_WIDTH))
            .h_full()
            .child(org.clone()),
    }
}

/// 占位说明：居中的一行灰字，说明这块视图的数据源还没接。
///
/// 返回**具体的 `Div`**（不是 `impl IntoElement`）：三个分支的类型必须一致，
/// 不透明类型无法与 `Div` 统一。
fn placeholder(text: &str, theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}
