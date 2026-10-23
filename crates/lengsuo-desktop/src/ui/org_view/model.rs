//! 组织域视图的数据类型：区域展示态、按钮态、拒绝提示与三类行/节点。

use crate::state::org_perm::DenialReason;

/// 列表区域的展示态。失败与空集必须分开。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionView<'a> {
    Loading,
    Failed { message: &'a str },
    Empty { text: &'static str },
    Ready,
}

/// 按钮状态。`Disabled` 的 `notice` 仅在服务端给出原因时存在。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionState {
    Enabled,
    Disabled { notice: Option<DenialNotice> },
}

impl ActionState {
    #[must_use]
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

/// 拒绝提示：文案、说明与自助入口分开。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenialNotice {
    pub message: &'static str,
    /// 无自助路径时的说明（如「联系管理员」）。
    pub hint: Option<&'static str>,
    /// 可点击的自助入口；策略拒绝为 `None`。
    pub action: Option<&'static str>,
}

/// 由服务端原因码生成提示。**不把三种原因合并成一句「无权限」**。
#[must_use]
pub const fn denial_notice(reason: DenialReason) -> DenialNotice {
    let message = reason.message();
    match reason {
        DenialReason::NoEntitlement => DenialNotice {
            message,
            hint: None,
            action: Some("升级购买"),
        },
        DenialReason::NoPermission => DenialNotice {
            message,
            hint: None,
            action: Some("申请权限"),
        },
        DenialReason::PolicyDenied => DenialNotice {
            message,
            hint: Some("请联系企业管理员"),
            action: None,
        },
    }
}

/// 部门树节点（先序 + 深度，直接驱动层级缩进渲染）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartmentNode {
    pub id: String,
    pub name: Option<String>,
    pub depth: usize,
    pub has_children: bool,
}

/// 成员行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRow {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

/// 角色行。`preset_key` / `is_builtin` / `is_ai_assistant` 原样展示。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleRow {
    pub id: String,
    pub name: Option<String>,
    pub preset_key: Option<String>,
    pub is_builtin: bool,
    pub is_ai_assistant: bool,
    /// 全局只读：AI 助手角色禁编辑、禁删除。
    pub read_only: bool,
    pub edit: ActionState,
    pub delete: ActionState,
}
