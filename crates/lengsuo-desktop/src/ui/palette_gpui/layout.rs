//! 面板的纯样式部件：遮罩 / 尺寸常量、底部键盘提示。
//!
//! 只做「描述字段 → 样式」的映射，不含任何状态与事件。
//!
//! 结果行（分组标题 + 条目）在 `search_rows.rs`。
#![cfg(feature = "ui")]

use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::command_palette;

/// 遮罩的淡化系数：把主题背景色的 alpha 减半，得到半透明遮罩层。
pub(super) const MASK_FADE: f32 = 0.5;

/// 面板宽度（像素）。
pub(super) const PANEL_WIDTH: f32 = 560.;
/// 命令列表最大高度（像素）。
pub(super) const LIST_MAX_HEIGHT: f32 = 320.;

/// 遮罩色：主题背景色淡化一半（不硬编码颜色）。
pub(super) fn mask_color(theme: &Theme) -> Hsla {
    let mut mask = theme.background;
    mask.fade_out(MASK_FADE);
    mask
}

/// 底部键盘提示。
pub(super) fn render_footer(
    view: &command_palette::CommandPaletteView,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(view.footer)
}
