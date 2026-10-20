//! 审批卡片（弹窗式）：允许 / 拒绝 / 总是允许。
//!
//! 三条硬约定：
//! - **默认挂起**：未决策前卡片显示为等待中，渲染本身绝不替用户放行。
//! - 决策只经 [`SessionStreamState::resolve_approval`]，已决策项二次操作返回 `false`。
//! - 卡片标题沿用 [`crate::theme::tool_titles`] 的人话映射，不暴露内部工具名。
//!
//! 「总是允许」的**持久化**属于运行时职责：界面只上报该选择（见
//! [`ApprovalAction::is_always`]），不自行落盘、不扩大授权。

use crate::state::session_stream::{PendingApproval, SessionStreamState};
use crate::theme::tool_titles::tool_title;

/// 审批操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalAction {
    Allow,
    Deny,
    AlwaysAllow,
}

impl ApprovalAction {
    /// 卡片上的三个操作（顺序即按钮顺序）。
    pub const ALL: [Self; 3] = [Self::Allow, Self::Deny, Self::AlwaysAllow];

    /// 按钮文案。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Allow => "允许",
            Self::Deny => "拒绝",
            Self::AlwaysAllow => "总是允许",
        }
    }

    /// 是否放行（决策结果写入 `resolved`）。
    #[must_use]
    pub fn allows(self) -> bool {
        !matches!(self, Self::Deny)
    }

    /// 是否为「总是允许」。运行时据此决定是否持久化该策略。
    #[must_use]
    pub fn is_always(self) -> bool {
        matches!(self, Self::AlwaysAllow)
    }
}

/// 一个审批按钮。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalButton {
    pub action: ApprovalAction,
    pub label: &'static str,
    /// 已决策后为 `false`：已决策项不得再点击。
    pub enabled: bool,
}

/// 审批卡片的渲染描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalCardView {
    pub id: String,
    /// 人话标题（如「执行命令」），不含内部工具名。
    pub title: String,
    /// 审批摘要（来自事件 payload，已是用户可读文本）。
    pub summary: String,
    /// 是否仍在等待用户决策。
    pub pending: bool,
    /// 已决策结果：`None` 表示挂起。
    pub resolved_allow: Option<bool>,
    pub buttons: Vec<ApprovalButton>,
}

impl ApprovalCardView {
    /// 从状态层审批项构建卡片；按钮可用性由 `resolved` 决定。
    #[must_use]
    pub fn from_approval(approval: &PendingApproval) -> Self {
        let pending = approval.resolved.is_none();
        Self {
            id: approval.id.clone(),
            title: tool_title(&approval.tool, None),
            summary: approval.summary.clone(),
            pending,
            resolved_allow: approval.resolved,
            buttons: ApprovalAction::ALL
                .iter()
                .map(|&action| ApprovalButton {
                    action,
                    label: action.label(),
                    enabled: pending,
                })
                .collect(),
        }
    }
}

/// 待审批卡片：只包含**未决策**项。
#[must_use]
pub fn pending_cards(state: &SessionStreamState) -> Vec<ApprovalCardView> {
    state
        .pending_approvals()
        .map(ApprovalCardView::from_approval)
        .collect()
}

/// 全部审批卡片（含已决策项，按钮全部禁用），用于历史回看。
#[must_use]
pub fn all_cards(state: &SessionStreamState) -> Vec<ApprovalCardView> {
    state
        .approvals()
        .iter()
        .map(ApprovalCardView::from_approval)
        .collect()
}

/// 提交一次审批决策。
///
/// 返回 `false` 表示被拒绝：项不存在、已决策（幂等保护），或校验失败。
/// 已决策项绝不会因重复点击被改成相反结果。
pub fn decide(state: &mut SessionStreamState, id: &str, action: ApprovalAction) -> bool {
    state.resolve_approval(id, action.allows())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approval(id: &str, tool: &str) -> PendingApproval {
        PendingApproval {
            id: id.into(),
            tool: tool.into(),
            summary: "rm -rf build".into(),
            resolved: None,
        }
    }

    #[test]
    fn pending_card_has_three_actions_and_no_default_release() {
        let state = SessionStreamState::new();
        let card = ApprovalCardView::from_approval(&approval("a1", "exec"));
        assert!(card.pending);
        assert_eq!(card.resolved_allow, None, "渲染不得默认放行");
        assert_eq!(card.buttons.len(), 3);
        let labels: Vec<&str> = card.buttons.iter().map(|b| b.label).collect();
        assert_eq!(labels, ["允许", "拒绝", "总是允许"]);
        assert!(card.buttons.iter().all(|b| b.enabled));
        let _ = state; // 渲染只读，不触碰状态
    }

    #[test]
    fn action_semantics() {
        assert!(ApprovalAction::Allow.allows());
        assert!(!ApprovalAction::Deny.allows());
        assert!(ApprovalAction::AlwaysAllow.allows());
        assert!(ApprovalAction::AlwaysAllow.is_always());
        assert!(!ApprovalAction::Allow.is_always());
    }

    #[test]
    fn decide_records_result_and_removes_pending_card() {
        let mut state = SessionStreamState::new();
        state.push_approval(approval("a1", "Bash"));
        assert_eq!(pending_cards(&state).len(), 1);

        assert!(decide(&mut state, "a1", ApprovalAction::Deny));
        assert!(
            pending_cards(&state).is_empty(),
            "已决策项不再出现在待审批列表"
        );
        assert_eq!(state.pending_approvals().count(), 0);

        let cards = all_cards(&state);
        assert_eq!(cards.len(), 1);
        assert!(!cards[0].pending);
        assert_eq!(cards[0].resolved_allow, Some(false));
        assert!(
            cards[0].buttons.iter().all(|b| !b.enabled),
            "已决策项的按钮必须全部禁用"
        );
    }

    #[test]
    fn resolved_item_cannot_be_decided_again() {
        let mut state = SessionStreamState::new();
        state.push_approval(approval("a1", "Bash"));
        assert!(decide(&mut state, "a1", ApprovalAction::Allow));
        assert!(
            !decide(&mut state, "a1", ApprovalAction::Deny),
            "已放行项不得被二次改成拒绝（反之亦然）"
        );
        assert_eq!(state.approvals()[0].resolved, Some(true));
    }

    #[test]
    fn always_allow_maps_to_allow_without_persisting_here() {
        let mut state = SessionStreamState::new();
        state.push_approval(approval("a1", "Bash"));
        assert!(decide(&mut state, "a1", ApprovalAction::AlwaysAllow));
        assert_eq!(state.approvals()[0].resolved, Some(true));
    }

    #[test]
    fn unknown_tool_does_not_leak_in_approval_title() {
        let card = ApprovalCardView::from_approval(&approval("a1", "SecretTool"));
        assert!(!card.title.contains("SecretTool"));
        assert_eq!(card.title, "执行操作");
    }
}
