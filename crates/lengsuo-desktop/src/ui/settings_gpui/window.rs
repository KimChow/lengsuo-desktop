//! 设置窗口视图本体：左栏导航 + 右栏内容 + 改主题即时生效。
//!
//! 本文件是接线层，只做「描述层字段 → 元素」的映射，**不实现业务语义**。
//!
//! 主题切换的机制：`gpui-component` 的全局主题被 `Theme::change` 改写，
//! 该函数内部会 `refresh_windows()`，所以**点完立刻整窗重绘** ——
//! 这就是「改即生效，无保存按钮」的落点。
//!
//! ⚠️ `gpui_kit::component::theme::Theme`（组件层）与本仓
//! `crate::theme::tokens::Theme`（语义层枚举）**同名不同物**：
//! 本文件只 `use` 组件层的并改名为 `GpuiTheme`，语义层的映射在
//! [`crate::ui::settings_gpui::view::ThemeChoice::semantic`] 里做。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

// 组件层的 `Theme` 与本仓语义层同名 ⇒ 改名引入，不混进 `use` 名单。
use gpui_kit::component::theme::{Theme as GpuiTheme, ThemeMode};

use crate::ui::settings_gpui::view::{PanelId, SettingsView, ThemeChoice};

use super::nav;
use super::theme_panel;

/// 设置窗口视图。
///
/// 持有描述层状态（而不是只持描述），因为交互要**写回**状态。
pub struct SettingsGpui {
    /// 描述层状态：面板清单、当前选中面板、主题取值。
    ///
    /// `pub(super)`：同模块的 `nav.rs` / `theme_panel.rs` 要读它写回选中态。
    pub(super) view: SettingsView,
    /// 窗口自身的焦点句柄：Esc 关闭设置窗口靠它。
    focus: FocusHandle,
}

impl SettingsGpui {
    /// 构造设置窗口视图。
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            view: SettingsView::default(),
            focus: cx.focus_handle(),
        }
    }

    /// 当前视图（只读）：宿主与测试读它。
    #[must_use]
    pub fn view(&self) -> &SettingsView {
        &self.view
    }

    /// 窗口焦点句柄（宿主把键盘焦点交给它）。
    #[must_use]
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// 切换主题：**改即生效**，无保存按钮。
    ///
    /// - `System`（跟随系统）取**当前窗口的系统外观**（`window.appearance()`），
    ///   落成组件层的 `ThemeMode::Light` / `ThemeMode::Dark`。
    /// - 语义层（[`crate::theme::tokens::Theme`]）只记录**用户的选择**，
    ///   它的亮/暗令牌尚未被 GPUI 渲染层消费 ⇒ 本卡只保证「入口 + 内存态 + 即时生效」。
    pub(super) fn set_theme(
        &mut self,
        choice: ThemeChoice,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.view.set_theme(choice) {
            // 取值没变：不必重绘。
            return;
        }
        let mode = match choice {
            ThemeChoice::System => ThemeMode::from(window.appearance()),
            ThemeChoice::Light => ThemeMode::Light,
            ThemeChoice::Dark => ThemeMode::Dark,
        };
        // `Theme::change` 内部会刷新所有窗口 ⇒ 改完立刻整窗生效。
        GpuiTheme::change(mode, None, cx);
        cx.notify();
    }
}

impl Render for SettingsGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 先把主题**克隆**一份：后面 helper 要拿 `&mut Context`，
        // 而 `cx.theme()` 借的是同一个 `cx` ⇒ 借用会打架。
        let theme = cx.theme().clone();

        // 左栏：15 个面板导航项。
        let sidebar = nav::render_sidebar(&self.view, &theme, cx);

        // 右栏：外观面板给三档主题；其余面板给占位文案。
        let content: AnyElement = match self.view.selected {
            PanelId::Appearance => theme_panel::render_theme_picker(&self.view, &theme, cx),
            _ => div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(format!(
                    "「{}」面板待实现：本卡只做导航与占位，功能见 docs/settings-window-status.md。",
                    self.view.current_title()
                ))
                .into_any_element(),
        };

        // 顶栏：当前面板名 + 关闭按钮。
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .child(div().text_sm().font_bold().child(self.view.current_title()))
            .child(
                div()
                    .id("settings-close")
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .child("关闭")
                    .on_click(cx.listener(
                        |_this: &mut SettingsGpui,
                         _event: &ClickEvent,
                         window: &mut Window,
                         _cx: &mut Context<SettingsGpui>| {
                            window.remove_window();
                        },
                    )),
            );

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.foreground)
            .id("lengsuo-settings-window")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(
                |_this: &mut SettingsGpui,
                 event: &KeyDownEvent,
                 window: &mut Window,
                 _cx: &mut Context<SettingsGpui>| {
                    // Esc 关闭设置窗口（窗口级快捷键，不与输入框抢）。
                    if event.keystroke.key.as_str() == "escape" {
                        window.remove_window();
                    }
                },
            ))
            .child(header)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .child(sidebar)
                    .child(div().flex_1().p_4().child(content)),
            )
            .into_any_element()
    }
}
