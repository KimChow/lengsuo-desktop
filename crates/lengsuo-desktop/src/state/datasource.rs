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

#[cfg(test)]
mod tests {
    use super::{LoadState, Provenance};

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
}
