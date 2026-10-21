//! 权限展示的**纯逻辑**判定（描述层，无 GPUI 依赖，可无窗口测）。
//!
//! ⛔ 只做「服务端结果 → 展示文案」的映射，**不做任何授权推理**。
//!
//! 与「安全中心」（`view::capability_rows`）的分工：
//! - 安全中心 = 总览：9 个能力位逐个给「可用 / 不可用 / 未下发」。
//! - 本模块 = 原因细分：「系统权限」面板把「不可用」再拆成三种拒绝原因
//!   （套餐不含 / 角色无权限 / 企业策略禁止）+ 各自的引导路径。

use crate::state::org_perm::{Capability, CapabilityFlags, DenialReason};

/// 单个能力在界面上的展示结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDisplay {
    /// 服务端允许。
    Allowed,
    /// 服务端拒绝，且给了原因。
    Denied { reason: DenialReason },
    /// 服务端拒绝但**没给原因**（不得编造）。
    DeniedWithoutReason,
    /// 服务端**没下发**这个标记（与「拒绝」是不同的事）。
    NotProvided,
}

/// 判定单个能力该怎么展示。
///
/// 只允许用 `flags.has_mark(cap)` + `flags.effective(cap).denial()`，
/// ⛔ 不新增任何推理（不按角色 / 套餐 / 身份重新推导授权）。
#[must_use]
pub fn permission_display(flags: &CapabilityFlags, cap: Capability) -> PermissionDisplay {
    if !flags.has_mark(cap) {
        return PermissionDisplay::NotProvided;
    }
    let effective = flags.effective(cap);
    if effective.is_allowed() {
        PermissionDisplay::Allowed
    } else {
        match effective.denial() {
            Some(reason) => PermissionDisplay::Denied { reason },
            None => PermissionDisplay::DeniedWithoutReason,
        }
    }
}

#[cfg(test)]
mod tests {
    // 显式导入符号（不用 `use super::*`）：与仓内约定保持一致，避免遮蔽风险。
    use crate::state::org_perm::{Capability, CapabilityFlags, DenialReason};
    use crate::ui::settings_gpui::view::permissions::{permission_display, PermissionDisplay};
    use serde_json::json;

    #[test]
    fn explicit_true_without_reason_is_allowed() {
        let flags = CapabilityFlags::from_payload(&json!({"can_view_members": true}));
        assert_eq!(
            permission_display(&flags, Capability::ViewMembers),
            PermissionDisplay::Allowed,
            "服务端明确允许 ⇒ Allowed"
        );
    }

    #[test]
    fn each_denial_reason_has_its_own_denied_display() {
        // 三种拒绝原因不可合并：必须各落进对应的 `Denied { reason }`。
        let entitlement = CapabilityFlags::from_payload(&json!({
            "can_invite_member": false,
            "deny_reasons": {"can_invite_member": "NO_ENTITLEMENT"}
        }));
        assert_eq!(
            permission_display(&entitlement, Capability::InviteMember),
            PermissionDisplay::Denied {
                reason: DenialReason::NoEntitlement
            }
        );

        let permission = CapabilityFlags::from_payload(&json!({
            "can_edit_member": false,
            "deny_reasons": {"can_edit_member": "NO_PERMISSION"}
        }));
        assert_eq!(
            permission_display(&permission, Capability::EditMember),
            PermissionDisplay::Denied {
                reason: DenialReason::NoPermission
            }
        );

        let policy = CapabilityFlags::from_payload(&json!({
            "can_remove_member": false,
            "deny_reasons": {"can_remove_member": "POLICY_DENIED"}
        }));
        assert_eq!(
            permission_display(&policy, Capability::RemoveMember),
            PermissionDisplay::Denied {
                reason: DenialReason::PolicyDenied
            }
        );
    }

    #[test]
    fn denied_without_reason_is_not_denied_nor_not_provided() {
        // 服务端只给了 `false` 没给原因 ⇒ 不编造成 `Denied`，也不是「未下发」。
        let flags = CapabilityFlags::from_payload(&json!({"can_create_role": false}));
        assert_eq!(
            permission_display(&flags, Capability::CreateRole),
            PermissionDisplay::DeniedWithoutReason
        );
    }

    #[test]
    fn missing_mark_is_not_provided_not_denied() {
        // 缺键（未下发）⇒ `NotProvided`，**不是** `Denied`。
        let flags = CapabilityFlags::from_payload(&json!({}));
        assert_eq!(
            permission_display(&flags, Capability::DeleteRole),
            PermissionDisplay::NotProvided
        );
    }

    #[test]
    fn contradictory_payload_fails_closed_to_denied_with_reason() {
        // 标记与原因码矛盾 ⇒ 按不可用处理，且保留服务端给的原因（不当作 Allowed）。
        let flags = CapabilityFlags::from_payload(&json!({
            "can_delete_role": true,
            "deny_reasons": {"can_delete_role": "POLICY_DENIED"}
        }));
        assert_eq!(
            permission_display(&flags, Capability::DeleteRole),
            PermissionDisplay::Denied {
                reason: DenialReason::PolicyDenied
            }
        );
    }
}
