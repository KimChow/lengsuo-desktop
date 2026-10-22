//! 右侧面板接线层的单元测试（无窗口可测）。
//!
//! ⛔ 不包一层 `mod tests {}`（clippy `module_inception` ⇒ `-D warnings` 整卡失败）；
//! ⛔ 不用 `use super::*`（父模块的 glob 会把 GPUI 的 `test` 宏带进作用域并遮蔽
//! 标准库 `#[test]`，见速查表 §12.3）⇒ 显式列出需要的符号。
#![cfg(test)]

use crate::state::file_intent::{FileIntent, FileIntentSource};
use crate::state::session_stream::{PendingApproval, SessionStreamState};
use crate::ui::approval_card::{self, ApprovalAction};
use crate::ui::layout::{RightPanelLayout, RightPanelTab};
use crate::ui::right_panel::{self, PreviewContent, RightPanelContent};

use crate::ui::right_panel_gpui::tabs::tab_entries;
use crate::ui::right_panel_gpui::{content_branch, ContentBranch, RightPanelGpui};

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
fn content_branch_covers_all_five_variants() {
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

    let preview = RightPanelContent::Preview(PreviewContent::Target {
        path: "src/main.rs".to_owned(),
    });
    assert_eq!(
        content_branch(&preview),
        ContentBranch::Preview {
            content: &PreviewContent::Target {
                path: "src/main.rs".to_owned()
            }
        }
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
        panel.session_mut(),
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

#[test]
fn preview_intent_round_trips_through_the_panel() {
    let mut panel =
        RightPanelGpui::from_state(&RightPanelLayout::new(), &SessionStreamState::new());
    assert_eq!(panel.preview_intent(), None, "默认没有预览意图");

    let intent = FileIntent::new("src/main.rs", FileIntentSource::FileCard);
    panel.set_preview_intent(Some(intent.clone()));
    assert_eq!(panel.preview_intent(), Some(&intent));

    panel.set_preview_intent(None);
    assert_eq!(panel.preview_intent(), None);
}

#[test]
fn push_take_decision_is_fifo() {
    let mut panel = RightPanelGpui::from_state(&approvals_layout(), &session_with_approval());
    panel.push_decision("a1".into(), ApprovalAction::Allow);
    panel.push_decision("a2".into(), ApprovalAction::Deny);

    let first = panel.take_decision().expect("应取出第一条");
    assert_eq!(first.approval_id, "a1");
    assert_eq!(first.action, ApprovalAction::Allow);

    let second = panel.take_decision().expect("应取出第二条");
    assert_eq!(second.approval_id, "a2");
    assert_eq!(second.action, ApprovalAction::Deny);

    assert_eq!(panel.take_decision(), None, "队列取空后应返回 None");
}

#[test]
fn take_decision_on_empty_queue_returns_none() {
    let mut panel = RightPanelGpui::from_state(&approvals_layout(), &session_with_approval());
    assert_eq!(panel.take_decision(), None);
    assert!(panel.pending_decisions().is_empty());
}

#[test]
fn push_decision_does_not_touch_preview_intent() {
    let mut panel = RightPanelGpui::from_state(&approvals_layout(), &session_with_approval());
    let intent = FileIntent::new("src/main.rs", FileIntentSource::FileCard);
    panel.set_preview_intent(Some(intent.clone()));

    panel.push_decision("a1".into(), ApprovalAction::Allow);

    assert_eq!(
        panel.preview_intent(),
        Some(&intent),
        "记决策不得影响预览意图（防 e29 回归）"
    );
}
