//! 左栏：15 个面板的导航项。
//!
//! 全部用 `div()` 自搭（**不引入任何组件库**）⇒ 零新依赖、零 API 不确定性。
//!
//! ⚠️ GPUI 的 `on_click` 只在**带 id 的**元素（`Stateful<Div>`）上存在，
//! 裸 `Div` 没有这个方法 ⇒ 每个导航项必须先 `.id(...)`。
//! id 必须**同屏唯一**：这里用面板的稳定 slug（`settings-panel-<slug>`）派生，
//! 不用下标（下标会随清单变动而漂移）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use crate::ui::settings_gpui::view::SettingsView;

use super::window::SettingsGpui;

/// 左栏宽度（像素）。
pub(super) const SIDEBAR_WIDTH: f32 = 200.;

/// 渲染左栏：15 个面板导航项。
pub(super) fn render_sidebar(
    view: &SettingsView,
    theme: &Theme,
    cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    let mut column = div()
        .flex()
        .flex_col()
        .gap_1()
        .w(px(SIDEBAR_WIDTH))
        .flex_none()
        .h_full()
        .py_2()
        .pr_2()
        .border_r_1()
        .border_color(theme.border);

    for panel in &view.panels {
        let selected = panel.id == view.selected;
        let id = panel.id;
        let title = panel.title;

        let item = div()
            .id(format!("settings-panel-{}", id.slug()))
            .px_2()
            .py_1()
            .rounded_sm()
            .text_sm()
            .cursor_pointer();
        let item = if selected {
            item.bg(theme.accent)
                .text_color(theme.accent_foreground)
                .font_bold()
        } else {
            item.text_color(theme.muted_foreground)
        };

        column = column.child(item.child(title).on_click(cx.listener(
            move |this: &mut SettingsGpui,
                  _event: &ClickEvent,
                  _window: &mut Window,
                  cx: &mut Context<SettingsGpui>| {
                // 选中态变了才重绘。
                if this.view.select(id) {
                    cx.notify();
                }
            },
        )));
    }

    column.into_any_element()
}
