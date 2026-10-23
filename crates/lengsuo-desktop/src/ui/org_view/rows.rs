//! 行/节点构建辅助：section_of、row_action、push_subtree、id_of、field_str。

use super::{denial_notice, ActionState, DenialNotice, DepartmentNode, OrgViewState, SectionView};
use crate::state::org::{ListStatus, OrgListState};
use crate::state::org_perm::{Capability, DenialReason};
use serde_json::Value;

pub(super) const READ_ONLY_NOTICE: DenialNotice = DenialNotice {
    message: "该角色为系统内置 AI 助手，全局只读",
    hint: Some("如需调整，请联系企业管理员"),
    action: None,
};

pub(super) const ID_ALIASES: [&str; 4] = ["id", "user_id", "role_id", "department_id"];

/// 列表区域的展示态。失败与空集必须分开。
pub(super) fn section_of<'a>(list: &'a OrgListState, empty_text: &'static str) -> SectionView<'a> {
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
pub(super) fn row_action(
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

pub(super) fn push_subtree(
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

pub(super) fn id_of(v: &Value, keys: &[&str]) -> String {
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

pub(super) fn field_str(v: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|k| v.get(*k))
        .find_map(Value::as_str)
        .map(str::to_owned)
}
