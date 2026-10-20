//! 组织域视图状态（对标端企业管理语义：部门 / 成员 / 角色）。
//!
//! 边界（对齐本地对照资料 `server-contract-proposal-org.md` 与 `-permission.md`）：
//! - **界面不判定权限**：本层只承载「服务端下发的可见性标记」，不做授权推理。
//!   服务端权威判定见权限域 PC-07 五项校验；界面禁用 ≠ 授权。
//! - **不缓存权威数据**：列表数据由打包时引入的客户端注入，本层只维护视图态。

use serde_json::Value;

/// 列表三态：查询失败、真实空集、就绪 —— 三者必须可区分，不可混为「空」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListStatus {
    /// 正在拉取（或尚未拉取过）。
    Loading,
    /// 就绪（可能为空集）。
    Ready,
    /// 查询失败（与「空集」语义不同）。
    Failed,
}

/// 组织域列表的通用视图状态：成员 / 角色 / 部门共用同一形态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OrgListState {
    items: Vec<Value>,
    loading: bool,
    last_error: Option<String>,
    selected: Option<String>,
}

impl OrgListState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn items(&self) -> &[Value] {
        &self.items
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    #[must_use]
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// 标记开始加载，不清空已有数据（避免界面闪空）。
    pub fn begin_load(&mut self) {
        self.loading = true;
    }

    /// 注入权威结果。空 Vec 是**合法的真实空集**，不是失败。
    pub fn apply(&mut self, items: Vec<Value>) {
        self.loading = false;
        self.last_error = None;
        self.items = items;
        if let Some(sel) = &self.selected {
            if !self
                .items
                .iter()
                .any(|i| value_id(i).as_deref() == Some(sel.as_str()))
            {
                self.selected = None;
            }
        }
    }

    /// 记录失败：清空数据并保持与空集可区分。
    pub fn fail(&mut self, message: impl Into<String>) {
        self.loading = false;
        self.last_error = Some(message.into());
        self.items.clear();
        self.selected = None;
    }

    /// 选中某条；不存在则不改变选中态。
    pub fn select(&mut self, id: &str) {
        if self
            .items
            .iter()
            .any(|i| value_id(i).as_deref() == Some(id))
        {
            self.selected = Some(id.to_owned());
        }
    }

    #[must_use]
    pub fn status(&self) -> ListStatus {
        if self.loading {
            ListStatus::Loading
        } else if self.last_error.is_some() {
            ListStatus::Failed
        } else {
            ListStatus::Ready
        }
    }

    /// 界面是否应展示「空态」：仅就绪且真实为零条时成立。
    /// 失败与加载中**不得**显示空态（否则会把故障伪装成「没有数据」）。
    #[must_use]
    pub fn should_show_empty_state(&self) -> bool {
        self.status() == ListStatus::Ready && self.items.is_empty()
    }
}

fn value_id(v: &Value) -> Option<String> {
    ["id", "user_id", "role_id", "department_id"]
        .iter()
        .filter_map(|k| v.get(*k))
        .find_map(|val| match val {
            Value::String(s) if !s.is_empty() => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_result_is_ready_not_failed() {
        let mut s = OrgListState::new();
        s.apply(vec![]);
        assert_eq!(s.status(), ListStatus::Ready);
        assert!(s.should_show_empty_state());
    }

    #[test]
    fn failure_must_not_look_like_empty_state() {
        let mut s = OrgListState::new();
        s.fail("permission denied");
        assert_eq!(s.status(), ListStatus::Failed);
        assert!(
            !s.should_show_empty_state(),
            "失败态不得显示空态，否则故障会被伪装成没有数据"
        );
    }

    #[test]
    fn loading_must_not_show_empty_state() {
        let mut s = OrgListState::new();
        s.begin_load();
        assert_eq!(s.status(), ListStatus::Loading);
        assert!(!s.should_show_empty_state());
    }

    #[test]
    fn selection_cleared_when_item_disappears() {
        let mut s = OrgListState::new();
        s.apply(vec![json!({"user_id": "u1"})]);
        s.select("u1");
        assert_eq!(s.selected(), Some("u1"));
        s.apply(vec![json!({"user_id": "u2"})]);
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn selecting_unknown_id_is_a_noop() {
        let mut s = OrgListState::new();
        s.apply(vec![json!({"id": "r1"})]);
        s.select("absent");
        assert_eq!(s.selected(), None);
    }

    #[test]
    fn id_lookup_supports_org_domain_aliases() {
        assert_eq!(value_id(&json!({"role_id": "r1"})).unwrap(), "r1");
        assert_eq!(value_id(&json!({"department_id": "d1"})).unwrap(), "d1");
        assert_eq!(value_id(&json!({"user_id": 42})).unwrap(), "42");
    }
}
