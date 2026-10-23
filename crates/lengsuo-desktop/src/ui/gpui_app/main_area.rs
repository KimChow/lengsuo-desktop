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
use crate::ui::main_view::MainView;
use crate::ui::org_gpui::OrgViewGpui;
use crate::ui::panel_views;
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
        // 波次 5：频道 / 智能体管理由占位改为**真实视图体**（当前为诚实占位数据，
        // 数据源在运行时侧；等跨线契约就绪后由 switch-source 类卡换源）。
        MainView::Channels => panel_views::channels::main_area(theme),
        MainView::Agents => panel_views::agents::main_area(theme),
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
        MainView::SkillsMarket => panel_views::skills_market::main_area(theme),
        MainView::ExpertCenter => panel_views::expert_center::main_area(theme),
        MainView::Discover => panel_views::discover::main_area(theme),
        MainView::MyFiles => panel_views::my_files::main_area(theme),
        MainView::Colleagues => panel_views::colleagues::main_area(theme),
        MainView::KnowledgeBase => panel_views::knowledge_base::main_area(theme),
        MainView::Automation => panel_views::automation::main_area(theme),
        MainView::Connectors => panel_views::connectors::main_area(theme),
        MainView::AgentMail => panel_views::agent_mail::main_area(theme),
        MainView::MessageCenter => panel_views::message_center::main_area(theme),
    }
}
