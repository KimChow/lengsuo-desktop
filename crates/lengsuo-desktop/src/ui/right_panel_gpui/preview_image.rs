//! 图片预览（`PreviewKind::Image`）的内容渲染——**由 `ui-preview-image` 卡实现**。
//!
//! 本卡只做**视图与状态**：图片容器 / 元信息行 / 缩放控件 / 未下发态。
//! 硬约束：⛔ 不读文件、⛔ 不解码图像（解码需新增依赖，属停止条件）；
//! 图像内容由运行时契约下发，未下发时诚实显示，⛔ 不伪造图像。
//!
//! 纯函数层（本段）无 GPUI 依赖，可无窗口测试。

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
