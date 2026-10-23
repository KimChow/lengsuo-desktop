//! 频道主区视图（波次 5，卡 `ui-channels-space`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：条目 / 清单三态 / 计数 / 逐字文案；
//! - **GPUI 层**（[`main_area`] 一族，逐项挂 `#[cfg(feature = "ui")]`）：把描述接到
//!   元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自文案全表命名空间 `collab`）。
//!
//! 数据源在运行时侧：`main_area` 目前以 `None` 建视图（诚实占位），等跨线契约就绪后
//! 由 `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。

// ────────────────────────── 纯数据层 ──────────────────────────

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "频道";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 已下发但清单为空的说明（key `collab.plan.filter.search.empty`）。
pub const EMPTY: &str = "暂无可选项";

/// 未读指示文案（key `collab.detail.activityUnreadAriaLabel`）。
pub const UNREAD_HINT: &str = "有未读动态";

/// 成员计数模板（key `collab.activity.memberFilter.countLabel`），`{count}` 为占位。
pub const MEMBER_COUNT_TEMPLATE: &str = "{count}个成员";

/// 单个频道行：名称 + 未读数 + 成员数 + 选中态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelEntry {
    /// 频道名（来自数据源的标识字符串，非界面文案）。
    pub name: String,
    /// 未读消息数。
    pub unread: u32,
    /// 频道成员数。
    pub member_count: u32,
    /// 是否选中（清单高亮态）。
    pub selected: bool,
}

impl ChannelEntry {
    /// 便捷构造（测试与接线共用）。
    #[must_use]
    pub fn new(name: impl Into<String>, unread: u32, member_count: u32, selected: bool) -> Self {
        Self {
            name: name.into(),
            unread,
            member_count,
            selected,
        }
    }

    /// 成员计数文案（把 `{count}` 占位填成实际数）。
    #[must_use]
    pub fn member_count_label(&self) -> String {
        MEMBER_COUNT_TEMPLATE.replace("{count}", &self.member_count.to_string())
    }

    /// 是否有未读。
    #[must_use]
    pub const fn has_unread(&self) -> bool {
        self.unread > 0
    }
}

/// 清单三态：**未下发 ≠ 空**（fail-closed 的核心区分）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelListState {
    /// 数据源未下发（⛔ 不是空清单）。
    NotDelivered,
    /// 已下发且为空。
    Empty,
    /// 已下发且有频道条目。
    Entries(Vec<ChannelEntry>),
}

impl ChannelListState {
    /// 条目切片（未下发/空态 → 空切片；渲染层据此建清单行节点）。
    #[must_use]
    pub fn entries(&self) -> &[ChannelEntry] {
        match self {
            Self::Entries(entries) => entries.as_slice(),
            Self::NotDelivered | Self::Empty => &[],
        }
    }
}

/// 频道视图的纯数据描述；GPUI 层只是它的纯函数。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelsView {
    /// 清单状态。
    pub state: ChannelListState,
}

impl ChannelsView {
    /// 由数据源建视图：`None` = 未下发（fail-closed），`Some(空)` = 已下发的空清单。
    #[must_use]
    pub fn build(entries: Option<Vec<ChannelEntry>>) -> Self {
        Self {
            state: match entries {
                None => ChannelListState::NotDelivered,
                Some(entries) if entries.is_empty() => ChannelListState::Empty,
                Some(entries) => ChannelListState::Entries(entries),
            },
        }
    }

    /// 清单行数（未下发/空态均为 0）。
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.state.entries().len()
    }

    /// 全频道未读合计。
    #[must_use]
    pub fn total_unread(&self) -> u32 {
        self.state.entries().iter().map(|entry| entry.unread).sum()
    }

    /// 全频道成员数合计。
    #[must_use]
    pub fn total_members(&self) -> u32 {
        self.state
            .entries()
            .iter()
            .map(|entry| entry.member_count)
            .sum()
    }
}

// ────────────────────────── GPUI 层 ──────────────────────────

#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::Theme;
#[cfg(feature = "ui")]
use gpui_kit::*;

/// 标题行节点。
#[cfg(feature = "ui")]
fn title_node(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(TITLE)
}

/// 单条频道清单行节点：名称 + 成员计数，有未读时右侧给指示，选中整行高亮。
#[cfg(feature = "ui")]
fn row_node(theme: &Theme, entry: &ChannelEntry) -> Div {
    let name = div()
        .text_color(theme.foreground)
        .child(SharedString::from(entry.name.as_str()));
    let name = if entry.selected {
        name.font_bold()
    } else {
        name
    };
    let unread_hint = entry.has_unread().then(|| {
        div()
            .rounded_sm()
            .bg(theme.muted)
            .px_2()
            .py_1()
            .text_xs()
            .text_color(theme.foreground)
            .child(UNREAD_HINT)
    });
    let row = div()
        .px_4()
        .py_2()
        .text_sm()
        .flex()
        .items_center()
        .justify_between();
    let row = if entry.selected {
        row.bg(theme.muted)
    } else {
        row
    };
    row.child(
        div().flex().flex_col().gap_1().child(name).child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(entry.member_count_label())),
        ),
    )
    .children(unread_hint)
}

/// 清单容器节点（每行一个 [`row_node`]）。
#[cfg(feature = "ui")]
fn list_node(theme: &Theme, view: &ChannelsView) -> Div {
    div().flex_1().flex().flex_col().py_2().children(
        view.state
            .entries()
            .iter()
            .map(|entry| row_node(theme, entry)),
    )
}

/// 空态节点（已下发但无频道）。
#[cfg(feature = "ui")]
fn empty_node(theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(EMPTY)
}

/// 未下发节点（fail-closed，⛔ 不得替换为空列表）。
#[cfg(feature = "ui")]
fn not_delivered_node(theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(NOT_DELIVERED)
}

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
#[must_use]
#[cfg(feature = "ui")]
pub fn main_area(theme: &Theme) -> Div {
    // 数据源未接入 ⇒ `None` ⇒ 如实显示「未下发」（fail-closed）。
    let view = ChannelsView::build(None);
    let body = match &view.state {
        ChannelListState::NotDelivered => not_delivered_node(theme),
        ChannelListState::Empty => empty_node(theme),
        ChannelListState::Entries(_) => list_node(theme, &view),
    };
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title_node(theme))
        .child(body)
}

#[cfg(test)]
mod tests {
    use super::{
        ChannelEntry, ChannelListState, ChannelsView, EMPTY, MEMBER_COUNT_TEMPLATE, NOT_DELIVERED,
        TITLE, UNREAD_HINT,
    };

    /// 三条测试条目（频道名是数据标识，非界面文案）。
    fn sample_entries() -> Vec<ChannelEntry> {
        vec![
            ChannelEntry::new("ops-room", 3, 8, true),
            ChannelEntry::new("code-review", 0, 21, false),
            ChannelEntry::new("daily-standup", 12, 6, false),
        ]
    }

    #[test]
    fn counts_and_row_count_are_correct() {
        let view = ChannelsView::build(Some(sample_entries()));
        assert_eq!(view.row_count(), 3);
        assert_eq!(view.total_unread(), 15);
        assert_eq!(view.total_members(), 35);
        assert_eq!(
            view.state
                .entries()
                .iter()
                .filter(|entry| entry.selected)
                .count(),
            1
        );
    }

    #[test]
    fn empty_input_yields_empty_state_not_entries() {
        let view = ChannelsView::build(Some(Vec::new()));
        assert!(matches!(view.state, ChannelListState::Empty));
        assert_eq!(view.row_count(), 0);
    }

    #[test]
    fn undelivered_source_is_not_an_empty_list() {
        // fail-closed：未下发 ≠ 空清单。
        let view = ChannelsView::build(None);
        assert!(matches!(view.state, ChannelListState::NotDelivered));
        assert_eq!(view.row_count(), 0);
    }

    #[test]
    fn texts_are_verbatim_from_collab_namespace() {
        assert_eq!(TITLE, "频道");
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(EMPTY, "暂无可选项");
        assert_eq!(UNREAD_HINT, "有未读动态");
        assert_eq!(MEMBER_COUNT_TEMPLATE, "{count}个成员");
    }

    #[test]
    fn member_count_label_fills_the_placeholder() {
        let entry = ChannelEntry::new("ops-room", 0, 21, false);
        assert_eq!(entry.member_count_label(), "21个成员");
        assert_eq!(
            ChannelEntry::new("x", 0, 0, false).member_count_label(),
            "0个成员"
        );
    }

    #[test]
    fn main_area_builds_without_panic() {
        // GPUI 层门：签名不变，默认主题下可建立。
        #[cfg(feature = "ui")]
        {
            use gpui_kit::component::Theme;
            let theme = Theme::default();
            let _ = super::main_area(&theme);
        }
    }
}
