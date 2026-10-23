//! 组织域视图的 GPUI 渲染分支。
//!
//! 本目录是 [`super`]（`org_gpui`）的子模块，只放**渲染分支**；
//! 类型定义与视图主体在 `mod.rs`。
//! 拆分：通用小工具在本文件，控件在 [`controls`]，列表内容在 [`lists`]。

mod controls;
mod lists;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::org_view::DenialNotice;

// `org_gpui` 直接调用的入口保持原路径 `sections::...` 不变；
// 只再导出真正被上层用到的符号（unused 再导出会被 clippy 拦下）。
pub(in crate::ui::org_gpui) use controls::{render_tabs, render_toolbar};
pub(in crate::ui::org_gpui) use lists::render_content;

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
