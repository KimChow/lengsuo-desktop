//! 波次 1 面板渲染断言：「记忆」面板。
//!
//! 断言口径见 `render_assert_tests/mod.rs`：不 panic + 元素可定位 + 布局非零。
//! ⚠️ 无窗口断言 **≠** 真机视觉验证（L3 另计，须在 Windows 实机做）。
use super::{assert_rendered, draw_probe};
use crate::ui::settings_gpui::view::PanelId;
use crate::ui::settings_gpui::SettingsGpui;
use gpui_kit::{AppContext as _, IntoElement as _};
/// 「记忆」面板必须真的画得出来，且布局非零。
#[test]
fn memory_panel_renders() {
    let bounds = draw_probe(|ctx| {
        ctx.new(|cx| SettingsGpui::with_panel(cx, PanelId::Memory))
            .into_any_element()
    });
    assert_rendered(bounds, "记忆面板");
}
