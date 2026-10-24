//! 项目协作邀请浮层（波次 6，卡 `ui-overlay-project-invitation`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（状态机 + 逐字文案 + 模板）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `projectInvitation`。
//! 数据源在运行时侧未下发 ⇒ [`main_area`] 如实渲染「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖）----------

/// 邀请徽标（badge）。
pub const BADGE: &str = "项目协作邀请";
/// 页面标题（title）。
pub const PAGE_TITLE: &str = "加入项目";
/// 副标题（subtitle）。
pub const SUBTITLE: &str = "你收到一份项目协作邀请";
/// 邀请人行模板（inviterLine）。
pub const INVITER_LINE: &str = "{name} 邀请你加入项目";
/// 邀请人缺失时的诚实兜底（unknownUser）。
pub const UNKNOWN_USER: &str = "未知用户";
/// 加入按钮文案（joinAction）。
pub const JOIN_ACTION: &str = "申请加入项目";
/// 加入中短文案（joiningShort）。
pub const JOINING_SHORT: &str = "加入中…";
/// 加入中长文案（joining）。
pub const JOINING: &str = "正在申请加入...";
/// 提交后须知（note）。
pub const NOTE: &str = "申请提交后需由项目管理员审批通过，届时你将收到通知";
/// 详情字段标签（field.*）。
pub const FIELD_PROJECT: &str = "项目名称";
pub const FIELD_ROLE: &str = "加入后角色";
pub const FIELD_MEMBER_COUNT: &str = "当前成员数";
pub const FIELD_EXPIRES_AT: &str = "有效期至";
/// 冲突分支（error.conflict）与说明（result.alreadyAcceptedDesc）。
pub const CONFLICT: &str = "邀请已被他人接受或处于终态";
pub const ALREADY_ACCEPTED_DESC: &str = "这个邀请已经处理完成，你现在可以直接返回主应用继续协作。";
/// 版本不支持分支（error.exclusiveBlocked）。
pub const EXCLUSIVE_BLOCKED: &str = "当前账号版本不支持团队协作功能，请联系企业管理员。";
/// 加载失败（error.loadFailed）。
pub const LOAD_FAILED: &str = "邀请信息加载失败";
/// 接受状态：已加入（result.joined*）。
pub const JOINED_TITLE: &str = "加入成功";
pub const JOINED_TAG: &str = "已加入";
pub const JOINED_DESC: &str = "你已成功加入该项目，现在可以开始协作。";
pub const JOINED_SHORT_DESC: &str = "稍候即可在项目列表中看到该项目";
/// 接受状态：等待审批（result.pending*）。
pub const PENDING_TITLE: &str = "申请已提交";
pub const PENDING_TAG: &str = "等待审批";
pub const PENDING_DESC: &str = "项目管理员审批通过后，你将正式加入该项目。";
/// 等待行模板（result.pendingWaitingFor）与兜底（result.pendingWaitingFallback）。
pub const PENDING_WAITING_FOR: &str = "等待 {name} 审批";
pub const PENDING_WAITING_FALLBACK: &str = "等待项目所有者审批";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 加入后角色（标签/说明逐字取自 role.* / roleDesc.*）。
#[derive(Clone, Copy, Debug)]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}

impl Role {
    /// 角色标签。
    pub fn label(self) -> &'static str {
        match self {
            Role::Admin => "管理员",
            Role::Editor => "成员",
            Role::Viewer => "访客",
        }
    }
    /// 角色说明。
    pub fn description(self) -> &'static str {
        match self {
            Role::Admin => "可管理成员、配置与项目权限",
            Role::Editor => "可参与协作、创建任务",
            Role::Viewer => "可查看项目内容与协作结果",
        }
    }
}

/// 邀请详情：值全部由数据源下发，缺省如实隐藏（⛔ 不伪造）。
#[derive(Clone, Debug, Default)]
pub struct InvitationDetail {
    pub project: Option<String>,
    pub inviter: Option<String>,
    pub role: Option<Role>,
    pub member_count: Option<u32>,
    pub expires_at: Option<String>,
}

impl InvitationDetail {
    /// 详情清单行（标签取自 field.*；值缺失的行不出现，⛔ 不渲染占位值）。
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        let mut rows = Vec::new();
        if let Some(v) = &self.project {
            rows.push((FIELD_PROJECT, v.clone()));
        }
        if let Some(r) = self.role {
            rows.push((FIELD_ROLE, format!("{} · {}", r.label(), r.description())));
        }
        if let Some(n) = self.member_count {
            rows.push((FIELD_MEMBER_COUNT, n.to_string()));
        }
        if let Some(v) = &self.expires_at {
            rows.push((FIELD_EXPIRES_AT, v.clone()));
        }
        rows
    }

    /// 邀请人行：模板逐字取自 inviterLine；缺失如实回退未知用户。
    pub fn inviter_line(&self) -> String {
        INVITER_LINE.replace("{name}", self.inviter.as_deref().unwrap_or(UNKNOWN_USER))
    }

    /// 空输入判定：邀请人行与清单都无内容 ⇒ 空态（⛔ 不是空列表）。
    pub fn is_empty(&self) -> bool {
        self.inviter.is_none() && self.rows().is_empty()
    }
}

/// 浮层状态机（枚举分支即渲染分支）。
#[derive(Clone, Debug)]
pub enum Phase {
    /// 数据未下发（fail-closed 默认态）。
    NotDelivered,
    /// 邀请详情（徽标 + 邀请人行 + 清单 + 加入按钮）。
    Detail(Box<InvitationDetail>),
    /// 正在申请加入。
    Joining,
    /// 申请已提交，等待审批。
    Pending { approver: Option<String> },
    /// 已加入（接受成功终态）。
    Joined,
    /// 冲突：邀请已被他人接受或处于终态。
    Conflict,
    /// 版本不支持团队协作（exclusiveBlocked）。
    VersionBlocked,
    /// 加载失败。
    Failed,
}

impl Phase {
    /// 主标题（各分支逐字文案）。
    pub fn headline(&self) -> &'static str {
        match self {
            Phase::NotDelivered => NOT_DELIVERED,
            Phase::Detail(_) => PAGE_TITLE,
            Phase::Joining => JOINING_SHORT,
            Phase::Pending { .. } => PENDING_TITLE,
            Phase::Joined => JOINED_TITLE,
            Phase::Conflict => CONFLICT,
            Phase::VersionBlocked => EXCLUSIVE_BLOCKED,
            Phase::Failed => LOAD_FAILED,
        }
    }

    /// 副文案；无则不渲染。
    pub fn description(&self) -> Option<&'static str> {
        match self {
            Phase::NotDelivered | Phase::VersionBlocked | Phase::Failed => None,
            Phase::Detail(_) => Some(SUBTITLE),
            Phase::Joining => Some(JOINING),
            Phase::Pending { .. } => Some(PENDING_DESC),
            Phase::Joined => Some(JOINED_DESC),
            Phase::Conflict => Some(ALREADY_ACCEPTED_DESC),
        }
    }

    /// 状态徽标：接受状态（已加入/等待审批）优先，否则用邀请徽标。
    pub fn tag(&self) -> &'static str {
        match self {
            Phase::Joined => JOINED_TAG,
            Phase::Pending { .. } => PENDING_TAG,
            _ => BADGE,
        }
    }

    /// 是否已接受（加入成功）。
    pub fn is_accepted(&self) -> bool {
        matches!(self, Phase::Joined)
    }
}

/// 等待审批行：模板逐字取自 pendingWaitingFor，无审批人回退 pendingWaitingFallback。
pub fn pending_waiting_line(approver: Option<&str>) -> String {
    match approver {
        Some(name) => PENDING_WAITING_FOR.replace("{name}", name),
        None => PENDING_WAITING_FALLBACK.to_string(),
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
///
/// 数据源在运行时侧未下发 ⇒ fail-closed 渲染 [`Phase::NotDelivered`]（如实「未下发」）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render_phase(&Phase::NotDelivered, theme)
}

/// 按状态机分支渲染整层（标题行 + 状态徽标 + 分支主体）。
fn render_phase(phase: &Phase, theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(render_header(phase, theme))
        .child(render_body(phase, theme))
}

/// 标题行：状态徽标（描边小徽章）+ 页面标题 + 副标题。
fn render_header(phase: &Phase, theme: &Theme) -> Div {
    let chip_border = match phase {
        Phase::Conflict => theme.danger,
        Phase::VersionBlocked => theme.warning,
        _ => theme.border,
    };
    div()
        .px_4()
        .py_3()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_sm()
                        .border_1()
                        .border_color(chip_border)
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(phase.tag()),
                )
                .child(
                    div()
                        .text_base()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(PAGE_TITLE),
                ),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(SUBTITLE),
        )
}

fn render_body(phase: &Phase, theme: &Theme) -> Div {
    match phase {
        Phase::NotDelivered => centered_line(NOT_DELIVERED, theme),
        Phase::Detail(detail) => render_detail(detail, theme),
        _ => render_state_page(phase, theme),
    }
}

/// 居中空态（fail-closed：如实「未下发」，⛔ 不渲染空列表）。
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

/// 详情分支：邀请人行 + 字段清单 + 加入按钮与须知。
fn render_detail(detail: &InvitationDetail, theme: &Theme) -> Div {
    if detail.is_empty() {
        // 空输入 ⇒ 空态，⛔ 不是空列表。
        return centered_line(NOT_DELIVERED, theme);
    }
    let mut list = div().flex().flex_col();
    for (label, value) in detail.rows() {
        list = list.child(
            div()
                .flex()
                .justify_between()
                .gap_4()
                .px_4()
                .py_1()
                .text_sm()
                .child(div().text_color(theme.muted_foreground).child(label))
                .child(div().text_color(theme.foreground).child(value)),
        );
    }
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_2()
        .py_2()
        .child(
            div()
                .px_4()
                .text_sm()
                .text_color(theme.foreground)
                .child(detail.inviter_line()),
        )
        .child(list)
        .child(
            div()
                .px_4()
                .py_3()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(theme.accent)
                        .text_color(theme.accent_foreground)
                        .text_sm()
                        .child(JOIN_ACTION),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(NOTE),
                ),
        )
}

/// 状态分支（Joining/Pending/Joined/Conflict/VersionBlocked/Failed）：居中主文案 + 副文案 + 分支附加行。
fn render_state_page(phase: &Phase, theme: &Theme) -> Div {
    let mut page = div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .px_4()
        .child(
            div()
                .text_base()
                .font_bold()
                .text_color(theme.foreground)
                .child(phase.headline()),
        );
    if let Some(desc) = phase.description() {
        page = page.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(desc),
        );
    }
    match phase {
        Phase::Pending { approver } => {
            page = page.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(pending_waiting_line(approver.as_deref())),
            );
        }
        Phase::Joined => {
            page = page.child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(JOINED_SHORT_DESC),
            );
        }
        _ => {}
    }
    page
}

#[cfg(test)]
mod tests {
    use super::{main_area, BADGE, NOT_DELIVERED};
    use gpui_kit::component::Theme;

    #[test]
    fn badge_is_taken_verbatim_and_not_blank() {
        assert_eq!(BADGE, "项目协作邀请");
    }

    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
