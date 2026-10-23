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
    /// **技能入口**（波次 5）。
    SkillsMarket,
    /// **专家中心入口**（波次 5）。
    ExpertCenter,
    /// **灵感社区入口**（波次 5）。
    Discover,
    /// **我的文件入口**（波次 5）。
    MyFiles,
    /// **助理协作空间入口**（波次 5）。
    Colleagues,
    /// **团队知识库入口**（波次 5）。
    KnowledgeBase,
    /// **定时任务入口**（波次 5）。
    Automation,
    /// **连接器入口**（波次 5）。
    Connectors,
    /// **智能体邮箱入口**（波次 5）。
    AgentMail,
    /// **消息中心入口**（波次 5）。
    MessageCenter,
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
/// 技能入口的标题（逐字取自界面文案全表）。
pub const SKILLSMARKET_TITLE: &str = "技能";
/// 专家中心入口的标题（逐字取自界面文案全表）。
pub const EXPERT_CENTER_TITLE: &str = "专家中心";
/// 灵感社区入口的标题（逐字取自界面文案全表）。
pub const DISCOVER_TITLE: &str = "灵感社区";
/// 我的文件入口的标题（逐字取自界面文案全表）。
pub const MY_FILES_TITLE: &str = "我的文件";
/// 助理协作空间入口的标题（逐字取自界面文案全表）。
pub const COLLEAGUES_TITLE: &str = "助理协作空间";
/// 团队知识库入口的标题（逐字取自界面文案全表）。
pub const KNOWLEDGE_BASE_TITLE: &str = "团队知识库";
/// 定时任务入口的标题（逐字取自界面文案全表）。
pub const AUTOMATION_TITLE: &str = "定时任务";
/// 连接器入口的标题（逐字取自界面文案全表）。
pub const CONNECTORS_TITLE: &str = "连接器";
/// 智能体邮箱入口的标题（逐字取自界面文案全表）。
pub const AGENT_MAIL_TITLE: &str = "智能体邮箱";
/// 消息中心入口的标题（逐字取自界面文案全表）。
pub const MESSAGE_CENTER_TITLE: &str = "消息中心";

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

/// 边栏的**十四个**导航入口（顺序即展示顺序：频道 → 智能体管理 → 环境 → 组织
/// → 技能 → 专家中心 → 灵感社区 → 我的文件 → 助理协作空间 → 团队知识库
/// → 定时任务 → 连接器 → 智能体邮箱 → 消息中心）。
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
        skills_market_entry(0, selected == Some(NavKind::SkillsMarket)),
        expert_center_entry(0, selected == Some(NavKind::ExpertCenter)),
        discover_entry(0, selected == Some(NavKind::Discover)),
        my_files_entry(0, selected == Some(NavKind::MyFiles)),
        colleagues_entry(0, selected == Some(NavKind::Colleagues)),
        knowledge_base_entry(0, selected == Some(NavKind::KnowledgeBase)),
        automation_entry(0, selected == Some(NavKind::Automation)),
        connectors_entry(0, selected == Some(NavKind::Connectors)),
        agent_mail_entry(0, selected == Some(NavKind::AgentMail)),
        message_center_entry(0, selected == Some(NavKind::MessageCenter)),
    ]
}

/// 构造技能入口。
#[must_use]
pub fn skills_market_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::SkillsMarket,
        title: SKILLSMARKET_TITLE,
        count,
        selected,
    }
}

/// 构造专家中心入口。
#[must_use]
pub fn expert_center_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::ExpertCenter,
        title: EXPERT_CENTER_TITLE,
        count,
        selected,
    }
}

/// 构造灵感社区入口。
#[must_use]
pub fn discover_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Discover,
        title: DISCOVER_TITLE,
        count,
        selected,
    }
}

/// 构造我的文件入口。
#[must_use]
pub fn my_files_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::MyFiles,
        title: MY_FILES_TITLE,
        count,
        selected,
    }
}

/// 构造助理协作空间入口。
#[must_use]
pub fn colleagues_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Colleagues,
        title: COLLEAGUES_TITLE,
        count,
        selected,
    }
}

/// 构造团队知识库入口。
#[must_use]
pub fn knowledge_base_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::KnowledgeBase,
        title: KNOWLEDGE_BASE_TITLE,
        count,
        selected,
    }
}

/// 构造定时任务入口。
#[must_use]
pub fn automation_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Automation,
        title: AUTOMATION_TITLE,
        count,
        selected,
    }
}

/// 构造连接器入口。
#[must_use]
pub fn connectors_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::Connectors,
        title: CONNECTORS_TITLE,
        count,
        selected,
    }
}

/// 构造智能体邮箱入口。
#[must_use]
pub fn agent_mail_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::AgentMail,
        title: AGENT_MAIL_TITLE,
        count,
        selected,
    }
}

/// 构造消息中心入口。
#[must_use]
pub fn message_center_entry(count: usize, selected: bool) -> NavEntry {
    NavEntry {
        kind: NavKind::MessageCenter,
        title: MESSAGE_CENTER_TITLE,
        count,
        selected,
    }
}
