//! `AppShell` 的渲染：标题栏 + 三区 + 输入区 + 终端抽屉 + 命令面板浮层。
//!
//! 从 `mod.rs` 拆出（e50）：**渲染顺序与注释原文一律保留**，尤其这两条不许改写：
//! - `title_bar` 必须**最先画**；
//! - 终端抽屉必须排在命令面板**之前**（命令面板是 `absolute` 全屏浮层，后画的会盖住它）。
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use super::launch::{HOTKEY_HINT, WINDOW_TITLE};
use super::main_area;
use super::shell::AppShell;
use super::sync::regions_for;

impl AppShell {
    /// 标题栏：窗口标题 + 快捷键提示。
    pub(super) fn title_bar(theme: &gpui_kit::component::Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .bg(theme.title_bar)
            .border_b_1()
            .border_color(theme.title_bar_border)
            .child(div().text_sm().font_bold().child(WINDOW_TITLE))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(HOTKEY_HINT),
            )
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 兜底回显：输入区发送后 `cx.notify()` 会经 observe 触发 `drain_outbox`；
        // 这里在组装元素树之前再消费一次，保证同一帧内回显已写入会话流。只读消费，不投递。
        self.drain_outbox(cx);

        // 兜底回写：右栏点审批按钮后 `cx.notify()` 会经 observe 触发 drain；
        // 这里在组装元素树之前再消费一次，保证同一帧内决策已进会话流。
        self.drain_approval_decisions(cx);

        let viewport_width = f32::from(window.viewport_size().width);
        // 三区宽度**每次渲染都从左分隔条读回**（拖拽结果落在它内部），
        // 根视图不另存版式，因此不存在第二份不同步的副本。
        let widths = regions_for(self.left_divider.read(cx).layout(), viewport_width);
        // 把实际宽度**下发给侧栏视图**：它此前写死 `SIDEBAR_DEFAULT_WIDTH` 渲染
        // 自身内容，导致拖动后内容仍按 260 画（2026-10-21 真机实测）。
        // 必须在 `cx.theme()` 之前做（那里开始借用 `cx`）。
        // 同时下发折叠态：第一帧就要按水合后的偏好渲染（否则权威版式只留 44px，
        // 侧栏却画成展开态）。
        let sidebar_collapsed = self.left_divider.read(cx).layout().sidebar().is_collapsed();
        self.sidebar.update(cx, |view, _| {
            view.set_width(widths.sidebar);
            view.set_collapsed(sidebar_collapsed);
        });

        let theme = cx.theme();
        // 左侧边栏：宽度来自分隔条持有的版式（默认 260px，可拖拽）。
        // **必须 `flex_none()`**：在 flex 行里默认 `flex_shrink: 1`，只写 `.w()` 会被
        // 收缩，表现成「拖了分隔条、边栏宽度却没变」（2026-10-21 真机实测）。
        let sidebar = div()
            .w(px(widths.sidebar))
            .flex_none()
            .h_full()
            .child(self.sidebar.clone());

        // 中间主区：按侧栏选中的导航入口切换显示哪一块。
        // ⛔ 频道 / 智能体管理**只显示占位说明**（数据源在运行时侧，本仓不实现）。
        // ✅ 环境管理是**真实视图**（演示数据），点侧栏「环境」入口即切过去。
        let stream = main_area::render_main_area(
            self.main_view,
            &self.session,
            &self.env_view,
            &self.org_view,
            theme,
        );

        // 右侧面板：上方是 tab 条（点击切换面板内容），下方是面板本身。
        // 同 sidebar：必须 `flex_none()`，否则拖拽改的宽度会被 flex 收缩吞掉。
        let right_panel = div()
            .flex()
            .flex_col()
            .w(px(widths.right_panel))
            .flex_none()
            .h_full()
            .child(self.tab_strip.clone())
            .child(div().flex_1().child(self.right_panel.clone()));

        // 中右两列：上面是会话流 + 分隔条 + 右栏，下面是输入区。
        let stream_and_panel = div()
            .flex()
            .flex_1()
            .child(stream)
            .child(self.right_divider.clone())
            .child(right_panel);
        let right_column = div()
            .flex()
            .flex_col()
            .flex_1()
            .child(stream_and_panel)
            .child(self.input.clone());

        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .id("lengsuo-app-shell")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key(event, window, cx);
            }))
            .child(Self::title_bar(theme))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(sidebar)
                    .child(self.left_divider.clone())
                    .child(right_column),
            )
            // 底部终端抽屉（默认收起 ⇒ 只占一条标题条）。
            //
            // ⚠️ **排在命令面板之前**：命令面板是整屏浮层（`absolute`），
            // 画在**后**的子元素会盖在它上面 ⇒ 抽屉必须排在前面，
            // 命令面板才真的「浮在最上层」（与上一行注释的意图一致）。
            .child(self.terminal_drawer.clone())
            // 命令面板浮在最上层；关闭时它自己渲染为空元素。
            .child(self.palette.clone())
    }
}
