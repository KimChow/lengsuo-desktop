//! 「账号」面板的描述层：把组织域列表态转成面板要渲染的描述。
//!
//! **只做界面**：只读消费 [`OrgListState`]，不实现成员管理、不做授权推理。
//!
//! 硬约束：
//! - **三态严格区分**：加载中 / 加载失败 / 真实空集 / 有内容，
//!   失败**不得**退化成空列表（fail-closed）。
//! - **不拿 id 冒充名字**：成员缺名字时用占位文案（与 [`super::data`] 的
//!   `GROUP_NAME_PLACEHOLDER` 同一条铁律）。
//!
//! 分层约定同 [`super::data`]：接线层只映射这里的判定结果。

use serde_json::Value;

use crate::state::org::{ListStatus, OrgListState};

/// 成员没有展示名时的占位文案（**不拿 id 冒充名字**）。
pub const ACCOUNT_NAME_PLACEHOLDER: &str = "未命名成员";

/// 成员 id 缺失时的占位文案（**不拿空串冒充 id**）。
pub const ACCOUNT_ID_PLACEHOLDER: &str = "（无 id）";

/// 「账号」面板的正文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountPanelBody {
    /// 正在加载。
    Loading,
    /// 加载失败（fail-closed：必须显示错误，**不得**退化成空列表）。
    Failed(String),
    /// 真实空集（服务端下发零条 —— 与「加载失败」是两件事）。
    Empty,
    /// 有内容：逐条列出成员的展示名与 id。
    Content(Vec<AccountRow>),
}

/// 一行成员。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountRow {
    /// 展示名（缺失时用 [`ACCOUNT_NAME_PLACEHOLDER`]）。
    pub name: String,
    /// 成员 id（缺失时用 [`ACCOUNT_ID_PLACEHOLDER`]）。
    pub id: String,
}

/// 从组织域列表态算出面板正文（fail-closed：失败优先于空集判定）。
#[must_use]
pub fn account_body(state: &OrgListState) -> AccountPanelBody {
    match state.status() {
        ListStatus::Loading => AccountPanelBody::Loading,
        ListStatus::Failed => {
            AccountPanelBody::Failed(state.last_error().unwrap_or("成员加载失败").to_owned())
        }
        ListStatus::Ready => {
            if state.is_empty() {
                AccountPanelBody::Empty
            } else {
                AccountPanelBody::Content(state.items().iter().map(account_row).collect())
            }
        }
    }
}

/// 把一条原始 JSON 变成一行成员（字段名按服务端下发，取不到就退到占位）。
fn account_row(item: &Value) -> AccountRow {
    let id = value_id(item).unwrap_or_else(|| ACCOUNT_ID_PLACEHOLDER.to_owned());
    let name = ["name", "display_name"]
        .iter()
        .filter_map(|key| item.get(*key))
        .find_map(|value| value.as_str())
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map_or_else(|| ACCOUNT_NAME_PLACEHOLDER.to_owned(), ToOwned::to_owned);
    AccountRow { name, id }
}

/// 取成员 id（与 `state::org` 里的 `value_id` 同形状，只取成员域的键）。
fn value_id(item: &Value) -> Option<String> {
    ["id", "user_id"]
        .iter()
        .filter_map(|key| item.get(*key))
        .find_map(|value| match value {
            Value::String(text) if !text.is_empty() => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    // 显式导入符号（不用 `use super::*`）：与仓内约定保持一致。
    use super::{account_body, AccountPanelBody, ACCOUNT_ID_PLACEHOLDER, ACCOUNT_NAME_PLACEHOLDER};
    use crate::state::org::OrgListState;
    use serde_json::json;

    #[test]
    fn empty_list_is_a_real_empty_set_not_an_error() {
        let mut state = OrgListState::new();
        state.apply(vec![]); // 权威结果：零条 ⇒ 才是「真实为空」
        assert_eq!(account_body(&state), AccountPanelBody::Empty);
    }

    #[test]
    fn a_never_loaded_list_is_loading_not_empty() {
        let state = OrgListState::new();
        assert!(
            matches!(account_body(&state), AccountPanelBody::Loading),
            "从未加载的成员清单必须显示「正在加载」，不得显示「暂无成员」"
        );
    }

    #[test]
    fn failure_is_not_reported_as_empty() {
        let mut state = OrgListState::new();
        state.fail("服务端不可用");
        assert!(matches!(account_body(&state), AccountPanelBody::Failed(_)));
        assert!(
            !matches!(account_body(&state), AccountPanelBody::Empty),
            "失败态不得显示成空集，否则故障会被伪装成没有数据"
        );
    }

    #[test]
    fn loading_is_not_reported_as_empty() {
        let mut state = OrgListState::new();
        state.begin_load();
        assert_eq!(account_body(&state), AccountPanelBody::Loading);
    }

    #[test]
    fn items_become_rows() {
        let mut state = OrgListState::new();
        state.apply(vec![json!({"id": "u1", "name": "张三"})]);
        match account_body(&state) {
            AccountPanelBody::Content(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].id, "u1");
                assert_eq!(rows[0].name, "张三");
            }
            other => panic!("期望有内容，实际 {other:?}"),
        }
    }

    #[test]
    fn missing_name_does_not_fall_back_to_id() {
        let mut state = OrgListState::new();
        state.apply(vec![json!({"id": "u1"})]);
        match account_body(&state) {
            AccountPanelBody::Content(rows) => {
                assert_eq!(rows[0].name, ACCOUNT_NAME_PLACEHOLDER);
                assert_eq!(rows[0].id, "u1");
            }
            other => panic!("期望有内容，实际 {other:?}"),
        }
    }

    #[test]
    fn missing_id_falls_back_to_placeholder_not_empty_string() {
        let mut state = OrgListState::new();
        state.apply(vec![json!({"name": "张三"})]);
        match account_body(&state) {
            AccountPanelBody::Content(rows) => {
                assert_eq!(rows[0].id, ACCOUNT_ID_PLACEHOLDER);
                assert_ne!(rows[0].id, "", "缺 id 时不得用空串冒充");
            }
            other => panic!("期望有内容，实际 {other:?}"),
        }
    }

    #[test]
    fn user_id_alias_is_accepted() {
        let mut state = OrgListState::new();
        state.apply(vec![json!({"user_id": "u9", "display_name": "李四"})]);
        match account_body(&state) {
            AccountPanelBody::Content(rows) => {
                assert_eq!(rows[0].id, "u9");
                assert_eq!(rows[0].name, "李四");
            }
            other => panic!("期望有内容，实际 {other:?}"),
        }
    }
}
