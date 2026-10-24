//! 环境自检浮层（波次 6，卡 `ui-overlay-self-check`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `selfCheck`。
//! 数据源在运行时侧：[`SelfCheckModel::not_delivered`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 层标题（逐字取自 `selfCheck.title`）。
pub const TITLE: &str = "诊断工具";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空清单）。
pub const NOT_DELIVERED: &str = "未下发";
/// 副标题（逐字取自 `selfCheck.subtitle.*`）。
pub const SUBTITLE_IDLE: &str = "点击“开始检测”执行环境诊断";
/// 检测进行中副标题。
pub const SUBTITLE_RUNNING: &str = "正在检测...";
/// 检测完成副标题。
pub const SUBTITLE_DONE: &str = "检测已完成";
/// 徽标计数模板（`selfCheck.badge.count*`，占位符 `{count}`）。
pub const BADGE_COUNT_PASS: &str = "{count} 通过";
pub const BADGE_COUNT_WARN: &str = "{count} 警告";
pub const BADGE_COUNT_FAIL: &str = "{count} 失败";
/// 总项数模板。
pub const BADGE_COUNT_ITEMS: &str = "{count} 项";
/// 单项结论标签（`selfCheck.status.*`）。
pub const STATUS_PASS: &str = "通过";
pub const STATUS_WARNING: &str = "警告";
pub const STATUS_FAIL: &str = "失败";
pub const STATUS_PENDING: &str = "待检测";
/// 分类标签（`selfCheck.category.*`）。
pub const CATEGORY_ENVIRONMENT: &str = "环境信息";
pub const CATEGORY_NETWORK: &str = "网络";
pub const CATEGORY_SECURITY: &str = "安全软件";
pub const CATEGORY_SYSTEM: &str = "系统资源";
/// FPS/内存/CPU 说明行（`selfCheck.fps.*`）。
pub const FPS_TITLE: &str = "性能指标";
pub const FPS_DESCRIPTION: &str = "在右下角显示 FPS、内存和 CPU";
pub const FPS_HINT: &str = "FPS 为当前窗口近 1 秒平均值";
/// 按钮文案（`selfCheck.start` / `selfCheck.rerun` / `selfCheck.close`）。
pub const START: &str = "开始检测";
pub const RERUN: &str = "重新检测";
pub const CLOSE: &str = "关闭";

/// 单项检测结论。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemStatus {
    /// 待检测（未开始时的诚实初值）。
    Pending,
    /// 通过。
    Pass,
    /// 警告。
    Warning,
    /// 失败。
    Fail,
}

impl ItemStatus {
    /// 结论标签（逐字取自 `selfCheck.status.*`）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => STATUS_PENDING,
            Self::Pass => STATUS_PASS,
            Self::Warning => STATUS_WARNING,
            Self::Fail => STATUS_FAIL,
        }
    }
}

/// 一条分类检测行。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SelfCheckItem {
    /// 分类标签（逐字取自 `selfCheck.category.*`）。
    pub category: &'static str,
    /// 结论。
    pub status: ItemStatus,
}

/// 徽标计数（通过/警告/失败 + 项数）。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct BadgeCounts {
    /// 通过数。
    pub pass: usize,
    /// 警告数。
    pub warn: usize,
    /// 失败数。
    pub fail: usize,
    /// 总项数（含待检测）。
    pub items: usize,
}

impl BadgeCounts {
    /// 从清单行计数（空清单 ⇒ 全 0）。
    pub fn from_items(items: &[SelfCheckItem]) -> Self {
        let mut counts = Self {
            items: items.len(),
            ..Self::default()
        };
        for item in items {
            match item.status {
                ItemStatus::Pass => counts.pass += 1,
                ItemStatus::Warning => counts.warn += 1,
                ItemStatus::Fail => counts.fail += 1,
                ItemStatus::Pending => {}
            }
        }
        counts
    }

    /// 「{count} 通过」。
    pub fn label_pass(self) -> String {
        fill_count(BADGE_COUNT_PASS, self.pass)
    }

    /// 「{count} 警告」。
    pub fn label_warn(self) -> String {
        fill_count(BADGE_COUNT_WARN, self.warn)
    }

    /// 「{count} 失败」。
    pub fn label_fail(self) -> String {
        fill_count(BADGE_COUNT_FAIL, self.fail)
    }

    /// 「{count} 项」。
    pub fn label_items(self) -> String {
        fill_count(BADGE_COUNT_ITEMS, self.items)
    }
}

/// 浮层状态机。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SelfCheckState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空清单）。
    NotDelivered,
    /// 已下发，未开始检测。
    Idle,
    /// 检测进行中。
    Running,
    /// 检测完成。
    Done,
}

/// 浮层模型（纯数据，Ⓐ 不持凭据、不判权限）。
pub struct SelfCheckModel {
    /// 状态机。
    pub state: SelfCheckState,
    /// 分类检测行。
    pub items: Vec<SelfCheckItem>,
}

impl SelfCheckModel {
    /// 数据源未接线时的诚实初始态。
    pub fn not_delivered() -> Self {
        Self {
            state: SelfCheckState::NotDelivered,
            items: Vec::new(),
        }
    }

    /// 是否未下发。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self.state, SelfCheckState::NotDelivered)
    }

    /// 是否渲染内容分支（徽标/清单行/FPS 行）；空清单 ⇒ 空态，不渲染空列表。
    pub fn show_content(&self) -> bool {
        !self.is_not_delivered() && !self.items.is_empty()
    }

    /// 副标题（未下发时即中央「未下发」行）。
    pub fn subtitle(&self) -> String {
        match self.state {
            SelfCheckState::NotDelivered => NOT_DELIVERED.to_string(),
            SelfCheckState::Idle => SUBTITLE_IDLE.to_string(),
            SelfCheckState::Running => SUBTITLE_RUNNING.to_string(),
            SelfCheckState::Done => SUBTITLE_DONE.to_string(),
        }
    }

    /// 徽标计数。
    pub fn badges(&self) -> BadgeCounts {
        BadgeCounts::from_items(&self.items)
    }
}

/// 把模板里的 `{count}` 占位符替换成数值。
fn fill_count(tmpl: &str, count: usize) -> String {
    tmpl.replace("{count}", &count.to_string())
}

// ---------- GPUI 层 ----------

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
        main_area, BadgeCounts, ItemStatus, SelfCheckItem, SelfCheckModel, SelfCheckState,
        CATEGORY_ENVIRONMENT, CATEGORY_NETWORK, CATEGORY_SECURITY, CATEGORY_SYSTEM,
        NOT_DELIVERED, SUBTITLE_DONE, SUBTITLE_IDLE, SUBTITLE_RUNNING,
    };
    use gpui_kit::component::Theme;

    fn sample_items() -> Vec<SelfCheckItem> {
        vec![
            SelfCheckItem {
                category: CATEGORY_ENVIRONMENT,
                status: ItemStatus::Pass,
            },
            SelfCheckItem {
                category: CATEGORY_NETWORK,
                status: ItemStatus::Pending,
            },
            SelfCheckItem {
                category: CATEGORY_SECURITY,
                status: ItemStatus::Warning,
            },
            SelfCheckItem {
                category: CATEGORY_SYSTEM,
                status: ItemStatus::Fail,
            },
        ]
    }

    #[test]
    fn badge_counts_and_labels_from_items() {
        let counts = BadgeCounts::from_items(&sample_items());
        assert_eq!(
            counts,
            BadgeCounts {
                pass: 1,
                warn: 1,
                fail: 1,
                items: 4
            }
        );
        assert_eq!(counts.label_pass(), "1 通过");
        assert_eq!(counts.label_warn(), "1 警告");
        assert_eq!(counts.label_fail(), "1 失败");
        assert_eq!(counts.label_items(), "4 项");
    }

    #[test]
    fn empty_items_is_empty_state_not_blank_list() {
        let model = SelfCheckModel {
            state: SelfCheckState::Idle,
            items: Vec::new(),
        };
        assert!(!model.show_content());
        assert_eq!(model.subtitle(), SUBTITLE_IDLE);
        assert_eq!(model.badges(), BadgeCounts::default());
    }

    #[test]
    fn not_delivered_is_fail_closed() {
        let model = SelfCheckModel::not_delivered();
        assert!(model.is_not_delivered());
        assert!(!model.show_content());
        assert_eq!(model.subtitle(), NOT_DELIVERED);
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn subtitle_tracks_state() {
        let mut model = SelfCheckModel {
            state: SelfCheckState::Idle,
            items: sample_items(),
        };
        assert_eq!(model.subtitle(), SUBTITLE_IDLE);
        model.state = SelfCheckState::Running;
        assert_eq!(model.subtitle(), SUBTITLE_RUNNING);
        model.state = SelfCheckState::Done;
        assert_eq!(model.subtitle(), SUBTITLE_DONE);
        assert!(model.show_content());
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
