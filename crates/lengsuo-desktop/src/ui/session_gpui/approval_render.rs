//! 审批卡的 GPUI 渲染分支（从 [`super::blocks`] 拆出，控制单文件行数）。
//!
//! 与 [`super::blocks`] 同属接线层：只把描述字段映射成元素，不重新实现业务判断。
//! 点击只产出**意图**（见 [`super::approval_intent`]）：不判定权限、不落盘、不放宽可见性。
//! 列表级三态分流见 [`render_approval_list`]：`Failed` ⇒ 失败原因上屏（fail-closed）。

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

use super::approval_intent::ApprovalRequest;
use super::blocks::approval_button_colors;
use super::{with_click, BlockCtx};
use crate::ui::approval_card::{
    ApprovalAction, ApprovalButton, ApprovalCardView, ApprovalListView,
};

/// 审批卡：**只读渲染**。三个按钮，点击只记本地选择，绝不放行。
pub(super) fn render_approval(card: &ApprovalCardView, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    let choice = ctx.ui.display_choice(&card.id);

    let status = div().text_xs();
    let status = match (card.pending, choice) {
        (true, None) => status.text_color(theme.muted_foreground).child("等待决策"),
        (true, Some(action)) => status.text_color(theme.warning).child(format!(
            "已选：{}（已提交意图，等待运行时确认）",
            action.label()
        )),
        (false, _) => status.text_color(theme.muted_foreground).child("已决策"),
    };

    let mut buttons = div().flex().flex_row().gap_2();
    for (i, btn) in card.buttons.iter().enumerate() {
        buttons = buttons.child(render_approval_button(
            &card.id,
            ctx.tool_of(&card.id),
            btn,
            i,
            choice,
            ctx,
        ));
    }

    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(div().text_sm().font_bold().child(card.title.clone()))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(card.summary.clone()),
        )
        .child(status)
        .child(buttons)
        .into_any_element()
}

/// 单个审批按钮（允许 / 拒绝 / 总是允许）。
pub(super) fn render_approval_button(
    id: &str,
    tool: Option<&str>,
    btn: &ApprovalButton,
    sub: usize,
    choice: Option<ApprovalAction>,
    ctx: &BlockCtx<'_>,
) -> AnyElement {
    let theme: &Theme = ctx.theme;
    let (bg, fg) = approval_button_colors(btn.action, theme);
    let selected = choice == Some(btn.action);
    let button = div()
        .px_3()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .bg(bg)
        .text_color(fg)
        .text_sm()
        .child(btn.label.to_owned());
    if !btn.enabled {
        // 已决策项的按钮必须禁用：不可点击，也不给交互反馈。
        return button.opacity(0.5).into_any_element();
    }
    let action = btn.action;
    let approval_id = id.to_owned();
    let tool = tool.map(str::to_owned);
    with_click(
        button,
        ctx.element_id("approval", sub),
        ctx.host,
        move |view| {
            // 点击只产出**意图**并记本地态：不判定权限、不落盘、不放宽任何可见性。
            // 工具名缺失时作用域为 Unknown（fail-closed），交由运行时保守处理。
            let request = ApprovalRequest::tool_scoped(Some(&approval_id), tool.as_deref(), action);
            // 意图由 `record_intent` 存入 `approval_intent`，宿主经
            // `take_approval_intent()` 取走；这里不执行、不放行。
            let _intent = view.record_intent(&request);
        },
    )
}

/// 审批区列表（C3 三态分流）：`Ready` ⇒ 逐卡渲染；`Loading` ⇒ 加载态；
/// `Failed` ⇒ 失败原因原样上屏（fail-closed，⛔ 不静默退化成空列表、⛔ 不伪造请求）。
///
/// 来源标注随 [`ApprovalListView`] 携带、如实可查；按本卡纪律不改卡片视觉结构，
/// 不另加界面角标。
// C3 接线入口：Block 流切到 ApprovalListView 后由宿主调用，此前仅测试可达。
#[allow(dead_code)]
pub(super) fn render_approval_list(view: &ApprovalListView, ctx: &BlockCtx<'_>) -> AnyElement {
    let theme = ctx.theme;
    match view {
        ApprovalListView::Loading => div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child("加载中…")
            .into_any_element(),
        ApprovalListView::Ready { cards, .. } => {
            let mut list = div().flex().flex_col().gap_2().w_full();
            for card in cards {
                list = list.child(render_approval(card, ctx));
            }
            list.into_any_element()
        }
        ApprovalListView::Failed { reason } => div()
            .text_xs()
            .text_color(theme.warning)
            .child(reason.clone())
            .into_any_element(),
    }
}
