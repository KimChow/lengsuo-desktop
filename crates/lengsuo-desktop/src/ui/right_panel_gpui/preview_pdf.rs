//! PDF 预览（`PreviewKind::Pdf`）的内容渲染——**由 `ui-preview-pdf` 卡实现**。
//!
//! 主理人落盘骨架：当前只给诚实占位，⛔ 不读文件、不解析 PDF。
//! 颜色一律取自 `theme`，⛔ 不硬编码。
use gpui_kit::*;

/// PDF 预览面板的纯状态：渲染是它的纯函数（A14 视图层）。
///
/// 页内容与总页数由运行时契约下发；`total == 0` 表示未下发。
#[derive(Clone, Copy, Debug, PartialEq)]
struct PdfPanelState {
    /// 当前页（1 起）。
    page: usize,
    /// 总页数；`0` = 内容未下发。
    total: usize,
    /// 缩放倍率（0.25~4.0）。
    zoom: f32,
}

impl PdfPanelState {
    /// 未下发的初始态：停在第 1 页、总页数未知、100% 缩放。
    fn undelivered() -> Self {
        Self {
            page: 1,
            total: 0,
            zoom: 1.0,
        }
    }
}

/// 页码步进：`cur + delta` 夹紧到 `1..=total`（未下发时停在 1）。
fn page_step(cur: usize, total: usize, delta: i8) -> usize {
    let next = cur as i64 + i64::from(delta);
    let upper = total.max(1) as i64;
    next.clamp(1, upper) as usize
}

/// 页码标签：形如 `3 / 12`；未下发（`total == 0`）时为 `– / –`。
fn page_label(cur: usize, total: usize) -> String {
    if total == 0 {
        "– / –".to_string()
    } else {
        format!("{cur} / {total}")
    }
}

/// 缩放步进：`cur + delta` 夹紧到 0.25~4.0。
fn zoom_step(cur: f32, delta: f32) -> f32 {
    (cur + delta).clamp(0.25, 4.0)
}

/// 缩放标签：形如 `100%`。
fn zoom_label(zoom: f32) -> String {
    format!("{}%", (zoom * 100.0).round() as u32)
}

/// 渲染PDF 预览（占位，待 `ui-preview-pdf` 填充）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(format!("{path}：PDF 预览尚未实现"))
        .into_any_element()
}
