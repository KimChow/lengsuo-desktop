//! 自动化·错误（波次 9，卡 `ui-overlay-automation-error`）。
//!
//! 分层约定（同 `automation_ds_form` / `credit_hint`）：纯数据层 + GPUI 层（`main_area`）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `automation.error`。
//! 全表计 56 键（docs/ui-parity-spec.md 计数行）；本卡随任务书逐字摘录 12 键，
//! 其余键未随卡下发 ⇒ 不收录（⛔ 禁止自编，待全表下发后补齐）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字；沿用波次 9 脚手架占位值，出处文案全表）。
pub const TITLE: &str = "自动化·错误";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 加载态文案（逐字；与同族卡片 `credit_hint` 同源全表键）。
pub const TEXT_LOADING: &str = "加载中...";
/// 空态文案（逐字；与同族卡片 `credit_hint` 同源全表键）。
pub const TEXT_EMPTY: &str = "暂无内容";

/// 命名空间前缀：完整键 = `NS_PREFIX` + "." + 短名。
pub const NS_PREFIX: &str = "automation.error";

/// 界面文案表：`automation.error` 命名空间（短名，按键排序，逐字摘录）。
/// 12 键 = `agentMissing` + 11 个 `code.*` 错误码；⛔ 增删改均破坏文案契约。
pub const TEXTS: [(&str, &str); 12] = [
    ("agentMissing", "未找到频道对应的助理，请稍后再试"),
    ("code.CHANNEL_NOT_ALLOWED", "投递渠道与任务类型不匹配"),
    (
        "code.CRON_INTERVAL_TOO_SHORT",
        "触发频率过高，最低每 5 分钟执行一次",
    ),
    ("code.INTERNAL_ERROR", "服务异常，请稍后再试"),
    (
        "code.INVALID_CRON_EXPR",
        "定时设置无效，请检查频率或格式后重试",
    ),
    ("code.INVALID_PARAM", "请求参数有误，请检查后重试"),
    ("code.INVALID_TOKEN", "凭证无效或已过期"),
    ("code.LOG_NOT_FOUND", "运行记录不存在"),
    (
        "code.NOT_PROJECT_MEMBER",
        "你不是该项目成员，无权执行此操作",
    ),
    ("code.OWNER_TYPE_IMMUTABLE", "任务归属不可修改"),
    ("code.PROJECT_NOT_FOUND", "项目不存在或已被删除"),
    (
        "code.RESOURCE_NOT_IN_PROJECT",
        "引用的资源不在项目可用范围内",
    ),
];

/// 按完整键（`前缀.短名`）取文案；未命中 ⇒ `None`（fail-closed，⛔ 不回退到自编文案）。
pub fn text(key: &str) -> Option<&'static str> {
    let name = key.strip_prefix(NS_PREFIX)?.strip_prefix('.')?;
    TEXTS.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

/// 按服务端错误码（如 `INVALID_TOKEN`）取 `code.*` 文案；未命中 ⇒ `None`。
pub fn code_text(code: &str) -> Option<&'static str> {
    text(&format!("{NS_PREFIX}.code.{code}"))
}

/// 视图态状态机：加载中 / 失败（fail-closed）/ 空态 / 正常。
/// 未拉取过 ≠ 空集：默认态必须是加载中（⛔ 不默认空态、不伪造错误码）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AutomationErrorState {
    /// 加载中：拉取未返回。
    #[default]
    Loading,
    /// 拉取失败：fail-closed 必须可见，⛔ 不得退化成空态。
    LoadFailed,
    /// 空态：已拉取且无错误。
    Empty,
    /// 正常：已拉取到服务端错误码。
    Ready {
        /// 服务端错误码（如 `INVALID_TOKEN`）。
        code: String,
    },
}

/// 由拉取结果解析视图态（纯函数）：未返回 ⇒ 加载中；失败 ⇒ fail-closed；
/// 成功无码（含空白码，视为无错误）⇒ 空态；成功有码 ⇒ 正常。
pub fn resolve(outcome: Option<Result<Option<&str>, ()>>) -> AutomationErrorState {
    match outcome {
        None => AutomationErrorState::Loading,
        Some(Err(())) => AutomationErrorState::LoadFailed,
        Some(Ok(None)) => AutomationErrorState::Empty,
        Some(Ok(Some(code))) => match code.trim() {
            "" => AutomationErrorState::Empty,
            trimmed => AutomationErrorState::Ready {
                code: trimmed.to_string(),
            },
        },
    }
}

impl AutomationErrorState {
    /// 正常态展示文案：已知码 ⇒ 逐字文案；未知码 ⇒ `INTERNAL_ERROR` 逐字兜底（⛔ 不自编）。
    /// 非正常态 ⇒ `None`（各分支渲染自理，⛔ 不混用文案）。
    #[must_use]
    pub fn message(&self) -> Option<&'static str> {
        match self {
            Self::Ready { code } => code_text(code).or(code_text("INTERNAL_ERROR")),
            _ => None,
        }
    }

    /// 正常态原始错误码（服务端数据，如实展示）；其余态 ⇒ `None`。
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Ready { code } => Some(code.as_str()),
            _ => None,
        }
    }
}

// ── GPUI 层（颜色一律 theme token，⛔ 不硬编码色值） ──────────────────

/// 浮层渲染（波次 9：浮层不占主区，返回整层容器由调用方叠加）。
/// 数据源在运行时侧：`main_area` 以默认态（加载中）渲染，等跨线契约就绪后换源。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &AutomationErrorState::default())
}

/// 按视图态渲染整层：标题行 + 状态分支主体（加载/失败/空态/正常）。
#[must_use]
pub fn render(theme: &Theme, state: &AutomationErrorState) -> Div {
    let body = match state {
        AutomationErrorState::Loading => centered_line(theme, TEXT_LOADING),
        AutomationErrorState::LoadFailed => centered_line(theme, NOT_DELIVERED),
        AutomationErrorState::Empty => centered_line(theme, TEXT_EMPTY),
        AutomationErrorState::Ready { .. } => error_body(theme, state),
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
                .child(TITLE),
        )
        .child(body)
}

/// 居中单行（加载 / 失败 fail-closed / 空态共用）。
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

/// 正常态主体：错误文案（逐字）+ 原始错误码 chip（服务端数据，如实展示）。
fn error_body(theme: &Theme, state: &AutomationErrorState) -> Div {
    let message = state.message().unwrap_or(NOT_DELIVERED);
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .py_3()
        .child(div().text_sm().text_color(theme.foreground).child(message))
        .child(
            div()
                .self_start()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(state.code().unwrap_or_default().to_string()),
        )
}
