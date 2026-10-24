//! 工具·审批 —— 波次 9，卡 `ui-overlay-tool-approval`。
//!
//! 分层约定（同 `credit_hint`）：纯数据层（文案表/状态机先行）+ GPUI 层（只做映射）。
//! 硬约束：fail-closed；文案命名空间 `tool.approval` 逐字取自文案全表，⛔ 自编禁令。
//! 命名空间共 24 有效键（品牌词已剔除），本卡落任务书逐字摘录的前 12 键；
//! 其余 12 键文案未随卡下发，按禁自编红线暂不落键，文案表就绪后补齐
//! （见 docs/ui-overlay-tool-approval-status.md）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 面板标题（逐字取自文案全表）。
pub const PANEL_TITLE: &str = "工具·审批";

// ===================== 纯数据层（文案表 + 状态机，无 GPUI 依赖） =====================

/// `tool.approval.allow`（逐字）。
pub const TEXT_ALLOW: &str = "允许";
/// `tool.approval.alwaysAllow`（逐字）。
pub const TEXT_ALWAYS_ALLOW: &str = "本次会话始终允许";
/// `tool.approval.alwaysAllowThisLaunch`（逐字）。
pub const TEXT_ALWAYS_ALLOW_THIS_LAUNCH: &str = "本次启动内始终允许";
/// `tool.approval.confirm`（逐字）。
pub const TEXT_CONFIRM: &str = "确认";
/// `tool.approval.confirmTitle.computerUse`（逐字）。
pub const TEXT_CONFIRM_TITLE_COMPUTER_USE: &str = "确认开始使用电脑操控能力？";
/// `tool.approval.confirmTitle.imageGen`（逐字）。
pub const TEXT_CONFIRM_TITLE_IMAGE_GEN: &str = "确认开始生成图片？";
/// `tool.approval.confirmTitle.imageProcessing`（逐字）。
pub const TEXT_CONFIRM_TITLE_IMAGE_PROCESSING: &str = "确认开始处理图片？";
/// `tool.approval.confirmTitle.videoGen`（逐字）。
pub const TEXT_CONFIRM_TITLE_VIDEO_GEN: &str = "确认开始生成视频？";
/// `tool.approval.creditHint.computerUse`（逐字）。
pub const TEXT_CREDIT_HINT_COMPUTER_USE: &str = "使用该能力将消耗额外积分，请谨慎操作";
/// `tool.approval.creditHint.imageGen`（逐字）。
pub const TEXT_CREDIT_HINT_IMAGE_GEN: &str =
    "当前模型生成图片将消耗额外积分，每张图片约 5-10积分，请谨慎操作";
/// `tool.approval.creditHint.imageProcessing`（逐字）。
pub const TEXT_CREDIT_HINT_IMAGE_PROCESSING: &str = "图片处理预计将消耗 5-10 积分，实际消耗以最终结算为准，请谨慎操作。允许后，当前会话内再次使用图片处理将不再提醒。";
/// `tool.approval.creditHint.videoGen`（逐字）。
pub const TEXT_CREDIT_HINT_VIDEO_GEN: &str =
    "生成视频将消耗额外积分，每 5 秒约 50-100 积分，请谨慎操作";

/// 命名空间 `tool.approval` 文案表（const 数组，键值逐字，按全键名排序；本卡前 12 键）。
pub const COPY_TABLE: [(&str, &str); 12] = [
    ("tool.approval.allow", TEXT_ALLOW),
    ("tool.approval.alwaysAllow", TEXT_ALWAYS_ALLOW),
    (
        "tool.approval.alwaysAllowThisLaunch",
        TEXT_ALWAYS_ALLOW_THIS_LAUNCH,
    ),
    ("tool.approval.confirm", TEXT_CONFIRM),
    (
        "tool.approval.confirmTitle.computerUse",
        TEXT_CONFIRM_TITLE_COMPUTER_USE,
    ),
    (
        "tool.approval.confirmTitle.imageGen",
        TEXT_CONFIRM_TITLE_IMAGE_GEN,
    ),
    (
        "tool.approval.confirmTitle.imageProcessing",
        TEXT_CONFIRM_TITLE_IMAGE_PROCESSING,
    ),
    (
        "tool.approval.confirmTitle.videoGen",
        TEXT_CONFIRM_TITLE_VIDEO_GEN,
    ),
    (
        "tool.approval.creditHint.computerUse",
        TEXT_CREDIT_HINT_COMPUTER_USE,
    ),
    (
        "tool.approval.creditHint.imageGen",
        TEXT_CREDIT_HINT_IMAGE_GEN,
    ),
    (
        "tool.approval.creditHint.imageProcessing",
        TEXT_CREDIT_HINT_IMAGE_PROCESSING,
    ),
    (
        "tool.approval.creditHint.videoGen",
        TEXT_CREDIT_HINT_VIDEO_GEN,
    ),
];

/// 按全键名查文案（纯函数；未知键返回 `None`，⛔ 不猜值）。
#[must_use]
pub fn copy_text(key: &str) -> Option<&'static str> {
    COPY_TABLE
        .iter()
        .find(|(table_key, _)| *table_key == key)
        .map(|(_, text)| *text)
}

/// 加载态文案（同族界面约定，逐字）。
pub const TEXT_LOADING: &str = "加载中...";
/// 空态文案（Normal 且无待审批请求；同族界面约定，逐字）。
pub const TEXT_EMPTY: &str = "暂无内容";
/// 失败/未下发的 fail-closed 文案（同族界面约定，逐字；⛔ 不退化成空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 浮层状态机：默认 `Loading`（未拉取过 ≠ 空集，⛔ 不把缺数据伪装成空态）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolApprovalState {
    /// 已接入且存在待审批请求。
    Normal,
    /// 未拉取过（诚实默认态）。
    #[default]
    Loading,
    /// 拉取失败（fail-closed：如实可见，不静默吞掉）。
    Failed,
}

/// 待审批能力（`confirmTitle`/`creditHint` 的 4 个分支）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    ComputerUse,
    ImageGen,
    ImageProcessing,
    VideoGen,
}

impl Capability {
    /// 全量分支（渲染回归按此遍历）。
    pub const ALL: [Self; 4] = [
        Self::ComputerUse,
        Self::ImageGen,
        Self::ImageProcessing,
        Self::VideoGen,
    ];

    /// 确认标题的全键名。
    #[must_use]
    pub const fn confirm_title_key(self) -> &'static str {
        match self {
            Self::ComputerUse => "tool.approval.confirmTitle.computerUse",
            Self::ImageGen => "tool.approval.confirmTitle.imageGen",
            Self::ImageProcessing => "tool.approval.confirmTitle.imageProcessing",
            Self::VideoGen => "tool.approval.confirmTitle.videoGen",
        }
    }

    /// 积分提示的全键名。
    #[must_use]
    pub const fn credit_hint_key(self) -> &'static str {
        match self {
            Self::ComputerUse => "tool.approval.creditHint.computerUse",
            Self::ImageGen => "tool.approval.creditHint.imageGen",
            Self::ImageProcessing => "tool.approval.creditHint.imageProcessing",
            Self::VideoGen => "tool.approval.creditHint.videoGen",
        }
    }

    /// 确认标题（逐字）。
    #[must_use]
    pub const fn confirm_title(self) -> &'static str {
        match self {
            Self::ComputerUse => TEXT_CONFIRM_TITLE_COMPUTER_USE,
            Self::ImageGen => TEXT_CONFIRM_TITLE_IMAGE_GEN,
            Self::ImageProcessing => TEXT_CONFIRM_TITLE_IMAGE_PROCESSING,
            Self::VideoGen => TEXT_CONFIRM_TITLE_VIDEO_GEN,
        }
    }

    /// 积分提示（逐字）。
    #[must_use]
    pub const fn credit_hint(self) -> &'static str {
        match self {
            Self::ComputerUse => TEXT_CREDIT_HINT_COMPUTER_USE,
            Self::ImageGen => TEXT_CREDIT_HINT_IMAGE_GEN,
            Self::ImageProcessing => TEXT_CREDIT_HINT_IMAGE_PROCESSING,
            Self::VideoGen => TEXT_CREDIT_HINT_VIDEO_GEN,
        }
    }
}

/// 审批动作（命名空间的 4 个动作键）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalAction {
    Allow,
    AlwaysAllow,
    AlwaysAllowThisLaunch,
    Confirm,
}

impl ApprovalAction {
    /// 全量动作（渲染按此遍历）。
    pub const ALL: [Self; 4] = [
        Self::Allow,
        Self::AlwaysAllow,
        Self::AlwaysAllowThisLaunch,
        Self::Confirm,
    ];

    /// 动作的全键名。
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Allow => "tool.approval.allow",
            Self::AlwaysAllow => "tool.approval.alwaysAllow",
            Self::AlwaysAllowThisLaunch => "tool.approval.alwaysAllowThisLaunch",
            Self::Confirm => "tool.approval.confirm",
        }
    }

    /// 动作文案（逐字）。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Allow => TEXT_ALLOW,
            Self::AlwaysAllow => TEXT_ALWAYS_ALLOW,
            Self::AlwaysAllowThisLaunch => TEXT_ALWAYS_ALLOW_THIS_LAUNCH,
            Self::Confirm => TEXT_CONFIRM,
        }
    }
}

/// 浮层数据模型：状态 + 待审批能力（`None` 在 Normal 下 ⇒ 空态）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolApproval {
    pub state: ToolApprovalState,
    pub request: Option<Capability>,
}

impl ToolApproval {
    /// 未拉取过的诚实默认态。
    #[must_use]
    pub fn loading() -> Self {
        Self {
            state: ToolApprovalState::Loading,
            request: None,
        }
    }

    /// 拉取失败（fail-closed）。
    #[must_use]
    pub fn failed() -> Self {
        Self {
            state: ToolApprovalState::Failed,
            request: None,
        }
    }

    /// 已接入、当前无待审批请求（空态，⛔ 不是空列表渲染）。
    #[must_use]
    pub fn empty() -> Self {
        Self {
            state: ToolApprovalState::Normal,
            request: None,
        }
    }

    /// 已接入、待审批的能力确认请求。
    #[must_use]
    pub fn capability(capability: Capability) -> Self {
        Self {
            state: ToolApprovalState::Normal,
            request: Some(capability),
        }
    }

    /// 空态判定：仅 Normal 且无待审批请求。
    #[must_use]
    pub fn is_empty_state(&self) -> bool {
        self.state == ToolApprovalState::Normal && self.request.is_none()
    }
}

// ===================== GPUI 层（只做映射，颜色一律取 theme token） =====================

/// 浮层渲染（波次 9：浮层不占主区，返回整层容器由调用方叠加）。
/// 数据源在运行时侧：`main_area` 以未拉取过的 Loading 默认模型渲染（fail-closed）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &ToolApproval::loading())
}

/// 按数据模型渲染整层：标题行 + 状态分支主体（加载/失败/空态/正常）。
#[must_use]
pub fn render(theme: &Theme, approval: &ToolApproval) -> Div {
    let body = match approval.state {
        ToolApprovalState::Loading => centered_line(theme, TEXT_LOADING),
        ToolApprovalState::Failed => centered_line(theme, NOT_DELIVERED),
        ToolApprovalState::Normal if approval.is_empty_state() => centered_line(theme, TEXT_EMPTY),
        ToolApprovalState::Normal => match approval.request {
            Some(capability) => approval_body(theme, capability),
            // Normal 但缺请求 ⇒ 同空态，⛔ 不渲染空壳。
            None => centered_line(theme, TEXT_EMPTY),
        },
    };
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
                .child(PANEL_TITLE),
        )
        .child(body)
}

/// 居中单行（加载 / 失败 / 空态共用）。
fn centered_line(theme: &Theme, text: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_string())
}

/// 正常主体：确认标题 + 积分提示 + 动作按钮行。
fn approval_body(theme: &Theme, capability: Capability) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .py_3()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(capability.confirm_title()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(capability.credit_hint()),
        )
        .child(actions_row(theme))
}

/// 动作按钮行：三档允许（次级）+ 确认（主）。
fn actions_row(theme: &Theme) -> Div {
    let mut row = div().flex().flex_wrap().gap_2();
    for action in ApprovalAction::ALL {
        row = row.child(action_button(theme, action));
    }
    row
}

/// 单个动作按钮（触达目标 ≥48dp：h_12；`Confirm` 为主按钮走 accent token）。
fn action_button(theme: &Theme, action: ApprovalAction) -> Div {
    let button = div()
        .h_12()
        .min_w(px(96.))
        .px_5()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm();
    if matches!(action, ApprovalAction::Confirm) {
        button.text_color(theme.accent)
    } else {
        button.text_color(theme.foreground)
    }
    .child(action.label())
}

#[cfg(test)]
mod tests {
    use super::{
        copy_text, main_area, render, ApprovalAction, Capability, ToolApproval, ToolApprovalState,
        COPY_TABLE, NOT_DELIVERED, PANEL_TITLE, TEXT_ALLOW, TEXT_ALWAYS_ALLOW,
        TEXT_ALWAYS_ALLOW_THIS_LAUNCH, TEXT_CONFIRM, TEXT_CONFIRM_TITLE_COMPUTER_USE,
        TEXT_CONFIRM_TITLE_IMAGE_GEN, TEXT_CONFIRM_TITLE_IMAGE_PROCESSING,
        TEXT_CONFIRM_TITLE_VIDEO_GEN, TEXT_CREDIT_HINT_COMPUTER_USE, TEXT_CREDIT_HINT_IMAGE_GEN,
        TEXT_CREDIT_HINT_IMAGE_PROCESSING, TEXT_CREDIT_HINT_VIDEO_GEN, TEXT_EMPTY, TEXT_LOADING,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn copy_table_is_verbatim_twelve_entries() {
        let expected: [(&str, &str); 12] = [
            ("tool.approval.allow", "允许"),
            ("tool.approval.alwaysAllow", "本次会话始终允许"),
            ("tool.approval.alwaysAllowThisLaunch", "本次启动内始终允许"),
            ("tool.approval.confirm", "确认"),
            ("tool.approval.confirmTitle.computerUse", "确认开始使用电脑操控能力？"),
            ("tool.approval.confirmTitle.imageGen", "确认开始生成图片？"),
            ("tool.approval.confirmTitle.imageProcessing", "确认开始处理图片？"),
            ("tool.approval.confirmTitle.videoGen", "确认开始生成视频？"),
            ("tool.approval.creditHint.computerUse", "使用该能力将消耗额外积分，请谨慎操作"),
            (
                "tool.approval.creditHint.imageGen",
                "当前模型生成图片将消耗额外积分，每张图片约 5-10积分，请谨慎操作",
            ),
            (
                "tool.approval.creditHint.imageProcessing",
                "图片处理预计将消耗 5-10 积分，实际消耗以最终结算为准，请谨慎操作。允许后，当前会话内再次使用图片处理将不再提醒。",
            ),
            (
                "tool.approval.creditHint.videoGen",
                "生成视频将消耗额外积分，每 5 秒约 50-100 积分，请谨慎操作",
            ),
        ];
        assert_eq!(COPY_TABLE, expected);
    }

    #[test]
    fn copy_table_keys_unique_and_lookup_by_full_key() {
        for &(key, _) in COPY_TABLE.iter() {
            assert_eq!(COPY_TABLE.iter().filter(|(k, _)| *k == key).count(), 1);
        }
        for &(key, text) in COPY_TABLE.iter() {
            assert_eq!(copy_text(key), Some(text));
        }
        assert_eq!(copy_text("tool.approval.unknownKey"), None);
        assert_eq!(copy_text(""), None);
    }

    #[test]
    fn capability_titles_and_hints_are_verbatim() {
        for capability in Capability::ALL {
            assert_eq!(
                copy_text(capability.confirm_title_key()),
                Some(capability.confirm_title())
            );
            assert_eq!(
                copy_text(capability.credit_hint_key()),
                Some(capability.credit_hint())
            );
        }
        assert_eq!(
            Capability::ComputerUse.confirm_title(),
            TEXT_CONFIRM_TITLE_COMPUTER_USE
        );
        assert_eq!(
            Capability::ImageGen.confirm_title(),
            TEXT_CONFIRM_TITLE_IMAGE_GEN
        );
        assert_eq!(
            Capability::ImageProcessing.confirm_title(),
            TEXT_CONFIRM_TITLE_IMAGE_PROCESSING
        );
        assert_eq!(
            Capability::VideoGen.confirm_title(),
            TEXT_CONFIRM_TITLE_VIDEO_GEN
        );
        assert_eq!(
            Capability::ComputerUse.credit_hint(),
            TEXT_CREDIT_HINT_COMPUTER_USE
        );
        assert_eq!(
            Capability::ImageGen.credit_hint(),
            TEXT_CREDIT_HINT_IMAGE_GEN
        );
        assert_eq!(
            Capability::ImageProcessing.credit_hint(),
            TEXT_CREDIT_HINT_IMAGE_PROCESSING
        );
        assert_eq!(
            Capability::VideoGen.credit_hint(),
            TEXT_CREDIT_HINT_VIDEO_GEN
        );
    }

    #[test]
    fn action_labels_are_verbatim() {
        for action in ApprovalAction::ALL {
            assert_eq!(copy_text(action.key()), Some(action.label()));
        }
        assert_eq!(ApprovalAction::Allow.label(), TEXT_ALLOW);
        assert_eq!(ApprovalAction::AlwaysAllow.label(), TEXT_ALWAYS_ALLOW);
        assert_eq!(
            ApprovalAction::AlwaysAllowThisLaunch.label(),
            TEXT_ALWAYS_ALLOW_THIS_LAUNCH
        );
        assert_eq!(ApprovalAction::Confirm.label(), TEXT_CONFIRM);
    }

    #[test]
    fn default_state_is_loading_not_empty() {
        assert_eq!(ToolApproval::default().state, ToolApprovalState::Loading);
        assert_eq!(ToolApproval::loading().state, ToolApprovalState::Loading);
        assert!(ToolApproval::loading().request.is_none());
        assert!(!ToolApproval::loading().is_empty_state());
    }

    #[test]
    fn empty_state_requires_normal_without_request() {
        assert!(ToolApproval::empty().is_empty_state());
        assert!(!ToolApproval::failed().is_empty_state());
        assert!(!ToolApproval::capability(Capability::ImageGen).is_empty_state());
    }

    #[test]
    fn renders_do_not_panic_for_every_state_and_capability() {
        let theme = Theme::default();
        let _ = main_area(&theme);
        let _ = render(&theme, &ToolApproval::loading());
        let _ = render(&theme, &ToolApproval::failed());
        let _ = render(&theme, &ToolApproval::empty());
        for capability in Capability::ALL {
            let _ = render(&theme, &ToolApproval::capability(capability));
        }
        assert!(!PANEL_TITLE.is_empty());
        assert!(!TEXT_LOADING.is_empty());
        assert!(!TEXT_EMPTY.is_empty());
        assert!(!NOT_DELIVERED.is_empty());
    }
}
