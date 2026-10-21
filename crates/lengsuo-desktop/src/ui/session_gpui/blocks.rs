//! 会话流各块的 GPUI 渲染分支。
//!
//! 本文件是 [`super`]（`session_gpui`）的子模块，只放**渲染分支**；
//! 类型定义与视图主体在 `mod.rs`。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use super::{BlockCtx, BUBBLE_MAX_W};
use crate::ui::approval_card::ApprovalAction;
use crate::ui::session_render::{Align, FileItem, ToolResultKind, ToolResultView};
use crate::ui::tool_card::ToolCard;

/// 用户气泡：右对齐，背景色与助手正文区分。
pub(super) fn render_user_bubble(text: &str, align: Align, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let row = div().flex().flex_row().w_full();
    let row = match align {
        Align::Right => row.justify_end(),
        Align::Left => row.justify_start(),
    };
    row.child(
        div()
            .max_w(px(BUBBLE_MAX_W))
            .px_3()
            .py_2()
            .rounded_lg()
            .bg(theme.accent)
            .text_color(theme.accent_foreground)
            .text_sm()
            .child(text.to_owned()),
    )
    .into_any_element()
}

/// 助手正文：左对齐，走轻量 Markdown 子集（见 [`super::markdown`]）。
pub(super) fn render_assistant_text(text: &str, align: Align, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let row = div().flex().flex_row().w_full();
    let row = match align {
        Align::Right => row.justify_end(),
        Align::Left => row.justify_start(),
    };
    row.child(super::markdown_view::render_markdown(text, theme))
        .into_any_element()
}

/// 工具调用卡片：默认收起，点标题展开。
///
/// 真正的渲染在 [`super::tool_card_view`]（图标 + 人话标题 + 参数摘要 + 可折叠）；
/// 这里只做转发，**不重新实现**任何判断。
pub(super) fn render_tool_call(card: &ToolCard, ctx: &BlockCtx<'_>) -> AnyElement {
    super::tool_card_view::render_tool_card(card, ctx.tool_detail(), ctx)
}

/// 工具结果：按 [`ToolResultKind`] 分三种形态渲染。
pub(super) fn render_tool_result(view: &ToolResultView, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let mut card = div()
        .flex()
        .flex_col()
        .w_full()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(
            div()
                .px_3()
                .py_2()
                .text_sm()
                .font_bold()
                .child(view.title.clone()),
        );
    card = match view.kind {
        ToolResultKind::Terminal => card.child(render_terminal(&view.text, ctx)),
        ToolResultKind::FileList => card.child(render_file_list(&view.files, ctx)),
        ToolResultKind::Diff => card.child(render_diff(&view.text, ctx)),
    };
    card.into_any_element()
}

/// 终端输出：等宽字体，逐行渲染（保留换行与缩进）。
pub(super) fn render_terminal(text: &str, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let mut body = div()
        .flex()
        .flex_col()
        .px_3()
        .pb_2()
        .whitespace_nowrap()
        .font_family(theme.mono_font_family.clone());
    for line in text.lines() {
        body = body.child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(line.to_owned()),
        );
    }
    body.into_any_element()
}

/// 文件列表：每项可点击（点击只改本地选中态 + 产出跳转意图）。
pub(super) fn render_file_list(files: &[FileItem], ctx: &BlockCtx<'_>) -> AnyElement {
    let mut list = div().flex().flex_col().gap_1().px_3().pb_2();
    for (i, item) in files.iter().enumerate() {
        list = list.child(super::file_card::render_file_row(item, i, ctx));
    }
    list.into_any_element()
}

/// diff：**左右分栏**（旧文件在左、新文件在右）+ 红绿对照。
///
/// 形态判断在 [`super::diff_split`]（描述层），渲染在 [`super::diff_split_view`]。
///
/// diff 可能很长 ⇒ 卡片内**自己滚动**（⛔ 不动外层会话流的滚动）。
pub(super) fn render_diff(text: &str, ctx: &BlockCtx<'_>) -> AnyElement {
    let body = super::diff_split_view::render_diff_split(text, ctx.theme);
    div()
        // `overflow_y_scroll` 只在**带 id 的**元素上可用（速查表 §12.2）。
        .id(ctx.element_id("diff", 0))
        .flex()
        .flex_col()
        .px_3()
        .pb_2()
        .max_h(px(DIFF_MAX_H))
        .overflow_y_scroll()
        .child(body)
        .into_any_element()
}

/// diff 卡片的最大高度（超出部分在卡片内滚动）。
const DIFF_MAX_H: f32 = 420.0;

/// 文件卡片网格。
pub(super) fn render_files_grid(files: &[FileItem], ctx: &BlockCtx<'_>) -> AnyElement {
    let mut grid = div().flex().flex_wrap().gap_2().w_full();
    for (i, item) in files.iter().enumerate() {
        grid = grid.child(super::file_card::render_file_card(item, i, ctx));
    }
    grid.into_any_element()
}

/// 断档提示：醒目提示条（描述层保证它排在首块）。
pub(super) fn render_stale_notice(text: &str, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    div()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(theme.button_warning)
        .text_color(theme.background)
        .text_sm()
        .font_bold()
        .child(text.to_owned())
        .into_any_element()
}

/// 审批按钮配色（取自主题，不硬编码）。
pub(super) fn approval_button_colors(action: ApprovalAction, theme: &Theme) -> (Hsla, Hsla) {
    match action {
        ApprovalAction::Allow => (theme.button_success, theme.button_success_foreground),
        ApprovalAction::Deny => (theme.button_danger, theme.button_danger_foreground),
        ApprovalAction::AlwaysAllow => (theme.button_secondary, theme.button_secondary_foreground),
    }
}
