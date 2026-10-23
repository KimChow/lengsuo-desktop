//! 图片预览（`PreviewKind::Image`）的内容渲染——**由 `ui-preview-image` 卡实现**。
//!
//! 本卡只做**视图与状态**：图片容器 / 元信息行 / 缩放控件 / 未下发态。
//! 硬约束：⛔ 不读文件、⛔ 不解码图像（解码需新增依赖，属停止条件）；
//! 图像内容由运行时契约下发，未下发时诚实显示，⛔ 不伪造图像。
//!
//! 纯函数层（本段）无 GPUI 依赖，可无窗口测试。
use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

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
    if fit { "适应窗口" } else { "原始大小" }
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
fn render_with(
    path: &str,
    fit: bool,
    zoom: f32,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    // 视图边界把传入缩放归一到合法档位（非法值落 1.0）；
    // 点击步进接线后，同一函数承担 +1/-1 步进。
    let zoom = next_zoom(zoom, 0);
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(meta_row(path, theme))
        .child(image_container(theme))
        .child(zoom_row(fit, zoom, theme))
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

/// 图片容器：`muted` 底 + `border` 描边 + 居中占位图记号；未下发时诚实占位。
fn image_container(theme: &gpui_kit::component::Theme) -> Div {
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
                .child(IMAGE_NOT_DELIVERED),
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
