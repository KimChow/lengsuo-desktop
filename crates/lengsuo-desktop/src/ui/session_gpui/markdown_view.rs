//! 轻量 Markdown 子集 → GPUI 元素树。
//!
//! 描述层来自 [`super::markdown`]（纯函数）；本文件只把片段映射成样式，
//! 颜色**全部**取自传入的 `&Theme`：
//! - 正文 / 标题：`theme.foreground`
//! - 斜体：`theme.muted_foreground`（gpui 无斜体开关，用弱化色区分）
//! - 行内代码：`theme.mono_font_family` + `theme.selection` 底色
//! - 代码块：`theme.mono_font_family` + `theme.selection` 底色

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::{div, AnyElement, IntoElement, ParentElement as _, Styled as _};

use super::markdown::{InlineSpan, MarkdownBlock};
use super::CONTENT_MAX_W;

/// 渲染一段正文（解析失败时解析层已回退成纯文本段落）。
pub(super) fn render_markdown(text: &str, theme: &Theme) -> AnyElement {
    let blocks = super::markdown::parse(text);
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .max_w(gpui_kit::px(CONTENT_MAX_W))
        .text_sm()
        .text_color(theme.foreground);
    for block in &blocks {
        col = col.child(render_block(block, theme));
    }
    col.into_any_element()
}

fn render_block(block: &MarkdownBlock, theme: &Theme) -> AnyElement {
    match block {
        MarkdownBlock::Paragraph(spans) => render_spans(spans, theme).into_any_element(),
        MarkdownBlock::Heading { level, spans } => {
            let size = match level {
                1 => 1.5,
                2 => 1.25,
                _ => 1.1,
            };
            div()
                .flex()
                .flex_row()
                .flex_wrap()
                .font_bold()
                .text_size(gpui_kit::rems(size))
                .child(render_spans(spans, theme))
                .into_any_element()
        }
        MarkdownBlock::ListItem { number, spans } => {
            let marker = number.map_or_else(|| "•".to_owned(), |n| format!("{n}."));
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(div().text_color(theme.muted_foreground).child(marker))
                .child(render_spans(spans, theme))
                .into_any_element()
        }
        MarkdownBlock::CodeBlock(body) => div()
            .w_full()
            .p_2()
            .rounded_sm()
            .bg(theme.selection)
            .font_family(theme.mono_font_family.clone())
            .whitespace_nowrap()
            .child(body.trim_end_matches('\n').to_owned())
            .into_any_element(),
    }
}

fn render_spans(spans: &[InlineSpan], theme: &Theme) -> impl IntoElement {
    let mut row = div().flex().flex_row().flex_wrap().items_center();
    for span in spans {
        row = row.child(render_span(span, theme));
    }
    row
}

fn render_span(span: &InlineSpan, theme: &Theme) -> AnyElement {
    match span {
        InlineSpan::Text(text) => div().child(text.clone()).into_any_element(),
        InlineSpan::Strong(text) => div().font_bold().child(text.clone()).into_any_element(),
        InlineSpan::Emphasis(text) => div()
            .text_color(theme.muted_foreground)
            .child(text.clone())
            .into_any_element(),
        InlineSpan::Code(text) => div()
            .px_1()
            .rounded_sm()
            .bg(theme.selection)
            .font_family(theme.mono_font_family.clone())
            .child(text.clone())
            .into_any_element(),
    }
}
