//! 「软件配置」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 把描述层（`view/software.rs`，文案逐字取 `settings.softwareConfig` 22 键）映射成元素：
//! - 「内置运行时」「工件空间依赖项」两区块是纯下发文案 ⇒ 静态渲染；
//! - 「运行时列表」是表格（列头 = 下发文案），行数据来自服务端快照 ——
//!   当前**没有运行时状态通道** ⇒ 走 `body(None)` 的「未下发」分支（fail-closed，
//!   ⛔ 不伪造版本/状态/目录）。
//!
//! 硬约束：**只读**（行内动作仅照抄参考面板的动作文案做展示，不可点、不给增删改）、
//! **fail-closed**、颜色**一律取自 `theme.*`**（⛔ 不硬编码）、
//! ⛔ 界面不判定权限、⛔ 界面不持有凭据。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::software as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 表格固定列宽（px）：列头与数据行共用，保证两侧行列对齐。
const W_TOOL: f32 = 112.;
const W_VERSION: f32 = 96.;
const W_STATUS: f32 = 72.;
const W_ACTIONS: f32 = 200.;
/// 渲染「软件配置」面板：静态区块照常出，「运行时列表」按服务端快照分叉。
pub(super) fn render_software_panel(theme: &Theme, _cx: &mut Context<SettingsGpui>) -> AnyElement {
    // 运行时状态通道属运行时侧（接线时再注入）；没有快照就如实走「未下发」。
    render_body(theme, &panel_view::body(None))
}

/// 面板主体渲染：标题 + 三个区块。
fn render_body(theme: &Theme, body: &panel_view::Body) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(panel_view::TITLE),
        )
        .child(section(
            theme,
            panel_view::BUNDLED_RUNTIME,
            &[
                panel_view::BUNDLED_RUNTIME_DESC,
                panel_view::BUNDLED_RUNTIME_PATH,
            ],
        ))
        .child(runtime_tools_section(theme, body))
        .child(section(
            theme,
            panel_view::WORKSPACE_DEPS,
            &[panel_view::TARGET_PLATFORM],
        ))
        .into_any_element()
}

/// 通用区块：小节标题 + 若干说明行（全部为下发文案，静态只读）。
fn section(theme: &Theme, title: &str, lines: &[&str]) -> Div {
    let mut el = div().flex().flex_col().gap_1().child(
        div()
            .text_sm()
            .font_medium()
            .text_color(theme.foreground)
            .child(title),
    );
    for line in lines {
        el = el.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(line),
        );
    }
    el
}

/// 「运行时列表」区块：小节标题 + 表格（列头 + 按快照分叉的行区）。
fn runtime_tools_section(theme: &Theme, body: &panel_view::Body) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_color(theme.foreground)
                .child(panel_view::RUNTIME_TOOLS),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .p_3()
                .child(table_header(theme))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(match body {
                            // fail-closed：未下发 ⇒ 显式提示，⛔ 不退化成空表格。
                            panel_view::Body::NotDelivered => not_delivered_row(theme),
                            panel_view::Body::Content(rows) => content_rows(theme, rows),
                        }),
                ),
        )
}

/// 表格列头：工具 / 版本 / 状态 / 安装目录 / 说明（末列为行内动作区，无列头文案）。
fn table_header(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(head_cell(theme, panel_view::COL_TOOL, W_TOOL))
        .child(head_cell(theme, panel_view::COL_VERSION, W_VERSION))
        .child(head_cell(theme, panel_view::COL_STATUS, W_STATUS))
        .child(head_cell(theme, panel_view::COL_INSTALL_DIR, None))
        .child(head_cell(theme, panel_view::COL_DESCRIPTION, None))
        .child(div().flex_1())
}

/// 列头单元格（`width = None` ⇒ 弹性列：安装目录 / 说明）。
fn head_cell(theme: &Theme, text: &str, width: Option<f32>) -> Div {
    let mut cell = div().text_xs().font_bold().text_color(theme.muted_foreground);
    cell = match width {
        Some(w) => cell.w(px(w)).flex_shrink_0(),
        None => cell.flex_1(),
    };
    cell.child(text)
}

/// 「未下发」行：占位提示落在**未下发分支**内（⛔ 不挪作已下发内容）。
fn not_delivered_row(theme: &Theme) -> Div {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(panel_view::NOT_DELIVERED)
}

/// 「有内容」行区：每个快照条目一行（字段全部来自服务端，界面不编值）。
fn content_rows(theme: &Theme, rows: &[panel_view::ToolRow]) -> Div {
    let mut list = div().flex().flex_col().gap_1();
    for row in rows {
        list = list.child(tool_row(theme, row));
    }
    list
}

/// 一条运行时工具行（只读）：五列数据 + 行内动作文案区。
fn tool_row(theme: &Theme, row: &panel_view::ToolRow) -> Div {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(
            div()
                .w(px(W_TOOL))
                .flex_shrink_0()
                .text_sm()
                .text_color(theme.foreground)
                .child(row.tool.clone()),
        )
        .child(
            div()
                .w(px(W_VERSION))
                .flex_shrink_0()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(row.version.clone()),
        )
        .child(
            div()
                .w(px(W_STATUS))
                .flex_shrink_0()
                .text_sm()
                .text_color(if row.enabled {
                    theme.foreground
                } else {
                    theme.muted_foreground
                })
                .child(row.status_text()),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(row.install_dir.clone()),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(row.description.clone()),
        )
        .child(row_actions(theme, row))
}

/// 行内动作区：照抄参考面板的动作文案（启用/禁用开关 + 重新安装 + 打开安装目录），
/// **只读展示**（弱化着色、不可点、不给增删改）。
fn row_actions(theme: &Theme, row: &panel_view::ToolRow) -> Div {
    div()
        .w(px(W_ACTIONS))
        .flex_shrink_0()
        .flex()
        .flex_row()
        .gap_2()
        .child(action_text(theme, row.toggle_label()))
        .child(action_text(theme, panel_view::ACTION_REINSTALL))
        .child(action_text(theme, panel_view::ACTION_OPEN_INSTALL_DIR))
}

/// 单个动作文案（弱化着色 = 非交互）。
fn action_text(theme: &Theme, text: &str) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text)
}
