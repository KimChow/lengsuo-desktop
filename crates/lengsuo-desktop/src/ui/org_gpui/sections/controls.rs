//! 控件级渲染：按钮、tab 栏、工具栏。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::org_perm::Capability;
use crate::ui::org_gpui::{section_primary_cap, OrgAction, OrgSection, OrgViewGpui};
use crate::ui::org_view::{ActionState, OrgViewState};

use super::render_notice_panel;

/// 操作按钮。
///
/// 可用性**只**由 [`ActionState`] 决定：本函数不做任何二次判断，也不叠加
/// 角色、套餐、身份之类的条件。
pub(in crate::ui::org_gpui) fn render_button(
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
pub(in crate::ui::org_gpui) fn render_tabs(
    active: OrgSection,
    theme: &Theme,
    cx: &mut Context<OrgViewGpui>,
) -> Div {
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
pub(in crate::ui::org_gpui) fn render_toolbar(
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
