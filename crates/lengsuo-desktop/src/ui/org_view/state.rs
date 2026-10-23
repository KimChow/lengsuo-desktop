//! OrgViewState：三张列表 + 一份服务端能力快照。

use super::rows::{field_str, id_of, push_subtree, row_action, section_of};
use super::{denial_notice, ActionState, DepartmentNode, MemberRow, RoleRow, SectionView};
use crate::state::org::OrgListState;
use crate::state::org_perm::{Capability, CapabilityFlags};
use serde_json::Value;

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
