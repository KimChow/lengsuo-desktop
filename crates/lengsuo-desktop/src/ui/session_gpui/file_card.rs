//! 会话流里的**文件卡片**（可点）与「点文件 → 跳预览」的**跳转意图**接线。
//!
//! 对齐基准：`docs/ui-parity-spec.md` 的 **A4**（点文件跳预览）。
//! 对标资料：13 册 §8 验收项「Edit diff 红绿正确；**点文件跳预览**」。
//!
//! 分层：
//! - **描述层**（[`crate::state::file_intent`]）：跳转意图的纯数据（路径 + 来源），
//!   ⛔ 不依赖 GPUI、无窗口可测；
//! - 本文件（接线层）：把描述接到 GPUI 元素树，**不重新实现**任何判断。
//!
//! 硬约束：
//! - ⛔ **本仓不实现预览器**（预览器套件属 P3，见 `ui-parity-spec.md` A14）：
//!   点击只**产出跳转意图**，不读文件、不开窗口、**不落盘**。
//! - 颜色一律取自 `cx.theme()`，⛔ 不硬编码。
//! - 元素 id 必须**同屏唯一**：用**文件路径**派生（见 [`file_element_id`]）。

use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use super::{with_click, BlockCtx};
use crate::state::file_intent::FileIntentSource;
use crate::ui::session_render::FileItem;

/// 生成文件条目的元素 id（**同屏唯一**：块序号 + 序号 + 文件路径）。
///
/// ⛔ 不用常量字面量：同屏两个不同的文件不能撞 id（撞了 GPUI 的交互状态会串台）。
pub(super) fn file_element_id(ctx: &BlockCtx<'_>, kind: &str, sub: usize, path: &str) -> String {
    format!("{}-{path}", ctx.element_id(kind, sub))
}

/// 点文件 ⇒ 产出跳转意图（**替换**旧意图，不累积）。
///
/// ⛔ 预览器未实现（P3）：这里只记意图，**不**读文件、不落盘。
pub(super) fn record_file_click(
    view: &mut super::SessionStreamGpui,
    path: &str,
    source: FileIntentSource,
) {
    view.ui.toggle_file(path);
    view.ui.request_file_preview(path, source);
}

/// 文件**网格卡片**（`Block::FilesGrid`）：可点，点击产出跳转意图。
pub(super) fn render_file_card(item: &FileItem, sub: usize, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let selected = ctx.ui.selected_file() == Some(item.path.as_str());
    // 跳转意图指向这个文件 ⇒ 高亮（意图是**真实存在**的，可如实显示）。
    let targeted = ctx.ui.file_intent().map(|i| i.path.as_str()) == Some(item.path.as_str());

    let mut card = div()
        .flex()
        .flex_col()
        .gap_1()
        .w(px(super::FILE_CARD_W))
        .px_2()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if selected || targeted {
            theme.accent
        } else {
            theme.border
        })
        .bg(theme.background);

    card = card
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(if selected || targeted {
                    theme.accent
                } else {
                    theme.foreground
                })
                .child(item.display.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(item.path.clone()),
        );

    if item.clickable {
        let path = item.path.clone();
        let id = file_element_id(ctx, "file-card", sub, &item.path);
        with_click(card, id, ctx.host, move |view| {
            record_file_click(view, &path, FileIntentSource::FileCard);
        })
    } else {
        card.into_any_element()
    }
}

/// 工具结果里的**文件列表行**（`ToolResultKind::FileList`）：可点，点击产出跳转意图。
pub(super) fn render_file_row(item: &FileItem, sub: usize, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let selected = ctx.ui.selected_file() == Some(item.path.as_str());
    let targeted = ctx.ui.file_intent().map(|i| i.path.as_str()) == Some(item.path.as_str());

    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .w_full()
        .px_2()
        .py_1()
        .rounded_sm()
        .text_sm()
        .text_color(if selected || targeted {
            theme.accent
        } else {
            theme.foreground
        });
    if selected || targeted {
        row = row.bg(theme.selection);
    }
    row = row
        .child(div().font_bold().child(item.display.clone()))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(item.path.clone()),
        );

    if item.clickable {
        let path = item.path.clone();
        let id = file_element_id(ctx, "file-list", sub, &item.path);
        with_click(row, id, ctx.host, move |view| {
            record_file_click(view, &path, FileIntentSource::FileList);
        })
    } else {
        row.into_any_element()
    }
}

/// 文件条目的**可点性**（描述层口径）：能不能点。
///
/// 只读描述层判定，**不**在这里改状态。
#[must_use]
#[cfg(test)]
pub(super) fn is_clickable(item: &FileItem) -> bool {
    item.clickable
}
