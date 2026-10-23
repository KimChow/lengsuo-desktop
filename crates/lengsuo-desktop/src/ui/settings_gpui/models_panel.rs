//! 「模型」面板的接线层（`feature = "ui"` 隔离）。
//!
//! ⚠️ 本文件当前是**波次 1 占位实现**：只把描述层的标题 + 「未下发」提示
//! 映射成元素，证明「该面板在 15 项导航里可点、点了真的渲染出内容」。
//! 由 `ui-settings-models` 卡补成完整面板（文案逐字取自 `settings.models`）。
//!
//! 硬约束：**只读**、**fail-closed**、颜色**一律取自 `cx.theme()`**（⛔ 不硬编码）、
//! ⛔ 界面不判定权限、⛔ 界面不持有凭据、⛔ 不伪造服务端数据。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::models as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 渲染「模型」面板（当前为占位：标题 + 「未下发」提示）。
pub(super) fn render_models_panel(theme: &Theme, _cx: &mut Context<SettingsGpui>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(panel_view::TITLE),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::NOT_DELIVERED),
        )
        .into_any_element()
}
