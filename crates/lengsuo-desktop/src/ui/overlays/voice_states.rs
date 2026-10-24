//! 语音输入状态浮层（波次 6，卡 `ui-overlay-voice-states`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（[`VoiceState`] / [`VoiceStatesModel`]）
//! 叠加 GPUI 层（[`main_area`]）。
//!
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `voice`。
//! 数据源在运行时侧：[`demo`] 是规格枚举的分支清单，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// —— 纯数据层（无 GPUI 依赖）——

/// 浮层标题（逐字取自 `voice.start`）。
pub const TITLE: &str = "语音输入";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 识别中徽标文案（逐字取自 `voice.processing`）。
pub const PROCESSING: &str = "识别中...";
/// 空态说明（逐字取自 `voice.info.emptyResult`）。
pub const EMPTY_RESULT: &str = "未检测到语音内容，请重试。";
/// 操作文案（逐字取自 `voice.start`）。
pub const BTN_START: &str = "语音输入";
/// 操作文案（逐字取自 `voice.stop`）。
pub const BTN_STOP: &str = "停止录制";
/// 操作文案（逐字取自 `voice.cancel`）。
pub const BTN_CANCEL: &str = "取消录制";

/// 语音输入浮层的可枚举状态（对应文案全表 `voice` 命名空间的分支）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceState {
    /// 识别中（`voice.processing`）。
    Processing,
    /// 取消录制（`voice.cancel`）。
    Cancelled,
    /// 空结果（`voice.info.emptyResult`）。
    EmptyResult,
    /// 识别失败（`voice.error.asrFailed`）。
    AsrFailed,
    /// 麦克风被其他应用占用（`voice.error.deviceBusy`）。
    DeviceBusy,
    /// 未检测到可用麦克风（`voice.error.noDevice`）。
    NoDevice,
}

impl VoiceState {
    /// 该状态的主文案（逐字取自文案全表 `voice` 命名空间）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Processing => PROCESSING,
            Self::Cancelled => BTN_CANCEL,
            Self::EmptyResult => EMPTY_RESULT,
            Self::AsrFailed => "语音识别失败，请重试。",
            Self::DeviceBusy => "麦克风正被其他应用占用。",
            Self::NoDevice => "未检测到可用的麦克风设备。",
        }
    }

    /// 是否错误分支（识别失败 / 设备占用 / 无设备）。
    #[must_use]
    pub fn is_error(self) -> bool {
        matches!(self, Self::AsrFailed | Self::DeviceBusy | Self::NoDevice)
    }
}

/// 规格枚举的分支清单（取消录制 / 识别失败 / 设备占用 / 无设备）。
pub const SPEC_BRANCHES: &[VoiceState] = &[
    VoiceState::Cancelled,
    VoiceState::AsrFailed,
    VoiceState::DeviceBusy,
    VoiceState::NoDevice,
];

/// 语音输入状态浮层的纯数据模型。
#[derive(Clone, Debug)]
pub struct VoiceStatesModel {
    /// 已下发的分支清单；`None` = 数据未下发（fail-closed ⇒ 「未下发」）。
    branches: Option<Vec<VoiceState>>,
}

impl VoiceStatesModel {
    /// 未下发（fail-closed 初始态）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self { branches: None }
    }

    /// 已下发；空清单 ⇒ 空态（不是空列表）。
    #[must_use]
    pub fn delivered(branches: Vec<VoiceState>) -> Self {
        Self {
            branches: Some(branches),
        }
    }

    /// 是否未下发。
    #[must_use]
    pub fn is_not_delivered(&self) -> bool {
        self.branches.is_none()
    }

    /// 当前占位文案：未下发 / 空态返回对应说明，有内容时为 `None`。
    #[must_use]
    pub fn placeholder(&self) -> Option<&'static str> {
        match self.branches.as_deref() {
            None => Some(NOT_DELIVERED),
            Some([]) => Some(EMPTY_RESULT),
            Some(_) => None,
        }
    }

    /// 清单行数据（未下发时为空切片）。
    #[must_use]
    pub fn rows(&self) -> &[VoiceState] {
        self.branches.as_deref().unwrap_or(&[])
    }

    /// 错误分支计数。
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.rows().iter().filter(|state| state.is_error()).count()
    }
}

/// 演示数据：规格枚举的 [`SPEC_BRANCHES`]；数据源就绪后由运行时替换。
#[must_use]
pub fn demo() -> VoiceStatesModel {
    VoiceStatesModel::delivered(SPEC_BRANCHES.to_vec())
}

// —— GPUI 层 ——

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
///
/// 结构：标题行（标题 + 识别中徽标）→ 内容区（[`overlay_body`] 三分支）
/// → 操作文案行（仅复刻文案，不接交互）。
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
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(TITLE),
                )
                .child(badge(theme, PROCESSING)),
        )
        .child(overlay_body(theme, &demo()))
        .child(action_row(theme))
}

/// 徽标：细边框小标签（识别中 / 错误态提示）。
fn badge(theme: &Theme, text: &'static str) -> Div {
    div()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(theme.accent)
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(text)
}

/// 内容区三分支：未下发（fail-closed）/ 空态 / 清单行。
fn overlay_body(theme: &Theme, model: &VoiceStatesModel) -> Div {
    if let Some(placeholder) = model.placeholder() {
        return div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(placeholder);
    }
    div().flex_1().flex().flex_col().gap_1().py_2().children(
        model
            .rows()
            .iter()
            .copied()
            .map(|state| branch_row(theme, state)),
    )
}

/// 单条清单行：分支标记（错误分支高亮）+ 主文案。
fn branch_row(theme: &Theme, state: VoiceState) -> Div {
    let marker_color = if state.is_error() {
        theme.accent
    } else {
        theme.muted_foreground
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_1()
        .text_sm()
        .child(div().text_xs().text_color(marker_color).child("•"))
        .child(div().text_color(theme.foreground).child(state.label()))
}

/// 操作文案行（语音输入 / 停止录制 / 取消录制；界面卡不接交互）。
fn action_row(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .children([BTN_START, BTN_STOP, BTN_CANCEL].map(|text| badge(theme, text)))
}

#[cfg(test)]
mod tests {
    use super::{
        action_row, badge, branch_row, demo, main_area, overlay_body, VoiceState, VoiceStatesModel,
        BTN_CANCEL, BTN_START, BTN_STOP, EMPTY_RESULT, NOT_DELIVERED, PROCESSING, SPEC_BRANCHES,
        TITLE,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn copy_is_verbatim_from_voice_namespace() {
        assert_eq!(TITLE, "语音输入");
        assert_eq!(PROCESSING, "识别中...");
        assert_eq!(EMPTY_RESULT, "未检测到语音内容，请重试。");
        assert_eq!(BTN_START, "语音输入");
        assert_eq!(BTN_STOP, "停止录制");
        assert_eq!(BTN_CANCEL, "取消录制");
        assert_eq!(VoiceState::AsrFailed.label(), "语音识别失败，请重试。");
        assert_eq!(VoiceState::DeviceBusy.label(), "麦克风正被其他应用占用。");
        assert_eq!(VoiceState::NoDevice.label(), "未检测到可用的麦克风设备。");
        assert_eq!(VoiceState::Cancelled.label(), "取消录制");
        assert_eq!(
            VoiceState::EmptyResult.label(),
            "未检测到语音内容，请重试。"
        );
    }

    #[test]
    fn is_error_marks_only_error_branches() {
        assert!(!VoiceState::Processing.is_error());
        assert!(!VoiceState::Cancelled.is_error());
        assert!(!VoiceState::EmptyResult.is_error());
        assert!(VoiceState::AsrFailed.is_error());
        assert!(VoiceState::DeviceBusy.is_error());
        assert!(VoiceState::NoDevice.is_error());
    }

    #[test]
    fn demo_lists_spec_branches_with_error_count() {
        let model = demo();
        assert!(!model.is_not_delivered());
        assert_eq!(model.placeholder(), None);
        assert_eq!(model.rows(), SPEC_BRANCHES);
        assert_eq!(model.error_count(), 3);
    }

    #[test]
    fn not_delivered_is_fail_closed_placeholder() {
        let model = VoiceStatesModel::not_delivered();
        assert!(model.is_not_delivered());
        assert!(model.rows().is_empty());
        assert_eq!(model.placeholder(), Some(NOT_DELIVERED));
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn empty_input_yields_empty_state_not_empty_list() {
        let model = VoiceStatesModel::delivered(Vec::new());
        assert!(!model.is_not_delivered());
        assert!(model.rows().is_empty());
        assert_eq!(model.placeholder(), Some(EMPTY_RESULT));
        assert_eq!(model.error_count(), 0);
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    #[test]
    fn overlay_body_covers_all_three_branches_without_panic() {
        let theme = Theme::default();
        // 未下发（fail-closed）
        let _ = overlay_body(&theme, &VoiceStatesModel::not_delivered());
        // 空态（空输入 ⇒ 占位文案，不是空列表）
        let _ = overlay_body(&theme, &VoiceStatesModel::delivered(Vec::new()));
        // 清单行（规格枚举四分支）
        let _ = overlay_body(&theme, &demo());
    }

    #[test]
    fn row_and_action_nodes_build_without_panic() {
        let theme = Theme::default();
        for state in SPEC_BRANCHES {
            let _ = branch_row(&theme, *state);
        }
        let _ = branch_row(&theme, VoiceState::Processing);
        let _ = branch_row(&theme, VoiceState::EmptyResult);
        let _ = badge(&theme, PROCESSING);
        let _ = action_row(&theme);
    }
}
