//! 「帮助与反馈」面板：只读展示应用版本、快捷键与命令表。
//!
//! ⛔ 本面板**不提供反馈提交**：那要网络与运行时，不在界面接线层。
//! ⛔ 本面板**不做网络检查 / 自检 / 资源监控**：同属运行时侧，不在本卡范围。
//!
//! 硬约束：
//! - **只读**：条目不可点击、不接反馈入口。
//! - **fail-closed**：加载失败必须显示错误，不得退化成空列表。
//! - **「真实为空」与「加载失败」用不同文案**（描述层的 [`HelpPanelBody`]）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::state::hotkeys::Binding;
use crate::theme::palette::COMMANDS;
use crate::ui::settings_gpui::view::{
    help_body, CommandRow, HelpPanelBody, HelpStatus, ShortcutRow, EMPTY_BINDINGS_TEXT,
};

use super::window::SettingsGpui;

/// 渲染帮助与反馈面板：版本 + 快捷键 + 命令表（只读）。
pub(super) fn render_help_panel(
    bindings: &[Binding],
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div().flex().flex_col().gap_2();

    column = column.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("以下为只读信息：反馈提交、网络检查与自检需要网络与运行时，不在此面板提供。"),
    );

    // 数据源是本仓的**本地同步**数据（编译期命令表 + 内存态绑定表）⇒ 恒为「就绪」。
    // 「加载中 / 加载失败」两态由描述层保留（与 e30 的账号面板同形），
    // 将来接入异步来源时不必改接线层的形状。
    match help_body(
        bindings,
        COMMANDS,
        env!("CARGO_PKG_VERSION"),
        HelpStatus::Ready,
    ) {
        HelpPanelBody::Loading => {
            column = column.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("正在加载帮助信息…"),
            );
        }
        HelpPanelBody::Failed(message) => {
            // fail-closed：失败必须显示错误，**不得**退化成空列表。
            column = column.child(render_banner("帮助信息加载失败", &message, theme));
        }
        HelpPanelBody::Empty => {
            column = column.child(render_empty("暂无可展示的帮助信息", theme));
        }
        HelpPanelBody::Content(content) => {
            column = column.child(render_version(&content.version, theme));
            column = column.child(render_section("快捷键", theme));
            if content.shortcuts.is_empty() {
                // 复用「个性化」面板的既有措辞：没有绑定就说没有，不装作一切正常。
                column = column.child(render_empty(EMPTY_BINDINGS_TEXT, theme));
            } else {
                for row in &content.shortcuts {
                    column = column.child(render_shortcut_row(row, theme));
                }
            }
            column = column.child(render_section("命令", theme));
            for row in &content.commands {
                column = column.child(render_command_row(row, theme));
            }
        }
    }

    column.into_any_element()
}

/// 版本行：应用版本。
fn render_version(version: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(div().text_sm().text_color(theme.foreground).child("版本"))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .whitespace_nowrap()
                .child(version.to_owned()),
        )
}

/// 小节标题。
fn render_section(title: &str, theme: &Theme) -> Div {
    div()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(title.to_owned())
}

/// 一条快捷键：人话标题 + 组合键。
fn render_shortcut_row(row: &ShortcutRow, theme: &Theme) -> Div {
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
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(row.title.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .whitespace_nowrap()
                .child(row.keys.clone()),
        )
}

/// 一条命令：人话标题 + 稳定 id。
fn render_command_row(row: &CommandRow, theme: &Theme) -> Div {
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
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(row.title.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .whitespace_nowrap()
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
