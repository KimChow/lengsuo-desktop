//! 筛选栏：关键词搜索框 + 筛选空态的恢复入口。
//!
//! 只做「元素 ← 纯逻辑」的接线，判定全在 [`super::mapping`]
//! （`filter_summary` / `empty_recovery` / `clear_filter`）。
//! 本文件不含任何业务判断，也不持有凭据。
use super::mapping::{clear_filter, CLEAR_FILTER_LABEL, SEARCH_PLACEHOLDER};
use super::EnvViewGpui;
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 搜索框一行。**无条件渲染**：关键词筛选必须可达。
pub(super) fn render_search_box(
    textarea: &Entity<TextareaState>,
    theme: &Theme,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex_1()
                .child(Textarea::new(textarea).appearance(false)),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SEARCH_PLACEHOLDER),
        )
}

/// 筛选空态的「清除筛选」：点击只上报**意图**（清筛选），不判权限、不碰业务。
pub(super) fn render_clear_filter_button(
    entity: &Entity<EnvViewGpui>,
    theme: &Theme,
) -> impl IntoElement {
    let entity = entity.clone();
    div()
        .id("env-clear-filter") // on_click 需要 Stateful ⇒ 必须先 id
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.accent)
        .cursor_pointer()
        .child(CLEAR_FILTER_LABEL)
        .on_click(move |_event, window, app| {
            entity.update(app, |view, cx| {
                clear_filter(view.state_mut());
                view.filter.sync_after_clear(window, cx);
                cx.notify();
            });
        })
}

/// 搜索框状态：编辑器**延迟创建**（第一次渲染时 `window` 才可用）。
#[derive(Default)]
pub(super) struct FilterBox {
    textarea: Option<Entity<TextareaState>>,
    /// 订阅句柄（drop 即退订）。
    _subscription: Option<Subscription>,
}

impl FilterBox {
    /// 首次渲染时建编辑器 + 订阅；变化即写回 `EnvGroupState::set_keyword`。
    pub(super) fn ensure(&mut self, window: &mut Window, cx: &mut Context<EnvViewGpui>) {
        if self.textarea.is_some() {
            return;
        }
        let textarea = cx.new(|cx| TextareaState::new(window, cx).placeholder(SEARCH_PLACEHOLDER));
        self._subscription = Some(cx.subscribe(&textarea, |this, _emitter, event, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let Some(box_) = this.filter.textarea.clone() else {
                return;
            };
            let text = box_.read(cx).value().to_string();
            if this.state_mut().keyword() == text.as_str() {
                return;
            }
            this.state_mut().set_keyword(text);
            cx.notify();
        }));
        self.textarea = Some(textarea);
    }

    /// 清筛选后把输入框里的字也清掉（否则界面与状态对不上）。
    pub(super) fn sync_after_clear(&mut self, window: &mut Window, cx: &mut Context<EnvViewGpui>) {
        let Some(textarea) = self.textarea.clone() else {
            return;
        };
        if textarea.read(cx).value().is_empty() {
            return;
        }
        textarea.update(cx, |textarea, textarea_cx| {
            textarea.set_value(String::new(), window, textarea_cx);
        });
    }

    pub(super) fn textarea(&self) -> Option<&Entity<TextareaState>> {
        self.textarea.as_ref()
    }
}
