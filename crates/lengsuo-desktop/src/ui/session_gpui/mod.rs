//! 会话流的 GPUI 接线层：把 [`crate::ui::session_render`] 的 [`Block`] 描述接到元素树。
//!
//! 分层约定（与 [`crate::ui::env_gpui`] 黄金范例一致）：
//! - **描述层**（`session_render`）：纯数据、无 GPUI 依赖、可在无窗口环境测。
//! - **接线层**（本模块）：`feature = "ui"` 隔离，只做「描述字段 → 样式」的映射，
//!   **不重新实现**任何业务判断。各块的渲染分支放在子模块 [`blocks`]。
//!
//! 七种块全部有渲染分支：用户气泡 / 助手正文 / 工具调用卡 / 工具结果（终端 ·
//! 文件列表 · diff 三态）/ 文件网格 / 审批卡 / 断档提示。
//!
//! 硬约束：
//! - 审批卡**渲染只读**：按钮点击只改局部界面态（[`SessionUiState`]），
//!   **绝不**调用 `resolve_approval`；未决策就显示「等待决策」，不得默认放行。
//! - 断档提示必须显眼（描述层保证它排在首块）。
//! - 颜色一律取自 `cx.theme()`，不硬编码。

#![cfg(feature = "ui")]

use std::borrow::Cow;
use std::collections::HashMap;

use gpui_kit::component::{ActiveTheme as _, Theme};
// 显式列举而不是 `use gpui_kit::*`：gpui 根上导出了自己的 `test` 属性宏，
// 通配导入会遮蔽内置的 `#[test]`，导致本文件的单元测试无法编译。
use gpui_kit::{
    div, AnyElement, Context, Div, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, Window,
};

pub mod approval_intent;
#[cfg(test)]
mod approval_intent_tests;
mod approval_render;
mod blocks;
pub mod diff_lines;
pub mod markdown;
mod markdown_view;
#[cfg(test)]
mod render_tests;

use crate::state::session_stream::{PendingApproval, SessionStreamState};
use crate::ui::approval_card::ApprovalAction;
use crate::ui::session_render::{self, Block};
use crate::ui::tool_card::ToolCard;

/// 正文最大宽度（助手正文用，避免长行铺满屏幕）。
pub(super) const CONTENT_MAX_W: f32 = 720.0;
/// 用户气泡最大宽度。
pub(super) const BUBBLE_MAX_W: f32 = 560.0;
/// 文件卡片宽度（网格按此换行）。
pub(super) const FILE_CARD_W: f32 = 180.0;

/// 会话流的**局部界面态**：展开 / 选中 / 审批按钮的本地选择。
///
/// 这些都是**界面态**，不写回 [`SessionStreamState`]：审批的真正决策仍要走
/// `resolve_approval`（本卡不接）。
#[derive(Debug, Clone, Default)]
pub struct SessionUiState {
    /// 工具调用卡片的展开状态（键为块序号）。
    pub expanded: HashMap<usize, bool>,
    /// 审批按钮的本地选择（键为审批 id）。
    pub approval_choice: HashMap<String, ApprovalAction>,
    /// 本地选中的文件路径。
    pub selected_file: Option<String>,
    /// 已选过「总是允许」的**作用域**（键为 [`approval_intent::ApprovalScope::key`]）。
    ///
    /// 只在内存、只在界面层；**不落盘**（持久化归宿主）。这张表只用于记住意图，
    /// **不得**据此跳过 / 自动通过任何后续审批（界面不判定权限）。
    pub always_allow_scope: approval_intent::AlwaysAllowMap,
    /// 选过「总是允许」的**本审批 id**（键为审批 id）；只用于该卡重绘后回显选中态。
    pub always_choice: HashMap<String, ApprovalAction>,
}

impl SessionUiState {
    /// 某块工具调用卡片是否展开（默认收起）。
    #[must_use]
    pub fn is_expanded(&self, index: usize, default_folded: bool) -> bool {
        self.expanded
            .get(&index)
            .copied()
            .unwrap_or(!default_folded)
    }

    /// 切换某块工具调用卡片的展开态。
    pub fn toggle_expanded(&mut self, index: usize, default_folded: bool) {
        let next = !self.is_expanded(index, default_folded);
        self.expanded.insert(index, next);
    }

    /// 记下审批按钮的本地选择（**不**代表真实决策）。
    pub fn choose_approval(&mut self, id: &str, action: ApprovalAction) {
        self.approval_choice.insert(id.to_owned(), action);
    }

    /// 某张待审批卡在重绘后应当显示的选中动作（按审批 id）。
    ///
    /// - `Allow` / `Deny` 取该审批 id 的本地选择；
    /// - `AlwaysAllow` 取该审批 id **自身的**「总是」选择。
    ///
    /// ⚠️ 只看本审批 id，**不**按作用域扩散：同工具 / 同会话里其它审批卡照样是未选态
    /// （见 `approval_intent` 的反例测试）。这只是**显示**，不改变审批状态。
    #[must_use]
    pub fn display_choice(&self, approval_id: &str) -> Option<ApprovalAction> {
        self.approval_choice
            .get(approval_id)
            .copied()
            .or_else(|| self.always_choice.get(approval_id).copied())
    }

    /// 按作用域查是否选过「总是允许」（仅反映意图，**不得**用于跳过审批）。
    #[must_use]
    pub fn scope_choice(&self, scope: &approval_intent::ApprovalScope) -> Option<ApprovalAction> {
        scope
            .key()
            .and_then(|key| self.always_allow_scope.get(&key).copied())
    }

    /// 产出并记下一次审批意图（**不**触发真实决策、不落盘）。
    ///
    /// - 「总是允许」且作用域已知 ⇒ 记入 [`Self::always_allow_scope`] 与
    ///   [`Self::always_choice`]（后者只让**本审批卡**回显选中态）；
    /// - 作用域未知 ⇒ 不记入任何表（fail-closed），意图仍原样返回给宿主；
    /// - 其它动作 ⇒ 走 [`Self::choose_approval`]。
    pub fn record_intent(
        &mut self,
        request: &approval_intent::ApprovalRequest<'_>,
    ) -> approval_intent::ApprovalIntent {
        let intent = approval_intent::ApprovalIntent::from_request(request);
        let Some(id) = request
            .approval_id
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            return intent;
        };
        if request.action.is_always() {
            if approval_intent::record_intent(&mut self.always_allow_scope, &intent)
                != approval_intent::Recorded::Skipped
            {
                self.always_choice
                    .insert(id.to_owned(), ApprovalAction::AlwaysAllow);
            }
        } else {
            self.choose_approval(id, request.action);
        }
        intent
    }

    /// 本地选中的文件路径。
    #[must_use]
    pub fn selected_file(&self) -> Option<&str> {
        self.selected_file.as_deref()
    }

    /// 选中某个文件（再点一次取消）。
    pub fn toggle_file(&mut self, path: &str) {
        if self.selected_file.as_deref() == Some(path) {
            self.selected_file = None;
        } else {
            self.selected_file = Some(path.to_owned());
        }
    }
}

/// 会话流的 GPUI 视图。
pub struct SessionStreamGpui {
    /// 会话状态：描述层的唯一数据来源。
    state: SessionStreamState,
    /// 局部界面态（不写回 `state`）。
    ui: SessionUiState,
}

impl SessionStreamGpui {
    /// 用一份会话状态构造视图。
    #[must_use]
    pub fn new(state: SessionStreamState) -> Self {
        Self {
            state,
            ui: SessionUiState::default(),
        }
    }

    /// 只读地取会话状态。
    #[must_use]
    pub fn state(&self) -> &SessionStreamState {
        &self.state
    }

    /// 可变地取会话状态（真实审批决策由调用方走 `resolve_approval`）。
    pub fn state_mut(&mut self) -> &mut SessionStreamState {
        &mut self.state
    }

    /// 只读地取局部界面态。
    #[must_use]
    pub fn ui_state(&self) -> &SessionUiState {
        &self.ui
    }

    /// 切换某块工具调用卡片的展开态。
    pub fn toggle_expanded(&mut self, index: usize, default_folded: bool) {
        self.ui.toggle_expanded(index, default_folded);
    }

    /// 记下审批按钮的本地选择（**不**触发真实决策）。
    pub fn choose_approval(&mut self, id: &str, action: ApprovalAction) {
        self.ui.choose_approval(id, action);
    }

    /// 产出并记下一次审批意图（**不**触发真实决策、不落盘）；返回的意图交宿主 / 运行时消费。
    pub fn record_intent(
        &mut self,
        request: &approval_intent::ApprovalRequest<'_>,
    ) -> approval_intent::ApprovalIntent {
        self.ui.record_intent(request)
    }
}

/// 渲染一个块所需的上下文。
///
/// - `theme`：颜色来源，一律取自 `cx.theme()`；
/// - `ui`：视图的局部界面态；
/// - `host`：交互宿主。给了才能点击（展开 / 选文件 / 审批按钮）；
/// - `index`：块在会话流中的序号（折叠态的键，也是元素 id 的唯一性来源）；
/// - `approvals`：渲染时的审批源（只读）。用于**只**取「审批 id → 工具名」，
///   让点击「总是允许」时能产出带作用域的意图；不参与任何放行判断。
pub struct BlockCtx<'a> {
    theme: &'a Theme,
    ui: Cow<'a, SessionUiState>,
    host: Option<&'a Entity<SessionStreamGpui>>,
    index: usize,
    approvals: Option<&'a [PendingApproval]>,
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
        }
    }

    /// 挂上审批源（只用于取工具名，供意图使用）。
    #[must_use]
    pub fn with_approvals(mut self, approvals: &'a [PendingApproval]) -> Self {
        self.approvals = Some(approvals);
        self
    }

    /// 某审批 id 的工具名（缺失返回 `None`；不伪造）。
    fn tool_of(&self, approval_id: &str) -> Option<&'a str> {
        self.approvals?
            .iter()
            .find(|a| a.id == approval_id)
            .map(|a| a.tool.as_str())
    }

    /// 该块是否处于展开态（工具调用卡片）。
    fn expanded(&self, card: &ToolCard) -> bool {
        self.ui.is_expanded(self.index, card.folded)
    }

    /// 生成一个在本视图内唯一的元素 id。
    fn element_id(&self, kind: &str, sub: usize) -> String {
        format!("session-{kind}-{}-{sub}", self.index)
    }
}

/// 给一个元素挂上点击回调（回调只改局部界面态）。
///
/// GPUI 的 `on_click` 只在**带 id 的**元素（`Stateful<Div>`）上可用，
/// 所以这里先 `.id(...)` 再挂回调。
pub(super) fn with_click(
    element: Div,
    id: String,
    host: Option<&Entity<SessionStreamGpui>>,
    action: impl Fn(&mut SessionStreamGpui) + 'static,
) -> AnyElement {
    let Some(host) = host.cloned() else {
        return element.into_any_element();
    };
    element
        .id(id)
        .cursor_pointer()
        .on_click(move |_event, _window, app| {
            host.update(app, |view, cx| {
                action(view);
                cx.notify();
            });
        })
        .into_any_element()
}

/// 渲染一个会话块（静态版：只出样式，不带点击回调）。
///
/// 交互版见 [`render_block_with`]。
#[must_use]
pub fn render_block(block: &Block) -> impl IntoElement {
    let theme = Theme::default();
    render_block_with(block, &BlockCtx::passive(&theme))
}

/// 渲染一个会话块（带主题与交互）。
///
/// 七种块的分支一个不少：`UserBubble` / `AssistantText` / `ToolCall` /
/// `ToolResult`（终端 · 文件列表 · diff）/ `FilesGrid` / `Approval` / `StaleNotice`。
#[must_use]
pub fn render_block_with(block: &Block, ctx: &BlockCtx<'_>) -> AnyElement {
    match block {
        Block::UserBubble { text, align } => blocks::render_user_bubble(text, *align, ctx),
        Block::AssistantText { text, align } => blocks::render_assistant_text(text, *align, ctx),
        Block::ToolCall(card) => blocks::render_tool_call(card, ctx),
        Block::ToolResult(view) => blocks::render_tool_result(view, ctx),
        Block::FilesGrid { files } => blocks::render_files_grid(files, ctx),
        Block::Approval(card) => approval_render::render_approval(card, ctx),
        Block::StaleNotice { text } => blocks::render_stale_notice(text, ctx),
    }
}

impl Render for SessionStreamGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let host = cx.entity();
        let blocks = session_render::render(&self.state);

        let mut root = div()
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .h_full()
            .p_3()
            .bg(theme.background)
            .text_color(theme.foreground);

        if blocks.is_empty() {
            return root.items_center().justify_center().child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("暂无消息"),
            );
        }

        for (index, block) in blocks.iter().enumerate() {
            let ctx = BlockCtx::interactive(&theme, &self.ui, &host, index)
                .with_approvals(self.state.approvals());
            root = root.child(render_block_with(block, &ctx));
        }
        root
    }
}
