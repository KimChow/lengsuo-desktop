//! 会话列表浮层（波次 7，卡 `ui-overlay-conversation-list`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间
//! `conversationList` / `sidebar`。数据源在运行时侧：当前以未下发快照渲染，
//! 等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ============ 纯数据层（无 GPUI 依赖） ============

/// 界面文案（逐字取自界面文案全表 `conversationList` / `sidebar` 命名空间）。
pub mod copy {
    /// `conversationList.customMenuDialog.title`
    pub const CUSTOM_MENU_TITLE: &str = "自定义菜单栏";
    /// `conversationList.customMenuDialog.primarySection`
    pub const PRIMARY_SECTION: &str = "一级菜单";
    /// `conversationList.customMenuDialog.contextMenu`
    pub const CONTEXT_MENU: &str = "自定义菜单";
    /// `conversationList.customMenuDialog.dragTip`
    pub const DRAG_TIP: &str = "拖拽可替换位置";
    /// `conversationList.customMenuDialog.fixedTip`（`{name}` 模板）
    pub const FIXED_TIP_TEMPLATE: &str = "{name}菜单不支持拖拽替换位置";
    /// `conversationList.customMenuDialog.moreSection`
    pub const MORE_SECTION: &str = "更多";
    /// `conversationList.customMenuDialog.moreEmpty`
    pub const MORE_EMPTY: &str = "暂无内容，可将不常用的菜单功能移动到\"更多\"下";
    /// `conversationList.newTaskContextMenu.openWindow`
    pub const OPEN_WINDOW: &str = "在独立窗口中打开";
    /// `sidebar.collapse`
    pub const COLLAPSE_SIDEBAR: &str = "收起侧边栏";
    /// `sidebar.mode.toggleToNext`
    pub const TOGGLE_TO_NEXT: &str = "试用新侧栏";
    /// `sidebar.mode.toggleToLegacy`
    pub const TOGGLE_TO_LEGACY: &str = "回到经典侧栏";
}

/// `{name}` 模板替换（`conversationList.customMenuDialog.fixedTip`）。
#[must_use]
pub fn fixed_tip(name: &str) -> String {
    copy::FIXED_TIP_TEMPLATE.replacen("{name}", name, 1)
}

/// 浮层数据下发状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConversationListState {
    /// 已下发会话分组。
    Normal,
    /// 数据源未下发（fail-closed：如实显示「未下发」）。
    #[default]
    NotDelivered,
}

/// 侧栏形态，决定切换按钮文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarMode {
    /// 经典侧栏（默认）。
    #[default]
    Legacy,
    /// 新侧栏。
    Next,
}

impl SidebarMode {
    /// 切换按钮文案（`sidebar.mode.toggleToNext` / `toggleToLegacy`）。
    #[must_use]
    pub fn toggle_label(self) -> &'static str {
        match self {
            SidebarMode::Legacy => copy::TOGGLE_TO_NEXT,
            SidebarMode::Next => copy::TOGGLE_TO_LEGACY,
        }
    }
}

/// 单条会话行：置顶/归档/未读计数均来自数据源字段，渲染层不伪造。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConversationItem {
    pub title: String,
    /// 置顶：组内排前、标题加粗。
    pub pinned: bool,
    /// 已归档：从常规列表行中隐藏。
    pub archived: bool,
    /// 未读数：>0 时渲染徽标。
    pub unread: u32,
}

/// 侧栏分组：分组标题 + 会话行。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConversationGroup {
    pub title: String,
    pub items: Vec<ConversationItem>,
}

impl ConversationGroup {
    /// 组内是否有可展示行（未归档）。
    pub fn has_visible(&self) -> bool {
        self.items.iter().any(|item| !item.archived)
    }

    /// 组内展示行：过滤已归档，置顶排前（稳定排序保持原相对序）。
    pub fn visible_rows(&self) -> Vec<&ConversationItem> {
        let mut rows: Vec<&ConversationItem> =
            self.items.iter().filter(|item| !item.archived).collect();
        rows.sort_by_key(|item| !item.pinned);
        rows
    }
}

/// 会话列表浮层纯数据模型。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConversationListOverlay {
    pub state: ConversationListState,
    pub mode: SidebarMode,
    pub groups: Vec<ConversationGroup>,
}

impl ConversationListOverlay {
    /// 当前快照：数据源未接入 ⇒ 未下发（⛔ 不得伪装成正常空列表）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            state: ConversationListState::NotDelivered,
            ..Self::default()
        }
    }

    /// 会话行总数。
    pub fn total_items(&self) -> usize {
        self.groups.iter().map(|group| group.items.len()).sum()
    }

    /// 置顶行数。
    pub fn pinned_count(&self) -> usize {
        self.items_iter().filter(|item| item.pinned).count()
    }

    /// 已归档行数。
    pub fn archived_count(&self) -> usize {
        self.items_iter().filter(|item| item.archived).count()
    }

    /// 未读总数（徽标聚合口径）。
    pub fn total_unread(&self) -> u32 {
        self.items_iter().map(|item| item.unread).sum()
    }

    fn items_iter(&self) -> impl Iterator<Item = &ConversationItem> {
        self.groups.iter().flat_map(|group| group.items.iter())
    }

    /// 全部展示行：各组未归档行、组序不变、组内置顶排前。
    pub fn visible_rows(&self) -> Vec<&ConversationItem> {
        self.groups
            .iter()
            .flat_map(|group| group.visible_rows())
            .collect()
    }

    /// 无任何会话行 ⇒ 空态文案（空态 ≠ 空列表：另有未下发态区分）。
    pub fn empty_state_text(&self) -> Option<&'static str> {
        (self.total_items() == 0).then_some(copy::MORE_EMPTY)
    }
}

// ============ GPUI 层 ============

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
///
/// 无数据源：以 [`ConversationListOverlay::not_delivered`] 快照渲染 ——
/// 「更多」分区如实显示空态 + 状态行「未下发」，⛔ 不伪造会话行/未读数。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&ConversationListOverlay::not_delivered(), theme)
}

/// 视图 = 状态的纯函数。
fn render(overlay: &ConversationListOverlay, theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header_row(theme))
        .child(section_label(copy::PRIMARY_SECTION, theme))
        .child(menu_row(copy::CONTEXT_MENU, theme))
        .child(menu_row(copy::OPEN_WINDOW, theme))
        .child(tip_row(copy::DRAG_TIP, theme))
        .child(section_label(copy::MORE_SECTION, theme))
        .child(content_area(overlay, theme))
        .children(state_line(overlay, theme))
        .child(footer_row(overlay.mode, theme))
}

/// 标题行：自定义菜单栏 + 收起侧边栏。
fn header_row(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(copy::CUSTOM_MENU_TITLE),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(copy::COLLAPSE_SIDEBAR),
        )
}

/// 分区标题（一级菜单 / 更多）。
fn section_label(label: &'static str, theme: &Theme) -> Div {
    div()
        .px_4()
        .pt_2()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(label)
}

/// 菜单行。
fn menu_row(label: &'static str, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .px_4()
        .py_1()
        .text_sm()
        .text_color(theme.foreground)
        .child(label)
}

/// 弱化提示行（拖拽可替换位置）。
fn tip_row(label: &'static str, theme: &Theme) -> Div {
    div()
        .px_4()
        .py_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(label)
}

/// 内容区：有数据 ⇒ 侧栏分组标题 + 列表行；无数据 ⇒ 空态（不是空列表）。
fn content_area(overlay: &ConversationListOverlay, theme: &Theme) -> Div {
    if overlay.visible_rows().is_empty() {
        return empty_state_row(theme);
    }
    overlay
        .groups
        .iter()
        .filter(|group| group.has_visible())
        .fold(div().flex().flex_col().gap_1(), |list, group| {
            list.child(group_section(group, theme))
        })
}

/// 空态（`conversationList.customMenuDialog.moreEmpty`）：居中弱化说明。
fn empty_state_row(theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .px_4()
        .py_6()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(copy::MORE_EMPTY)
}

/// 侧栏分组：分组标题 + 组内列表行。
fn group_section(group: &ConversationGroup, theme: &Theme) -> Div {
    let section = div().flex().flex_col().gap_1().child(
        div()
            .px_4()
            .pt_2()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(SharedString::from(group.title.as_str())),
    );
    group.visible_rows().into_iter().fold(section, |acc, item| {
        acc.child(conversation_row(item, theme))
    })
}

/// 列表行：归档弱化、置顶加粗、未读 >0 挂徽标。
fn conversation_row(item: &ConversationItem, theme: &Theme) -> Div {
    let color = if item.archived {
        theme.muted_foreground
    } else {
        theme.foreground
    };
    let row = div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_1()
        .text_sm()
        .text_color(color);
    let row = if item.pinned { row.font_bold() } else { row };
    let row = row.child(SharedString::from(item.title.as_str()));
    if item.unread > 0 {
        row.child(unread_badge(item.unread, theme))
    } else {
        row
    }
}

/// 未读徽标：只渲染数据源给的数值，⛔ 不编造。
fn unread_badge(unread: u32, theme: &Theme) -> Div {
    div()
        .flex_shrink_0()
        .px_1()
        .rounded_sm()
        .bg(theme.accent)
        .text_xs()
        .text_color(theme.foreground)
        .child(SharedString::from(unread.to_string()))
}

/// 状态行：未下发时的 fail-closed 说明。
fn state_line(overlay: &ConversationListOverlay, theme: &Theme) -> Option<Div> {
    (overlay.state == ConversationListState::NotDelivered).then(|| {
        div()
            .flex()
            .items_center()
            .justify_center()
            .py_2()
            .border_t_1()
            .border_color(theme.border)
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED)
    })
}

/// 底部动作行：侧栏形态切换。
fn footer_row(mode: SidebarMode, theme: &Theme) -> Div {
    div()
        .mt_auto()
        .flex()
        .items_center()
        .justify_end()
        .px_4()
        .py_2()
        .border_t_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.foreground)
        .child(mode.toggle_label())
}
#[cfg(test)]
mod tests {
    use super::{copy, main_area, NOT_DELIVERED};
    use gpui_kit::component::Theme;
    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert_eq!(copy::CUSTOM_MENU_TITLE, "自定义菜单栏");
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
