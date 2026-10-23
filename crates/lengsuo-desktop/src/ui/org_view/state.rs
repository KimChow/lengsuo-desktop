//! OrgViewState：三张列表 + 一份服务端能力快照。

use super::rows::{field_str, id_of, push_subtree, row_action, section_of};
use super::{denial_notice, ActionState, DepartmentNode, MemberRow, RoleRow, SectionView};
use crate::state::datasource::{LoadState, Provenance};
use crate::state::org::OrgListState;
use crate::state::org_perm::{Capability, CapabilityFlags};
use serde_json::Value;

/// C7 组织树节点（契约 §2 逐字：**组织树**）。
///
/// 键名对齐描述层既有取值键（`id` / `parent_id` / `name`），⛔ 不另造一套。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgTreeItem {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: Option<String>,
}

/// C7 成员表行（契约 §2 逐字：**成员表**）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgMemberItem {
    pub id: String,
    pub name: Option<String>,
    pub email: Option<String>,
}

/// C7 角色表行（契约 §2 逐字：**角色表**）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgRoleItem {
    pub id: String,
    pub name: Option<String>,
    pub preset_key: Option<String>,
    pub is_builtin: bool,
    pub is_ai_assistant: bool,
}

/// C7 组织域消费侧快照（契约 §2 逐字：组织树 / 成员表 / 角色表 + **能力位**）。
///
/// 生产侧线：服务端（未接通）。本类型只是消费侧形态，⛔ 不实现运行时语义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgSnapshot {
    pub org_tree: Vec<OrgTreeItem>,
    pub members: Vec<OrgMemberItem>,
    pub roles: Vec<OrgRoleItem>,
    pub capabilities: CapabilityFlags,
}

/// 快照行 → 描述层既有 `Value` 行形态；`None` 字段**省略键**
/// （与既有演示数据的缺省形态一致，不落显式 null）。
fn obj_row(pairs: Vec<(&'static str, Option<&str>)>, flags: Vec<(&'static str, bool)>) -> Value {
    let mut obj = serde_json::Map::new();
    for (key, value) in pairs {
        if let Some(value) = value {
            obj.insert(key.into(), Value::String(value.to_owned()));
        }
    }
    for (key, value) in flags {
        obj.insert(key.into(), Value::Bool(value));
    }
    Value::Object(obj)
}

impl OrgSnapshot {
    /// 演示快照：与既有演示出口**同一份**演示数据；能力位为空标记
    /// （服务端未下发 ⇒ 界面如实显示不可用，⛔ 不伪造能力位）。
    #[must_use]
    pub fn demo() -> Self {
        Self {
            org_tree: vec![
                OrgTreeItem {
                    id: "d1".into(),
                    parent_id: None,
                    name: Some("研发一部".into()),
                },
                OrgTreeItem {
                    id: "d2".into(),
                    parent_id: None,
                    name: Some("华南组".into()),
                },
            ],
            members: vec![
                OrgMemberItem {
                    id: "u1".into(),
                    name: Some("张三".into()),
                    email: Some("zhang@example.com".into()),
                },
                OrgMemberItem {
                    id: "u2".into(),
                    name: Some("李四".into()),
                    email: Some("li@example.com".into()),
                },
            ],
            roles: vec![
                OrgRoleItem {
                    id: "r1".into(),
                    name: Some("管理员".into()),
                    preset_key: Some("admin".into()),
                    is_builtin: true,
                    is_ai_assistant: false,
                },
                OrgRoleItem {
                    id: "r2".into(),
                    name: Some("AI 助手".into()),
                    preset_key: Some("ai_assistant".into()),
                    is_builtin: false,
                    is_ai_assistant: true,
                },
            ],
            capabilities: CapabilityFlags::default(),
        }
    }

    /// 三张列表各自的行数据（顺序 = 快照顺序 = 服务端顺序）。
    fn values(&self) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
        let tree = self
            .org_tree
            .iter()
            .map(|n| {
                obj_row(
                    vec![
                        ("id", Some(n.id.as_str())),
                        ("parent_id", n.parent_id.as_deref()),
                        ("name", n.name.as_deref()),
                    ],
                    Vec::new(),
                )
            })
            .collect();
        let members = self
            .members
            .iter()
            .map(|m| {
                obj_row(
                    vec![
                        ("id", Some(m.id.as_str())),
                        ("name", m.name.as_deref()),
                        ("email", m.email.as_deref()),
                    ],
                    Vec::new(),
                )
            })
            .collect();
        let roles = self
            .roles
            .iter()
            .map(|r| {
                obj_row(
                    vec![
                        ("id", Some(r.id.as_str())),
                        ("name", r.name.as_deref()),
                        ("preset_key", r.preset_key.as_deref()),
                    ],
                    vec![
                        ("is_builtin", r.is_builtin),
                        ("is_ai_assistant", r.is_ai_assistant),
                    ],
                )
            })
            .collect();
        (tree, members, roles)
    }
}

/// 组织域视图状态：三张列表 + 一份服务端能力快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgViewState {
    departments: OrgListState,
    members: OrgListState,
    roles: OrgListState,
    caps: CapabilityFlags,
    /// 数据来源标注：契约未就绪 ⇒ 恒为 [`Provenance::Demo`]（⛔ 不许标 Runtime）。
    provenance: Provenance,
}

impl Default for OrgViewState {
    /// 默认态：三张列表「尚未拉取」（Loading）+ 空能力位 + Demo 标注。
    fn default() -> Self {
        Self {
            departments: OrgListState::new(),
            members: OrgListState::new(),
            roles: OrgListState::new(),
            caps: CapabilityFlags::default(),
            provenance: Provenance::Demo,
        }
    }
}

impl OrgViewState {
    #[must_use]
    pub fn new(caps: CapabilityFlags) -> Self {
        Self {
            caps,
            ..Self::default()
        }
    }

    /// 数据来源标注（界面必须如实显示；契约未就绪 ⇒ Demo）。
    #[must_use]
    pub fn provenance(&self) -> Provenance {
        self.provenance
    }

    /// C7 注入入口：把消费侧契约三态落进视图状态（**只换来源，不改结构**）。
    ///
    /// - `Ready`：三张列表就绪，能力位取自快照；
    /// - `Loading`：三张列表保持「尚未拉取」；
    /// - `Failed`：三张列表带原因失败（fail-closed，⛔ 不退化成空列表、不伪造数据）。
    ///
    /// 来源恒标 `Demo`：契约未就绪，⛔ 不许标 `Runtime`。
    #[must_use]
    pub fn from_snapshot(load: LoadState<OrgSnapshot>) -> Self {
        let mut state = Self::default();
        match load {
            LoadState::Ready(snapshot) => {
                let (tree, members, roles) = snapshot.values();
                state.departments.apply(tree);
                state.members.apply(members);
                state.roles.apply(roles);
                state.caps = snapshot.capabilities;
            }
            LoadState::Failed(reason) => {
                state.departments.fail(reason.clone());
                state.members.fail(reason.clone());
                state.roles.fail(reason);
            }
            LoadState::Loading => {}
        }
        state
    }

    /// 演示出口：[`Self::from_snapshot`] 的薄封装（同一份演示快照）。
    #[must_use]
    pub fn demo() -> Self {
        Self::from_snapshot(LoadState::Ready(OrgSnapshot::demo()))
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

#[cfg(test)]
mod tests {
    use crate::state::datasource::{LoadState, Provenance};
    use crate::state::org::ListStatus;
    use crate::state::org_perm::{Capability, CapabilityFlags};
    use crate::ui::org_view::ActionState;
    use serde_json::json;

    use super::{OrgSnapshot, OrgTreeItem, OrgViewState};

    const FAIL_REASON: &str = "C7 组织域数据未接通（演示态）";

    /// ① 注入快照 ⇒ 部门树节点数 == 快照（嵌套子树不丢节点）。
    #[test]
    fn ready_snapshot_fills_lists_with_same_counts() {
        let snapshot = OrgSnapshot {
            org_tree: vec![
                OrgTreeItem {
                    id: "d1".into(),
                    parent_id: None,
                    name: Some("研发一部".into()),
                },
                OrgTreeItem {
                    id: "d2".into(),
                    parent_id: Some("d1".into()),
                    name: Some("华南组".into()),
                },
                OrgTreeItem {
                    id: "d3".into(),
                    parent_id: Some("d2".into()),
                    name: None,
                },
            ],
            ..OrgSnapshot::demo()
        };
        let mut state = OrgViewState::from_snapshot(LoadState::Ready(snapshot));
        assert_eq!(
            state.department_rows().len(),
            3,
            "部门树节点数必须等于快照节点数"
        );
        assert_eq!(state.member_rows().len(), 2);
        assert_eq!(state.role_rows().len(), 2);
        assert_eq!(state.departments_mut().status(), ListStatus::Ready);
    }

    /// ③ Failed ⇒ 三张列表带原因失败（错误文案可显示），不 panic、不伪装成空列表。
    #[test]
    fn failed_snapshot_keeps_error_not_empty_list() {
        let mut state = OrgViewState::from_snapshot(LoadState::Failed(FAIL_REASON.to_owned()));
        assert_eq!(state.departments_mut().last_error(), Some(FAIL_REASON));
        assert_eq!(state.members_mut().last_error(), Some(FAIL_REASON));
        assert_eq!(state.roles_mut().last_error(), Some(FAIL_REASON));
        assert_eq!(state.departments_mut().status(), ListStatus::Failed);
        assert!(
            !state.departments_mut().should_show_empty_state(),
            "失败态不得退化成空列表"
        );
        assert!(state.department_rows().is_empty(), "失败态不得伪造数据");
    }

    /// 三态之 Loading：保持「尚未拉取」，不显示空态。
    #[test]
    fn loading_snapshot_stays_loading() {
        let mut state = OrgViewState::from_snapshot(LoadState::Loading);
        assert_eq!(state.departments_mut().status(), ListStatus::Loading);
        assert_eq!(state.members_mut().status(), ListStatus::Loading);
        assert_eq!(state.roles_mut().status(), ListStatus::Loading);
    }

    /// ② 能力位可见性按快照渲染：快照下发的能力 ⇒ Enabled；未下发 ⇒ Disabled。
    #[test]
    fn capability_visibility_follows_snapshot() {
        let allowed = OrgSnapshot {
            capabilities: CapabilityFlags::from_payload(&json!({"can_invite_member": true})),
            ..OrgSnapshot::demo()
        };
        let state = OrgViewState::from_snapshot(LoadState::Ready(allowed));
        assert!(matches!(
            state.action(Capability::InviteMember),
            ActionState::Enabled
        ));
        assert!(
            matches!(
                state.action(Capability::ManageDepartment),
                ActionState::Disabled { .. }
            ),
            "快照未下发的能力必须禁用"
        );

        let empty = OrgViewState::from_snapshot(LoadState::Ready(OrgSnapshot::demo()));
        assert!(matches!(
            empty.action(Capability::InviteMember),
            ActionState::Disabled { .. }
        ));
        // 角色行按钮同样由快照能力位支配；ai_assistant 行只读。
        assert!(empty
            .role_rows()
            .iter()
            .all(|r| matches!(r.edit, ActionState::Disabled { .. })));
        assert!(empty.role_rows()[1].read_only);
    }

    /// demo 出口是 from_snapshot 的薄封装：两者产出完全一致。
    #[test]
    fn demo_outlet_is_a_thin_wrapper_of_from_snapshot() {
        assert_eq!(
            OrgViewState::demo(),
            OrgViewState::from_snapshot(LoadState::Ready(OrgSnapshot::demo()))
        );
    }

    /// 来源标注：契约未就绪 ⇒ 三种注入结果一律 Demo，不许标 Runtime。
    #[test]
    fn provenance_stays_demo_until_contract_lands() {
        let states = [
            OrgViewState::from_snapshot(LoadState::Loading),
            OrgViewState::from_snapshot(LoadState::Failed(FAIL_REASON.to_owned())),
            OrgViewState::demo(),
        ];
        for state in &states {
            assert_eq!(state.provenance(), Provenance::Demo);
        }
    }
}
