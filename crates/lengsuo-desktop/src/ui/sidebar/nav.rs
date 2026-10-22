//! 边栏的四个导航入口：频道入口 + 智能体管理入口 + 浏览器环境管理入口 + 组织入口。
//!
//! **只做界面**：入口的标题、计数、选中态。**不实现**频道收发、不实现智能体创建
//! （那是运行时/业务语义，⛔ 本仓不碰）。
//!
//! 纯数据、无 GPUI 依赖 ⇒ 可在无窗口环境直接测。

/// 边栏的导航入口种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavKind {
    /// 频道入口（会话注册成 IM 频道，产出自动推送）。
    Channels,
    /// 智能体管理入口（多代理组织）。
    Agents,
    /// **浏览器环境管理入口**（B 域；e35 新增）。
    Environments,
    /// **组织与权限可见性入口**（A 域；e38 新增）。
    Organization,
}

/// 一个导航入口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavEntry {
    /// 入口种类（决定文案与 id）。
    pub kind: NavKind,
    /// 展示标题（中文，取自本模块，接线层不另造一套）。
    pub title: &'static str,
    /// 该入口下的条目数（角标；0 表示不显示角标）。
    pub count: usize,
    /// 该入口是否被选中。
    pub selected: bool,
}

/// 频道入口的标题。
pub const CHANNELS_TITLE: &str = "频道";
/// 智能体管理入口的标题。
pub const AGENTS_TITLE: &str = "智能体管理";
/// 浏览器环境管理入口的标题。
pub const ENVIRONMENTS_TITLE: &str = "环境";
/// **组织与权限可见性入口**的标题（A 域；e38 新增）。
pub const ORGANIZATION_TITLE: &str = "组织";

/// 构造频道入口。
#[must_use]
pub fn channels_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Channels,
        title: CHANNELS_TITLE,
        count,
        selected,
    }
}

/// 构造智能体管理入口。
#[must_use]
pub fn agents_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Agents,
        title: AGENTS_TITLE,
        count,
        selected,
    }
}

/// 构造浏览器环境管理入口。
#[must_use]
pub fn environments_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Environments,
        title: ENVIRONMENTS_TITLE,
        count,
        selected,
    }
}

/// 构造组织与权限可见性入口。
#[must_use]
pub fn organization_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Organization,
        title: ORGANIZATION_TITLE,
        count,
        selected,
    }
}

/// 边栏的四个导航入口（顺序即展示顺序：频道 → 智能体管理 → 环境 → 组织）。
#[must_use]
pub fn nav_entries(
    channels: usize,
    agents: usize,
    environments: usize,
    organization: usize,
    selected: Option<NavKind>,
) -> Vec<NavEntry> {
    vec![
        channels_entry(channels, selected == Some(NavKind::Channels)),
        agents_entry(agents, selected == Some(NavKind::Agents)),
        environments_entry(environments, selected == Some(NavKind::Environments)),
        organization_entry(organization, selected == Some(NavKind::Organization)),
    ]
}
