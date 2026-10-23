//! 右侧面板的 GPUI 接线层：把 [`crate::ui::right_panel`] 的渲染描述接到元素树。
//!
//! 分层约定（同 [`crate::ui::env_gpui`]）：
//! - **描述层**（`right_panel`）：纯数据、无 GPUI 依赖，可在无窗口环境测。
//! - **接线层**（本模块）：把描述变成 GPUI 元素，**不重新实现**任何业务判断。
//!
//! 文件划分：`tabs.rs`（tab 入口 / 折叠按钮 / 角标）、`approvals.rs`（审批卡片）、
//! `preview.rs`（文件预览内容）、`tests.rs`（单元测试）。
//!
//! 硬约束：
//! - **三个 tab 始终可见**：折叠态也保留 tab 入口，否则无法恢复
//!   （描述层的硬约束，接线层不得绕过）。
//! - **不伪造审批决策**：未决策的审批显示为「等待你的决策」；只有用户点击按钮
//!   才经 [`approval_card::decide`] 落决策，渲染本身绝不替用户放行。
//! - **预览器未实现**（P3，见 `docs/ui-parity-spec.md` A14）：本层只显示
//!   「要预览哪个文件」，不读文件、不渲染文件内容。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
#![cfg(feature = "ui")]

mod approvals;
mod preview;
mod preview_code;
mod preview_image;
mod preview_office;
mod preview_pdf;
mod preview_text;
mod tabs;
mod tasks;
#[cfg(test)]
mod tests;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::file_intent::FileIntent;
use crate::state::session_stream::SessionStreamState;
use crate::ui::approval_card::{ApprovalAction, ApprovalCardView};
use crate::ui::layout::RightPanelLayout;
use crate::ui::right_panel::{self, PreviewContent, RightPanelContent};

use tabs::{render_tab_button, render_toggle, tab_entries};

/// 折叠窄条里的展开按钮文案（窄条只有几十像素宽，放不下「展开」两个字）。
pub const EXPAND_LABEL: &str = "展";
/// 展开态的折叠按钮文案。
pub const COLLAPSE_LABEL: &str = "收起";

/// 右栏内产生、**尚未回写**到权威会话流的审批决策。
///
/// 右栏只持有会话流的**副本**，`approval_card::decide` 只改这份副本；
/// 真正的权威状态在会话流里。所以右栏把「用户点了什么」排进本队列，
/// 由宿主（`AppShell`）逐条取走并写入会话流（缺口 G1 的收敛）。
///
/// 与输入区的 `InputAreaGpui::take_pending` 同形：**队列归生产者所有**，
/// 宿主只读消费，不自建第二份队列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingDecision {
    /// 被决策的审批项 id。
    pub approval_id: String,
    /// 用户点的动作。
    pub action: ApprovalAction,
}

/// 右侧面板的 GPUI 视图。
///
/// 持有**版式状态**（[`RightPanelLayout`]）与**会话流状态**（[`SessionStreamState`]）的副本：
/// - 点击 tab / 折叠按钮只改版式（[`RightPanelLayout::select_tab`] /
///   [`RightPanelLayout::toggle_collapsed`]）；
/// - 点击审批按钮只改本视图持有的会话流副本（决策经 [`approval_card::decide`]），
///   宿主需用 [`Self::session_mut`] 把决策回写到共享状态（缺口见状态文档 G1）。
pub struct RightPanelGpui {
    layout: RightPanelLayout,
    session: SessionStreamState,
    /// 当前要预览的文件（由宿主从会话流的跳转意图下发）。
    preview_intent: Option<FileIntent>,
    /// 待回写的审批决策（右栏只记意图，回写由宿主负责）。
    decisions: Vec<PendingDecision>,
}

impl RightPanelGpui {
    /// 用版式状态与会话流状态构造视图。
    #[must_use]
    pub fn new(layout: RightPanelLayout, session: SessionStreamState) -> Self {
        Self {
            layout,
            session,
            preview_intent: None,
            decisions: Vec::new(),
        }
    }

    /// 从只读引用构造视图（内部克隆一份状态）。
    #[must_use]
    pub fn from_state(layout: &RightPanelLayout, session: &SessionStreamState) -> Self {
        Self::new(layout.clone(), session.clone())
    }

    /// 当前版式状态（只读）。
    #[must_use]
    pub fn layout(&self) -> &RightPanelLayout {
        &self.layout
    }

    /// 当前版式状态（可写）。
    pub fn layout_mut(&mut self) -> &mut RightPanelLayout {
        &mut self.layout
    }

    /// 当前会话流状态（只读）。
    #[must_use]
    pub fn session(&self) -> &SessionStreamState {
        &self.session
    }

    /// 当前会话流状态（可写）：宿主据此把面板内的审批决策回写到共享状态。
    pub fn session_mut(&mut self) -> &mut SessionStreamState {
        &mut self.session
    }

    /// 当前预览意图（只读）。
    #[must_use]
    pub fn preview_intent(&self) -> Option<&FileIntent> {
        self.preview_intent.as_ref()
    }

    /// 写入预览意图（宿主下发）。
    pub fn set_preview_intent(&mut self, intent: Option<FileIntent>) {
        self.preview_intent = intent;
    }

    /// 记一条待回写的审批决策（由 `approvals` 的点击处理调用）。
    pub fn push_decision(&mut self, approval_id: String, action: ApprovalAction) {
        self.decisions.push(PendingDecision {
            approval_id,
            action,
        });
    }

    /// 取走最早的一条待回写决策（宿主消费入口，FIFO）。
    pub fn take_decision(&mut self) -> Option<PendingDecision> {
        if self.decisions.is_empty() {
            return None;
        }
        Some(self.decisions.remove(0))
    }

    /// 待回写决策队列（只读，供调试与测试）。
    #[must_use]
    pub fn pending_decisions(&self) -> &[PendingDecision] {
        &self.decisions
    }
}

/// 面板内容的五个分支（与 [`RightPanelContent`] 一一对应）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentBranch<'a> {
    /// 折叠态：不渲染内容。
    Collapsed,
    /// 占位内容（任务清单）。
    Placeholder { text: &'a str },
    /// 预览内容（当前要预览的文件）。
    Preview { content: &'a PreviewContent },
    /// 待审批卡片（只含未决策项）。
    Approvals { cards: &'a [ApprovalCardView] },
    /// 空态（审批待办为空 / 未选择文件）。
    Empty { text: &'a str },
}

/// 把描述层的内容映射到五个渲染分支。
fn content_branch(content: &RightPanelContent) -> ContentBranch<'_> {
    match content {
        RightPanelContent::Collapsed => ContentBranch::Collapsed,
        RightPanelContent::Placeholder { text } => ContentBranch::Placeholder { text },
        RightPanelContent::Preview(content) => ContentBranch::Preview { content },
        RightPanelContent::Approvals(cards) => ContentBranch::Approvals { cards },
        RightPanelContent::Empty { text } => ContentBranch::Empty { text },
    }
}

/// 占位内容（任务清单）。
fn render_placeholder(text: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}

/// 空态内容（审批待办为空 / 未选择文件）。
fn render_empty(text: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}

impl Render for RightPanelGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let view = right_panel::render_with_preview(
            &self.layout,
            &self.session,
            self.preview_intent.as_ref(),
        );
        let entity = cx.entity();
        let entries = tab_entries(&view);

        // 折叠态：只保留窄条与三个 tab 入口，**不渲染内容**。
        // 三个 tab 仍然可见，否则用户无法从折叠态恢复。
        if view.collapsed {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .gap_1()
                .h_full()
                .w(px(view.width))
                .p_1()
                .bg(theme.background)
                .border_l_1()
                .border_color(theme.border)
                .child(render_toggle(true, &entity, theme))
                .children(
                    entries
                        .iter()
                        .map(|entry| render_tab_button(entry, true, &entity, theme)),
                );
        }

        // 展开态：标题行（当前 tab）+ tab 行 + 内容区。
        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .pt_2()
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(view.active_tab.label()),
            )
            .child(render_toggle(false, &entity, theme));

        let tab_bar = div()
            .flex()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(theme.border)
            .children(
                entries
                    .iter()
                    .map(|entry| render_tab_button(entry, false, &entity, theme)),
            );

        // 五种内容分支：外层是同一个滚动容器，内层按分支给内容。
        let body: AnyElement = match content_branch(&view.content) {
            ContentBranch::Collapsed => div().into_any_element(),
            ContentBranch::Placeholder { text } => {
                render_placeholder(text, theme).into_any_element()
            }
            ContentBranch::Preview { content } => {
                preview::render_preview(content, theme).into_any_element()
            }
            ContentBranch::Approvals { cards } => div()
                .flex()
                .flex_col()
                .gap_2()
                .children(
                    cards
                        .iter()
                        .map(|card| approvals::render_approval_card(card, &entity, theme)),
                )
                .into_any_element(),
            ContentBranch::Empty { text } => div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(render_empty(text, theme))
                .into_any_element(),
        };

        let content = div()
            .id("right-panel-content")
            .flex()
            .flex_col()
            .flex_1()
            .overflow_y_scroll()
            .p_2()
            .child(body);

        div()
            .flex()
            .flex_col()
            .h_full()
            .w(px(view.width))
            .bg(theme.background)
            .text_color(theme.foreground)
            .border_l_1()
            .border_color(theme.border)
            .child(header)
            .child(tab_bar)
            .child(content)
    }
}
