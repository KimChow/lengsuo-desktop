//! 代码预览（`PreviewKind::Code`）的内容渲染——**由 `ui-preview-code` 卡实现**。
//!
//! 主理人落盘骨架：当前只给诚实占位，⛔ 不读文件、不解析内容。
//! 颜色一律取自 `theme`，⛔ 不硬编码。
use gpui_kit::*;

/// 渲染代码预览（占位，待 `ui-preview-code` 填充）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(format!("{path}：代码预览尚未实现"))
        .into_any_element()
}
