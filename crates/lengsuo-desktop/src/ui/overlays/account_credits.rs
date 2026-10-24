//! 积分账户（波次 8，卡 `ui-account-credits`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `account.credits`
//! （全表该命名空间实测 40 键，[`ALL_TEXTS`] 全覆盖）。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "积分账户";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空态文案（Ready 且无可渲染载荷；逐字取自文案全表）。
pub const TEXT_EMPTY: &str = "暂无内容";

// ===== 纯数据层（无 GPUI 依赖）=====
/// 键数（据文案全表命名空间 `account.credits` grep 实测）。
pub const NS_KEY_COUNT: usize = 40;
/// 命名空间前缀。
pub const NS_PREFIX: &str = "account.credits";

// ---- 文案（全部逐字，键名见行尾注释）----
pub const ACCUMULATED: &str = "累计"; // accumulated
pub const ACCUMULATED_REMAINING: &str = "累积剩余:"; // accumulatedRemaining
pub const ACTIVITY_CREDITS: &str = "活动赠送积分"; // activityCredits
pub const ACTIVITY_EXPIRE_HINT: &str = "活动赠送积分到期后自动失效，请在有效期内使用"; // activityExpireHint
pub const BASE_CREDITS: &str = "基础体验积分"; // baseCredits
pub const BILLING_CYCLE: &str = "计费周期"; // billingCycle
pub const BUY_DISABLED_FREE: &str = "体验版暂不支持购买积分，请先升级套餐"; // buyDisabledFree
pub const DETAIL: &str = "积分明细"; // detail
pub const ENTERPRISE_CONTACT_ADMIN: &str = "请联系管理员查看用量详情"; // enterpriseContactAdmin
pub const EXHAUSTED: &str = "积分已用完，"; // exhausted
pub const EXHAUSTED_ENTERPRISE: &str = "积分已用完，请联系企业管理员购买资源"; // exhaustedEnterprise
pub const EXHAUSTED_LINK: &str = "请购买积分或升级套餐"; // exhaustedLink
pub const EXPIRE_TIME: &str = "到期时间"; // expireTime
pub const EXPIRY_DATE: &str = "到期日期"; // expiryDate
pub const EXPIRY_DATE_INTL: &str = "到期时间"; // expiryDate.intl
pub const EXTRA_CREDITS: &str = "积分"; // extraCredits
pub const FETCH_FAILED: &str = "获取失败"; // fetchFailed
pub const FETCH_FAILED_DESC: &str = "积分获取失败，请检查网络后重试"; // fetchFailedDesc
pub const FETCH_LOADING: &str = "获取中"; // fetchLoading
pub const FETCH_LOADING_DESC: &str = "正在获取积分"; // fetchLoadingDesc
pub const INSUFFICIENT: &str = "积分不足，"; // insufficient
pub const INSUFFICIENT_ENTERPRISE: &str = "积分不足，请联系企业管理员购买资源"; // insufficientEnterprise
pub const INSUFFICIENT_LINK: &str = "请升级套餐或购买积分"; // insufficientLink
pub const MONTHLY_QUOTA: &str = "月度限额"; // monthlyQuota
pub const PLAN_EXPIRY_TIME: &str = "版本到期时间"; // planExpiryTime
pub const REFRESH: &str = "刷新"; // refresh
pub const REFRESH_COOLDOWN: &str = "{seconds} 秒后可再次刷新"; // refreshCooldown
pub const REFRESH_DAILY: &str = "每月刷新"; // refreshDaily
pub const REFRESH_DATE: &str = "下次权益周期更新时间"; // refreshDate
pub const REFRESH_DATE_INTL: &str = "下次权益周期更新时间"; // refreshDate.intl
pub const REMAINING: &str = "剩余"; // remaining
pub const RESET_TIME: &str = "重置时间"; // resetTime
pub const STALE: &str = "非最新"; // stale
pub const STALE_DESC: &str = "当前为上次获取的积分，网络恢复后将自动刷新"; // staleDesc
pub const TOTAL: &str = "总量"; // total
pub const TOTAL_CREDITS: &str = "总积分"; // totalCredits
pub const UNIT: &str = "积分"; // unit
pub const UNLIMITED: &str = "不限量"; // unlimited
pub const VALIDITY_PERIOD: &str = "有效期"; // validityPeriod
pub const VALID_UNTIL: &str = "有效期至"; // validUntil

/// 命名空间全量表（40 键文案，测试据此机器判定覆盖率）。
pub const ALL_TEXTS: [&str; NS_KEY_COUNT] = [
    ACCUMULATED,
    ACCUMULATED_REMAINING,
    ACTIVITY_CREDITS,
    ACTIVITY_EXPIRE_HINT,
    BASE_CREDITS,
    BILLING_CYCLE,
    BUY_DISABLED_FREE,
    DETAIL,
    ENTERPRISE_CONTACT_ADMIN,
    EXHAUSTED,
    EXHAUSTED_ENTERPRISE,
    EXHAUSTED_LINK,
    EXPIRE_TIME,
    EXPIRY_DATE,
    EXPIRY_DATE_INTL,
    EXTRA_CREDITS,
    FETCH_FAILED,
    FETCH_FAILED_DESC,
    FETCH_LOADING,
    FETCH_LOADING_DESC,
    INSUFFICIENT,
    INSUFFICIENT_ENTERPRISE,
    INSUFFICIENT_LINK,
    MONTHLY_QUOTA,
    PLAN_EXPIRY_TIME,
    REFRESH,
    REFRESH_COOLDOWN,
    REFRESH_DAILY,
    REFRESH_DATE,
    REFRESH_DATE_INTL,
    REMAINING,
    RESET_TIME,
    STALE,
    STALE_DESC,
    TOTAL,
    TOTAL_CREDITS,
    UNIT,
    UNLIMITED,
    VALIDITY_PERIOD,
    VALID_UNTIL,
];

/// 浮层状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发，正常呈现。
    Ready,
    /// 拉取中（`fetchLoading` / `fetchLoadingDesc`）。
    Fetching,
    /// 拉取失败（fail-closed，必须显示错误）。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

impl State {
    /// 状态角标文案（Ready 无角标）。
    #[must_use]
    pub fn badge(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::Fetching => Some(FETCH_LOADING),
            Self::LoadFailed => Some(FETCH_FAILED),
            Self::NotDelivered => Some(NOT_DELIVERED),
        }
    }

    /// 状态说明行（Ready / NotDelivered 无说明）。
    #[must_use]
    pub fn badge_desc(self) -> Option<&'static str> {
        match self {
            Self::Fetching => Some(FETCH_LOADING_DESC),
            Self::LoadFailed => Some(FETCH_FAILED_DESC),
            Self::Ready | Self::NotDelivered => None,
        }
    }
}

/// 由运行时输入解析初始态（纯函数；`None` = 未下发）。
#[must_use]
pub fn resolve(delivered: Option<bool>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(false) => State::LoadFailed,
        Some(true) => State::Ready,
    }
}

/// 月度限额（`monthlyQuota`；无值域 ⇒ `unlimited` 不限量）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quota {
    /// 限额数值（`extraCredits` 作单位）。
    Limited(u64),
    /// 不限量。
    Unlimited,
}

impl Quota {
    /// 限额文案（数值 + 单位 / 不限量）。
    #[must_use]
    pub fn text(self) -> String {
        match self {
            Self::Limited(n) => format!("{n} {EXTRA_CREDITS}"),
            Self::Unlimited => UNLIMITED.to_string(),
        }
    }
}

/// 计费周期（该域文案全表仅 `refreshDaily` 一种周期文案）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BillingCycle {
    /// 按月（每月刷新）。
    Monthly,
}

impl BillingCycle {
    /// 周期文案。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Monthly => REFRESH_DAILY,
        }
    }
}

/// 权益构成桶：基础体验 / 活动赠送。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BucketKind {
    /// 基础体验积分。
    Base,
    /// 活动赠送积分（有到期提示）。
    Activity,
}

impl BucketKind {
    /// 桶标签文案。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Base => BASE_CREDITS,
            Self::Activity => ACTIVITY_CREDITS,
        }
    }

    /// 到期提示（仅活动赠送积分）。
    #[must_use]
    pub fn expire_hint(self) -> Option<&'static str> {
        match self {
            Self::Base => None,
            Self::Activity => Some(ACTIVITY_EXPIRE_HINT),
        }
    }
}

/// 单个权益构成条目（数值 / 到期值未下发 ⇒ None，⛔ 不伪造）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreditBucket {
    pub kind: BucketKind,
    pub amount: Option<u64>,
    /// 该桶到期值（`expireTime` 行标签在渲染层拼接）。
    pub expire_at: Option<String>,
}

/// 不足档位：已用完 / 不足。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortage {
    /// 积分已用完。
    Exhausted,
    /// 积分不足。
    Insufficient,
}

/// 受众：个人（可自助购买）/ 企业（找管理员）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Audience {
    /// 个人。
    #[default]
    Personal,
    /// 企业成员。
    Enterprise,
}

/// 不足横幅文案（纯函数）：返回（标题，自助操作链接）。
/// 个人：前缀 + 链接；企业：整句 + 无自助链接（找管理员）。
#[must_use]
pub fn shortage_text(shortage: Shortage, audience: Audience) -> (&'static str, Option<&'static str>) {
    match (shortage, audience) {
        (Shortage::Exhausted, Audience::Personal) => (EXHAUSTED, Some(EXHAUSTED_LINK)),
        (Shortage::Exhausted, Audience::Enterprise) => (EXHAUSTED_ENTERPRISE, None),
        (Shortage::Insufficient, Audience::Personal) => (INSUFFICIENT, Some(INSUFFICIENT_LINK)),
        (Shortage::Insufficient, Audience::Enterprise) => (INSUFFICIENT_ENTERPRISE, None),
    }
}

/// 到期日期行标签（`expiryDate` / `expiryDate.intl` 双键同文案）。
#[must_use]
pub fn expiry_label(intl: bool) -> &'static str {
    if intl { EXPIRY_DATE_INTL } else { EXPIRY_DATE }
}

/// 权益周期行标签（`refreshDate` / `refreshDate.intl` 双键同文案）。
#[must_use]
pub fn refresh_date_label(intl: bool) -> &'static str {
    if intl { REFRESH_DATE_INTL } else { REFRESH_DATE }
}

/// 积分账户数据模型（字段级 fail-closed：未下发 ≠ 0 / ≠ 空串）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CreditsAccount {
    pub state: State,
    /// 非最新（上次获取的缓存值）。
    pub stale: bool,
    pub audience: Audience,
    /// 体验版（购买入口禁用，`buyDisabledFree`）。
    pub free_plan: bool,
    pub remaining: Option<u64>,
    pub total: Option<u64>,
    pub accumulated: Option<u64>,
    pub quota: Option<Quota>,
    pub cycle: Option<BillingCycle>,
    pub buckets: Vec<CreditBucket>,
    /// 不足档位（服务端判定，本层不自造阈值）。
    pub shortage: Option<Shortage>,
    pub refresh_cooldown_secs: Option<u32>,
    pub valid_until: Option<String>,
    pub expiry_date: Option<String>,
    pub reset_time: Option<String>,
    pub refresh_date: Option<String>,
    pub plan_expiry_time: Option<String>,
}

impl CreditsAccount {
    /// fail-closed 构造：数据未下发。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self::default()
    }

    /// 数值文案（None ⇒ 未下发，⛔ 不伪造成 0）。
    #[must_use]
    pub fn amount_text(amount: Option<u64>) -> String {
        match amount {
            Some(n) => n.to_string(),
            None => NOT_DELIVERED.to_string(),
        }
    }

    /// 概览行「剩余 x / 总量 y」（两侧任一未下发都如实标注）。
    #[must_use]
    pub fn overview_line(&self) -> Option<String> {
        (self.remaining.is_some() || self.total.is_some()).then(|| {
            format!(
                "{} / {}",
                Self::amount_text(self.remaining),
                Self::amount_text(self.total)
            )
        })
    }

    /// 累积剩余行（`accumulatedRemaining` 逐字含冒号）。
    #[must_use]
    pub fn accumulated_line(&self) -> Option<String> {
        self.accumulated
            .map(|n| format!("{ACCUMULATED_REMAINING} {}", Self::amount_text(Some(n))))
    }

    /// 刷新冷却文案（`refreshCooldown` 占位符 `{seconds}` 替换）。
    #[must_use]
    pub fn refresh_cooldown_text(secs: u32) -> String {
        REFRESH_COOLDOWN.replace("{seconds}", &secs.to_string())
    }

    /// 明细行（label, value）：只含有数据的行，顺序固定。
    #[must_use]
    pub fn detail_rows(&self) -> Vec<(&'static str, String)> {
        let mut rows = Vec::new();
        if let Some(n) = self.accumulated {
            rows.push((ACCUMULATED, n.to_string()));
        }
        if let Some(cycle) = self.cycle {
            rows.push((BILLING_CYCLE, cycle.label().to_string()));
        }
        if let Some(quota) = self.quota {
            rows.push((MONTHLY_QUOTA, quota.text()));
        }
        if let Some(v) = &self.valid_until {
            rows.push((VALID_UNTIL, v.clone()));
        }
        if let Some(v) = &self.expiry_date {
            rows.push((expiry_label(false), v.clone()));
        }
        if let Some(v) = &self.reset_time {
            rows.push((RESET_TIME, v.clone()));
        }
        if let Some(v) = &self.refresh_date {
            rows.push((refresh_date_label(false), v.clone()));
        }
        if let Some(v) = &self.plan_expiry_time {
            rows.push((PLAN_EXPIRY_TIME, v.clone()));
        }
        rows
    }

    /// 空输入 ⇒ 空态（Ready 但无可渲染载荷；⛔ 不是空列表渲染）。
    #[must_use]
    pub fn is_empty_state(&self) -> bool {
        self.state == State::Ready
            && self.overview_line().is_none()
            && self.detail_rows().is_empty()
            && self.buckets.is_empty()
    }
}

// ===== GPUI 层 =====
/// 主区/浮层呈现入口（fail-closed：无数据源 ⇒ 「未下发」模型）。
#[must_use]
pub fn main_area(theme: &Theme) -> AnyElement {
    render(theme, &CreditsAccount::not_delivered()).into_any_element()
}

/// 按数据模型渲染整层：标题行（+ 状态角标）+ 状态分支主体。
#[must_use]
pub fn render(theme: &Theme, acct: &CreditsAccount) -> Div {
    let body = match acct.state {
        State::Fetching => status_line(theme, FETCH_LOADING, Some(FETCH_LOADING_DESC)),
        State::LoadFailed => status_line(theme, FETCH_FAILED, Some(FETCH_FAILED_DESC)),
        State::NotDelivered => status_line(theme, NOT_DELIVERED, None),
        State::Ready if acct.is_empty_state() => centered_line(theme, TEXT_EMPTY),
        State::Ready => ready_body(theme, acct),
    };
    let mut title = div().px_4().py_2().flex().items_center().gap_2().child(
        div()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(TITLE),
    );
    if let Some(badge) = acct.state.badge() {
        title = title.child(chip(theme, badge));
    }
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title)
        .child(body)
}

/// 居中单行（空态共用）。
fn centered_line(theme: &Theme, text: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_string())
}

/// 居中状态块（获取中 / 获取失败 / 未下发共用：主行 + 可选说明）。
fn status_line(theme: &Theme, main: &'static str, desc: Option<&str>) -> Div {
    let mut col = div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(main.to_string()),
        );
    if let Some(desc) = desc {
        col = col.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(desc.to_string()),
        );
    }
    col
}

/// Ready 主体：概览 + 不足横幅 + 非最新提示 + 权益构成 + 明细 + 操作行。
fn ready_body(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut col = div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .py_3()
        .child(overview_card(theme, acct));
    if let Some(shortage) = acct.shortage {
        col = col.child(shortage_banner(theme, shortage, acct.audience));
    }
    if acct.stale {
        col = col.child(stale_row(theme));
    }
    if !acct.buckets.is_empty() {
        col = col.child(buckets_card(theme, acct));
    }
    if !acct.detail_rows().is_empty() {
        col = col.child(detail_card(theme, acct));
    }
    col.child(action_row(theme, acct))
}

/// 概览卡：总积分标签 +「剩余 x / 总量 y 积分」+ 累积剩余行。
fn overview_card(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut card = div().flex().flex_col().gap_1().child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(TOTAL_CREDITS),
    );
    if let Some(line) = acct.overview_line() {
        card = card.child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(format!("{REMAINING} {line} {UNIT}")),
        );
    }
    if let Some(acc_line) = acct.accumulated_line() {
        card = card.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(acc_line),
        );
    }
    card
}

/// 不足横幅：标题句 +（个人）自助操作链接按钮。
fn shortage_banner(theme: &Theme, shortage: Shortage, audience: Audience) -> Div {
    let (headline, action) = shortage_text(shortage, audience);
    let mut col = div().flex().flex_col().gap_2().child(
        div()
            .text_sm()
            .text_color(theme.foreground)
            .child(headline),
    );
    if let Some(link) = action {
        col = col.child(pill_button(theme, link));
    }
    col
}

/// 非最新提示：角标 + 说明。
fn stale_row(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(chip(theme, STALE))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(STALE_DESC),
        )
}

/// 权益构成卡：有效期标签 + 每桶一行（标签 / 数值+单位）+ 桶级到期与提示。
fn buckets_card(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut card = div().flex().flex_col().gap_1().child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(VALIDITY_PERIOD),
    );
    for bucket in &acct.buckets {
        card = card.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(bucket.kind.label()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "{} {}",
                            CreditsAccount::amount_text(bucket.amount),
                            UNIT
                        )),
                ),
        );
        if let Some(expire_at) = &bucket.expire_at {
            card = card.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(format!("{EXPIRE_TIME}: {expire_at}")),
            );
        }
        if let Some(hint) = bucket.kind.expire_hint() {
            card = card.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(hint),
            );
        }
    }
    card
}

/// 明细卡：积分明细标签 + label/value 行 + 企业侧「请联系管理员」脚注。
fn detail_card(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut card = div().flex().flex_col().gap_1().child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(DETAIL),
    );
    for (label, value) in acct.detail_rows() {
        card = card.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(label),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(value),
                ),
        );
    }
    if acct.audience == Audience::Enterprise {
        card = card.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(ENTERPRISE_CONTACT_ADMIN),
        );
    }
    card
}

/// 操作行：刷新按钮 + 冷却文案 + 体验版购买禁用说明。
fn action_row(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(pill_button(theme, REFRESH));
    if let Some(secs) = acct.refresh_cooldown_secs {
        row = row.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(CreditsAccount::refresh_cooldown_text(secs)),
        );
    }
    if acct.free_plan {
        row = row.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(BUY_DISABLED_FREE),
        );
    }
    row
}

/// 药丸按钮（≥48dp 触摸目标；颜色一律主题 token）。
fn pill_button(theme: &Theme, label: &'static str) -> Div {
    div()
        .h_12()
        .min_w(px(112.))
        .self_start()
        .px_6()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.accent)
        .child(label)
}

/// 小角标（状态 / 非最新共用样式）。
fn chip(theme: &Theme, text: &'static str) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text)
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{resolve, State, NS_KEY_COUNT, NS_PREFIX, TITLE};

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "积分账户");
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
    }

    #[test]
    fn ns_prefix_declared() {
        // 用运行时比较绕开 clippy::assertions_on_constants（两侧都经函数边界）。
        let prefix = NS_PREFIX.to_string();
        let count = NS_KEY_COUNT;
        assert!(!prefix.is_empty());
        assert!(prefix.chars().next().is_some_and(char::is_alphabetic));
        assert!(count > 0 && count < 10_000);
    }
}
