//! 右侧面板「文件预览」tab 的内容渲染。
//!
//! 分层：描述层（`crate::ui::right_panel`）已决定「预览哪个文件」；
//! 本文件按**文件类型**分派到同目录的预览子模块，**不重新实现**任何判断。
//!
//! 硬约束：
//! - ⛔ **不读文件、不渲染文件内容**（解码器属 A14 后续卡；本层只做类型分派 + 诚实占位）。
//! - 颜色一律取自 `cx.theme()`，⛔ 不硬编码。
use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use crate::state::preview_kind::{classify, PreviewKind};
use crate::ui::right_panel::{PreviewContent, PREVIEW_UNSUPPORTED_TEXT};

use super::{preview_code, preview_image, preview_office, preview_pdf, preview_text};

/// 渲染「待预览文件」。
///
/// 有目标时显示**文件路径**与**判定出的类型**（都是真实存在的事实，不是编造的内容），
/// 再按类型分派到对应预览子模块。
pub(super) fn render_preview(
    content: &PreviewContent,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let PreviewContent::Target { path } = content;
    let kind = classify(path);
    let body: AnyElement = match kind {
        PreviewKind::Code => preview_code::render(path, theme),
        PreviewKind::Text => preview_text::render(path, theme),
        PreviewKind::Image => preview_image::render(path, theme),
        PreviewKind::Pdf => preview_pdf::render(path, theme),
        PreviewKind::Office => preview_office::render(path, theme),
        PreviewKind::Diagram | PreviewKind::Other => unsupported(theme),
    };
    div()
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
                .child(kind.label()),
        )
        .child(body)
}

/// 尚未实现的类型（图表 / 其他）的诚实占位：⛔ 不伪造内容。
fn unsupported(theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(PREVIEW_UNSUPPORTED_TEXT)
        .into_any_element()
}
