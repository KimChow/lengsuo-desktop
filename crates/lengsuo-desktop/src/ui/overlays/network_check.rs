//! 网络检测浮层（波次 6，卡 `ui-overlay-network-check`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `networkCheck`。
//! 数据源在运行时侧：无源时如实显示「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ─────────── 纯数据层（无 GPUI 依赖） ───────────

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "网络检测";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 检测进行中的状态行（逐字）。
pub const CHECKING: &str = "检测中...";
/// 已下发但清单为空时的空态说明（逐字；是空态，不是空清单）。
pub const EMPTY_HINT: &str = "正在收集代理、Hosts、服务、TCP 和丢包检测结果…";
/// 检测项区块标题（逐字）。
pub const CHECKS_TITLE: &str = "检测项";

/// 徽标状态；文案逐字取自 `badge` 子命名空间。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Badge {
    /// 失败
    Error,
    /// 等待中
    Pending,
    /// 检测中
    Running,
    /// 通过
    Success,
    /// 不支持
    Unsupported,
    /// 警告
    Warning,
}

impl Badge {
    /// 徽标中文文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "失败",
            Self::Pending => "等待中",
            Self::Running => "检测中",
            Self::Success => "通过",
            Self::Unsupported => "不支持",
            Self::Warning => "警告",
        }
    }

    /// 仍在进行（检测中/等待中）⇒ 清单顶部展示状态行。
    #[must_use]
    pub fn in_progress(self) -> bool {
        matches!(self, Self::Running | Self::Pending)
    }
}

/// 检测项条目（标题/说明逐字）。
#[derive(Debug)]
pub struct CheckItem {
    /// 条目标题（逐字）。
    pub title: &'static str,
    /// 条目说明（逐字）。
    pub description: &'static str,
}

/// 检测项目录（固定 5 项，逐字）；仅作清单骨架，⛔ 不附带伪造数值。
pub const CHECK_ITEMS: &[CheckItem] = &[
    CheckItem {
        title: "Hosts 解析",
        description: "解析本地 Hosts 文件并与当前系统解析结果进行对比。",
    },
    CheckItem {
        title: "丢包率",
        description: "测量到当前目标主机的 ICMP 丢包率作为辅助信号。",
    },
    CheckItem {
        title: "代理检测",
        description: "检测有效的代理配置并验证其可达性。",
    },
    CheckItem {
        title: "服务连通性",
        description: "验证服务端点在当前运行时设置下是否可达。",
    },
    CheckItem {
        title: "TCP 连接延迟",
        description: "测量与当前目标主机的 TCP 连接延迟。",
    },
];

/// 检测项清单行 = 目录条目 + 徽标状态。
#[derive(Debug)]
pub struct CheckRow {
    /// 目录条目。
    pub item: &'static CheckItem,
    /// 该项徽标状态。
    pub badge: Badge,
}

/// 浮层数据三态：未下发 ≠ 空态 ≠ 清单。
#[derive(Debug)]
pub enum OverlayState {
    /// 数据源未下发（fail-closed）。
    NotDelivered,
    /// 已下发但检测项清单为空（空态，不是空清单）。
    Empty,
    /// 检测项清单行。
    Checks(Vec<CheckRow>),
}

impl OverlayState {
    /// 无数据源时的默认态（fail-closed）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self::NotDelivered
    }

    /// 按逐项状态构造清单；空输入归一化为空态（⛔ 不是空清单）。
    #[must_use]
    pub fn from_badges(badges: &[Badge]) -> Self {
        if badges.is_empty() {
            Self::Empty
        } else {
            Self::Checks(
                badges
                    .iter()
                    .zip(CHECK_ITEMS)
                    .map(|(&badge, item)| CheckRow { item, badge })
                    .collect(),
            )
        }
    }

    /// 清单行数（未下发与空态均为 0）。
    #[must_use]
    pub fn row_count(&self) -> usize {
        match self {
            Self::Checks(rows) => rows.len(),
            _ => 0,
        }
    }
}

/// 汇总徽标：失败 > 检测中 > 等待中 > 警告 > 不支持 > 通过。
#[must_use]
pub fn overall_badge(rows: &[CheckRow]) -> Badge {
    let any = |b: Badge| rows.iter().any(|row| row.badge == b);
    if any(Badge::Error) {
        Badge::Error
    } else if any(Badge::Running) {
        Badge::Running
    } else if any(Badge::Pending) {
        Badge::Pending
    } else if any(Badge::Warning) {
        Badge::Warning
    } else if any(Badge::Unsupported) {
        Badge::Unsupported
    } else {
        Badge::Success
    }
}

// ─────────── GPUI 层 ───────────

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
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
    use super::{
        main_area, overall_badge, Badge, CHECK_ITEMS, CheckRow, OverlayState, NOT_DELIVERED,
        TITLE,
    };
    use gpui_kit::component::Theme;
    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }
    #[test]
    fn from_badges_builds_rows_and_counts() {
        let state = OverlayState::from_badges(&[
            Badge::Success,
            Badge::Running,
            Badge::Pending,
            Badge::Error,
            Badge::Warning,
        ]);
        assert_eq!(state.row_count(), CHECK_ITEMS.len());
        let OverlayState::Checks(rows) = &state else {
            panic!("应构造出清单态");
        };
        assert_eq!(rows[0].badge, Badge::Success);
        assert_eq!(rows[3].badge, Badge::Error);
        assert_eq!(rows[2].item.title, "服务连通性");
    }
    #[test]
    fn empty_input_normalizes_to_empty_state_not_checks() {
        let state = OverlayState::from_badges(&[]);
        assert!(matches!(state, OverlayState::Empty));
        assert_eq!(state.row_count(), 0);
        assert!(!matches!(state, OverlayState::NotDelivered));
    }
    #[test]
    fn default_is_fail_closed_not_delivered() {
        assert!(matches!(
            OverlayState::not_delivered(),
            OverlayState::NotDelivered
        ));
    }
    #[test]
    fn overall_badge_precedence() {
        let rows = |badges: &[Badge]| -> Vec<CheckRow> {
            badges
                .iter()
                .zip(CHECK_ITEMS)
                .map(|(&badge, item)| CheckRow { item, badge })
                .collect()
        };
        assert_eq!(
            overall_badge(&rows(&[Badge::Success, Badge::Error])),
            Badge::Error
        );
        assert_eq!(
            overall_badge(&rows(&[Badge::Success, Badge::Running])),
            Badge::Running
        );
        assert_eq!(
            overall_badge(&rows(&[Badge::Pending, Badge::Success])),
            Badge::Pending
        );
        assert_eq!(
            overall_badge(&rows(&[Badge::Warning, Badge::Success])),
            Badge::Warning
        );
        assert_eq!(
            overall_badge(&rows(&[Badge::Unsupported, Badge::Success])),
            Badge::Unsupported
        );
        assert_eq!(overall_badge(&rows(&[Badge::Success; 5])), Badge::Success);
    }
    #[test]
    fn badge_labels_verbatim() {
        assert_eq!(Badge::Error.label(), "失败");
        assert_eq!(Badge::Pending.label(), "等待中");
        assert_eq!(Badge::Running.label(), "检测中");
        assert_eq!(Badge::Success.label(), "通过");
        assert_eq!(Badge::Unsupported.label(), "不支持");
        assert_eq!(Badge::Warning.label(), "警告");
    }
    #[test]
    fn catalog_items_verbatim() {
        assert_eq!(CHECK_ITEMS.len(), 5);
        assert_eq!(CHECK_ITEMS[0].title, "Hosts 解析");
        assert_eq!(
            CHECK_ITEMS[0].description,
            "解析本地 Hosts 文件并与当前系统解析结果进行对比。"
        );
        assert_eq!(CHECK_ITEMS[1].title, "丢包率");
        assert_eq!(CHECK_ITEMS[2].description, "检测有效的代理配置并验证其可达性。");
        assert_eq!(CHECK_ITEMS[4].title, "TCP 连接延迟");
    }
    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
