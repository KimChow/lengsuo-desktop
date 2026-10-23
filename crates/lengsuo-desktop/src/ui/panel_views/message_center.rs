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

/// 主区渲染（分派臂入口，签名不变）。
///
/// 运行时数据未下发 ⇒ 固定渲染 fail-closed 默认模型。
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render_view(theme, &MessageCenterView::not_delivered())
}

/// 按纯数据模型渲染：标题行 + 页签 + （清单行 / 空态 / 未下发）。
#[must_use]
pub fn render_view(theme: &Theme, view: &MessageCenterView) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header_row(theme, view))
        .child(content(theme, view))
}

/// 标题行：视图标题 + 全部/待处理页签 + 全部已读。
fn header_row(theme: &Theme, view: &MessageCenterView) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .border_b(px(1.))
        .border_color(theme.border)
        .child(
            div()
                .debug_selector(|| "msg-center-title".to_owned())
                .mr_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(tab_chip(theme, Tab::All, view.tab))
        .child(tab_chip(theme, Tab::Pending, view.tab))
        .child(div().flex_1())
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(MARK_ALL_READ),
        )
}

/// 页签：选中态用 `muted` 底色 + 前景文字；未选中弱化为次要文字。
fn tab_chip(theme: &Theme, tab: Tab, active: Tab) -> Div {
    let chip = if tab == active {
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(theme.muted)
            .text_color(theme.foreground)
    } else {
        div().px_2().py_1().text_color(theme.muted_foreground)
    };
    chip.text_sm().child(tab.label())
}

/// 内容区：清单 / 空态 / 未下发三态分派。
fn content(theme: &Theme, view: &MessageCenterView) -> Div {
    let area = div().flex_1().flex().flex_col();
    match &view.state {
        MessageListState::NotDelivered => area.child(centered_note(
            theme,
            "msg-center-notdelivered",
            NOT_DELIVERED,
        )),
        MessageListState::Empty(reason) => {
            area.child(centered_note(theme, "msg-center-empty", reason.label()))
        }
        MessageListState::Entries(_) => {
            let rows = view.visible_rows();
            // 页签过滤后为空仍是**空态**（不是空白列表）。
            if rows.is_empty() {
                area.child(centered_note(theme, "msg-center-empty", EMPTY))
            } else {
                let mut list = div().flex().flex_col();
                for (index, entry) in rows.iter().enumerate() {
                    list = list.child(row(theme, index, entry));
                }
                area.child(list)
            }
        }
    }
}

/// 居中弱化说明（空态 / 未下发共用骨架）。
fn centered_note(theme: &Theme, selector: &'static str, text: &'static str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .debug_selector(move || selector.to_owned())
        .child(text)
}

/// 一条消息行：未读圆点（或「已读」）+ 发件人 + 时间 + 行操作。
fn row(theme: &Theme, index: usize, entry: &MessageEntry) -> Div {
    let mut line = div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .border_b(px(1.))
        .border_color(theme.border)
        .debug_selector(move || format!("msg-center-row-{index}"));

    // 「未读」在文案全表无 key ⇒ 未读用 accent 圆点表达；已读显示文案。
    line = if entry.unread {
        line.child(div().size(px(6.)).rounded_full().bg(theme.accent))
    } else {
        line.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(READ),
        )
    };

    let mut actions = div()
        .flex()
        .gap_3()
        .text_xs()
        .text_color(theme.muted_foreground);
    if entry.unread {
        actions = actions.child(MARK_READ);
    }
    actions = actions.child(GOTO_MAILBOX).child(DELETE);

    line.child(
        div()
            .text_sm()
            .text_color(theme.foreground)
            .child(SharedString::from(entry.sender.clone())),
    )
    .child(div().flex_1())
    .child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(SharedString::from(entry.time.label())),
    )
    .child(actions)
}

#[cfg(test)]
mod tests {
    use super::{main_area, NOT_DELIVERED, TITLE};
    use gpui_kit::component::Theme;

    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert_eq!(TITLE, "\u{6d88}\u{606f}\u{4e2d}\u{5fc3}");
    }

    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "\u{672a}\u{4e0b}\u{53d1}");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
