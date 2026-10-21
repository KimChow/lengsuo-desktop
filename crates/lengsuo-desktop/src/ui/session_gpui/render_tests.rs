//! 会话流渲染分支的纯逻辑测试（不开窗口）。
//!
//! 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
//! `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进来并遮蔽标准库的 `#[test]`，导致测试无法编译。
    use super::super::diff_lines::{classify, DiffLineKind};
    use super::super::render_block_with;
    use super::super::{ApprovalAction, BlockCtx, SessionStreamGpui, SessionUiState};
    use crate::state::session_stream::{
        MessageRole, PendingApproval, SessionMessage, SessionStreamState,
    };
    use crate::ui::session_render::{self, Block};
    use gpui_kit::component::Theme;

    fn msg(role: MessageRole, content: &str, tool: Option<&str>) -> SessionMessage {
        SessionMessage {
            role,
            content: content.into(),
            tool: tool.map(str::to_owned),
            ts_ms: 0,
        }
    }

    #[test]
    fn tool_call_starts_folded_and_toggles() {
        let mut ui = SessionUiState::default();
        assert!(ui.is_tool_folded("tc-a"), "工具卡片默认收起");
        ui.toggle_tool_fold("tc-a");
        assert!(!ui.is_tool_folded("tc-a"), "点击后展开");
        ui.toggle_tool_fold("tc-a");
        assert!(ui.is_tool_folded("tc-a"), "再点一次收起");
    }

    #[test]
    fn approval_choice_is_local_only_and_never_resolves() {
        let mut state = SessionStreamState::new();
        state.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "Bash".into(),
            summary: "rm -rf build".into(),
            resolved: None,
        });
        let mut view = SessionStreamGpui::new(state);
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
    fn file_selection_toggles_off() {
        let mut ui = SessionUiState::default();
        ui.toggle_file("src/main.rs");
        assert_eq!(ui.selected_file(), Some("src/main.rs"));
        ui.toggle_file("src/main.rs");
        assert_eq!(ui.selected_file(), None);
    }

    #[test]
    fn diff_lines_are_classified_for_coloring() {
        assert_eq!(classify("+added"), DiffLineKind::Added);
        assert_eq!(classify("-removed"), DiffLineKind::Removed);
        assert_eq!(classify("@@ -1 +1 @@"), DiffLineKind::Hunk);
        assert_eq!(classify("+++ b/x"), DiffLineKind::Header);
        assert_eq!(classify(" context"), DiffLineKind::Context);
    }

    #[test]
    fn every_block_kind_has_a_render_branch() {
        let theme = Theme::default();
        let ctx = BlockCtx::passive(&theme);
        let mut state = SessionStreamState::new();
        assert!(state.push_message(1, msg(MessageRole::User, "你好", None)));
        assert!(state.push_message(2, msg(MessageRole::Assistant, "好的", None)));
        assert!(state.push_message(3, msg(MessageRole::ToolCall, "src/main.rs", Some("Read"))));
        assert!(state.push_message(4, msg(MessageRole::ToolResult, "src/main.rs", Some("Glob"))));
        state.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "Bash".into(),
            summary: "ls".into(),
            resolved: None,
        });
        state.mark_gap();

        let blocks = session_render::render(&state);
        let kinds: Vec<&str> = blocks
            .iter()
            .map(|b| match b {
                Block::UserBubble { .. } => "user",
                Block::AssistantText { .. } => "assistant",
                Block::ToolCall(_) => "tool_call",
                Block::ToolResult(_) => "tool_result",
                Block::FilesGrid { .. } => "files",
                Block::Approval(_) => "approval",
                Block::StaleNotice { .. } => "stale",
            })
            .collect();
        for want in [
            "user",
            "assistant",
            "tool_call",
            "tool_result",
            "approval",
            "stale",
        ] {
            assert!(kinds.contains(&want), "缺少分支：{want}（实际 {kinds:?}）");
        }
        // 七种块都能渲染出元素（不 panic）。
        for block in &blocks {
            let _ = render_block_with(block, &ctx);
        }
        // 文件网格也有分支。
        let _ = render_block_with(&session_render::files_grid(&["a.rs".into()]), &ctx);
    }
}
