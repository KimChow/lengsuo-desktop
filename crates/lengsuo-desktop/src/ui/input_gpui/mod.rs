//! 输入区的 GPUI 接线层：把 [`crate::ui::input_area`] 的描述接到 GPUI 元素树。
//!
//! 分层约定（与 [`crate::ui::env_gpui`] 黄金范例一致）：
//! - **描述层**（`input_area`）：纯数据、无 GPUI 依赖；「空白禁发 / 等待确认期间禁重发」
//!   这些语义由它保证，接线层**不重新实现**。
//! - **接线层**（本模块）：只做「描述字段 → 样式」的映射。
//!
//! 三条硬约束：
//! - **不写网络**：发送只产出 [`OutgoingInput`]，放进回调或公共队列，**不投递**。
//! - **颜色走 `cx.theme()`**：不硬编码。
//! - **不得声称中文输入可用**：IME 修复未完成，中文输入在此只用于**复现**问题。
//!
//! 本卡（e17）对齐 A5 / A11 / A12：
//! - **A11 模式下拉**：一个触发器 + 展开的 5 项列表（[`mode_menu`]）。
//! - **A5 发送后变停止按钮**：等待确认期间按钮变「停止」，点击只**产出停止意图**
//!   （[`StopIntent`]），**不执行** `agent:stop`（属运行时职责）。
//! - **A12 `@` 文件引用**：草稿里出现 `@词` 时弹快选层，候选路径**由调用方注入**
//!   （[`InputAreaGpui::set_candidates`]），本仓**不读盘**。
//!
//! 弹层一律用 `div().absolute()` 自搭，**不引入任何组件库**（零新依赖）。

#[cfg(feature = "ui")]
mod logic;
#[cfg(feature = "ui")]
mod mention;
#[cfg(feature = "ui")]
mod mode_menu;
#[cfg(feature = "ui")]
mod send_button;
#[cfg(all(test, feature = "ui"))]
mod tests;

use crate::ui::input_area::{InputAreaState, InputMode, OutgoingInput};

#[cfg(feature = "ui")]
use crate::ui::gpui_input::PLACEHOLDER;
#[cfg(feature = "ui")]
use crate::ui::input_area::StopIntent;
#[cfg(feature = "ui")]
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
#[cfg(feature = "ui")]
use gpui_kit::component::ActiveTheme as _;
#[cfg(feature = "ui")]
use gpui_kit::*;

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
    /// 描述层状态：草稿 / 模式 / 等待确认 / 候选路径。
    state: InputAreaState,
    /// 多行编辑器状态（复用 `Textarea` 组件，与 `gpui_input` 的骨架同一套）。
    textarea: Entity<TextareaState>,
    /// 待投递输入队列：**不投递**，只排队等调用方取走。
    outbox: Vec<OutgoingInput>,
    /// 可选的发送回调：拿到 [`OutgoingInput`] 后由调用方决定怎么用。
    on_send: Option<Box<dyn FnMut(OutgoingInput) + 'static>>,
    /// 模式弹层是否展开。
    mode_menu_open: bool,
    /// `@` 快选弹层是否展开。
    mention_open: bool,
    /// 停止意图队列：与 outbox 同构，**由调用方取走**。
    stop_outbox: Vec<StopIntent>,
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
            mode_menu_open: false,
            mention_open: false,
            stop_outbox: Vec::new(),
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

    /// 注入候选文件路径（调用方提供；本仓**不读盘**、不扫描文件系统）。
    pub fn set_candidates(
        &mut self,
        paths: impl IntoIterator<Item = String>,
        cx: &mut Context<Self>,
    ) {
        self.state.set_candidates(paths);
        cx.notify();
    }

    /// 取走最早的停止意图（界面层只上报意图，**不执行** `agent:stop`）。
    pub fn take_stop(&mut self) -> Option<StopIntent> {
        if self.stop_outbox.is_empty() {
            return None;
        }
        Some(self.stop_outbox.remove(0))
    }

    /// 待处理的停止意图队列（只读）。
    #[must_use]
    pub fn pending_stops(&self) -> &[StopIntent] {
        &self.stop_outbox
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
            self.sync_mention_state();
            cx.notify();
        }
    }

    /// 草稿变化后同步 `@` 快选层的开关：有 `@查询词` 就自动展开，无则收起。
    fn sync_mention_state(&mut self) {
        self.mention_open = self.state.mention_query().is_some();
    }

    /// 产出一次待投递输入并入队。空白或等待确认期间什么都不做。
    fn submit_and_queue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 以编辑器的实际内容为准：IME 组字期间草稿可能还没跟上。
        self.sync_draft(cx);
        let Some(outgoing) = take_outgoing(&mut self.state) else {
            return;
        };
        // 清空编辑器。`set_value` 不会发 `InputEvent::Change`，因此不会回环。
        self.textarea.update(cx, |textarea, textarea_cx| {
            textarea.set_value("", window, textarea_cx);
        });
        self.outbox.push(outgoing.clone());
        if let Some(callback) = self.on_send.as_mut() {
            callback(outgoing);
        }
        cx.notify();
    }

    /// 把描述层草稿写回编辑器（插入 `@` 路径后调用，避免旧文本被 `sync_draft` 写回）。
    fn write_back_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.state.draft().to_owned();
        self.textarea.update(cx, |textarea, textarea_cx| {
            textarea.set_value(&text, window, textarea_cx);
        });
    }
}

#[cfg(feature = "ui")]
impl Render for InputAreaGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // `theme` 需要独占借用 `cx`，因此先 clone 一份再往下传。
        let theme = cx.theme().clone();

        // 模式条：触发器 + 展开时下拉的 5 个选项（A11）。
        let mode_elements = mode_menu::render(self, &theme, cx);

        // 发送 / 停止按钮（A5）。
        let send_element = send_button::render(self, &theme, cx);

        // `@` 快选层（A12）。
        let mention_element = mention::render(self, &theme, cx);

        div()
            .relative()
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .p_3()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                // 快选层以输入框为坐标系（`relative` 锚点），
                // 绝对定位在输入框左下角，浮在输入区之上。
                div()
                    .relative()
                    .w_full()
                    .child(Textarea::new(&self.textarea).h(px(120.)))
                    .child(mention_element),
            )
            // 底部行：左下是模式下拉（A11 原文「底部：模式下拉」），
            // 右下是发送/停止按钮（A5）。
            //
            // ⚠️ 绘制顺序 = 树序（gpui 无 z-index），所以模式下拉这一行
            // **必须排在多行输入框之后**，向上展开的面板才不会被输入框盖住。
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_end()
                    .justify_between()
                    .gap_2()
                    .child(mode_elements)
                    .child(send_element),
            )
    }
}
