//! 文本文档预览（`PreviewKind::Text`）的内容渲染——**由 `ui-preview-text` 卡实现**。
//!
//! 主理人落盘骨架：当前只给诚实占位，⛔ 不读文件、不解析内容。
//! 颜色一律取自 `theme`，⛔ 不硬编码。
use gpui_kit::*;

/// 渲染文本文档预览（占位，待 `ui-preview-text` 填充）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(format!("{path}：文本预览尚未实现"))
        .into_any_element()
}
