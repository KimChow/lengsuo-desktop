//! 渲染一个会话块所需的上下文（从 [`super`] 拆出，控制单文件行数）。
//!
//! [`BlockCtx`] 只**取数据**：颜色、局部界面态、交互宿主、审批源、工具接线细节。
//! 它不做任何业务判断，也不写任何状态 —— 状态变更一律经 [`super::with_click`]
//! 回调到 [`super::SessionStreamGpui`]。

use std::borrow::Cow;

use gpui_kit::component::Theme;
use gpui_kit::Entity;

use super::tool_fold::ToolCallDetail;
use super::{SessionStreamGpui, SessionUiState};
use crate::state::session_stream::PendingApproval;

/// 渲染一个块所需的上下文。
///
/// - `theme`：颜色来源，一律取自 `cx.theme()`；
/// - `ui`：视图的局部界面态；
/// - `host`：交互宿主。给了才能点击（展开 / 选文件 / 审批按钮）；
/// - `index`：块在会话流中的序号（元素 id 的唯一性来源）；
/// - `approvals`：渲染时的审批源（只读）。用于**只**取「审批 id → 工具名」，
///   让点击「总是允许」时能产出带作用域的意图；不参与任何放行判断。
/// - `tools`：本会话流里所有工具调用的**接线细节**（按出现顺序），
///   供工具卡片取「消息 id + 完整参数」。
pub struct BlockCtx<'a> {
    pub(super) theme: &'a Theme,
    pub(super) ui: Cow<'a, SessionUiState>,
    pub(super) host: Option<&'a Entity<SessionStreamGpui>>,
    pub(super) index: usize,
    pub(super) approvals: Option<&'a [PendingApproval]>,
    pub(super) tools: Option<&'a [ToolCallDetail]>,
    pub(super) tool_ordinal: usize,
}

impl<'a> BlockCtx<'a> {
    /// 无交互的静态上下文（预览 / 测试用）。
    #[must_use]
    pub fn passive(theme: &'a Theme) -> Self {
        Self {
            theme,
            ui: Cow::Owned(SessionUiState::default()),
            host: None,
            index: 0,
            approvals: None,
            tools: None,
            tool_ordinal: 0,
        }
    }

    /// 交互上下文（`Render` 实现里用）。
    #[must_use]
    pub fn interactive(
        theme: &'a Theme,
        ui: &'a SessionUiState,
        host: &'a Entity<SessionStreamGpui>,
        index: usize,
    ) -> Self {
        Self {
            theme,
            ui: Cow::Borrowed(ui),
            host: Some(host),
            index,
            approvals: None,
            tools: None,
            tool_ordinal: 0,
        }
    }

    /// 挂上审批源（只用于取工具名，供意图使用）。
    #[must_use]
    pub fn with_approvals(mut self, approvals: &'a [PendingApproval]) -> Self {
        self.approvals = Some(approvals);
        self
    }

    /// 挂上工具调用的接线细节（消息 id + 完整参数）。
    #[must_use]
    pub fn with_tools(mut self, tools: &'a [ToolCallDetail]) -> Self {
        self.tools = Some(tools);
        self
    }

    /// 本块是会话流里的第几个工具调用（从 0 起）。
    #[must_use]
    pub fn with_tool_ordinal(mut self, ordinal: usize) -> Self {
        self.tool_ordinal = ordinal;
        self
    }

    /// 本块的工具接线细节（不是工具调用块则返回 `None`）。
    #[must_use]
    pub fn tool_detail(&self) -> Option<&'a ToolCallDetail> {
        self.tools?.get(self.tool_ordinal)
    }

    /// 某审批 id 的工具名（缺失返回 `None`；不伪造）。
    pub(super) fn tool_of(&self, approval_id: &str) -> Option<&'a str> {
        self.approvals?
            .iter()
            .find(|a| a.id == approval_id)
            .map(|a| a.tool.as_str())
    }

    /// 生成一个在本视图内唯一的元素 id。
    pub(super) fn element_id(&self, kind: &str, sub: usize) -> String {
        format!("session-{kind}-{}-{sub}", self.index)
    }
}
