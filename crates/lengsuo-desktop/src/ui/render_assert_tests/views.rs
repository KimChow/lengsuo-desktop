//! E 段：此前零渲染断言的视图族 —— 每个 `impl Render` 至少真的画一帧。
//!
//! 只断言「画得出来 + 布局非零（+ 声明宽度真的生效）」，不做像素级断言：
//! 无窗口环境没有 GPU 光栅化。
use super::{assert_rendered, draw_probe, CANVAS_H, CANVAS_W};

use gpui_kit::{div, px, AppContext as _, IntoElement as _, ParentElement as _, Styled as _};

use crate::state::session_stream::SessionStreamState;
use crate::ui::layout::{RightPanelLayout, SIDEBAR_COLLAPSED_WIDTH};
use crate::ui::layout_gpui::{DividerGpui, DividerSide, RightPanelTabGpui};
use crate::ui::palette_gpui::CommandPaletteGpui;
use crate::ui::right_panel_gpui::RightPanelGpui;
use crate::ui::session_gpui::SessionStreamGpui;
use crate::ui::settings_gpui::SettingsGpui;
use crate::ui::sidebar_gpui::SidebarViewGpui;

/// 会话流**实体**（不只是纯函数 `render_block`）。
#[test]
fn session_stream_entity_renders() {
    let bounds = draw_probe(|ctx| ctx.new(|_cx| SessionStreamGpui::demo()).into_any_element());
    assert_rendered(bounds, "会话流实体");
}

/// 侧栏：默认状态要画得出来。
#[test]
fn sidebar_view_renders() {
    let bounds = draw_probe(|ctx| ctx.new(|_cx| SidebarViewGpui::demo()).into_any_element());
    assert_rendered(bounds, "侧栏");
}

/// ★ 关键断言：宿主下发的宽度**真的作用到布局上**。
///
/// `sidebar_gpui/mod.rs:237` 用 `render(&self.state, self.width)`；
/// 写死默认宽度会让拖分隔条后内容仍按默认宽渲染（2026-10-21 真机实测过）。
/// 这条能抓到「宽度没接上」——只断言不 panic 抓不到。
#[test]
fn sidebar_view_honours_host_width() {
    const W: f32 = 320.0;
    let bounds = draw_probe(|ctx| {
        ctx.new(|_cx| {
            let mut view = SidebarViewGpui::demo();
            view.set_width(W);
            view
        })
        .into_any_element()
    });
    let bounds = assert_rendered(bounds, "侧栏（自定义宽度）");
    assert!(
        bounds.size.width > px(W - 1.0) && bounds.size.width < px(W + 1.0),
        "侧栏宽度没按宿主下发值渲染：期望 {W}px 实测 {:?}",
        bounds.size.width
    );
}

/// 折叠态只渲染窄条 ⇒ 必须比展开态窄（抓「折叠分支没生效」）。
///
/// ⚠️ 折叠分支本身不改变外框宽度：`render` 两个分支都用宿主下发的
/// `self.width`（`sidebar_gpui/mod.rs:246` / `:302`）。真实宿主在折叠时把
/// 宽度切到 [`SIDEBAR_COLLAPSED_WIDTH`]，所以这里按宿主流程走：先
/// `toggle_collapsed()` 再下发窄条宽度。
#[test]
fn sidebar_collapsed_is_narrower_than_expanded() {
    let expanded = draw_probe(|ctx| ctx.new(|_cx| SidebarViewGpui::demo()).into_any_element());
    let collapsed = draw_probe(|ctx| {
        ctx.new(|_cx| {
            let mut view = SidebarViewGpui::demo();
            view.toggle_collapsed();
            view.set_width(SIDEBAR_COLLAPSED_WIDTH);
            view
        })
        .into_any_element()
    });
    let expanded = assert_rendered(expanded, "侧栏（展开）");
    let collapsed = assert_rendered(collapsed, "侧栏（折叠）");
    assert!(
        collapsed.size.width < expanded.size.width,
        "折叠态没有比展开态窄 ⇒ 折叠分支没生效（collapsed={:?} expanded={:?}）",
        collapsed.size.width,
        expanded.size.width
    );
}

/// 命令面板：**不灌演示条目**（fail-closed），无注入也要画得出来。
///
/// 面板默认关闭（关闭即渲染空元素），这里只调用 `open()`、不注入条目。
/// 面板本体是 `.absolute().size_full()` 的遮罩层，脱离文档流 ⇒ 测试侧给一个
/// 固定画布当宿主容器，否则探针量到的尺寸恒为 0。
#[test]
fn command_palette_renders_without_items() {
    let bounds = draw_probe(|ctx| {
        let palette = ctx
            .new(|cx| {
                let mut view = CommandPaletteGpui::new(cx);
                view.state_mut().open();
                view
            })
            .into_any_element();
        div()
            .w(px(CANVAS_W))
            .h(px(CANVAS_H))
            .child(palette)
            .into_any_element()
    });
    assert_rendered(bounds, "命令面板（无注入）");
}

/// 设置窗：默认构造（能力标记为空 ⇒ 面板显示「服务端未下发」）。
#[test]
fn settings_window_renders() {
    let bounds = draw_probe(|ctx| ctx.new(SettingsGpui::new).into_any_element());
    assert_rendered(bounds, "设置窗");
}

/// 右栏。
#[test]
fn right_panel_renders() {
    let bounds = draw_probe(|ctx| {
        ctx.new(|_cx| {
            RightPanelGpui::new(RightPanelLayout::default(), SessionStreamState::default())
        })
        .into_any_element()
    });
    assert_rendered(bounds, "右栏");
}

/// 右栏 tab 条。
#[test]
fn right_panel_tab_strip_renders() {
    let bounds = draw_probe(|ctx| {
        ctx.new(|_cx| RightPanelTabGpui::new(RightPanelLayout::default()))
            .into_any_element()
    });
    assert_rendered(bounds, "右栏 tab 条");
}

/// 分隔条：左右两侧样式不同，都必须能画。
///
/// 分隔条是 `h_full()` ⇒ 必须有确定高度的宿主容器才有布局尺寸。
#[test]
fn divider_renders_both_sides() {
    for side in [DividerSide::Left, DividerSide::Right] {
        let bounds = draw_probe(|ctx| {
            let divider = ctx.new(|_cx| DividerGpui::new(side)).into_any_element();
            div().h(px(200.0)).child(divider).into_any_element()
        });
        assert_rendered(bounds, "分隔条");
    }
}
