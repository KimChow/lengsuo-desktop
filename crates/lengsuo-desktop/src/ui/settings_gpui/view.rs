//! 设置窗口的渲染描述：15 个面板的导航清单 + 当前选中面板 + 主题取值。
//!
//! 纯数据、无 GPUI 依赖 ⇒ 可在无窗口环境直接测（`cargo test` **不开** `ui`
//! feature 也覆盖到本模块）。
//!
//! 分层约定（与 [`crate::ui::command_palette`] 等描述层一致）：
//! - 面板清单是**唯一真源**：接线层不另造一套标题，也不自己数面板。
//! - 主题取值的语义与 [`crate::theme::tokens::Theme`] 一一对应
//!   （`System` / `Light` / `Dark`）；本层只做「取值 ↔ 语义层」的映射，
//!   「解析成亮/暗并应用到渲染层」由接线层负责。
//! - 面板**只有导航与占位内容**：除「外观」与「安全中心」外的 13 个面板功能
//!   尚未实现，详见 `docs/settings-window-status.md`。
//! - 「安全中心」的能力位展示**只读**服务端下发的 `can_*` 标记：
//!   ⛔ 本层**不做授权推理**，只把服务端结果映射成展示文案。

use crate::state::org_perm::{Capability, CapabilityFlags, DenialReason};
use crate::theme::tokens::Theme as SemanticTheme;

/// 面板标识（稳定 id，派发用，不用标题派发）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    /// 账号。
    Account,
    /// 模型。
    Models,
    /// 外观（主题）。
    Appearance,
    /// 软件配置。
    SoftwareConfig,
    /// 扩展。
    Extensions,
    /// 安全中心。
    Security,
    /// 记忆。
    Memory,
    /// 个性化。
    Personalization,
    /// 数据管理。
    DataManagement,
    /// 订阅用量。
    SubscriptionUsage,
    /// 系统权限。
    SystemPermissions,
    /// 帮助与反馈。
    Help,
    /// Claw 设置。
    Claw,
    /// 实验室。
    Laboratory,
    /// MCP 授权同意。
    McpConsent,
}

impl PanelId {
    /// 稳定 slug：用作 GPUI 元素 id 的后缀（同屏唯一、跨版本稳定）。
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Models => "models",
            Self::Appearance => "appearance",
            Self::SoftwareConfig => "software-config",
            Self::Extensions => "extensions",
            Self::Security => "security",
            Self::Memory => "memory",
            Self::Personalization => "personalization",
            Self::DataManagement => "data-management",
            Self::SubscriptionUsage => "subscription-usage",
            Self::SystemPermissions => "system-permissions",
            Self::Help => "help",
            Self::Claw => "claw",
            Self::Laboratory => "laboratory",
            Self::McpConsent => "mcp-consent",
        }
    }
}

/// 一个面板的导航项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelEntry {
    /// 面板 id。
    pub id: PanelId,
    /// 展示标题（中文，取自描述层；接线层不另造一套）。
    pub title: &'static str,
}

/// 15 个面板的导航清单（顺序即展示顺序）。
///
/// 顺序对齐 `docs/ui-parity-spec.md` 的 A8 条目：账号 → 模型 → 外观 →
/// 软件配置 → 扩展 → 安全中心 → 记忆 → 个性化 → 数据管理 → 订阅用量 →
/// 系统权限 → 帮助与反馈 → Claw 设置 → 实验室 → MCP 授权同意。
pub const PANELS: &[PanelEntry] = &[
    PanelEntry {
        id: PanelId::Account,
        title: "账号",
    },
    PanelEntry {
        id: PanelId::Models,
        title: "模型",
    },
    PanelEntry {
        id: PanelId::Appearance,
        title: "外观",
    },
    PanelEntry {
        id: PanelId::SoftwareConfig,
        title: "软件配置",
    },
    PanelEntry {
        id: PanelId::Extensions,
        title: "扩展",
    },
    PanelEntry {
        id: PanelId::Security,
        title: "安全中心",
    },
    PanelEntry {
        id: PanelId::Memory,
        title: "记忆",
    },
    PanelEntry {
        id: PanelId::Personalization,
        title: "个性化",
    },
    PanelEntry {
        id: PanelId::DataManagement,
        title: "数据管理",
    },
    PanelEntry {
        id: PanelId::SubscriptionUsage,
        title: "订阅用量",
    },
    PanelEntry {
        id: PanelId::SystemPermissions,
        title: "系统权限",
    },
    PanelEntry {
        id: PanelId::Help,
        title: "帮助与反馈",
    },
    PanelEntry {
        id: PanelId::Claw,
        title: "Claw 设置",
    },
    PanelEntry {
        id: PanelId::Laboratory,
        title: "实验室",
    },
    PanelEntry {
        id: PanelId::McpConsent,
        title: "MCP 授权同意",
    },
];

/// 主题取值（对应本仓 [`crate::theme::tokens::Theme`] 的三档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeChoice {
    /// 跟随系统。
    #[default]
    System,
    /// 浅色。
    Light,
    /// 深色。
    Dark,
}

impl ThemeChoice {
    /// 映射到本仓**语义层**的主题枚举（`tokens::Theme`）。
    #[must_use]
    pub const fn semantic(self) -> SemanticTheme {
        match self {
            Self::System => SemanticTheme::System,
            Self::Light => SemanticTheme::Light,
            Self::Dark => SemanticTheme::Dark,
        }
    }

    /// 稳定 slug（GPUI 元素 id 用，同屏唯一）。
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// 主题取值的三个可选项（设置窗口「外观」面板用）。
pub const THEME_CHOICES: &[ThemeChoice] =
    &[ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];

/// 主题取值的展示文案。
#[must_use]
pub const fn theme_label(choice: ThemeChoice) -> &'static str {
    match choice {
        ThemeChoice::System => "跟随系统",
        ThemeChoice::Light => "浅色",
        ThemeChoice::Dark => "深色",
    }
}

/// 面板选中态 + 主题取值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsView {
    /// 面板清单（导航项）。
    pub panels: Vec<PanelEntry>,
    /// 当前选中的面板。
    pub selected: PanelId,
    /// 当前主题取值。
    pub theme: ThemeChoice,
}

impl Default for SettingsView {
    fn default() -> Self {
        Self {
            panels: PANELS.to_vec(),
            selected: PanelId::Account,
            theme: ThemeChoice::default(),
        }
    }
}

impl SettingsView {
    /// 选中面板；返回是否**发生了变化**（没变则不必重绘）。
    pub fn select(&mut self, id: PanelId) -> bool {
        if self.selected == id {
            return false;
        }
        self.selected = id;
        true
    }

    /// 设置主题取值；返回是否**发生了变化**（没变则不必重绘）。
    pub fn set_theme(&mut self, theme: ThemeChoice) -> bool {
        if self.theme == theme {
            return false;
        }
        self.theme = theme;
        true
    }

    /// 当前面板的展示标题（找不到时回落到「设置」）。
    #[must_use]
    pub fn current_title(&self) -> &'static str {
        self.panels
            .iter()
            .find(|panel| panel.id == self.selected)
            .map_or("设置", |panel| panel.title)
    }
}

/// 能力位的中文名（设置窗口「安全中心」面板用）。
///
/// ⛔ 这里只做**展示文案**，不含任何授权语义。
#[must_use]
pub const fn capability_label(cap: Capability) -> &'static str {
    match cap {
        Capability::ViewMembers => "查看成员",
        Capability::InviteMember => "邀请成员",
        Capability::EditMember => "编辑成员",
        Capability::RemoveMember => "移除成员",
        Capability::CreateRole => "创建角色",
        Capability::EditRole => "编辑角色",
        Capability::DeleteRole => "删除角色",
        Capability::AssignRole => "分配角色",
        Capability::ManageDepartment => "管理部门",
    }
}

/// 能力位在「安全中心」面板里的展示结论（**只读**服务端结果）。
///
/// ⛔ 本枚举**不做授权推理**：可用性完全来自服务端下发的 `can_*` 标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    /// 服务端明确下发 `true` ⇒ 可用。
    Available,
    /// 服务端下发 `false`（或标记与原因码矛盾）⇒ 不可用。
    ///
    /// `denial` 仅承载服务端给出的原因码；**没给就是没给，不编造**。
    Denied(Option<DenialReason>),
    /// 服务端**没有下发**该标记。
    ///
    /// ⚠️ 与 [`CapabilityStatus::Denied`] 是**两件事**：
    /// 「未下发」不代表服务端判定不可用，只代表界面拿不到结论。
    NotDelivered,
}

impl CapabilityStatus {
    /// 状态文案（自解释，**不与其它状态混用**）。
    ///
    /// - 可用 ⇒ 「可用」；
    /// - 不可用 ⇒ 「不可用：{服务端原因}」（服务端没给原因就只说「不可用」）；
    /// - 未下发 ⇒ 「服务端未下发」（**不是「不可用」**：服务端没表达任何结论）。
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::Available => "可用".to_owned(),
            Self::Denied(Some(reason)) => format!("不可用：{}", reason.message()),
            Self::Denied(None) => "不可用".to_owned(),
            Self::NotDelivered => "服务端未下发".to_owned(),
        }
    }
}

/// 「安全中心」面板的一行：一个能力位 + 它的展示结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRow {
    /// 能力位（对应服务端的 `can_*` 标记字段）。
    pub capability: Capability,
    /// 中文名。
    pub label: &'static str,
    /// 展示结论。
    pub status: CapabilityStatus,
}

/// 服务端是否下发了**至少一个**能力标记。
///
/// 用于区分「服务端未下发能力标记」与「服务端下发了、但某个能力不可用」：
/// 前者是**取不到结论**，后者才是**结论为不可用**。
#[must_use]
pub fn has_delivered_marks(flags: &CapabilityFlags) -> bool {
    Capability::ALL.iter().any(|cap| flags.has_mark(*cap))
}

/// 「安全中心」面板的行清单：固定 9 个能力位，逐个给出**只读**结论。
///
/// 顺序即 [`Capability::ALL`] 的顺序。
#[must_use]
pub fn capability_rows(flags: &CapabilityFlags) -> Vec<CapabilityRow> {
    Capability::ALL
        .iter()
        .map(|cap| {
            let capability = *cap;
            // ⚠️ 「未下发」与「不可用」必须分开：`has_mark` 为假时
            // 服务端**没有表达任何结论**，不能当成服务端判定的「不可用」。
            let status = if !flags.has_mark(capability) {
                CapabilityStatus::NotDelivered
            } else {
                let eff = flags.effective(capability);
                if eff.is_allowed() {
                    CapabilityStatus::Available
                } else {
                    CapabilityStatus::Denied(eff.denial())
                }
            };
            CapabilityRow {
                capability,
                label: capability_label(capability),
                status,
            }
        })
        .collect()
}
