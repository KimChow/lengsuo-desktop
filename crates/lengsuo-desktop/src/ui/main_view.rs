//! 主区视图路由：侧栏导航入口 → 主区显示哪一块。
//!
//! **只做界面**：本模块只决定「主区当前显示哪个视图」，⛔ 不实现频道收发、
//! 不实现智能体创建（那是运行时 / 业务语义，本仓不碰）。
//!
//! 纯数据、无 GPUI 依赖 ⇒ 可在无窗口环境直接测。
//!
//! 对齐规格 `ui-parity-spec.md` §1 条目 **A15**：边栏的频道入口与智能体管理入口
//! 已就位（e19），但入口点击后主区**没有消费者** —— 本模块补上这段路由。

use crate::ui::sidebar::nav::NavKind;

/// 主区当前显示的视图。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    /// 首页（默认；照参照：大标题 + 场景徽章 + 推荐问句）。
    Home,
    /// 会话流（还没选任何导航入口时）。
    Sessions,
    /// 频道视图（点侧栏「频道」入口）。
    Channels,
    /// 智能体管理视图（点侧栏「智能体管理」入口）。
    Agents,
    /// **浏览器环境管理**（B 域；点侧栏「环境」入口）。
    Environments,
    /// **组织与权限可见性**（A 域；点侧栏「组织」入口）。
    Organization,
    /// **技能**（点侧栏「技能」入口）。
    SkillsMarket,
    /// **专家中心**（点侧栏「专家中心」入口）。
    ExpertCenter,
    /// **灵感社区**（点侧栏「灵感社区」入口）。
    Discover,
    /// **我的文件**（点侧栏「我的文件」入口）。
    MyFiles,
    /// **助理协作空间**（点侧栏「助理协作空间」入口）。
    Colleagues,
    /// **团队知识库**（点侧栏「团队知识库」入口）。
    KnowledgeBase,
    /// **定时任务**（点侧栏「定时任务」入口）。
    Automation,
    /// **连接器**（点侧栏「连接器」入口）。
    Connectors,
    /// **智能体邮箱**（点侧栏「智能体邮箱」入口）。
    AgentMail,
    /// **消息中心**（点侧栏「消息中心」入口）。
    MessageCenter,
}

/// 频道视图的说明文案（数据源在运行时侧，本仓未接）。
pub const CHANNELS_PLACEHOLDER: &str = "频道：数据源在运行时侧，本仓暂不提供";
/// 智能体管理视图的说明文案。
pub const AGENTS_PLACEHOLDER: &str = "智能体管理：数据源在运行时侧，本仓暂不提供";

/// 根据侧栏选中的导航入口，决定主区显示什么。
///
/// - `None` ⇒ [`MainView::Sessions`]（还没选任何入口 ⇒ 显示会话流）。
/// - `Some(Channels)` ⇒ [`MainView::Channels`]。
/// - `Some(Agents)` ⇒ [`MainView::Agents`]。
/// - `Some(Environments)` ⇒ [`MainView::Environments`]。
/// - `Some(Organization)` ⇒ [`MainView::Organization`]。
#[must_use]
pub fn resolve(selected_nav: Option<NavKind>) -> MainView {
    match selected_nav {
        None => MainView::Sessions,
        Some(NavKind::Channels) => MainView::Channels,
        Some(NavKind::Agents) => MainView::Agents,
        Some(NavKind::Environments) => MainView::Environments,
        Some(NavKind::Organization) => MainView::Organization,
        Some(NavKind::SkillsMarket) => MainView::SkillsMarket,
        Some(NavKind::ExpertCenter) => MainView::ExpertCenter,
        Some(NavKind::Discover) => MainView::Discover,
        Some(NavKind::MyFiles) => MainView::MyFiles,
        Some(NavKind::Colleagues) => MainView::Colleagues,
        Some(NavKind::KnowledgeBase) => MainView::KnowledgeBase,
        Some(NavKind::Automation) => MainView::Automation,
        Some(NavKind::Connectors) => MainView::Connectors,
        Some(NavKind::AgentMail) => MainView::AgentMail,
        Some(NavKind::MessageCenter) => MainView::MessageCenter,
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::main_view::{resolve, MainView};
    use crate::ui::sidebar::nav::NavKind;
    use crate::ui::sidebar::SidebarState;

    #[test]
    fn no_nav_selected_shows_sessions() {
        assert_eq!(resolve(None), MainView::Sessions);
    }

    #[test]
    fn channels_nav_switches_to_channels() {
        assert_eq!(resolve(Some(NavKind::Channels)), MainView::Channels);
    }

    #[test]
    fn agents_nav_switches_to_agents() {
        assert_eq!(resolve(Some(NavKind::Agents)), MainView::Agents);
    }

    /// A17：浏览器环境管理入口 ⇒ 主区切到环境视图。
    #[test]
    fn environments_nav_switches_to_environments() {
        assert_eq!(resolve(Some(NavKind::Environments)), MainView::Environments);
    }

    /// A18：组织与权限可见性入口 ⇒ 主区切到组织视图。
    #[test]
    fn organization_nav_switches_to_organization() {
        assert_eq!(resolve(Some(NavKind::Organization)), MainView::Organization);
    }

    /// 端到端契约（纯逻辑版）：侧栏点击写入的**那个** `NavKind`，
    /// 必须正好是主区路由读到的那个。
    ///
    /// 侧栏的点击处理走的就是 [`crate::ui::sidebar_gpui::select_nav`] 这一条路径；
    /// 根视图的 [`crate::ui::gpui_app`] 观察侧栏后读 `state().selected_nav()`
    /// 再交给 [`resolve`]。此测试锁住两者的**类型一致性** ——
    /// 之前入口是死按钮，就是因为这段链路根本不存在。
    #[test]
    fn the_nav_kind_written_by_the_sidebar_click_drives_the_main_view() {
        let mut state = SidebarState::new();
        assert!(crate::ui::sidebar_gpui::select_nav(
            &mut state,
            NavKind::Channels
        ));
        assert_eq!(resolve(state.selected_nav()), MainView::Channels);

        assert!(crate::ui::sidebar_gpui::select_nav(
            &mut state,
            NavKind::Agents
        ));
        assert_eq!(resolve(state.selected_nav()), MainView::Agents);

        // A17：环境入口走的是同一条链路（侧栏点击 → 描述层选中态 → 主区路由）。
        assert!(crate::ui::sidebar_gpui::select_nav(
            &mut state,
            NavKind::Environments
        ));
        assert_eq!(resolve(state.selected_nav()), MainView::Environments);

        // A18：组织入口走的是同一条链路（侧栏点击 → 描述层选中态 → 主区路由）。
        assert!(crate::ui::sidebar_gpui::select_nav(
            &mut state,
            NavKind::Organization
        ));
        assert_eq!(resolve(state.selected_nav()), MainView::Organization);
    }
}
