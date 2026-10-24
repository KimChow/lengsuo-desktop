//! 图片预览（`PreviewKind::Image`）的内容渲染——**由 `ui-preview-image` 卡实现**。
//!
//! 本卡只做**视图与状态**：图片容器 / 元信息行 / 缩放控件 / 未下发态。
//! 硬约束：⛔ 不读文件、⛔ 不解码图像（解码需新增依赖，属停止条件）；
//! 图像内容由运行时契约下发，未下发时诚实显示，⛔ 不伪造图像。
//!
//! 纯函数层（本段）无 GPUI 依赖，可无窗口测试。
use std::io::{Cursor, Read as _};
use std::sync::Arc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use crate::state::preview_payload::{DecodeError, ImagePayload, PreviewPayload, MAX_BYTES};

/// 缩放档位（0.25 / 0.5 / 1.0 / 2.0 / 4.0）。
pub(super) fn zoom_steps() -> [f32; 5] {
    [0.25, 0.5, 1.0, 2.0, 4.0]
}

/// 在 `zoom_steps` 上步进并夹紧：`dir > 0` 放大、`dir < 0` 缩小、`0` 原档不动；
/// 越过两端停在首/末档；`cur` 不在档位上（含 NaN）落回 1.0。
pub(super) fn next_zoom(cur: f32, dir: i8) -> f32 {
    let steps = zoom_steps();
    let Some(idx) = steps.iter().position(|&s| s == cur) else {
        return 1.0;
    };
    let next = (idx as i8 + dir.signum()).clamp(0, steps.len() as i8 - 1);
    steps[next as usize]
}

/// 适应开关的当前态文案：`true` = 适应窗口，`false` = 原始大小。
pub(super) fn fit_label(fit: bool) -> &'static str {
    if fit {
        "适应窗口"
    } else {
        "原始大小"
    }
}

// ---------------------------------------------------------------------------
// 接线层（`div()` 自搭，⛔ 不引入新组件库）
// ---------------------------------------------------------------------------

use crate::state::preview_kind::classify;

/// 未下发态的诚实文案：图像数据由运行时契约下发，本层不伪造。
const IMAGE_NOT_DELIVERED: &str = "图像未下发（等运行时契约）";

/// 渲染图片预览（默认态：适应窗口 + 100%）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    render_with(path, true, 1.0, theme)
}

/// 带显式状态的纯视图函数：渲染结果只依赖参数，不依赖 GPUI 侧可变状态。
fn render_with(path: &str, fit: bool, zoom: f32, theme: &gpui_kit::component::Theme) -> AnyElement {
    render_shell(path, fit, zoom, IMAGE_NOT_DELIVERED, theme)
}

/// 外壳底座：元信息行 + 占位容器（提示文案可换）+ 缩放行。
/// 视图边界把传入缩放归一到合法档位（非法值落 1.0）；点击步进接线后同一函数承担步进。
fn render_shell(
    path: &str,
    fit: bool,
    zoom: f32,
    note: &str,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(meta_row(path, theme))
        .child(image_container(note, theme))
        .child(zoom_row(fit, next_zoom(zoom, 0), theme))
        .into_any_element()
}

/// 元信息行：文件名（真实 `path` 的末段）+ 判定类型标签，均不编造。
fn meta_row(path: &str, theme: &gpui_kit::component::Theme) -> Div {
    let file_name = path.rsplit(['/', '\\']).next().unwrap_or(path);
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
                .child(file_name.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(classify(path).label()),
        )
}

/// 图片容器：`muted` 底 + `border` 描边 + 居中占位图记号 + 一行如实提示（文案由调用方给）。
fn image_container(note: &str, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .w_full()
        .min_h_48()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .child(placeholder_mark(theme))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(note.to_string()),
        )
}

/// 居中的占位图记号：空描边画框 + 角上圆点（纯几何记号，不是伪图像）。
fn placeholder_mark(theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .items_start()
        .justify_end()
        .p_2()
        .size_12()
        .rounded_sm()
        .border_1()
        .border_color(theme.muted_foreground)
        .child(div().size_3().rounded_full().bg(theme.muted_foreground))
}

/// 缩放控件行：`-` / 百分比 / `+` / 适应开关，触摸目标 ≥48dp。
fn zoom_row(fit: bool, zoom: f32, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(zoom_button("-", theme))
        .child(
            div()
                .flex()
                .min_h_12()
                .min_w_12()
                .items_center()
                .justify_center()
                .text_xs()
                .font_family(theme.mono_font_family.clone())
                .text_color(theme.foreground)
                .child(format!("{:.0}%", zoom * 100.0)),
        )
        .child(zoom_button("+", theme))
        .child(fit_toggle(fit, theme))
}

/// 单个缩放按钮：`min_h_12`/`min_w_12` = 48dp 触摸目标。
fn zoom_button(label: &str, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .min_h_12()
        .min_w_12()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .text_sm()
        .text_color(theme.foreground)
        .child(label.to_string())
}

/// 适应开关按钮：文案即当前态（`fit_label`）。
fn fit_toggle(fit: bool, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .min_h_12()
        .px_3()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .text_xs()
        .text_color(theme.foreground)
        .child(fit_label(fit).to_string())
}

// ---------------------------------------------------------------------------
// A14 解码入口（a14-preview-decode-deps 基建落盘；**诚实占位**）
// ---------------------------------------------------------------------------
//
// 契约（A14 卡在此填实，⛔ 不要改签名、⛔ 不要动别的文件）：
//   · `decode`               —— 把「待预览文件」解码成载荷。字节可由运行时下发（`bytes`）；
//                              未下发时自行读本地文件。拿不到内容 ⇒ `NotDelivered` / `Failed`，
//                              ⛔ 绝不伪造内容、⛔ 不吞错。
//   · `render_with_payload`  —— 把载荷画出来。**默认退回既有占位视图**，因此外壳
//                              （头信息 / 缩放控件 / 「未下发」提示行）行为与本次基建前一致。

/// 解压后像素数上限（16 MP）：防解压炸弹，超限 fail-closed。
const MAX_PIXELS: u64 = 16_000_000;

/// 读文件前 [`MAX_BYTES`] 字节（⛔ 不整包进内存），返回 (字节, 原始长度)。
fn read_capped(path: &str) -> Result<(Vec<u8>, usize), String> {
    let len = std::fs::metadata(path).map_err(|e| e.to_string())?.len() as usize;
    let mut buf = vec![0u8; len.min(MAX_BYTES)];
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    file.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok((buf, len))
}

/// 从字节解出 (格式名, 宽, 高)；识别不出 / 尺寸解不出 ⇒ Err（fail-closed）。
fn sniff(data: &[u8]) -> Result<(String, u32, u32), String> {
    let reader = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let format = reader.format().ok_or("无法识别图像格式")?;
    let (width, height) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(format!("像素数超限（{width}×{height} > {MAX_PIXELS}）"));
    }
    Ok((format!("{format:?}").to_lowercase(), width, height))
}

/// A14：解码待预览图片。
///
/// 字节优先用运行时下发（`bytes`：⛔ 只用传入字节，不碰文件系统）；
/// 未下发则读 `path`（按 [`MAX_BYTES`] 截断）。拿不到 / 识别不出 ⇒ `Failed`，⛔ 绝不伪造。
pub(super) fn decode(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let (capped, source_len) = match bytes {
        Some(b) => (b[..b.len().min(MAX_BYTES)].to_vec(), b.len()),
        None => match read_capped(path) {
            Ok(pair) => pair,
            Err(e) => return PreviewPayload::Failed(DecodeError::Io(e)),
        },
    };
    let (format, width, height) = match sniff(&capped) {
        Ok(it) => it,
        Err(msg) => return PreviewPayload::Failed(DecodeError::Parse(msg)),
    };
    PreviewPayload::Image(ImagePayload {
        bytes: capped,
        format,
        width,
        height,
        source_len,
    })
}

/// 载荷的元信息行：宽×高 · 格式 · 字节数（截断时如实标注原大小）。
fn info_line(p: &ImagePayload) -> String {
    if p.source_len > p.bytes.len() {
        format!(
            "{} × {} · {} · 已截断（原 {} 字节，预览 {} 字节）",
            p.width,
            p.height,
            p.format,
            p.source_len,
            p.bytes.len()
        )
    } else {
        format!(
            "{} × {} · {} · {} 字节",
            p.width,
            p.height,
            p.format,
            p.bytes.len()
        )
    }
}

/// A14：带载荷渲染。图片载荷 ⇒ 真像素（整图解码成功时，适应窗口夹紧）；
/// 其余态（未下发 / 失败 / 非图片载荷）如实说明，⛔ 不伪造图像。
pub(super) fn render_with_payload(
    path: &str,
    payload: &PreviewPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    match payload {
        PreviewPayload::Image(p) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(meta_row(path, theme))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(info_line(p)),
            )
            .child(image_view(p, theme))
            .child(zoom_row(true, zoom_steps()[2], theme))
            .into_any_element(),
        PreviewPayload::NotDelivered { .. } => render(path, theme),
        PreviewPayload::Failed(err) => render_shell(path, true, 1.0, &err.line(), theme),
        _ => render_shell(path, true, 1.0, "载荷不是图片（类型不匹配）", theme),
    }
}

/// 图片视图：整图解码成功 ⇒ `img()` 真像素；失败（如截断字节解不出整图）⇒ 如实占位。
fn image_view(p: &ImagePayload, theme: &gpui_kit::component::Theme) -> Div {
    let container = div()
        .flex()
        .w_full()
        .min_h_48()
        .items_center()
        .justify_center()
        .overflow_hidden()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted);
    match image::load_from_memory(&p.bytes) {
        Ok(decoded) => container.child(
            img(Arc::new(RenderImage::new(vec![image::Frame::new(
                decoded.to_rgba8(),
            )])))
            .object_fit(ObjectFit::Contain)
            .w_full(),
        ),
        Err(_) => container.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child("像素渲染不可用（字节不完整，无法整图解码）"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{fit_label, next_zoom, zoom_steps};

    #[test]
    fn zoom_steps_cover_quarter_to_4x() {
        assert_eq!(zoom_steps(), [0.25, 0.5, 1.0, 2.0, 4.0]);
    }

    #[test]
    fn next_zoom_clamps_at_both_ends() {
        assert_eq!(next_zoom(4.0, 1), 4.0);
        assert_eq!(next_zoom(0.25, -1), 0.25);
        assert_eq!(next_zoom(1.0, 1), 2.0);
        assert_eq!(next_zoom(1.0, -1), 0.5);
        assert_eq!(next_zoom(1.0, 0), 1.0);
    }

    #[test]
    fn next_zoom_invalid_falls_back_to_100pct() {
        assert_eq!(next_zoom(0.3, 1), 1.0);
        assert_eq!(next_zoom(f32::NAN, -1), 1.0);
    }

    #[test]
    fn fit_label_both_states() {
        assert_eq!(fit_label(true), "适应窗口");
        assert_eq!(fit_label(false), "原始大小");
    }
}

/// 接线层的无窗口渲染断言：`TestAppContext` + `VisualTestContext::draw`
/// 跑完整 layout + prepaint + paint。
///
/// ⛔ 不写 `use super::*`：避免 GPUI 的 `test` 属性宏遮蔽标准库 `#[test]`。
#[cfg(all(test, feature = "ui"))]
mod render_tests {
    use super::{render_with, zoom_steps};
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, AnyElement, Bounds, InteractiveElement as _, IntoElement,
        ParentElement as _, Pixels, Styled as _, TestAppContext,
    };

    /// 探针选择器（每帧一个，用例独立窗口）。
    const PROBE: &str = "preview-image-probe";
    const CANVAS_W: f32 = 800.0;
    const CANVAS_H: f32 = 600.0;

    /// 无窗口渲染一帧，返回探针实际布局 bounds；`None` ⇒ 探针没进渲染帧。
    fn draw_probe(build: impl FnOnce(&Theme) -> AnyElement) -> Option<Bounds<Pixels>> {
        let mut ctx = TestAppContext::single();
        ctx.update(|cx| cx.set_global(Theme::default()));

        let element = build(&Theme::default());

        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(CANVAS_W), px(CANVAS_H)),
            |_window, _cx| {
                div()
                    .flex()
                    .items_start()
                    .child(div().debug_selector(|| PROBE.to_owned()).child(element))
                    .into_any_element()
            },
        );
        visual.debug_bounds(PROBE)
    }

    fn assert_rendered(bounds: Option<Bounds<Pixels>>, what: &str) -> Bounds<Pixels> {
        let bounds =
            bounds.unwrap_or_else(|| panic!("{what}：探针没进入渲染帧（debug_selector 未记录）"));
        assert!(
            bounds.size.width > px(0.0) && bounds.size.height > px(0.0),
            "{what}：布局宽高为 0，元素没真正参与排版"
        );
        bounds
    }

    /// ① 默认态（适应窗口 + 100%）：渲染元素树非空。
    #[test]
    fn image_panel_renders_fit_state() {
        let bounds =
            draw_probe(|theme| render_with("shots/screen.png", true, zoom_steps()[2], theme));
        assert_rendered(bounds, "图片预览（适应窗口态）");
    }

    /// ② 原始大小态（fit=false + 最小档）：分支切换后仍画得出来。
    #[test]
    fn image_panel_renders_original_size_state() {
        let bounds = draw_probe(|theme| render_with("a/b.jpg", false, zoom_steps()[0], theme));
        assert_rendered(bounds, "图片预览（原始大小态）");
    }

    /// ③ 非法缩放档（0.3 不在档位上）在视图边界归一为 1.0，不炸渲染。
    #[test]
    fn image_panel_normalizes_invalid_zoom() {
        let bounds = draw_probe(|theme| render_with("c/d.webp", true, 0.3, theme));
        assert_rendered(bounds, "图片预览（非法档位归一态）");
    }
}
