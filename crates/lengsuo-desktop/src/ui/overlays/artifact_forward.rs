//! 转发到团队浮层（波次 8，卡 `ui-overlay-artifact-forward`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `artifactForward`。
//! 数据源在运行时侧：无源时如实显示「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自 `artifactForward.title`）。
pub const TITLE: &str = "转发到团队";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 加载中文案（逐字取自 `artifactForward.joining`）。
pub const JOINING: &str = "正在加载团队会话…";
/// 加载失败文案（逐字取自 `artifactForward.joinFailed`）。
pub const JOIN_FAILED: &str = "团队会话加载失败，请重试";
/// 本地文件缺失文案（逐字取自 `artifactForward.missingFile`）。
pub const MISSING_FILE: &str = "找不到要转发的本地文件";
/// 环境不支持文案（逐字取自 `artifactForward.unavailable`）。
pub const UNAVAILABLE: &str = "当前环境不支持团队转发";
/// 打开失败文案（逐字取自 `artifactForward.openFailed`）。
pub const OPEN_FAILED: &str = "无法打开团队转发，请稍后重试";
/// 成功模板（逐字取自 `artifactForward.success`；占位符 `{count}`）。
pub const SUCCESS_TEMPLATE: &str = "已转发到 {count} 个目标";
/// 部分失败模板（逐字取自 `artifactForward.partialFailure`；占位符 `{successCount}`/`{failedCount}`）。
pub const PARTIAL_FAILURE_TEMPLATE: &str =
    "已成功转发到 {successCount} 个目标，{failedCount} 个失败";
/// 留言失败模板（逐字取自 `artifactForward.noteFailure`；占位符 `{count}`）。
pub const NOTE_FAILURE_TEMPLATE: &str = "文件已发送，{count} 条留言发送失败";

/// 转发结果三分支：成功 / 部分失败 / 留言失败。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// 全部目标成功（`artifactForward.success`）。
    Success { count: usize },
    /// 部分目标失败（`artifactForward.partialFailure`）。
    PartialFailure {
        success_count: usize,
        failed_count: usize,
    },
    /// 文件已发送但有留言发送失败（`artifactForward.noteFailure`）。
    NoteFailure { count: usize },
}

impl Outcome {
    /// 按逐目标成败汇总；空输入 ⇒ `None`（空态由 [`State::from_results`] 归一化，⛔ 不伪造 0 成功）。
    #[must_use]
    pub fn from_results(results: &[bool]) -> Option<Self> {
        if results.is_empty() {
            return None;
        }
        let success_count = results.iter().filter(|&&ok| ok).count();
        let failed_count = results.len() - success_count;
        Some(if failed_count == 0 {
            Self::Success {
                count: success_count,
            }
        } else {
            Self::PartialFailure {
                success_count,
                failed_count,
            }
        })
    }

    /// 填充计数后的完整文案（模板逐字，仅替换占位符）。
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Success { count } => SUCCESS_TEMPLATE.replace("{count}", &count.to_string()),
            Self::PartialFailure {
                success_count,
                failed_count,
            } => PARTIAL_FAILURE_TEMPLATE
                .replace("{successCount}", &success_count.to_string())
                .replace("{failedCount}", &failed_count.to_string()),
            Self::NoteFailure { count } => {
                NOTE_FAILURE_TEMPLATE.replace("{count}", &count.to_string())
            }
        }
    }

    /// 目标级 (成功, 失败) 计数；留言失败不按目标计数 ⇒ `None`。
    #[must_use]
    pub fn target_counts(&self) -> Option<(usize, usize)> {
        match self {
            Self::Success { count } => Some((*count, 0)),
            Self::PartialFailure {
                success_count,
                failed_count,
            } => Some((*success_count, *failed_count)),
            Self::NoteFailure { .. } => None,
        }
    }
}

/// 状态语义色（渲染层映射到主题 token；纯数据层不碰 GPUI）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// 中性（加载 / 空态 / 未下发）。
    Muted,
    /// 成功类。
    Ok,
    /// 失败类。
    Danger,
}

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 环境不支持团队转发。
    Unavailable,
    /// 找不到要转发的本地文件。
    MissingFile,
    /// 正在加载团队会话。
    Joining,
    /// 团队会话加载失败。
    JoinFailed,
    /// 已出结果（成功 / 部分失败 / 留言失败）。
    Done(Outcome),
    /// 打开团队转发失败。
    OpenFailed,
    /// 目标清单为空（空态，⛔ 不是空清单、不是未下发）。
    Empty,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

impl State {
    /// 按逐目标成败汇总构造；空输入 ⇒ 空态 [`State::Empty`]。
    #[must_use]
    pub fn from_results(results: &[bool]) -> Self {
        match Outcome::from_results(results) {
            Some(outcome) => Self::Done(outcome),
            None => Self::Empty,
        }
    }

    /// 当前状态主文案（逐字模板 + 计数填充；空态无主文案）。
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Unavailable => UNAVAILABLE.to_owned(),
            Self::MissingFile => MISSING_FILE.to_owned(),
            Self::Joining => JOINING.to_owned(),
            Self::JoinFailed => JOIN_FAILED.to_owned(),
            Self::Done(outcome) => outcome.message(),
            Self::OpenFailed => OPEN_FAILED.to_owned(),
            Self::Empty => String::new(),
            Self::NotDelivered => NOT_DELIVERED.to_owned(),
        }
    }

    /// 状态语义色（加载/空态/未下发中性、成功强调、失败危险；色值由渲染层取主题 token）。
    #[must_use]
    pub fn tone(&self) -> Tone {
        match self {
            Self::Unavailable
            | Self::MissingFile
            | Self::JoinFailed
            | Self::Done(Outcome::PartialFailure { .. })
            | Self::Done(Outcome::NoteFailure { .. })
            | Self::OpenFailed => Tone::Danger,
            Self::Done(Outcome::Success { .. }) => Tone::Ok,
            Self::Joining | Self::Empty | Self::NotDelivered => Tone::Muted,
        }
    }

    /// 目标级计数（仅结果态有；见 [`Outcome::target_counts`]）。
    #[must_use]
    pub fn target_counts(&self) -> Option<(usize, usize)> {
        match self {
            Self::Done(outcome) => outcome.target_counts(),
            _ => None,
        }
    }
}
/// 主区渲染（无数据源时的 fail-closed 默认态：标题 + 「未下发」）。
pub fn main_area(theme: &Theme) -> Div {
    render_overlay(theme, &State::default())
}

/// 按纯数据层状态渲染整层：标题行 + 状态卡（消息行 + 计数行）+ 空态/未下发分支。
fn render_overlay(theme: &Theme, state: &State) -> Div {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .child(header_row(theme))
        .child(body(theme, state))
}

/// 标题行：视图标题。
fn header_row(theme: &Theme) -> Div {
    div().flex().items_center().px_4().py_2().child(
        div()
            .text_sm()
            .font_bold()
            .text_color(theme.foreground)
            .child(TITLE),
    )
}

/// 内容区分支：未下发（fail-closed）/ 空态（不渲染清单）/ 状态卡。
fn body(theme: &Theme, state: &State) -> Div {
    match state {
        State::NotDelivered => centered_note(theme, NOT_DELIVERED),
        State::Empty => div().flex_1(),
        _ => status_card(theme, state),
    }
}

/// 居中说明（未下发）。
fn centered_note(theme: &Theme, text: &'static str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 状态卡：状态点 + 主文案行 +（结果态）目标计数行。
fn status_card(theme: &Theme, state: &State) -> Div {
    let mut card = div()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .pb_4()
        .child(status_row(theme, state.message(), state.tone()));
    if let Some((success_count, failed_count)) = state.target_counts() {
        card = card.child(counts_row(theme, success_count, failed_count));
    }
    card
}

/// 状态点 + 主文案行（主文案 = 逐字模板填充计数）。
fn status_row(theme: &Theme, message: String, tone: Tone) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(status_dot(theme, tone))
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(SharedString::from(message)),
        )
}

/// 状态点（颜色按语义取主题 token，⛔ 不硬编码色值）。
fn status_dot(theme: &Theme, tone: Tone) -> Div {
    let color = match tone {
        Tone::Muted => theme.muted_foreground,
        Tone::Ok => theme.accent,
        Tone::Danger => theme.danger,
    };
    div().w_2().h_2().rounded_full().bg(color)
}

/// 目标计数行：成功 / 失败两枚计数 chip（纯数值，语义色取主题 token）。
fn counts_row(theme: &Theme, success_count: usize, failed_count: usize) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(count_chip(theme, success_count, Tone::Ok))
        .child(count_chip(theme, failed_count, Tone::Danger))
}

/// 单枚计数 chip（纯数值；⛔ 不伪造条目名/数值，数值全部来自纯数据层计数）。
fn count_chip(theme: &Theme, value: usize, tone: Tone) -> Div {
    let color = match tone {
        Tone::Muted => theme.muted_foreground,
        Tone::Ok => theme.accent,
        Tone::Danger => theme.danger,
    };
    div()
        .px_2()
        .py_1()
        .border_1()
        .border_color(color)
        .rounded_sm()
        .text_xs()
        .text_color(color)
        .child(SharedString::from(value.to_string()))
}
#[cfg(test)]
mod tests {
    use super::{
        main_area, render_overlay, Outcome, State, Tone, JOINING, JOIN_FAILED, MISSING_FILE,
        NOTE_FAILURE_TEMPLATE, NOT_DELIVERED, OPEN_FAILED, PARTIAL_FAILURE_TEMPLATE,
        SUCCESS_TEMPLATE, TITLE, UNAVAILABLE,
    };
    use gpui_kit::component::Theme;

    /// ③ 文案逐字一致：`artifactForward` 命名空间全 9 键。
    #[test]
    fn copy_verbatim_from_namespace() {
        assert_eq!(TITLE, "转发到团队");
        assert_eq!(JOINING, "正在加载团队会话…");
        assert_eq!(JOIN_FAILED, "团队会话加载失败，请重试");
        assert_eq!(MISSING_FILE, "找不到要转发的本地文件");
        assert_eq!(UNAVAILABLE, "当前环境不支持团队转发");
        assert_eq!(OPEN_FAILED, "无法打开团队转发，请稍后重试");
        assert_eq!(SUCCESS_TEMPLATE, "已转发到 {count} 个目标");
        assert_eq!(
            PARTIAL_FAILURE_TEMPLATE,
            "已成功转发到 {successCount} 个目标，{failedCount} 个失败"
        );
        assert_eq!(NOTE_FAILURE_TEMPLATE, "文件已发送，{count} 条留言发送失败");
    }

    /// ① 纯函数层状态/计数正确。
    #[test]
    fn from_results_builds_outcome_and_counts() {
        let all_ok = State::from_results(&[true, true, true]);
        assert_eq!(all_ok, State::Done(Outcome::Success { count: 3 }));
        assert_eq!(all_ok.target_counts(), Some((3, 0)));
        let mixed = State::from_results(&[true, false, true, false]);
        assert_eq!(
            mixed,
            State::Done(Outcome::PartialFailure {
                success_count: 2,
                failed_count: 2
            })
        );
        assert_eq!(mixed.target_counts(), Some((2, 2)));
        // 留言失败不按目标计数 ⇒ 无计数行数据。
        assert_eq!(
            State::Done(Outcome::NoteFailure { count: 4 }).target_counts(),
            None
        );
    }

    /// ② 空输入 ⇒ 空态（不是空清单、不是未下发）；默认态 fail-closed。
    #[test]
    fn empty_input_is_empty_state_and_default_fail_closed() {
        assert_eq!(Outcome::from_results(&[]), None);
        assert_eq!(State::from_results(&[]), State::Empty);
        assert_ne!(State::from_results(&[]), State::NotDelivered);
        assert_eq!(State::default(), State::NotDelivered);
    }

    /// 占位符填充：仅替换 `{count}` 等，模板其余部分逐字。
    #[test]
    fn message_fills_placeholders_verbatim() {
        assert_eq!(Outcome::Success { count: 3 }.message(), "已转发到 3 个目标");
        assert_eq!(
            Outcome::PartialFailure {
                success_count: 2,
                failed_count: 1
            }
            .message(),
            "已成功转发到 2 个目标，1 个失败"
        );
        assert_eq!(
            Outcome::NoteFailure { count: 4 }.message(),
            "文件已发送，4 条留言发送失败"
        );
    }

    /// ⑤ 状态分支覆盖：每个分支一条（主文案 + 语义色）。
    #[test]
    fn every_state_branch_has_message_and_tone() {
        let branches = [
            (State::Unavailable, UNAVAILABLE, Tone::Danger),
            (State::MissingFile, MISSING_FILE, Tone::Danger),
            (State::Joining, JOINING, Tone::Muted),
            (State::JoinFailed, JOIN_FAILED, Tone::Danger),
            (State::OpenFailed, OPEN_FAILED, Tone::Danger),
            (
                State::Done(Outcome::Success { count: 1 }),
                "已转发到 1 个目标",
                Tone::Ok,
            ),
            (
                State::Done(Outcome::PartialFailure {
                    success_count: 1,
                    failed_count: 1,
                }),
                "已成功转发到 1 个目标，1 个失败",
                Tone::Danger,
            ),
            (
                State::Done(Outcome::NoteFailure { count: 2 }),
                "文件已发送，2 条留言发送失败",
                Tone::Danger,
            ),
            (State::Empty, "", Tone::Muted),
            (State::NotDelivered, NOT_DELIVERED, Tone::Muted),
        ];
        for (state, expected, tone) in branches {
            assert_eq!(state.message(), expected, "message for {state:?}");
            assert_eq!(state.tone(), tone, "tone for {state:?}");
        }
    }

    /// ④ `main_area` 与全分支 `render_overlay` 不 panic。
    #[test]
    fn render_all_branches_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
        let states = [
            State::Unavailable,
            State::MissingFile,
            State::Joining,
            State::JoinFailed,
            State::OpenFailed,
            State::Empty,
            State::NotDelivered,
            State::from_results(&[true]),
            State::from_results(&[true, false]),
            State::Done(Outcome::NoteFailure { count: 3 }),
        ];
        for state in &states {
            let _ = render_overlay(&theme, state);
        }
    }
}
