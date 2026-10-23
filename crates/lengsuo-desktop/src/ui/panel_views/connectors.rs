//! 连接器主区视图（波次 5，卡 `ui-connector-center`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：条目 / 空态 / 诚实占位文案；
//! - **GPUI 层**（[`main_area`]）：把描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自文案全表命名空间 `connectors`，
//!   空态文案取自同表 `automation.connectorPicker.empty`）。
//!
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后由
//! `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ===================== 纯数据层（无 GPUI 依赖） =====================

/// 视图标题（`connectors.title`，逐字）。
pub const TITLE: &str = "连接器";

/// 标题行副标题（`connectors.tipInfo`，逐字）。
pub const TIP_INFO: &str = "连接第三方应用";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 已下发但清单为空的空态文案（`automation.connectorPicker.empty`，逐字）。
pub const EMPTY_HINT: &str = "暂无可选连接器";

/// 分组标题：可用连接器（`connectors.available`，逐字）。
pub const GROUP_AVAILABLE: &str = "可用连接器";

/// 分组标题：危险操作（`connectors.dangerZone`，逐字）。
pub const GROUP_DANGER: &str = "危险操作";

/// 授权状态：已连接（`connectors.connected`，逐字）。
pub const STATUS_CONNECTED: &str = "已连接";

/// 授权状态：未连接（`connectors.disconnected`，逐字）。
pub const STATUS_DISCONNECTED: &str = "未连接";

/// 未连接条目的动作文案（`connectors.authorizeButtonText`，逐字）。
pub const ACTION_AUTHORIZE: &str = "授权";

/// 危险区动作文案（`connectors.revokeAccess`，逐字）。
pub const ACTION_REVOKE: &str = "撤销授权";

/// 危险区确认说明（`connectors.revokeAccessDesc`，逐字）。
pub const REVOKE_DESC: &str = "确定要撤销此连接器的授权吗？";

/// 连接器目录条目：名称 + 一句话说明（文案全表内置，非服务端数据）。
pub struct CatalogEntry {
    /// 连接器名称（逐字）。
    pub name: &'static str,
    /// 一句话说明（逐字）。
    pub desc: &'static str,
}

/// 界面内置目录（渲染包自带的连接器清单文案；**授权状态**才是运行时数据）。
pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        name: "CNB",
        desc: "连接 CNB，访问内部代码仓库与相关资源。",
    },
    CatalogEntry {
        name: "GitHub",
        desc: "连接 GitHub，访问仓库、Issues 与 Pull Requests。",
    },
    CatalogEntry {
        name: "Figma",
        desc: "连接 Figma，访问设计文件与组件资源。",
    },
    CatalogEntry {
        name: "工蜂 Git",
        desc: "连接工蜂，访问企业代码仓库与项目协作数据。",
    },
];

/// 一行连接器：目录条目 + 授权状态。
pub struct ConnectorRow {
    /// 连接器名称。
    pub name: &'static str,
    /// 一句话说明。
    pub desc: &'static str,
    /// 是否已连接（运行时下发，界面不自行推断）。
    pub connected: bool,
}

impl ConnectorRow {
    /// 状态文案。
    #[must_use]
    pub fn status_text(&self) -> &'static str {
        if self.connected {
            STATUS_CONNECTED
        } else {
            STATUS_DISCONNECTED
        }
    }

    /// 行内动作文案（已连接条目的撤销动作在危险区，不在行内）。
    #[must_use]
    pub fn action_text(&self) -> Option<&'static str> {
        if self.connected {
            None
        } else {
            Some(ACTION_AUTHORIZE)
        }
    }
}

/// 连接器主区的可枚举状态。
pub enum ConnectorCenter {
    /// 数据未下发：fail-closed，如实显示「未下发」。
    NotDelivered,
    /// 已下发但无条目：空态（不是空清单）。
    Empty,
    /// 已下发：条目 + 授权状态。
    Entries(Vec<ConnectorRow>),
}

/// 结构节点：元素树的纯数据投影（GPUI 层与测试共用同一结构口径）。
#[derive(Debug, PartialEq)]
pub enum Node {
    /// 标题行节点。
    Title,
    /// 分组标题节点。
    Group(&'static str),
    /// 清单条目节点。
    Entry {
        /// 连接器名称。
        name: &'static str,
        /// 状态文案。
        status: &'static str,
        /// 行内动作文案（已连接时无）。
        action: Option<&'static str>,
    },
    /// 危险区条目节点（已连接条目才有）。
    Danger {
        /// 连接器名称。
        name: &'static str,
    },
    /// 居中提示节点（未下发 / 空态）。
    Hint(&'static str),
}

impl ConnectorCenter {
    /// 构造视图状态：`None` = 数据未下发（fail-closed）；
    /// `Some(空)` = 已下发但无条目（空态）；`Some(状态表)` = 按内置目录逐项出条目
    /// （状态表与目录按位对齐，短表只出前 N 项）。
    #[must_use]
    pub fn build(statuses: Option<&[bool]>) -> Self {
        match statuses {
            None => Self::NotDelivered,
            Some([]) => Self::Empty,
            Some(list) => Self::Entries(
                CATALOG
                    .iter()
                    .zip(list)
                    .map(|(entry, connected)| ConnectorRow {
                        name: entry.name,
                        desc: entry.desc,
                        connected: *connected,
                    })
                    .collect(),
            ),
        }
    }

    /// 结构节点序列（恒以 [`Node::Title`] 打头）。
    #[must_use]
    pub fn nodes(&self) -> Vec<Node> {
        let mut out = vec![Node::Title];
        match self {
            Self::NotDelivered => out.push(Node::Hint(NOT_DELIVERED)),
            Self::Empty => out.push(Node::Hint(EMPTY_HINT)),
            Self::Entries(rows) => {
                out.push(Node::Group(GROUP_AVAILABLE));
                for row in rows {
                    out.push(Node::Entry {
                        name: row.name,
                        status: row.status_text(),
                        action: row.action_text(),
                    });
                }
                if rows.iter().any(|row| row.connected) {
                    out.push(Node::Group(GROUP_DANGER));
                    for row in rows.iter().filter(|row| row.connected) {
                        out.push(Node::Danger { name: row.name });
                    }
                }
            }
        }
        out
    }
}

// ===================== GPUI 层 =====================

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
///
/// 数据源未接线：当前固定 fail-closed（如实显示「未下发」）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &ConnectorCenter::NotDelivered)
}

/// 按给定状态渲染（接线卡以真实状态调用；GPUI 层逐字读纯数据层文案）。
#[must_use]
pub fn render(theme: &Theme, state: &ConnectorCenter) -> Div {
    let body = match state {
        ConnectorCenter::NotDelivered => centered_hint(theme, NOT_DELIVERED),
        ConnectorCenter::Empty => centered_hint(theme, EMPTY_HINT),
        ConnectorCenter::Entries(rows) => entries_body(theme, rows),
    };
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title_row(theme))
        .child(body)
}

/// 标题行：标题 + 副标题。
fn title_row(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(TIP_INFO),
        )
}

/// 居中提示（未下发 / 空态）。
fn centered_hint(theme: &Theme, text: &'static str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 清单主体：可用连接器分组 + 行，已连接条目再进危险区分组。
fn entries_body(theme: &Theme, rows: &[ConnectorRow]) -> Div {
    let mut list = div()
        .flex_1()
        .flex()
        .flex_col()
        .child(group_header(theme, GROUP_AVAILABLE));
    for row in rows {
        list = list.child(entry_row(theme, row));
    }
    let connected: Vec<&ConnectorRow> = rows.iter().filter(|row| row.connected).collect();
    if !connected.is_empty() {
        list = list.child(group_header(theme, GROUP_DANGER));
        for row in connected {
            list = list.child(danger_row(theme, row));
        }
    }
    list
}

/// 分组标题行。
fn group_header(theme: &Theme, text: &'static str) -> Div {
    div()
        .px_4()
        .py_2()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(text)
}

/// 清单行：名称 + 说明在左，状态 + 动作在右。
fn entry_row(theme: &Theme, row: &ConnectorRow) -> Div {
    let mut actions = div()
        .flex()
        .items_center()
        .gap_2()
        .child(status_text(theme, row));
    if let Some(action) = row.action_text() {
        actions = actions.child(action_chip(theme, action));
    }
    div()
        .px_4()
        .py_2()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(row.name),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(row.desc),
                ),
        )
        .child(actions)
}

/// 状态文案：已连接用强调色，未连接用弱化色。
fn status_text(theme: &Theme, row: &ConnectorRow) -> Div {
    let color = if row.connected {
        theme.accent
    } else {
        theme.muted_foreground
    };
    div()
        .px_2()
        .py_1()
        .text_sm()
        .text_color(color)
        .child(row.status_text())
}

/// 动作位（描边小片；行为由接线卡接，这里只做展示）。
fn action_chip(theme: &Theme, text: &'static str) -> Div {
    div()
        .px_2()
        .py_1()
        .text_sm()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_color(theme.foreground)
        .child(text)
}

/// 危险区行：名称 + 确认说明 + 撤销动作位。
fn danger_row(theme: &Theme, row: &ConnectorRow) -> Div {
    div()
        .px_4()
        .py_2()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(row.name),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(REVOKE_DESC),
                ),
        )
        .child(action_chip(theme, ACTION_REVOKE))
}

#[cfg(test)]
mod tests {
    use super::{
        main_area, render, ConnectorCenter, Node, ACTION_AUTHORIZE, ACTION_REVOKE, CATALOG,
        EMPTY_HINT, GROUP_AVAILABLE, GROUP_DANGER, NOT_DELIVERED, REVOKE_DESC, STATUS_CONNECTED,
        STATUS_DISCONNECTED, TIP_INFO, TITLE,
    };
    use gpui_kit::component::Theme;

    /// 文案逐字一致（对 ≥3 个 key 断言）。
    #[test]
    fn copy_is_verbatim_from_the_table() {
        assert_eq!(TITLE, "连接器");
        assert_eq!(TIP_INFO, "连接第三方应用");
        assert_eq!(GROUP_AVAILABLE, "可用连接器");
        assert_eq!(GROUP_DANGER, "危险操作");
        assert_eq!(STATUS_CONNECTED, "已连接");
        assert_eq!(STATUS_DISCONNECTED, "未连接");
        assert_eq!(ACTION_AUTHORIZE, "授权");
        assert_eq!(ACTION_REVOKE, "撤销授权");
        assert_eq!(REVOKE_DESC, "确定要撤销此连接器的授权吗？");
        assert_eq!(EMPTY_HINT, "暂无可选连接器");
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    /// 无数据源 ⇒ fail-closed：标题 + 「未下发」提示，不得退化为空清单。
    #[test]
    fn none_input_is_fail_closed_not_delivered() {
        let nodes = ConnectorCenter::build(None).nodes();
        assert_eq!(nodes, vec![Node::Title, Node::Hint(NOT_DELIVERED)]);
    }

    /// 空输入 ⇒ 空态提示（不是空列表）。
    #[test]
    fn empty_input_renders_empty_state_not_blank_list() {
        let nodes = ConnectorCenter::build(Some(&[])).nodes();
        assert_eq!(nodes, vec![Node::Title, Node::Hint(EMPTY_HINT)]);
        assert!(!nodes.iter().any(|node| matches!(node, Node::Group(_))));
    }

    /// render 各状态分支均不 panic（未下发 / 空态 / 清单）。
    #[test]
    fn render_all_states_without_panic() {
        let theme = Theme::default();
        let _ = render(&theme, &ConnectorCenter::build(None));
        let _ = render(&theme, &ConnectorCenter::build(Some(&[])));
        let _ = render(
            &theme,
            &ConnectorCenter::build(Some(&[true, false, false, false])),
        );
    }

    /// 元素树结构：标题节点 + ≥3 个清单/分组节点；分组与计数正确。
    #[test]
    fn entries_tree_has_title_and_at_least_three_list_nodes() {
        let nodes = ConnectorCenter::build(Some(&[true, false, false, false])).nodes();
        assert_eq!(nodes.first(), Some(&Node::Title));
        let groups: Vec<&'static str> = nodes
            .iter()
            .filter_map(|node| match node {
                Node::Group(text) => Some(*text),
                _ => None,
            })
            .collect();
        assert_eq!(groups, vec![GROUP_AVAILABLE, GROUP_DANGER]);
        let entries: Vec<&'static str> = nodes
            .iter()
            .filter_map(|node| match node {
                Node::Entry { name, .. } => Some(*name),
                _ => None,
            })
            .collect();
        assert_eq!(entries.len(), CATALOG.len());
        assert!(entries.len() >= 3);
        assert_eq!(entries, vec!["CNB", "GitHub", "Figma", "工蜂 Git"]);
        assert!(nodes.contains(&Node::Danger { name: "CNB" }));
    }

    /// 状态/动作随授权状态走：未连接 → 未连接 + 授权；已连接 → 无行内动作、进危险区撤销。
    #[test]
    fn status_and_action_follow_connection_state() {
        let nodes = ConnectorCenter::build(Some(&[false, true])).nodes();
        assert!(nodes.contains(&Node::Entry {
            name: "CNB",
            status: STATUS_DISCONNECTED,
            action: Some(ACTION_AUTHORIZE),
        }));
        assert!(nodes.contains(&Node::Entry {
            name: "GitHub",
            status: STATUS_CONNECTED,
            action: None,
        }));
        assert!(nodes.contains(&Node::Danger { name: "GitHub" }));
        assert!(!nodes.contains(&Node::Danger { name: "CNB" }));
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
