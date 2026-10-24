//! 分享选择浮层（波次 6，卡 `ui-overlay-share-selection`，对齐 ui-parity-spec §8 A39）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `shareSelection`
//! （品牌词键 ⛔ 不用）。数据源在运行时侧：[`ShareSelectionState::initial`] 恒为未下发。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "分享选择";
/// 渠道组标题（`shareSelection.channelGroupLabel`）。
pub const CHANNEL_GROUP_LABEL: &str = "选择分享方式";
/// 退出按钮文案（`shareSelection.close`）。
pub const CLOSE: &str = "退出分享";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 最多可分享对话条数提示模板（`shareSelection.maxPairsTooltip`，占位符 `{count}`）。
pub const MAX_PAIRS_TOOLTIP: &str = "最多可选择 {count} 条对话进行分享";
/// 工具渠道行（`shareSelection.channel.*`，按全表键序）：复制链接 / 浏览器打开 / 生成二维码。
pub const TOOL_CHANNELS: &[&str] = &["复制链接", "浏览器打开", "生成二维码"];
/// 平台渠道行（`shareSelection.channel.*`，按全表键序）。
pub const PLATFORM_CHANNELS: &[&str] = &[
    "Discord",
    "Facebook",
    "朋友圈",
    "Slack",
    "Telegram",
    "X",
    "微信",
];

/// 浮层可枚举状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShareSelectionState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空集）。
    NotDelivered,
    /// 数据已下发：带文案模板行的正常态。
    Ready(SharePayload),
}

/// 下发载荷（纯数据，Ⓐ 不持凭据、不判权限）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharePayload {
    /// 文案模板行（运行时下发；空 ⇒ 空态，不是空列表）。
    pub templates: Vec<String>,
    /// 最多可分享对话条数（驱动计数提示）。
    pub max_pairs: usize,
}

/// 把模板里的 `{count}` 占位符替换成数值。
fn fill_count(tmpl: &str, count: usize) -> String {
    tmpl.replace("{count}", &count.to_string())
}

impl ShareSelectionState {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self::NotDelivered
    }

    /// 是否处于未下发态。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self, Self::NotDelivered)
    }

    /// 文案模板行：已下发且非空才有；否则调用方如实显示「未下发」。
    pub fn template_rows(&self) -> Option<&[String]> {
        match self {
            Self::NotDelivered => None,
            Self::Ready(payload) if payload.templates.is_empty() => None,
            Self::Ready(payload) => Some(&payload.templates),
        }
    }

    /// 计数提示（`maxPairsTooltip` 填 `{count}`）；未下发 ⇒ None（⛔ 不伪造数值）。
    pub fn max_pairs_hint(&self) -> Option<String> {
        match self {
            Self::NotDelivered => None,
            Self::Ready(payload) => Some(fill_count(MAX_PAIRS_TOOLTIP, payload.max_pairs)),
        }
    }
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
    use super::{main_area, NOT_DELIVERED, TITLE};
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
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
