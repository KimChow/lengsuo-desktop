//! 协作 · 成员（波次 9，卡 `ui-overlay-collab-members`）。
//!
//! 分层约定（同 `panel_views` / `collab_activity`）：纯数据层 + GPUI 层。
//! 硬约束：fail-closed（加载/失败不得退化成空列表）；⛔ 不伪造服务端数据；
//! 文案逐字取自命名空间 `collab.members`（29 键全收）。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 卡面名（任务书目标名，非文案表键）。
pub const TITLE: &str = "协作·成员";
/// 列表加载中（含「尚未拉取过」；命名空间无此键，沿用仓内既有措辞）。
pub const LOADING: &str = "加载中…";
/// 列表加载失败（fail-closed 分支标签；命名空间无此键，沿用仓内既有措辞）。
pub const LOAD_FAILED: &str = "加载失败";
/// 成员列表真实为空（≠ 未拉取/失败；措辞同仓内既有「暂无成员」）。
pub const EMPTY: &str = "暂无成员";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.members";
/// 命名空间键总数（据文案全表实测；本卡全收，`TEXTS.len()` 必须相等）。
pub const NS_KEY_COUNT: usize = 29;

/// 文案表（逐字取自文案全表命名空间 `collab.members`；本卡收录全部 29 键）。
pub const TEXTS: &[(&str, &str)] = &[
    ("collab.members.actions.cancel", "取消"),
    ("collab.members.actions.remove", "移除成员"),
    (
        "collab.members.actions.removeConfirmContent",
        "确定将「{name}」从协作中移除吗？移除后对方将无法继续参与协作。",
    ),
    ("collab.members.actions.removeConfirmTitle", "移除成员"),
    ("collab.members.invite.copied", "已复制邀请链接"),
    ("collab.members.invite.copying", "正在生成邀请链接..."),
    ("collab.members.invite.copyLink", "复制链接"),
    (
        "collab.members.invite.errors.copyFailed",
        "邀请链接已生成，但复制失败，请手动重试",
    ),
    ("collab.members.invite.errors.createFailed", "创建邀请链接失败"),
    (
        "collab.members.invite.errors.missingProject",
        "项目信息缺失，暂时无法创建邀请链接",
    ),
    (
        "collab.members.invite.errors.notSupported",
        "当前环境暂未接入邀请链接能力",
    ),
    ("collab.members.invite.joinRoleLabel", "申请加入后的角色"),
    (
        "collab.members.invite.note",
        "非项目成员需申请加入后方可进入协作",
    ),
    (
        "collab.members.invite.warning",
        "被邀请人加入后，将可使用项目配置的 Skill / 专家，其中可能包含其无权限使用的资源。请确认后再邀请。",
    ),
    ("collab.members.joinMode.applyThenJoin", "申请后加入"),
    ("collab.members.joinMode.directJoin", "直接加入"),
    ("collab.members.joinMode.label", "获得链接的人"),
    ("collab.members.joinMode.updateFailed", "加入方式更新失败"),
    ("collab.members.ownerTag", "所有者"),
    ("collab.members.popover.title", "项目团队成员 · {count}"),
    (
        "collab.members.popover.tooltip",
        "请勿未经允许分享个人信息或第三方内容，详情请查阅我们的服务条款。",
    ),
    ("collab.members.popover.tooltipAria", "使用说明"),
    ("collab.members.popover.unknownMember", "未知"),
    ("collab.members.role.admin", "所有者"),
    ("collab.members.role.editor", "管理员"),
    ("collab.members.role.viewer", "成员"),
    ("collab.members.toast.removed", "已移除成员"),
    ("collab.members.toast.removeFailed", "移除成员失败"),
    ("collab.members.toast.roleChangeFailed", "修改角色失败"),
];

/// 按键取文案（纯函数；缺键返回 `None`，调用方自行 fail-closed）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 按键取文案；键在本卡静态表内，缺键属程序性错误。
fn must_text(key: &str) -> &'static str {
    text(key).unwrap_or_else(|| panic!("collab.members 文案表缺键: {key}"))
}

/// 用 `{placeholder}` 对替换模板占位符（纯函数）。
fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (placeholder, value) in pairs {
        out = out.replace(placeholder, value);
    }
    out
}

/// 成员角色（键逐字对应文案表 `collab.members.role.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Admin,
    Editor,
    Viewer,
}

impl Role {
    /// 文案表键。
    pub fn key(self) -> &'static str {
        match self {
            Self::Admin => "collab.members.role.admin",
            Self::Editor => "collab.members.role.editor",
            Self::Viewer => "collab.members.role.viewer",
        }
    }

    /// 角色显示名（逐字取自文案表）。
    pub fn label(self) -> &'static str {
        must_text(self.key())
    }
}

/// 单个成员条目（纯数据；由调用方供数，本卡无数据源 ⇒ 不构造）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollabMember {
    /// 显示名（空串由渲染层兜底为 `popover.unknownMember`）。
    pub name: String,
    /// 角色。
    pub role: Role,
}

/// 视图态状态机（默认 `Loading`：未拉取过 ≠ 空集，fail-closed）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 加载中（含尚未拉取过）。
    #[default]
    Loading,
    /// 加载失败（fail-closed：⛔ 不得退化成空列表/正常）。
    Failed,
    /// 已下发且真实为空（≠ 失败，≠ 未拉取）。
    Empty,
    /// 正常：成员列表按序呈现。
    Ready(Vec<CollabMember>),
}

/// 邀请失败原因（键逐字对应 `collab.members.invite.errors.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InviteError {
    CreateFailed,
    MissingProject,
    NotSupported,
    CopyFailed,
}

impl InviteError {
    /// 文案表键。
    pub fn key(self) -> &'static str {
        match self {
            Self::CreateFailed => "collab.members.invite.errors.createFailed",
            Self::MissingProject => "collab.members.invite.errors.missingProject",
            Self::NotSupported => "collab.members.invite.errors.notSupported",
            Self::CopyFailed => "collab.members.invite.errors.copyFailed",
        }
    }
}

/// 邀请链接动作流（文案逐字取自 `collab.members.invite.*`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InviteState {
    /// 初始：按钮位显示「复制链接」。
    #[default]
    Idle,
    /// 正在生成。
    Copying,
    /// 已复制。
    Copied,
    /// 失败（fail-closed：如实呈现错误文案，不回落 Idle）。
    Failed(InviteError),
}

impl InviteState {
    /// 按钮位/状态位文案（纯函数）。
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => must_text("collab.members.invite.copyLink"),
            Self::Copying => must_text("collab.members.invite.copying"),
            Self::Copied => must_text("collab.members.invite.copied"),
            Self::Failed(err) => must_text(err.key()),
        }
    }
}

/// 加入方式（键逐字对应 `collab.members.joinMode.*`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JoinMode {
    /// 申请后加入。
    #[default]
    ApplyThenJoin,
    /// 直接加入。
    DirectJoin,
}

impl JoinMode {
    /// 文案表键。
    pub fn key(self) -> &'static str {
        match self {
            Self::ApplyThenJoin => "collab.members.joinMode.applyThenJoin",
            Self::DirectJoin => "collab.members.joinMode.directJoin",
        }
    }

    /// 显示名（逐字取自文案表）。
    pub fn label(self) -> &'static str {
        must_text(self.key())
    }
}

/// 正文主文案（纯函数）：`Some(label)` ⇒ 单行提示分支；`None` ⇒ 成员列表分支。
/// fail-closed：`Failed` 映射 [`LOAD_FAILED`]，⛔ 不与空态混同。
pub fn body_label(state: &State) -> Option<&'static str> {
    match state {
        State::Loading => Some(LOADING),
        State::Failed => Some(LOAD_FAILED),
        State::Empty => Some(EMPTY),
        State::Ready(_) => None,
    }
}

/// 成员数（标题 {count} 用；非 `Ready` 态按 0 呈现）。
pub fn member_count(state: &State) -> usize {
    match state {
        State::Ready(list) => list.len(),
        _ => 0,
    }
}

/// 浮层标题（`popover.title` 模板 + `{count}`，纯函数）。
pub fn popover_title(state: &State) -> String {
    fill(
        must_text("collab.members.popover.title"),
        &[("{count}", &member_count(state).to_string())],
    )
}

/// 移除确认正文（`actions.removeConfirmContent` 模板 + `{name}`，纯函数）。
pub fn remove_confirm_content(name: &str) -> String {
    fill(
        must_text("collab.members.actions.removeConfirmContent"),
        &[("{name}", name)],
    )
}

/// 成员显示名（纯函数）：空串兜底 `popover.unknownMember`。
pub fn display_name(member: &CollabMember) -> &str {
    if member.name.is_empty() {
        must_text("collab.members.popover.unknownMember")
    } else {
        &member.name
    }
}

// ===== GPUI 层 =====

/// 主区/浮层呈现入口（fail-closed：无数据源 ⇒ 默认「加载中」，⛔ 不伪造成员）。
pub fn main_area(theme: &Theme) -> AnyElement {
    members_panel(theme, &State::default(), &InviteState::default())
}

/// 面板骨架：标题行 + 说明行 + 正文（按状态机分支）+ 加入方式 + 邀请区；颜色一律 `theme.*`。
pub fn members_panel(theme: &Theme, state: &State, invite: &InviteState) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground)
        .child(title_row(theme, state))
        .child(tooltip_row())
        .child(body(theme, state))
        .child(join_mode_row())
        .child(invite_section(theme, invite))
        .into_any_element()
}

/// 标题行：`popover.title`（`{count}` 已填）+ 说明入口文案。
fn title_row(theme: &Theme, state: &State) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(popover_title(state))),
        )
        .child(SharedString::from(must_text(
            "collab.members.popover.tooltipAria",
        )))
        .into_any_element()
}

/// 说明行：`popover.tooltip` 全文（muted）。
fn tooltip_row() -> AnyElement {
    div()
        .child(SharedString::from(must_text(
            "collab.members.popover.tooltip",
        )))
        .into_any_element()
}

/// 正文：`body_label` 驱动的分支渲染（加载中/失败 fail-closed/空态/成员列表）。
fn body(theme: &Theme, state: &State) -> AnyElement {
    let col = div().flex().flex_col().gap_2();
    match body_label(state) {
        Some(label) => col.child(SharedString::from(label)).into_any_element(),
        None => col.children(member_rows(theme, state)).into_any_element(),
    }
}

/// 成员行集（仅 `Ready` 分支进入）。
fn member_rows(theme: &Theme, state: &State) -> Vec<AnyElement> {
    match state {
        State::Ready(list) => list.iter().map(|m| member_row(theme, m)).collect(),
        _ => Vec::new(),
    }
}

/// 单行成员：显示名（空串兜底「未知」）+ 角色位文案。
fn member_row(theme: &Theme, member: &CollabMember) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .text_color(theme.foreground)
        .child(SharedString::from(display_name(member)))
        .child(SharedString::from(member_tag(member)))
        .into_any_element()
}

/// 角色位：Admin 呈 `ownerTag`，其余用角色文案（均逐字取自文案表）。
fn member_tag(member: &CollabMember) -> &'static str {
    match member.role {
        Role::Admin => must_text("collab.members.ownerTag"),
        other => other.label(),
    }
}

/// 加入方式行：`joinMode.label` + 两档选项文案（选中态由调用方接线后驱动）。
fn join_mode_row() -> AnyElement {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(SharedString::from(must_text(
            "collab.members.joinMode.label",
        )))
        .child(SharedString::from(JoinMode::ApplyThenJoin.label()))
        .child(SharedString::from(JoinMode::DirectJoin.label()))
        .into_any_element()
}

/// 邀请区：加入角色说明 + 按钮位文案（`InviteState` 驱动）+ 提示 + 警示。
fn invite_section(theme: &Theme, invite: &InviteState) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(SharedString::from(must_text(
            "collab.members.invite.joinRoleLabel",
        )))
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(invite.label())),
        )
        .child(SharedString::from(must_text("collab.members.invite.note")))
        .child(SharedString::from(must_text(
            "collab.members.invite.warning",
        )))
        .into_any_element()
}

/// 移除确认卡：标题 + 正文（`{name}` 已填）+ [取消]/[移除成员] 动作位；
/// 由调用方在确认时机呈现。
pub fn remove_confirm_card(theme: &Theme, name: &str) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(must_text(
                    "collab.members.actions.removeConfirmTitle",
                ))),
        )
        .child(SharedString::from(remove_confirm_content(name)))
        .child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(SharedString::from(must_text(
                    "collab.members.actions.cancel",
                )))
                .child(SharedString::from(must_text(
                    "collab.members.actions.remove",
                ))),
        )
        .into_any_element()
}
