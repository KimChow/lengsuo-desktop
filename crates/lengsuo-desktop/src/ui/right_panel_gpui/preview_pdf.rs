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
fn render_panel(path: &str, state: &PdfPanelState, theme: &gpui_kit::component::Theme) -> AnyElement {
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
fn tool_button(label: &'static str, intent: String, theme: &gpui_kit::component::Theme) -> AnyElement {
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
                        .child(format!("{path}")),
                ),
        )
        .into_any_element()
}
