//! 环境视图的 GPUI 接线层：把 [`crate::ui::env_view`] 的渲染描述接到元素树。
//!
//! **这是本仓接线层的黄金范例** —— 其他 `*_gpui.rs` 照这个文件的结构写。
//!
//! 分层约定：
//! - **描述层**（`env_view`）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（本文件）：`feature = "ui"` 隔离，把描述变成 GPUI 元素。
//!   本文件**不重新实现**任何业务判断，只做「描述字段 → 样式」的映射。
//!
//! 硬约束：
//! - fail-closed：加载失败必须显示错误，**不得**退化成空列表。
//! - 「真实为空」与「筛选无结果」用不同文案（描述层已区分，此处不得合并）。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::ui::env_view::{self, CardStatus, EnvView};

/// 环境视图的 GPUI 视图。
pub struct EnvViewGpui {
    /// 渲染描述；接线层不持有业务状态，只持有描述。
    view: EnvView,
}

impl EnvViewGpui {
    /// 用一份渲染描述构造视图。
    #[must_use]
    pub fn new(view: EnvView) -> Self {
        Self { view }
    }

    /// 从状态生成描述并构造视图（接线层最常用的入口）。
    #[must_use]
    pub fn from_state(state: &crate::state::env_group::EnvGroupState) -> Self {
        Self::new(env_view::render(state))
    }
}

/// 状态标签的展示样式（颜色 + 文案）。
///
/// 状态色取自主题；文案来自描述层的 [`CardStatus::label`]，不在此另造一套。
fn status_style(
    status: CardStatus,
    theme: &gpui_kit::component::Theme,
) -> (gpui_kit::Hsla, &'static str) {
    let color = match status {
        CardStatus::Ready => theme.accent,
        CardStatus::Degraded => theme.button_warning,
        CardStatus::Failed => theme.button_danger,
    };
    (color, status.label())
}

/// 单张环境卡片。
fn render_card(card: &env_view::EnvCard, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let (status_color, status_text) = status_style(card.status, theme);
    let name = if card.name_is_placeholder {
        format!("（未命名环境）{}", card.id)
    } else {
        card.name.clone()
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .w(px(220.))
        .child(div().text_sm().font_bold().child(name))
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(card.group.name.clone()),
                )
                .child(div().text_xs().text_color(status_color).child(status_text)),
        )
}

/// 左侧分组栏的一行。
fn render_group_row(
    row: &env_view::GroupRow,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let mut text = div().text_sm().child(row.label.clone());
    if row.selected {
        text = text.font_bold().text_color(theme.accent);
    }
    div()
        .flex()
        .justify_between()
        .px_2()
        .py_1()
        .rounded_sm()
        .child(text)
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(row.env_count.to_string()),
        )
}

/// 顶部提示条（降级提示 / 加载失败）。
fn render_notice(
    text: &str,
    color: gpui_kit::Hsla,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(color)
        .text_xs()
        .text_color(theme.background)
        .child(text.to_owned())
}

impl Render for EnvViewGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let view = &self.view;

        let root = div()
            .flex()
            .h_full()
            .w_full()
            .bg(theme.background)
            .text_color(theme.foreground);

        // 加载中：显示加载态，不显示空列表（fail-closed 的第一步）。
        if view.loading {
            return root.flex_col().items_center().justify_center().child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("正在加载环境……"),
            );
        }

        // 加载失败：显示错误原因，绝不退化成空列表。
        if let Some(error) = &view.error {
            return root
                .flex_col()
                .gap_2()
                .p_4()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.button_danger)
                        .child("环境列表加载失败"),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(error.clone()),
                );
        }

        // 左侧分组栏
        let sidebar = div()
            .w(px(200.))
            .h_full()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .border_r_1()
            .border_color(theme.border)
            .children(view.sidebar.iter().map(|row| render_group_row(row, &theme)));

        // 右侧主区
        let mut main = div().flex_1().h_full().flex().flex_col().gap_2().p_3();

        // 降级提示（描述层保证 invalid_count > 0 时必有）
        if let Some(notice) = &view.degraded_notice {
            main = main.child(render_notice(notice, theme.button_warning, &theme));
        }

        // 卡片网格
        if view.cards.is_empty() {
            // 真实为空与筛选无结果用不同文案（描述层已区分）
            let text = view.empty_text.unwrap_or("暂无环境");
            main = main.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(text),
            );
        } else {
            let grid = div()
                .flex()
                .flex_wrap()
                .gap_2()
                .children(view.cards.iter().map(|card| render_card(card, &theme)));
            main = main.child(grid);
        }

        root.child(sidebar).child(main)
    }
}
