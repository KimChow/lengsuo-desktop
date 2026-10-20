//! 组织域能力标记与拒绝原因（界面**只读**服务端结果）。
//!
//! 服务端契约：
//!
//! ```text
//! effective(feature) = entitled(company, feature) AND permitted(member, feature.code)
//! ```
//!
//! 这个 AND **由服务端权威计算**。本模块只读取服务端下发的 `can_*` 标记与
//! 拒绝原因码，绝不根据角色、套餐或用户身份在界面侧重新推导权限。
//! 界面入口可见或禁用都不构成授权证据，后端仍须独立校验。
//!
//! fail-closed：标记缺失、类型不对或载荷自相矛盾时一律按「不可用」处理；
//! 拿不到原因码时也不凭空编造原因。

use std::collections::BTreeMap;

use serde_json::Value;

/// 服务端下发的拒绝原因。双轨独立，界面必须**分原因**展示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenialReason {
    /// `NO_ENTITLEMENT`：公司套餐未包含该功能（权益轨）。
    NoEntitlement,
    /// `NO_PERMISSION`：成员角色没有该操作权限（权限轨）。
    NoPermission,
    /// 企业策略拒绝（策略轨，由策略权威裁决）。
    PolicyDenied,
}

impl DenialReason {
    /// 解析服务端原因码；未知码返回 `None`，不猜测、不归类为「无权限」。
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim() {
            "NO_ENTITLEMENT" => Some(Self::NoEntitlement),
            "NO_PERMISSION" => Some(Self::NoPermission),
            "POLICY_DENIED" => Some(Self::PolicyDenied),
            _ => None,
        }
    }

    /// 界面文案：三种原因**不可合并**成一句「无权限」。
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::NoEntitlement => "当前套餐不包含此功能",
            Self::NoPermission => "你没有该操作权限",
            Self::PolicyDenied => "该操作已被企业策略禁止",
        }
    }

    /// 引导路径。策略拒绝只提示联系管理员，**不接自助申请入口**。
    #[must_use]
    pub const fn guidance(self) -> Guidance {
        match self {
            Self::NoEntitlement => Guidance::Upgrade,
            Self::NoPermission => Guidance::RequestAccess,
            Self::PolicyDenied => Guidance::ContactAdmin,
        }
    }
}

/// 拒绝后的引导路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guidance {
    /// 升级购买（权益轨）。
    Upgrade,
    /// 申请权限（成员权限轨）。
    RequestAccess,
    /// 仅提示联系管理员：企业策略没有自助路径。
    ContactAdmin,
}

impl Guidance {
    /// 引导入口文案。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Upgrade => "升级购买",
            Self::RequestAccess => "申请权限",
            Self::ContactAdmin => "联系管理员",
        }
    }

    /// 是否可渲染自助入口。`ContactAdmin` 只提示，不提供按钮。
    #[must_use]
    pub const fn is_self_service(self) -> bool {
        matches!(self, Self::Upgrade | Self::RequestAccess)
    }
}

/// 界面关心的能力，对应服务端下发的 `can_*` 标记。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    ViewMembers,
    InviteMember,
    EditMember,
    RemoveMember,
    CreateRole,
    EditRole,
    DeleteRole,
    AssignRole,
    ManageDepartment,
}

impl Capability {
    /// 全部能力（解析载荷时遍历，避免漏字段）。
    pub const ALL: [Self; 9] = [
        Self::ViewMembers,
        Self::InviteMember,
        Self::EditMember,
        Self::RemoveMember,
        Self::CreateRole,
        Self::EditRole,
        Self::DeleteRole,
        Self::AssignRole,
        Self::ManageDepartment,
    ];

    /// 对应的服务端标记字段名。
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::ViewMembers => "can_view_members",
            Self::InviteMember => "can_invite_member",
            Self::EditMember => "can_edit_member",
            Self::RemoveMember => "can_remove_member",
            Self::CreateRole => "can_create_role",
            Self::EditRole => "can_edit_role",
            Self::DeleteRole => "can_delete_role",
            Self::AssignRole => "can_assign_role",
            Self::ManageDepartment => "can_manage_department",
        }
    }
}

/// 单个能力的界面侧结果：服务端裁决的**只读快照**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effective {
    allowed: bool,
    denial: Option<DenialReason>,
}

impl Effective {
    /// 服务端明确允许。
    #[must_use]
    pub const fn allowed() -> Self {
        Self {
            allowed: true,
            denial: None,
        }
    }

    /// 不可用；`denial` 仅承载服务端给出的原因。
    #[must_use]
    pub const fn denied(denial: Option<DenialReason>) -> Self {
        Self {
            allowed: false,
            denial,
        }
    }

    #[must_use]
    pub const fn is_allowed(self) -> bool {
        self.allowed
    }

    #[must_use]
    pub const fn denial(self) -> Option<DenialReason> {
        self.denial
    }
}

/// 服务端下发的 `can_*` 标记集合。
///
/// 只保存服务端结果，不保存角色、套餐或用户身份——本结构无法、也不应
/// 重新计算 `entitled AND permitted`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityFlags {
    marks: BTreeMap<&'static str, bool>,
    reasons: BTreeMap<&'static str, DenialReason>,
}

impl CapabilityFlags {
    /// 从一条服务端载荷解析能力标记。
    ///
    /// 只接受 JSON 布尔值：字符串 `"true"`、`null`、缺字段都视为**没有标记**。
    /// 拒绝原因从 `deny_reasons.<can_*>` 读取（例如
    /// `{"can_delete_role": false, "deny_reasons": {"can_delete_role": "NO_PERMISSION"}}`）。
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        let mut marks = BTreeMap::new();
        let mut reasons = BTreeMap::new();
        let reason_map = payload.get("deny_reasons").and_then(Value::as_object);

        for cap in Capability::ALL {
            if let Some(mark) = payload.get(cap.key()).and_then(Value::as_bool) {
                marks.insert(cap.key(), mark);
            }
            if let Some(reason) = reason_map
                .and_then(|m| m.get(cap.key()))
                .and_then(Value::as_str)
                .and_then(DenialReason::from_code)
            {
                reasons.insert(cap.key(), reason);
            }
        }

        Self { marks, reasons }
    }

    /// 服务端是否下发了该标记（仅用于诊断展示，不参与授权推理）。
    #[must_use]
    pub fn has_mark(&self, cap: Capability) -> bool {
        self.marks.contains_key(cap.key())
    }

    /// `effective` 的界面侧表示：**只读服务端结果**。
    ///
    /// - 显式 `true` 且无拒绝原因 → 可用；
    /// - 其余情况（`false` / 缺失 / 类型错误 / `true` 与原因矛盾）→ 不可用。
    ///
    /// 缺失标记时 `denial` 为 `None`：不可用，但不编造原因。
    #[must_use]
    pub fn effective(&self, cap: Capability) -> Effective {
        let reason = self.reasons.get(cap.key()).copied();
        match (self.marks.get(cap.key()).copied(), reason) {
            (Some(true), None) => Effective::allowed(),
            _ => Effective::denied(reason),
        }
    }

    /// 便捷判定，语义等同 [`Self::effective`] 的 `is_allowed`。
    #[must_use]
    pub fn allows(&self, cap: Capability) -> bool {
        self.effective(cap).is_allowed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_mark_defaults_to_disabled() {
        let flags = CapabilityFlags::from_payload(&json!({}));
        let eff = flags.effective(Capability::DeleteRole);
        assert!(!eff.is_allowed(), "缺失标记必须 fail-closed");
        assert!(!flags.allows(Capability::DeleteRole));
        assert!(!flags.has_mark(Capability::DeleteRole));
        assert_eq!(eff.denial(), None, "缺失标记不得编造拒绝原因");
    }

    #[test]
    fn non_boolean_mark_is_treated_as_missing() {
        for value in [json!("true"), json!(1), json!(null)] {
            let flags = CapabilityFlags::from_payload(&json!({"can_edit_role": value}));
            assert!(!flags.has_mark(Capability::EditRole));
            assert!(!flags.allows(Capability::EditRole));
        }
    }

    #[test]
    fn explicit_false_is_disabled_and_true_is_enabled() {
        let denied = CapabilityFlags::from_payload(&json!({"can_remove_member": false}));
        assert!(!denied.allows(Capability::RemoveMember));

        let allowed = CapabilityFlags::from_payload(&json!({"can_remove_member": true}));
        assert!(allowed.allows(Capability::RemoveMember));
        assert_eq!(allowed.effective(Capability::RemoveMember).denial(), None);
    }

    #[test]
    fn only_server_mark_decides_not_roles_or_entitlement() {
        // 界面上没有 can_* 时，哪怕载荷自称 admin / entitled 也不可用。
        let inferred = CapabilityFlags::from_payload(&json!({
            "roles": ["admin"],
            "is_admin": true,
            "entitled": true
        }));
        assert!(!inferred.allows(Capability::DeleteRole));

        // 服务端给了 can_*，界面不再自己做 AND。
        let server_says_yes = CapabilityFlags::from_payload(&json!({
            "roles": [],
            "entitled": false,
            "can_delete_role": true
        }));
        assert!(server_says_yes.allows(Capability::DeleteRole));
    }

    #[test]
    fn three_denial_reasons_stay_distinct() {
        let cases = [
            (
                "NO_ENTITLEMENT",
                DenialReason::NoEntitlement,
                Guidance::Upgrade,
            ),
            (
                "NO_PERMISSION",
                DenialReason::NoPermission,
                Guidance::RequestAccess,
            ),
            (
                "POLICY_DENIED",
                DenialReason::PolicyDenied,
                Guidance::ContactAdmin,
            ),
        ];
        let mut messages = Vec::new();
        for (code, reason, guidance) in cases {
            assert_eq!(DenialReason::from_code(code), Some(reason));
            assert_eq!(reason.guidance(), guidance);
            messages.push(reason.message());
        }
        messages.sort_unstable();
        messages.dedup();
        assert_eq!(messages.len(), 3, "三种拒绝原因必须是三句不同文案");
    }

    #[test]
    fn policy_denied_has_no_self_service_entry() {
        let guidance = DenialReason::PolicyDenied.guidance();
        assert_eq!(guidance, Guidance::ContactAdmin);
        assert!(!guidance.is_self_service(), "策略拒绝不设自助路径");
        assert_eq!(guidance.label(), "联系管理员");
        assert!(Guidance::Upgrade.is_self_service());
        assert!(Guidance::RequestAccess.is_self_service());
    }

    #[test]
    fn unknown_reason_code_is_not_fabricated() {
        assert_eq!(DenialReason::from_code("SOMETHING_NEW"), None);
        let flags = CapabilityFlags::from_payload(&json!({
            "can_delete_role": false,
            "deny_reasons": {"can_delete_role": "SOMETHING_NEW"}
        }));
        let eff = flags.effective(Capability::DeleteRole);
        assert!(!eff.is_allowed());
        assert_eq!(eff.denial(), None);
    }

    #[test]
    fn reasons_are_per_capability() {
        let flags = CapabilityFlags::from_payload(&json!({
            "can_edit_role": true,
            "can_delete_role": false,
            "deny_reasons": {"can_delete_role": "NO_PERMISSION"}
        }));
        assert!(flags.allows(Capability::EditRole));
        assert_eq!(
            flags.effective(Capability::DeleteRole).denial(),
            Some(DenialReason::NoPermission)
        );
    }

    #[test]
    fn contradictory_payload_fails_closed() {
        let flags = CapabilityFlags::from_payload(&json!({
            "can_delete_role": true,
            "deny_reasons": {"can_delete_role": "POLICY_DENIED"}
        }));
        let eff = flags.effective(Capability::DeleteRole);
        assert!(!eff.is_allowed(), "自相矛盾时按不可用处理");
        assert_eq!(eff.denial(), Some(DenialReason::PolicyDenied));
    }
}
