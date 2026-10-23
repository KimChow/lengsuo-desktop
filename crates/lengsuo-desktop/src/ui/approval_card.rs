//! 审批卡片（弹窗式）：允许 / 拒绝 / 总是允许。
//!
//! 三条硬约定：
//! - **默认挂起**：未决策前卡片显示为等待中，渲染本身绝不替用户放行。
//! - 决策只经 [`SessionStreamState::resolve_approval`]，已决策项二次操作返回 `false`。
//! - 卡片标题沿用 [`crate::theme::tool_titles`] 的人话映射，不暴露内部工具名。
//!
//! 「总是允许」的**持久化**属于运行时职责：界面只上报该选择（见
//! [`ApprovalAction::is_always`]），不自行落盘、不扩大授权。
//!
//! 数据来源（C3）：卡片从消费侧契约 [`crate::state::datasource`] 注入
//! （[`ApprovalListView::from_requests`]）；本地演示出口 [`demo_list`] 是该入口的
//! 薄封装，来源如实标注 [`Provenance::Demo`]。加载失败必须以错误文案上屏
//! （fail-closed），不许静默退化成空列表。

use crate::state::datasource::{
    ApprovalDecision, ApprovalDecisionKind, ApprovalRequest, LoadState, Provenance,
};
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
    /// 从 C3 消费侧审批请求 `{id, action, detail}` 构建卡片（契约形状的唯一构造路径）。
    ///
    /// `action` 为自由文本（取值域契约待定）：按工具名走 [`tool_title`] 的人话映射，
    /// 未知值回退通用标题，不泄漏内部名。新卡一律挂起，渲染不默认放行。
    #[must_use]
    pub fn from_request(request: &ApprovalRequest) -> Self {
        Self {
            id: request.id.clone(),
            title: tool_title(&request.action, None),
            summary: request.detail.clone(),
            pending: true,
            resolved_allow: None,
            buttons: ApprovalAction::ALL
                .iter()
                .map(|&action| ApprovalButton {
                    action,
                    label: action.label(),
                    enabled: true,
                })
                .collect(),
        }
    }

    /// 演示出口（薄封装）：本地审批项转成 C3 形状后走 [`Self::from_request`]，
    /// 再并回已决策结果；按钮可用性由 `resolved` 决定。
    #[must_use]
    pub fn from_approval(approval: &PendingApproval) -> Self {
        let mut card = Self::from_request(&ApprovalRequest {
            id: approval.id.clone(),
            action: approval.tool.clone(),
            detail: approval.summary.clone(),
        });
        card.apply_result(approval.resolved);
        card
    }

    /// 并回决策结果：只改卡片展示态（挂起标记、结果标注、按钮可用性）。
    /// 是否放行由运行时决定，界面不判定权限。
    fn apply_result(&mut self, resolved: Option<bool>) {
        self.pending = resolved.is_none();
        self.resolved_allow = resolved;
        let enabled = self.pending;
        for button in &mut self.buttons {
            button.enabled = enabled;
        }
    }
}

/// C3 决定 → 卡片动作（三值穷举；不存在第四种决定，新增变体会在此编译失败）。
#[must_use]
pub fn action_of_decision(decision: ApprovalDecisionKind) -> ApprovalAction {
    match decision {
        ApprovalDecisionKind::Allow => ApprovalAction::Allow,
        ApprovalDecisionKind::Deny => ApprovalAction::Deny,
        ApprovalDecisionKind::Always => ApprovalAction::AlwaysAllow,
    }
}

/// 卡片动作 → C3 决定（三值穷举；点击上报的 `decision` 即由它产出，无第四种）。
#[must_use]
pub fn decision_of_action(action: ApprovalAction) -> ApprovalDecisionKind {
    match action {
        ApprovalAction::Allow => ApprovalDecisionKind::Allow,
        ApprovalAction::Deny => ApprovalDecisionKind::Deny,
        ApprovalAction::AlwaysAllow => ApprovalDecisionKind::Always,
    }
}

/// 并回一条 C3 审批结果 `{id, decision}`（按 `id` 匹配卡片）。
///
/// 只更新展示态（已决策标注、按钮禁用）；是否放行由运行时决定。
/// 未匹配到 `id` ⇒ `false`（不伪造并回）。
pub fn apply_decision(cards: &mut [ApprovalCardView], result: &ApprovalDecision) -> bool {
    let Some(card) = cards.iter_mut().find(|card| card.id == result.id) else {
        return false;
    };
    card.apply_result(Some(action_of_decision(result.decision).allows()));
    true
}

/// 审批区视图模型：C3 消费侧三态（fail-closed——失败必须带原因上屏，
/// 不许静默退化成空列表）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalListView {
    /// 正在加载（界面显示加载态）。
    Loading,
    /// 就绪：卡片集 + 来源标注（演示数据必须如实标 [`Provenance::Demo`]）。
    Ready {
        cards: Vec<ApprovalCardView>,
        provenance: Provenance,
    },
    /// 失败：界面必须显示该原因。
    Failed { reason: String },
}

impl ApprovalListView {
    /// 可注入入口：C3 审批请求三态 → 审批区视图（契约形状的唯一进料口）。
    #[must_use]
    pub fn from_requests(
        requests: LoadState<Vec<ApprovalRequest>>,
        provenance: Provenance,
    ) -> Self {
        match requests {
            LoadState::Loading => Self::Loading,
            LoadState::Ready(list) => Self::Ready {
                cards: list.iter().map(ApprovalCardView::from_request).collect(),
                provenance,
            },
            LoadState::Failed(reason) => Self::Failed { reason },
        }
    }

    /// 就绪态的卡片集；加载/失败态为空切片（渲染须先按三态分流，
    /// 不得拿它冒充「没有审批」）。
    #[must_use]
    pub fn cards(&self) -> &[ApprovalCardView] {
        match self {
            Self::Ready { cards, .. } => cards,
            Self::Loading | Self::Failed { .. } => &[],
        }
    }

    /// 失败原因；未失败为 `None`（渲染层据此上屏）。
    #[must_use]
    pub fn failure(&self) -> Option<&str> {
        match self {
            Self::Failed { reason } => Some(reason.as_str()),
            Self::Loading | Self::Ready { .. } => None,
        }
    }
}

/// 演示出口（薄封装）：本地会话态转 C3 形状后经 [`ApprovalListView::from_requests`]
/// 出视图，来源如实标注 [`Provenance::Demo`]。
#[must_use]
pub fn demo_list(state: &SessionStreamState) -> ApprovalListView {
    let requests = state
        .pending_approvals()
        .map(|approval| ApprovalRequest {
            id: approval.id.clone(),
            action: approval.tool.clone(),
            detail: approval.summary.clone(),
        })
        .collect();
    ApprovalListView::from_requests(LoadState::Ready(requests), Provenance::Demo)
}

/// 待审批卡片：只包含**未决策**项（既有出口，薄封装自 [`demo_list`]）。
#[must_use]
pub fn pending_cards(state: &SessionStreamState) -> Vec<ApprovalCardView> {
    demo_list(state).cards().to_vec()
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
/// 决策先落成 C3 结果形状 `{id, decision}`（见 [`decision_of_action`]），再写回
/// 本地会话态；是否放行由运行时决定，界面不判定权限。
/// 返回 `false` 表示被拒绝：项不存在、已决策（幂等保护），或校验失败。
/// 已决策项绝不会因重复点击被改成相反结果。
pub fn decide(state: &mut SessionStreamState, id: &str, action: ApprovalAction) -> bool {
    let decision = ApprovalDecision {
        id: id.to_owned(),
        decision: decision_of_action(action),
    };
    state.resolve_approval(id, action_of_decision(decision.decision).allows())
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
