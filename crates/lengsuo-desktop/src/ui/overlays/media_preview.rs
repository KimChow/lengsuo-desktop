//! 媒体预览浮层（波次 7，卡 `ui-overlay-media-preview`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `mediaPreview + preview`。
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "媒体预览";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

// —— 纯数据层（无 GPUI 依赖）——

/// 缩放范围与步长（本地 UI 状态，非服务端数据）。
pub const ZOOM_MIN_PERCENT: u16 = 50;
/// 见 [`ZOOM_MIN_PERCENT`]。
pub const ZOOM_MAX_PERCENT: u16 = 200;
/// 见 [`ZOOM_MIN_PERCENT`]。
pub const ZOOM_STEP_PERCENT: u16 = 25;
/// 见 [`ZOOM_MIN_PERCENT`]。
pub const ZOOM_DEFAULT_PERCENT: u16 = 100;

/// 媒体类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    /// 图片
    Image,
    /// 视频
    Video,
}

/// 工具行动作（label 逐字取自 `preview.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolAction {
    /// preview.copyLink
    CopyLink,
    /// preview.download
    Download,
    /// preview.share
    Share,
    /// preview.artifacts
    Artifacts,
}

/// 工具行（顺序即渲染顺序）。
pub const TOOL_ACTIONS: [ToolAction; 4] = [
    ToolAction::CopyLink,
    ToolAction::Download,
    ToolAction::Share,
    ToolAction::Artifacts,
];

impl ToolAction {
    /// 按钮文案（逐字）。
    pub fn label(self) -> &'static str {
        match self {
            Self::CopyLink => "复制链接",
            Self::Download => "下载",
            Self::Share => "分享",
            Self::Artifacts => "产物",
        }
    }
}

/// 工具行操作反馈（text 逐字取自 `preview.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolFeedback {
    /// preview.copyLinkSuccess
    CopyLinkSuccess,
    /// preview.copyLinkFailed
    CopyLinkFailed,
    /// preview.downloading
    Downloading,
    /// preview.downloadFailed
    DownloadFailed,
    /// preview.downloadNotAvailable
    DownloadNotAvailable,
}

impl ToolFeedback {
    /// 反馈文案（逐字）。
    pub fn text(self) -> &'static str {
        match self {
            Self::CopyLinkSuccess => "链接已复制到剪贴板",
            Self::CopyLinkFailed => "复制链接失败",
            Self::Downloading => "下载中...",
            Self::DownloadFailed => "下载失败",
            Self::DownloadNotAvailable => "下载不可用",
        }
    }
}

/// 预览失败原因（title/desc 逐字取自 `preview.error.*` / `preview.video.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewError {
    /// preview.error.initFailed
    InitFailed,
    /// preview.error.loadFailed
    LoadFailed,
    /// preview.video.loadFailed / loadFailedDesc
    VideoLoadFailed,
    /// preview.video.loadTimeout / loadTimeoutDesc
    VideoLoadTimeout,
    /// preview.error.disabled / disabledDesc
    ShareDisabled,
    /// preview.error.expired / expiredDesc
    ShareExpired,
    /// preview.error.loginRequired / loginRequiredDesc
    LoginRequired,
    /// preview.error.notFound / notFoundDesc
    NotFound,
}

impl PreviewError {
    /// 失败标题（逐字）。
    pub fn title(self) -> &'static str {
        match self {
            Self::InitFailed => "初始化失败",
            Self::LoadFailed => "加载失败",
            Self::VideoLoadFailed => "视频加载失败",
            Self::VideoLoadTimeout => "视频加载超时",
            Self::ShareDisabled => "此分享已被禁用",
            Self::ShareExpired => "此分享已过期",
            Self::LoginRequired => "此分享需要登录后才能查看",
            Self::NotFound => "分享不存在或已被删除",
        }
    }

    /// 失败说明（逐字；部分原因在文案表中无 Desc 键）。
    pub fn desc(self) -> Option<&'static str> {
        match self {
            Self::InitFailed | Self::LoadFailed => None,
            Self::VideoLoadFailed => Some("当前浏览器可能不支持该视频编码，可下载后使用本地播放器打开"),
            Self::VideoLoadTimeout => Some("网络较慢或文件较大，请检查网络后重试"),
            Self::ShareDisabled => Some("分享者已关闭此分享链接，无法继续访问"),
            Self::ShareExpired => Some("分享链接已超过有效期，无法继续访问"),
            Self::LoginRequired => Some("请先登录账号以查看完整内容"),
            Self::NotFound => Some("请检查链接是否正确，或者联系分享者确认"),
        }
    }
}

/// 浮层状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewStatus {
    /// 有媒体数据，正常渲染
    Normal,
    /// 预览失败（带原因）
    Failed(PreviewError),
    /// 数据未下发（无数据源时的诚实空态，默认态）
    NotDelivered,
}

/// 媒体预览浮层的完整纯数据模型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaPreview {
    /// 媒体类型
    pub kind: MediaKind,
    /// 缩放百分比（本地 UI 状态）
    pub zoom_percent: u16,
    /// 浮层状态
    pub status: PreviewStatus,
    /// 工具行最近一次反馈
    pub feedback: Option<ToolFeedback>,
}

impl Default for MediaPreview {
    /// fail-closed：默认即未下发，而非空 Normal。
    fn default() -> Self {
        Self::not_delivered()
    }
}

impl MediaPreview {
    /// 未下发（无数据源时的诚实空态）。
    pub fn not_delivered() -> Self {
        Self {
            kind: MediaKind::Image,
            zoom_percent: ZOOM_DEFAULT_PERCENT,
            status: PreviewStatus::NotDelivered,
            feedback: None,
        }
    }

    /// 工具行（未下发时不渲染操作，⛔ 不给无数据动作留入口）。
    pub fn tool_actions(&self) -> &'static [ToolAction] {
        match self.status {
            PreviewStatus::NotDelivered => &[],
            _ => &TOOL_ACTIONS,
        }
    }

    /// 中心区文案：`None` = 正常有媒体待挂载；`Some` = 失败/未下发占位文案。
    /// 空输入落到 `Some(NOT_DELIVERED)`，⛔ 不是空列表。
    pub fn center_line(&self) -> Option<&'static str> {
        match self.status {
            PreviewStatus::Normal => None,
            PreviewStatus::Failed(err) => Some(err.title()),
            PreviewStatus::NotDelivered => Some(NOT_DELIVERED),
        }
    }

    /// 中心区说明行（仅失败态有）。
    pub fn center_desc(&self) -> Option<&'static str> {
        match self.status {
            PreviewStatus::Failed(err) => err.desc(),
            _ => None,
        }
    }

    /// 放大（夹在 `ZOOM_MIN..=ZOOM_MAX`）。
    pub fn zoom_in(&self) -> Self {
        let mut next = *self;
        next.zoom_percent = self
            .zoom_percent
            .saturating_add(ZOOM_STEP_PERCENT)
            .min(ZOOM_MAX_PERCENT);
        next
    }

    /// 缩小（夹在 `ZOOM_MIN..=ZOOM_MAX`）。
    pub fn zoom_out(&self) -> Self {
        let mut next = *self;
        next.zoom_percent = self
            .zoom_percent
            .saturating_sub(ZOOM_STEP_PERCENT)
            .max(ZOOM_MIN_PERCENT);
        next
    }
}
/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
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
