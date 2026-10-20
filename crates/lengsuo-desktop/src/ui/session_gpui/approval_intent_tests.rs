//! [`super::approval_intent`] 的纯逻辑测试（不开窗口）。
//!
//! 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
//! `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。

#[cfg(test)]
mod tests {
    use super::super::approval_intent::{
        record_intent, scope_for, AlwaysAllowMap, ApprovalIntent, ApprovalRequest, ApprovalScope,
        Recorded, ScopeKind,
    };
    use super::super::{SessionStreamGpui, SessionUiState};
    use crate::state::session_stream::{PendingApproval, SessionStreamState};
    use crate::ui::approval_card::ApprovalAction;

    fn always(id: Option<&str>, tool: Option<&str>) -> ApprovalIntent {
        ApprovalIntent::from_request(&ApprovalRequest::tool_scoped(
            id,
            tool,
            ApprovalAction::AlwaysAllow,
        ))
    }

    #[test]
    fn always_allow_produces_intent_with_scope_and_id() {
        let intent = always(Some("a1"), Some("Bash"));
        assert_eq!(intent.action, ApprovalAction::AlwaysAllow);
        assert_eq!(intent.approval_id, "a1", "审批 id 原样带上");
        assert_eq!(intent.tool.as_deref(), Some("Bash"));
        assert_eq!(
            intent.scope,
            ApprovalScope::Tool {
                tool: "Bash".into()
            }
        );
        assert!(intent.is_scoped_always());
    }

    #[test]
    fn session_scope_is_selected_explicitly() {
        let request = ApprovalRequest {
            approval_id: Some("a2"),
            tool: Some("Bash"),
            session_id: Some("s1"),
            kind: ScopeKind::Session,
            action: ApprovalAction::AlwaysAllow,
        };
        let scope = scope_for(&request);
        assert_eq!(
            scope,
            ApprovalScope::Session {
                session_id: Some("s1".into())
            }
        );
        assert_eq!(scope.key().as_deref(), Some("session:s1"));
    }

    #[test]
    fn unknown_inputs_are_fail_closed_not_all() {
        for intent in [
            always(None, Some("Bash")),
            always(Some(""), Some("Bash")),
            always(Some("   "), Some("Bash")),
            always(Some("a1"), None),
            always(Some("a1"), Some("  ")),
        ] {
            assert!(
                intent.scope.is_unknown(),
                "未知输入必须落到 Unknown，实际 {:?}",
                intent.scope
            );
            assert_eq!(intent.scope.key(), None, "未知作用域没有可用的键");
            assert_ne!(
                intent.scope,
                ApprovalScope::Session { session_id: None },
                "不得伪装成某个会话"
            );
        }
        // 不是「总是允许」的选择不携带作用域。
        let allow = ApprovalIntent::from_request(&ApprovalRequest::tool_scoped(
            Some("a1"),
            Some("Bash"),
            ApprovalAction::Allow,
        ));
        assert!(!allow.is_scoped_always());
        assert!(allow.scope.is_unknown());
    }

    #[test]
    fn always_allow_local_state_survives_repaint() {
        let mut map = AlwaysAllowMap::default();
        let intent = always(Some("a1"), Some("Bash"));
        assert_eq!(record_intent(&mut map, &intent), Recorded::Inserted);
        // 「重绘后」再查同一个键，仍能拿到动作。
        let again = always(Some("a1"), Some("Bash"));
        let key = again.scope.key().expect("已知作用域有键");
        assert_eq!(map.get(&key), Some(&ApprovalAction::AlwaysAllow));
    }

    #[test]
    fn another_approval_is_not_affected_by_previous_always_allow() {
        let mut map = AlwaysAllowMap::default();
        record_intent(&mut map, &always(Some("a1"), Some("Read")));
        let other = always(Some("a2"), Some("Bash"));
        assert_eq!(
            map.get(&other.scope.key().unwrap()),
            None,
            "换一个 id ⇒ 不受上一张卡影响"
        );
        let unknown = always(None, Some("Bash"));
        assert_eq!(record_intent(&mut map, &unknown), Recorded::Skipped);
        assert_eq!(map.len(), 1, "未知作用域不得写入本地态");
    }

    #[test]
    fn same_scope_always_allow_is_idempotent() {
        let mut map = AlwaysAllowMap::default();
        assert_eq!(
            record_intent(&mut map, &always(Some("a1"), Some("Bash"))),
            Recorded::Inserted
        );
        assert_eq!(
            record_intent(&mut map, &always(Some("a2"), Some("Bash"))),
            Recorded::Idempotent
        );
        assert_eq!(map.len(), 1, "同作用域不重复累加");
    }

    fn stream_with(ids: &[&str]) -> SessionStreamState {
        let mut state = SessionStreamState::new();
        for id in ids {
            state.push_approval(PendingApproval {
                id: (*id).into(),
                tool: "Bash".into(),
                summary: "ls".into(),
                resolved: None,
            });
        }
        state
    }

    #[test]
    fn tool_call_starts_folded_and_toggles() {
        let mut ui = SessionUiState::default();
        assert!(!ui.is_expanded(0, true), "工具卡片默认收起");
        ui.toggle_expanded(0, true);
        assert!(ui.is_expanded(0, true));
        ui.toggle_expanded(0, true);
        assert!(!ui.is_expanded(0, true));
    }

    #[test]
    fn file_selection_toggles_off() {
        let mut ui = SessionUiState::default();
        ui.toggle_file("src/main.rs");
        assert_eq!(ui.selected_file(), Some("src/main.rs"));
        ui.toggle_file("src/main.rs");
        assert_eq!(ui.selected_file(), None);
    }

    #[test]
    fn approval_choice_is_local_only_and_never_resolves() {
        let mut view = SessionStreamGpui::new(stream_with(&["a1"]));
        view.choose_approval("a1", ApprovalAction::Allow);

        assert_eq!(
            view.ui_state().approval_choice.get("a1"),
            Some(&ApprovalAction::Allow)
        );
        assert_eq!(
            view.state().approvals()[0].resolved,
            None,
            "本地选择不得伪造审批决策"
        );
    }

    #[test]
    fn always_allow_choice_survives_repaint_in_view() {
        let mut view = SessionStreamGpui::new(stream_with(&["a1"]));
        let intent = view.record_intent(&ApprovalRequest::tool_scoped(
            Some("a1"),
            Some("Bash"),
            ApprovalAction::AlwaysAllow,
        ));
        assert!(intent.is_scoped_always());
        assert_eq!(intent.approval_id, "a1");

        // 模拟重绘：渲染时按审批 id 重新查询选中态。
        assert_eq!(
            view.ui_state().display_choice("a1"),
            Some(ApprovalAction::AlwaysAllow),
            "重绘后仍显示选中态"
        );
        assert_eq!(
            view.state().approvals()[0].resolved,
            None,
            "意图不得伪造决策"
        );
        assert_eq!(view.ui_state().always_allow_scope.len(), 1);
    }

    #[test]
    fn only_the_clicked_card_is_selected_others_stay_unselected() {
        // 两张卡**同工具**：反例要证明「一张卡点了，全部变绿」不会发生。
        let mut view = SessionStreamGpui::new(stream_with(&["a1", "a2"]));
        view.record_intent(&ApprovalRequest::tool_scoped(
            Some("a1"),
            Some("Bash"),
            ApprovalAction::AlwaysAllow,
        ));

        assert_eq!(
            view.ui_state().display_choice("a1"),
            Some(ApprovalAction::AlwaysAllow)
        );
        assert_eq!(
            view.ui_state().display_choice("a2"),
            None,
            "反例：一张卡点了，其它卡必须仍是未选态"
        );
        assert_eq!(
            view.state().pending_approvals().count(),
            2,
            "界面不得因「总是允许」隐藏任何审批卡"
        );
    }
}
