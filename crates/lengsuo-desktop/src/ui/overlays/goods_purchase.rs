//! 购买信息浮层（波次 8，卡 `ui-overlay-goods-purchase`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间
//! `goodsPurchase`（界面文案全表，17 键）。
//! 数据源在运行时侧：跨线契约就绪前，渲染层如实显示「未下发」。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（⛔ 无 GPUI 依赖） ----------

/// 视图名（模块元数据；命名空间内无标题键，标题行渲染支付结果横幅）。
pub const TITLE: &str = "购买信息浮层";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 界面文案（逐字取自文案全表命名空间 `goodsPurchase`，17 键，⛔ 不得改写）。
pub mod copy {
    /// 支付结果 · 成功。
    pub const PAY_SUCCESS: &str = "支付成功";
    /// 支付结果 · 超时。
    pub const PAY_TIMEOUT: &str = "支付超时，请重试";
    /// 底部协议链接（原文含书名号）。
    pub const AGREEMENT: &str = "《第三方付费服务使用规则》";
    /// 行标签 · 支付金额。
    pub const AMOUNT_LABEL: &str = "支付金额";
    /// 行标签 · 使用周期。
    pub const CYCLE_LABEL: &str = "使用周期";
    /// 使用周期 · 按月（`{date}` 占位原样保留，到期日由服务端下发）。
    pub const CYCLE_MONTHLY_TEMPLATE: &str = "{date}到期";
    /// 使用周期 · 永久。
    pub const CYCLE_PERMANENT: &str = "长期有效";
    /// 行标签 · 购买日期。
    pub const DATE_LABEL: &str = "购买日期";
    /// 退款 · 可退。
    pub const REFUNDABLE: &str = "支持7天内退款";
    /// 退款 · 可退时的操作。
    pub const REFUND_ACTION: &str = "申请退款";
    /// 退款 · 已过可退期。
    pub const REFUND_EXPIRED: &str = "超过7天不可退款";
    /// 行标签 · 退款。
    pub const REFUND_LABEL: &str = "退款";
    /// 续期 CTA（`{amount}` 占位原样保留，金额由服务端下发）。
    pub const RENEW_CTA_TEMPLATE: &str = "¥{amount}续期1个月";
    /// 退款流 · 失败。
    pub const REFUND_FAILED: &str = "退款失败，请稍后重试";
    /// 退款流 · 处理中。
    pub const REFUND_IN_PROGRESS: &str = "退款处理中，请稍后重试";
    /// 退款流 · 提交中。
    pub const REFUND_SUBMITTING: &str = "退款中…";
    /// 退款流 · 成功。
    pub const REFUND_SUCCESS: &str = "退款成功";

    /// 全部 17 键（与文案全表该命名空间一一对应；供一致性校验）。
    pub const ALL: [&str; 17] = [
        PAY_SUCCESS,
        PAY_TIMEOUT,
        AGREEMENT,
        AMOUNT_LABEL,
        CYCLE_LABEL,
        CYCLE_MONTHLY_TEMPLATE,
        CYCLE_PERMANENT,
        DATE_LABEL,
        REFUNDABLE,
        REFUND_ACTION,
        REFUND_EXPIRED,
        REFUND_LABEL,
        RENEW_CTA_TEMPLATE,
        REFUND_FAILED,
        REFUND_IN_PROGRESS,
        REFUND_SUBMITTING,
        REFUND_SUCCESS,
    ];
}

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    Loading,
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

impl State {
    /// 只有 `Normal` 展示数据；未拉取/加载中/失败一律 fail-closed（⇒「未下发」）。
    pub fn shows_info(self) -> bool {
        matches!(self, State::Normal)
    }
}

/// 支付结果（对应 `PAY_SUCCESS` / `PAY_TIMEOUT`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayResult {
    Success,
    Timeout,
}

impl PayResult {
    pub fn text(self) -> &'static str {
        match self {
            PayResult::Success => copy::PAY_SUCCESS,
            PayResult::Timeout => copy::PAY_TIMEOUT,
        }
    }
}

/// 使用周期（对应 `CYCLE_PERMANENT` / `CYCLE_MONTHLY_TEMPLATE`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cycle {
    /// 长期有效。
    Permanent,
    /// 按月，到期日由服务端下发（⛔ 不伪造）。
    Monthly { expiry: String },
}

impl Cycle {
    pub fn text(&self) -> String {
        match self {
            Cycle::Permanent => copy::CYCLE_PERMANENT.to_string(),
            Cycle::Monthly { expiry } => fill_slot(copy::CYCLE_MONTHLY_TEMPLATE, "{date}", expiry),
        }
    }
}

/// 退款态（对应 `REFUNDABLE` / `REFUND_EXPIRED` 与退款流 4 态）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefundState {
    /// 支持7天内退款（可发起申请退款）。
    Refundable,
    /// 超过7天不可退款。
    Expired,
    /// 退款中…（提交中）。
    Submitting,
    /// 退款处理中，请稍后重试。
    InProgress,
    /// 退款成功。
    Success,
    /// 退款失败，请稍后重试。
    Failed,
}

impl RefundState {
    pub fn text(self) -> &'static str {
        match self {
            RefundState::Refundable => copy::REFUNDABLE,
            RefundState::Expired => copy::REFUND_EXPIRED,
            RefundState::Submitting => copy::REFUND_SUBMITTING,
            RefundState::InProgress => copy::REFUND_IN_PROGRESS,
            RefundState::Success => copy::REFUND_SUCCESS,
            RefundState::Failed => copy::REFUND_FAILED,
        }
    }
}

/// 槽位填充（`{date}` / `{amount}`；纯函数，模板逐字取自文案全表）。
fn fill_slot(template: &str, slot: &str, value: &str) -> String {
    template.replace(slot, value)
}

/// 已下发的购买信息（任一字段 `None` = 该项未下发；⛔ 不伪造条目/数值）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PurchaseInfo {
    /// 支付金额（服务端下发原文；未下发 = `None`）。
    pub amount: Option<String>,
    /// 使用周期。
    pub cycle: Option<Cycle>,
    /// 购买日期（服务端下发原文；未下发 = `None`）。
    pub purchased_on: Option<String>,
    /// 支付结果。
    pub pay: Option<PayResult>,
    /// 退款态。
    pub refund: Option<RefundState>,
    /// 续期金额（服务端下发原文；未下发 = 不渲染续期 CTA）。
    pub renew_amount: Option<String>,
}

impl PurchaseInfo {
    /// 固定 4 行的行标签（金额/周期/日期/退款）。
    pub fn row_labels() -> [&'static str; 4] {
        [
            copy::AMOUNT_LABEL,
            copy::CYCLE_LABEL,
            copy::DATE_LABEL,
            copy::REFUND_LABEL,
        ]
    }

    /// 各行取值（未下发 ⇒ `None`，渲染层如实显示「未下发」）。
    pub fn row_values(&self) -> [Option<String>; 4] {
        [
            self.amount.clone(),
            self.cycle.as_ref().map(|c| c.text()),
            self.purchased_on.clone(),
            self.refund.map(|r| r.text().to_string()),
        ]
    }

    /// 空态判定：无任何已下发内容 ⇒ 空态视图（⛔ 不是空列表）。
    pub fn is_empty(&self) -> bool {
        self.amount.is_none()
            && self.cycle.is_none()
            && self.purchased_on.is_none()
            && self.pay.is_none()
            && self.refund.is_none()
            && self.renew_amount.is_none()
    }

    /// 续期 CTA 文案（金额未下发 ⇒ `None`，⛔ 不伪造数值）。
    pub fn renew_cta(&self) -> Option<String> {
        self.renew_amount
            .as_deref()
            .map(|a| fill_slot(copy::RENEW_CTA_TEMPLATE, "{amount}", a))
    }
}

// ---------- GPUI 层 ----------

/// 主区渲染（浮层不占主区，签名固定）。
///
/// 当前无运行时数据源 ⇒ `PurchaseInfo::default()` + `State::default()`，
/// fail-closed 如实显示「未下发」；跨线契约就绪后换源即可。
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &PurchaseInfo::default(), State::default())
}

/// 按纯数据层渲染：标题行（支付结果横幅）+ 内容节点 + 空态/fail-closed 分支。
fn render(theme: &Theme, info: &PurchaseInfo, state: State) -> Div {
    let mut col = div().flex().flex_col().gap_2();
    // 标题行：支付结果横幅（未下发 ⇒ 「未下发」）。
    col = col.child(pay_banner(theme, info.pay, state));
    if !state.shows_info() || info.is_empty() {
        // 空态（fail-closed）：未拉取过 ≠ 空集，⛔ 不渲染空行列表。
        return col.child(
            div()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        );
    }
    // 内容节点：固定 4 行（标签 + 已下发值，值未下发如实显示）。
    for (label, value) in PurchaseInfo::row_labels()
        .into_iter()
        .zip(info.row_values())
    {
        col = col.child(info_row(theme, label, value));
    }
    // 退款可发起时的操作节点。
    if info.refund == Some(RefundState::Refundable) {
        col = col.child(div().text_color(theme.accent).child(copy::REFUND_ACTION));
    }
    // 续期 CTA（金额未下发 ⇒ 不渲染，⛔ 不伪造数值）。
    if let Some(cta) = info.renew_cta() {
        col = col.child(
            div()
                .text_color(theme.accent_foreground)
                .child(SharedString::from(cta)),
        );
    }
    // 底部协议（原文含书名号）。
    col.child(
        div()
            .text_color(theme.muted_foreground)
            .child(copy::AGREEMENT),
    )
}

/// 标题行：支付结果横幅（成功/超时/未下发三分支）。
fn pay_banner(theme: &Theme, pay: Option<PayResult>, state: State) -> Div {
    match (state.shows_info(), pay) {
        (true, Some(PayResult::Success)) => {
            div().text_color(theme.success).child(copy::PAY_SUCCESS)
        }
        (true, Some(PayResult::Timeout)) => div().text_color(theme.danger).child(copy::PAY_TIMEOUT),
        _ => div()
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED),
    }
}

/// 内容行：标签（弱化）+ 值（未下发 ⇒ 如实显示，⛔ 不留空冒充）。
fn info_row(theme: &Theme, label: &'static str, value: Option<String>) -> Div {
    let value_node = match value {
        Some(v) => div()
            .text_color(theme.foreground)
            .child(SharedString::from(v)),
        None => div()
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED),
    };
    div()
        .flex()
        .flex_row()
        .justify_between()
        .child(div().text_color(theme.muted_foreground).child(label))
        .child(value_node)
}

#[cfg(test)]
mod tests {
    use super::{
        copy, main_area, pay_banner, render, Cycle, PayResult, PurchaseInfo, RefundState, State,
        NOT_DELIVERED, TITLE,
    };
    use gpui_kit::component::Theme;

    /// 测试夹具：一份完整下发的购买信息（仅测试用，⛔ 不进渲染默认值）。
    fn delivered_sample() -> PurchaseInfo {
        PurchaseInfo {
            amount: Some("¥12.00".into()),
            cycle: Some(Cycle::Permanent),
            purchased_on: Some("2026-10-01".into()),
            pay: Some(PayResult::Success),
            refund: Some(RefundState::Refundable),
            renew_amount: Some("12.00".into()),
        }
    }

    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
    }

    #[test]
    fn title_not_empty() {
        assert!(!TITLE.is_empty());
    }

    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }

    /// ① 纯函数层状态/计数正确。
    #[test]
    fn pure_layer_state_and_counts() {
        assert!(State::Normal.shows_info());
        assert!(!State::Loading.shows_info());
        assert!(!State::Failed.shows_info());
        assert!(!State::NotDelivered.shows_info());
        assert_eq!(copy::ALL.len(), 17);
        assert!(copy::ALL.iter().all(|s| !s.is_empty()));
        assert_eq!(PurchaseInfo::row_labels().len(), 4);
        let sample = delivered_sample();
        assert_eq!(
            sample.row_values().iter().filter(|v| v.is_some()).count(),
            4
        );
    }

    /// ② 空输入 ⇒ 空态（不是空列表）。
    #[test]
    fn empty_input_yields_empty_state() {
        let info = PurchaseInfo::default();
        assert!(info.is_empty());
        assert!(info.renew_cta().is_none());
        assert!(info.row_values().iter().all(Option::is_none));
        assert!(!NOT_DELIVERED.is_empty());
    }

    /// ③ 文案逐字一致（对多个键断言 + 模板填充）。
    #[test]
    fn copy_is_verbatim() {
        assert_eq!(copy::AMOUNT_LABEL, "支付金额");
        assert_eq!(copy::CYCLE_LABEL, "使用周期");
        assert_eq!(copy::DATE_LABEL, "购买日期");
        assert_eq!(copy::REFUND_LABEL, "退款");
        assert_eq!(copy::PAY_SUCCESS, "支付成功");
        assert_eq!(copy::PAY_TIMEOUT, "支付超时，请重试");
        assert_eq!(copy::CYCLE_PERMANENT, "长期有效");
        assert_eq!(copy::REFUNDABLE, "支持7天内退款");
        assert_eq!(copy::REFUND_EXPIRED, "超过7天不可退款");
        assert_eq!(copy::CYCLE_MONTHLY_TEMPLATE, "{date}到期");
        assert_eq!(copy::RENEW_CTA_TEMPLATE, "¥{amount}续期1个月");
        assert_eq!(
            Cycle::Monthly {
                expiry: "2026-11-24".into()
            }
            .text(),
            "2026-11-24到期"
        );
        assert_eq!(PayResult::Timeout.text(), "支付超时，请重试");
        assert_eq!(
            delivered_sample().renew_cta().as_deref(),
            Some("¥12.00续期1个月")
        );
    }

    /// ⑤ 状态分支覆盖（每个分支一条断言）。
    #[test]
    fn render_state_branches() {
        let theme = Theme::default();
        let sample = delivered_sample();
        // Normal：走数据分支（4 行 + 退款操作 + 续期 CTA + 协议）。
        let _ = render(&theme, &sample, State::Normal);
        assert!(State::Normal.shows_info());
        // Loading / Failed / NotDelivered：一律 fail-closed（⇒「未下发」）。
        for state in [State::Loading, State::Failed, State::NotDelivered] {
            let _ = render(&theme, &sample, state);
            assert!(!state.shows_info());
        }
        // 空数据 + Normal：空态分支（不是空列表）。
        let _ = render(&theme, &PurchaseInfo::default(), State::Normal);
    }

    /// ⑤ 标题行（支付结果横幅）三分支。
    #[test]
    fn pay_banner_branches() {
        let theme = Theme::default();
        let _ = pay_banner(&theme, Some(PayResult::Success), State::Normal);
        let _ = pay_banner(&theme, Some(PayResult::Timeout), State::Normal);
        let _ = pay_banner(&theme, None, State::Normal);
        // 状态非 Normal 时即使有支付结果也 fail-closed。
        let _ = pay_banner(&theme, Some(PayResult::Success), State::NotDelivered);
    }

    /// ⑤ 退款态六分支文案逐字落位。
    #[test]
    fn refund_state_text_branches() {
        assert_eq!(RefundState::Refundable.text(), "支持7天内退款");
        assert_eq!(RefundState::Expired.text(), "超过7天不可退款");
        assert_eq!(RefundState::Submitting.text(), "退款中…");
        assert_eq!(RefundState::InProgress.text(), "退款处理中，请稍后重试");
        assert_eq!(RefundState::Success.text(), "退款成功");
        assert_eq!(RefundState::Failed.text(), "退款失败，请稍后重试");
    }

    /// ⑤ 部分下发：已有值落位、未下发行如实缺省（⛔ 不伪造）。
    #[test]
    fn partial_delivery_keeps_undelivered_rows() {
        let info = PurchaseInfo {
            amount: Some("¥12.00".into()),
            ..Default::default()
        };
        assert!(!info.is_empty());
        let values = info.row_values();
        assert_eq!(values[0].as_deref(), Some("¥12.00"));
        assert!(values[1..].iter().all(Option::is_none));
        let _ = render(&Theme::default(), &info, State::Normal);
    }
}
