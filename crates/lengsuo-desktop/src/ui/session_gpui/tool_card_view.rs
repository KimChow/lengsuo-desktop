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

    let mut container = div()
        .flex()
        .flex_col()
        .w_full()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(header);
    // 空参数不渲染空摘要行（无参调用时卡片只剩标题行）。
    let body_text = if folded { &view.summary } else { &view.args };
    if !body_text.is_empty() {
        container = container.child(body);
    }

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

#[cfg(test)]
mod tests {
    use super::super::tool_fold::{describe, visible_text, ToolCallDetail};
    use super::super::SessionUiState;
    use crate::ui::tool_card::ToolCard;

    fn detail_of(tool: &str, args: &str) -> ToolCallDetail {
        ToolCallDetail {
            id: format!("card-{tool}"),
            tool: Some(tool.to_owned()),
            args: args.to_owned(),
        }
    }

    #[test]
    fn five_categories_render_icon_human_title_and_summary() {
        // A2/A16/A10：五类各 1 条——图标非空 + 人话标题非空 + 摘要非空 + 不露英文名。
        // 摘要语义：读=路径、命令=命令行、搜索=关键词（短参数原样进标题与摘要行）。
        let cases = [
            ("Read", "src/main.rs", "读取"),
            ("Write", "docs/plan.md", "创建"),
            ("Edit", "src/lib.rs", "修改"),
            ("Bash", "cargo test", "执行命令"),
            ("Grep", "fn main", "搜索"),
        ];
        for (tool, args, label) in cases {
            let card = ToolCard::from_tool(Some(tool), args);
            let view = describe(&card, Some(&detail_of(tool, args)), true);
            assert!(!view.icon.is_empty(), "{tool} 图标非空");
            assert_eq!(view.title, format!("{label} {args}"), "{tool} 人话标题");
            assert_eq!(view.summary, args, "{tool} 摘要应为参数原文");
            assert!(
                !view.title.contains(tool),
                "标题不得露出工具英文名：{}",
                view.title
            );
            assert!(view.folded, "{tool} 默认折叠");
        }
    }

    #[test]
    fn unknown_tool_full_render_path_hides_english_name() {
        let card = ToolCard::from_tool(Some("SecretInternalTool"), "payload");
        let view = describe(
            &card,
            Some(&detail_of("SecretInternalTool", "payload")),
            true,
        );
        assert!(
            !visible_text(&view).contains("SecretInternalTool"),
            "整卡可见文字不得露出工具英文名：{}",
            visible_text(&view)
        );
    }

    #[test]
    fn static_preview_without_detail_uses_card_title() {
        let card = ToolCard::from_tool(Some("Read"), "src/main.rs");
        let view = describe(&card, None, card.folded);
        assert!(!view.icon.is_empty());
        assert_eq!(view.title, "读取 src/main.rs");
        assert_eq!(view.summary, "src/main.rs");
    }

    #[test]
    fn fold_state_is_remembered_per_message_id() {
        let mut ui = SessionUiState::default();
        assert!(ui.is_tool_folded("m-1"), "默认折叠");
        ui.toggle_tool_fold("m-1");
        assert!(!ui.is_tool_folded("m-1"), "点击展开");
        assert!(ui.is_tool_folded("m-2"), "别的卡片不受影响");
    }
}
