//! 深链落地状态浮层（波次 8，卡 `ui-overlay-deeplink-landing`）。
//!
//! 分层约定（同 `panel_views` / `task_starter`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `deeplink`（20 键）。
//! 数据源在运行时侧：无源时按 fail-closed 渲染「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ============================================================
// 纯数据层（无 GPUI 依赖；视图是它的纯函数）
// ============================================================

/// 视图标题（波次 8 浮层名，取自对齐条目 §11）。
pub const TITLE: &str = "深链落地状态浮层";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// `deeplink.share.expired`（逐字）。
pub const SHARE_EXPIRED: &str = "分享链接已失效";
/// `deeplink.marketDetail.notFound`（逐字）。
pub const MARKET_NOT_FOUND: &str = "内容不存在或已下架";
/// `deeplink.marketDetail.failed`（逐字）。
pub const MARKET_FAILED: &str = "打开失败，请稍后重试";
/// `deeplink.task.connectorAuth.title`（逐字）。
pub const CONNECTOR_AUTH_TITLE: &str = "需要授权连接器";
/// `deeplink.task.connectorAuth.content`（逐字；`{names}` 由 [`connector_auth_content`] 填充）。
pub const CONNECTOR_AUTH_CONTENT: &str = "以下连接器尚未授权：{names}。授权后才能在本任务中使用。";
/// `deeplink.task.connectorAuth.connect`（逐字）。
pub const CONNECTOR_AUTH_CONNECT: &str = "去连接";
/// `deeplink.task.connectorAuth.ignore`（逐字）。
pub const CONNECTOR_AUTH_IGNORE: &str = "忽略";
/// `deeplink.task.cwdNotExist`（逐字；`{path}` 由 [`cwd_skip_notice`] 填充）。
pub const CWD_NOT_EXIST: &str = "工作目录不存在，已跳过：{path}";
/// `deeplink.task.cwdSkippedClaw`（逐字）。
pub const CWD_SKIPPED_CLAW: &str = "Claw 工作区暂不支持作为 deeplink 的目标目录，已跳过：{path}";
/// `deeplink.task.cwdSkippedWeb`（逐字）。
pub const CWD_SKIPPED_WEB: &str = "Web 端不支持本地路径，已忽略 deeplink 中的目标目录：{path}";
/// `deeplink.task.draftOverwrite.title`（逐字）。
pub const DRAFT_OVERWRITE_TITLE: &str = "覆盖当前草稿？";
/// `deeplink.task.draftOverwrite.content`（逐字）。
pub const DRAFT_OVERWRITE_CONTENT: &str = "当前输入框有未发送内容，继续将用新任务覆盖。是否继续？";
/// `deeplink.task.draftOverwrite.ok`（逐字）。
pub const DRAFT_OVERWRITE_OK: &str = "覆盖";
/// `deeplink.task.draftOverwrite.cancel`（逐字）。
pub const DRAFT_OVERWRITE_CANCEL: &str = "取消";
/// `deeplink.task.error.missingRequired`（逐字）。
pub const ERROR_MISSING_REQUIRED: &str = "链接缺少必要参数（prompt 或 payloadId），无法创建任务";
/// `deeplink.task.error.payload.error`（逐字）。
pub const ERROR_PAYLOAD_ERROR: &str = "短连接解析失败，请稍后重试";
/// `deeplink.task.error.payload.expired`（逐字）。
pub const ERROR_PAYLOAD_EXPIRED: &str = "短连接已过期，请重新获取";
/// `deeplink.task.error.payload.forbidden`（逐字）。
pub const ERROR_PAYLOAD_FORBIDDEN: &str = "无权访问该短连接";
/// `deeplink.task.error.payloadUnavailable`（逐字）。
pub const ERROR_PAYLOAD_UNAVAILABLE: &str = "短连接解析暂不可用，请稍后重试";
/// `deeplink.task.skills.notFound`（逐字；`{names}` 由 [`skills_not_found`] 填充）。
pub const SKILLS_NOT_FOUND: &str =
    "未匹配到以下技能：{names}。请检查 deeplink 中的 skill 名（支持技能 ID、英文名或显示名）";

/// 填充 `connectorAuth.content` 的 `{names}`（连接器名以「、」连接）。
#[must_use]
pub fn connector_auth_content(names: &[String]) -> String {
    CONNECTOR_AUTH_CONTENT.replacen("{names}", &names.join("、"), 1)
}

/// 填充 `skills.notFound` 的 `{names}`（技能名以「、」连接）。
#[must_use]
pub fn skills_not_found(names: &[String]) -> String {
    SKILLS_NOT_FOUND.replacen("{names}", &names.join("、"), 1)
}

/// 目标目录被跳过的原因（对应 `deeplink.task.cwd*` 三键）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CwdSkipKind {
    /// 目录不存在（`cwdNotExist`）。
    NotExist,
    /// Claw 工作区不支持（`cwdSkippedClaw`）。
    ClawUnsupported,
    /// Web 端不支持本地路径（`cwdSkippedWeb`）。
    WebUnsupported,
}

/// 填充对应 `cwd*` 键的 `{path}`。
#[must_use]
pub fn cwd_skip_notice(kind: CwdSkipKind, path: &str) -> String {
    let template = match kind {
        CwdSkipKind::NotExist => CWD_NOT_EXIST,
        CwdSkipKind::ClawUnsupported => CWD_SKIPPED_CLAW,
        CwdSkipKind::WebUnsupported => CWD_SKIPPED_WEB,
    };
    template.replacen("{path}", path, 1)
}

/// 任务短链接错误种类（逐字取自 `deeplink.task.error.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskErrorKind {
    /// 缺必要参数（`error.missingRequired`）。
    MissingRequired,
    /// 解析失败（`error.payload.error`）。
    PayloadError,
    /// 已过期（`error.payload.expired`）。
    PayloadExpired,
    /// 无权访问（`error.payload.forbidden`）。
    PayloadForbidden,
    /// 解析暂不可用（`error.payloadUnavailable`）。
    PayloadUnavailable,
}

impl TaskErrorKind {
    /// 错误文案（逐字）。
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::MissingRequired => ERROR_MISSING_REQUIRED,
            Self::PayloadError => ERROR_PAYLOAD_ERROR,
            Self::PayloadExpired => ERROR_PAYLOAD_EXPIRED,
            Self::PayloadForbidden => ERROR_PAYLOAD_FORBIDDEN,
            Self::PayloadUnavailable => ERROR_PAYLOAD_UNAVAILABLE,
        }
    }
}

/// 浮层状态机的可枚举状态（fail-closed：数据未下发时停在 [`State::NotDelivered`]）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据未下发（默认；如实显示「未下发」，⛔ 不得伪造）。
    #[default]
    NotDelivered,
    /// 分享链接已失效（`deeplink.share.expired`）。
    ShareExpired,
    /// 目标内容不存在或已下架（`deeplink.marketDetail.notFound`）。
    MarketNotFound,
    /// 打开失败（`deeplink.marketDetail.failed`）。
    MarketFailed,
    /// 任务短链接错误（`deeplink.task.error.*`，5 种）。
    TaskError(TaskErrorKind),
    /// 需要先授权连接器（`deeplink.task.connectorAuth.*`）。
    ConnectorAuth,
}

impl State {
    /// 状态主文案（逐字；未下发 ⇒ 「未下发」）。
    #[must_use]
    pub fn status_text(self) -> &'static str {
        match self {
            Self::NotDelivered => NOT_DELIVERED,
            Self::ShareExpired => SHARE_EXPIRED,
            Self::MarketNotFound => MARKET_NOT_FOUND,
            Self::MarketFailed => MARKET_FAILED,
            Self::TaskError(kind) => kind.text(),
            Self::ConnectorAuth => CONNECTOR_AUTH_TITLE,
        }
    }

    /// 主操作按钮文案（仅连接器授权分支有「去连接」）。
    #[must_use]
    pub fn primary_action(self) -> Option<&'static str> {
        match self {
            Self::ConnectorAuth => Some(CONNECTOR_AUTH_CONNECT),
            _ => None,
        }
    }

    /// 次操作按钮文案（仅连接器授权分支有「忽略」）。
    #[must_use]
    pub fn secondary_action(self) -> Option<&'static str> {
        match self {
            Self::ConnectorAuth => Some(CONNECTOR_AUTH_IGNORE),
            _ => None,
        }
    }
}

/// 浮层纯数据模型（无 GPUI 依赖；视图是它的纯函数）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeeplinkLandingModel {
    /// 当前状态。
    pub state: State,
    /// 连接器授权分支：尚未授权的连接器名（未下发 ⇒ 空；⛔ 不伪造）。
    pub pending_connectors: Vec<String>,
    /// 落地过程中被跳过的目标目录提示（原因 + 路径；⛔ 不伪造）。
    pub cwd_skips: Vec<(CwdSkipKind, String)>,
    /// 未匹配到的技能名（⛔ 不伪造）。
    pub unmatched_skills: Vec<String>,
    /// 输入框存在未发送草稿（草稿覆盖确认块仅真实冲突位时显示）。
    pub draft_conflict: bool,
}

impl DeeplinkLandingModel {
    /// 未下发时的诚实模型（fail-closed 默认）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self::default()
    }

    /// 由下发的落地结果推导状态，优先级：链接失效 > 内容不存在 > 连接器未授权；
    /// 三者皆否 ⇒ 「打开失败」兜底（本函数只在落地需提示时被调用）。
    /// 空输入（未失效但目标不存在且无连接器）⇒ 「内容不存在或已下架」空态（⛔ 不是空列表）。
    #[must_use]
    pub fn from_delivered(share_expired: bool, found: bool, pending_connectors: &[String]) -> Self {
        let state = if share_expired {
            State::ShareExpired
        } else if !found {
            State::MarketNotFound
        } else if pending_connectors.is_empty() {
            State::MarketFailed
        } else {
            State::ConnectorAuth
        };
        Self {
            state,
            pending_connectors: pending_connectors.to_vec(),
            ..Self::default()
        }
    }

    /// 当前可渲染的提示行数（cwd 跳过 + 技能未匹配；未下发 ⇒ 0；⛔ 不伪造）。
    #[must_use]
    pub fn notice_count(&self) -> usize {
        match self.state {
            State::NotDelivered => 0,
            _ => self.cwd_skips.len() + usize::from(!self.unmatched_skills.is_empty()),
        }
    }

    /// 是否渲染草稿覆盖确认块（仅真实下发的冲突位）。
    #[must_use]
    pub fn shows_draft_overwrite(&self) -> bool {
        self.draft_conflict && self.state != State::NotDelivered
    }
}

/// 主区渲染（GPUI 层随后续提交补全；当前保持诚实占位）。
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(TITLE))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}
#[cfg(test)]
mod tests {
    use super::{main_area, State, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
    }
    #[test]
    fn title_not_empty() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
