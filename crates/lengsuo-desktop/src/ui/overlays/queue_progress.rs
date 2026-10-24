//! 队列与进度浮层（波次 7，卡 `ui-overlay-queue-progress`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `queueBanner + progress`。
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "队列与进度";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

// ── 纯数据层（无 GPUI 依赖；文案逐字取自 `queueBanner` / `progress` 命名空间）──

/// 排队横幅的升级档位（key + 中文文案）。
pub struct UpgradeTier {
    pub key: &'static str,
    pub label: &'static str,
}

/// 进度阶段条目（key + 中文文案）。
pub struct ProgressPhase {
    pub key: &'static str,
    pub label: &'static str,
}

/// 浮层状态（可枚举；渲染按状态走 fail-closed 分支）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayState {
    /// 已下发进度条目。
    Normal,
    /// 等待下发。
    Loading,
    /// 下发失败（等同未下发处理，不伪造内容）。
    Failed,
    /// 服务端未下发（fail-closed 默认态）。
    NotDelivered,
}

/// 排队横幅升级档位目录（`queueBanner.upgrade.*`，3 档由低到高）。
pub const BANNER_TIERS: [UpgradeTier; 3] = [
    UpgradeTier {
        key: "queueBanner.upgrade.toPro",
        label: "升级标准版",
    },
    UpgradeTier {
        key: "queueBanner.upgrade.toAdvanced",
        label: "升级高级版",
    },
    UpgradeTier {
        key: "queueBanner.upgrade.toFlagship",
        label: "升级旗舰版",
    },
];

/// 进度阶段文案目录（运行时按下发 key 查表；仅收无品牌词键，按生命周期排序）。
pub const PHASES: [ProgressPhase; 14] = [
    ProgressPhase {
        key: "progress.phase.preparing",
        label: "准备中",
    },
    ProgressPhase {
        key: "progress.phase.model_requesting",
        label: "等待模型响应",
    },
    ProgressPhase {
        key: "progress.phase.model_streaming",
        label: "生成回复中",
    },
    ProgressPhase {
        key: "progress.phase.model_streaming.Agent",
        label: "正在准备子任务",
    },
    ProgressPhase {
        key: "progress.phase.model_streaming.AskUserQuestion",
        label: "正在准备提问",
    },
    ProgressPhase {
        key: "progress.phase.model_streaming.Bash",
        label: "正在准备执行命令",
    },
    ProgressPhase {
        key: "progress.phase.tool_executing",
        label: "调用工具中",
    },
    ProgressPhase {
        key: "progress.phase.tool_executing.Edit",
        label: "正在编辑文件",
    },
    ProgressPhase {
        key: "progress.phase.waiting_for_user",
        label: "等待回复",
    },
    ProgressPhase {
        key: "progress.phase.waiting_for_permission",
        label: "等待授权",
    },
    ProgressPhase {
        key: "progress.compacting",
        label: "压缩上下文中",
    },
    ProgressPhase {
        key: "progress.phase.retrying",
        label: "重试中",
    },
    ProgressPhase {
        key: "progress.phase.model_done",
        label: "处理结果中",
    },
    ProgressPhase {
        key: "progress.phase.finalization_file_projection_sync_notify",
        label: "正在同步文件，请勿中断以免丢失数据",
    },
];

/// 空态提示（Normal 态下条目为空时展示；逐字取自 `progress.phase.preparing`）。
pub const EMPTY_HINT: &str = "准备中";

/// 按下发 key 查进度文案；查不到返回 `None`（fail-closed，不猜词）。
#[must_use]
pub fn phase_label(key: &str) -> Option<&'static str> {
    PHASES.iter().find(|phase| phase.key == key).map(|phase| phase.label)
}

/// 浮层快照：状态 + 已下发的进度 key 清单。
pub struct QueueProgress {
    pub state: OverlayState,
    pub items: Vec<&'static str>,
}

impl QueueProgress {
    /// 渲染可见的进度文案：仅 Normal 态查表返回，其余态一律为空（走提示行）。
    #[must_use]
    pub fn visible_labels(&self) -> Vec<&'static str> {
        if self.state != OverlayState::Normal {
            return Vec::new();
        }
        self.items.iter().filter_map(|key| phase_label(key)).collect()
    }

    /// 状态提示行文案：空态/加载走「准备中」，未下发/失败走「未下发」。
    #[must_use]
    pub fn hint(&self) -> &'static str {
        match self.state {
            OverlayState::Normal | OverlayState::Loading => EMPTY_HINT,
            OverlayState::Failed | OverlayState::NotDelivered => NOT_DELIVERED,
        }
    }
}

/// 默认快照：未下发（运行期无数据源时的诚实默认，⛔ 不伪造条目）。
#[must_use]
pub fn not_delivered() -> QueueProgress {
    QueueProgress {
        state: OverlayState::NotDelivered,
        items: Vec::new(),
    }
}

/// 由下发 key 清单构造 Normal 快照。
#[must_use]
pub fn delivered(items: &[&'static str]) -> QueueProgress {
    QueueProgress {
        state: OverlayState::Normal,
        items: items.to_vec(),
    }
}
/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
/// 运行期无数据源 ⇒ 恒以未下发快照渲染（fail-closed，不伪造条目）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&not_delivered(), theme)
}

/// 按快照渲染整层：标题行 + 排队横幅 + 进度区。
fn render(overlay: &QueueProgress, theme: &Theme) -> Div {
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
        .child(banner_row(theme))
        .child(progress_area(overlay, theme))
}

/// 排队横幅：3 档升级档位（h_12 = 48px 触摸目标）。
fn banner_row(theme: &Theme) -> Div {
    let mut row = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap_2()
        .px_4()
        .pb_2()
        .border_b_1()
        .border_color(theme.border);
    for tier in &BANNER_TIERS {
        row = row.child(
            div()
                .h_12()
                .flex()
                .items_center()
                .px_3()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .text_sm()
                .text_color(theme.accent)
                .child(tier.label),
        );
    }
    row
}

/// 进度区：Normal 态有条目 → 清单行；否则按状态提示（空态 / 加载 / 未下发）。
fn progress_area(overlay: &QueueProgress, theme: &Theme) -> Div {
    let labels = overlay.visible_labels();
    let mut area = div().flex_1().flex().flex_col().p_4().gap_2();
    if labels.is_empty() {
        area = area.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(overlay.hint()),
        );
    } else {
        for label in labels {
            area = area.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(div().w_2().h_2().rounded_full().bg(theme.accent))
                    .child(div().text_sm().text_color(theme.foreground).child(label)),
            );
        }
    }
    area
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
