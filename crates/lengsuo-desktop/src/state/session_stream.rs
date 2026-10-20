//! 会话流视图状态（工作台语义）。
//!
//! 承载「会话消息 + 工具卡片 + 审批」的界面态。数据由打包时引入的客户端注入。
//!
//! 关键约定：
//! - 工具名对用户**不可见**：界面标题走 [`crate::theme::tool_titles`] 的人话映射。
//! - 审批是**挂起态**：未决策前界面必须显示为等待中，不得默认放行。

use serde_json::Value;

/// 消息角色（对齐参考客户端基线：四类消息）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    ToolCall,
    ToolResult,
}

/// 一条会话消息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMessage {
    pub role: MessageRole,
    /// 展示正文；大结果不入此处（由引用承载）。
    pub content: String,
    /// 工具名（仅 `ToolCall` / `ToolResult` 有意义）。
    pub tool: Option<String>,
    pub ts_ms: i64,
}

/// 待审批项。`resolved` 为 `None` 表示仍挂起。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingApproval {
    pub id: String,
    pub tool: String,
    pub summary: String,
    pub resolved: Option<bool>,
}

/// 会话流状态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionStreamState {
    messages: Vec<SessionMessage>,
    approvals: Vec<PendingApproval>,
    last_seq: u64,
    stale: bool,
}

impl SessionStreamState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn messages(&self) -> &[SessionMessage] {
        &self.messages
    }

    #[must_use]
    pub fn approvals(&self) -> &[PendingApproval] {
        &self.approvals
    }

    /// 尚未决策的审批项。
    pub fn pending_approvals(&self) -> impl Iterator<Item = &PendingApproval> {
        self.approvals.iter().filter(|a| a.resolved.is_none())
    }

    #[must_use]
    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// 是否处于「事件流断档」状态：断档时界面须提示并等待重放，
    /// **不得**假装完整（参考客户端基线：历史重放不重跑工具）。
    #[must_use]
    pub fn is_stale(&self) -> bool {
        self.stale
    }

    /// 追加一条消息。**seq 必须单调递增**，重复或回退的 seq 被拒绝（返回 `false`）。
    pub fn push_message(&mut self, seq: u64, msg: SessionMessage) -> bool {
        if seq <= self.last_seq {
            return false;
        }
        self.last_seq = seq;
        self.stale = false;
        self.messages.push(msg);
        true
    }

    /// 标记事件流出现序号缺口（断档）。
    pub fn mark_gap(&mut self) {
        self.stale = true;
    }

    /// 登记一个待审批项。
    pub fn push_approval(&mut self, approval: PendingApproval) {
        self.approvals.push(approval);
    }

    /// 对某个审批项作出决策。已决策项**不可重复决策**（返回 `false`）。
    pub fn resolve_approval(&mut self, id: &str, allow: bool) -> bool {
        match self.approvals.iter_mut().find(|a| a.id == id) {
            Some(a) if a.resolved.is_none() => {
                a.resolved = Some(allow);
                true
            }
            _ => false,
        }
    }

    /// 清空（切换会话）。
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// 从事件 payload 解析一条消息（容错：字段缺失返回 `None`，不 panic）。
#[must_use]
pub fn parse_message(payload: &Value) -> Option<SessionMessage> {
    let role = match payload.get("role").and_then(Value::as_str)? {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "tool_call" => MessageRole::ToolCall,
        "tool_result" => MessageRole::ToolResult,
        _ => return None,
    };
    Some(SessionMessage {
        role,
        content: payload
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        tool: payload
            .get("tool")
            .and_then(Value::as_str)
            .map(str::to_owned),
        ts_ms: payload.get("ts_ms").and_then(Value::as_i64).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn msg(role: MessageRole) -> SessionMessage {
        SessionMessage {
            role,
            content: "c".into(),
            tool: None,
            ts_ms: 0,
        }
    }

    #[test]
    fn seq_must_be_monotonic() {
        let mut s = SessionStreamState::new();
        assert!(s.push_message(1, msg(MessageRole::User)));
        assert!(
            !s.push_message(1, msg(MessageRole::User)),
            "重复 seq 必须拒绝"
        );
        assert!(
            !s.push_message(0, msg(MessageRole::User)),
            "回退 seq 必须拒绝"
        );
        assert!(s.push_message(2, msg(MessageRole::Assistant)));
        assert_eq!(s.messages().len(), 2);
        assert_eq!(s.last_seq(), 2);
    }

    #[test]
    fn gap_marks_stale_and_next_message_clears_it() {
        let mut s = SessionStreamState::new();
        s.push_message(1, msg(MessageRole::User));
        s.mark_gap();
        assert!(s.is_stale());
        s.push_message(5, msg(MessageRole::Assistant));
        assert!(!s.is_stale(), "收到新事件后应解除断档态");
    }

    #[test]
    fn approval_starts_pending() {
        let mut s = SessionStreamState::new();
        s.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "exec".into(),
            summary: "run rm".into(),
            resolved: None,
        });
        assert_eq!(s.pending_approvals().count(), 1);
    }

    #[test]
    fn approval_cannot_be_resolved_twice() {
        let mut s = SessionStreamState::new();
        s.push_approval(PendingApproval {
            id: "a1".into(),
            tool: "exec".into(),
            summary: "s".into(),
            resolved: None,
        });
        assert!(s.resolve_approval("a1", false));
        assert!(
            !s.resolve_approval("a1", true),
            "已决策的审批不得被二次放行"
        );
        assert_eq!(s.pending_approvals().count(), 0);
    }

    #[test]
    fn resolving_unknown_approval_is_a_noop() {
        let mut s = SessionStreamState::new();
        assert!(!s.resolve_approval("absent", true));
    }

    #[test]
    fn parse_message_handles_known_roles_and_rejects_unknown() {
        let m = parse_message(&json!({"role": "tool_call", "tool": "Read", "ts_ms": 5})).unwrap();
        assert_eq!(m.role, MessageRole::ToolCall);
        assert_eq!(m.tool.as_deref(), Some("Read"));
        assert_eq!(m.ts_ms, 5);
        assert!(parse_message(&json!({"role": "???"})).is_none());
        assert!(parse_message(&json!({})).is_none());
    }

    #[test]
    fn clear_resets_everything() {
        let mut s = SessionStreamState::new();
        s.push_message(3, msg(MessageRole::User));
        s.mark_gap();
        s.clear();
        assert_eq!(s.last_seq(), 0);
        assert!(s.messages().is_empty());
        assert!(!s.is_stale());
    }
}
