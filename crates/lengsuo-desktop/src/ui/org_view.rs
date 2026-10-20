//! 组织域视图（部门树 / 成员 / 角色 / 能力按钮）。
//!
//! 渲染语义层：只把 [`OrgListState`] 与服务端下发的 [`CapabilityFlags`]
//! 转成可直接绘制的行/节点。**不做授权推理**：
//!
//! - 按钮可用性来自服务端 `can_*` 标记（缺失即禁用）；
//! - 拒绝文案与引导来自服务端原因码，三种原因分别展示；
//! - `ai_assistant` 角色全局只读是产品不可变规则（服务端强制），
//!   界面如实反映，不依赖任何权限推理。
//!
//! 列表三态复用 [`OrgListState::should_show_empty_state`]：失败态绝不显示空态。

use crate::state::org::{ListStatus, OrgListState};
use crate::state::org_perm::{Capability, CapabilityFlags, DenialReason};
use serde_json::Value;

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

/// 组织域视图状态：三张列表 + 一份服务端能力快照。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrgViewState {
    departments: OrgListState,
    members: OrgListState,
    roles: OrgListState,
    caps: CapabilityFlags,
}

impl OrgViewState {
    #[must_use]
    pub fn new(caps: CapabilityFlags) -> Self {
        Self {
            caps,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn caps(&self) -> &CapabilityFlags {
        &self.caps
    }

    pub fn departments_mut(&mut self) -> &mut OrgListState {
        &mut self.departments
    }

    pub fn members_mut(&mut self) -> &mut OrgListState {
        &mut self.members
    }

    pub fn roles_mut(&mut self) -> &mut OrgListState {
        &mut self.roles
    }

    /// 工具栏/表单按钮状态：只读服务端标记。
    #[must_use]
    pub fn action(&self, cap: Capability) -> ActionState {
        let eff = self.caps.effective(cap);
        if eff.is_allowed() {
            ActionState::Enabled
        } else {
            ActionState::Disabled {
                notice: eff.denial().map(denial_notice),
            }
        }
    }

    #[must_use]
    pub fn departments_section(&self) -> SectionView<'_> {
        section_of(&self.departments, "暂无部门")
    }

    #[must_use]
    pub fn members_section(&self) -> SectionView<'_> {
        section_of(&self.members, "暂无成员")
    }

    #[must_use]
    pub fn roles_section(&self) -> SectionView<'_> {
        section_of(&self.roles, "暂无角色")
    }

    /// 部门树：按服务端顺序先序展开，`depth` 表示层级。
    ///
    /// 父节点缺失、自引用或环上的节点**不静默丢弃**：作为根节点展示。
    #[must_use]
    pub fn department_rows(&self) -> Vec<DepartmentNode> {
        let items = self.departments.items();
        let ids: Vec<String> = items
            .iter()
            .map(|v| id_of(v, &["id", "department_id"]))
            .collect();
        let parents: Vec<Option<String>> = items
            .iter()
            .map(|v| field_str(v, &["parent_id", "parent_department_id"]).filter(|p| !p.is_empty()))
            .collect();

        let known: std::collections::BTreeSet<&str> = ids
            .iter()
            .enumerate()
            .filter(|(_, id)| !id.is_empty())
            .map(|(_, id)| id.as_str())
            .collect();
        let child_of = |parent: &Option<String>, child: &str| {
            parent
                .as_deref()
                .is_some_and(|p| p != child && known.contains(p))
        };

        let mut visited = vec![false; items.len()];
        let mut out = Vec::new();
        for idx in 0..items.len() {
            if !visited[idx] && !child_of(&parents[idx], &ids[idx]) {
                push_subtree(items, &ids, &parents, idx, 0, &mut visited, &mut out);
            }
        }
        // 环（无根可达）也要展示，避免数据故障被伪装成「没有部门」。
        for idx in 0..items.len() {
            if !visited[idx] {
                push_subtree(items, &ids, &parents, idx, 0, &mut visited, &mut out);
            }
        }
        out
    }

    /// 成员行。
    #[must_use]
    pub fn member_rows(&self) -> Vec<MemberRow> {
        self.members
            .items()
            .iter()
            .map(|v| MemberRow {
                id: id_of(v, &["id", "user_id"]),
                name: field_str(v, &["name", "display_name"]),
                email: field_str(v, &["email"]),
            })
            .collect()
    }

    /// 角色行：展示三字段，并按服务端标记 + 只读规则给出按钮状态。
    #[must_use]
    pub fn role_rows(&self) -> Vec<RoleRow> {
        self.roles
            .items()
            .iter()
            .map(|v| {
                let preset_key = field_str(v, &["preset_key"]);
                let is_ai_assistant = v.get("is_ai_assistant").and_then(Value::as_bool)
                    == Some(true)
                    || preset_key.as_deref() == Some("ai_assistant");
                let read_only = is_ai_assistant;
                RoleRow {
                    id: id_of(v, &["id", "role_id"]),
                    name: field_str(v, &["name"]),
                    preset_key,
                    is_builtin: v.get("is_builtin").and_then(Value::as_bool) == Some(true),
                    is_ai_assistant,
                    read_only,
                    edit: row_action(v, "can_edit", Capability::EditRole, self, read_only),
                    delete: row_action(v, "can_delete", Capability::DeleteRole, self, read_only),
                }
            })
            .collect()
    }
}

const READ_ONLY_NOTICE: DenialNotice = DenialNotice {
    message: "该角色为系统内置 AI 助手，全局只读",
    hint: Some("如需调整，请联系企业管理员"),
    action: None,
};

fn section_of<'a>(list: &'a OrgListState, empty_text: &'static str) -> SectionView<'a> {
    match list.status() {
        ListStatus::Loading => SectionView::Loading,
        ListStatus::Failed => SectionView::Failed {
            message: list.last_error().unwrap_or("加载失败"),
        },
        ListStatus::Ready if list.should_show_empty_state() => {
            SectionView::Empty { text: empty_text }
        }
        ListStatus::Ready => SectionView::Ready,
    }
}

/// 行内按钮：AI 助手角色优先只读；否则用行内服务端标记，
/// 行内缺失时回退到全局快照。两处都是服务端结果，**不做 AND 推理**。
fn row_action(
    item: &Value,
    key: &str,
    global: Capability,
    view: &OrgViewState,
    read_only: bool,
) -> ActionState {
    if read_only {
        return ActionState::Disabled {
            notice: Some(READ_ONLY_NOTICE),
        };
    }
    match item.get(key).and_then(Value::as_bool) {
        Some(true) => ActionState::Enabled,
        Some(false) => {
            let reason = item
                .get("deny_reasons")
                .and_then(|m| m.get(key))
                .and_then(Value::as_str)
                .and_then(DenialReason::from_code);
            ActionState::Disabled {
                notice: reason.map(denial_notice),
            }
        }
        // 行内没有标记 → 用服务端下发的全局结果；全局也缺失则 fail-closed。
        None => view.action(global),
    }
}

fn push_subtree(
    items: &[Value],
    ids: &[String],
    parents: &[Option<String>],
    start: usize,
    depth: usize,
    visited: &mut [bool],
    out: &mut Vec<DepartmentNode>,
) {
    if visited[start] {
        return;
    }
    visited[start] = true;
    let id = &ids[start];
    let has_children = (0..items.len())
        .any(|i| parents[i].as_deref() == Some(id.as_str()) && !id.is_empty() && i != start);
    out.push(DepartmentNode {
        id: id.clone(),
        name: field_str(&items[start], &["name", "title"]),
        depth,
        has_children,
    });
    for (i, parent) in parents.iter().enumerate() {
        if parent.as_deref() == Some(id.as_str()) && !id.is_empty() && i != start {
            push_subtree(items, ids, parents, i, depth + 1, visited, out);
        }
    }
}

const ID_ALIASES: [&str; 4] = ["id", "user_id", "role_id", "department_id"];

fn id_of(v: &Value, keys: &[&str]) -> String {
    keys.iter()
        .chain(ID_ALIASES.iter())
        .filter_map(|k| v.get(*k))
        .find_map(|val| match val {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .unwrap_or_default()
}

fn field_str(v: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| v.get(*k))
        .find_map(Value::as_str)
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        assert_eq!(
            v.roles_section(),
            SectionView::Empty {
                text: "暂无角色"
            }
        );
        assert_eq!(
            v.departments_section(),
            SectionView::Empty {
                text: "暂无部门"
            }
        );
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
}
