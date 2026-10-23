//! 智能体邮箱主区视图（波次 5，卡 `ui-agent-mail-pane`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖；不开 `feature = "ui"` 也可编译/测试）：
//!   开通状态三态（未开通/开通中/已开通）+ 别名/地址行 + 启停用操作行；
//! - **GPUI 层**（[`main_area`]）：把纯数据层接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自文案全表命名空间 `agentMail`）。
//!
//! 数据源在运行时侧：等跨线契约就绪后由 `ui-*-switch-source` 类卡换数据源
//! （**只换数据源、不改视图结构**）。

#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::Theme;
#[cfg(feature = "ui")]
use gpui_kit::*;

// ---------------------------------------------------------------------------
// 纯数据层（无 GPUI 依赖）
// ---------------------------------------------------------------------------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "智能体邮箱";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 账号信息分组的标题行文案（逐字）。
pub const ACCOUNT_INFO_GROUP: &str = "智能体邮箱账号信息";

/// 未开通状态徽标文案（逐字）。
pub const STATUS_NOT_ACTIVATED: &str = "邮箱尚未开通";

/// 开通中状态徽标文案（逐字）。
pub const STATUS_ACTIVATING: &str = "开通中";

/// 已开通（运行中）状态徽标文案（逐字）。
pub const STATUS_RUNNING: &str = "运行中";

/// 未开通时的引导提示行（逐字）。
pub const HINT_NOT_ACTIVATED: &str =
    "智能体邮箱是为你的 Workbuddy 创建的邮箱。你可以在 Workbuddy 对话中接收或发送邮件。";

/// 开通中时的提示行（逐字）。
pub const HINT_ACTIVATING: &str = "正在开通中，请稍候";

/// 已开通别名/地址行模板（逐字；`{alias}` 在构造时替换，模板本身保持原文）。
pub const ACTIVATED_SUCCESS: &str = "邮箱已开通：{alias}";

/// 空态说明文案（逐字；空态 ≠ 空列表）。
pub const EMPTY_HINT: &str = "暂无匹配邮件";

/// 操作行文案：停用（逐字）。
pub const ACTION_DEACTIVATE: &str = "停用";

/// 操作行文案：前往邮箱（逐字）。
pub const ACTION_GOTO_MAILBOX: &str = "前往邮箱";

/// 操作行文案：确认开通（逐字）。
pub const ACTION_CONFIRM_ACTIVATE: &str = "确认开通";

/// 开通状态三态：未开通 / 开通中 / 已开通。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activation {
    /// 未开通。
    NotActivated,
    /// 开通中。
    Activating,
    /// 已开通（运行中）。
    Activated,
}

impl Activation {
    /// 状态徽标文案（逐字）。
    #[must_use]
    pub fn status_label(self) -> &'static str {
        match self {
            Self::NotActivated => STATUS_NOT_ACTIVATED,
            Self::Activating => STATUS_ACTIVATING,
            Self::Activated => STATUS_RUNNING,
        }
    }

    /// 开通引导提示行（已开通时无提示）。
    #[must_use]
    pub fn hint_label(self) -> Option<&'static str> {
        match self {
            Self::NotActivated => Some(HINT_NOT_ACTIVATED),
            Self::Activating => Some(HINT_ACTIVATING),
            Self::Activated => None,
        }
    }

    /// 启停用操作行文案（开通中无操作）。
    #[must_use]
    pub fn action_labels(self) -> &'static [&'static str] {
        match self {
            Self::Activated => &[ACTION_DEACTIVATE, ACTION_GOTO_MAILBOX],
            Self::NotActivated => &[ACTION_CONFIRM_ACTIVATE],
            Self::Activating => &[],
        }
    }
}

/// 已开通别名/地址行：对逐字模板做 `{alias}` 替换。
#[must_use]
pub fn activated_success_line(alias: &str) -> String {
    ACTIVATED_SUCCESS.replace("{alias}", alias)
}

/// 清单/分组行（可枚举；GPUI 层逐行渲染）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// 分组标题行。
    Group(&'static str),
    /// 状态徽标行。
    Status(&'static str),
    /// 引导提示行。
    Hint(&'static str),
    /// 别名/地址行。
    Alias(String),
    /// 操作行（按钮文案集合）。
    Actions(&'static [&'static str]),
}

/// 面板数据状态：未下发（fail-closed）/ 空态 / 清单。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneState {
    /// 数据未下发 ⇒ 如实显示「未下发」。
    NotDelivered,
    /// 已下发但无任何可展示内容 ⇒ 空态（⛔ 不是空清单）。
    Empty,
    /// 账号清单：首个一定是分组行，且总行数 ≥ 3。
    Entries(Vec<Row>),
}

/// 智能体邮箱面板的纯数据视图。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentMailPane {
    /// 标题行文案。
    pub title: &'static str,
    /// 面板数据状态。
    pub state: PaneState,
}

/// 运行时账号快照（跨线契约就绪前的数据形状；`None` 字段表示该字段未下发）。
pub struct AccountSnapshot {
    /// 开通状态。
    pub activation: Option<Activation>,
    /// 邮箱别名/地址。
    pub alias: Option<String>,
}

/// 构造器：由账号快照推导面板状态。
///
/// - 快照未下发（`None`）⇒ [`PaneState::NotDelivered`]（fail-closed）；
/// - 快照已下发但两个字段皆缺 ⇒ [`PaneState::Empty`]（空态，不是空清单）；
/// - 其余 ⇒ 分组行 + 状态/提示/别名/操作行（≥3 行）。
#[must_use]
pub fn build(snapshot: Option<&AccountSnapshot>) -> AgentMailPane {
    let Some(snapshot) = snapshot else {
        return AgentMailPane {
            title: TITLE,
            state: PaneState::NotDelivered,
        };
    };
    let mut rows = Vec::new();
    if let Some(activation) = snapshot.activation {
        rows.push(Row::Status(activation.status_label()));
        if let Some(hint) = activation.hint_label() {
            rows.push(Row::Hint(hint));
        }
    }
    if let Some(alias) = &snapshot.alias {
        rows.push(Row::Alias(activated_success_line(alias)));
    }
    if let Some(activation) = snapshot.activation {
        let actions = activation.action_labels();
        if !actions.is_empty() {
            rows.push(Row::Actions(actions));
        }
    }
    let state = if rows.is_empty() {
        PaneState::Empty
    } else {
        rows.insert(0, Row::Group(ACCOUNT_INFO_GROUP));
        PaneState::Entries(rows)
    };
    AgentMailPane {
        title: TITLE,
        state,
    }
}

// ---------------------------------------------------------------------------
// GPUI 层
// ---------------------------------------------------------------------------

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
#[cfg(feature = "ui")]
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
        activated_success_line, build, AccountSnapshot, Activation, PaneState, Row,
        ACCOUNT_INFO_GROUP, ACTION_CONFIRM_ACTIVATE, ACTION_DEACTIVATE, ACTION_GOTO_MAILBOX,
        ACTIVATED_SUCCESS, EMPTY_HINT, HINT_ACTIVATING, HINT_NOT_ACTIVATED, NOT_DELIVERED,
        STATUS_ACTIVATING, STATUS_NOT_ACTIVATED, STATUS_RUNNING, TITLE,
    };

    /// 数据源未就绪：构造器必须 fail-closed 到「未下发」，不得伪造条目。
    #[test]
    fn build_without_snapshot_is_not_delivered() {
        let pane = build(None);
        assert_eq!(pane.title, TITLE);
        assert_eq!(pane.state, PaneState::NotDelivered);
    }

    /// 已下发快照：分组 + 状态 + 别名 + 操作行，计数与内容可枚举。
    #[test]
    fn delivered_snapshot_rows_group_and_count() {
        let snapshot = AccountSnapshot {
            activation: Some(Activation::Activated),
            alias: Some("work@example.com".to_owned()),
        };
        assert_eq!(
            build(Some(&snapshot)).state,
            PaneState::Entries(vec![
                Row::Group(ACCOUNT_INFO_GROUP),
                Row::Status(STATUS_RUNNING),
                Row::Alias("邮箱已开通：work@example.com".to_owned()),
                Row::Actions(&[ACTION_DEACTIVATE, ACTION_GOTO_MAILBOX]),
            ])
        );
    }

    /// 标题节点 + ≥3 个清单/分组节点（元素树的数据侧形状）。
    #[test]
    fn delivered_rows_have_title_and_at_least_three_nodes() {
        for activation in [
            Activation::NotActivated,
            Activation::Activating,
            Activation::Activated,
        ] {
            let snapshot = AccountSnapshot {
                activation: Some(activation),
                alias: Some("a@b.c".to_owned()),
            };
            let PaneState::Entries(rows) = build(Some(&snapshot)).state else {
                panic!("{activation:?} 应为清单态");
            };
            assert!(rows.len() >= 3, "{activation:?} 行数不足：{}", rows.len());
            assert!(matches!(rows[0], Row::Group(_)));
        }
    }

    /// 空输入 ⇒ 空态（不是空清单）。
    #[test]
    fn delivered_but_blank_is_empty_state_not_empty_list() {
        let snapshot = AccountSnapshot {
            activation: None,
            alias: None,
        };
        assert_eq!(build(Some(&snapshot)).state, PaneState::Empty);
    }

    /// 文案逐字一致（对多个 key 断言，⛔ 不许自编中文）。
    #[test]
    fn copy_is_verbatim_from_string_table() {
        assert_eq!(TITLE, "智能体邮箱");
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(ACCOUNT_INFO_GROUP, "智能体邮箱账号信息");
        assert_eq!(STATUS_NOT_ACTIVATED, "邮箱尚未开通");
        assert_eq!(STATUS_ACTIVATING, "开通中");
        assert_eq!(STATUS_RUNNING, "运行中");
        assert_eq!(HINT_ACTIVATING, "正在开通中，请稍候");
        assert_eq!(
            HINT_NOT_ACTIVATED,
            "智能体邮箱是为你的 Workbuddy 创建的邮箱。你可以在 Workbuddy 对话中接收或发送邮件。"
        );
        assert_eq!(ACTION_DEACTIVATE, "停用");
        assert_eq!(ACTION_GOTO_MAILBOX, "前往邮箱");
        assert_eq!(ACTION_CONFIRM_ACTIVATE, "确认开通");
        assert_eq!(EMPTY_HINT, "暂无匹配邮件");
        assert_eq!(ACTIVATED_SUCCESS, "邮箱已开通：{alias}");
        assert_eq!(activated_success_line("kimi"), "邮箱已开通：kimi");
    }

    /// 三态徽标与操作行一一对应。
    #[test]
    fn activation_states_map_labels_and_actions() {
        assert_eq!(
            Activation::NotActivated.status_label(),
            STATUS_NOT_ACTIVATED
        );
        assert_eq!(
            Activation::NotActivated.action_labels(),
            [ACTION_CONFIRM_ACTIVATE].as_slice()
        );
        assert_eq!(Activation::Activating.status_label(), STATUS_ACTIVATING);
        assert!(Activation::Activating.action_labels().is_empty());
        assert_eq!(Activation::Activated.status_label(), STATUS_RUNNING);
        assert_eq!(
            Activation::Activated.action_labels(),
            [ACTION_DEACTIVATE, ACTION_GOTO_MAILBOX].as_slice()
        );
    }

    /// `main_area` 不 panic（GPUI 层冒烟）。
    #[cfg(feature = "ui")]
    #[test]
    fn main_area_builds_without_panic() {
        let theme = gpui_kit::component::Theme::default();
        let _ = super::main_area(&theme);
    }
}
