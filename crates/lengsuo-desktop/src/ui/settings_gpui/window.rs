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
use crate::state::hotkeys::{
    Binding, HotkeyRegistry, Key as HotkeyKey, Modifiers as HotkeyModifiers,
};
use crate::state::org::OrgListState;
use crate::state::org_perm::CapabilityFlags;
use crate::ui::gpui_app::AppShell;
use crate::ui::palette_gpui::hotkey_from_keystroke;
use crate::ui::settings_gpui::view::{HotkeyPanel, PanelId, SettingsView, ThemeChoice};

use super::data_panel;
use super::help_panel;
use super::hotkey_panel;
use super::hotkey_panel::EditState;
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
    /// 本仓的真实快捷键绑定（**可改**）：「个性化」面板的数据源 + 改键裁决对象。
    ///
    /// ⛔ 改键**只在本窗口内存里**：不落盘（持久化不在本卡范围）。
    registry: HotkeyRegistry,
    /// 改键状态（等键 / 冲突 / 失败）。
    ///
    /// ⚠️ 类型是接线层的投影 [`EditState`]：描述层的 `HotkeyEdit` 住在
    /// `view` 的私有子模块里 ⇒ 本层**不可命名**（见 `hotkey_panel.rs`）。
    edit: EditState,
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
    /// 宿主主界面：改键成功后把新注册表**写回**它 ⇒ 主界面立刻用新键。
    ///
    /// ⚠️ 弱引用：宿主已销毁（设置窗口后开、主界面先关）⇒ 写回自动跳过（不 panic）。
    host: Option<WeakEntity<AppShell>>,
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
            registry: default_registry(),
            edit: EditState::default(),
            data: EnvGroupState::new(),
            org: OrgListState::new(),
            focus: cx.focus_handle(),
            host: None,
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
            registry: registry_from(bindings),
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

    /// 带环境分组状态、快捷键绑定**与宿主**构造（宿主用）。
    ///
    /// ⛔ **为什么不能链式调用** `with_data(..).with_bindings(..)`：上面每个
    /// `with_*` 都用 `..Self::new(cx)` 展开剩余字段 ⇒ 后一个构造器会把前一个
    /// 已经注入的字段**重置回默认值**（`data` 变回空清单、注册表变回内置表）。
    /// ⇒ 要同时带多份数据，必须走这一个构造器。
    ///
    /// `host` 是宿主主界面的弱引用：改键成功后经它把新注册表写回宿主
    /// ⇒ 主界面下一次按键就用新键（**改即生效**）。
    #[must_use]
    pub(crate) fn with_data_and_bindings(
        cx: &mut Context<Self>,
        data: EnvGroupState,
        bindings: Vec<Binding>,
        host: Option<WeakEntity<AppShell>>,
    ) -> Self {
        Self {
            data,
            registry: registry_from(bindings),
            host,
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

    /// 开始改键：进入「等下一个组合键」状态。
    ///
    /// 「等键」提示文案由描述层给（[`HotkeyPanel::edit_status_text`]），
    /// 本层不自己拼文案。
    pub(super) fn begin_edit(&mut self, command_id: &'static str, cx: &mut Context<Self>) {
        self.edit = EditState {
            target: Some(command_id),
            status: HotkeyPanel::edit_status_text(&HotkeyPanel::edit_awaiting(command_id)),
        };
        cx.notify();
    }

    /// 处理窗口级按键：改键中 ⇒ 这次按键是「新组合键」；否则 Esc 关窗。
    fn handle_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // 1. 正在改键 ⇒ 这次按键是「新组合键」。
        if let Some(target) = self.edit.target {
            let Some(pressed) = hotkey_from_keystroke(&event.keystroke) else {
                return;
            };
            // 裸 Esc ⇒ **取消改键**（不算失败），且**不关窗口**：
            // Esc 本身是 `overlay.close` 的绑定，用户在改键时按 Esc 是想退出改键。
            if pressed.key == HotkeyKey::Escape && pressed.modifiers == HotkeyModifiers::NONE {
                self.edit = EditState::default();
                cx.notify();
                return;
            }
            // 裁决：成功 / 冲突 / 失败都由描述层判（本层**不自己实现冲突检测**）。
            let decision = HotkeyPanel::edit_intent(self.registry.bindings(), target, pressed);
            if decision.is_idle() {
                // 成功 ⇒ 落表 + 写回宿主。
                if let Err(reason) = self.registry.rebind(target, pressed) {
                    eprintln!("[lengsuo-desktop] 改键落表失败：{reason}");
                    return;
                }
                self.edit = EditState::default();
                self.write_back_host(cx);
                cx.notify();
                return;
            }
            // 失败 ⇒ 说清为什么（文案由描述层给）；命令 id 与组合键落日志。
            self.edit.target = None;
            self.edit.status = HotkeyPanel::edit_status_text(&decision);
            if let Some(detail) = decision.failure_detail() {
                eprintln!("[lengsuo-desktop] 改键未生效：{detail}");
            }
            cx.notify();
            return;
        }
        // 2. 不在改键 ⇒ Esc 关闭设置窗口（窗口级快捷键，不与输入框抢）。
        if event.keystroke.key.as_str() == "escape" {
            window.remove_window();
        }
    }

    /// 把改键后的注册表**写回宿主** ⇒ 主界面下一次按键就用新键（改即生效）。
    ///
    /// ⚠️ 宿主可能已经没了（主界面先关）⇒ 弱引用升级失败就跳过（不 panic）。
    fn write_back_host(&mut self, cx: &mut Context<Self>) {
        let Some(host) = self.host.as_ref().and_then(WeakEntity::upgrade) else {
            return;
        };
        let registry = self.registry.clone();
        host.update(cx, move |shell, cx| {
            shell.registry = registry;
            cx.notify();
        });
    }
}

/// 读内置默认注册表；失败时回落到**空表**并记日志。
///
/// ⛔ 不 panic、不 unwrap：内置表万一自相冲突，界面应显示「没有可用的快捷键」
/// 并留下日志，而不是让整个设置窗口开不起来。
fn default_registry() -> HotkeyRegistry {
    match HotkeyRegistry::with_defaults() {
        Ok(registry) => registry,
        Err(reason) => {
            eprintln!("[lengsuo-desktop] 内置快捷键表不可用：{reason}");
            empty_registry()
        }
    }
}

/// 宿主给的绑定表 ⇒ 注册表。
///
/// 宿主表不可用（未知命令 / 自相冲突）⇒ 回落到**内置表**；内置表也不可用
/// ⇒ 用**空表**（面板显示「没有可用的快捷键」，fail-closed）。
fn registry_from(bindings: Vec<Binding>) -> HotkeyRegistry {
    match HotkeyRegistry::from_bindings(bindings.iter().copied()) {
        Ok(registry) => registry,
        Err(reason) => {
            eprintln!("[lengsuo-desktop] 宿主快捷键表不可用，回落为内置表：{reason}");
            default_registry()
        }
    }
}

/// 空注册表（面板会显示「没有可用的快捷键」）。
///
/// `from_bindings` 对空表**必成功**（没有绑定 ⇒ 既无冲突也无未知命令），
/// `Err` 分支不可达 ⇒ 用 `expect` 把「代码级缺陷」显式化。
fn empty_registry() -> HotkeyRegistry {
    HotkeyRegistry::from_bindings(Vec::new()).expect("空快捷键表不该失败")
}

impl Render for SettingsGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 先把主题**克隆**一份：后面 helper 要拿 `&mut Context`，
        // 而 `cx.theme()` 借的是同一个 `cx` ⇒ 借用会打架。
        let theme = cx.theme().clone();

        // 左栏：15 个面板导航项。
        let sidebar = nav::render_sidebar(&self.view, &theme, cx);

        // 右栏：外观面板给三档主题；安全中心给能力位只读清单；
        // 个性化给快捷键清单（**可改键**）；账号给成员只读清单；
        // 帮助与反馈给版本 / 快捷键 / 命令表只读清单；其余面板给占位文案。
        let content: AnyElement = match self.view.selected {
            PanelId::Account => org_panel::render_org_panel(&self.org, &theme, cx),
            PanelId::Appearance => theme_panel::render_theme_picker(&self.view, &theme, cx),
            PanelId::Security => security_panel::render_security_panel(&self.flags, &theme, cx),
            PanelId::Personalization => {
                hotkey_panel::render_hotkey_panel(self.registry.bindings(), &self.edit, &theme, cx)
            }
            PanelId::SystemPermissions => {
                system_perm_panel::render_system_perm_panel(&self.flags, &theme, cx)
            }
            PanelId::DataManagement => data_panel::render_data_panel(&self.data, &theme, cx),
            PanelId::Help => help_panel::render_help_panel(self.registry.bindings(), &theme, cx),
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
                |this: &mut SettingsGpui,
                 event: &KeyDownEvent,
                 window: &mut Window,
                 cx: &mut Context<SettingsGpui>| {
                    // 改键中 ⇒ 捕获这次按键；否则 Esc 关窗。
                    this.handle_key(event, window, cx);
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

#[cfg(test)]
mod tests {
    // 显式列出符号（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进来并遮蔽标准库的 `#[test]`。
    use crate::state::env_group::EnvGroupState;
    use serde_json::json;

    /// 注入设置窗口的环境状态必须**带着宿主给的条目**（宿主 ⇄ 「数据管理」面板
    /// 之间的数据契约）。
    ///
    /// ⛔ `SettingsGpui::with_data` 需要 `&mut Context<Self>` ⇒ **无窗口环境
    /// 造不出视图** ⇒ 这里只断言**状态层**的注入语义（条目的字段契约）；
    /// 视图层的接线靠**编译 + 真机**验证（见 `docs/settings-window-status.md`）。
    #[test]
    fn injected_env_state_keeps_its_entries() {
        let mut data = EnvGroupState::new();
        data.apply_entries(vec![json!({"id": "e1", "name": "构建机 A"})]);

        assert_eq!(data.raw_count(), 1, "注入的状态必须带着宿主给的条目");
        let entry = &data.entries()[0];
        assert_eq!(entry.id, "e1", "面板按 id 标识条目");
        assert_eq!(
            entry.name.as_deref(),
            Some("构建机 A"),
            "面板按 name 展示条目"
        );
    }
}
