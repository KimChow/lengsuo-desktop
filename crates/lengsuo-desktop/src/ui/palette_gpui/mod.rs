//! 命令面板的 GPUI 接线层：把 [`crate::ui::command_palette`] 的渲染描述接到元素树。
//!
//! 分层约定（与 `env_gpui.rs` 一致）：
//! - **描述层**（`command_palette`）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（本目录）：`feature = "ui"` 隔离，只做「描述字段 → 样式 / 事件」的映射。
//!
//! 文件拆分（每个文件单一职责，均 ≤ 400 行）：
//! - `mod.rs`（本文件）：面板本体 + 打开 / 关闭 / 同步 / 按键 + `Render`。
//! - `keys.rs`：平台按键 → [`Hotkey`] / [`PaletteKey`] 的转换 + 其测试。
//! - `layout.rs`：遮罩与尺寸常量、底部键盘提示。
//! - `input_box.rs`：搜索框（`Textarea` 绑定与双向同步判据）。
//! - `search_rows.rs`：多来源搜索结果（分组标题 + 条目）。
//!
//! 硬约定：
//! - 面板关闭时**什么都不渲染**（[`command_palette::render`] 返回 `None`）。
//! - 条目**按 id 派发**：回执拿到的是 [`PaletteAction::Execute`] 带出的 id；
//!   本卡**不真正执行**条目，只到回调为止。
//! - 快捷键复用既有的 [`HotkeyRegistry`]，**不新造注册表**；用户改键后依旧生效。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
//!
//! 搜索框编辑：已接 [`Textarea`]，双向绑定与焦点路径见 `input_box.rs` 与
//! `docs/palette-gpui-status.md`。**Windows 中文 IME 可用性未验证**。
#![cfg(feature = "ui")]

/// 条目派发回执的类型：拿到**条目 id** 与面板自己的 [`Context`]。
///
/// 带 `Context` 是因为回执可能要开新窗口，而 `Context` 实现了
/// `DerefMut<Target = App>` ⇒ `&mut **cx` 即 `&mut App`。
///
/// id 传 `&str`：非命令来源（技能 / MCP / 文件 / 会话）的 id 在运行时才有，
/// 不是 `'static`；回执只读不存 ⇒ 传借用即可。
pub type ExecuteCallback<T> = Box<dyn Fn(&str, &mut Context<T>) + 'static>;

mod input_box;
mod keys;
mod layout;
mod search_rows;
#[cfg(test)]
mod tests;

use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::hotkeys::HotkeyRegistry;
use crate::state::palette_state::PaletteState;
use crate::ui::command_palette::{self, PaletteAction};

use self::input_box::{PanelSync, PanelSyncState, PLACEHOLDER};
use self::layout::{mask_color, render_footer, PANEL_WIDTH};

// `gpui_app.rs` 等既有调用方用的路径保持不变。
pub use self::keys::{hotkey_from_keystroke, palette_key_from_keystroke};

/// 命令面板的 GPUI 视图：遮罩 + 搜索框 + 命令列表。
///
/// 持有面板状态本身（而不是只持描述），因为键盘事件要**写回**状态。
pub struct CommandPaletteGpui {
    /// 面板状态（描述层的唯一真源）。
    state: PaletteState,
    /// 快捷键注册表：复用既有注册表，不新造。
    registry: HotkeyRegistry,
    /// 面板自身的焦点句柄：宿主打开面板时先交给这里。
    focus: FocusHandle,
    /// 搜索框编辑器状态：真实文本编辑（含 IME 组合文本）唯一的落地处。
    ///
    /// 首次渲染时创建（构造时拿不到窗口），之后一直复用。
    textarea: Option<Entity<TextareaState>>,
    /// 双向同步账本：上一轮两边的值，用来判断哪一侧真的变了。
    sync: PanelSyncState,
    /// 打开面板时下一个按键留给宿主（见 [`Self::release_next_key`]）。
    release_next_key: bool,
    /// 命令执行回调：收到**命令 id**，本卡只到回调为止。
    on_execute: ExecuteCallback<Self>,
    /// 编辑器事件订阅（经由 `Self::ensure_textarea` 的 `cx.subscribe` 创建）：
    /// 必须随视图存活，drop 即退订。
    _subscription: Option<Subscription>,
}

impl CommandPaletteGpui {
    /// 用内置快捷键表构造视图。
    ///
    /// **签名保持不变**（`gpui_app.rs` 以 `cx.new(CommandPaletteGpui::new)` 调用，不在本卡
    /// touches 内）。`Context` 上没有公开的取窗口方法（`update` 是私有的），所以
    /// [`TextareaState`] **延迟创建**：第一次渲染时窗口可用，在那里建（见
    /// [`Self::ensure_textarea`]）。选择理由写在 `docs/palette-gpui-status.md`。
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::with_registry(default_registry(), cx)
    }

    /// 用指定快捷键表构造视图（用户改键后由宿主传入新的注册表）。
    ///
    /// 编辑器在此后第一次渲染时才创建。
    #[must_use]
    pub fn with_registry(registry: HotkeyRegistry, cx: &mut Context<Self>) -> Self {
        Self {
            state: PaletteState::new(),
            registry,
            focus: cx.focus_handle(),
            textarea: None,
            sync: PanelSyncState::default(),
            release_next_key: false,
            on_execute: Box::new(|_, _| {}),
            _subscription: None,
        }
    }

    /// 首次渲染时按需创建编辑器，并订阅它的事件。
    ///
    /// 订阅句柄存进 [`Self::_subscription`]（与 `input_area.rs` 的 `InputAreaGpui`
    /// 同一写法）；drop 即退订。
    fn ensure_textarea(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.textarea.is_some() {
            return;
        }
        let textarea = cx.new(|cx| {
            // 面板关键词是单行语义：Enter 不换行，而是提交（PressEnter）。
            TextareaState::new(window, cx)
                .placeholder(PLACEHOLDER)
                .submit_on_enter(true)
        });
        self._subscription = Some(cx.subscribe(&textarea, |this, _emitter, event, cx| {
            this.on_textarea_event(event, cx);
        }));
        self.textarea = Some(textarea);
    }

    /// 搜索框编辑器（尚未渲染过时为 `None`）。
    #[must_use]
    pub fn textarea(&self) -> Option<&Entity<TextareaState>> {
        self.textarea.as_ref()
    }

    /// 设置条目执行回调。回调收到的参数是**条目 id**，不是标题。
    ///
    /// 第二个参数是本面板的 [`Context`]：回执要开新窗口就需要它
    /// （`Context` 实现了 `DerefMut<Target = App>` ⇒ `&mut **cx` 即 `&mut App`）。
    #[must_use]
    pub fn on_execute(mut self, callback: impl Fn(&str, &mut Context<Self>) + 'static) -> Self {
        self.on_execute = Box::new(callback);
        self
    }

    /// 设置条目执行回调（**原地改**，不需要消费 `self`）。
    ///
    /// 与 [`Self::on_execute`] 的区别：那个是构造期用的链式版本（消费 `self`），
    /// 这个给「先构造好、之后再挂回调」的宿主用。
    pub fn set_on_execute(&mut self, callback: impl Fn(&str, &mut Context<Self>) + 'static) {
        self.on_execute = Box::new(callback);
    }

    /// 面板状态（只读）。
    #[must_use]
    pub fn state(&self) -> &PaletteState {
        &self.state
    }

    /// 面板状态（可写）：宿主可在此打开 / 关闭面板。
    ///
    /// 写完后请调用 [`Self::sync_search_box`] 让搜索框跟上；经本类型的
    /// [`Self::open`] / [`Self::close`] 改状态则已自动同步。
    pub fn state_mut(&mut self) -> &mut PaletteState {
        &mut self.state
    }

    /// 面板的焦点句柄：宿主需要自己接管焦点时用。
    ///
    /// 面板内部的键盘事件挂在**这个**句柄上（一路 bubbling 到编辑器），
    /// 所以焦点停在这里时方向键 / 回车 / Esc 照常生效。
    #[must_use]
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// 打开面板：清空上次关键词，把键盘焦点交给面板，**并让下一个按键留给宿主**。
    ///
    /// 为什么让位：`gpui_app.rs` 的根视图 `handle_key` 先对 `Ctrl+K` 调
    /// `state_mut().open()`，再把焦点交给本面板；同一个按键随后也到达这里，
    /// 若照常处理会因 `overlay.close`（`Ctrl+W` 等）或 `Char` 把面板刚清空的关键词
    /// 又写回去。让位**只发生一次**，之后的按键一律正常处理。
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.open();
        window.focus(&self.focus, cx);
        self.release_next_key = true;
        self.sync_search_box(window, cx);
        cx.notify();
    }

    /// 关闭面板。下一次打开时搜索框与关键词一起清空。
    ///
    /// 关闭时**不**改写搜索框：描述层约定关键词保留到下次打开，
    /// 若此刻清空，下次按 Esc 关面板后关键词与列表会短暂对不上。
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.state.close();
        cx.notify();
    }

    /// 把面板关键词回写到搜索框（仅当两边不一致时）。
    ///
    /// `TextareaState::set_value` 不发 `Change` 事件，所以不会回环。
    pub fn sync_search_box(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(textarea) = self.textarea.clone() else {
            // 还没渲染过：编辑器不存在，等首次渲染时按关键词初始化。
            return;
        };
        let current = textarea.read(cx).value().to_string();
        if let Some(value) = self.sync.pull_into_search_box(&self.state, &current) {
            textarea.update(cx, |textarea, textarea_cx| {
                textarea.set_value(value, window, textarea_cx);
            });
        }
    }

    /// 下一个到达本面板的按键不处理（由 [`Self::open`] 置位）。
    pub fn release_next_key(&mut self) {
        self.release_next_key = true;
    }

    /// 处理一次**全局**快捷键（宿主把平台按键转换成 [`Hotkey`] 后调用）。
    ///
    /// 只处理 `palette.open` 与 `overlay.close`；其余命令返回
    /// [`PaletteAction::Ignored`]，由宿主继续按 id 派发。
    pub fn handle_hotkey(
        &mut self,
        event: crate::state::hotkeys::Hotkey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PaletteAction {
        let action = command_palette::handle_hotkey(&mut self.state, &self.registry, event);
        match action {
            PaletteAction::Open => {
                window.focus(&self.focus, cx);
                self.release_next_key = true;
                self.sync_search_box(window, cx);
                cx.notify();
            }
            PaletteAction::Close => cx.notify(),
            _ => {}
        }
        action
    }

    /// 处理一次面板内的按键：先问快捷键注册表，再按面板语义处理。
    ///
    /// 文本字符与退格由编辑器消费（编辑器会发 `InputEvent::Change`），这里不重复写入，
    /// 避免一个字被输入两次。
    pub fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.release_next_key {
            self.release_next_key = false;
            return;
        }

        // 快捷键优先：`palette.open` / `overlay.close` 归本面板，其余交给宿主。
        if let Some(hotkey) = hotkey_from_keystroke(&event.keystroke) {
            match command_palette::handle_hotkey(&mut self.state, &self.registry, hotkey) {
                PaletteAction::Open => {
                    window.focus(&self.focus, cx);
                    self.sync_search_box(window, cx);
                    cx.notify();
                    return;
                }
                PaletteAction::Close => {
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        // 面板语义：过滤、选择、执行、关闭。文本编辑留给编辑器。
        let Some(key) = palette_key_from_keystroke(&event.keystroke) else {
            return;
        };
        match command_palette::handle_key(&mut self.state, key) {
            // 按 id 派发：本卡**不执行**条目，只把 id 交给回调。
            PaletteAction::Execute(id) => {
                (self.on_execute)(&id, cx);
                cx.notify();
            }
            PaletteAction::Redraw | PaletteAction::Close | PaletteAction::Open => cx.notify(),
            PaletteAction::Ignored => {}
        }
    }

    /// 编辑器事件：内容变化 ⇒ 写回关键词；获得焦点 ⇒ 重置让位标记。
    fn on_textarea_event(&mut self, event: &InputEvent, cx: &mut Context<Self>) {
        match event {
            InputEvent::Change => {
                let Some(textarea) = self.textarea.clone() else {
                    return;
                };
                let text = textarea.read(cx).value().to_string();
                if self.sync.accept_search_box(&mut self.state, &text) {
                    cx.notify();
                }
            }
            InputEvent::Focus => self.release_next_key = false,
            InputEvent::PressEnter { .. } | InputEvent::Blur => {}
        }
    }
}

/// 内置快捷键表。
///
/// 内置表若自相冲突（不该发生），退化为**空表**而不是 panic：
/// 打不开面板好过整个界面崩掉。
fn default_registry() -> HotkeyRegistry {
    HotkeyRegistry::with_defaults().unwrap_or_else(|_| {
        HotkeyRegistry::from_bindings([]).expect("空绑定表必定既无未知命令也无冲突")
    })
}

impl Render for CommandPaletteGpui {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // 面板关闭：什么都不渲染。
        let Some(view) = command_palette::render(&self.state) else {
            return div().into_any_element();
        };

        // 首次绘制：建编辑器并把关键词写进去；之后每次绘制把两边对齐
        // （只在不一致时写，`set_value` 不回环）。
        self.ensure_textarea(window, cx);
        self.sync_search_box(window, cx);

        // 结果列表：按来源分组（命令 / 技能 / MCP / 文件 / 会话）。
        let list = search_rows::render_results(&view, &theme);

        // 搜索框（编辑器已在上面建好；理论上不会是 `None`）。
        let search_box: AnyElement = match self.textarea.as_ref() {
            Some(textarea) => {
                input_box::render_search_box(textarea, &view, &theme).into_any_element()
            }
            None => div().into_any_element(),
        };

        // 面板本体
        let panel = div()
            .w(px(PANEL_WIDTH))
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .shadow_lg()
            .child(search_box)
            .child(list)
            .child(render_footer(&view, &theme));

        // 半透明遮罩层：压暗背景，把注意力集中到面板上。
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(mask_color(&theme))
            .id("lengsuo-command-palette")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                view.handle_key_down(event, window, cx);
            }))
            .child(panel)
            .into_any_element()
    }
}
