//! 审批「意图」与界面本地态：纯数据 + 纯函数，不依赖 GPUI，可在无窗口环境测。
//!
//! 职责分层（本卡 e11 只做第 ① 层）：
//! 1. **界面层（本文件）**：点击「总是允许」时产出 [`ApprovalIntent`]，
//!    并在内存里记下「哪个作用域被选过」；**不判定权限、不放宽任何可见性**。
//! 2. **运行时**：消费意图、按作用域执行（不在本卡）。
//! 3. **宿主**：负责策略**持久化**（本卡未做，也不做；本文件不落盘、不读凭据）。
//!
//! 铁律：
//! - 界面不判定权限：产出意图 ≠ 已生效。界面**不**因此隐藏 / 自动通过后续审批卡。
//! - fail-closed：审批 id 或工具名缺失时，作用域一律为 [`ApprovalScope::Unknown`]，
//!   ⛔ 绝不用默认值伪装成「对全部总是允许」。
//! - 内部工具名只进意图（机器对机器），**不进任何界面文案**。

use std::collections::HashMap;

use crate::ui::approval_card::ApprovalAction;

/// 作用域种类：说明「对什么总是允许」由哪一层界定。
///
/// 界面层只把**它能确定**的那一种写进意图；拿不准就 [`Self::Unknown`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// 本次会话（会话内一直允许）。
    Session,
    /// 某个工具（只对该工具一直允许）。
    Tool,
}

/// 「总是允许」的作用域：对**什么**总是允许。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApprovalScope {
    /// 只对本次会话生效。`session_id` 为 `None` 表示「当前会话」。
    Session { session_id: Option<String> },
    /// 只对某个工具生效。
    Tool { tool: String },
    /// 输入不足，作用域**未知**：运行时必须按最保守方式处理（不得视为全部放行）。
    Unknown,
}

impl ApprovalScope {
    /// 是否为「未知」作用域。运行时据此走保守路径。
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// 界面本地态的键（同键即同作用域）。未知作用域**没有**可用的键。
    #[must_use]
    pub fn key(&self) -> Option<String> {
        match self {
            Self::Session { session_id } => Some(match session_id {
                Some(id) => format!("session:{id}"),
                // "current" 而不是 "*"：避免读成「所有会话」。
                None => "session:current".to_owned(),
            }),
            Self::Tool { tool } => Some(format!("tool:{tool}")),
            Self::Unknown => None,
        }
    }
}

/// 一次点击的输入（界面层能拿到的全部信息）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalRequest<'a> {
    /// 审批 id（缺失 ⇒ 作用域未知）。
    pub approval_id: Option<&'a str>,
    /// 工具名（缺失 ⇒ 作用域未知）。
    pub tool: Option<&'a str>,
    /// 会话 id（可选；`None` 表示当前会话）。
    pub session_id: Option<&'a str>,
    /// 作用域种类。
    pub kind: ScopeKind,
    /// 用户选的动作。
    pub action: ApprovalAction,
}

impl<'a> ApprovalRequest<'a> {
    /// 便捷构造：只给审批 id 与工具名（作用域按工具）。
    #[must_use]
    pub fn tool_scoped(
        approval_id: Option<&'a str>,
        tool: Option<&'a str>,
        action: ApprovalAction,
    ) -> Self {
        Self {
            approval_id,
            tool,
            session_id: None,
            kind: ScopeKind::Tool,
            action,
        }
    }
}

/// 解析作用域（fail-closed，规则确定）。
///
/// - 审批 id 缺失 / 空白 ⇒ [`ApprovalScope::Unknown`]（连定位到哪个审批都做不到）；
/// - 工具名缺失 / 空白 ⇒ [`ApprovalScope::Unknown`]（工具名是策略定位键，缺了就未知，
///   **不**降级猜测成「对全部总是允许」）；
/// - 其余按 [`ScopeKind`]：`Tool` ⇒ 该工具；`Session` ⇒ 该会话（会话 id 缺失时为「当前会话」）。
#[must_use]
pub fn scope_for(request: &ApprovalRequest<'_>) -> ApprovalScope {
    let has_id = request.approval_id.is_some_and(|id| !id.trim().is_empty());
    let tool = request.tool.map(str::trim).filter(|t| !t.is_empty());
    if !has_id || tool.is_none() {
        return ApprovalScope::Unknown;
    }
    match request.kind {
        ScopeKind::Tool => ApprovalScope::Tool {
            tool: tool.expect("已判定存在").to_owned(),
        },
        ScopeKind::Session => ApprovalScope::Session {
            session_id: request
                .session_id
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
        },
    }
}

/// 一次审批点击产出的**意图**，交给运行时 / 宿主消费（界面不执行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalIntent {
    /// 产出意图的动作。
    pub action: ApprovalAction,
    /// 对什么总是允许。
    pub scope: ApprovalScope,
    /// 触发本次意图的审批 id（原样带上，空串表示缺失）。
    pub approval_id: String,
    /// 工具名；缺失为 `None`（不伪造）。
    pub tool: Option<String>,
}

impl ApprovalIntent {
    /// 从点击处的输入产出意图。
    ///
    /// 只有 [`ApprovalAction::AlwaysAllow`] 才是「总是」语义；其它动作的作用域一律
    /// [`ApprovalScope::Unknown`]（它们不构成策略）。
    #[must_use]
    pub fn from_request(request: &ApprovalRequest<'_>) -> Self {
        let scope = if request.action.is_always() {
            scope_for(request)
        } else {
            ApprovalScope::Unknown
        };
        Self {
            action: request.action,
            scope,
            approval_id: request.approval_id.unwrap_or_default().to_owned(),
            tool: request
                .tool
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned),
        }
    }

    /// 是否为「总是允许」且作用域已知（界面可以记入本地态）。
    #[must_use]
    pub fn is_scoped_always(&self) -> bool {
        self.action.is_always() && !self.scope.is_unknown()
    }
}

/// 界面层内存态：作用域 → 动作。只在内存、只在界面层，**不落盘**。
pub type AlwaysAllowMap = HashMap<String, ApprovalAction>;

/// 把意图记入作用域表的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recorded {
    /// 新记入一个作用域。
    Inserted,
    /// 同作用域重复「总是允许」——幂等，未新增条目。
    Idempotent,
    /// 意图不是「总是允许」或作用域未知——**不记入**（fail-closed）。
    Skipped,
}

/// 把意图记入作用域表（按作用域键，幂等）。返回是否真正新增了条目。
pub fn record_intent(map: &mut AlwaysAllowMap, intent: &ApprovalIntent) -> Recorded {
    if !intent.is_scoped_always() {
        return Recorded::Skipped;
    }
    let Some(key) = intent.scope.key() else {
        return Recorded::Skipped;
    };
    if let std::collections::hash_map::Entry::Vacant(slot) = map.entry(key) {
        slot.insert(intent.action);
        Recorded::Inserted
    } else {
        Recorded::Idempotent
    }
}
