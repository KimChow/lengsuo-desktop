//! 「订阅用量」面板的渲染描述（**描述层**：无 GPUI 依赖 ⇒ 无 `ui` feature 也能测）。
//!
//! ⛔ 只做「服务端下发的快照 → 面板该显示什么」的映射，不实现任何业务语义：
//! 购买积分 / 关闭自动续费等流程属于业务层，界面只展示 + 上报意图
//! （接线见 `subscription_panel.rs`）。
//!
//! 文案**逐字取自仓外文案全表**的 `subscriptionUsage` 命名空间（59 键），
//! ⛔ 不许自己编中文；`addon.*`（购买弹窗）与 `cancelRenew.*`（确认弹窗）
//! 两组弹窗文案不在本面板展示范围，故不在此定义。
//!
//! 硬约定：**只读**、**fail-closed**（未下发就显示「未下发」，⛔ 不退化成空列表）、
//! ⛔ 不做授权推理、⛔ 不持有凭据、⛔ 不伪造服务端数据。

/// 面板标题（与 `PANELS` 清单中的 title 一致，接线层不另造标题）。
pub const TITLE: &str = "订阅用量";

/// 服务端尚未下发内容时的提示（fail-closed：不伪造数据、不退化成空列表）。
///
/// ⚠️ 仅限「未下发」分支使用；验收口径把该分支排除在文案全表逐字命中之外。
pub const NOT_DELIVERED: &str = "服务端尚未下发内容。";

// ---- 汇总区（`subscriptionUsage` 顶层键） ----

/// `subscriptionUsage.autoRenewOn`
pub const AUTO_RENEW_ON: &str = "自动续费中";
/// `subscriptionUsage.expireTime`（`{time}` 由 [`expire_time_text`] 填充）
pub const EXPIRE_TIME: &str = "到期时间 {time}";
/// `subscriptionUsage.totalCredits`
pub const TOTAL_CREDITS: &str = "总剩余积分";

// ---- 积分明细区（`subscriptionUsage.breakdown.*`） ----

/// `subscriptionUsage.breakdown.title`
pub const BREAKDOWN_TITLE: &str = "积分明细";
/// `subscriptionUsage.breakdown.used`（`{used}`/`{total}` 由 [`used_text`] 填充）
pub const BREAKDOWN_USED: &str = "已使用 {used}/{total}";
/// `subscriptionUsage.breakdown.remainPercent`（`{percent}` 由 [`remain_percent_text`] 填充）
pub const BREAKDOWN_REMAIN_PERCENT: &str = "剩余{percent}%";
/// `subscriptionUsage.breakdown.nearestExpireTime`（`{time}` 由 [`nearest_expire_text`] 填充）
pub const BREAKDOWN_NEAREST_EXPIRE_TIME: &str = "最近到期时间 {time}";
/// `subscriptionUsage.breakdown.refreshTime`（`{time}` 由 [`refresh_time_text`] 填充）
pub const BREAKDOWN_REFRESH_TIME: &str = "下次刷新时间 {time}";
/// `subscriptionUsage.breakdown.viewDetail`
pub const VIEW_DETAIL: &str = "查看明细";

// ---- 分组明细状态（`subscriptionUsage.detail.*`） ----

/// `subscriptionUsage.detail.active`
pub const DETAIL_ACTIVE: &str = "有效期内";
/// `subscriptionUsage.detail.inactive`
pub const DETAIL_INACTIVE: &str = "已过期";
/// `subscriptionUsage.detail.empty`
pub const DETAIL_EMPTY: &str = "暂无信息";
/// `subscriptionUsage.detail.loadFailed`
pub const DETAIL_LOAD_FAILED: &str = "加载失败，请稍后重试";

// ---- 到期提示（`subscriptionUsage.expiry.*`；档位由数据层判定，描述层只映射文案） ----

/// `subscriptionUsage.expiry.contains`
pub const EXPIRY_CONTAINS: &str = "包含即将到期的积分";
/// `subscriptionUsage.expiry.days`
pub const EXPIRY_DAYS: &str = "3天内到期";
/// `subscriptionUsage.expiry.hours`
pub const EXPIRY_HOURS: &str = "24小时内到期";

// ---- 积分分组名（`subscriptionUsage.group.*`） ----

/// `subscriptionUsage.group.addon`
pub const GROUP_ADDON: &str = "购买积分";
/// `subscriptionUsage.group.benefit`
pub const GROUP_BENEFIT: &str = "平台奖励积分";
/// `subscriptionUsage.group.planBase`
pub const GROUP_PLAN_BASE: &str = "套餐基础积分";
/// `subscriptionUsage.group.planBonus`
pub const GROUP_PLAN_BONUS: &str = "套餐赠送积分";

// ---- 操作区（`subscriptionUsage` 顶层动作键；⛔ 只展示 + 上报意图，不实现支付） ----

/// `subscriptionUsage.buyCredits`
pub const BUY_CREDITS: &str = "购买积分";
/// `subscriptionUsage.upgradePlan`
pub const UPGRADE_PLAN: &str = "升级套餐";
/// `subscriptionUsage.cancelAutoRenew`
pub const CANCEL_AUTO_RENEW: &str = "关闭自动续费";

/// 只读行（⛔ 不可点、不给增删改；label 为文案常量，value 为已下发的展示串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub label: &'static str,
    pub value: String,
}

/// 积分明细的单行（全部字段已是展示串，接线层只照抄，不做二次判定）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakdownRow {
    /// 分组名（[`group_label`] 的映射结果）。
    pub label: &'static str,
    /// 有效期内 / 已过期（⇒ [`DETAIL_ACTIVE`] / [`DETAIL_INACTIVE`]）。
    pub active: bool,
    /// 用量文案（该组加载失败 ⇒ [`DETAIL_LOAD_FAILED`]，fail-closed）。
    pub used: String,
    /// 剩余百分比文案（该组加载失败 ⇒ 空串，接线层不渲染该行）。
    pub remain: String,
}

/// 积分分组种类（决定分组名文案；⛔ 不拿种类名冒充服务端数据）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    PlanBase,
    PlanBonus,
    Benefit,
    Addon,
}

/// 分组在界面上的名字（逐字取 `subscriptionUsage.group.*`）。
#[must_use]
pub const fn group_label(kind: GroupKind) -> &'static str {
    match kind {
        GroupKind::PlanBase => GROUP_PLAN_BASE,
        GroupKind::PlanBonus => GROUP_PLAN_BONUS,
        GroupKind::Benefit => GROUP_BENEFIT,
        GroupKind::Addon => GROUP_ADDON,
    }
}

/// 到期提示档位（离到期多近**由数据层判定**；描述层只映射文案，⛔ 不在 UI 做阈值推理）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryHint {
    /// `subscriptionUsage.expiry.contains`
    Contains,
    /// `subscriptionUsage.expiry.days`
    Days,
    /// `subscriptionUsage.expiry.hours`
    Hours,
}

/// 到期提示文案（档位 → 逐字文案）。
#[must_use]
pub const fn expiry_hint_text(hint: ExpiryHint) -> &'static str {
    match hint {
        ExpiryHint::Contains => EXPIRY_CONTAINS,
        ExpiryHint::Days => EXPIRY_DAYS,
        ExpiryHint::Hours => EXPIRY_HOURS,
    }
}

/// 单个分组的用量（`failed` ⇒ 该组明细加载失败，行内显示失败文案，fail-closed）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupUsage {
    pub kind: GroupKind,
    pub used: u64,
    pub total: u64,
    /// 有效期内 / 已过期。
    pub active: bool,
    /// 该组明细是否加载失败。
    pub failed: bool,
}

/// 服务端下发的一次快照（数据层给什么就传什么；⛔ 描述层不自行补全）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// 自动续费中。
    pub auto_renew: bool,
    /// 到期时间（原始串，展示前由 [`expire_time_text`] 填充）。
    pub expire_time: String,
    /// 总剩余积分。
    pub total_credits: u64,
    /// 最近到期时间（未下发 ⇒ `None`，⛔ 不编时间）。
    pub nearest_expire: Option<String>,
    /// 下次刷新时间（未下发 ⇒ `None`）。
    pub refresh_time: Option<String>,
    /// 到期提示档位（未下发 ⇒ `None`）。
    pub expiry_hint: Option<ExpiryHint>,
    /// 积分明细分组（空 ⇒ 接线层显示「暂无信息」，⛔ 不退化成空列表）。
    pub groups: Vec<GroupUsage>,
}

/// 有内容时的面板主体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// 自动续费徽标（true ⇒ 显示 [`AUTO_RENEW_ON`]）。
    pub auto_renew: bool,
    /// 到期时间整行（`到期时间 {time}` 填充结果）。
    pub expire_time: String,
    /// 总剩余积分行。
    pub total_credits: Row,
    /// 积分明细行。
    pub breakdown: Vec<BreakdownRow>,
    /// 最近到期时间整行（未下发 ⇒ `None`）。
    pub nearest_expire: Option<String>,
    /// 下次刷新时间整行（未下发 ⇒ `None`）。
    pub refresh_time: Option<String>,
    /// 到期提示文案（未下发 ⇒ `None`）。
    pub expiry_hint: Option<&'static str>,
    /// 操作区文案（⛔ 只展示：不实现支付、不判定权限；意图上报待交互通道接入）。
    pub actions: Vec<&'static str>,
}

/// 面板主体（fail-closed：未下发 ⇒ [`SubscriptionBody::NotDelivered`]，⛔ 不退化成空清单）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionBody {
    /// 服务端尚未下发内容。
    NotDelivered,
    /// 有可展示内容。
    Content(Box<Content>),
}

/// `到期时间 {time}` 的填充结果。
#[must_use]
pub fn expire_time_text(time: &str) -> String {
    EXPIRE_TIME.replace("{time}", time)
}

/// `最近到期时间 {time}` 的填充结果。
#[must_use]
pub fn nearest_expire_text(time: &str) -> String {
    BREAKDOWN_NEAREST_EXPIRE_TIME.replace("{time}", time)
}

/// `下次刷新时间 {time}` 的填充结果。
#[must_use]
pub fn refresh_time_text(time: &str) -> String {
    BREAKDOWN_REFRESH_TIME.replace("{time}", time)
}

/// `已使用 {used}/{total}` 的填充结果。
#[must_use]
pub fn used_text(used: u64, total: u64) -> String {
    BREAKDOWN_USED
        .replace("{used}", &used.to_string())
        .replace("{total}", &total.to_string())
}

/// `剩余{percent}%` 的填充结果。
#[must_use]
pub fn remain_percent_text(percent: u64) -> String {
    BREAKDOWN_REMAIN_PERCENT.replace("{percent}", &percent.to_string())
}

/// 剩余百分比（展示算术，非业务判定；`total` 为 0 视为 0%，⛔ 不除零）。
#[must_use]
/// （非 `const fn`：`Option::unwrap_or` 尚不是 const 稳定，改用 checked_div + unwrap_or 满足 clippy
/// `manual_checked_ops`。）
pub fn remain_percent(used: u64, total: u64) -> u64 {
    ((total.saturating_sub(used)) * 100)
        .checked_div(total)
        .unwrap_or(0)
}

/// 由快照映射面板主体（纯函数：未下发 ⇒ [`SubscriptionBody::NotDelivered`]）。
#[must_use]
pub fn body(snapshot: Option<&Snapshot>) -> SubscriptionBody {
    let Some(snapshot) = snapshot else {
        return SubscriptionBody::NotDelivered;
    };
    let breakdown = snapshot
        .groups
        .iter()
        .map(|group| BreakdownRow {
            label: group_label(group.kind),
            active: group.active,
            used: if group.failed {
                DETAIL_LOAD_FAILED.to_owned()
            } else {
                used_text(group.used, group.total)
            },
            remain: if group.failed {
                String::new()
            } else {
                remain_percent_text(remain_percent(group.used, group.total))
            },
        })
        .collect::<Vec<_>>();
    SubscriptionBody::Content(Box::new(Content {
        auto_renew: snapshot.auto_renew,
        expire_time: expire_time_text(&snapshot.expire_time),
        total_credits: Row {
            label: TOTAL_CREDITS,
            value: snapshot.total_credits.to_string(),
        },
        breakdown,
        nearest_expire: snapshot.nearest_expire.as_deref().map(nearest_expire_text),
        refresh_time: snapshot.refresh_time.as_deref().map(refresh_time_text),
        expiry_hint: snapshot.expiry_hint.map(expiry_hint_text),
        actions: vec![BUY_CREDITS, UPGRADE_PLAN, CANCEL_AUTO_RENEW],
    }))
}
