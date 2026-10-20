//! GPUI 骨架输入区：一个用于复现中文 IME 的多行输入框。
//!
//! 本模块只做渲染，不读取 [`crate::state`]、不持有业务状态、不投递任何内容。
//! 占位符与默认文本均为中文，用于验证 CJK 字形加载以及输入法候选窗的定位。
#![cfg(feature = "ui")]

use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::*;

/// 中文占位符：启动后应能在输入框内看到这段文字。
pub const PLACEHOLDER: &str = "在此输入中文，检查输入法候选窗与光标位置是否一致……";

/// 最小多行输入框。
pub struct SkeletonInput {
    state: Entity<TextareaState>,
}

impl SkeletonInput {
    /// 创建输入框并把焦点放到输入区，启动后可直接触发输入法。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| TextareaState::new(window, cx).placeholder(PLACEHOLDER));
        let focus_handle = state.read(cx).focus_handle(cx);
        window.focus(&focus_handle, cx);
        Self { state }
    }

    /// 当前草稿文本（含换行）。骨架不消费它，仅用于调试与后续测试。
    pub fn value(&self, cx: &App) -> SharedString {
        self.state.read(cx).value()
    }
}

impl Render for SkeletonInput {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w_full().child(Textarea::new(&self.state).h(px(180.)))
    }
}
