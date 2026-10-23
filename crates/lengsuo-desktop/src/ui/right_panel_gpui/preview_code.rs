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

/// 渲染代码预览（占位，接线层待填充）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(format!("{path}：代码预览尚未实现"))
        .into_any_element()
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
