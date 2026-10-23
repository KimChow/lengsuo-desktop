//! 「模型」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 职责 = 把描述层（`view::models`）的状态映射成元素：面板说明、「已保存模型」
//! 三态（fail-closed：未下发 ≠ 空集）、静态目录区块（高级配置 / 能力开关 / 本地模型）。
//! 区块标题与行**只读**（不可点、无增删改、无保存按钮）；颜色一律取自 `theme`
//! （⛔ 不硬编码）。模型配置下发源尚未接线 ⇒ 生产路径固定走「未下发」分支，
//! ⛔ 不编模型名 / 不编数值。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::models as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 渲染「模型」面板（接线层：把描述层状态映射成元素）。
///
/// 模型配置下发源尚未接线 ⇒ 固定 `None`（未下发 ≠ 空集，fail-closed）；
/// 有内容 / 空态分支由 [`models_panel`] 承接（渲染断言直接驱动它走全部分支）。
pub(super) fn render_models_panel(theme: &Theme, _cx: &mut Context<SettingsGpui>) -> AnyElement {
    models_panel(theme, None).into_any_element()
}

/// 面板体：`saved` = 已下发的模型参数值清单（`None` = 未下发）。
fn models_panel(theme: &Theme, saved: Option<&[String]>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(title_text(theme, panel_view::TITLE))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::SUBTITLE),
        )
        .child(saved_block(theme, &panel_view::body(saved)))
        .children(panel_view::catalog().iter().map(|s| section(theme, s)))
}

/// 「已保存模型」区块：三态由纯函数 `view::models::body` 判定，接线层只做映射。
fn saved_block(theme: &Theme, state: &panel_view::SavedModels) -> Div {
    let block = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(title_text(theme, panel_view::SAVED_TITLE));
    match state {
        // 未下发：兜底提示（⛔ 不冒充空态、不编条目）。
        panel_view::SavedModels::NotDelivered => {
            block.child(muted_text(theme, panel_view::NOT_DELIVERED))
        }
        // 已下发的空态：标题 + 说明。
        panel_view::SavedModels::Empty => block
            .child(plain_text(theme, panel_view::EMPTY_TITLE))
            .child(muted_text(theme, panel_view::EMPTY_DESCRIPTION)),
        // 已下发的条目：模型参数值原样透传（不加动作、不加工）。
        panel_view::SavedModels::Entries(names) => block
            .children(names.iter().map(|name| plain_text(theme, name.clone()))),
    }
}

/// 标题行（面板标题 / 区块标题共用一套 token）。
fn title_text(theme: &Theme, text: &'static str) -> Div {
    div()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(text)
}

/// 静态目录区块 → 元素：标题（描述层为 `None` 的区块不编标题）+ 说明行 + 只读行。
fn section(theme: &Theme, s: &panel_view::Section) -> Div {
    let block = div().flex().flex_col().gap_2();
    let block = match s.title {
        Some(title) => block.child(title_text(theme, title)),
        None => block,
    };
    let block = if s.intro.is_empty() {
        block
    } else {
        block.child(muted_text(theme, s.intro))
    };
    block.children(s.rows.iter().map(|r| row(theme, r)))
}

/// 只读行：标签 + 说明（⛔ 不可点、无编辑动作）。
fn row(theme: &Theme, r: &panel_view::Row) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(plain_text(theme, r.label))
        .child(muted_text(theme, r.value.clone()))
}

/// 普通只读行（前景色文字）。
fn plain_text(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .text_color(theme.foreground)
        .child(text.into())
}

/// 弱化只读行（说明 / 兜底提示）。
fn muted_text(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.into())
}
