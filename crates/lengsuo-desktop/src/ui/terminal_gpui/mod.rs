//! 集成终端抽屉的 **GPUI 接线层**：把描述层接到元素树。
//!
//! 分层约定（与 [`crate::ui::env_gpui`] 黄金范例一致）：
//! - **描述层**（[`crate::ui::terminal`]）：纯数据、无 GPUI 依赖，可在无窗口环境测。
//! - **接线层**（本目录）：只做「描述字段 → 样式 / 事件」的映射，**不重新实现**业务判断。
//!
//! 硬约束：
//! - ⛔ **不 spawn 进程、不实现 PTY**：运行命令是运行时侧的事，
//!   本层只把「用户点了运行」记成意图（[`crate::state::terminal::TerminalIntent`]）。
//! - ⛔ **不解析 ANSI 转义序列**：输出原样等宽显示。
//! - ⛔ **不落盘**：抽屉开合与命令历史都只在内存里。
//! - 颜色一律取自 `cx.theme()`，⛔ 不硬编码。
//! - 抽屉**默认收起**（对齐 13 册「底部抽屉式默认收起」）。
//!
//! 文件划分（每个文件单一职责）：
//! - `mod.rs`（本文件）：视图本体 + 输入行编辑器的绑定 + `Render`。
//! - `drawer.rs`：抽屉容器（收起 ⇄ 展开、标题条、内容区）。
//! - `tabs.rs`：tab 条（输出 / 历史）。
//! - `output_view.rs`：输出区 / 历史区（等宽、代码块底色）。
//! - `input_row.rs`：输入行 + 运行 / 中断按钮。
//! - `tests.rs`：无窗口可测的单测。
#![cfg(feature = "ui")]

mod drawer;
mod input_row;
mod output_view;
mod tabs;
#[cfg(test)]
mod tests;

use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::terminal::TerminalState;
use crate::ui::terminal::TerminalView;

/// 终端抽屉视图。
///
/// 持有**一份**终端会话态副本（[`TerminalState`]）：点标题条改的是本副本，
/// 宿主（`AppShell`）经 [`Self::state_mut`] 读回意图。
pub struct TerminalDrawerGpui {
    /// 终端会话态（描述层的唯一真源）。
    state: TerminalState,
    /// 输入行编辑器状态：**首次渲染时创建**（构造时拿不到窗口）。
    ///
    /// 与命令面板搜索框同一写法（见 `ui/palette_gpui/mod.rs`）。
    textarea: Option<Entity<TextareaState>>,
    /// 编辑器事件订阅：drop 即退订。
    _subscription: Option<Subscription>,
}

impl Default for TerminalDrawerGpui {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalDrawerGpui {
    /// 构造（**默认收起**）。
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: TerminalState::default(),
            textarea: None,
            _subscription: None,
        }
    }

    /// 只读访问状态。
    #[must_use]
    pub fn state(&self) -> &TerminalState {
        &self.state
    }

    /// 可写访问状态（宿主注入输出 / 消费意图）。
    pub fn state_mut(&mut self) -> &mut TerminalState {
        &mut self.state
    }

    /// 渲染描述（**委托给描述层**，⛔ 不在这里重新判断）。
    #[must_use]
    pub fn view(&self) -> TerminalView {
        crate::ui::terminal::terminal_view(&self.state)
    }

    /// 输入行编辑器（尚未渲染过时为 `None`）。
    #[must_use]
    pub fn textarea(&self) -> Option<&Entity<TextareaState>> {
        self.textarea.as_ref()
    }

    /// 首次渲染时按需创建输入行编辑器，并订阅它的事件。
    ///
    /// `Context` 上没有公开的取窗口方法，所以编辑器**延迟创建**：
    /// 第一次渲染时窗口可用，在那里建（与命令面板搜索框同一写法）。
    fn ensure_textarea(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.textarea.is_some() {
            return;
        }
        let textarea = cx.new(|cx| {
            // 终端输入行是**单行语义**：Enter 不换行，而是提交（PressEnter）。
            TextareaState::new(window, cx).submit_on_enter(true)
        });
        self._subscription = Some(cx.subscribe_in(
            &textarea,
            window,
            |this: &mut TerminalDrawerGpui, _emitter, event: &InputEvent, window, cx| {
                this.on_textarea_event(event, window, cx);
            },
        ));
        self.textarea = Some(textarea);
    }

    /// 编辑器事件：内容变化 ⇒ 写回输入行；回车 ⇒ 提交命令。
    fn on_textarea_event(
        &mut self,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::Change => {
                let Some(textarea) = self.textarea.clone() else {
                    return;
                };
                // 文本字符与退格**由编辑器消费**（它自己发 Change）⇒ 这里只回写，
                // 不重复写入。
                let text = textarea.read(cx).value().to_string();
                if self.state.input != text {
                    self.state.set_input(text);
                    cx.notify();
                }
            }
            InputEvent::PressEnter { shift, .. } => {
                // Shift+Enter 是编辑器自己的换行语义，终端命令行不认。
                if *shift {
                    return;
                }
                self.submit(window, cx);
            }
            InputEvent::Focus | InputEvent::Blur => {}
        }
    }

    /// 提交输入行：登记运行意图（**不执行**）。
    ///
    /// 空白输入由状态层 fail-closed（不产生任何意图）⇒ 这里**不重复判断**，
    /// 只在真的提交了（意图队列增长）时才清空编辑器。
    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 以编辑器的实际内容为准（IME 组字期间状态可能还没跟上）。
        self.sync_input(cx);
        let before = self.state.pending.len();
        self.state.submit_input();
        if self.state.pending.len() == before {
            // 空白输入：状态层什么都没做 ⇒ 编辑器也保持原样（不吞用户的输入）。
            return;
        }
        // 清空编辑器。`set_value` 不发 `Change` 事件 ⇒ 不会回环。
        if let Some(textarea) = self.textarea.clone() {
            textarea.update(cx, |textarea, textarea_cx| {
                textarea.set_value("", window, textarea_cx);
            });
        }
        cx.notify();
    }

    /// 把编辑器内容同步到状态层的输入行。
    fn sync_input(&mut self, cx: &mut Context<Self>) {
        let Some(textarea) = self.textarea.clone() else {
            return;
        };
        let text = textarea.read(cx).value().to_string();
        if self.state.input != text {
            self.state.set_input(text);
        }
    }
}

impl Render for TerminalDrawerGpui {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 首次绘制：建输入行编辑器（之后每帧复用）。
        self.ensure_textarea(window, cx);

        let theme = cx.theme().clone();
        let view = self.view();
        drawer::render_drawer(&view, self.textarea.as_ref(), &theme, cx)
    }
}
