//! 输出区 / 历史区的渲染。
//!
//! 对齐 13 册 §3.1「终端输出（**等宽黑底**）」：
//! - **等宽**：`theme.mono_font_family`（沿用 `ui/session_gpui/tool_card_view.rs` 的先例）。
//! - **底色**：`theme.selection` —— 本仓**代码块底色**的既有取法
//!   （`ui/session_gpui/markdown_view.rs` 的代码块 / 行内代码都用它），
//!   ⛔ 不硬编码颜色、不新造主题键。
//!
//! ⛔ **不解析 ANSI 转义序列**：输出原样显示（不转义、不着色）。
//! ⛔ **不做滚动条**：内容超出时只显示**最近若干行**（见 [`MAX_VISIBLE_LINES`]）。

use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::terminal::{HISTORY_EMPTY_TEXT, OUTPUT_EMPTY_TEXT};

/// 内容区最多显示的行数（超出时只保留**最近**的行）。
pub(super) const MAX_VISIBLE_LINES: usize = 12;

/// 渲染输出区。
pub(super) fn render_output(lines: &[String], theme: &Theme) -> AnyElement {
    if lines.is_empty() {
        return empty(OUTPUT_EMPTY_TEXT, theme);
    }
    let start = lines.len().saturating_sub(MAX_VISIBLE_LINES);
    mono_block(&lines[start..], theme)
}

/// 渲染历史区（描述层已给**倒序**：最近的在最前）。
pub(super) fn render_history(entries: &[String], theme: &Theme) -> AnyElement {
    if entries.is_empty() {
        return empty(HISTORY_EMPTY_TEXT, theme);
    }
    let start = entries.len().saturating_sub(MAX_VISIBLE_LINES);
    mono_block(&entries[start..], theme)
}

/// 空态文案（真实为空，⛔ 不是加载失败）。
fn empty(text: &'static str, theme: &Theme) -> AnyElement {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
        .into_any_element()
}

/// 等宽黑底的一组行。
fn mono_block(lines: &[String], theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .h_full()
        .p_2()
        .rounded_sm()
        .bg(theme.selection)
        .font_family(theme.mono_font_family.clone())
        .text_xs()
        .text_color(theme.foreground)
        .children(
            lines
                .iter()
                .map(|line| div().w_full().whitespace_nowrap().child(line.clone())),
        )
        .into_any_element()
}
