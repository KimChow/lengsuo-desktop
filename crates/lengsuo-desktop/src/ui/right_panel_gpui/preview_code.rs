//! 代码预览（`PreviewKind::Code`）：等宽代码区 + 行号列 + 截断提示 + 未下发态。
//!
//! 硬约束：
//! - ⛔ 不读文件、不做文件 I/O：预览内容由运行时契约下发（未就绪 ⇒ 显示诚实提示，不伪造内容）。
//! - 颜色一律取自 `theme`，⛔ 不硬编码。
use gpui_kit::*;

/// 截断阈值：超过则只渲染前 `MAX_LINES` 行（§7：10MB 级文件不卡渲染进程）。
pub(super) const MAX_LINES: usize = 2000;

/// 是否截断（`total > MAX_LINES`）。
pub(super) fn is_truncated(total: usize) -> bool {
    total > MAX_LINES
}

/// 可见行范围 `[start, end)`：截断时只取前 `MAX_LINES` 行，否则全量。
pub(super) fn visible_range(total: usize) -> (usize, usize) {
    if is_truncated(total) {
        (0, MAX_LINES)
    } else {
        (0, total)
    }
}

/// 行号列宽（字符数）：最大**可见**行号的十进制位数，最小 3。
pub(super) fn line_no_width(total: usize) -> usize {
    visible_range(total).1.max(1).to_string().len().max(3)
}

/// 等宽容器标记：挂在容器 `.id()` 上，渲染断言据此定位等宽字体区。
pub(super) const MONO_CONTAINER_ID: &str = "preview-code-mono";

/// 未下发态的诚实提示（⛔ 不伪造代码内容）。
pub(super) const NOT_DELIVERED_TEXT: &str = "内容未下发（等运行时契约）";

/// 渲染代码预览主体。内容未下发 ⇒ 未下发态；`render_lines` 保留给契约下发后接线。
pub(super) fn render(_path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    render_lines(None, theme)
}

/// `lines` 为契约下发的行（`None` = 未下发）。
fn render_lines(lines: Option<&[&str]>, theme: &gpui_kit::component::Theme) -> AnyElement {
    let container = div()
        .id(MONO_CONTAINER_ID)
        .font_family(theme.mono_font_family.clone())
        .bg(theme.muted)
        .rounded_md()
        .p_2()
        .text_xs();
    match lines {
        None => container
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED_TEXT)
            .into_any_element(),
        Some(lines) => container
            .child(
                // ponytail: 行数 ≤ MAX_LINES 已封顶，overflow 滚动够用；虚拟化等契约下发卡改签名时再上
                div()
                    .id("preview-code-scroll")
                    .overflow_y_scroll()
                    .max_h(px(480.0))
                    .child(code_rows(lines, theme)),
            )
            .into_any_element(),
    }
}

/// 逐行渲染：行号列（右对齐、等宽补位）+ 内容；截断时追加提示行。
fn code_rows(lines: &[&str], theme: &gpui_kit::component::Theme) -> Div {
    let total = lines.len();
    let (start, end) = visible_range(total);
    let width = line_no_width(total);
    let mut rows = div().flex().flex_col().gap_0p5();
    for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
        rows = rows.child(line_row(idx + 1, line, width, theme));
    }
    if is_truncated(total) {
        rows = rows.child(
            div()
                .mt_2()
                .text_color(theme.muted_foreground)
                .child(format!("已截断（仅显示前 {MAX_LINES} 行）")),
        );
    }
    rows
}

/// 单行：行号右对齐（按 `width` 补位，等宽字体下自然对齐）+ 代码文本。
fn line_row(no: usize, text: &str, width: usize, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .gap_2()
        .child(
            div()
                .flex_shrink_0()
                .text_color(theme.muted_foreground)
                .child(format!("{no:>width$}", width = width)),
        )
        .child(div().min_w_0().child(text.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 行号列宽：最小 3 位、按位数扩展、截断后以 2000 计。
    #[test]
    fn line_no_width_min_three_and_digit_count() {
        assert_eq!(line_no_width(9), 3);
        assert_eq!(line_no_width(1234), 4);
        assert_eq!(line_no_width(0), 3);
        assert_eq!(line_no_width(2001), 4);
    }

    /// 截断边界：1999 不截、2001 截，可见范围随之收缩到前 2000 行。
    #[test]
    fn truncation_boundary() {
        assert!(!is_truncated(1999));
        assert!(is_truncated(2001));
        assert_eq!(visible_range(1999), (0, 1999));
        assert_eq!(visible_range(2001), (0, MAX_LINES));
    }
}
