//! 右侧面板「文件预览」tab 的内容渲染。
//!
//! 分层：描述层（`crate::ui::right_panel`）已决定「预览哪个文件」；
//! 本文件只把描述变成元素，**不重新实现**任何判断。
//!
//! 硬约束：
//! - ⛔ **不读文件、不渲染文件内容**（预览器属 P3，见 `docs/ui-parity-spec.md` A14）。
//! - 颜色一律取自 `cx.theme()`，⛔ 不硬编码。
use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use crate::ui::right_panel::{PreviewContent, PREVIEW_UNSUPPORTED_TEXT};

/// 渲染「待预览文件」。
///
/// 有目标时显示**文件路径**（这是真实存在的目标，不是编造的内容）。
pub(super) fn render_preview(
    content: &PreviewContent,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    match content {
        PreviewContent::Target { path } => div()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(path.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(PREVIEW_UNSUPPORTED_TEXT),
            ),
    }
}
