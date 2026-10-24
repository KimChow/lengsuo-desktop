//! 分享任务浮层（波次 7，卡 `ui-overlay-share-task`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（状态机 + 逐字文案 + 模板）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `shareTask`。
//! 数据源在运行时侧未下发 ⇒ [`main_area`] 如实渲染「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖）----------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "分享任务";
/// 弹窗标题（dialog.title）。
pub const DIALOG_TITLE: &str = "分享";
/// 产物清单标题模板（dialog.artifactList）。
pub const ARTIFACT_LIST: &str = "本次任务包含以下产物（{count}）";
/// 产物为空时的说明（dialog.noArtifacts）。
pub const NO_ARTIFACTS: &str = "暂无产物";
/// 消息数量标签（dialog.messageCount）。
pub const MESSAGE_COUNT: &str = "消息数量";
/// 复制链接按钮（dialog.copyLinkBtn）。
pub const COPY_LINK_BTN: &str = "复制链接";
/// 复制成功反馈（dialog.copyLinkSuccess）。
pub const COPY_LINK_SUCCESS: &str = "已复制链接";
/// 复制失败反馈（dialog.copyLinkFailed）。
pub const COPY_LINK_FAILED: &str = "复制链接失败";
/// 生成分享链接中（dialog.generating；与 dialog.generatingLink 同文，不再单列）。
pub const GENERATING: &str = "正在生成分享链接...";
/// 分享进行中（dialog.loading）。
pub const LOADING: &str = "分享中...";
/// 分享成功终态（dialog.shareSuccess）。
pub const SHARE_SUCCESS: &str = "分享成功";
/// 分享失败标题（dialog.shareFailed）。
pub const SHARE_FAILED: &str = "分享失败";
/// 加载失败（dialog.error）。
pub const LOAD_ERROR: &str = "加载失败";
/// 空态：没有可分享的内容（noContent）。
pub const NO_CONTENT: &str = "没有可分享的内容";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 上传阶段失败原因（文案逐字取自 uploadStage.*）。
#[derive(Clone, Copy, Debug)]
pub enum UploadStage {
    /// 登录状态已失效（uploadStage.authExpired）。
    AuthExpired,
    /// 服务配置异常（uploadStage.configInvalid）。
    ConfigInvalid,
    /// 文件上传中断（uploadStage.cosPutFailed）。
    CosPutFailed,
    /// 无法获取上传凭证（uploadStage.credentialFailed）。
    CredentialFailed,
    /// 导入资料库失败（uploadStage.importFailed）。
    ImportFailed,
    /// 资料库处理失败（uploadStage.pollFailed）。
    PollFailed,
    /// 处理超时（uploadStage.timeout）。
    Timeout,
}

impl UploadStage {
    /// 失败文案（逐字）。
    pub fn text(self) -> &'static str {
        match self {
            UploadStage::AuthExpired => "登录状态已失效，请重新登录后再试",
            UploadStage::ConfigInvalid => "服务配置异常，请重启客户端后再试",
            UploadStage::CosPutFailed => "文件上传中断，请重试",
            UploadStage::CredentialFailed => "无法获取上传凭证，请检查网络后重试",
            UploadStage::ImportFailed => "导入资料库失败，请重试",
            UploadStage::PollFailed => "资料库处理失败，请重试或联系客服",
            UploadStage::Timeout => "处理超时，文件可能仍在后台处理，请稍后到资料库查看",
        }
    }
}

/// 分享失败原因（文案逐字取自 shareTask.* 错误键）。
#[derive(Clone, Copy, Debug)]
pub enum FailureReason {
    /// 加载失败（dialog.error）。
    LoadError,
    /// 构建分享内容失败（dialog.buildTemplateFailed）。
    BuildTemplateFailed,
    /// 获取分享内容失败（dialog.collectArtifactsFailed）。
    CollectArtifactsFailed,
    /// 分享确认失败（dialog.confirmFailed）。
    ConfirmFailed,
    /// 内容安全审核失败（dialog.contentSecurityReviewFailed）。
    ContentSecurityReviewFailed,
    /// 文件超过 20MB（dialog.fileTooLarge）。
    FileTooLarge,
    /// 分享准备失败（dialog.prepareFailed）。
    PrepareFailed,
    /// 分享服务不可用（dialog.shareServiceUnavailable）。
    ShareServiceUnavailable,
    /// 产物上传失败（dialog.uploadArtifactsFailed）。
    UploadArtifactsFailed,
    /// 上传分享模板失败（dialog.uploadTemplateFailed）。
    UploadTemplateFailed,
    /// 表格行数超过限制（csvTooManyRows）。
    CsvTooManyRows,
    /// 文件格式暂不支持（formatUnsupported）。
    FormatUnsupported,
    /// 敏感内容（moderationBlocked）。
    ModerationBlocked,
    /// 安全策略拦截（securityBlocked）。
    SecurityBlocked,
    /// 任务标题过长（titleTooLong）。
    TitleTooLong,
    /// 上传失败（uploadFailed）。
    UploadFailed,
    /// 上传阶段失败（uploadStage.*）。
    Stage(UploadStage),
}

impl FailureReason {
    /// 失败文案（逐字）。
    pub fn text(self) -> &'static str {
        match self {
            FailureReason::LoadError => LOAD_ERROR,
            FailureReason::BuildTemplateFailed => "构建分享内容失败，请稍后重试",
            FailureReason::CollectArtifactsFailed => "获取分享内容失败，请稍后重试",
            FailureReason::ConfirmFailed => "分享确认失败，请稍后重试",
            FailureReason::ContentSecurityReviewFailed => "分享任务内容安全审核失败",
            FailureReason::FileTooLarge => "分享任务中文件超过 20MB，无法分享",
            FailureReason::PrepareFailed => "分享准备失败，请稍后重试",
            FailureReason::ShareServiceUnavailable => "分享服务不可用，请稍后重试",
            FailureReason::UploadArtifactsFailed => "产物上传失败，请稍后重试",
            FailureReason::UploadTemplateFailed => "上传分享模板失败，请稍后重试",
            FailureReason::CsvTooManyRows => "表格行数超过限制，无法分享",
            FailureReason::FormatUnsupported => "文件格式暂不支持，无法分享",
            FailureReason::ModerationBlocked => "任务名称或产物标题包含敏感内容，无法分享",
            FailureReason::SecurityBlocked => "内容涉及违规，已被安全策略拦截，无法分享",
            FailureReason::TitleTooLong => "任务标题过长，请重命名后再试",
            FailureReason::UploadFailed => "上传失败，请重试",
            FailureReason::Stage(stage) => stage.text(),
        }
    }
}

/// 复制链接反馈（copyLinkBtn 按下后的三态）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CopyFeedback {
    /// 未复制（无反馈行）。
    #[default]
    Idle,
    /// 已复制链接（copyLinkSuccess）。
    Copied,
    /// 复制失败（copyLinkFailed）。
    Failed,
}

impl CopyFeedback {
    /// 反馈文案；Idle 无反馈不渲染。
    pub fn text(self) -> Option<&'static str> {
        match self {
            CopyFeedback::Idle => None,
            CopyFeedback::Copied => Some(COPY_LINK_SUCCESS),
            CopyFeedback::Failed => Some(COPY_LINK_FAILED),
        }
    }
}

/// 分享内容（值全部由数据源下发，缺省如实隐藏，⛔ 不伪造）。
#[derive(Clone, Debug, Default)]
pub struct ShareContent {
    /// 产物名清单。
    pub artifacts: Vec<String>,
    /// 消息数量（None ⇒ 该行不出现）。
    pub message_count: Option<u32>,
    /// 分享链接（None ⇒ 不渲染链接与复制按钮）。
    pub link: Option<String>,
    /// 复制反馈。
    pub copy_feedback: CopyFeedback,
}

impl ShareContent {
    /// 产物清单标题：模板逐字取自 dialog.artifactList，{count} 如实填清单长度。
    pub fn artifact_header(&self) -> String {
        ARTIFACT_LIST.replace("{count}", &self.artifacts.len().to_string())
    }

    /// 消息数量清单行；未下发不出现。
    pub fn count_rows(&self) -> Vec<(&'static str, String)> {
        match self.message_count {
            Some(n) => vec![(MESSAGE_COUNT, n.to_string())],
            None => Vec::new(),
        }
    }

    /// 空输入判定：无产物、无消息数量、无链接 ⇒ 空态（⛔ 不是空列表）。
    pub fn is_empty(&self) -> bool {
        self.artifacts.is_empty() && self.message_count.is_none() && self.link.is_none()
    }

    /// 链接可复制（无链接不渲染复制按钮，⛔ 不渲染无效按钮）。
    pub fn can_copy(&self) -> bool {
        self.link.is_some()
    }
}

/// 浮层状态机（枚举分支即渲染分支）。
#[derive(Clone, Debug)]
pub enum Phase {
    /// 数据未下发（fail-closed 默认态）。
    NotDelivered,
    /// 正在生成分享链接。
    Generating,
    /// 分享中（上传/确认进行时）。
    Loading,
    /// 分享内容就绪（产物清单 + 消息数量 + 链接与复制反馈）。
    Ready(Box<ShareContent>),
    /// 分享成功终态。
    Succeeded,
    /// 分享失败终态（含原因文案）。
    Failed(FailureReason),
}

impl Phase {
    /// 状态页主文案；无独立状态页的分支不渲染。
    pub fn headline(&self) -> Option<&'static str> {
        match self {
            Phase::NotDelivered => Some(NOT_DELIVERED),
            Phase::Generating => Some(GENERATING),
            Phase::Loading => Some(LOADING),
            Phase::Ready(_) => None,
            Phase::Succeeded => Some(SHARE_SUCCESS),
            Phase::Failed(_) => Some(SHARE_FAILED),
        }
    }

    /// 状态页副文案（失败原因）；无则不渲染。
    pub fn description(&self) -> Option<&'static str> {
        match self {
            Phase::Failed(reason) => Some(reason.text()),
            _ => None,
        }
    }

    /// 是否分享成功终态。
    pub fn is_succeeded(&self) -> bool {
        matches!(self, Phase::Succeeded)
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
///
/// 数据源在运行时侧未下发 ⇒ fail-closed 渲染 [`Phase::NotDelivered`]（如实「未下发」）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render_phase(&Phase::NotDelivered, theme)
}

/// 按状态机分支渲染整层（标题行 + 分支主体）。
fn render_phase(phase: &Phase, theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(render_header(theme))
        .child(render_body(phase, theme))
}

/// 标题行：弹窗标题徽标（dialog.title）+ 视图标题。
fn render_header(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(DIALOG_TITLE),
        )
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
}

fn render_body(phase: &Phase, theme: &Theme) -> Div {
    match phase {
        Phase::Ready(content) => render_content(content, theme),
        Phase::NotDelivered => centered_line(NOT_DELIVERED, theme),
        _ => render_state_page(phase, theme),
    }
}

/// 居中单行状态（fail-closed：如实「未下发」，⛔ 不渲染空列表）。
fn centered_line(text: &'static str, theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 带内边距的弱化说明行（如「暂无产物」）。
fn padded_muted_line(text: &'static str, theme: &Theme) -> Div {
    div()
        .px_4()
        .py_1()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 状态分支（Generating/Loading/Succeeded/Failed）：居中主文案 + 失败原因。
fn render_state_page(phase: &Phase, theme: &Theme) -> Div {
    let mut page = div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .px_4();
    if let Some(headline) = phase.headline() {
        page = page.child(
            div()
                .text_base()
                .font_bold()
                .text_color(theme.foreground)
                .child(headline),
        );
    }
    if let Some(desc) = phase.description() {
        page = page.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(desc),
        );
    }
    page
}

/// 就绪分支：产物清单 + 消息数量 + 链接与复制反馈。
fn render_content(content: &ShareContent, theme: &Theme) -> Div {
    if content.is_empty() {
        // 空输入 ⇒ 空态，⛔ 不是空列表。
        return centered_line(NO_CONTENT, theme);
    }
    let mut body = div().flex_1().flex().flex_col().gap_2().py_2().child(
        div()
            .px_4()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(content.artifact_header()),
    );
    if content.artifacts.is_empty() {
        body = body.child(padded_muted_line(NO_ARTIFACTS, theme));
    } else {
        for name in &content.artifacts {
            body = body.child(
                div()
                    .px_4()
                    .py_1()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(name.clone()),
            );
        }
    }
    for (label, value) in content.count_rows() {
        body = body.child(
            div()
                .px_4()
                .flex()
                .justify_between()
                .gap_4()
                .text_sm()
                .child(div().text_color(theme.muted_foreground).child(label))
                .child(div().text_color(theme.foreground).child(value)),
        );
    }
    if content.can_copy() {
        body = body
            .child(
                div()
                    .px_4()
                    .py_1()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(content.link.clone().unwrap_or_default()),
            )
            .child(render_copy_area(content, theme));
    }
    body
}

/// 复制链接按钮 + 复制反馈行（无反馈则只有按钮）。
fn render_copy_area(content: &ShareContent, theme: &Theme) -> Div {
    let mut area = div().px_4().py_2().flex().flex_col().gap_1().child(
        div()
            .self_start()
            .px_3()
            .py_2()
            .rounded_md()
            .bg(theme.accent)
            .text_color(theme.accent_foreground)
            .text_sm()
            .child(COPY_LINK_BTN),
    );
    if let Some(feedback) = content.copy_feedback.text() {
        let color = match content.copy_feedback {
            CopyFeedback::Failed => theme.danger,
            _ => theme.accent,
        };
        area = area.child(div().text_xs().text_color(color).child(feedback));
    }
    area
}
#[cfg(test)]
mod tests {
    use super::{
        main_area, render_phase, CopyFeedback, FailureReason, Phase, ShareContent, UploadStage,
        ARTIFACT_LIST, COPY_LINK_BTN, COPY_LINK_FAILED, COPY_LINK_SUCCESS, DIALOG_TITLE,
        GENERATING, LOADING, LOAD_ERROR, MESSAGE_COUNT, NOT_DELIVERED, NO_ARTIFACTS, NO_CONTENT,
        SHARE_FAILED, SHARE_SUCCESS, TITLE,
    };
    use gpui_kit::component::Theme;

    fn sample_content() -> ShareContent {
        ShareContent {
            artifacts: vec!["迁移清单.md".into(), "架构图.png".into()],
            message_count: Some(12),
            link: Some("https://example.invalid/s/t1".into()),
            copy_feedback: CopyFeedback::Idle,
        }
    }

    #[test]
    fn copy_is_verbatim_from_table() {
        assert_eq!(TITLE, "分享任务");
        assert_eq!(DIALOG_TITLE, "分享");
        assert_eq!(ARTIFACT_LIST, "本次任务包含以下产物（{count}）");
        assert_eq!(NO_ARTIFACTS, "暂无产物");
        assert_eq!(MESSAGE_COUNT, "消息数量");
        assert_eq!(COPY_LINK_BTN, "复制链接");
        assert_eq!(COPY_LINK_SUCCESS, "已复制链接");
        assert_eq!(COPY_LINK_FAILED, "复制链接失败");
        assert_eq!(GENERATING, "正在生成分享链接...");
        assert_eq!(LOADING, "分享中...");
        assert_eq!(SHARE_SUCCESS, "分享成功");
        assert_eq!(SHARE_FAILED, "分享失败");
        assert_eq!(LOAD_ERROR, "加载失败");
        assert_eq!(NO_CONTENT, "没有可分享的内容");
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn failure_reasons_are_verbatim() {
        let cases = [
            (FailureReason::LoadError, "加载失败"),
            (
                FailureReason::BuildTemplateFailed,
                "构建分享内容失败，请稍后重试",
            ),
            (
                FailureReason::CollectArtifactsFailed,
                "获取分享内容失败，请稍后重试",
            ),
            (FailureReason::ConfirmFailed, "分享确认失败，请稍后重试"),
            (
                FailureReason::ContentSecurityReviewFailed,
                "分享任务内容安全审核失败",
            ),
            (
                FailureReason::FileTooLarge,
                "分享任务中文件超过 20MB，无法分享",
            ),
            (FailureReason::PrepareFailed, "分享准备失败，请稍后重试"),
            (
                FailureReason::ShareServiceUnavailable,
                "分享服务不可用，请稍后重试",
            ),
            (
                FailureReason::UploadArtifactsFailed,
                "产物上传失败，请稍后重试",
            ),
            (
                FailureReason::UploadTemplateFailed,
                "上传分享模板失败，请稍后重试",
            ),
            (FailureReason::CsvTooManyRows, "表格行数超过限制，无法分享"),
            (
                FailureReason::FormatUnsupported,
                "文件格式暂不支持，无法分享",
            ),
            (
                FailureReason::ModerationBlocked,
                "任务名称或产物标题包含敏感内容，无法分享",
            ),
            (
                FailureReason::SecurityBlocked,
                "内容涉及违规，已被安全策略拦截，无法分享",
            ),
            (FailureReason::TitleTooLong, "任务标题过长，请重命名后再试"),
            (FailureReason::UploadFailed, "上传失败，请重试"),
        ];
        for (reason, text) in cases {
            assert_eq!(reason.text(), text);
        }
    }

    #[test]
    fn upload_stage_reasons_are_verbatim() {
        let cases = [
            (UploadStage::AuthExpired, "登录状态已失效，请重新登录后再试"),
            (
                UploadStage::ConfigInvalid,
                "服务配置异常，请重启客户端后再试",
            ),
            (UploadStage::CosPutFailed, "文件上传中断，请重试"),
            (
                UploadStage::CredentialFailed,
                "无法获取上传凭证，请检查网络后重试",
            ),
            (UploadStage::ImportFailed, "导入资料库失败，请重试"),
            (UploadStage::PollFailed, "资料库处理失败，请重试或联系客服"),
            (
                UploadStage::Timeout,
                "处理超时，文件可能仍在后台处理，请稍后到资料库查看",
            ),
        ];
        for (stage, text) in cases {
            assert_eq!(FailureReason::Stage(stage).text(), text);
        }
    }

    #[test]
    fn artifact_header_fills_count_template_verbatim() {
        assert_eq!(
            sample_content().artifact_header(),
            "本次任务包含以下产物（2）"
        );
        assert_eq!(
            ShareContent::default().artifact_header(),
            "本次任务包含以下产物（0）"
        );
    }

    #[test]
    fn empty_input_yields_empty_state_not_empty_list() {
        let empty = ShareContent::default();
        assert!(empty.is_empty());
        assert!(empty.artifacts.is_empty());
        assert!(empty.count_rows().is_empty());
        assert!(!empty.can_copy());
        // 只有消息数量、无产物无链接：不是空态，也不渲染空清单。
        let count_only = ShareContent {
            message_count: Some(3),
            ..ShareContent::default()
        };
        assert!(!count_only.is_empty());
        assert_eq!(
            count_only.count_rows(),
            vec![(MESSAGE_COUNT, "3".to_string())]
        );
        assert_eq!(count_only.artifact_header(), "本次任务包含以下产物（0）");
        // 只有链接：可复制，但产物清单仍如实为空计数。
        let link_only = ShareContent {
            link: Some("https://example.invalid/s/t2".into()),
            ..ShareContent::default()
        };
        assert!(!link_only.is_empty());
        assert!(link_only.can_copy());
    }

    #[test]
    fn copy_feedback_text_mapping() {
        assert_eq!(CopyFeedback::Idle.text(), None);
        assert_eq!(CopyFeedback::Copied.text(), Some(COPY_LINK_SUCCESS));
        assert_eq!(CopyFeedback::Failed.text(), Some(COPY_LINK_FAILED));
        assert_eq!(sample_content().copy_feedback, CopyFeedback::Idle);
    }

    #[test]
    fn phase_headline_description_and_success_mapping() {
        assert_eq!(Phase::NotDelivered.headline(), Some(NOT_DELIVERED));
        assert_eq!(Phase::Generating.headline(), Some(GENERATING));
        assert_eq!(Phase::Loading.headline(), Some(LOADING));
        assert_eq!(Phase::Ready(Box::default()).headline(), None);
        assert_eq!(Phase::Succeeded.headline(), Some(SHARE_SUCCESS));
        assert!(Phase::Succeeded.is_succeeded());
        assert_eq!(
            Phase::Failed(FailureReason::FileTooLarge).headline(),
            Some(SHARE_FAILED)
        );
        assert_eq!(
            Phase::Failed(FailureReason::FileTooLarge).description(),
            Some("分享任务中文件超过 20MB，无法分享")
        );
        assert_eq!(Phase::Succeeded.description(), None);
        assert!(!Phase::Failed(FailureReason::LoadError).is_succeeded());
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    #[test]
    fn render_phase_covers_all_branches_without_panic() {
        let theme = Theme::default();
        let branches = [
            Phase::NotDelivered,
            Phase::Generating,
            Phase::Loading,
            Phase::Ready(Box::new(sample_content())),
            Phase::Ready(Box::default()),
            Phase::Ready(Box::new(ShareContent {
                message_count: Some(3),
                copy_feedback: CopyFeedback::Copied,
                ..ShareContent::default()
            })),
            Phase::Ready(Box::new(ShareContent {
                artifacts: vec!["仅产物".into()],
                copy_feedback: CopyFeedback::Failed,
                ..ShareContent::default()
            })),
            Phase::Succeeded,
            Phase::Failed(FailureReason::LoadError),
            Phase::Failed(FailureReason::Stage(UploadStage::PollFailed)),
        ];
        for phase in &branches {
            let _ = render_phase(phase, &theme);
        }
    }
}
