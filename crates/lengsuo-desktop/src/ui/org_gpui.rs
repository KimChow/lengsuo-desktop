//! 组织域视图的 GPUI 接线层：把 [`crate::ui::org_view`] 的渲染描述接到元素树。
//!
//! 分层约定与 [`crate::ui::env_gpui`] 一致：
//! - **描述层**（`org_view`）：纯数据、无 GPUI 依赖、无窗口可测。
//! - **接线层**（本文件）：`feature = "ui"` 隔离，把描述变成 GPUI 元素。
//!
//! ## 权限边界（本文件最重要的一条）
//!
//! **界面只承载服务端下发的标记，不做授权推理。**
//!
//! 本文件对权限的全部使用只有一处：把描述层给出的 [`ActionState`] 映射成样式。
//!
//! | `ActionState` | 界面表现 |
//! |---|---|
//! | `Enabled` | 高亮按钮，可点击 |
//! | `Disabled { notice: None }` | 置灰按钮，**不编造原因** |
//! | `Disabled { notice: Some(n) }` | 置灰按钮 + 服务端给出的原因与引导 |
//!
//! 本文件**不含**任何「因为角色是 X 所以能 Y」的判断：
//! - 工具栏按钮的可用性一律来自 `OrgViewState::action(cap)`（读服务端 `can_*` 快照）；
//! - 行内按钮的可用性一律来自描述层已算好的 `RoleRow::edit` / `RoleRow::delete`；
//! - 拒绝文案一律来自服务端原因码经 `denial_notice()` 的产物，本文件不重写、不合并。
//!
//! **界面禁用 ≠ 授权**：置灰只是可见性标记，真正的权限校验在服务端 + 运行时。
#![cfg(feature = "ui")]

use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::org_perm::Capability;
use crate::ui::org_view::{
    ActionState, DenialNotice, DepartmentNode, MemberRow, OrgViewState, RoleRow, SectionView,
};

/// 组织域的三个分区：部门树 / 成员表 / 角色表。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrgSection {
    /// 部门树（层级缩进）。
    Departments,
    /// 成员表。
    Members,
    /// 角色表。
    Roles,
}

impl OrgSection {
    /// tab 顺序。
    pub const ALL: [Self; 3] = [Self::Departments, Self::Members, Self::Roles];

    /// tab 文案。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Departments => "部门",
            Self::Members => "成员",
            Self::Roles => "角色",
        }
    }
}

/// 界面发出的操作意图。
///
/// **只表达「用户点了哪个按钮」**，不携带也不推导权限：能不能点由服务端标记
/// 决定，意图本身只是把「点了哪颗按钮」告诉上层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrgAction {
    /// 切换分区。
    SetSection(OrgSection),
    /// 新建部门。
    CreateDepartment,
    /// 邀请成员。
    InviteMember,
    /// 编辑成员（参数为成员 id）。
    EditMember(String),
    /// 移除成员（参数为成员 id）。
    RemoveMember(String),
    /// 新建角色。
    CreateRole,
    /// 分配角色。
    AssignRole,
    /// 编辑角色（参数为角色 id）。
    EditRole(String),
    /// 删除角色（参数为角色 id）。
    DeleteRole(String),
}

/// 操作意图分发器。接线层**不接真实数据源**，由上层注入回调。
pub type OrgActionDispatcher = Rc<dyn Fn(OrgAction)>;

/// 组织域视图的 GPUI 视图。
pub struct OrgViewGpui {
    /// 渲染描述 + 服务端能力快照（只承载，不推理）。
    state: OrgViewState,
    /// 当前分区（tab）。
    section: OrgSection,
    /// 操作意图分发器；缺省时按钮只渲染、不派发。
    dispatch: Option<OrgActionDispatcher>,
}

impl OrgViewGpui {
    /// 用一份渲染描述构造视图。
    #[must_use]
    pub fn new(state: OrgViewState) -> Self {
        Self {
            state,
            section: OrgSection::Departments,
            dispatch: None,
        }
    }

    /// 构造视图并注入操作意图分发器。
    #[must_use]
    pub fn with_dispatcher(state: OrgViewState, dispatch: OrgActionDispatcher) -> Self {
        Self {
            state,
            section: OrgSection::Departments,
            dispatch: Some(dispatch),
        }
    }

    /// 只读访问渲染描述。
    #[must_use]
    pub fn state(&self) -> &OrgViewState {
        &self.state
    }

    /// 可写访问渲染描述（注入服务端数据时用）。
    pub fn state_mut(&mut self) -> &mut OrgViewState {
        &mut self.state
    }

    /// 当前分区。
    #[must_use]
    pub fn section(&self) -> OrgSection {
        self.section
    }

    /// 切换分区。
    pub fn set_section(&mut self, section: OrgSection) {
        self.section = section;
    }
}

/// 分区的主操作：工具栏第一颗按钮对应的服务端能力。
///
/// 这里只是「哪个分区放哪颗按钮」的映射，**不是权限判断**——按钮能不能点
/// 仍然只看服务端下发的 `can_*` 标记。
const fn section_primary_cap(section: OrgSection) -> Capability {
    match section {
        OrgSection::Departments => Capability::ManageDepartment,
        OrgSection::Members => Capability::InviteMember,
        OrgSection::Roles => Capability::CreateRole,
    }
}

/// 名称展示：有名字用名字，没名字用占位 + id（不编造业务语义）。
fn display_label(label: Option<&str>, id: &str, placeholder: &str) -> String {
    if let Some(text) = label.filter(|text| !text.trim().is_empty()) {
        return text.to_owned();
    }
    if id.is_empty() {
        placeholder.to_owned()
    } else {
        format!("{placeholder} {id}")
    }
}

/// 加载中 / 空态的占位块。
fn status_block(text: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .p_6()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}

/// 失败块：显示原因，**绝不退化成空态**。
fn error_block(title: &str, message: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.button_danger)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(message.to_owned()),
        )
}

/// 拒绝提示面板：文案、说明、自助入口分开呈现，均来自服务端原因码。
fn render_notice_panel(notice: &DenialNotice, theme: &Theme) -> Div {
    let mut panel = div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(theme.muted)
        .child(
            div()
                .text_xs()
                .font_bold()
                .text_color(theme.foreground)
                .child(notice.message),
        );
    if let Some(hint) = notice.hint {
        panel = panel.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(hint),
        );
    }
    if let Some(action) = notice.action {
        panel = panel.child(div().text_xs().text_color(theme.link).child(action));
    }
    panel
}

/// 操作按钮。
///
/// 可用性**只**由 [`ActionState`] 决定：本函数不做任何二次判断，也不叠加
/// 角色、套餐、身份之类的条件。
fn render_button(
    label: &'static str,
    action: ActionState,
    intent: OrgAction,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
    let mut button = div()
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm()
        .whitespace_nowrap()
        .child(label);

    if action.is_enabled() {
        button = button
            .bg(theme.button_primary)
            .text_color(theme.button_primary_foreground)
            .cursor_pointer();
        let listener = cx.listener(
            move |view: &mut OrgViewGpui,
                  _event: &ClickEvent,
                  _window: &mut Window,
                  _cx: &mut Context<OrgViewGpui>| {
                if let Some(dispatch) = &view.dispatch {
                    dispatch(intent.clone());
                }
            },
        );
        button = button.on_click(listener);
    } else {
        button = button.bg(theme.muted).text_color(theme.muted_foreground);
    }

    let mut row = div().flex().items_center().gap_2().child(button);
    // 服务端给了原因才显示原因；没有原因就**不编造**。
    if let ActionState::Disabled {
        notice: Some(notice),
    } = action
    {
        row = row.child(
            div()
                .text_xs()
                .text_color(theme.warning_foreground)
                .child(notice.message),
        );
    }
    row
}

/// 顶部 tab 栏。
fn render_tabs(active: OrgSection, theme: &Theme, cx: &mut Context<OrgViewGpui>) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_2()
        .border_b_1()
        .border_color(theme.border);

    for section in OrgSection::ALL {
        let listener = cx.listener(
            move |view: &mut OrgViewGpui,
                  _event: &ClickEvent,
                  _window: &mut Window,
                  cx: &mut Context<OrgViewGpui>| {
                view.section = section;
                cx.notify();
            },
        );
        let tab = if section == active {
            div()
                .px_3()
                .py_1()
                .rounded_sm()
                .text_sm()
                .font_bold()
                .bg(theme.accent)
                .text_color(theme.accent_foreground)
                .cursor_pointer()
                .child(section.label())
                .on_click(listener)
        } else {
            div()
                .px_3()
                .py_1()
                .rounded_sm()
                .text_sm()
                .text_color(theme.muted_foreground)
                .cursor_pointer()
                .child(section.label())
                .on_click(listener)
        };
        row = row.child(tab);
    }
    row
}

/// 工具栏：分区内与能力相关的操作 + 服务端给出的拒绝提示。
fn render_toolbar(
    section: OrgSection,
    state: &OrgViewState,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
    let mut buttons = div().flex().items_center().gap_2().px_3().py_2();
    match section {
        OrgSection::Departments => {
            buttons = buttons.child(render_button(
                "新建部门",
                state.action(Capability::ManageDepartment),
                OrgAction::CreateDepartment,
                theme,
                cx,
            ));
        }
        OrgSection::Members => {
            buttons = buttons.child(render_button(
                "邀请成员",
                state.action(Capability::InviteMember),
                OrgAction::InviteMember,
                theme,
                cx,
            ));
        }
        OrgSection::Roles => {
            buttons = buttons
                .child(render_button(
                    "新建角色",
                    state.action(Capability::CreateRole),
                    OrgAction::CreateRole,
                    theme,
                    cx,
                ))
                .child(render_button(
                    "分配角色",
                    state.action(Capability::AssignRole),
                    OrgAction::AssignRole,
                    theme,
                    cx,
                ));
        }
    }

    let mut column = div()
        .flex()
        .flex_col()
        .border_b_1()
        .border_color(theme.border)
        .child(buttons);

    // 拒绝提示只展示服务端给的原文，不改写、不合并成一句「无权限」。
    if let ActionState::Disabled {
        notice: Some(notice),
    } = state.action(section_primary_cap(section))
    {
        column = column.child(div().px_3().pb_2().child(render_notice_panel(&notice, theme)));
    }
    column
}

/// 部门树的一个节点：按 `depth` 层级缩进。
fn render_department_node(node: &DepartmentNode, theme: &Theme) -> Div {
    let name = display_label(node.name.as_deref(), &node.id, "（未命名部门）");
    let marker = if node.has_children { "▾" } else { "·" };
    div()
        .flex()
        .items_center()
        .gap_2()
        .py_1()
        .pl(px(16. * node.depth as f32))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(marker),
        )
        .child(div().text_sm().child(name))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(node.id.clone()),
        )
}

/// 部门分区内容。
fn render_departments(state: &OrgViewState, theme: &Theme) -> Div {
    match state.departments_section() {
        SectionView::Loading => status_block("正在加载部门……", theme),
        SectionView::Failed { message } => error_block("部门加载失败", message, theme),
        SectionView::Empty { text } => status_block(text, theme),
        SectionView::Ready => {
            let mut body = div().flex().flex_col().gap_1().p_3();
            for node in &state.department_rows() {
                body = body.child(render_department_node(node, theme));
            }
            body
        }
    }
}

/// 成员表的一行。
fn render_member_row(
    row: &MemberRow,
    state: &OrgViewState,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
    let name = display_label(row.name.as_deref(), &row.id, "（未命名成员）");
    let email = row.email.clone().unwrap_or_else(|| "—".to_owned());
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(div().w(px(160.)).text_sm().truncate().child(name))
        .child(
            div()
                .w(px(240.))
                .text_sm()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(email),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .gap_2()
                .child(render_button(
                    "编辑",
                    state.action(Capability::EditMember),
                    OrgAction::EditMember(row.id.clone()),
                    theme,
                    cx,
                ))
                .child(render_button(
                    "移除",
                    state.action(Capability::RemoveMember),
                    OrgAction::RemoveMember(row.id.clone()),
                    theme,
                    cx,
                )),
        )
}

/// 成员分区内容。
fn render_members_section(
    state: &OrgViewState,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
    match state.members_section() {
        SectionView::Loading => status_block("正在加载成员……", theme),
        SectionView::Failed { message } => error_block("成员加载失败", message, theme),
        SectionView::Empty { text } => status_block(text, theme),
        SectionView::Ready => {
            let header = div()
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .bg(theme.table_head)
                .text_color(theme.table_head_foreground)
                .text_xs()
                .font_bold()
                .child(div().w(px(160.)).child("姓名"))
                .child(div().w(px(240.)).child("邮箱"))
                .child(div().flex_1().child("操作"));
            let mut body = div().flex().flex_col();
            for row in &state.member_rows() {
                body = body.child(render_member_row(row, state, theme, cx));
            }
            div().flex().flex_col().child(header).child(body)
        }
    }
}

/// 角色表的一行。行内按钮状态由描述层算好（`row.edit` / `row.delete`）。
fn render_role_row(row: &RoleRow, theme: &Theme, cx: &mut Context<OrgViewGpui>) -> Div {
    let name = display_label(row.name.as_deref(), &row.id, "（未命名角色）");
    let preset = row.preset_key.clone().unwrap_or_else(|| "—".to_owned());
    let kind = if row.is_ai_assistant {
        "AI 助手".to_owned()
    } else if row.is_builtin {
        "内置".to_owned()
    } else {
        "自定义".to_owned()
    };
    let badge = if row.read_only {
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("只读")
    } else {
        div()
    };

    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(div().w(px(160.)).text_sm().truncate().child(name))
        .child(
            div()
                .w(px(140.))
                .text_sm()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(preset),
        )
        .child(
            div()
                .w(px(120.))
                .flex()
                .items_center()
                .gap_1()
                .child(div().text_xs().child(kind))
                .child(badge),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .gap_2()
                .child(render_button(
                    "编辑",
                    row.edit,
                    OrgAction::EditRole(row.id.clone()),
                    theme,
                    cx,
                ))
                .child(render_button(
                    "删除",
                    row.delete,
                    OrgAction::DeleteRole(row.id.clone()),
                    theme,
                    cx,
                )),
        )
}

/// 角色分区内容。
fn render_roles_section(state: &OrgViewState, theme: &Theme, cx: &mut Context<OrgViewGpui>) -> Div {
    match state.roles_section() {
        SectionView::Loading => status_block("正在加载角色……", theme),
        SectionView::Failed { message } => error_block("角色加载失败", message, theme),
        SectionView::Empty { text } => status_block(text, theme),
        SectionView::Ready => {
            let header = div()
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .bg(theme.table_head)
                .text_color(theme.table_head_foreground)
                .text_xs()
                .font_bold()
                .child(div().w(px(160.)).child("名称"))
                .child(div().w(px(140.)).child("预置标识"))
                .child(div().w(px(120.)).child("类型"))
                .child(div().flex_1().child("操作"));
            let mut body = div().flex().flex_col();
            for row in &state.role_rows() {
                body = body.child(render_role_row(row, theme, cx));
            }
            div().flex().flex_col().child(header).child(body)
        }
    }
}

/// 分区内容。
fn render_content(
    state: &OrgViewState,
    section: OrgSection,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
    match section {
        OrgSection::Departments => render_departments(state, theme),
        OrgSection::Members => render_members_section(state, theme, cx),
        OrgSection::Roles => render_roles_section(state, theme, cx),
    }
}

impl Render for OrgViewGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let section = self.section;
        let state = &self.state;

        div()
            .flex()
            .flex_col()
            .h_full()
            .w_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(render_tabs(section, &theme, cx))
            .child(render_toolbar(section, state, &theme, cx))
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .child(render_content(state, section, &theme, cx)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_sections_have_distinct_labels() {
        let labels: Vec<&str> = OrgSection::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(labels.len(), 3);
        assert_eq!(labels, vec!["部门", "成员", "角色"]);
    }

    #[test]
    fn each_section_maps_to_its_own_primary_capability() {
        let caps = OrgSection::ALL.map(section_primary_cap);
        assert_eq!(caps[0], Capability::ManageDepartment);
        assert_eq!(caps[1], Capability::InviteMember);
        assert_eq!(caps[2], Capability::CreateRole);
    }

    #[test]
    fn missing_label_falls_back_to_placeholder_and_id() {
        assert_eq!(
            display_label(Some("研发"), "d1", "（未命名部门）"),
            "研发"
        );
        assert_eq!(
            display_label(None, "d1", "（未命名部门）"),
            "（未命名部门） d1"
        );
        assert_eq!(
            display_label(Some("   "), "", "（未命名部门）"),
            "（未命名部门）"
        );
    }

    #[test]
    fn actions_carry_no_permission_information() {
        // 意图只描述「点了哪颗按钮」，不含任何权限判断。
        let action = OrgAction::DeleteRole("r-1".to_owned());
        assert_eq!(action, OrgAction::DeleteRole("r-1".to_owned()));
        assert_ne!(action, OrgAction::DeleteRole("r-2".to_owned()));
    }
}
