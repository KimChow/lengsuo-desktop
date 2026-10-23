//! `ui::org_view` 的单测（原 `org_view.rs` 第 352–568 行，逻辑一字未改）。
//! 显式 `use` 符号，不用 `super::*`。

use super::rows::READ_ONLY_NOTICE;
use super::{denial_notice, ActionState, OrgViewState, SectionView};
use crate::state::org_perm::{Capability, CapabilityFlags, DenialReason};
use serde_json::{json, Value};

fn view(payload: Value) -> OrgViewState {
    OrgViewState::new(CapabilityFlags::from_payload(&payload))
}

#[test]
fn missing_capability_disables_button_without_inventing_reason() {
    let v = view(json!({}));
    assert_eq!(
        v.action(Capability::DeleteRole),
        ActionState::Disabled { notice: None }
    );
    assert!(!v.action(Capability::DeleteRole).is_enabled());
}

#[test]
fn button_follows_server_mark_not_roles() {
    let v = view(json!({"roles": ["admin"], "can_delete_role": false}));
    assert!(!v.action(Capability::DeleteRole).is_enabled());

    let v = view(json!({"roles": [], "can_delete_role": true}));
    assert!(v.action(Capability::DeleteRole).is_enabled());
}

#[test]
fn denial_notices_are_distinct_and_policy_has_no_action() {
    let ent = denial_notice(DenialReason::NoEntitlement);
    let perm = denial_notice(DenialReason::NoPermission);
    let policy = denial_notice(DenialReason::PolicyDenied);
    assert_eq!(ent.action, Some("升级购买"));
    assert_eq!(perm.action, Some("申请权限"));
    assert_eq!(policy.action, None, "策略拒绝不接申请入口");
    assert_eq!(policy.hint, Some("请联系企业管理员"));
    assert_ne!(ent.message, perm.message);
    assert_ne!(perm.message, policy.message);
}

#[test]
fn capability_reason_is_rendered_for_the_right_button() {
    let v = view(json!({
        "can_edit_role": false,
        "can_delete_role": false,
        "deny_reasons": {
            "can_edit_role": "NO_ENTITLEMENT",
            "can_delete_role": "NO_PERMISSION"
        }
    }));
    assert_eq!(
        v.action(Capability::EditRole),
        ActionState::Disabled {
            notice: Some(denial_notice(DenialReason::NoEntitlement))
        }
    );
    assert_eq!(
        v.action(Capability::DeleteRole),
        ActionState::Disabled {
            notice: Some(denial_notice(DenialReason::NoPermission))
        }
    );
}

#[test]
fn failed_list_is_not_an_empty_state() {
    let mut v = view(json!({}));
    v.members_mut().fail("network down");
    assert_eq!(
        v.members_section(),
        SectionView::Failed {
            message: "network down"
        }
    );
    v.members_mut().apply(vec![]);
    assert_eq!(
        v.members_section(),
        SectionView::Empty {
            text: "暂无成员"
        }
    );
    // 未拉取过的列表是「加载中」，不是「空集」。
    assert_eq!(v.roles_section(), SectionView::Loading);
    assert_eq!(v.departments_section(), SectionView::Loading);
}

#[test]
fn loading_list_is_not_an_empty_state() {
    let mut v = view(json!({}));
    v.roles_mut().begin_load();
    assert_eq!(v.roles_section(), SectionView::Loading);
}

#[test]
fn ai_assistant_role_is_read_only_even_when_server_allows() {
    let mut v = view(json!({"can_edit_role": true, "can_delete_role": true}));
    v.roles_mut().apply(vec![json!({
        "role_id": "r-ai",
        "name": "AI 助手",
        "preset_key": "ai_assistant",
        "is_builtin": true,
        "is_ai_assistant": true
    })]);
    let row = &v.role_rows()[0];
    assert!(row.read_only);
    assert_eq!(
        row.edit,
        ActionState::Disabled {
            notice: Some(READ_ONLY_NOTICE)
        }
    );
    assert_eq!(
        row.delete,
        ActionState::Disabled {
            notice: Some(READ_ONLY_NOTICE)
        }
    );
}

#[test]
fn ai_assistant_is_detected_by_preset_key_or_flag() {
    let mut v = view(json!({"can_edit_role": true}));
    v.roles_mut().apply(vec![
        json!({"role_id": "a", "preset_key": "ai_assistant"}),
        json!({"role_id": "b", "is_ai_assistant": true}),
        json!({"role_id": "c", "preset_key": "ops", "is_builtin": true}),
    ]);
    let rows = v.role_rows();
    assert!(rows[0].read_only);
    assert!(rows[1].read_only);
    assert!(!rows[2].read_only);
    assert!(rows[2].is_builtin);
    assert_eq!(rows[2].edit, ActionState::Enabled);
    assert_eq!(rows[2].preset_key.as_deref(), Some("ops"));
}

#[test]
fn row_level_mark_overrides_global_and_fails_closed() {
    let mut v = view(json!({"can_delete_role": true}));
    v.roles_mut().apply(vec![
        json!({"role_id": "r1", "can_delete": false,
               "deny_reasons": {"can_delete": "POLICY_DENIED"}}),
        json!({"role_id": "r2"}),
    ]);
    let rows = v.role_rows();
    assert_eq!(
        rows[0].delete,
        ActionState::Disabled {
            notice: Some(denial_notice(DenialReason::PolicyDenied))
        }
    );
    assert_eq!(
        rows[1].delete,
        ActionState::Enabled,
        "行内缺失回退服务端全局结果"
    );
}

#[test]
fn department_tree_is_preorder_with_depth() {
    let mut v = view(json!({}));
    v.departments_mut().apply(vec![
        json!({"department_id": "d1", "name": "总部"}),
        json!({"department_id": "d2", "parent_id": "d1", "name": "研发"}),
        json!({"department_id": "d3", "parent_id": "d2", "name": "前端"}),
        json!({"department_id": "d4", "parent_id": "d1", "name": "销售"}),
    ]);
    let rows = v.department_rows();
    let got: Vec<(&str, usize)> = rows.iter().map(|r| (r.id.as_str(), r.depth)).collect();
    assert_eq!(got, vec![("d1", 0), ("d2", 1), ("d3", 2), ("d4", 1)]);
    assert!(rows[0].has_children);
    assert!(!rows[2].has_children);
}

#[test]
fn orphan_and_cyclic_departments_are_still_shown() {
    let mut v = view(json!({}));
    v.departments_mut().apply(vec![
        json!({"id": "a", "parent_id": "b"}),
        json!({"id": "b", "parent_id": "a"}),
        json!({"id": "c", "parent_id": "missing"}),
        json!({"id": "d", "parent_id": "d"}),
    ]);
    let rows = v.department_rows();
    assert_eq!(rows.len(), 4, "环与孤儿不得被静默丢弃");
    // 环里必有一个节点被当作根展开；另一个作为它的子节点，深度仍有限。
    let roots = rows.iter().filter(|r| r.depth == 0).count();
    assert!(roots >= 2, "孤儿 c 与自引用 d 必须是根节点");
    assert!(rows.iter().all(|r| r.depth <= 1), "环不得导致无限递归");
    let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["a", "b", "c", "d"]);
}

#[test]
fn failed_departments_do_not_render_as_empty_tree() {
    let mut v = view(json!({}));
    v.departments_mut().fail("boom");
    assert!(v.department_rows().is_empty());
    assert_eq!(
        v.departments_section(),
        SectionView::Failed { message: "boom" }
    );
}

#[test]
fn member_rows_read_org_domain_fields() {
    let mut v = view(json!({}));
    v.members_mut().apply(vec![json!({
        "user_id": 42, "name": "张三", "email": "z@example.com"
    })]);
    let rows = v.member_rows();
    assert_eq!(rows[0].id, "42");
    assert_eq!(rows[0].name.as_deref(), Some("张三"));
    assert_eq!(rows[0].email.as_deref(), Some("z@example.com"));
}
