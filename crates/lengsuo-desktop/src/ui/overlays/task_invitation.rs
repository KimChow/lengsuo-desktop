//! 任务邀请浮层（波次 8，卡 `ui-overlay-task-invitation`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `taskInvitation`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "任务邀请浮层";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

// ===== 纯数据层（无 GPUI 依赖）=====
// 以下 14 条界面文案逐字取自文案全表命名空间 `taskInvitation`（键清单见 [`KEYS`]）。

/// `taskInvitation.alreadyMemberDesc`
pub const ALREADY_MEMBER_DESC: &str = "无需重复加入，可直接打开任务继续协作。";
/// `taskInvitation.alreadyMemberTitle`
pub const ALREADY_MEMBER_TITLE: &str = "你已是该任务的协作者";
/// `taskInvitation.error.linkInvalidDesc`
pub const LINK_INVALID_DESC: &str = "该邀请已过期、已被撤回或已被使用，请联系邀请人重新发送。";
/// `taskInvitation.error.loadFailed`
pub const LOAD_FAILED: &str = "加载失败";
/// `taskInvitation.error.loginRequiredDesc`
pub const LOGIN_REQUIRED_DESC: &str = "登录后才能查看并处理这份协作邀请。";
/// `taskInvitation.error.submitFailedDesc`
pub const SUBMIT_FAILED_DESC: &str = "接受邀请失败，请稍后重试。";
/// `taskInvitation.inviterSuffix`（前导空格逐字保留）
pub const INVITER_SUFFIX: &str = " 邀请你协作此任务";
/// `taskInvitation.joinAction`
pub const JOIN_ACTION: &str = "接受邀请";
/// `taskInvitation.note`
pub const NOTE: &str = "你尚未加入该项目，接受后需等待管理员审批";
/// `taskInvitation.openTask`
pub const OPEN_TASK: &str = "打开任务";
/// `taskInvitation.result.acceptedBannerText`
pub const ACCEPTED_BANNER_TEXT: &str = "已加入协作";
/// `taskInvitation.result.acceptedRedirectDesc`
pub const ACCEPTED_REDIRECT_DESC: &str = "正在跳转到任务…";
/// `taskInvitation.result.pendingApprovalDesc`
pub const PENDING_APPROVAL_DESC: &str =
    "你尚未加入该项目，已向管理员发送加入申请，审批通过后可加入协作";
/// `taskInvitation.untitledTask`
pub const UNTITLED_TASK: &str = "未命名任务";

/// 命名空间 `taskInvitation` 键清单（计数/唯一性自检用）。
pub const KEYS: [&str; 14] = [
    "taskInvitation.alreadyMemberDesc",
    "taskInvitation.alreadyMemberTitle",
    "taskInvitation.error.linkInvalidDesc",
    "taskInvitation.error.loadFailed",
    "taskInvitation.error.loginRequiredDesc",
    "taskInvitation.error.submitFailedDesc",
    "taskInvitation.inviterSuffix",
    "taskInvitation.joinAction",
    "taskInvitation.note",
    "taskInvitation.openTask",
    "taskInvitation.result.acceptedBannerText",
    "taskInvitation.result.acceptedRedirectDesc",
    "taskInvitation.result.pendingApprovalDesc",
    "taskInvitation.untitledTask",
];

/// 浮层状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 邀请有效，待接受。
    Pending,
    /// 接受成功，正在跳转任务。
    Accepted,
    /// 接受成功，等待管理员审批。
    PendingApproval,
    /// 已是该任务的协作者（⛔ 不可再接受）。
    AlreadyMember,
    /// 邀请过期/撤回/已使用（错误·过期）。空输入也落此空态，⛔ 不是空列表。
    LinkInvalid,
    /// 未登录（错误·登录）。
    LoginRequired,
    /// 接受提交失败（错误·提交失败）。
    SubmitFailed,
    /// 拉取邀请失败。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 运行时输入（`None` ⇒ 无数据源，fail-closed）。
pub struct Input {
    pub logged_in: bool,
    pub load_failed: bool,
    /// `None` = 无邀请记录（空输入 ⇒ [`State::LinkInvalid`] 空态）；`Some(already_member)`。
    pub invitation: Option<bool>,
}

/// 接受邀请事件（状态转移）。
pub enum Event {
    /// 接受成功；`pending_approval` = 还需管理员审批。
    AcceptSucceeded { pending_approval: bool },
    /// 接受提交失败。
    AcceptFailed,
}

impl State {
    /// 由运行时输入解析初始态（纯函数）。
    pub fn resolve(input: Option<&Input>) -> State {
        let Some(input) = input else {
            return State::NotDelivered;
        };
        if !input.logged_in {
            return State::LoginRequired;
        }
        if input.load_failed {
            return State::LoadFailed;
        }
        match input.invitation {
            None => State::LinkInvalid,
            Some(true) => State::AlreadyMember,
            Some(false) => State::Pending,
        }
    }

    /// 状态转移（纯函数；仅 [`State::Pending`] 可接受邀请，其余态保持不变）。
    pub fn on(self, event: Event) -> State {
        match (self, event) {
            (State::Pending, Event::AcceptSucceeded { pending_approval: true }) => {
                State::PendingApproval
            }
            (State::Pending, Event::AcceptSucceeded { pending_approval: false }) => State::Accepted,
            (State::Pending, Event::AcceptFailed) => State::SubmitFailed,
            (other, _) => other,
        }
    }

    /// 主文案（逐字取自命名空间常量）。
    pub fn headline(&self) -> &'static str {
        match self {
            State::Pending => UNTITLED_TASK,
            State::Accepted | State::PendingApproval => ACCEPTED_BANNER_TEXT,
            State::AlreadyMember => ALREADY_MEMBER_TITLE,
            State::LinkInvalid => LINK_INVALID_DESC,
            State::LoginRequired => LOGIN_REQUIRED_DESC,
            State::SubmitFailed => SUBMIT_FAILED_DESC,
            State::LoadFailed => LOAD_FAILED,
            State::NotDelivered => NOT_DELIVERED,
        }
    }

    /// 次文案（逐字；无则为空切片）。
    pub fn detail_lines(&self) -> &'static [&'static str] {
        match self {
            State::Pending => &[INVITER_SUFFIX, NOTE],
            State::Accepted => &[ACCEPTED_REDIRECT_DESC],
            State::PendingApproval => &[PENDING_APPROVAL_DESC],
            State::AlreadyMember => &[ALREADY_MEMBER_DESC],
            _ => &[],
        }
    }

    /// 动作文案（逐字；无动作分支为 `None`）。
    pub fn action_label(&self) -> Option<&'static str> {
        match self {
            State::Pending => Some(JOIN_ACTION),
            State::AlreadyMember => Some(OPEN_TASK),
            _ => None,
        }
    }

    /// 是否错误分支（过期/登录/提交失败/加载失败）。
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            State::LinkInvalid | State::LoginRequired | State::SubmitFailed | State::LoadFailed
        )
    }
}

/// 浮层视图模型（视图是状态的纯函数）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskInvitation {
    state: State,
}

impl TaskInvitation {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    /// 无数据源时的实况（fail-closed：未下发）。
    pub fn fail_closed() -> Self {
        Self::default()
    }

    /// 由运行时输入构造（纯函数）。
    pub fn from_input(input: Option<&Input>) -> Self {
        Self::new(State::resolve(input))
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// 事件转移后的新视图模型（不改自身）。
    pub fn transitioned(&self, event: Event) -> Self {
        Self::new(self.state.on(event))
    }
}

/// 主区渲染（诚实占位：仅标题 + 未下发说明；本卡负责补全真实视图）。
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
