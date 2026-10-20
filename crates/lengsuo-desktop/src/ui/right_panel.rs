//! 右侧面板：任务清单 / 文件预览 / 审批待办三 tab，可折叠。
//!
//! 本模块只把版式状态（见 [`RightPanelLayout`]）与
//! [`SessionStreamState`] 转成渲染描述：
//! - **任务清单**与**文件预览**本卡只有占位说明（数据源另卡接入）；
//! - **审批待办**只读消费 `pending_approvals()`，渲染为审批卡片；
//!   渲染不会替用户决策。
//!
//! 折叠时面板只保留窄条与三个 tab 入口，不渲染 tab 内容。

use crate::state::session_stream::SessionStreamState;
use crate::ui::approval_card::{self, ApprovalCardView};
use crate::ui::layout::{RightPanelLayout, RightPanelTab};

/// 任务清单 tab 的占位文案。
pub const TASKS_PLACEHOLDER: &str = "任务清单占位：等待任务状态接入后显示待办与进度";
/// 文件预览 tab 的占位文案。
pub const PREVIEW_PLACEHOLDER: &str = "文件预览占位：选择文件后在此显示预览";
/// 审批待办为空时的文案。
pub const APPROVALS_EMPTY_TEXT: &str = "暂无待审批事项";

/// tab 的占位文案；审批待办由状态驱动，没有占位。
#[must_use]
pub const fn tab_placeholder(tab: RightPanelTab) -> Option<&'static str> {
    match tab {
        RightPanelTab::Tasks => Some(TASKS_PLACEHOLDER),
        RightPanelTab::Preview => Some(PREVIEW_PLACEHOLDER),
        RightPanelTab::Approvals => None,
    }
}

/// 一个 tab 按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabButton {
    pub tab: RightPanelTab,
    pub label: &'static str,
    pub selected: bool,
    /// 角标数量；审批待办在有待办项时显示待审批数。
    pub badge: Option<usize>,
}

/// 面板内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RightPanelContent {
    /// 折叠态：不渲染内容，只保留 tab 入口。
    Collapsed,
    /// 占位内容（任务清单 / 文件预览）。
    Placeholder { text: &'static str },
    /// 待审批卡片（只含未决策项）。
    Approvals(Vec<ApprovalCardView>),
    /// 审批待办为空。
    Empty { text: &'static str },
}

/// 右侧面板渲染描述。
#[derive(Debug, Clone, PartialEq)]
pub struct RightPanelView {
    /// 实际占位宽度（折叠时为窄条宽度）。
    pub width: f32,
    pub collapsed: bool,
    pub active_tab: RightPanelTab,
    /// 三个 tab 始终存在，折叠态也不隐藏（否则无法恢复）。
    pub tabs: Vec<TabButton>,
    pub content: RightPanelContent,
    /// 未决策审批数，供外层画全局角标。
    pub pending_approval_count: usize,
}

/// 生成右侧面板渲染描述。
///
/// `pending_approval_count` 与审批卡片都直接来自会话流状态；
/// 本函数不修改状态。
#[must_use]
pub fn render(layout: &RightPanelLayout, session: &SessionStreamState) -> RightPanelView {
    let pending_approval_count = session.pending_approvals().count();
    let active_tab = layout.tab();
    let tabs = RightPanelTab::ALL
        .iter()
        .map(|&tab| TabButton {
            tab,
            label: tab.label(),
            selected: tab == active_tab,
            badge: (tab == RightPanelTab::Approvals && pending_approval_count > 0)
                .then_some(pending_approval_count),
        })
        .collect();

    let content = if layout.is_collapsed() {
        RightPanelContent::Collapsed
    } else {
        match active_tab {
            RightPanelTab::Tasks => RightPanelContent::Placeholder {
                text: TASKS_PLACEHOLDER,
            },
            RightPanelTab::Preview => RightPanelContent::Placeholder {
                text: PREVIEW_PLACEHOLDER,
            },
            RightPanelTab::Approvals => {
                let cards = approval_card::pending_cards(session);
                if cards.is_empty() {
                    RightPanelContent::Empty {
                        text: APPROVALS_EMPTY_TEXT,
                    }
                } else {
                    RightPanelContent::Approvals(cards)
                }
            }
        }
    };

    RightPanelView {
        width: layout.effective_width(),
        collapsed: layout.is_collapsed(),
        active_tab,
        tabs,
        content,
        pending_approval_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::session_stream::{PendingApproval, SessionMessage};

    fn session_with_approval() -> SessionStreamState {
        let mut session = SessionStreamState::new();
        assert!(session.push_message(
            1,
            SessionMessage {
                role: crate::state::session_stream::MessageRole::User,
                content: "请执行".into(),
                tool: None,
                ts_ms: 0,
            }
        ));
        session.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "Bash".into(),
            summary: "rm -rf build".into(),
            resolved: None,
        });
        session
    }

    #[test]
    fn three_tabs_are_always_present() {
        let view = render(&RightPanelLayout::new(), &SessionStreamState::new());
        let labels: Vec<&str> = view.tabs.iter().map(|tab| tab.label).collect();
        assert_eq!(labels, ["任务清单", "文件预览", "审批待办"]);
        assert_eq!(view.tabs.len(), 3);
        assert_eq!(view.active_tab, RightPanelTab::Tasks);
        assert!(view.tabs[0].selected);
        assert!(view.tabs[1..].iter().all(|tab| !tab.selected));
        assert_eq!(view.width, crate::ui::layout::RIGHT_PANEL_DEFAULT_WIDTH);
        assert!(!view.collapsed);
    }

    #[test]
    fn tasks_and_preview_are_explicit_placeholders() {
        let mut layout = RightPanelLayout::new();
        assert_eq!(
            tab_placeholder(RightPanelTab::Tasks),
            Some(TASKS_PLACEHOLDER)
        );
        assert_eq!(
            tab_placeholder(RightPanelTab::Preview),
            Some(PREVIEW_PLACEHOLDER)
        );
        assert_eq!(tab_placeholder(RightPanelTab::Approvals), None);

        layout.select_tab(RightPanelTab::Preview);
        let view = render(&layout, &SessionStreamState::new());
        assert_eq!(
            view.content,
            RightPanelContent::Placeholder {
                text: PREVIEW_PLACEHOLDER
            }
        );
        assert!(view.tabs[1].selected);
    }

    #[test]
    fn approvals_tab_renders_pending_cards_with_badge() {
        let mut layout = RightPanelLayout::new();
        layout.select_tab(RightPanelTab::Approvals);
        let session = session_with_approval();
        let view = render(&layout, &session);

        assert_eq!(view.pending_approval_count, 1);
        assert_eq!(view.tabs[2].badge, Some(1));
        assert_eq!(view.tabs[0].badge, None);
        match view.content {
            RightPanelContent::Approvals(cards) => {
                assert_eq!(cards.len(), 1);
                assert!(cards[0].pending);
                assert!(cards[0].buttons.iter().all(|button| button.enabled));
            }
            other => panic!("期望审批卡片，得到 {other:?}"),
        }
        assert_eq!(
            session.pending_approvals().count(),
            1,
            "只读渲染不得替用户决策"
        );
    }

    #[test]
    fn approvals_tab_without_pending_has_empty_text() {
        let mut layout = RightPanelLayout::new();
        layout.select_tab(RightPanelTab::Approvals);
        let view = render(&layout, &SessionStreamState::new());
        assert_eq!(
            view.content,
            RightPanelContent::Empty {
                text: APPROVALS_EMPTY_TEXT
            }
        );
        assert_eq!(view.tabs[2].badge, None);
    }

    #[test]
    fn decided_approvals_are_not_shown() {
        let mut session = session_with_approval();
        assert!(session.resolve_approval("a1", true));
        let mut layout = RightPanelLayout::new();
        layout.select_tab(RightPanelTab::Approvals);
        let view = render(&layout, &session);
        assert_eq!(
            view.content,
            RightPanelContent::Empty {
                text: APPROVALS_EMPTY_TEXT
            }
        );
        assert_eq!(view.pending_approval_count, 0);
    }

    #[test]
    fn collapsed_panel_keeps_tab_entries_but_hides_content() {
        let mut layout = RightPanelLayout::new();
        layout.collapse();
        let view = render(&layout, &session_with_approval());
        assert!(view.collapsed);
        assert_eq!(view.width, crate::ui::layout::RIGHT_PANEL_COLLAPSED_WIDTH);
        assert_eq!(view.content, RightPanelContent::Collapsed);
        assert_eq!(view.tabs.len(), 3, "折叠后仍须保留三个 tab 入口");
        assert_eq!(view.pending_approval_count, 1, "折叠不吞审批角标");
    }
}
