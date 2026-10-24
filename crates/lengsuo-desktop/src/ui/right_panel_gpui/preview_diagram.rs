//! 图表 / 白板预览（`PreviewKind::Diagram`）的内容渲染。
//!
//! 对齐 `docs/ui-parity-spec.md` **A14**（预览器套件）。本模块是**新落点**：
//! 与 `preview_image/code/text/pdf/office` 并列，由 `preview.rs` 按类型分派。
//!
//! 硬约束（同其余预览模块）：
//! - **fail-closed**：拿不到内容必须如实显示，⛔ 不伪造图形。
//! - ⛔ 不判权限、不持凭据、不联网。
//! - 颜色一律取自主题 token。
use gpui_kit::component::Theme;
use gpui_kit::*;

/// A14：解码待预览图形文件（`.svg` / 图形描述）。当前为**诚实占位**。
pub(super) fn decode(
    _path: &str,
    _bytes: Option<&[u8]>,
) -> crate::state::preview_payload::PreviewPayload {
    crate::state::preview_payload::PreviewPayload::NotDelivered {
        reason: crate::state::preview_payload::DECODE_NOT_DELIVERED,
    }
}

/// A14：带载荷渲染。当前仅渲染**诚实占位**（⛔ 不画假图形）。
pub(super) fn render_with_payload(
    path: &str,
    payload: &crate::state::preview_payload::PreviewPayload,
    theme: &Theme,
) -> AnyElement {
    let line = match payload {
        crate::state::preview_payload::PreviewPayload::NotDelivered { reason } => {
            (*reason).to_owned()
        }
        crate::state::preview_payload::PreviewPayload::Failed(err) => err.line(),
        _ => "图形预览尚未接入（A14）".to_owned(),
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(div().truncate().child(path.to_owned()))
        .child(line)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{decode, render_with_payload};
    use crate::state::preview_payload::{PreviewPayload, DECODE_NOT_DELIVERED};
    use gpui_kit::component::Theme;

    #[test]
    fn decode_is_honest_placeholder_for_now() {
        assert_eq!(
            decode("board.svg", None),
            PreviewPayload::NotDelivered {
                reason: DECODE_NOT_DELIVERED
            }
        );
    }

    #[test]
    fn render_with_payload_builds_without_panic() {
        let p = decode("board.svg", None);
        let theme = Theme::default();
        let _ = render_with_payload("board.svg", &p, &theme);
    }
}
