//! 积分账户（波次 8，卡 `ui-account-credits`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造；文案逐字取自 `account.credits`（实测 40 键，测试逐键断言）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

pub const TITLE: &str = "积分账户";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空态文案（Ready 且无可渲染载荷；逐字取自文案全表）。
pub const TEXT_EMPTY: &str = "暂无内容";

// ===== 纯数据层（无 GPUI 依赖）=====
/// 键数（据文案全表命名空间 `account.credits` grep 实测；测试逐键断言）。
pub const NS_KEY_COUNT: usize = 40;
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

/// 浮层状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    Ready,
    Fetching,
    /// 拉取失败（fail-closed，必须显示错误）。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

impl State {
    /// 状态角标（Ready 无角标；取 `fetchLoading` / `fetchFailed` / 未下发）。
    #[must_use]
    pub fn badge(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::Fetching => Some(FETCH_LOADING),
            Self::LoadFailed => Some(FETCH_FAILED),
            Self::NotDelivered => Some(NOT_DELIVERED),
        }
    }
}

/// 由运行时输入解析初始态（`None` = 未下发）。
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
    Unlimited,
}

impl Quota {
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
    Base,
    /// 活动赠送积分（有到期提示）。
    Activity,
}

impl BucketKind {
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
    /// 该桶到期值（渲染层配 `expireTime` 行标签）。
    pub expire_at: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortage {
    Exhausted,
    Insufficient,
}

/// 受众：个人（可自助购买）/ 企业（找管理员）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Audience {
    #[default]
    Personal,
    Enterprise,
}

/// 不足横幅文案（纯函数）：个人 = 前缀 + 自助链接；企业 = 整句、无自助链接。
#[must_use]
pub fn shortage_text(
    shortage: Shortage,
    audience: Audience,
) -> (&'static str, Option<&'static str>) {
    match (shortage, audience) {
        (Shortage::Exhausted, Audience::Personal) => (EXHAUSTED, Some(EXHAUSTED_LINK)),
        (Shortage::Exhausted, Audience::Enterprise) => (EXHAUSTED_ENTERPRISE, None),
        (Shortage::Insufficient, Audience::Personal) => (INSUFFICIENT, Some(INSUFFICIENT_LINK)),
        (Shortage::Insufficient, Audience::Enterprise) => (INSUFFICIENT_ENTERPRISE, None),
    }
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
    /// fail-closed 构造（未下发；⛔ 不伪造成 0 / 空串）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self::default()
    }

    /// 数值文案（None ⇒ 未下发）。
    #[must_use]
    pub fn amount_text(amount: Option<u64>) -> String {
        match amount {
            Some(n) => n.to_string(),
            None => NOT_DELIVERED.to_string(),
        }
    }

    /// 概览行「x / y」（两侧任一未下发都如实标注）。
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
        let timestamps = [
            (VALID_UNTIL, &self.valid_until),
            (EXPIRY_DATE, &self.expiry_date),
            (RESET_TIME, &self.reset_time),
            (REFRESH_DATE, &self.refresh_date),
            (PLAN_EXPIRY_TIME, &self.plan_expiry_time),
        ];
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
        for (label, value) in timestamps {
            if let Some(value) = value {
                rows.push((label, value.clone()));
            }
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
        State::Ready if acct.is_empty_state() => status_line(theme, TEXT_EMPTY, None),
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
        title = title.child(framed(theme, badge, false));
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

fn status_line(theme: &Theme, main: &'static str, desc: Option<&str>) -> Div {
    let mut col = div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(fg_sm(theme, main));
    if let Some(desc) = desc {
        col = col.child(muted_xs(theme, desc));
    }
    col
}

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
        col = col.child(pair_row(
            framed(theme, STALE, false),
            muted_xs(theme, STALE_DESC),
        ));
    }
    if !acct.buckets.is_empty() {
        col = col.child(buckets_card(theme, acct));
    }
    if !acct.detail_rows().is_empty() {
        col = col.child(detail_card(theme, acct));
    }
    col.child(action_row(theme, acct))
}

fn overview_card(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut card = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(muted_xs(theme, TOTAL_CREDITS));
    if let Some(line) = acct.overview_line() {
        card = card.child(fg_sm(theme, &format!("{REMAINING} {line} {UNIT}")));
    }
    if let Some(acc_line) = acct.accumulated_line() {
        card = card.child(muted_xs(theme, &acc_line));
    }
    card
}

fn shortage_banner(theme: &Theme, shortage: Shortage, audience: Audience) -> Div {
    let (headline, action) = shortage_text(shortage, audience);
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(fg_sm(theme, headline));
    if let Some(link) = action {
        col = col.child(framed(theme, link, true));
    }
    col
}

fn buckets_card(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut card = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(muted_xs(theme, VALIDITY_PERIOD));
    for bucket in &acct.buckets {
        let amount = format!("{} {}", CreditsAccount::amount_text(bucket.amount), UNIT);
        card = card.child(pair_row(
            fg_sm(theme, bucket.kind.label()),
            muted_xs(theme, &amount),
        ));
        if let Some(expire_at) = &bucket.expire_at {
            card = card.child(muted_xs(theme, &format!("{EXPIRE_TIME}: {expire_at}")));
        }
        if let Some(hint) = bucket.kind.expire_hint() {
            card = card.child(muted_xs(theme, hint));
        }
    }
    card
}

fn detail_card(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut card = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(muted_xs(theme, DETAIL));
    for (label, value) in acct.detail_rows() {
        card = card.child(pair_row(muted_xs(theme, label), fg_sm(theme, &value)));
    }
    if acct.audience == Audience::Enterprise {
        card = card.child(muted_xs(theme, ENTERPRISE_CONTACT_ADMIN));
    }
    card
}

fn action_row(theme: &Theme, acct: &CreditsAccount) -> Div {
    let mut row = div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(framed(theme, REFRESH, true));
    if let Some(secs) = acct.refresh_cooldown_secs {
        let cooldown = CreditsAccount::refresh_cooldown_text(secs);
        row = row.child(muted_xs(theme, &cooldown));
    }
    if acct.free_plan {
        row = row.child(muted_xs(theme, BUY_DISABLED_FREE));
    }
    row
}

/// 描边小件：`big` = 药丸按钮（≥48dp 触摸目标）；否则状态角标。颜色一律主题 token。
fn framed(theme: &Theme, label: &'static str, big: bool) -> Div {
    let mut node = div().rounded_md().border_1().border_color(theme.border);
    if big {
        node = node
            .h_12()
            .min_w(px(112.))
            .self_start()
            .px_6()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.accent);
    } else {
        node = node
            .px_2()
            .py_1()
            .text_xs()
            .text_color(theme.muted_foreground);
    }
    node.child(label)
}

fn pair_row(first: Div, second: Div) -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap_2()
        .child(first)
        .child(second)
}

fn muted_xs(theme: &Theme, text: &str) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text.to_string())
}

fn fg_sm(theme: &Theme, text: &str) -> Div {
    div()
        .text_sm()
        .text_color(theme.foreground)
        .child(text.to_string())
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块 `use gpui_kit::*` 会把 GPUI 的 `test` 宏
    // 带进来遮蔽 `#[test]` ⇒ recursion limit。显式列符号。
    use super::{
        main_area, render, resolve, shortage_text, Audience, BillingCycle, BucketKind,
        CreditBucket, CreditsAccount, Quota, Shortage, State, ACCUMULATED, ACCUMULATED_REMAINING,
        ACTIVITY_CREDITS, ACTIVITY_EXPIRE_HINT, BASE_CREDITS, BILLING_CYCLE, BUY_DISABLED_FREE,
        DETAIL, ENTERPRISE_CONTACT_ADMIN, EXHAUSTED, EXHAUSTED_ENTERPRISE, EXHAUSTED_LINK,
        EXPIRE_TIME, EXPIRY_DATE, EXPIRY_DATE_INTL, EXTRA_CREDITS, FETCH_FAILED, FETCH_FAILED_DESC,
        FETCH_LOADING, FETCH_LOADING_DESC, INSUFFICIENT, INSUFFICIENT_ENTERPRISE,
        INSUFFICIENT_LINK, MONTHLY_QUOTA, NOT_DELIVERED, NS_KEY_COUNT, NS_PREFIX, PLAN_EXPIRY_TIME,
        REFRESH, REFRESH_COOLDOWN, REFRESH_DAILY, REFRESH_DATE, REFRESH_DATE_INTL, REMAINING,
        RESET_TIME, STALE, STALE_DESC, TEXT_EMPTY, TITLE, TOTAL, TOTAL_CREDITS, UNIT, UNLIMITED,
        VALIDITY_PERIOD, VALID_UNTIL,
    };
    use gpui_kit::component::Theme;

    fn bare(state: State) -> CreditsAccount {
        CreditsAccount {
            state,
            ..CreditsAccount::default()
        }
    }

    fn bucket(kind: BucketKind, amount: Option<u64>, expire_at: Option<&str>) -> CreditBucket {
        CreditBucket {
            kind,
            amount,
            expire_at: expire_at.map(str::to_string),
        }
    }

    fn full() -> CreditsAccount {
        CreditsAccount {
            state: State::Ready,
            stale: true,
            audience: Audience::Enterprise,
            free_plan: true,
            remaining: Some(12),
            total: Some(100),
            accumulated: Some(500),
            quota: Some(Quota::Limited(1000)),
            cycle: Some(BillingCycle::Monthly),
            buckets: vec![
                bucket(BucketKind::Base, Some(60), None),
                bucket(BucketKind::Activity, Some(40), Some("2026-11-01")),
            ],
            shortage: Some(Shortage::Exhausted),
            refresh_cooldown_secs: Some(30),
            valid_until: Some("2026-12-31".into()),
            expiry_date: Some("2026-11-30".into()),
            reset_time: Some("2026-11-01".into()),
            refresh_date: Some("2026-11-08".into()),
            plan_expiry_time: Some("2027-01-01".into()),
        }
    }

    #[test]
    fn fail_closed_default_and_resolve() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(CreditsAccount::not_delivered().state, State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(resolve(Some(false)), State::LoadFailed);
        assert_eq!(resolve(Some(true)), State::Ready);
    }

    #[test]
    fn state_badges_cover_all_four_branches_verbatim() {
        assert_eq!(State::Fetching.badge(), Some(FETCH_LOADING));
        assert_eq!(State::LoadFailed.badge(), Some(FETCH_FAILED));
        assert_eq!(State::NotDelivered.badge(), Some(NOT_DELIVERED));
        assert_eq!(State::Ready.badge(), None);
    }

    #[test]
    fn all_forty_texts_verbatim() {
        let count = NS_KEY_COUNT;
        let prefix = NS_PREFIX;
        assert_eq!(count, 40);
        assert_eq!(prefix, "account.credits");
        assert_eq!((TITLE, NOT_DELIVERED), ("积分账户", "未下发"));
        assert_eq!((TEXT_EMPTY, ACCUMULATED), ("暂无内容", "累计"));
        assert_eq!((ACCUMULATED_REMAINING, REMAINING), ("累积剩余:", "剩余"));
        assert_eq!(ACTIVITY_CREDITS, "活动赠送积分");
        assert_eq!(BASE_CREDITS, "基础体验积分");
        assert_eq!((DETAIL, TOTAL_CREDITS), ("积分明细", "总积分"));
        assert_eq!((TOTAL, UNLIMITED), ("总量", "不限量"));
        assert_eq!((BILLING_CYCLE, MONTHLY_QUOTA), ("计费周期", "月度限额"));
        assert_eq!((PLAN_EXPIRY_TIME, REFRESH), ("版本到期时间", "刷新"));
        assert_eq!(BUY_DISABLED_FREE, "体验版暂不支持购买积分，请先升级套餐");
        assert_eq!(ENTERPRISE_CONTACT_ADMIN, "请联系管理员查看用量详情");
        assert_eq!((EXHAUSTED, INSUFFICIENT), ("积分已用完，", "积分不足，"));
        assert_eq!(EXHAUSTED_LINK, "请购买积分或升级套餐");
        assert_eq!(INSUFFICIENT_LINK, "请升级套餐或购买积分");
        assert_eq!(EXHAUSTED_ENTERPRISE, "积分已用完，请联系企业管理员购买资源");
        assert_eq!(
            INSUFFICIENT_ENTERPRISE,
            "积分不足，请联系企业管理员购买资源"
        );
        assert_eq!((EXPIRE_TIME, EXPIRY_DATE), ("到期时间", "到期日期"));
        assert_eq!(EXPIRY_DATE_INTL, "到期时间");
        assert_eq!((EXTRA_CREDITS, UNIT), ("积分", "积分"));
        assert_eq!((FETCH_FAILED, FETCH_LOADING), ("获取失败", "获取中"));
        assert_eq!((REFRESH_DAILY, RESET_TIME), ("每月刷新", "重置时间"));
        assert_eq!(FETCH_FAILED_DESC, "积分获取失败，请检查网络后重试");
        assert_eq!(FETCH_LOADING_DESC, "正在获取积分");
        assert_eq!(REFRESH_COOLDOWN, "{seconds} 秒后可再次刷新");
        assert_eq!(REFRESH_DATE, "下次权益周期更新时间");
        assert_eq!(REFRESH_DATE_INTL, "下次权益周期更新时间");
        assert_eq!(STALE, "非最新");
        assert_eq!(STALE_DESC, "当前为上次获取的积分，网络恢复后将自动刷新");
        assert_eq!((VALIDITY_PERIOD, VALID_UNTIL), ("有效期", "有效期至"));
        assert_eq!(
            ACTIVITY_EXPIRE_HINT,
            "活动赠送积分到期后自动失效，请在有效期内使用"
        );
    }

    #[test]
    fn empty_input_is_empty_state_not_empty_list() {
        let empty = bare(State::Ready);
        assert_eq!(empty.state, State::Ready);
        assert!(empty.is_empty_state());
        assert_eq!(empty.overview_line(), None);
        assert!(empty.detail_rows().is_empty());
        assert!(empty.buckets.is_empty());
        let hidden = CreditsAccount::not_delivered();
        assert!(!hidden.is_empty_state());
        assert_eq!(hidden.state, State::NotDelivered);
    }

    #[test]
    fn shortage_text_four_branches_verbatim() {
        let (ex, ins) = (Shortage::Exhausted, Shortage::Insufficient);
        let (p, ent) = (Audience::Personal, Audience::Enterprise);
        assert_eq!(shortage_text(ex, p), (EXHAUSTED, Some(EXHAUSTED_LINK)));
        assert_eq!(shortage_text(ex, ent), (EXHAUSTED_ENTERPRISE, None));
        let want = (INSUFFICIENT, Some(INSUFFICIENT_LINK));
        assert_eq!(shortage_text(ins, p), want);
        assert_eq!(shortage_text(ins, ent), (INSUFFICIENT_ENTERPRISE, None));
    }

    #[test]
    fn pure_text_functions() {
        assert_eq!(Quota::Limited(1000).text(), format!("1000 {EXTRA_CREDITS}"));
        assert_eq!(Quota::Unlimited.text(), UNLIMITED.to_string());
        assert_eq!(BillingCycle::Monthly.label(), REFRESH_DAILY);
        assert_eq!(BucketKind::Base.label(), BASE_CREDITS);
        assert_eq!(BucketKind::Base.expire_hint(), None);
        assert_eq!(BucketKind::Activity.label(), ACTIVITY_CREDITS);
        let hint = BucketKind::Activity.expire_hint();
        assert_eq!(hint, Some(ACTIVITY_EXPIRE_HINT));
        assert_eq!(CreditsAccount::amount_text(Some(0)), "0");
        assert_eq!(CreditsAccount::amount_text(None), NOT_DELIVERED);
        assert_eq!(full().overview_line().as_deref(), Some("12 / 100"));
        let mut partial = bare(State::Ready);
        partial.remaining = Some(12);
        assert_eq!(partial.overview_line().as_deref(), Some("12 / 未下发"));
        assert_eq!(full().accumulated_line().as_deref(), Some("累积剩余: 500"));
        let cooldown = CreditsAccount::refresh_cooldown_text(30);
        assert_eq!(cooldown, "30 秒后可再次刷新");
    }

    #[test]
    fn detail_rows_fixed_order_and_only_present_fields() {
        let rows = full().detail_rows();
        let joined = rows.iter().map(|(k, _)| *k).collect::<Vec<_>>().join("|");
        assert_eq!(
            joined,
            "累计|计费周期|月度限额|有效期至|到期日期|重置时间|下次权益周期更新时间|版本到期时间"
        );
        assert_eq!(rows[0].1, "500");
        assert_eq!(rows[1].1, REFRESH_DAILY);
        assert!(bare(State::Ready).detail_rows().is_empty());
    }

    #[test]
    fn main_area_and_render_do_not_panic_across_branches() {
        let theme = Theme::default();
        let _ = main_area(&theme);
        for acct in [
            bare(State::Fetching),
            bare(State::LoadFailed),
            CreditsAccount::not_delivered(),
            bare(State::Ready),
            full(),
        ] {
            let _ = render(&theme, &acct);
        }
    }
}
