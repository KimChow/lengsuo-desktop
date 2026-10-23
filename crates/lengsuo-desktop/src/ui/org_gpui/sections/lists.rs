//! 列表内容渲染：部门树、成员表、角色表，以及分区内容分发。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::org_perm::Capability;
use crate::ui::org_gpui::{OrgAction, OrgSection, OrgViewGpui};
use crate::ui::org_view::{DepartmentNode, MemberRow, OrgViewState, RoleRow, SectionView};

use super::controls::render_button;
use super::{display_label, error_block, status_block};

/// 部门树的一个节点：按 `depth` 层级缩进。
pub(in crate::ui::org_gpui) fn render_department_node(node: &DepartmentNode, theme: &Theme) -> Div {
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
pub(in crate::ui::org_gpui) fn render_departments(state: &OrgViewState, theme: &Theme) -> Div {
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
pub(in crate::ui::org_gpui) fn render_member_row(
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
pub(in crate::ui::org_gpui) fn render_members_section(
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
pub(in crate::ui::org_gpui) fn render_role_row(
    row: &RoleRow,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
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
pub(in crate::ui::org_gpui) fn render_roles_section(
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
pub(in crate::ui::org_gpui) fn render_content(
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
