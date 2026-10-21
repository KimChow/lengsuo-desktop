//! 工具卡片的 GPUI 渲染分支：图标 + 人话标题 + 参数摘要 + 可折叠。
//!
//! 与 [`super::tool_fold`]（纯逻辑）分工：那边产出**描述**，这边只把描述画成元素，
//! **不重新实现**任何业务判断。
//!
//! 形态（对齐参考客户端的折叠卡片）：
//!
//! ```text
//! ┌────────────────────────────────────┐
//! │ ▤ 读取 src/main.rs            ▶   │  ← 标题行（可点击）
//! │ src/main.rs                       │  ← 第二行：参数摘要（折叠态 ≤60 字）
//! └────────────────────────────────────┘
//! ```
//!
//! 展开后第二行显示**完整参数**（不截断）。点击标题行 ⇒ 展开 / 收起。
//!
//! 硬约束：
//! - **R13-1**：界面上任何位置都不得出现工具英文名 ⇒ 标题取描述层的人话标题，
//!   图标用 unicode 符号（⛔ 不引图标依赖）。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
//! - 折叠态按**消息 id** 记忆（[`super::SessionUiState`]），**不落盘**。

use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use super::tool_fold::{describe, ToolCallDetail};
use super::{with_click, BlockCtx};
use crate::ui::tool_card::ToolCard;

/// 工具卡片：图标 + 人话标题 + 参数摘要，默认折叠；点击标题行展开 / 收起。
pub(super) fn render_tool_card(
    card: &ToolCard,
    detail: Option<&ToolCallDetail>,
    ctx: &BlockCtx<'_>,
) -> AnyElement {
    let theme = ctx.theme;
    // 折叠态按**消息 id** 记忆；没有接线细节时退回描述层的默认折叠。
    let folded = match detail {
        Some(d) => ctx.ui.is_tool_folded(&d.id),
        None => card.folded,
    };
    let view = describe(card, detail, folded);

    let marker = if folded { "▶" } else { "▼" };
    let header = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_2()
        .w_full()
        .px_3()
        .py_2()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(view.icon),
                )
                .child(div().text_sm().font_bold().child(view.title.clone())),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(marker),
        );

    // 折叠态显示摘要（≤60 字）；展开态显示**完整参数**（不截断）。
    let body = if folded {
        div()
            .px_3()
            .pb_2()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(view.summary.clone())
    } else {
        div()
            .px_3()
            .pb_2()
            .text_xs()
            .text_color(theme.foreground)
            .font_family(theme.mono_font_family.clone())
            .child(view.args.clone())
    };

    let container = div()
        .flex()
        .flex_col()
        .w_full()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(header)
        .child(body);

    // 折叠态只在**有消息 id** 时可点（静态预览没有状态可记）。
    let Some(detail) = detail else {
        return container.into_any_element();
    };
    let id = detail.id.clone();
    let element_id = format!("tool-card-{id}");
    with_click(container, element_id, ctx.host, move |view| {
        view.toggle_tool_fold(&id);
    })
}
