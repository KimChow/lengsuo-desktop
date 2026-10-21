//! 「数据管理」面板：展示环境分组清单与健康度（**只读**）。
//!
//! ⛔ 本面板**不提供增删改**：环境管理属于业务语义，不在界面接线层。
//! 判定全在描述层 [`crate::ui::settings_gpui::view::data_panel_body`] 等，
//! 本文件只做「描述 → 元素」的映射。
//!
//! 硬约束：
//! - **只读**：行不可点击、不接增删改入口。
//! - **fail-closed**：加载失败必须显示错误，不得退化成空列表。
//! - **「原始为空」与「筛选后为空」用不同文案**（描述层的 [`empty_state_text`]）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::env_group::{EnvGroupState, GroupSummary};
use crate::ui::settings_gpui::view::{
    data_panel_body, empty_state_text, group_display_name, DataPanelBody, DataSummary,
};

use super::window::SettingsGpui;

/// 渲染数据管理面板：分组清单 + 健康度 + 空态区分。
pub(super) fn render_data_panel(
    state: &EnvGroupState,
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("以下为只读清单：环境数据的增删改不在此面板提供。"),
    );

    match data_panel_body(state) {
        DataPanelBody::Loading => {
            column = column.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("正在加载环境数据…"),
            );
        }
        DataPanelBody::Failed(message) => {
            column = column.child(render_banner("环境数据加载失败", &message, theme));
        }
        DataPanelBody::Empty(reason) => {
            column = column.child(render_empty(empty_state_text(reason), theme));
        }
        DataPanelBody::Content(summary) => {
            column = column.child(render_summary(&summary, theme));
            for group in state.groups() {
                column = column.child(render_group_row(group, theme));
            }
            // 未分组是合法状态：有未分组条目（含坏条目）时单独一行，不猜归属。
            if state.ungrouped_count() > 0 || state.ungrouped_invalid_count() > 0 {
                column = column.child(render_count_row(
                    "未分组",
                    state.ungrouped_count(),
                    state.ungrouped_invalid_count(),
                    theme,
                ));
            }
        }
    }

    column.into_any_element()
}

/// 顶部汇总：四条只读数字。
fn render_summary(summary: &DataSummary, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .flex_wrap()
        .gap_2()
        .child(stat_chip("总条目", summary.total, theme))
        .child(stat_chip("解析失败", summary.invalid, theme))
        .child(stat_chip("分组", summary.group_count, theme))
        .child(stat_chip("未分组", summary.ungrouped, theme))
}

/// 一个汇总数字块（标签 + 数值，颜色一律取自主题）。
fn stat_chip(label: &'static str, value: usize, theme: &Theme) -> Div {
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
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(value.to_string()),
        )
}

/// 一行的分组：左为分组名，右为条目数 +（有坏条目时）解析失败数。
fn render_group_row(group: &GroupSummary, theme: &Theme) -> Div {
    let name = group_display_name(group);
    render_count_row(&name, group.env_count, group.invalid_count, theme)
}

/// 一行「名称 + 条目数」的通用骨架（分组行 / 未分组行共用）。
///
/// 解析失败数 > 0 时用弱化色标出（铁律要求不硬编码颜色）。
fn render_count_row(name: &str, count: usize, invalid: usize, theme: &Theme) -> Div {
    let name = name.to_owned();
    let right = if invalid > 0 {
        div()
            .flex()
            .flex_col()
            .items_end()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .whitespace_nowrap()
                    .child(format!("{} 个条目", count)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .whitespace_nowrap()
                    .child(format!("{} 条解析失败", invalid)),
            )
    } else {
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .whitespace_nowrap()
            .child(format!("{} 个条目", count))
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .py_2()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .child(div().text_sm().text_color(theme.foreground).child(name))
        .child(right)
}

/// 空态：一条弱化文案，不带「一切正常」的误导。
fn render_empty(text: &str, theme: &Theme) -> Div {
    let text = text.to_owned();
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
                .child(text),
        )
}

/// 失败横幅：标题 + 错误原因（fail-closed，不让用户把错误当成空数据）。
fn render_banner(title: &str, message: &str, theme: &Theme) -> Div {
    let title = title.to_owned();
    let message = message.to_owned();
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
                .font_bold()
                .text_color(theme.foreground)
                .child(title),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(message),
        )
}
