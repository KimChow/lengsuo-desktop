//! 消息中心主区视图（波次 5，卡 `ui-message-center`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：条目 / 空态 / 诚实占位文案；
//! - **GPUI 层**（[`main_area`]）：把描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自界面文案全表命名空间 `msgCenter`）。
//!
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后由
//! `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

// ───────────────────────── 纯数据层（无 GPUI 依赖） ─────────────────────────

/// 视图标题（逐字取自界面文案全表 `msgCenter.title`）。
pub const TITLE: &str = "消息中心";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 空态：暂无消息（`msgCenter.empty`）。
pub const EMPTY: &str = "暂无消息";

/// 空态：暂无缓存消息（`msgCenter.cacheEmpty`）。
pub const CACHE_EMPTY: &str = "暂无缓存消息";

/// 空态：暂无离线消息（`msgCenter.offlineEmpty`）。
pub const OFFLINE_EMPTY: &str = "暂无离线消息";

/// 行操作：删除（`msgCenter.delete`）。
pub const DELETE: &str = "删除";

/// 行操作：前往查看（`msgCenter.agentMail.gotoMailbox`）。
pub const GOTO_MAILBOX: &str = "前往查看";

/// 行操作：标为已读（`msgCenter.markRead`）。
pub const MARK_READ: &str = "标为已读";

/// 行操作：全部已读（`msgCenter.markAllRead`）。
pub const MARK_ALL_READ: &str = "全部已读";

/// 状态：已读（`msgCenter.read`）。
/// 「未读」在文案全表无对应 key ⇒ 未读用视觉圆点表达，不编造文案。
pub const READ: &str = "已读";

/// 页签：全部（`msgCenter.tabAll`）。
pub const TAB_ALL: &str = "全部";

/// 页签：待处理（`msgCenter.tabPending`）。
pub const TAB_PENDING: &str = "待处理";

/// 相对时间模板：刚刚（`msgCenter.timeJustNow`）。
pub const TIME_JUST_NOW: &str = "刚刚";

/// 相对时间模板：今天（`msgCenter.timeToday`）。
pub const TIME_TODAY: &str = "今天";

/// 相对时间模板：昨天（`msgCenter.timeYesterday`）。
pub const TIME_YESTERDAY: &str = "昨天";

/// 相对时间模板：`{n}` 分钟前（`msgCenter.timeMinutesAgo`）。
pub const TIME_MINUTES_AGO: &str = "{n} 分钟前";

/// 相对时间模板：`{n}` 小时前（`msgCenter.timeHoursAgo`）。
pub const TIME_HOURS_AGO: &str = "{n} 小时前";

/// 相对时间模板：`{n}` 天前（`msgCenter.timeDaysAgo`）。
pub const TIME_DAYS_AGO: &str = "{n} 天前";

/// 消息行的时间标签（模板逐字取自文案全表，`{n}` 为占位符）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelativeTime {
    JustNow,
    Today,
    Yesterday,
    MinutesAgo(u32),
    HoursAgo(u32),
    DaysAgo(u32),
}

impl RelativeTime {
    /// 把模板里的 `{n}` 替换成实际数值，得到显示文案。
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::JustNow => TIME_JUST_NOW.to_owned(),
            Self::Today => TIME_TODAY.to_owned(),
            Self::Yesterday => TIME_YESTERDAY.to_owned(),
            Self::MinutesAgo(n) => TIME_MINUTES_AGO.replace("{n}", &n.to_string()),
            Self::HoursAgo(n) => TIME_HOURS_AGO.replace("{n}", &n.to_string()),
            Self::DaysAgo(n) => TIME_DAYS_AGO.replace("{n}", &n.to_string()),
        }
    }
}

/// 一条消息行的描述（**只描述**；运行时数据未下发时不会有实例）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEntry {
    /// 发件人（`msgCenter.agentMail.sender`）。
    pub sender: String,
    /// 时间标签。
    pub time: RelativeTime,
    /// 未读标记（界面用圆点表达，不显示「未读」文案）。
    pub unread: bool,
}

/// 空态的成因（三种空态文案各自对应一种数据面）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyReason {
    /// 常规：暂无消息。
    Messages,
    /// 缓存面：暂无缓存消息。
    Cache,
    /// 离线面：暂无离线消息。
    Offline,
}

impl EmptyReason {
    /// 空态文案（逐字取自文案全表）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Messages => EMPTY,
            Self::Cache => CACHE_EMPTY,
            Self::Offline => OFFLINE_EMPTY,
        }
    }
}

/// 消息清单的可枚举状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageListState {
    /// 数据未下发（fail-closed 唯一运行时默认）。
    NotDelivered,
    /// 已下发且确认为空。
    Empty(EmptyReason),
    /// 已下发且有清单。
    Entries(Vec<MessageEntry>),
}

/// 顶部页签（全部 / 待处理）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    All,
    Pending,
}

impl Tab {
    /// 页签文案（逐字取自文案全表）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::All => TAB_ALL,
            Self::Pending => TAB_PENDING,
        }
    }
}

/// 消息中心视图的纯数据模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageCenterView {
    /// 当前页签。
    pub tab: Tab,
    /// 清单状态。
    pub state: MessageListState,
}

impl MessageCenterView {
    /// 由数据面构造视图模型。
    #[must_use]
    pub fn build(tab: Tab, state: MessageListState) -> Self {
        Self { tab, state }
    }

    /// 运行时唯一诚实默认：数据未下发（⛔ 不伪造条目、不退化为空列表）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self::build(Tab::All, MessageListState::NotDelivered)
    }

    /// 待处理（未读）条数。
    #[must_use]
    pub fn pending_count(&self) -> usize {
        match &self.state {
            MessageListState::Entries(entries) => entries.iter().filter(|e| e.unread).count(),
            _ => 0,
        }
    }

    /// 当前页签下应渲染的清单行（全部 ⇒ 全量；待处理 ⇒ 仅未读）。
    #[must_use]
    pub fn visible_rows(&self) -> Vec<&MessageEntry> {
        match (&self.state, self.tab) {
            (MessageListState::Entries(entries), Tab::All) => entries.iter().collect(),
            (MessageListState::Entries(entries), Tab::Pending) => {
                entries.iter().filter(|e| e.unread).collect()
            }
            (_, _) => Vec::new(),
        }
    }
}

// ───────────────────────────── GPUI 层 ─────────────────────────────

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}

#[cfg(test)]
mod tests {
    use super::{main_area, NOT_DELIVERED, TITLE};
    use gpui_kit::component::Theme;

    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert_eq!(TITLE, "消息中心");
    }

    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
