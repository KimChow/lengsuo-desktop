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

use crate::state::env_group::EnvGroupState;
use crate::state::hotkeys::{Binding, HotkeyRegistry};
use crate::state::org::OrgListState;
use crate::state::org_perm::CapabilityFlags;
use crate::ui::settings_gpui::view::{PanelId, SettingsView, ThemeChoice};

use super::data_panel;
use super::hotkey_panel;
use super::nav;
use super::org_panel;
use super::security_panel;
use super::system_perm_panel;
use super::theme_panel;

/// 设置窗口视图。
///
/// 持有描述层状态（而不是只持描述），因为交互要**写回**状态。
pub struct SettingsGpui {
    /// 描述层状态：面板清单、当前选中面板、主题取值。
    ///
    /// `pub(super)`：同模块的 `nav.rs` / `theme_panel.rs` 要读它写回选中态。
    pub(super) view: SettingsView,
    /// 服务端下发的能力标记（**只读快照**）：「安全中心」面板的数据源。
    ///
    /// ⛔ 本字段只承载服务端结果，界面**不做授权推理**。
    flags: CapabilityFlags,
    /// 本仓的真实快捷键绑定（**只读**）：「个性化」面板的数据源。
    ///
    /// ⛔ 本面板**只读**：改键需要持久化 + 冲突裁决，不在本卡范围。
    bindings: Vec<Binding>,
    /// 浏览器环境分组视图状态（只读快照）：「数据管理」面板的数据源。
    ///
    /// ⛔ 本字段只承载快照，界面**不做环境管理**（增删改是业务语义）。
    data: EnvGroupState,
    /// 组织域成员清单（**只读快照**）：「账号」面板的数据源。
    ///
    /// ⛔ 本字段只承载服务端结果，界面**不做成员管理**（增删改是业务语义）。
    org: OrgListState,
    /// 窗口自身的焦点句柄：Esc 关闭设置窗口靠它。
    focus: FocusHandle,
}

impl SettingsGpui {
    /// 构造设置窗口视图。
    ///
    /// **初始主题取自偏好快照**（`theme.json`）：首帧已经水合成暗色，
    /// 若设置窗口还显示「跟随系统」被选中，用户看到的会自相矛盾；
    /// 更要命的是点「跟随系统」会被 `set_theme` 判成「没变化」⇒ 直接返回，
    /// 用户在暗色下**再也点不回亮色**。
    ///
    /// 能力标记用**空集合**初始化 ⇒ 「安全中心」面板显示「服务端未下发能力标记」。
    /// 宿主要带真实标记 ⇒ 用 [`Self::with_flags`]。
    ///
    /// 快捷键绑定用**内置默认表**初始化 ⇒ 「个性化」面板直接有内容可看。
    /// 宿主要带自定义表 ⇒ 用 [`Self::with_bindings`]。
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        let view = SettingsView {
            theme: crate::ui::gpui_app::prefs_file::read_theme().0,
            ..SettingsView::default()
        };
        Self {
            view,
            flags: CapabilityFlags::default(),
            bindings: default_bindings(),
            data: EnvGroupState::new(),
            org: OrgListState::new(),
            focus: cx.focus_handle(),
        }
    }

    /// 带能力标记构造（宿主/测试用；默认构造 ⇒ 空标记 ⇒ 面板显示「服务端未下发」）。
    #[must_use]
    pub fn with_flags(cx: &mut Context<Self>, flags: CapabilityFlags) -> Self {
        Self {
            flags,
            ..Self::new(cx)
        }
    }

    /// 带指定绑定构造（宿主/测试用；默认构造 ⇒ 内置默认绑定表）。
    #[must_use]
    pub fn with_bindings(cx: &mut Context<Self>, bindings: Vec<Binding>) -> Self {
        Self {
            bindings,
            ..Self::new(cx)
        }
    }

    /// 带环境分组状态构造（宿主/测试用；默认构造 ⇒ 空状态）。
    #[must_use]
    pub fn with_data(cx: &mut Context<Self>, data: EnvGroupState) -> Self {
        Self {
            data,
            ..Self::new(cx)
        }
    }

    /// 带组织域成员清单构造（宿主/测试用；默认构造 ⇒ 空状态）。
    ///
    /// ⛔ 清单是**只读快照**：本卡只展示服务端下发什么，不做成员增删改。
    #[must_use]
    pub fn with_org(cx: &mut Context<Self>, org: OrgListState) -> Self {
        Self {
            org,
            ..Self::new(cx)
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
            //
            // ⚠️ 但**仍要落盘**：设置窗口的主题可能来自偏好快照（见 `new`），
            // 而快照可能与落盘内容不一致（例如别的进程改过、或文件被手工改坏）。
            // 这里无条件写一次，保证「磁盘上的偏好 == 用户看到的选择」。
            if let Err(reason) = crate::ui::gpui_app::prefs_file::write_theme(choice) {
                eprintln!("[lengsuo-desktop] 主题偏好未落盘：{reason}");
            }
            return;
        }
        let mode = match choice {
            ThemeChoice::System => ThemeMode::from(window.appearance()),
            ThemeChoice::Light => ThemeMode::Light,
            ThemeChoice::Dark => ThemeMode::Dark,
        };
        // `Theme::change` 内部会刷新所有窗口 ⇒ 改完立刻整窗生效。
        GpuiTheme::change(mode, None, cx);
        // 落盘用户选择 ⇒ 下次启动首帧就是它（A9）。
        if let Err(reason) = crate::ui::gpui_app::prefs_file::write_theme(choice) {
            eprintln!("[lengsuo-desktop] 主题偏好未落盘：{reason}");
        }
        cx.notify();
    }
}

/// 读内置默认绑定表；失败时回落到**空表**并记日志。
///
/// ⛔ 不 panic、不 unwrap：内置表万一自相冲突，界面应显示「没有可用的快捷键」
/// 并留下日志，而不是让整个设置窗口开不起来。
fn default_bindings() -> Vec<Binding> {
    match HotkeyRegistry::with_defaults() {
        Ok(registry) => registry.bindings().to_vec(),
        Err(reason) => {
            eprintln!("[lengsuo-desktop] 内置快捷键表不可用：{reason}");
            Vec::new()
        }
    }
}

impl Render for SettingsGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 先把主题**克隆**一份：后面 helper 要拿 `&mut Context`，
        // 而 `cx.theme()` 借的是同一个 `cx` ⇒ 借用会打架。
        let theme = cx.theme().clone();

        // 左栏：15 个面板导航项。
        let sidebar = nav::render_sidebar(&self.view, &theme, cx);

        // 右栏：外观面板给三档主题；安全中心给能力位只读清单；
        // 个性化给快捷键只读清单；账号给成员只读清单；其余面板给占位文案。
        let content: AnyElement = match self.view.selected {
            PanelId::Account => org_panel::render_org_panel(&self.org, &theme, cx),
            PanelId::Appearance => theme_panel::render_theme_picker(&self.view, &theme, cx),
            PanelId::Security => security_panel::render_security_panel(&self.flags, &theme, cx),
            PanelId::Personalization => {
                hotkey_panel::render_hotkey_panel(&self.bindings, &theme, cx)
            }
            PanelId::SystemPermissions => {
                system_perm_panel::render_system_perm_panel(&self.flags, &theme, cx)
            }
            PanelId::DataManagement => data_panel::render_data_panel(&self.data, &theme, cx),
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
                    // 内容区自己滚动：「安全中心」有 9 行能力位，
                    // 窗口不够高时不能把行裁掉。
                    .child(
                        div()
                            .id("settings-content")
                            .flex_1()
                            .p_4()
                            .overflow_y_scroll()
                            .child(content),
                    ),
            )
            .into_any_element()
    }
}
