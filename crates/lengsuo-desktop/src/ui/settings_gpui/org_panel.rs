//! 「账号」面板：展示组织域成员清单（**只读**）。
//!
//! ⛔ 本面板**不提供增删改**：成员管理属于业务语义，不在界面接线层。
//! ⛔ 本面板**不做授权推理**：可见性完全来自服务端下发的列表。
//!
//! 硬约束：
//! - **只读**：行不可点击、不接增删改入口。
//! - **fail-closed**：加载失败必须显示错误，不得退化成空列表。
//! - **「真实为空」与「加载失败」用不同文案**（描述层的 [`AccountPanelBody`]）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::org::OrgListState;
use crate::ui::settings_gpui::view::{account_body, AccountPanelBody, AccountRow};

use super::window::SettingsGpui;

/// 渲染账号面板：只读成员清单。
pub(super) fn render_org_panel(
    state: &OrgListState,
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("以下成员由服务端下发，仅供查看；界面不提供增删改。"),
    );

    match account_body(state) {
        AccountPanelBody::Loading => {
            column = column.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("正在加载成员…"),
            );
        }
        AccountPanelBody::Failed(message) => {
            // fail-closed：失败必须显示错误，**不得**退化成空列表。
            column = column.child(render_banner("成员加载失败", &message, theme));
        }
        AccountPanelBody::Empty => {
            column = column.child(render_empty("暂无成员", theme));
        }
        AccountPanelBody::Content(rows) => {
            for row in &rows {
                column = column.child(render_row(row, theme));
            }
        }
    }

    column.into_any_element()
}

/// 一行成员：展示名 + id。
fn render_row(row: &AccountRow, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(row.name.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(row.id.clone()),
        )
}

/// 空态：一条弱化文案，不带「一切正常」的误导。
fn render_empty(text: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(text.to_owned()),
        )
}

/// 失败横幅：标题 + 错误原因（fail-closed，不让用户把错误当成空数据）。
fn render_banner(title: &str, message: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_4()
        .rounded_md()
        .border_1()
        .border_color(theme.danger)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.danger)
                .child(title.to_owned()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(message.to_owned()),
        )
}
