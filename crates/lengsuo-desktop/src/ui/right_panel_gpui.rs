//! 右侧面板的 GPUI 接线层：把 [`crate::ui::right_panel`] 的渲染描述接到元素树。
//!
//! 分层约定（同 [`crate::ui::env_gpui`]）：
//! - **描述层**（`right_panel`）：纯数据、无 GPUI 依赖，可在无窗口环境测。
//! - **接线层**（本文件）：把描述变成 GPUI 元素，**不重新实现**任何业务判断。
//!
//! 硬约束：
//! - **三个 tab 始终可见**：折叠态也保留 tab 入口，否则无法恢复
//!   （描述层的硬约束，接线层不得绕过）。
//! - **不伪造审批决策**：未决策的审批显示为「等待你的决策」；只有用户点击按钮
//!   才经 [`approval_card::decide`] 落决策，渲染本身绝不替用户放行。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use crate::state::session_stream::SessionStreamState;
use crate::ui::approval_card::{self, ApprovalAction, ApprovalButton, ApprovalCardView};
use crate::ui::layout::{RightPanelLayout, RightPanelTab};
use crate::ui::right_panel::{self, RightPanelContent, RightPanelView};

/// 折叠窄条里的展开按钮文案（窄条只有几十像素宽，放不下「展开」两个字）。
pub const EXPAND_LABEL: &str = "展";
/// 展开态的折叠按钮文案。
pub const COLLAPSE_LABEL: &str = "收起";

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
}

impl RightPanelGpui {
    /// 用版式状态与会话流状态构造视图。
    #[must_use]
    pub fn new(layout: RightPanelLayout, session: SessionStreamState) -> Self {
        Self { layout, session }
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
}

/// 一个 tab 入口的展示信息（纯数据，可在无窗口环境测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabEntry {
    /// tab 身份（点击时回写版式状态）。
    pub tab: RightPanelTab,
    /// 完整标签（展开态用）。
    pub label: String,
    /// 紧凑标签（折叠窄条用）：取描述层标签的首字。
    pub compact: String,
    /// 是否选中。
    pub selected: bool,
    /// 角标数量；`None` 表示不显示角标。
    pub badge: Option<usize>,
}

/// 面板内容的四个分支（与 [`RightPanelContent`] 一一对应）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentBranch<'a> {
    /// 折叠态：不渲染内容。
    Collapsed,
    /// 占位内容（任务清单 / 文件预览）。
    Placeholder { text: &'a str },
    /// 待审批卡片（只含未决策项）。
    Approvals { cards: &'a [ApprovalCardView] },
    /// 审批待办为空。
    Empty { text: &'a str },
}

/// 折叠窄条里的紧凑标签：取描述层标签的首字。
///
/// 这样窄条里仍然能分辨三个 tab，且不会另造一套标签文案
/// —— 文案只有一个来源（描述层）。
fn compact_label(label: &str) -> String {
    label.chars().next().map(String::from).unwrap_or_default()
}

/// 把描述层的 tab 列表转成展示条目。
fn tab_entries(view: &RightPanelView) -> Vec<TabEntry> {
    view.tabs
        .iter()
        .map(|tab| TabEntry {
            tab: tab.tab,
            label: tab.label.to_owned(),
            compact: compact_label(tab.label),
            selected: tab.selected,
            badge: tab.badge,
        })
        .collect()
}

/// 把描述层的内容映射到四个渲染分支。
fn content_branch(content: &RightPanelContent) -> ContentBranch<'_> {
    match content {
        RightPanelContent::Collapsed => ContentBranch::Collapsed,
        RightPanelContent::Placeholder { text } => ContentBranch::Placeholder { text },
        RightPanelContent::Approvals(cards) => ContentBranch::Approvals { cards },
        RightPanelContent::Empty { text } => ContentBranch::Empty { text },
    }
}

/// tab 上的角标数字。
fn render_badge(count: usize, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .rounded_full()
        .px_1()
        .text_xs()
        .text_color(theme.button_warning_foreground)
        .bg(theme.button_warning)
        .child(count.to_string())
}

/// 一个 tab 入口。
///
/// `collapsed` 为真时只显示紧凑标签（窄条），否则显示完整标签。
/// 点击切换 tab：描述层规定折叠态选 tab 会展开面板。
fn render_tab_button(
    entry: &TabEntry,
    collapsed: bool,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let label_color = if entry.selected {
        theme.accent_foreground
    } else {
        theme.muted_foreground
    };
    let label = if collapsed {
        entry.compact.clone()
    } else {
        entry.label.clone()
    };

    let mut button = div()
        .id(format!("right-panel-tab-{}", entry.tab.id()))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .cursor_pointer();
    if collapsed {
        button = button.w_full();
    } else {
        button = button.flex_1();
    }
    if entry.selected {
        button = button.bg(theme.accent);
    }
    let mut button = button.child(div().text_sm().text_color(label_color).child(label));
    if let Some(count) = entry.badge {
        button = button.child(render_badge(count, theme));
    }

    let entity = entity.clone();
    let tab = entry.tab;
    button.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            view.layout.select_tab(tab);
            cx.notify();
        });
    })
}

/// 折叠 / 展开按钮。
fn render_toggle(
    collapsed: bool,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let label = if collapsed {
        EXPAND_LABEL
    } else {
        COLLAPSE_LABEL
    };
    let mut button = div()
        .id("right-panel-toggle")
        .px_2()
        .py_1()
        .rounded_sm()
        .text_xs()
        .border_1()
        .border_color(theme.border)
        .cursor_pointer();
    button = if collapsed {
        button.text_color(theme.muted_foreground)
    } else {
        button.text_color(theme.foreground)
    };
    let button = button.child(label);

    let entity = entity.clone();
    button.on_click(move |_event, _window, app| {
        entity.update(app, |view, cx| {
            view.layout.toggle_collapsed();
            cx.notify();
        });
    })
}

/// 占位内容（任务清单 / 文件预览）。
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

/// 空态内容（审批待办为空）。
fn render_empty(text: &str, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}

/// 一个审批按钮。
///
/// 只有 `enabled` 的按钮才挂点击处理；点击经 [`approval_card::decide`] 落决策，
/// 接线层自己**不**判定「允许」。
fn render_approval_button(
    card: &ApprovalCardView,
    button: &ApprovalButton,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let enabled = button.enabled;
    let (background, foreground) = match button.action {
        ApprovalAction::Allow => (theme.button_success, theme.button_success_foreground),
        ApprovalAction::Deny => (theme.button_danger, theme.button_danger_foreground),
        ApprovalAction::AlwaysAllow => (theme.button_warning, theme.button_warning_foreground),
    };
    let mut element = div()
        .id(format!(
            "right-panel-approval-{}-{}",
            card.id,
            button.action.label()
        ))
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_xs();
    element = if enabled {
        element
            .bg(background)
            .text_color(foreground)
            .cursor_pointer()
    } else {
        element.text_color(theme.muted_foreground)
    };
    let element = element.child(button.label);

    if enabled {
        let entity = entity.clone();
        let id = card.id.clone();
        let action = button.action;
        element.on_click(move |_event, _window, app| {
            entity.update(app, |view, cx| {
                approval_card::decide(&mut view.session, &id, action);
                cx.notify();
            });
        })
    } else {
        element
    }
}

/// 一张审批卡片。
fn render_approval_card(
    card: &ApprovalCardView,
    entity: &Entity<RightPanelGpui>,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let (status_color, status_text) = if card.pending {
        (theme.button_warning, "等待你的决策")
    } else if card.resolved_allow == Some(true) {
        (theme.button_success, "已放行")
    } else {
        (theme.button_danger, "已拒绝")
    };
    let buttons = div().flex().flex_wrap().gap_2().children(
        card.buttons
            .iter()
            .map(|button| render_approval_button(card, button, entity, theme)),
    );

    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.background)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(card.title.clone()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(card.summary.clone()),
        )
        .child(div().text_xs().text_color(status_color).child(status_text))
        .child(buttons)
}

impl Render for RightPanelGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let view = right_panel::render(&self.layout, &self.session);
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

        // 四种内容分支：外层是同一个滚动容器，内层按分支给内容。
        let body: AnyElement = match content_branch(&view.content) {
            ContentBranch::Collapsed => div().into_any_element(),
            ContentBranch::Placeholder { text } => {
                render_placeholder(text, theme).into_any_element()
            }
            ContentBranch::Approvals { cards } => div()
                .flex()
                .flex_col()
                .gap_2()
                .children(
                    cards
                        .iter()
                        .map(|card| render_approval_card(card, &entity, theme)),
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

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 宏带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
    use super::{content_branch, tab_entries, ContentBranch, RightPanelGpui};
    use crate::state::session_stream::{PendingApproval, SessionStreamState};
    use crate::ui::approval_card::{self, ApprovalAction};
    use crate::ui::layout::{RightPanelLayout, RightPanelTab};
    use crate::ui::right_panel::{self, RightPanelContent};

    fn session_with_approval() -> SessionStreamState {
        let mut session = SessionStreamState::new();
        session.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "Bash".into(),
            summary: "rm -rf build".into(),
            resolved: None,
        });
        session
    }

    fn approvals_layout() -> RightPanelLayout {
        let mut layout = RightPanelLayout::new();
        layout.select_tab(RightPanelTab::Approvals);
        layout
    }

    #[test]
    fn collapsed_view_still_exposes_three_tab_entries() {
        let mut layout = RightPanelLayout::new();
        layout.collapse();
        let view = right_panel::render(&layout, &session_with_approval());
        let entries = tab_entries(&view);

        assert_eq!(entries.len(), 3, "折叠后仍须保留三个 tab 入口");
        let labels: Vec<&str> = entries.iter().map(|entry| entry.label.as_str()).collect();
        assert_eq!(labels, ["任务清单", "文件预览", "审批待办"]);
    }

    #[test]
    fn compact_labels_come_from_the_description_layer() {
        let view = right_panel::render(&RightPanelLayout::new(), &SessionStreamState::new());
        let entries = tab_entries(&view);
        let compact: Vec<&str> = entries.iter().map(|entry| entry.compact.as_str()).collect();
        assert_eq!(compact, ["任", "文", "审"]);
        for entry in &entries {
            assert!(
                entry.label.starts_with(entry.compact.as_str()),
                "紧凑标签必须来自描述层标签"
            );
        }
    }

    #[test]
    fn badge_follows_pending_approval_count() {
        let view = right_panel::render(&approvals_layout(), &session_with_approval());
        let entries = tab_entries(&view);
        assert_eq!(entries[2].badge, Some(1));
        assert_eq!(entries[0].badge, None);
        assert_eq!(entries[1].badge, None);
    }

    #[test]
    fn content_branch_covers_all_four_variants() {
        let placeholder = RightPanelContent::Placeholder { text: "占位" };
        assert_eq!(
            content_branch(&placeholder),
            ContentBranch::Placeholder { text: "占位" }
        );

        let empty = RightPanelContent::Empty { text: "暂无" };
        assert_eq!(
            content_branch(&empty),
            ContentBranch::Empty { text: "暂无" }
        );

        assert_eq!(
            content_branch(&RightPanelContent::Collapsed),
            ContentBranch::Collapsed
        );

        let cards = approval_card::pending_cards(&session_with_approval());
        let approvals = RightPanelContent::Approvals(cards);
        match content_branch(&approvals) {
            ContentBranch::Approvals { cards } => {
                assert_eq!(cards.len(), 1);
                assert!(cards[0].pending, "未决策即等待中");
                assert!(
                    cards[0].buttons.iter().all(|button| button.enabled),
                    "未决策时按钮可用，等待用户操作"
                );
            }
            other => panic!("期望审批分支，得到 {other:?}"),
        }
    }

    #[test]
    fn rendering_never_decides_on_behalf_of_the_user() {
        let session = session_with_approval();
        let view = right_panel::render(&approvals_layout(), &session);
        assert_eq!(view.pending_approval_count, 1);
        assert_eq!(
            session.pending_approvals().count(),
            1,
            "只读渲染不得替用户决策"
        );
    }

    #[test]
    fn deciding_inside_the_panel_updates_only_the_local_copy() {
        let mut panel = RightPanelGpui::from_state(&approvals_layout(), &session_with_approval());
        assert!(approval_card::decide(
            &mut panel.session_mut(),
            "a1",
            ApprovalAction::Allow
        ));
        assert_eq!(panel.session().pending_approvals().count(), 0);
        assert_eq!(
            panel.session().approvals()[0].resolved,
            Some(true),
            "决策必须真实落在状态里，不得伪造"
        );
    }
}
