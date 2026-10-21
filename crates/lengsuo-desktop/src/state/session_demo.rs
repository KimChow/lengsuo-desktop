//! 演示会话数据（含**五类真实工具调用**）。
//!
//! 为什么单独一个文件：工具卡片要能被**肉眼**与**测试**覆盖，演示数据必须
//! 真的产出「读取 / 创建 / 修改 / 执行命令 / 搜索」五种卡片，而不是只放审批卡。
//!
//! 约定：
//! - 本文件只造**纯数据**（[`SessionStreamState`]），不碰 GPUI、不落盘。
//! - 工具名**只**出现在 [`SessionMessage::tool`]（内部字段）；
//!   界面上能看到的文字一律经 [`crate::theme::tool_titles`] 转成人话。
//! - 参数正文里**不得**出现工具英文名本身（否则会破坏 R13-1 的可见性约束）。
//!
//! ⚠️ 本卡边界：`ui/gpui_app/demo.rs`（应用装配层）**不在**本卡可改范围内，
//! 因此这份演示数据目前只由 [`crate::ui::session_gpui::SessionStreamGpui::demo`]
//! 与本文件的单元测试消费；把它接进运行中的主界面需要改 `ui/gpui_app/demo.rs`
//! （一行替换），归属后续卡片。

use crate::state::session_stream::{
    MessageRole, PendingApproval, SessionMessage, SessionStreamState,
};

/// 一根演示消息。
fn message(role: MessageRole, content: &str, tool: Option<&str>) -> SessionMessage {
    SessionMessage {
        role,
        content: content.into(),
        tool: tool.map(str::to_owned),
        ts_ms: 0,
    }
}

/// 演示用会话流：覆盖四类消息、**五类真实工具调用**与一个挂起审批。
///
/// 五类工具调用（对应参考客户端的验收项「工具卡片五类真实触发渲染正确」）：
///
/// | 序号 | 工具 | 参数正文 | 界面人话标题 |
/// |---|---|---|---|
/// | 1 | 读取 | `scripts/card_guard.py` | 读取 |
/// | 2 | 创建 | `docs/session-render-status.md` | 创建 |
/// | 3 | 修改 | `crates/lengsuo-desktop/src/ui/tool_card.rs` | 修改 |
/// | 4 | 执行命令 | `cargo test --workspace` | 执行命令 |
/// | 5 | 搜索 | `fn render_tool_call` | 搜索 |
#[must_use]
pub fn demo_session() -> SessionStreamState {
    let mut state = SessionStreamState::new();
    state.push_message(
        1,
        message(MessageRole::User, "帮我看一下构建脚本为什么失败", None),
    );
    state.push_message(
        2,
        message(MessageRole::Assistant, "好的，我先读取入口脚本。", None),
    );
    // ① 读取
    state.push_message(
        3,
        message(MessageRole::ToolCall, "scripts/card_guard.py", Some("Read")),
    );
    state.push_message(
        4,
        message(
            MessageRole::ToolResult,
            "#!/usr/bin/env python3\nprint('ok')",
            Some("Read"),
        ),
    );
    // ② 创建
    state.push_message(
        5,
        message(
            MessageRole::ToolCall,
            "docs/session-render-status.md",
            Some("Write"),
        ),
    );
    // ③ 修改
    state.push_message(
        6,
        message(
            MessageRole::ToolCall,
            "crates/lengsuo-desktop/src/ui/tool_card.rs",
            Some("Edit"),
        ),
    );
    // ④ 执行命令
    state.push_message(
        7,
        message(
            MessageRole::ToolCall,
            "cargo test --workspace",
            Some("Bash"),
        ),
    );
    state.push_message(
        8,
        message(
            MessageRole::ToolResult,
            "running 264 tests\ntest result: ok",
            Some("Bash"),
        ),
    );
    // ⑤ 搜索
    state.push_message(
        9,
        message(MessageRole::ToolCall, "fn render_tool_call", Some("Grep")),
    );
    state.push_approval(PendingApproval {
        id: "demo-approval-1".into(),
        tool: "Bash".into(),
        summary: "运行 cargo test --workspace".into(),
        resolved: None,
    });
    state
}

/// 演示会话里的**五类工具调用**（工具名 + 参数正文）。
///
/// 供测试逐类核对「界面不得出现工具英文名」。
#[must_use]
pub fn demo_tool_calls() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Read", "scripts/card_guard.py"),
        ("Write", "docs/session-render-status.md"),
        ("Edit", "crates/lengsuo-desktop/src/ui/tool_card.rs"),
        ("Bash", "cargo test --workspace"),
        ("Grep", "fn render_tool_call"),
    ]
}

#[cfg(test)]
mod tests {
    use super::{demo_session, demo_tool_calls};
    use crate::state::session_stream::MessageRole;

    #[test]
    fn the_demo_session_covers_five_real_tool_calls() {
        let state = demo_session();
        let calls: Vec<&crate::state::session_stream::SessionMessage> = state
            .messages()
            .iter()
            .filter(|m| m.role == MessageRole::ToolCall)
            .collect();
        assert_eq!(calls.len(), 5, "五类真实工具调用一个都不能少");
        let tools: Vec<&str> = calls.iter().filter_map(|m| m.tool.as_deref()).collect();
        assert_eq!(
            tools,
            vec!["Read", "Write", "Edit", "Bash", "Grep"],
            "五类顺序固定：读取 / 创建 / 修改 / 执行命令 / 搜索"
        );
    }

    #[test]
    fn the_demo_session_keeps_the_four_roles_and_one_pending_approval() {
        let state = demo_session();
        let mut roles: Vec<MessageRole> = state.messages().iter().map(|m| m.role).collect();
        roles.sort_by_key(|role| format!("{role:?}"));
        roles.dedup();
        assert_eq!(roles.len(), 4, "演示数据必须覆盖四类消息");
        assert_eq!(state.pending_approvals().count(), 1);
    }

    #[test]
    fn the_demo_arguments_never_contain_the_tool_name() {
        for (tool, args) in demo_tool_calls() {
            assert!(
                !args.to_lowercase().contains(&tool.to_lowercase()),
                "参数正文里不得出现工具英文名 {tool}，实际：{args}"
            );
        }
    }
}
