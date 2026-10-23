//! 「视图 ← 运行时」数据源契约（**消费侧**，纯逻辑层：无 GPUI、无 IO、无 async）。
//!
//! 对齐 `docs/ui-runtime-contract.md` §2 契约总表 C1~C5（会话流增量事件 / 工具卡片
//! 数据 / 审批请求与结果 / 停止投递 / 终端 PTY）：消费侧快照类型 + 只读
//! [`ViewDataSource`] trait + [`DemoSource`] 演示出口。
//!
//! 纪律（`docs/ui-runtime-contract.md` §1 第 3 条 / §5）：
//! - 契约未就绪 ⇒ 界面**保持演示数据 + 如实标注**；⛔ 不许伪造服务端数据、⛔ 不许假装已接通。
//! - **fail-closed**：加载失败必须能表示成 [`LoadState::Failed`]，界面显示错误，
//!   ⛔ 不许静默退化成空列表。
//! - 本层**只描述界面怎么消费**，⛔ 不实现任何运行时语义（那是运行时/服务端线的活）。

/// 一次视图读取的三态（fail-closed：失败必须能被表达，不能静默退化成空）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadState<T> {
    /// 正在加载（界面显示加载态）。
    Loading,
    /// 成功：界面可渲染这份快照。
    Ready(T),
    /// 失败：带原因（界面**必须**显示错误文案）。
    Failed(String),
}

impl<T> LoadState<T> {
    /// 是否已就绪。
    #[must_use]
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// 失败原因；未失败 ⇒ `None`。
    #[must_use]
    pub fn failure(&self) -> Option<&str> {
        match self {
            Self::Failed(reason) => Some(reason.as_str()),
            _ => None,
        }
    }
}

/// 数据来源标注：界面**如实显示**当前是演示数据还是真实运行时数据。
///
/// 契约未就绪时界面必须仍标 [`Provenance::Demo`]，⛔ 不许标成 `Runtime`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provenance {
    /// 演示数据（占位）。
    Demo,
    /// 真实运行时/服务端数据。
    Runtime,
}

impl Provenance {
    /// 是否仍是演示数据。
    #[must_use]
    pub fn is_demo(self) -> bool {
        matches!(self, Self::Demo)
    }
}

// ---------------------------------------------------------------------------
// C1~C5 消费侧快照：字段名与 §2 契约表逐字一致（camelCase wire 名按 Rust 惯例
// 记 snake_case，行尾注释逐字引用原字段名；表里无法确定的字段以
// `TODO(契约待定)` 标注，⛔ 不自己发明）。
// ---------------------------------------------------------------------------

/// C1 消息类型枚举（§2 逐字：user / assistant-text / tool_use / tool_result / files / approval）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationMessageKind {
    User,          // `user`
    AssistantText, // `assistant-text`
    ToolUse,       // `tool_use`
    ToolResult,    // `tool_result`
    Files,         // `files`
    Approval,      // `approval`
}

/// C1 增量批次操作（§2 逐字：append / patch）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationDeltaOp {
    Append, // `append`
    Patch,  // `patch`
}

/// C1 增量批次中的一条消息。
#[derive(Debug, Clone, PartialEq)]
pub struct ConversationItem {
    pub kind: ConversationMessageKind, // §2 C1 六类枚举
    /// TODO(契约待定)：六类消息各自的 payload 字段形状未定稿（§2 只定了枚举），暂以中性 JSON 值承载。
    pub payload: serde_json::Value,
}

/// C1 会话流增量事件快照：事件名 + 增量批次（append/patch）。
#[derive(Debug, Clone, PartialEq)]
pub struct ConversationSnapshot {
    pub event: String,                // §2 C1：事件名
    pub op: ConversationDeltaOp,      // §2 C1：append / patch
    pub batch: Vec<ConversationItem>, // §2 C1：增量批次
}

/// C2 工具结果种类（§2 逐字：text / files / diff）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolResultKind {
    Text,  // `text`
    Files, // `files`
    Diff,  // `diff`
}

/// C2 工具卡片快照：tool_use 的 `{name, args}` + tool_result 的 `{kind, payload}`（§2：并入 C1）。
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCardSnapshot {
    pub name: String, // tool_use `name`
    /// TODO(契约待定)：args 的内部结构未定稿（§2 只给字段名），暂以中性 JSON 值承载。
    pub args: serde_json::Value, // tool_use `args`
    pub kind: ToolResultKind, // tool_result `kind`
    /// TODO(契约待定)：payload 随 kind（text/files/diff）变化的形状未定稿，暂以中性 JSON 值承载。
    pub payload: serde_json::Value, // tool_result `payload`
}

/// C3 审批决定（§2 逐字：allow / deny / always）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecisionKind {
    Allow,  // `allow`
    Deny,   // `deny`
    Always, // `always`
}

/// C3 审批请求快照：`{id, action, detail}`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub id: String, // `id`
    /// TODO(契约待定)：action 的取值域未在 §2 列出，暂以自由文本承载。
    pub action: String, // `action`
    pub detail: String, // `detail`
}

/// C3 审批结果快照：`{id, decision: allow|deny|always}`（UI → 运行时的发射方向，不属于只读 trait）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalDecision {
    pub id: String,                     // `id`
    pub decision: ApprovalDecisionKind, // `decision`
}

/// C4 停止投递：UI → 运行时 `agent:stop {sessionId, seq}`；运行时回执 `stopped`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopAck {
    pub session_id: String, // `sessionId`
    pub seq: u64,           // `seq`
    pub stopped: bool,      // 回执 `stopped`
}

/// C5 终端 PTY 快照：`{id, cwd, shell}` → 输出 chunk 流 + `interrupt`。
/// TODO(契约待定)：§2 C5 的「跨重启历史」未给字段形状，快照暂未落历史字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalChunk {
    pub id: String,    // `id`
    pub cwd: String,   // `cwd`
    pub shell: String, // `shell`
    /// TODO(契约待定)：chunk 的编码/粒度未定稿（§2 只说「输出 chunk 流」），暂按 UTF-8 文本承载。
    pub output: Vec<String>, // 输出 chunk 流
    pub interrupted: bool, // `interrupt` 已生效
}

/// 「视图 ← 运行时」数据源：**界面只消费**。
///
/// ⛔ 不实现运行时语义、⛔ 不发网络、⛔ 不落盘。每个能力一个只读方法，返回
/// [`LoadState`] 三态；来源由 [`Provenance`] 如实标注。
pub trait ViewDataSource {
    /// 数据来源标注（界面必须如实显示）。
    fn provenance(&self) -> Provenance;

    /// C1 会话流增量事件（§2：C2 工具卡片数据并入本流）。
    fn conversation(&self) -> LoadState<ConversationSnapshot>;

    /// C2 工具卡片数据（从 C1 流中抽出的卡片视图）。
    fn tool_cards(&self) -> LoadState<Vec<ToolCardSnapshot>>;

    /// C3 审批请求（待用户决定的队列）。审批结果由用户**发射**（UI → 运行时），
    /// 不属于本只读 trait。
    fn approvals(&self) -> LoadState<Vec<ApprovalRequest>>;

    /// C4 停止投递回执（`agent:stop {sessionId, seq}` 之后的 `stopped`）。
    fn stop_ack(&self) -> LoadState<StopAck>;

    /// C5 终端 PTY 快照。
    fn terminal(&self) -> LoadState<TerminalChunk>;
}

/// 演示态数据源：现阶段演示数据的**唯一出口**。
///
/// 契约未就绪 ⇒ 各能力如实返回 [`LoadState::Failed`]（fail-closed，⛔ 不许用空
/// 列表冒充"没有数据"，⛔ 不许伪造服务端数据）；来源标注 [`Provenance::Demo`]。
#[derive(Debug, Clone, Copy, Default)]
pub struct DemoSource;

impl ViewDataSource for DemoSource {
    fn provenance(&self) -> Provenance {
        Provenance::Demo
    }

    fn conversation(&self) -> LoadState<ConversationSnapshot> {
        LoadState::Failed("C1 会话流增量事件未接通（演示态）".to_string())
    }

    fn tool_cards(&self) -> LoadState<Vec<ToolCardSnapshot>> {
        LoadState::Failed("C2 工具卡片数据未接通（演示态）".to_string())
    }

    fn approvals(&self) -> LoadState<Vec<ApprovalRequest>> {
        LoadState::Failed("C3 审批请求未接通（演示态）".to_string())
    }

    fn stop_ack(&self) -> LoadState<StopAck> {
        LoadState::Failed("C4 停止投递回执未接通（演示态）".to_string())
    }

    fn terminal(&self) -> LoadState<TerminalChunk> {
        LoadState::Failed("C5 终端 PTY 未接通（演示态）".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApprovalDecision, ApprovalDecisionKind, ApprovalRequest, ConversationDeltaOp,
        ConversationItem, ConversationMessageKind, ConversationSnapshot, DemoSource, LoadState,
        Provenance, StopAck, TerminalChunk, ToolCardSnapshot, ToolResultKind, ViewDataSource,
    };

    #[test]
    fn failure_carries_its_reason() {
        let s: LoadState<u8> = LoadState::Failed("运行时未接入".to_string());
        assert_eq!(s.failure(), Some("运行时未接入"));
        assert!(!s.is_ready());
    }

    #[test]
    fn ready_has_no_failure() {
        let s: LoadState<u8> = LoadState::Ready(1);
        assert!(s.is_ready());
        assert_eq!(s.failure(), None);
    }

    #[test]
    fn demo_is_not_runtime() {
        assert!(Provenance::Demo.is_demo());
        assert!(!Provenance::Runtime.is_demo());
    }

    #[test]
    fn demo_source_is_labeled_demo() {
        assert!(DemoSource.provenance().is_demo());
    }

    /// 契约未就绪 ⇒ 如实 `Failed`（fail-closed，⛔ 不许拿空数据冒充）。
    fn expect_failed<T>(state: LoadState<T>) -> String {
        match state {
            LoadState::Failed(reason) => reason,
            _ => panic!("契约未就绪必须 fail-closed 返回 Failed，不许拿空数据冒充"),
        }
    }

    #[test]
    fn demo_source_fail_closed_with_reason() {
        let src = DemoSource;
        for (cap, reason) in [
            ("C1", expect_failed(src.conversation())),
            ("C2", expect_failed(src.tool_cards())),
            ("C3", expect_failed(src.approvals())),
            ("C4", expect_failed(src.stop_ack())),
            ("C5", expect_failed(src.terminal())),
        ] {
            assert!(!reason.is_empty(), "{cap} 失败原因不许为空");
            assert!(reason.contains(cap), "{cap} 原因须标注能力编号：{reason}");
            assert!(
                reason.contains("未接通"),
                "{cap} 原因须如实说明未接通：{reason}"
            );
        }
    }

    #[test]
    fn c1_conversation_snapshot_fields() {
        let snap = ConversationSnapshot {
            event: "delta".to_string(),
            op: ConversationDeltaOp::Append,
            batch: vec![ConversationItem {
                kind: ConversationMessageKind::AssistantText,
                payload: serde_json::json!({"text": "你好"}),
            }],
        };
        assert_eq!(snap.event, "delta");
        assert_eq!(snap.op, ConversationDeltaOp::Append);
        assert_eq!(snap.batch[0].kind, ConversationMessageKind::AssistantText);
        assert_eq!(snap.batch[0].payload["text"], "你好");
        // §2 C1 六类消息枚举均可表达
        let six = [
            ConversationMessageKind::User,
            ConversationMessageKind::AssistantText,
            ConversationMessageKind::ToolUse,
            ConversationMessageKind::ToolResult,
            ConversationMessageKind::Files,
            ConversationMessageKind::Approval,
        ];
        assert_eq!(six.len(), 6);
        // patch 操作同样可表达
        let patched = ConversationSnapshot {
            op: ConversationDeltaOp::Patch,
            ..snap
        };
        assert_eq!(patched.op, ConversationDeltaOp::Patch);
    }

    #[test]
    fn c2_tool_card_snapshot_fields() {
        let card = ToolCardSnapshot {
            name: "read_file".to_string(),
            args: serde_json::json!({"path": "/tmp/x"}),
            kind: ToolResultKind::Diff,
            payload: serde_json::json!({"diff": "+ 新增"}),
        };
        assert_eq!(card.name, "read_file");
        assert_eq!(card.kind, ToolResultKind::Diff);
        assert_eq!(card.args["path"], "/tmp/x");
        assert_eq!(card.payload["diff"], "+ 新增");
        // §2 C2 三种结果种类均可表达
        assert_ne!(ToolResultKind::Text, ToolResultKind::Files);
        assert_ne!(ToolResultKind::Files, ToolResultKind::Diff);
    }

    #[test]
    fn c3_approval_types() {
        let req = ApprovalRequest {
            id: "apr-1".to_string(),
            action: "run_command".to_string(),
            detail: "rm -rf /tmp/demo".to_string(),
        };
        let decision = ApprovalDecision {
            id: req.id.clone(),
            decision: ApprovalDecisionKind::Always,
        };
        assert_eq!(req.action, "run_command");
        assert_eq!(req.detail, "rm -rf /tmp/demo");
        assert_eq!(decision.id, "apr-1");
        assert_eq!(decision.decision, ApprovalDecisionKind::Always);
        // §2 C3 三种决定均可表达
        assert_ne!(ApprovalDecisionKind::Allow, ApprovalDecisionKind::Deny);
        assert_ne!(ApprovalDecisionKind::Deny, ApprovalDecisionKind::Always);
    }

    #[test]
    fn c4_stop_ack_fields() {
        let ack = StopAck {
            session_id: "sess-7".to_string(),
            seq: 42,
            stopped: true,
        };
        assert_eq!(ack.session_id, "sess-7");
        assert_eq!(ack.seq, 42);
        assert!(ack.stopped);
    }

    #[test]
    fn c5_terminal_chunk_fields() {
        let chunk = TerminalChunk {
            id: "pty-1".to_string(),
            cwd: "/home/u".to_string(),
            shell: "bash".to_string(),
            output: vec!["$ node -v".to_string(), "v20.0.0".to_string()],
            interrupted: false,
        };
        assert_eq!(chunk.id, "pty-1");
        assert_eq!(chunk.cwd, "/home/u");
        assert_eq!(chunk.shell, "bash");
        assert_eq!(chunk.output.len(), 2);
        assert!(!chunk.interrupted);
    }

    #[test]
    fn trait_object_usable() {
        let src: &dyn ViewDataSource = &DemoSource;
        assert!(src.provenance().is_demo());
        assert!(src.conversation().failure().is_some());
    }
}
