//! 「账号」面板的描述层：把组织域列表态转成面板要渲染的描述。
//!
//! **只做界面**：只读消费 [`OrgListState`]，不实现成员管理、不做授权推理。
//!
//! C8（预备）：另落「账号档案 / 订阅用量 / 能力位」消费侧快照（[`AccountSnapshot`]），
//! 经 [`AccountPanelBody::from_account`] 从消费侧契约出口取数；契约未就绪 ⇒
//! 界面保持演示 + 如实标注（⛔ 不伪造服务端数据、⛔ 不把 Demo 标成 Runtime）。
//!
//! 硬约束：
//! - **三态严格区分**：加载中 / 加载失败 / 真实空集 / 有内容，
//!   失败**不得**退化成空列表（fail-closed）。
//! - **不拿 id 冒充名字**：成员缺名字时用占位文案（与 [`super::data`] 的
//!   `GROUP_NAME_PLACEHOLDER` 同一条铁律）。
//!
//! 分层约定同 [`super::data`]：接线层只映射这里的判定结果。

use serde_json::Value;

use crate::state::datasource::LoadState;
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
    /// C8 快照就绪：账号档案 / 订阅用量 / 能力位逐行（行结构与 [`AccountRow`] 同形）。
    AccountContent(Vec<AccountLine>),
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

// ---------------------------------------------------------------------------
// C8 消费侧快照（账号档案 / 订阅配额用量 / 安全能力位）。
// 只描述界面怎么消费：⛔ 不实现运行时语义、⛔ 不伪造服务端数据。
// ---------------------------------------------------------------------------

/// C8 账号档案：展示名 + 用户 ID。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountProfile {
    /// 账号展示名（缺失时渲染占位，不拿空串冒充）。
    pub name: String,
    /// 用户 ID。
    pub user_id: String,
}

/// C8 订阅用量：配额 + 已用 + 单位（数字只来自下发快照，界面不造数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionUsage {
    /// 配额（总量）。
    pub quota: u64,
    /// 已用。
    pub used: u64,
    /// 单位（如「积分」）。
    pub unit: String,
}

/// C8 能力位三态：可用 / 不可用 / **未下发**。
///
/// 「未下发」是安全能力标记的真实现状（服务端没发），如实显示是**正确行为**；
/// ⛔ 不许编成可用 / 不可用，更不许显示成空或 0。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityMark {
    /// 可用（服务端下发了能力标记且为开）。
    Available,
    /// 不可用（服务端下发了能力标记且为关）。
    Unavailable,
    /// 未下发（服务端没下发能力标记 —— 真实现状，非错误）。
    NotIssued,
}

impl CapabilityMark {
    /// 三态的面板文案。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => "可用",
            Self::Unavailable => "不可用",
            Self::NotIssued => "未下发",
        }
    }
}

/// C8 一条安全能力标记：能力名 + 三态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityBit {
    /// 能力名（来自下发，不编造）。
    pub name: String,
    /// 三态标记。
    pub mark: CapabilityMark,
}

/// C8 账号面板快照：档案 + 订阅用量 + 能力位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSnapshot {
    /// 账号档案。
    pub profile: AccountProfile,
    /// 订阅配额用量。
    pub usage: SubscriptionUsage,
    /// 安全能力标记（未下发 ⇒ 逐条 [`CapabilityMark::NotIssued`]，如实显示）。
    pub capabilities: Vec<CapabilityBit>,
}

/// C8 快照行：主文案（账号名 / 标签 / 能力名）+ 次文案（id / 数值 / 三态）。
///
/// 行结构与 [`AccountRow`] 同形（接线层共用同一行区块渲染）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountLine {
    /// 主文案。
    pub primary: String,
    /// 次文案。
    pub secondary: String,
}

/// 用量行主文案（account.credits.monthlyQuota）。
const USAGE_LABEL: &str = "月度限额";

impl AccountPanelBody {
    /// C8 消费侧快照态 → 面板正文（fail-closed：`Failed` 直通，不退化成空）。
    #[must_use]
    pub fn from_account(state: LoadState<AccountSnapshot>) -> Self {
        match state {
            LoadState::Loading => Self::Loading,
            LoadState::Failed(reason) => Self::Failed(reason),
            LoadState::Ready(snapshot) => snapshot.into(),
        }
    }
}

impl From<AccountSnapshot> for AccountPanelBody {
    fn from(snapshot: AccountSnapshot) -> Self {
        Self::AccountContent(account_lines(&snapshot))
    }
}

/// 快照 → 行清单：档案一行 / 用量一行 / 能力位逐条。
fn account_lines(snapshot: &AccountSnapshot) -> Vec<AccountLine> {
    let mut lines = vec![
        AccountLine {
            primary: profile_name(snapshot),
            secondary: profile_user_id(snapshot),
        },
        AccountLine {
            primary: USAGE_LABEL.to_owned(),
            secondary: usage_text(&snapshot.usage),
        },
    ];
    lines.extend(snapshot.capabilities.iter().map(|bit| AccountLine {
        primary: bit.name.clone(),
        secondary: bit.mark.label().to_owned(),
    }));
    lines
}

/// 档案行主文案：账号名（缺失 ⇒ 占位，不拿空串冒充）。
fn profile_name(snapshot: &AccountSnapshot) -> String {
    let name = snapshot.profile.name.trim();
    if name.is_empty() {
        ACCOUNT_NAME_PLACEHOLDER.to_owned()
    } else {
        name.to_owned()
    }
}

/// 档案行次文案：用户 ID（缺失 ⇒ 占位）。
fn profile_user_id(snapshot: &AccountSnapshot) -> String {
    let id = snapshot.profile.user_id.trim();
    if id.is_empty() {
        ACCOUNT_ID_PLACEHOLDER.to_owned()
    } else {
        id.to_owned()
    }
}

/// 用量行次文案：「已用 {used} / {quota} {unit}」（格式对齐文案全表的用量条目）。
fn usage_text(usage: &SubscriptionUsage) -> String {
    if usage.unit.trim().is_empty() {
        format!("已用 {} / {}", usage.used, usage.quota)
    } else {
        format!("已用 {} / {} {}", usage.used, usage.quota, usage.unit)
    }
}

#[cfg(test)]
mod tests {
    // 显式导入符号（不用 `use super::*`）：与仓内约定保持一致。
    use super::{
        account_body, AccountPanelBody, AccountProfile, AccountSnapshot, CapabilityBit,
        CapabilityMark, SubscriptionUsage, ACCOUNT_ID_PLACEHOLDER, ACCOUNT_NAME_PLACEHOLDER,
    };
    use crate::state::datasource::LoadState;
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

    // --- C8 快照正文（消费侧契约出口）---

    /// 造一份就绪快照（数字 / 名字只出现在这里，断言只对映射结果负责）。
    fn ready_snapshot() -> LoadState<AccountSnapshot> {
        LoadState::Ready(AccountSnapshot {
            profile: AccountProfile {
                name: "张三".to_string(),
                user_id: "u-1".to_string(),
            },
            usage: SubscriptionUsage {
                quota: 10,
                used: 3,
                unit: "积分".to_string(),
            },
            capabilities: vec![CapabilityBit {
                name: "安全能力".to_string(),
                mark: CapabilityMark::NotIssued,
            }],
        })
    }

    #[test]
    fn not_issued_capability_shows_not_issued_never_empty_or_zero() {
        match AccountPanelBody::from_account(ready_snapshot()) {
            AccountPanelBody::AccountContent(lines) => {
                let bit = lines
                    .iter()
                    .find(|line| line.primary == "安全能力")
                    .expect("能力行可定位");
                assert_eq!(bit.secondary, "未下发");
                assert_ne!(bit.secondary, "", "未下发不得显示成空");
                assert_ne!(bit.secondary, "0", "未下发不得显示成 0");
            }
            other => panic!("快照就绪必须是账号行，实际 {other:?}"),
        }
    }

    #[test]
    fn ready_snapshot_makes_account_name_and_usage_locatable() {
        match AccountPanelBody::from_account(ready_snapshot()) {
            AccountPanelBody::AccountContent(lines) => {
                assert!(
                    lines.iter().any(|line| line.primary == "张三"),
                    "账号名必须可定位"
                );
                assert!(
                    lines.iter().any(|line| line.primary == "月度限额"
                        && line.secondary == "已用 3 / 10 积分"),
                    "订阅用量必须可定位（配额 + 已用 + 单位）"
                );
            }
            other => panic!("快照就绪必须是账号行，实际 {other:?}"),
        }
    }

    #[test]
    fn failed_account_state_keeps_error_text() {
        let body =
            AccountPanelBody::from_account(LoadState::Failed("账号服务不可用".to_string()));
        assert!(
            matches!(body, AccountPanelBody::Failed(ref reason) if reason == "账号服务不可用"),
            "fail-closed：失败必须带原因透出，不 panic、不退化成空"
        );
    }

    #[test]
    fn loading_account_state_is_loading() {
        assert!(matches!(
            AccountPanelBody::from_account(LoadState::<AccountSnapshot>::Loading),
            AccountPanelBody::Loading
        ));
    }

    #[test]
    fn capability_mark_three_states_have_distinct_labels() {
        assert_eq!(CapabilityMark::Available.label(), "可用");
        assert_eq!(CapabilityMark::Unavailable.label(), "不可用");
        assert_eq!(CapabilityMark::NotIssued.label(), "未下发");
        assert_ne!(CapabilityMark::NotIssued.label(), CapabilityMark::Available.label());
    }

    #[test]
    fn missing_profile_fields_fall_back_to_placeholders() {
        let body = AccountPanelBody::from_account(LoadState::Ready(AccountSnapshot {
            profile: AccountProfile {
                name: "  ".to_string(),
                user_id: String::new(),
            },
            usage: SubscriptionUsage {
                quota: 0,
                used: 0,
                unit: String::new(),
            },
            capabilities: vec![],
        }));
        match body {
            AccountPanelBody::AccountContent(lines) => {
                assert_eq!(lines[0].primary, ACCOUNT_NAME_PLACEHOLDER);
                assert_eq!(lines[0].secondary, ACCOUNT_ID_PLACEHOLDER);
                assert_eq!(lines[1].secondary, "已用 0 / 0");
            }
            other => panic!("期望账号行，实际 {other:?}"),
        }
    }
}
