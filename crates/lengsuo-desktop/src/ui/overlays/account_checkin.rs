//! 签到（波次 8，卡 `ui-account-checkin`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `account.checkin`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "签到";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "account.checkin";
/// 键数（据文案全表命名空间 `account.checkin`）。
pub const NS_KEY_COUNT: usize = 39;

// ===== 纯数据层（无 GPUI 依赖）=====
/// 文案全表：命名空间 `account.checkin` 共 [`NS_KEY_COUNT`] 键（逐字；⛔ 不得增删改）。
pub const COPY: &[(&str, &str)] = &[
    ("account.checkin.alreadyClaimed", "今日已领取"),
    ("account.checkin.available.description", "每日免费领取 {credits} Credits"),
    ("account.checkin.available.descriptionStreak", "今日免费领取 {credits} Credits"),
    ("account.checkin.available.title", "🎁 今日礼包"),
    ("account.checkin.bubble.claimCredits", "领取 {credits} 积分"),
    ("account.checkin.claim", "立即领取"),
    ("account.checkin.claimed.rewardTip", "🎁 连续签到{days}天，{credits}积分等你拿。"),
    ("account.checkin.claimed.title", "今日礼包已到账"),
    ("account.checkin.claimedToday", "今日已领"),
    ("account.checkin.claimFailed", "领取失败，请稍后重试"),
    ("account.checkin.claiming", "领取中..."),
    ("account.checkin.claimNow", "立即领取 →"),
    ("account.checkin.claimSuccess", "领取成功！获得 {credits} Credits"),
    ("account.checkin.claimSuccessBtn", "+{credits} ✓"),
    ("account.checkin.creditValue", "{credits} 通用积分"),
    ("account.checkin.dailyClaimPrefix", "每日可领 "),
    ("account.checkin.daysValue", "{days} 天"),
    ("account.checkin.indicator", "你有 {credits} 积分待领取"),
    ("account.checkin.offlineCountdown", "⏳ 签到倒计时"),
    ("account.checkin.offlineCountdownDesc.date", "5 月 10 日"),
    ("account.checkin.offlineCountdownDesc.prefix", "签到功能将于 "),
    ("account.checkin.offlineCountdownDesc.suffix", " 下线，更丰富的积分玩法即将上线，敬请期待"),
    ("account.checkin.offlineReminder.date", "5 月 10 日"),
    ("account.checkin.offlineReminder.prefix", "签到将于 "),
    ("account.checkin.offlineReminder.suffix", " 下线，更多赚积分玩法已在路上！"),
    ("account.checkin.offlineTag", "5/10 结束"),
    ("account.checkin.periodSummaryMiddle", " · 累计 "),
    ("account.checkin.periodSummaryPrefix", "本期已领 "),
    ("account.checkin.periodTag", "1期·5/31结束"),
    ("account.checkin.periodTagDynamic", "1期·{date}结束"),
    ("account.checkin.periodTagFull", "{season}期·{date}结束"),
    ("account.checkin.periodTitle", "本期：专家团能量包"),
    ("account.checkin.progress.distancePrefix", "距离大奖还差"),
    ("account.checkin.progress.distanceSuffix", "天"),
    ("account.checkin.reward.arrived", "{credits} Pts 大奖已到账"),
    ("account.checkin.reward.description", "连续签到 · 领 10 倍大奖"),
    ("account.checkin.title", "Buddy 加油站"),
    ("account.checkin.tomorrow", "明日再来"),
    ("account.checkin.tryExperts", "体验专家团 →"),
];

/// 取命名空间内文案（纯函数；未收录 key ⇒ `None`，fail-closed，不猜不编）。
pub fn copy(key: &str) -> Option<&'static str> {
    COPY.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 占位符填充：`{credits}` / `{days}`（纯函数；未知占位符原样保留）。
pub fn fill(template: &str, credits: u32, days: u32) -> String {
    template
        .replace("{credits}", &credits.to_string())
        .replace("{days}", &days.to_string())
}

/// 服务端下发的日签数据快照（⛔ 全部字段以下发为准，缺省即未下发）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DailyData {
    /// 今日礼包 Credits 数。
    pub credits: u32,
    /// 连续签到天数（已领 ⇒ 含今日）。
    pub streak_days: u32,
    /// 今日是否已领。
    pub claimed: bool,
}

/// 浮层/面板状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发且今日未领（`available.*` 分支）。
    Available { credits: u32, streak_days: u32 },
    /// 领取请求进行中（`claiming`）。
    Claiming { credits: u32, streak_days: u32 },
    /// 今日已领（`claimed.*` 分支）。
    Claimed { credits: u32, streak_days: u32 },
    /// 领取失败（fail-closed，显示 `claimFailed`）。
    ClaimFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 由运行时输入解析初始态（纯函数；`None` ⇒ 未下发，⛔ 不猜）。
pub fn resolve(data: Option<&DailyData>) -> State {
    match data {
        None => State::NotDelivered,
        Some(d) if d.claimed => State::Claimed {
            credits: d.credits,
            streak_days: d.streak_days,
        },
        Some(d) => State::Available {
            credits: d.credits,
            streak_days: d.streak_days,
        },
    }
}

/// 命名空间内常用键（供 GPUI 层与测试取用，避免散落字面量）。
pub mod keys {
    pub const AVAILABLE_TITLE: &str = "account.checkin.available.title";
    pub const CLAIMED_TITLE: &str = "account.checkin.claimed.title";
    pub const CLAIM_FAILED: &str = "account.checkin.claimFailed";
    pub const CLAIM_NOW: &str = "account.checkin.claimNow";
    pub const CLAIMING: &str = "account.checkin.claiming";
    pub const CLAIM: &str = "account.checkin.claim";
    pub const TOMORROW: &str = "account.checkin.tomorrow";
    pub const PANEL_TITLE: &str = "account.checkin.title";
}

impl State {
    /// 点击领取：仅 `Available` 进入 `Claiming`，其余分支原样返回（纯函数）。
    pub fn begin_claim(self) -> State {
        match self {
            State::Available {
                credits,
                streak_days,
            } => State::Claiming {
                credits,
                streak_days,
            },
            other => other,
        }
    }

    /// 领取结果：`Claiming` ⇒ 成功转 `Claimed`（连续天数含今日 +1）／失败转 `ClaimFailed`。
    pub fn claim_result(self, ok: bool) -> State {
        match self {
            State::Claiming {
                credits,
                streak_days,
            } if ok => State::Claimed {
                credits,
                streak_days: streak_days + 1,
            },
            State::Claiming { .. } => State::ClaimFailed,
            other => other,
        }
    }

    /// 状态卡标题（逐字；文案缺失回落「未下发」，fail-closed）。
    pub fn card_title(&self) -> &'static str {
        match self {
            State::Available { .. } | State::Claiming { .. } | State::ClaimFailed => {
                copy(keys::AVAILABLE_TITLE).unwrap_or(NOT_DELIVERED)
            }
            State::Claimed { .. } => copy(keys::CLAIMED_TITLE).unwrap_or(NOT_DELIVERED),
            State::NotDelivered => NOT_DELIVERED,
        }
    }

    /// 主行动文案（逐字）。
    pub fn primary_label(&self) -> &'static str {
        match self {
            State::Available { .. } => copy(keys::CLAIM_NOW).unwrap_or(NOT_DELIVERED),
            State::Claiming { .. } => copy(keys::CLAIMING).unwrap_or(NOT_DELIVERED),
            State::Claimed { .. } => copy(keys::TOMORROW).unwrap_or(NOT_DELIVERED),
            State::ClaimFailed => copy(keys::CLAIM).unwrap_or(NOT_DELIVERED),
            State::NotDelivered => NOT_DELIVERED,
        }
    }

    /// 描述行（逐字模板 + 填充 `{credits}`/`{days}`）。
    pub fn description(&self) -> String {
        match self {
            State::Available {
                credits,
                streak_days,
            } => {
                let key = if *streak_days > 1 {
                    "account.checkin.available.descriptionStreak"
                } else {
                    "account.checkin.available.description"
                };
                fill(copy(key).unwrap_or(NOT_DELIVERED), *credits, *streak_days)
            }
            State::Claiming {
                credits,
                streak_days,
            } => fill(
                copy("account.checkin.bubble.claimCredits").unwrap_or(NOT_DELIVERED),
                *credits,
                *streak_days,
            ),
            State::Claimed {
                credits,
                streak_days,
            } => fill(
                copy("account.checkin.claimed.rewardTip").unwrap_or(NOT_DELIVERED),
                *credits,
                *streak_days,
            ),
            State::ClaimFailed => copy(keys::CLAIM_FAILED)
                .unwrap_or(NOT_DELIVERED)
                .to_string(),
            State::NotDelivered => NOT_DELIVERED.to_string(),
        }
    }

    /// 是否可发起领取（仅 `Available`）。
    pub fn can_claim(&self) -> bool {
        matches!(self, State::Available { .. })
    }

    /// fail-closed 态（未下发/领取失败：必须显式呈现，⛔ 不落空列表）。
    pub fn is_fail_closed(&self) -> bool {
        matches!(self, State::NotDelivered | State::ClaimFailed)
    }
}

// ===== GPUI 层 =====
/// 主区/浮层呈现入口（无数据源 ⇒ `NotDelivered` 空态，fail-closed 呈现「未下发」）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render(&resolve(None), theme).into_any_element()
}

/// 整棵树渲染（视图是状态的纯函数；颜色/字号一律 `theme.*` 与语义类，⛔ 不硬编码色值）。
pub fn render(state: &State, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .w_full()
        .text_sm()
        .text_color(theme.foreground)
        // 标题行：视图标题 + 面板标题（均逐字取自文案全表）。
        .child(div().font_bold().child(SharedString::from(format!(
            "{} · {}",
            TITLE,
            copy(keys::PANEL_TITLE).unwrap_or_default()
        ))))
        // 内容节点 1：状态卡（按分支呈现；`NotDelivered` ⇒ 「未下发」空态）。
        .child(state_card(state, theme))
        // 内容节点 2：本期大奖信息条（静态文案，逐字；⛔ 不含动态数值）。
        .child(reward_strip(theme))
        // 内容节点 3：下线倒计时提示（静态文案，逐字拼接 prefix/date/suffix）。
        .child(offline_strip(theme))
        // 内容节点 4：次级动作行（静态文案，逐字）。
        .child(footer_strip(theme))
}

/// 状态卡：`NotDelivered` ⇒ 「未下发」空态（⛔ 不是空列表）；其余分支标题/描述/主行动逐字。
fn state_card(state: &State, theme: &Theme) -> impl IntoElement {
    let card = div().flex().flex_col().gap_1().w_full().text_sm();
    if matches!(state, State::NotDelivered) {
        return card
            .text_color(theme.muted_foreground)
            .child(SharedString::from(NOT_DELIVERED));
    }
    card.text_color(theme.foreground)
        .child(
            div()
                .font_bold()
                .child(SharedString::from(state.card_title())),
        )
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(state.description())),
        )
        .child(
            div()
                .text_color(theme.accent)
                .child(SharedString::from(state.primary_label())),
        )
}

/// 本期大奖信息条：`reward.description` / `periodTitle` / `periodTag`（逐字）。
fn reward_strip(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(
            copy("account.checkin.reward.description").unwrap_or_default(),
        ))
        .child(SharedString::from(
            copy("account.checkin.periodTitle").unwrap_or_default(),
        ))
        .child(SharedString::from(
            copy("account.checkin.periodTag").unwrap_or_default(),
        ))
}

/// 下线倒计时提示：`offlineCountdown` + `offlineCountdownDesc.{prefix,date,suffix}`（逐字拼接）。
fn offline_strip(theme: &Theme) -> impl IntoElement {
    let line = format!(
        "{}{}{}",
        copy("account.checkin.offlineCountdownDesc.prefix").unwrap_or_default(),
        copy("account.checkin.offlineCountdownDesc.date").unwrap_or_default(),
        copy("account.checkin.offlineCountdownDesc.suffix").unwrap_or_default(),
    );
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(
            copy("account.checkin.offlineCountdown").unwrap_or_default(),
        ))
        .child(SharedString::from(line))
}

/// 次级动作行：`tomorrow` / `tryExperts`（逐字）。
fn footer_strip(theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(
            copy("account.checkin.tomorrow").unwrap_or_default(),
        ))
        .child(SharedString::from(
            copy("account.checkin.tryExperts").unwrap_or_default(),
        ))
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{copy, resolve, State, COPY, NS_KEY_COUNT, TITLE};

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "签到");
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
    }

    #[test]
    fn copy_table_verbatim() {
        // 逐字抽查 ≥3 键 + 全表键数与声明一致。
        assert_eq!(copy("account.checkin.claim"), Some("立即领取"));
        assert_eq!(copy("account.checkin.claimed.title"), Some("今日礼包已到账"));
        assert_eq!(copy("account.checkin.offlineTag"), Some("5/10 结束"));
        let count = COPY.len();
        assert_eq!(count, NS_KEY_COUNT);
    }
}
