//! 右侧面板：任务清单 / 文件预览 / 审批待办三 tab，可折叠。
//!
//! 本模块只把版式状态（见 [`RightPanelLayout`]）与
//! [`SessionStreamState`] 转成渲染描述：
//! - **任务清单**本卡只有占位说明（数据源另卡接入）；
//! - **文件预览**只读消费会话流的**跳转意图**（[`FileIntent`]），显示「要预览哪个文件」；
//!   ⛔ **不读文件、不渲染文件内容**（预览器属 P3，见 `docs/ui-parity-spec.md` A14）；
//! - **审批待办**只读消费 `pending_approvals()`，渲染为审批卡片；
//!   渲染不会替用户决策。
//!
//! 折叠时面板只保留窄条与三个 tab 入口，不渲染 tab 内容。

use crate::state::file_intent::FileIntent;
use crate::state::session_stream::SessionStreamState;
use crate::ui::approval_card::{self, ApprovalCardView};
use crate::ui::layout::{RightPanelLayout, RightPanelTab};

/// 任务清单 tab 的占位文案。
pub const TASKS_PLACEHOLDER: &str = "任务清单占位：等待任务状态接入后显示待办与进度";
/// 文件预览 tab 的占位文案。
pub const PREVIEW_PLACEHOLDER: &str = "文件预览占位：选择文件后在此显示预览";
/// 审批待办为空时的文案。
pub const APPROVALS_EMPTY_TEXT: &str = "暂无待审批事项";
/// 预览 tab 在**没有**跳转意图时的文案（真实为空）。
pub const PREVIEW_EMPTY_TEXT: &str = "未选择文件";
/// 预览 tab 在**有**跳转意图、但预览器本体未实现时的说明文案。
pub const PREVIEW_UNSUPPORTED_TEXT: &str =
    "预览器尚未实现（属 P3）：当前只显示待预览文件，不渲染内容";

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

/// 预览内容：当前要预览的文件（来自会话流的跳转意图）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewContent {
    /// 预览器本体未实现（P3）：只显示「要预览哪个文件」，不渲染文件内容。
    Target { path: String },
}

/// 面板内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RightPanelContent {
    /// 折叠态：不渲染内容，只保留 tab 入口。
    Collapsed,
    /// 占位内容（任务清单 / 文件预览）。
    Placeholder { text: &'static str },
    /// 预览内容（当前要预览的文件）。
    Preview(PreviewContent),
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

/// 生成右侧面板渲染描述（**没有**预览意图）。
///
/// 等价于 [`render_with_preview`] 且 `preview = None`。
///
/// 保留这个两参入口是因为版式层（`ui/layout.rs`）也在调 `render`，
/// 而**版式层不在本卡的改动边界内** ⇒ 这里必须留一个兼容入口，
/// 真正的实现只有 [`render_with_preview`] 一处（不另写第二套判断）。
#[must_use]
pub fn render(layout: &RightPanelLayout, session: &SessionStreamState) -> RightPanelView {
    render_with_preview(layout, session, None)
}

/// 生成右侧面板渲染描述（含**预览意图**）。
///
/// `preview` 是会话流侧产生的**跳转意图**（谁点的、点的是哪个文件）：
/// - 有意图且当前 tab 是「文件预览」⇒ 内容为 [`RightPanelContent::Preview`]；
/// - 无意图 ⇒ 内容为 [`RightPanelContent::Empty`]（**不是**占位文案）。
///
/// `pending_approval_count` 与审批卡片都直接来自会话流状态；
/// 本函数不修改状态。
#[must_use]
pub fn render_with_preview(
    layout: &RightPanelLayout,
    session: &SessionStreamState,
    preview: Option<&FileIntent>,
) -> RightPanelView {
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
            RightPanelTab::Preview => match preview {
                Some(intent) => RightPanelContent::Preview(PreviewContent::Target {
                    path: intent.path.clone(),
                }),
                None => RightPanelContent::Empty {
                    text: PREVIEW_EMPTY_TEXT,
                },
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
    use crate::state::file_intent::FileIntentSource;
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

    fn preview_layout() -> RightPanelLayout {
        let mut layout = RightPanelLayout::new();
        layout.select_tab(RightPanelTab::Preview);
        layout
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
            RightPanelContent::Empty {
                text: PREVIEW_EMPTY_TEXT
            },
            "没有跳转意图时，预览 tab 是**空态**，不是占位文案"
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

    #[test]
    fn preview_intent_shows_the_target_file() {
        let intent = FileIntent::new("src/main.rs", FileIntentSource::FileCard);
        let view =
            render_with_preview(&preview_layout(), &SessionStreamState::new(), Some(&intent));
        assert_eq!(
            view.content,
            RightPanelContent::Preview(PreviewContent::Target {
                path: "src/main.rs".to_owned()
            }),
            "有跳转意图时，预览 tab 必须显示待预览文件"
        );
    }

    #[test]
    fn preview_without_intent_is_empty_not_placeholder() {
        let view = render_with_preview(&preview_layout(), &SessionStreamState::new(), None);
        assert_eq!(
            view.content,
            RightPanelContent::Empty {
                text: PREVIEW_EMPTY_TEXT
            },
            "没有意图时是空态，不是占位"
        );
    }

    #[test]
    fn preview_intent_does_not_affect_the_tasks_tab() {
        let intent = FileIntent::new("src/main.rs", FileIntentSource::FileList);
        let view = render_with_preview(
            &RightPanelLayout::new(),
            &SessionStreamState::new(),
            Some(&intent),
        );
        assert_eq!(
            view.content,
            RightPanelContent::Placeholder {
                text: TASKS_PLACEHOLDER
            },
            "任务清单 tab 的既有行为不得回归"
        );
    }

    #[test]
    fn approvals_tab_ignores_the_preview_intent() {
        let intent = FileIntent::new("src/main.rs", FileIntentSource::FileCard);
        let mut layout = RightPanelLayout::new();
        layout.select_tab(RightPanelTab::Approvals);
        let view = render_with_preview(&layout, &SessionStreamState::new(), Some(&intent));
        assert_eq!(
            view.content,
            RightPanelContent::Empty {
                text: APPROVALS_EMPTY_TEXT
            },
            "审批待办 tab 的既有行为不得回归"
        );
    }
}
