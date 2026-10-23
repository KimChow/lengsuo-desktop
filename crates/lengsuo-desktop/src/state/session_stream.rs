//! 会话流视图状态（工作台语义）。
//!
//! 承载「会话消息 + 工具卡片 + 审批」的界面态。数据由打包时引入的客户端注入。
//!
//! 关键约定：
//! - 工具名对用户**不可见**：界面标题走 [`crate::theme::tool_titles`] 的人话映射。
//! - 审批是**挂起态**：未决策前界面必须显示为等待中，不得默认放行。

use serde_json::Value;

use crate::state::datasource::{
    ConversationDeltaOp, ConversationItem, ConversationMessageKind, ConversationSnapshot,
};

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

/// [`apply_delta`] 的应用结果：成功几条、跳过几条。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeltaOutcome {
    pub applied: usize,
    pub skipped: usize,
}

/// C1 增量批次里的一条 → 现有 [`SessionMessage`] 的容错转换。
///
/// §2 C1 六类枚举里 `files` / `approval` 超出现有四角色（payload 形状契约未定稿）：
/// 转不过去就返回 `None`（调用方跳过并计数），⛔ 不往消息里塞伪数据。
fn item_to_message(item: &ConversationItem) -> Option<SessionMessage> {
    let role = match &item.kind {
        ConversationMessageKind::User => MessageRole::User,
        ConversationMessageKind::AssistantText => MessageRole::Assistant,
        ConversationMessageKind::ToolUse => MessageRole::ToolCall,
        ConversationMessageKind::ToolResult => MessageRole::ToolResult,
        ConversationMessageKind::Files | ConversationMessageKind::Approval => return None,
    };
    let payload = &item.payload;
    Some(SessionMessage {
        role,
        content: payload
            .get("text")
            .or_else(|| payload.get("content"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        tool: payload
            .get("tool")
            .or_else(|| payload.get("name"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        ts_ms: payload.get("ts_ms").and_then(Value::as_i64).unwrap_or(0),
    })
}

/// 按 C1 的 [`ConversationDeltaOp`]（`append` / `patch`）把增量批次应用进会话流状态。
///
/// 纯函数（无 IO、不 panic）：只换数据来源，**不改** [`SessionMessage`] 的结构与字段。
/// - `append`：批次按序追加；seq 取 payload 的 `seq`（缺省 `last_seq + 1`），
///   重复 / 回退被 [`SessionStreamState::push_message`] 拒绝并计入跳过（防重放）；
/// - `patch`：按 payload 的 `index` 定位**已有**消息，只改其 `content`
///   （取 `text` / `content` 字段），⛔ 不动其他消息、不增删；
///   目标缺失或字段不符 ⇒ 跳过，不 panic。
#[must_use]
pub fn apply_delta(state: &mut SessionStreamState, delta: &ConversationSnapshot) -> DeltaOutcome {
    let mut outcome = DeltaOutcome::default();
    match &delta.op {
        ConversationDeltaOp::Append => {
            for item in &delta.batch {
                let Some(msg) = item_to_message(item) else {
                    outcome.skipped += 1;
                    continue;
                };
                let seq = item
                    .payload
                    .get("seq")
                    .and_then(Value::as_u64)
                    .unwrap_or_else(|| state.last_seq() + 1);
                if state.push_message(seq, msg) {
                    outcome.applied += 1;
                } else {
                    outcome.skipped += 1;
                }
            }
        }
        ConversationDeltaOp::Patch => {
            for item in &delta.batch {
                let patched = item
                    .payload
                    .get("index")
                    .and_then(Value::as_u64)
                    .and_then(|index| state.messages.get_mut(index as usize))
                    .and_then(|target| {
                        let text = item
                            .payload
                            .get("text")
                            .or_else(|| item.payload.get("content"))?
                            .as_str()?;
                        target.content = text.to_owned();
                        Some(())
                    });
                match patched {
                    Some(()) => outcome.applied += 1,
                    None => outcome.skipped += 1,
                }
            }
        }
    }
    outcome
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

    // ----- C1 增量应用（apply_delta）：append / patch 两分支 -----

    fn delta(op: ConversationDeltaOp, batch: Vec<ConversationItem>) -> ConversationSnapshot {
        ConversationSnapshot {
            event: "delta".into(),
            op,
            batch,
        }
    }

    fn item(kind: ConversationMessageKind, payload: Value) -> ConversationItem {
        ConversationItem { kind, payload }
    }

    #[test]
    fn append_adds_messages_in_order() {
        let mut s = SessionStreamState::new();
        let _ = apply_delta(
            &mut s,
            &delta(
                ConversationDeltaOp::Append,
                vec![
                    item(ConversationMessageKind::User, json!({"text": "第一"})),
                    item(
                        ConversationMessageKind::AssistantText,
                        json!({"text": "第二"}),
                    ),
                ],
            ),
        );
        let out = apply_delta(
            &mut s,
            &delta(
                ConversationDeltaOp::Append,
                vec![item(ConversationMessageKind::User, json!({"text": "第三"}))],
            ),
        );
        assert_eq!(out.applied, 1, "append 后消息数应 +1");
        let texts: Vec<&str> = s.messages().iter().map(|m| m.content.as_str()).collect();
        assert_eq!(texts, ["第一", "第二", "第三"], "顺序必须稳定");
    }

    #[test]
    fn patch_touches_only_the_target_message() {
        let mut s = SessionStreamState::new();
        let _ = apply_delta(
            &mut s,
            &delta(
                ConversationDeltaOp::Append,
                vec![
                    item(ConversationMessageKind::User, json!({"text": "甲"})),
                    item(
                        ConversationMessageKind::AssistantText,
                        json!({"text": "乙"}),
                    ),
                ],
            ),
        );
        let out = apply_delta(
            &mut s,
            &delta(
                ConversationDeltaOp::Patch,
                vec![item(
                    ConversationMessageKind::AssistantText,
                    json!({"index": 1, "text": "乙（改）"}),
                )],
            ),
        );
        assert_eq!((out.applied, out.skipped), (1, 0));
        assert_eq!(s.messages()[1].content, "乙（改）", "目标消息应被改写");
        assert_eq!(s.messages()[0].content, "甲", "patch 不得动其他消息");
        assert_eq!(s.messages().len(), 2, "patch 不得增删消息");
        // 目标越界 / 字段缺失 ⇒ 跳过，不 panic
        let out = apply_delta(
            &mut s,
            &delta(
                ConversationDeltaOp::Patch,
                vec![item(
                    ConversationMessageKind::AssistantText,
                    json!({"index": 9}),
                )],
            ),
        );
        assert_eq!((out.applied, out.skipped), (0, 1));
    }

    #[test]
    fn append_skips_unmappable_kinds_and_replayed_seq() {
        let mut s = SessionStreamState::new();
        let out = apply_delta(
            &mut s,
            &delta(
                ConversationDeltaOp::Append,
                vec![
                    item(ConversationMessageKind::Files, json!({})),
                    item(
                        ConversationMessageKind::User,
                        json!({"text": "ok", "seq": 5}),
                    ),
                ],
            ),
        );
        assert_eq!(
            (out.applied, out.skipped),
            (1, 1),
            "files / approval 现阶段映射不了 ⇒ 跳过，不伪造"
        );
        let out = apply_delta(
            &mut s,
            &delta(
                ConversationDeltaOp::Append,
                vec![item(
                    ConversationMessageKind::User,
                    json!({"text": "重放", "seq": 5}),
                )],
            ),
        );
        assert_eq!(
            (out.applied, out.skipped),
            (0, 1),
            "重复 seq 必须拒绝（防重放）"
        );
        assert_eq!(s.messages().len(), 1);
        assert_eq!(s.messages()[0].role, MessageRole::User);
    }
}
