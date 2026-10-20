//! 会话流各块的 GPUI 渲染分支。
//!
//! 本文件是 [`super`]（`session_gpui`）的子模块，只放**渲染分支**；
//! 类型定义与视图主体在 `mod.rs`。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use super::diff_lines::DiffLineKind;
use super::{with_click, BlockCtx, BUBBLE_MAX_W, FILE_CARD_W};
use crate::ui::approval_card::{ApprovalAction, ApprovalButton, ApprovalCardView};
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
pub(super) fn render_tool_call(card: &ToolCard, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let expanded = ctx.expanded(card);
    let folded = card.folded;

    let marker = if expanded { "▼" } else { "▶" };
    let header = div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .w_full()
        .px_3()
        .py_2()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(marker),
        )
        .child(div().text_sm().font_bold().child(card.title.clone()));

    let index = ctx.index;
    let header = with_click(
        header,
        ctx.element_id("tool-call", 0),
        ctx.host,
        move |view| view.toggle_expanded(index, folded),
    );

    let mut body = div()
        .flex()
        .flex_col()
        .w_full()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(header);

    if expanded {
        body = body.child(
            div()
                .px_3()
                .pb_2()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(card.summary.clone()),
        );
    }
    body.into_any_element()
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

/// 文件列表：每项可点击（点击只改本地选中态）。
pub(super) fn render_file_list(files: &[FileItem], ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let mut list = div().flex().flex_col().gap_1().px_3().pb_2();
    for (i, item) in files.iter().enumerate() {
        let selected = ctx.ui.selected_file() == Some(item.path.as_str());
        let row = div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .w_full()
            .px_2()
            .py_1()
            .rounded_sm()
            .text_sm()
            .text_color(if selected {
                theme.accent
            } else {
                theme.foreground
            });
        let row = if selected {
            row.bg(theme.selection)
        } else {
            row
        };
        let row = row
            .child(div().font_bold().child(item.display.clone()))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(item.path.clone()),
            );
        let row = if item.clickable {
            let path = item.path.clone();
            with_click(row, ctx.element_id("file", i), ctx.host, move |view| {
                view.ui.toggle_file(&path);
            })
        } else {
            row.into_any_element()
        };
        list = list.child(row);
    }
    list.into_any_element()
}

/// diff：按增 / 删 / 上下文分色并带行号（分类在 [`super::diff_lines`]）。
pub(super) fn render_diff(text: &str, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let mut body = div()
        .flex()
        .flex_col()
        .px_3()
        .pb_2()
        .whitespace_nowrap()
        .font_family(theme.mono_font_family.clone());
    for line in super::diff_lines::split(text) {
        let color = match line.kind {
            DiffLineKind::Added => theme.success,
            DiffLineKind::Removed => theme.danger,
            DiffLineKind::Hunk => theme.accent,
            DiffLineKind::Header | DiffLineKind::Plain => theme.foreground,
            DiffLineKind::Context => theme.muted_foreground,
        };
        body = body.child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .text_sm()
                .child(
                    div()
                        .text_color(theme.muted_foreground)
                        .child(format!("{:>4}", line.no)),
                )
                .child(div().text_color(color).child(line.text)),
        );
    }
    body.into_any_element()
}

/// 文件卡片网格。
pub(super) fn render_files_grid(files: &[FileItem], ctx: &BlockCtx<'_>) -> AnyElement {
    let mut grid = div().flex().flex_wrap().gap_2().w_full();
    for (i, item) in files.iter().enumerate() {
        grid = grid.child(render_file_card(item, i, ctx));
    }
    grid.into_any_element()
}

/// 单张文件卡片。
pub(super) fn render_file_card(item: &FileItem, sub: usize, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let selected = ctx.ui.selected_file() == Some(item.path.as_str());
    let mut card = div()
        .flex()
        .flex_col()
        .gap_1()
        .w(px(FILE_CARD_W))
        .px_2()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .bg(theme.background);
    card = card
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(if selected {
                    theme.accent
                } else {
                    theme.foreground
                })
                .child(item.display.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(item.path.clone()),
        );
    if item.clickable {
        let path = item.path.clone();
        with_click(
            card,
            ctx.element_id("file-card", sub),
            ctx.host,
            move |view| {
                view.ui.toggle_file(&path);
            },
        )
    } else {
        card.into_any_element()
    }
}

/// 审批卡：**只读渲染**。三个按钮，点击只记本地选择，绝不放行。
pub(super) fn render_approval(card: &ApprovalCardView, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let choice = ctx.ui.approval_choice.get(&card.id).copied();

    let status = div().text_xs();
    let status = match (card.pending, choice) {
        (true, None) => status.text_color(theme.muted_foreground).child("等待决策"),
        (true, Some(action)) => status
            .text_color(theme.warning)
            .child(format!("已在界面选择：{}（尚未生效）", action.label())),
        (false, _) => status.text_color(theme.muted_foreground).child("已决策"),
    };

    let mut buttons = div().flex().flex_row().gap_2();
    for (i, btn) in card.buttons.iter().enumerate() {
        buttons = buttons.child(render_approval_button(&card.id, btn, i, choice, ctx));
    }

    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(div().text_sm().font_bold().child(card.title.clone()))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(card.summary.clone()),
        )
        .child(status)
        .child(buttons)
        .into_any_element()
}

/// 单个审批按钮（允许 / 拒绝 / 总是允许）。
pub(super) fn render_approval_button(
    id: &str,
    btn: &ApprovalButton,
    sub: usize,
    choice: Option<ApprovalAction>,
    ctx: &BlockCtx<'_>,
) -> AnyElement {
    let theme = ctx.theme;
    let (bg, fg) = approval_button_colors(btn.action, theme);
    let selected = choice == Some(btn.action);
    let button = div()
        .px_3()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .bg(bg)
        .text_color(fg)
        .text_sm()
        .child(btn.label.to_owned());
    if !btn.enabled {
        // 已决策项的按钮必须禁用：不可点击，也不给交互反馈。
        return button.opacity(0.5).into_any_element();
    }
    let action = btn.action;
    let approval_id = id.to_owned();
    with_click(
        button,
        ctx.element_id("approval", sub),
        ctx.host,
        move |view| view.choose_approval(&approval_id, action),
    )
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
