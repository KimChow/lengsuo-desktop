//! 命令面板的 GPUI 接线层：把 [`crate::ui::command_palette`] 的渲染描述接到元素树。
//!
//! 分层约定（与 `env_gpui.rs` 一致）：
//! - **描述层**（`command_palette`）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（本文件）：`feature = "ui"` 隔离，只做「描述字段 → 样式 / 事件」的映射。
//!
//! 硬约定：
//! - 面板关闭时**什么都不渲染**（[`command_palette::render`] 返回 `None`）。
//! - 命令**按 id 派发**：回执拿到的是 [`PaletteAction::Execute`] 带出的 id；
//!   本卡**不真正执行**命令，只到回调为止。
//! - 快捷键复用既有的 [`HotkeyRegistry`]，**不新造注册表**；用户改键后依旧生效。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
//!
//! 已知缺口（见 `docs/palette-gpui-status.md`）：
//! - 搜索框是**只读展示**，真实文本编辑（含中文 IME）未接 [`Textarea`]。
//! - 全局快捷键的**捕获**由宿主负责（本卡只提供转换与分发入口）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::hotkeys::{
    Hotkey, HotkeyRegistry, Key as HotkeyKey, Modifiers as HotkeyModifiers,
};
use crate::state::palette_state::PaletteState;
use crate::ui::command_palette::{self, PaletteAction, PaletteKey, PaletteRow};

/// 遮罩的淡化系数：把主题背景色的 alpha 减半，得到半透明遮罩层。
const MASK_FADE: f32 = 0.5;

/// 面板宽度（像素）。
const PANEL_WIDTH: f32 = 560.;
/// 命令列表最大高度（像素）。
const LIST_MAX_HEIGHT: f32 = 320.;

/// 命令面板的 GPUI 视图：遮罩 + 搜索框 + 命令列表。
///
/// 持有面板状态本身（而不是只持描述），因为键盘事件要**写回**状态。
pub struct CommandPaletteGpui {
    /// 面板状态（描述层的唯一真源）。
    state: PaletteState,
    /// 快捷键注册表：复用既有注册表，不新造。
    registry: HotkeyRegistry,
    /// 面板自身的焦点句柄：打开时把键盘焦点交到这里。
    focus: FocusHandle,
    /// 命令执行回调：收到**命令 id**，本卡只到回调为止。
    on_execute: Box<dyn Fn(&'static str) + 'static>,
}

impl CommandPaletteGpui {
    /// 用内置快捷键表构造视图。
    #[must_use]
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self::with_registry(default_registry(), cx)
    }

    /// 用指定快捷键表构造视图（用户改键后由宿主传入新的注册表）。
    #[must_use]
    pub fn with_registry(registry: HotkeyRegistry, cx: &mut Context<Self>) -> Self {
        Self {
            state: PaletteState::new(),
            registry,
            focus: cx.focus_handle(),
            on_execute: Box::new(|_| {}),
        }
    }

    /// 设置命令执行回调。回调收到的参数是**命令 id**，不是标题。
    #[must_use]
    pub fn on_execute(mut self, callback: impl Fn(&'static str) + 'static) -> Self {
        self.on_execute = Box::new(callback);
        self
    }

    /// 面板状态（只读）。
    #[must_use]
    pub fn state(&self) -> &PaletteState {
        &self.state
    }

    /// 面板状态（可写）：宿主可在此打开 / 关闭面板。
    pub fn state_mut(&mut self) -> &mut PaletteState {
        &mut self.state
    }

    /// 面板的焦点句柄：宿主需要自己接管焦点时用。
    #[must_use]
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    /// 打开面板：清空上次关键词，并把键盘焦点交给面板。
    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.open();
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// 关闭面板。
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.state.close();
        cx.notify();
    }

    /// 处理一次**全局**快捷键（宿主把平台按键转换成 [`Hotkey`] 后调用）。
    ///
    /// 只处理 `palette.open` 与 `overlay.close`；其余命令返回
    /// [`PaletteAction::Ignored`]，由宿主继续按 id 派发。
    pub fn handle_hotkey(
        &mut self,
        event: Hotkey,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> PaletteAction {
        let action = command_palette::handle_hotkey(&mut self.state, &self.registry, event);
        match action {
            PaletteAction::Open => {
                window.focus(&self.focus, cx);
                cx.notify();
            }
            PaletteAction::Close => cx.notify(),
            _ => {}
        }
        action
    }

    /// 处理一次面板内的按键：先问快捷键注册表，再按面板语义处理。
    pub fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 快捷键优先：`palette.open` / `overlay.close` 归本面板，其余交给宿主。
        if let Some(hotkey) = hotkey_from_keystroke(&event.keystroke) {
            match command_palette::handle_hotkey(&mut self.state, &self.registry, hotkey) {
                PaletteAction::Open => {
                    window.focus(&self.focus, cx);
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

        // 面板语义：过滤、选择、执行、关闭。
        let Some(key) = palette_key_from_keystroke(&event.keystroke) else {
            return;
        };
        match command_palette::handle_key(&mut self.state, key) {
            // 按 id 派发：本卡**不执行**命令，只把 id 交给回调。
            PaletteAction::Execute(id) => {
                (self.on_execute)(id);
                cx.notify();
            }
            PaletteAction::Redraw | PaletteAction::Close | PaletteAction::Open => cx.notify(),
            PaletteAction::Ignored => {}
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

/// 把平台按键转换成既有的 [`Hotkey`] 形状。
///
/// 转换规则与 [`Hotkey::parse`] 一致：ASCII 字母统一小写，修饰键**完全相等**才匹配。
#[must_use]
pub fn hotkey_from_keystroke(keystroke: &Keystroke) -> Option<Hotkey> {
    let modifiers = HotkeyModifiers {
        ctrl: keystroke.modifiers.control,
        shift: keystroke.modifiers.shift,
        alt: keystroke.modifiers.alt,
        meta: keystroke.modifiers.platform,
    };
    let key = match keystroke.key.as_str() {
        "escape" => HotkeyKey::Escape,
        "enter" => HotkeyKey::Enter,
        "backspace" => HotkeyKey::Backspace,
        "tab" => HotkeyKey::Tab,
        "space" => HotkeyKey::Space,
        "up" => HotkeyKey::Up,
        "down" => HotkeyKey::Down,
        "left" => HotkeyKey::Left,
        "right" => HotkeyKey::Right,
        other => {
            let mut chars = other.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            HotkeyKey::Character(if c.is_ascii_alphabetic() {
                c.to_ascii_lowercase()
            } else {
                c
            })
        }
    };
    Some(Hotkey::new(modifiers, key))
}

/// 把平台按键转换成面板语义按键。
///
/// 带 `Ctrl` / `Alt` / `Meta` 的组合键**不是**文本输入（否则 `Ctrl+S` 会把
/// `s` 打进搜索框）；命名键（方向键、回车、Esc、退格）照常映射。
#[must_use]
pub fn palette_key_from_keystroke(keystroke: &Keystroke) -> Option<PaletteKey> {
    match keystroke.key.as_str() {
        "up" => return Some(PaletteKey::Up),
        "down" => return Some(PaletteKey::Down),
        "enter" => return Some(PaletteKey::Enter),
        "escape" => return Some(PaletteKey::Escape),
        "backspace" => return Some(PaletteKey::Backspace),
        _ => {}
    }
    if keystroke.modifiers.control || keystroke.modifiers.alt || keystroke.modifiers.platform {
        return None;
    }
    // `key_char` 是「这次按键能打出的字符」，中文输入法走的就是它。
    let text = keystroke
        .key_char
        .as_deref()
        .unwrap_or(keystroke.key.as_str());
    let mut chars = text.chars();
    let c = chars.next()?;
    if chars.next().is_some() || c.is_control() {
        return None;
    }
    Some(PaletteKey::Char(c))
}

/// 命令列表的一行：标题 + 快捷键提示。
fn render_row(row: &PaletteRow, hint: Option<&str>, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let mut title = div().text_sm().child(row.title);
    let mut line = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .py_2()
        .rounded_sm();
    if row.selected {
        line = line.bg(theme.accent);
        title = title.font_bold().text_color(theme.accent_foreground);
    } else {
        title = title.text_color(theme.foreground);
    }
    line = line.child(title);
    if let Some(hint) = hint {
        let hint_color = if row.selected {
            theme.accent_foreground
        } else {
            theme.muted_foreground
        };
        line = line.child(div().text_xs().text_color(hint_color).child(hint.to_owned()));
    }
    line
}

/// 顶部搜索框：显示当前关键词；为空时显示占位文案。
fn render_input(view: &command_palette::CommandPaletteView, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let text = if view.query.is_empty() {
        div()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(view.placeholder)
    } else {
        div()
            .text_sm()
            .text_color(theme.foreground)
            .child(view.query.clone())
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(div().text_sm().text_color(theme.muted_foreground).child(">"))
        .child(text)
}

/// 底部键盘提示。
fn render_footer(view: &command_palette::CommandPaletteView, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(view.footer)
}

impl Render for CommandPaletteGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        // 面板关闭：什么都不渲染。
        let Some(view) = command_palette::render(&self.state) else {
            return div().into_any_element();
        };

        // 快捷键提示：按命令 **id** 去注册表里查一条提示文案。
        // id 只用于**查提示**，派发仍由描述层按 id 完成。
        let hints: Vec<Option<String>> = self
            .state
            .filtered()
            .iter()
            .map(|command| {
                self.registry
                    .bindings()
                    .iter()
                    .find(|binding| binding.command_id == command.id)
                    .map(|binding| binding.hotkey.to_string())
            })
            .collect();
        let rows: Vec<(&PaletteRow, Option<&str>)> = view
            .rows
            .iter()
            .zip(hints.iter())
            .map(|(row, hint)| (row, hint.as_deref()))
            .collect();

        // 命令列表
        let mut list = div()
            .flex()
            .flex_col()
            .max_h(px(LIST_MAX_HEIGHT))
            .overflow_y_scrollbar();
        if rows.is_empty() {
            // 空白结果给出显式提示，不静默显示空列表。
            let empty = view.empty_text.unwrap_or("没有匹配的命令");
            list = list.child(
                div()
                    .px_3()
                    .py_4()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(empty),
            );
        } else {
            list = list.children(rows.iter().map(|(row, hint)| render_row(row, *hint, &theme)));
        }

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
            .child(render_input(&view, &theme))
            .child(list)
            .child(render_footer(&view, &theme));

        // 半透明遮罩层：压暗背景，把注意力集中到面板上。
        let mut mask = theme.background;
        mask.fade_out(MASK_FADE);

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(mask)
            .id("lengsuo-command-palette")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                view.handle_key_down(event, window, cx);
            }))
            .child(panel)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 造一个平台按键事件。
    fn keystroke(key: &str, key_char: Option<&str>, ctrl: bool, shift: bool) -> Keystroke {
        Keystroke {
            modifiers: Modifiers {
                control: ctrl,
                shift,
                ..Modifiers::default()
            },
            key: key.to_owned(),
            key_char: key_char.map(str::to_owned),
        }
    }

    #[test]
    fn named_keys_map_to_palette_keys() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("up", None, false, false)),
            Some(PaletteKey::Up)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("down", None, false, false)),
            Some(PaletteKey::Down)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("enter", None, false, false)),
            Some(PaletteKey::Enter)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("escape", None, false, false)),
            Some(PaletteKey::Escape)
        );
        assert_eq!(
            palette_key_from_keystroke(&keystroke("backspace", None, false, false)),
            Some(PaletteKey::Backspace)
        );
    }

    #[test]
    fn plain_letters_become_char_keys() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("k", Some("k"), false, false)),
            Some(PaletteKey::Char('k'))
        );
    }

    #[test]
    fn ime_character_is_preferred_over_the_raw_key() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("a", Some("设"), false, false)),
            Some(PaletteKey::Char('设'))
        );
    }

    #[test]
    fn control_combinations_are_not_text_input() {
        assert_eq!(
            palette_key_from_keystroke(&keystroke("s", Some("s"), true, false)),
            None,
            "Ctrl+S 不能把 s 打进搜索框"
        );
    }

    #[test]
    fn ctrl_k_converts_to_the_registered_hotkey() {
        let hotkey = hotkey_from_keystroke(&keystroke("k", Some("k"), true, false))
            .expect("Ctrl+K 应能转换");
        assert_eq!(
            hotkey,
            Hotkey::parse("Ctrl+K").expect("内置快捷键文本必须可解析")
        );
    }

    #[test]
    fn ctrl_shift_p_converts_to_the_registered_hotkey() {
        let hotkey = hotkey_from_keystroke(&keystroke("p", Some("P"), true, true))
            .expect("Ctrl+Shift+P 应能转换");
        assert_eq!(
            hotkey,
            Hotkey::parse("Ctrl+Shift+P").expect("内置快捷键文本必须可解析")
        );
    }

    #[test]
    fn default_registry_opens_the_palette() {
        let registry = default_registry();
        let ctrl_k = Hotkey::parse("Ctrl+K").expect("可解析");
        assert_eq!(
            registry.command_for(ctrl_k),
            Some(crate::state::hotkeys::command_ids::PALETTE_OPEN)
        );
    }

    #[test]
    fn panel_stays_closed_until_opened() {
        let mut state = PaletteState::new();
        assert!(!state.is_open());
        assert!(command_palette::render(&state).is_none());
        state.open();
        let view = command_palette::render(&state).expect("打开后应有描述");
        assert_eq!(view.query, "", "打开时关键词必须为空");
        assert_eq!(view.rows.len(), state.filtered().len());
    }

    #[test]
    fn selection_stops_at_the_edges() {
        let mut state = PaletteState::new();
        state.open();
        command_palette::handle_key(&mut state, PaletteKey::Up);
        assert_eq!(state.selected_index(), Some(0), "首项上移不变");
        for _ in 0..state.filtered().len() + 2 {
            command_palette::handle_key(&mut state, PaletteKey::Down);
        }
        assert_eq!(
            state.selected_index(),
            Some(state.filtered().len() - 1),
            "末项下移不变"
        );
    }
}
