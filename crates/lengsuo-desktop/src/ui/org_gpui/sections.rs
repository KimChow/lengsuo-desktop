//! 组织域视图的 GPUI 渲染分支。
//!
//! 本文件是 [`super`]（`org_gpui`）的子模块，只放**渲染分支**；
//! 类型定义与视图主体在 `mod.rs`。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use super::{section_primary_cap, OrgAction, OrgSection, OrgViewGpui};
use crate::state::org_perm::Capability;
use crate::ui::org_view::{
    ActionState, DenialNotice, DepartmentNode, MemberRow, OrgViewState, RoleRow, SectionView,
};

/// 名称展示：有名字用名字，没名字用占位 + id（不编造业务语义）。
pub(super) fn display_label(label: Option<&str>, id: &str, placeholder: &str) -> String {
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
pub(super) fn status_block(text: &str, theme: &Theme) -> Div {
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
pub(super) fn error_block(title: &str, message: &str, theme: &Theme) -> Div {
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
pub(super) fn render_notice_panel(notice: &DenialNotice, theme: &Theme) -> Div {
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
pub(super) fn render_button(
    label: &'static str,
    action: ActionState,
    intent: OrgAction,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
    let mut button = div()
        .id(format!("org-action-{label}"))
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
pub(super) fn render_tabs(active: OrgSection, theme: &Theme, cx: &mut Context<OrgViewGpui>) -> Div {
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
                .id(format!("org-tab-{}", section.label()))
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
                .id(format!("org-tab-{}", section.label()))
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
pub(super) fn render_toolbar(
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
        column = column.child(
            div()
                .px_3()
                .pb_2()
                .child(render_notice_panel(&notice, theme)),
        );
    }
    column
}

/// 部门树的一个节点：按 `depth` 层级缩进。
pub(super) fn render_department_node(node: &DepartmentNode, theme: &Theme) -> Div {
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
pub(super) fn render_departments(state: &OrgViewState, theme: &Theme) -> Div {
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
pub(super) fn render_member_row(
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
pub(super) fn render_members_section(
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
pub(super) fn render_role_row(row: &RoleRow, theme: &Theme, cx: &mut Context<OrgViewGpui>) -> Div {
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
pub(super) fn render_roles_section(
    state: &OrgViewState,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
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
pub(super) fn render_content(
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
