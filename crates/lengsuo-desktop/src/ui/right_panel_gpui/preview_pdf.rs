//! PDF 预览（`PreviewKind::Pdf`）的视图与状态——页工具栏 / 缩放控件 / 未下发态（A14）。
//!
//! ⛔ 不读文件、不解析 PDF：页内容与总页数由运行时契约下发，
//! 未下发时诚实显示占位文案，不伪造页内容。
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

/// 渲染 PDF 预览面板：页工具栏 + 页面占位区（当前未下发）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    render_panel(path, &PdfPanelState::undelivered(), theme)
}

/// 面板主体：工具栏在上，页面占位区撑满余下空间。
fn render_panel(
    path: &str,
    state: &PdfPanelState,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(render_toolbar(state, theme))
        .child(render_page_area(path, theme))
        .into_any_element()
}

/// 页工具栏：左「上一页 / 页码 / 下一页」，右「`-` / 百分比 / `+`」；触摸目标 ≥48dp。
fn render_toolbar(state: &PdfPanelState, theme: &gpui_kit::component::Theme) -> AnyElement {
    let next_page = page_step(state.page, state.total, 1);
    let prev_page = page_step(state.page, state.total, -1);
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(tool_button(
                    "<",
                    format!("上一页 {}→{}", state.page, prev_page),
                    theme,
                ))
                .child(metric_label(page_label(state.page, state.total), theme))
                .child(tool_button(
                    ">",
                    format!("下一页 {}→{}", state.page, next_page),
                    theme,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(tool_button(
                    "-",
                    format!("缩小 {:.2}→{:.2}", state.zoom, zoom_step(state.zoom, -0.25)),
                    theme,
                ))
                .child(metric_label(zoom_label(state.zoom), theme))
                .child(tool_button(
                    "+",
                    format!("放大 {:.2}→{:.2}", state.zoom, zoom_step(state.zoom, 0.25)),
                    theme,
                )),
        )
        .into_any_element()
}

/// 48×48 的工具按钮；点击只上报意图（不判定、不改宿主状态）。
fn tool_button(
    label: &'static str,
    intent: String,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    div()
        .flex_none()
        .w_12()
        .h_12()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .hover(|style| style.bg(theme.muted))
        .child(label)
        .id(label)
        .on_click(move |_event, _window, _app| {
            eprintln!("[lengsuo-desktop] PDF 预览点击（意图未接线）：{intent}");
        })
        .into_any_element()
}

/// 页码 / 缩放读数：等宽字体 + 48dp 最小宽，读数变化不挤动相邻按钮。
fn metric_label(text: String, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .flex_none()
        .min_w_12()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .font_family(theme.mono_font_family.clone())
        .text_color(theme.foreground)
        .child(text)
        .into_any_element()
}

/// 页面占位区：`theme.muted` 底 + `theme.border` 描边；未下发态诚实占位，⛔ 不伪造页内容。
fn render_page_area(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .flex_1()
        .m_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(theme.muted)
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .text_color(theme.muted_foreground)
                .child(div().text_sm().child("PDF 未下发（等运行时契约）"))
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.mono_font_family.clone())
                        .child(path.to_string()),
                ),
        )
        .into_any_element()
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

/// A14：解码待预览文件。当前为**诚实占位**（不读文件、不解码）。
pub(super) fn decode(
    _path: &str,
    _bytes: Option<&[u8]>,
) -> crate::state::preview_payload::PreviewPayload {
    crate::state::preview_payload::PreviewPayload::NotDelivered {
        reason: crate::state::preview_payload::DECODE_NOT_DELIVERED,
    }
}

/// A14：带载荷渲染。默认退回既有占位视图（外壳不变）。
pub(super) fn render_with_payload(
    path: &str,
    payload: &crate::state::preview_payload::PreviewPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let _ = payload;
    render(path, theme)
}

#[cfg(test)]
mod tests {
    use super::{page_label, page_step, zoom_label, zoom_step, PdfPanelState};

    #[test]
    fn page_step_clamps_to_bounds() {
        assert_eq!(page_step(12, 12, 1), 12, "上界夹紧：不越过总页数");
        assert_eq!(page_step(1, 12, -1), 1, "下界夹紧：不小于第 1 页");
        assert_eq!(page_step(3, 12, 1), 4);
        assert_eq!(page_step(3, 12, -1), 2);
        assert_eq!(page_step(1, 0, 1), 1, "未下发（total=0）：停在 1");
    }

    #[test]
    fn page_label_formats() {
        assert_eq!(page_label(3, 12), "3 / 12");
        assert_eq!(page_label(1, 1), "1 / 1");
        assert_eq!(page_label(1, 0), "– / –", "未下发：不伪造页码");
    }

    #[test]
    fn zoom_step_clamps() {
        assert_eq!(zoom_step(4.0, 0.5), 4.0, "上界夹紧：不越过 400%");
        assert_eq!(zoom_step(0.25, -0.5), 0.25, "下界夹紧：不小于 25%");
        assert_eq!(zoom_step(1.0, 0.25), 1.25);
    }

    #[test]
    fn zoom_label_formats() {
        assert_eq!(zoom_label(1.0), "100%");
        assert_eq!(zoom_label(1.25), "125%");
        assert_eq!(zoom_label(0.25), "25%");
    }

    #[test]
    fn undelivered_state_is_honest() {
        let state = PdfPanelState::undelivered();
        assert_eq!(state.total, 0);
        assert_eq!(page_label(state.page, state.total), "– / –");
        assert_eq!(zoom_label(state.zoom), "100%");
    }
}
