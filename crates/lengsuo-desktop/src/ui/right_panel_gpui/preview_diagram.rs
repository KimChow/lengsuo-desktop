//! 图表 / 白板预览（`PreviewKind::Diagram`）的内容渲染。
//!
//! 对齐 `docs/ui-parity-spec.md` **A14**（预览器套件）。本模块是**新落点**：
//! 与 `preview_image/code/text/pdf/office` 并列，由 `preview.rs` 按类型分派。
//!
//! 硬约束（同其余预览模块）：
//! - **fail-closed**：拿不到内容必须如实显示，⛔ 不伪造图形。
//! - ⛔ 不判权限、不持凭据、不联网。
//! - 颜色一律取自主题 token。
use crate::state::preview_payload::{DecodeError, ImagePayload, PreviewPayload, MAX_BYTES};
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 图形描述类（`.mermaid` / `.drawio` / …）尚无解码器 ⇒ 诚实未下发的统一说明。
const DESC_NOT_DELIVERED: &str = "图形描述解码未接入（mermaid / drawio 等文本描述格式）";

/// A14：解码待预览图形文件。`.svg` 解析根元素宽高（解不出记 0）；
/// 其余图形描述格式尚无解码器 ⇒ [`PreviewPayload::NotDelivered`]。
pub(super) fn decode(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let ext = path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    if ext == "svg" {
        decode_svg(path, bytes)
    } else {
        PreviewPayload::NotDelivered {
            reason: DESC_NOT_DELIVERED,
        }
    }
}

/// `.svg` 解码：`bytes = Some(..)` 时**只用传入字节**（不碰文件系统）；
/// `None` 时读 `path`（失败 ⇒ [`DecodeError::Io`]）。超 [`MAX_BYTES`] 截断，
/// 以 `source_len` 如实标注（⛔ 不整包进内存）。
fn decode_svg(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let mut owned = Vec::new();
    let (data, raw_len): (&[u8], usize) = match bytes {
        Some(b) => (b, b.len()),
        None => match read_capped(path, &mut owned) {
            Ok(n) => (&owned, n),
            Err(msg) => return PreviewPayload::Failed(DecodeError::Io(msg)),
        },
    };
    let truncated = raw_len > MAX_BYTES;
    let capped = &data[..data.len().min(MAX_BYTES)];
    // 截断会破坏 XML 结构：宽高解不出记 0，字节照常交付（界面如实标注）；
    // 未截断却解析失败 ⇒ fail-closed 返回 Failed，⛔ 不伪装成空图。
    let (width, height) = match svg_root_size(capped) {
        Ok(dims) => dims,
        Err(_) if truncated => (0, 0),
        Err(msg) => return PreviewPayload::Failed(DecodeError::Parse(msg)),
    };
    PreviewPayload::Image(ImagePayload {
        bytes: capped.to_vec(),
        format: "svg".to_owned(),
        width,
        height,
        source_len: raw_len,
    })
}

/// 按 [`MAX_BYTES`] 有界读取：只装入「上限 + 1」字节以探测超限；
/// 返回文件**真实总字节数**（供「已截断」标注）。
fn read_capped(path: &str, out: &mut Vec<u8>) -> Result<usize, String> {
    use std::io::Read as _;
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    file.take(MAX_BYTES as u64 + 1)
        .read_to_end(out)
        .map_err(|e| e.to_string())?;
    if out.len() > MAX_BYTES {
        Ok(std::fs::metadata(path).map_or(out.len(), |m| m.len() as usize))
    } else {
        Ok(out.len())
    }
}

/// 解析 SVG 根元素的像素宽高；根不是 `<svg>` / 结构不合法 ⇒ `Err`（原因如实上屏）。
fn svg_root_size(bytes: &[u8]) -> Result<(u32, u32), String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("非 UTF-8 字节：{e}"))?;
    let doc = roxmltree::Document::parse(text).map_err(|e| format!("SVG 结构不合法：{e}"))?;
    let root = doc.root_element();
    if root.tag_name().name() != "svg" {
        return Err(format!("根元素是 <{}>，不是 <svg>", root.tag_name().name()));
    }
    Ok((
        attr_px(root.attribute("width")),
        attr_px(root.attribute("height")),
    ))
}

/// `width`/`height` 属性 → 像素数：接受 `120` / `120px`；百分比、缺省、解不出 ⇒ 0。
fn attr_px(value: Option<&str>) -> u32 {
    value
        .and_then(|v| v.trim().trim_end_matches("px").parse::<f64>().ok())
        .map_or(0, |n| n.round().max(0.0) as u32)
}

/// A14：带载荷渲染——`.svg` 走**真矢量渲染**（`svg().data(..)`，resvg 随 gpui 自带）；
/// 截断载荷结构已破坏 ⇒ 只如实标注，⛔ 不画不完整图形。
pub(super) fn render_with_payload(
    path: &str,
    payload: &PreviewPayload,
    theme: &Theme,
) -> AnyElement {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let col = div().flex().flex_col().gap_2();
    match payload {
        PreviewPayload::Image(img) if img.format == "svg" => col
            .child(meta_row(file_name, &img.format, theme))
            .child(hint_row(&info_line(img), theme))
            .child(svg_stage(img, theme))
            .into_any_element(),
        PreviewPayload::NotDelivered { reason } => col
            .child(meta_row(file_name, "diagram", theme))
            .child(hint_row(reason, theme))
            .into_any_element(),
        PreviewPayload::Failed(err) => col
            .child(meta_row(file_name, "diagram", theme))
            .child(hint_row(&err.line(), theme))
            .into_any_element(),
        // 其他载荷不该分派到图表预览：如实说明，不硬凑内容。
        _ => col
            .child(meta_row(file_name, "diagram", theme))
            .child(hint_row("载荷类型与图表预览不匹配", theme))
            .into_any_element(),
    }
}

/// 元信息行：文件名（`path` 末段，真实值）+ 格式标签。
fn meta_row(file_name: &str, tag: &str, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(file_name.to_owned()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(tag.to_owned()),
        )
}

/// 次级信息 / 提示行（颜色取主题 token，保证对比度）。
fn hint_row(line: &str, theme: &Theme) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(line.to_owned())
}

/// 元信息一行文案：尺寸（解不出如实标「尺寸未知」）+ 字节数与截断标注。
fn info_line(img: &ImagePayload) -> String {
    let dims = if img.width > 0 && img.height > 0 {
        format!("{}×{} 像素", img.width, img.height)
    } else {
        "尺寸未知".to_owned()
    };
    if img.source_len > img.bytes.len() {
        format!(
            "{dims} · 原文件 {} 字节，已按上限截断至 {} 字节",
            img.source_len,
            img.bytes.len()
        )
    } else {
        format!("{dims} · {} 字节", img.source_len)
    }
}

/// 图形舞台：有界容器内真实渲染矢量；截断 ⇒ 诚实占位说明（⛔ 不伪造图形）。
fn svg_stage(img: &ImagePayload, theme: &Theme) -> Div {
    let stage = div()
        .h_64()
        .flex()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .bg_color(theme.muted)
        .p_2();
    if img.source_len > img.bytes.len() {
        stage.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child("内容超过上限被截断，无法完整渲染矢量图"),
        )
    } else {
        stage.child(svg().data(&img.bytes).size_full())
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, render_with_payload, DESC_NOT_DELIVERED};
    use crate::state::preview_payload::{DecodeError, PreviewPayload};
    use gpui_kit::component::Theme;

    #[test]
    fn decode_missing_file_is_honest_io_failure() {
        assert!(matches!(
            decode("definitely/missing/board.svg", None),
            PreviewPayload::Failed(DecodeError::Io(_))
        ));
    }

    #[test]
    fn diagram_description_formats_are_not_delivered() {
        assert_eq!(
            decode("flow.mermaid", Some(b"graph TD; A-->B")),
            PreviewPayload::NotDelivered {
                reason: DESC_NOT_DELIVERED
            }
        );
    }

    #[test]
    fn render_with_payload_builds_without_panic() {
        let p = PreviewPayload::NotDelivered {
            reason: DESC_NOT_DELIVERED,
        };
        let theme = Theme::default();
        let _ = render_with_payload("flow.mermaid", &p, &theme);
    }
}
