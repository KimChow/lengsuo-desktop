//! 右侧面板的 **tab 入口**渲染：tab 按钮、角标、折叠 / 展开按钮。
//!
//! 只把描述层（`crate::ui::right_panel`）给出的展示信息变成元素，
//! **不重新实现**任何业务判断。

use gpui_kit::*;

use super::{RightPanelGpui, COLLAPSE_LABEL, EXPAND_LABEL};
use crate::ui::layout::RightPanelTab;
use crate::ui::right_panel::RightPanelView;

/// 一个 tab 入口的展示信息（纯数据，可在无窗口环境测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabEntry {
    /// tab 身份（点击时回写版式状态）。
    pub tab: RightPanelTab,
    /// 完整标签（展开态用）。
    pub label: String,
    /// 紧凑标签（折叠窄条用）：取描述层标签的首字。
    pub compact: String,
    /// 是否选中。
    pub selected: bool,
    /// 角标数量；`None` 表示不显示角标。
    pub badge: Option<usize>,
}

/// 折叠窄条里的紧凑标签：取描述层标签的首字。
///
/// 这样窄条里仍然能分辨三个 tab，且不会另造一套标签文案
/// —— 文案只有一个来源（描述层）。
pub(super) fn compact_label(label: &str) -> String {
    label.chars().next().map(String::from).unwrap_or_default()
}

/// 把描述层的 tab 列表转成展示条目。
pub(super) fn tab_entries(view: &RightPanelView) -> Vec<TabEntry> {
    view.tabs
        .iter()
        .map(|tab| TabEntry {
            tab: tab.tab,
            label: tab.label.to_owned(),
            compact: compact_label(tab.label),
            selected: tab.selected,
            badge: tab.badge,
        })
        .collect()
}

/// tab 上的角标数字。
fn render_badge(count: usize, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .rounded_full()
        .px_1()
        .text_xs()
        .text_color(theme.button_warning_foreground)
        .bg(theme.button_warning)
        .child(count.to_string())
}

/// 一个 tab 入口。
///
/// `collapsed` 为真时只显示紧凑标签（窄条），否则显示完整标签。
/// 点击切换 tab：描述层规定折叠态选 tab 会展开面板。
pub(super) fn render_tab_button(
    entry: &TabEntry,
    collapsed: bool,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let label_color = if entry.selected {
        theme.accent_foreground
    } else {
        theme.muted_foreground
    };
    let label = if collapsed {
        entry.compact.clone()
    } else {
        entry.label.clone()
    };

    let mut button = div()
        .id(format!("right-panel-tab-{}", entry.tab.id()))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .cursor_pointer();
    if collapsed {
        button = button.w_full();
    } else {
        button = button.flex_1();
    }
    if entry.selected {
        button = button.bg(theme.accent);
    }
    let mut button = button.child(div().text_sm().text_color(label_color).child(label));
    if let Some(count) = entry.badge {
        button = button.child(render_badge(count, theme));
    }

    let entity = entity.clone();
    let tab = entry.tab;
    button.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            view.layout_mut().select_tab(tab);
            cx.notify();
        });
    })
}

/// 折叠 / 展开按钮。
pub(super) fn render_toggle(
    collapsed: bool,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let label = if collapsed {
        EXPAND_LABEL
    } else {
        COLLAPSE_LABEL
    };
    let mut button = div()
        .id("right-panel-toggle")
        .px_2()
        .py_1()
        .rounded_sm()
        .text_xs()
        .border_1()
        .border_color(theme.border)
        .cursor_pointer();
    button = if collapsed {
        button.text_color(theme.muted_foreground)
    } else {
        button.text_color(theme.foreground)
    };
    let button = button.child(label);

    let entity = entity.clone();
    button.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            view.layout_mut().toggle_collapsed();
            cx.notify();
        });
    })
}
