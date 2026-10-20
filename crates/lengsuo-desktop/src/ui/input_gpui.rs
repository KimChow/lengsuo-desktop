//! 输入区的 GPUI 接线层：把 [`crate::ui::input_area`] 的描述接到 GPUI 元素树。
//!
//! 分层约定（与 [`crate::ui::env_gpui`] 黄金范例一致）：
//! - **描述层**（`input_area`）：纯数据、无 GPUI 依赖；「空白禁发 / 等待确认期间禁重发」
//!   这些语义由它保证，接线层**不重新实现**。
//! - **接线层**（本文件）：只做「描述字段 → 样式」的映射。
//!
//! 三条硬约束：
//! - **不写网络**：发送只产出 [`OutgoingInput`]，放进回调或公共队列，**不投递**。
//! - **颜色走 `cx.theme()`**：不硬编码。
//! - **不得声称中文输入可用**：IME 修复未完成，中文输入在此只用于**复现**问题。
//!
//! 纯逻辑（模式切换、高亮判定、产出待投递输入）不依赖 GPUI，因此**始终可编译、可测**；
//! 只有 GPUI 元素部分受 `feature = "ui"` 隔离。

use crate::ui::input_area::{InputAreaState, InputMode, OutgoingInput};

#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::ActiveTheme as _;
#[cfg(feature = "ui")]
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
#[cfg(feature = "ui")]
use gpui_kit::*;
// `on_click` 来自 `StatefulInteractiveElement`，不 import 就没有这个方法。
#[cfg(feature = "ui")]
use gpui_kit::StatefulInteractiveElement as _;

#[cfg(feature = "ui")]
use crate::ui::gpui_input::PLACEHOLDER;

/// 模式按钮的渲染描述。
///
/// 标签取自描述层的 [`InputMode::label`]，高亮由当前模式决定；
/// 接线层不在此另造一套文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeButton {
    /// 该按钮代表的模式。
    pub mode: InputMode,
    /// 展示文案（中文）。
    pub label: &'static str,
    /// 是否为当前选中模式（选中者高亮）。
    pub selected: bool,
}

/// 模式条渲染描述：5 个模式按钮，**有且只有一个**高亮。
#[must_use]
pub fn mode_buttons(current: InputMode) -> [ModeButton; 5] {
    InputMode::ALL.map(|mode| ModeButton {
        mode,
        label: mode.label(),
        selected: mode_is_selected(current, mode),
    })
}

/// 高亮判定：只有当前模式高亮。
#[must_use]
pub fn mode_is_selected(current: InputMode, candidate: InputMode) -> bool {
    current == candidate
}

/// 切换模式。返回模式是否真的发生了变化（重复点击同一模式不算变化）。
pub fn apply_mode(state: &mut InputAreaState, mode: InputMode) -> bool {
    if state.mode() == mode {
        return false;
    }
    state.set_mode(mode);
    true
}

/// 产出一次待投递的输入。
///
/// 空白草稿、等待确认期间都返回 `None` —— 语义由描述层的
/// [`InputAreaState::submit`] 保证，这里不重复判断。
pub fn take_outgoing(state: &mut InputAreaState) -> Option<OutgoingInput> {
    state.submit()
}

/// 输入区的 GPUI 视图。
#[cfg(feature = "ui")]
pub struct InputAreaGpui {
    /// 描述层状态：草稿 / 模式 / 等待确认。
    state: InputAreaState,
    /// 多行编辑器状态（复用 `Textarea` 组件，与 `gpui_input` 的骨架同一套）。
    textarea: Entity<TextareaState>,
    /// 待投递输入队列：**不投递**，只排队等调用方取走。
    outbox: Vec<OutgoingInput>,
    /// 可选的发送回调：拿到 [`OutgoingInput`] 后由调用方决定怎么用。
    on_send: Option<Box<dyn FnMut(OutgoingInput) + 'static>>,
    /// 订阅必须随视图一起存活，drop 即退订。
    _subscription: Subscription,
}

#[cfg(feature = "ui")]
impl InputAreaGpui {
    /// 创建输入区：模式条 + 多行输入框 + 发送按钮。
    ///
    /// `Enter` 发送、`Shift+Enter` 换行由 `TextareaState::submit_on_enter(true)`
    /// 提供（多行模式下 Enter 视为提交、Shift+Enter 插入换行），接线层只订阅
    /// [`InputEvent::PressEnter`] 并复用描述层的发送语义。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let textarea = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(PLACEHOLDER)
                .submit_on_enter(true)
        });

        let subscription = cx.subscribe_in(
            &textarea,
            window,
            |this: &mut InputAreaGpui, _emitter, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.sync_draft(cx),
                InputEvent::PressEnter { shift, .. } => {
                    // Shift+Enter 已被编辑器当作换行吃掉，这里只处理纯 Enter 发送。
                    if !*shift {
                        this.submit_and_queue(window, cx);
                    }
                }
                InputEvent::Focus | InputEvent::Blur => {}
            },
        );

        Self {
            state: InputAreaState::new(),
            textarea,
            outbox: Vec::new(),
            on_send: None,
            _subscription: subscription,
        }
    }

    /// 当前描述层状态（只读）。
    #[must_use]
    pub fn state(&self) -> &InputAreaState {
        &self.state
    }

    /// 当前草稿。
    #[must_use]
    pub fn draft(&self) -> &str {
        self.state.draft()
    }

    /// 当前模式。
    #[must_use]
    pub fn mode(&self) -> InputMode {
        self.state.mode()
    }

    /// 切换模式（高亮随之更新）。
    pub fn select_mode(&mut self, mode: InputMode, cx: &mut Context<Self>) -> bool {
        let changed = apply_mode(&mut self.state, mode);
        if changed {
            cx.notify();
        }
        changed
    }

    /// 运行时已受理：解除「等待确认」，发送按钮恢复可用。
    pub fn finish_send(&mut self, cx: &mut Context<Self>) {
        self.state.finish_send();
        cx.notify();
    }

    /// 设置发送回调。回调拿到 [`OutgoingInput`]，**由调用方决定投递与否**。
    pub fn set_on_send(&mut self, callback: impl FnMut(OutgoingInput) + 'static) {
        self.on_send = Some(Box::new(callback));
    }

    /// 待投递输入队列（只读）。
    #[must_use]
    pub fn pending_outgoing(&self) -> &[OutgoingInput] {
        &self.outbox
    }

    /// 取走最早的一条待投递输入。
    pub fn take_pending(&mut self) -> Option<OutgoingInput> {
        if self.outbox.is_empty() {
            return None;
        }
        Some(self.outbox.remove(0))
    }

    /// 编辑器里的实际内容（调试用；不等同于草稿，IME 组字期间可能领先）。
    #[must_use]
    pub fn textarea_value(&self, cx: &App) -> SharedString {
        self.textarea.read(cx).value()
    }

    /// 把编辑器内容同步到描述层草稿。
    fn sync_draft(&mut self, cx: &mut Context<Self>) {
        let text = self.textarea.read(cx).value().to_string();
        let current = self.state.draft().to_owned();
        if current != text {
            self.state.set_draft(text);
            cx.notify();
        }
    }

    /// 产出一次待投递输入并入队。空白或等待确认期间什么都不做。
    fn submit_and_queue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 以编辑器的实际内容为准：IME 组字期间草稿可能还没跟上。
        self.sync_draft(cx);
        let Some(outgoing) = take_outgoing(&mut self.state) else {
            return;
        };
        // 清空编辑器。`set_value` 不会发 `InputEvent::Change`，因此不会回环。
        self.textarea
            .update(cx, |textarea, textarea_cx| {
                textarea.set_value("", window, textarea_cx);
            });
        self.outbox.push(outgoing.clone());
        if let Some(callback) = self.on_send.as_mut() {
            callback(outgoing);
        }
        cx.notify();
    }
}

#[cfg(feature = "ui")]
impl Render for InputAreaGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let buttons = mode_buttons(self.state.mode());

        // 模式切换条：当前模式高亮，点击切换。
        let mut mode_elements: Vec<AnyElement> = Vec::with_capacity(buttons.len());
        for button in buttons {
            let mode = button.mode;
            let listener = cx.listener(
                move |this: &mut InputAreaGpui, _: &ClickEvent, _window: &mut Window, cx| {
                    this.select_mode(mode, cx);
                },
            );
            let element = if button.selected {
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_sm()
                    .font_bold()
                    .bg(theme.accent)
                    .text_color(theme.accent_foreground)
                    .cursor_pointer()
                    .on_click(listener)
                    .child(button.label)
                    .into_any_element()
            } else {
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .text_sm()
                    .bg(theme.button)
                    .text_color(theme.button_foreground)
                    .cursor_pointer()
                    .on_click(listener)
                    .child(button.label)
                    .into_any_element()
            };
            mode_elements.push(element);
        }

        // 发送按钮：描述层判定不可发送时置灰，且不注册点击（点了也没反应）。
        let send = self.state.send_button();
        let send_listener = cx.listener(
            |this: &mut InputAreaGpui, _: &ClickEvent, window: &mut Window, cx| {
                this.submit_and_queue(window, cx);
            },
        );
        let send_element: AnyElement = if send.enabled {
            div()
                .px_3()
                .py_1()
                .rounded_sm()
                .text_sm()
                .font_bold()
                .bg(theme.button_primary)
                .text_color(theme.button_primary_foreground)
                .cursor_pointer()
                .on_click(send_listener)
                .child(send.label)
                .into_any_element()
        } else {
            div()
                .px_3()
                .py_1()
                .rounded_sm()
                .text_sm()
                .bg(theme.button_secondary)
                .text_color(theme.muted_foreground)
                .child(send.label)
                .into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .p_3()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(div().flex().flex_row().gap_1().children(mode_elements))
            .child(
                div()
                    .w_full()
                    .child(Textarea::new(&self.textarea).h(px(120.))),
            )
            .child(div().flex().justify_end().child(send_element))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_bar_renders_five_modes_with_exactly_one_selected() {
        let buttons = mode_buttons(InputMode::Ask);
        assert_eq!(buttons.len(), 5);
        assert_eq!(buttons.map(|b| b.mode), InputMode::ALL);
        assert_eq!(
            buttons.map(|b| b.label),
            ["快速", "问答", "规划", "执行", "专家"]
        );
        let selected: Vec<InputMode> = buttons
            .iter()
            .filter(|b| b.selected)
            .map(|b| b.mode)
            .collect();
        assert_eq!(selected, vec![InputMode::Ask]);
    }

    #[test]
    fn switching_mode_updates_state_and_highlight() {
        let mut state = InputAreaState::new();
        assert_eq!(state.mode(), InputMode::Ask);
        for mode in InputMode::ALL {
            assert!(apply_mode(&mut state, mode), "切换到 {mode:?} 应生效");
            assert_eq!(state.mode(), mode);
            let selected: Vec<InputMode> = mode_buttons(state.mode())
                .iter()
                .filter(|b| b.selected)
                .map(|b| b.mode)
                .collect();
            assert_eq!(selected, vec![mode], "高亮必须跟着切换");
        }
        assert!(!apply_mode(&mut state, InputMode::Expert), "重复点击不算变化");
        assert_eq!(state.mode(), InputMode::Expert);
    }

    #[test]
    fn blank_draft_is_never_produced_for_delivery() {
        let mut state = InputAreaState::new();
        assert!(take_outgoing(&mut state).is_none());
        state.set_draft("   \n\t ");
        assert!(take_outgoing(&mut state).is_none(), "纯空白也不得产出");
        assert_eq!(state.draft(), "   \n\t ", "未发送则草稿保持原样");
    }

    #[test]
    fn in_flight_blocks_resending() {
        let mut state = InputAreaState::new();
        state.set_mode(InputMode::Plan);
        state.set_draft("你好\n世界");
        let outgoing = take_outgoing(&mut state).expect("非空草稿应产出");
        assert_eq!(outgoing.text, "你好\n世界");
        assert_eq!(outgoing.mode, InputMode::Plan);
        assert!(state.is_in_flight());
        assert!(
            take_outgoing(&mut state).is_none(),
            "等待确认期间不得重复产出"
        );
        state.finish_send();
        assert!(take_outgoing(&mut state).is_none(), "草稿已清空");
    }
}
