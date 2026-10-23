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
//!   点击产出的意图经 [`SessionStreamGpui::approval_intent`]（只读）/
//!   [`SessionStreamGpui::take_approval_intent`]（一次性投递）交给宿主转运行时；
//!   界面本身不执行它。
//! - 断档提示必须显眼（描述层保证它排在首块）。
//! - 颜色一律取自 `cx.theme()`，不硬编码。

#![cfg(feature = "ui")]

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
mod block_ctx;
mod blocks;
pub mod diff_lines;
pub mod diff_split;
#[cfg(test)]
mod diff_split_tests;
mod diff_split_view;
mod file_card;
pub mod markdown;
mod markdown_view;
#[cfg(test)]
mod render_tests;
#[cfg(test)]
mod tool_card_tests;
mod tool_card_view;
pub mod tool_fold;
#[cfg(test)]
mod tool_fold_tests;

pub use block_ctx::BlockCtx;

use crate::state::datasource::{DemoSource, LoadState, Provenance, ViewDataSource};
use crate::state::file_intent::{FileIntent, FileIntentSource, FileIntentState};
use crate::state::session_stream::{apply_delta, SessionStreamState};
use crate::ui::approval_card::ApprovalAction;
use crate::ui::session_render::{self, Block};

/// 正文最大宽度（助手正文用，避免长行铺满屏幕）。
pub(super) const CONTENT_MAX_W: f32 = 720.0;
/// 用户气泡最大宽度。
pub(super) const BUBBLE_MAX_W: f32 = 560.0;
/// 文件卡片宽度（网格按此换行）。
pub(super) const FILE_CARD_W: f32 = 180.0;

/// 会话流的**局部界面态**：折叠 / 选中 / 审批按钮的本地选择。
///
/// 这些都是**界面态**，不写回 [`SessionStreamState`]：审批的真正决策仍要走
/// `resolve_approval`（本视图不接；意图由宿主经 `take_approval_intent()` 取走）。
#[derive(Debug, Clone, Default)]
pub struct SessionUiState {
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
    /// 最近一次审批点击产出的**意图**（只留最近一次，不累积）。
    ///
    /// 供宿主经 [`SessionStreamGpui::approval_intent`] / [`SessionStreamGpui::take_approval_intent`]
    /// 取走并转交运行时。界面**不执行**它：不调 `resolve_approval`、不放行、不落盘。
    pub approval_intent: Option<approval_intent::ApprovalIntent>,
    /// 工具卡片的折叠态（键为**消息 id**，见 [`tool_fold::tool_call_id`]）。
    ///
    /// 只在内存、只在界面层；**不落盘**（重启回到默认折叠）。
    ///
    /// ⚠️ 键**必须**是消息 id 而不是块序号：块序号会随新消息插入而位移，
    /// 折叠态就会记到别的卡片上。
    pub tool_folded: HashMap<String, bool>,
    /// 「点文件 → 跳预览」的**跳转意图**（只留最近一次）。
    ///
    /// ⛔ **预览器未实现**（属 P3，见 `ui-parity-spec.md` A14）：这里只记**意图**，
    /// 不读文件、不开窗口、**不落盘**。宿主将来消费它即可。
    pub file_intent: FileIntentState,
}

impl SessionUiState {
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
        // ★ 关键：无论能否记入本地作用域表，意图本身都要交给宿主。
        //   作用域是否 Unknown 由运行时按最保守方式判定，界面不替它决定。
        self.approval_intent = Some(intent.clone());
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

    /// 最近一次审批点击的**意图**（没点过时是 `None`）。
    ///
    /// 只读；宿主可反复读（便于复核与重放）。
    #[must_use]
    pub fn approval_intent(&self) -> Option<&approval_intent::ApprovalIntent> {
        self.approval_intent.as_ref()
    }

    /// **取走**最近一次审批意图（一次性投递：取走后变 `None`，避免重复投递）。
    pub fn take_approval_intent(&mut self) -> Option<approval_intent::ApprovalIntent> {
        self.approval_intent.take()
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

    /// 当前「点文件 → 跳预览」的**跳转意图**（没点过时是 `None`）。
    #[must_use]
    pub fn file_intent(&self) -> Option<&FileIntent> {
        self.file_intent.intent()
    }

    /// 记下一次跳转意图（**替换**旧的，不累积）。
    ///
    /// ⛔ 预览器未实现（P3）：只记意图，**不**读文件、**不**落盘。
    pub fn request_file_preview(&mut self, path: &str, source: FileIntentSource) {
        self.file_intent.request(path, source);
    }
}

/// 会话流的 GPUI 视图。
pub struct SessionStreamGpui {
    /// 会话状态：描述层的唯一数据来源。
    state: SessionStreamState,
    /// 局部界面态（不写回 `state`）。
    ui: SessionUiState,
    /// 数据来源标注（契约未就绪 ⇒ [`Provenance::Demo`]，界面须如实显示）。
    source: Provenance,
    /// C1 会话流读取的三态（fail-closed：`Failed` 必须显示错误，不退化成空列表）。
    load: LoadState<()>,
}

impl SessionStreamGpui {
    /// 用一份会话状态构造视图（宿主直塞状态）。
    ///
    /// 来源按 [`Provenance::Demo`] 如实标注（⛔ 不标 `Runtime`——真实来源必须走
    /// [`Self::from_source`]，不许伪造）。
    #[must_use]
    pub fn new(state: SessionStreamState) -> Self {
        Self {
            state,
            ui: SessionUiState::default(),
            source: Provenance::Demo,
            load: LoadState::Ready(()),
        }
    }

    /// 从「视图 ← 运行时」数据源构造视图（消费侧契约出口；三态如实表达）：
    ///
    /// - `Ready` ⇒ 从空状态按 C1 增量批次（[`apply_delta`]）水合；
    /// - `Loading` ⇒ 渲染加载态文案（不显示「暂无消息」冒充空会话）；
    /// - `Failed` ⇒ **fail-closed**：渲染错误文案，并保持演示会话
    ///   （契约未就绪 ⇒ 保持 demo + 如实标注，⛔ 不伪造运行时数据、不退化成空列表）。
    #[must_use]
    pub fn from_source(src: &impl ViewDataSource) -> Self {
        let mut state = SessionStreamState::new();
        let load = match src.conversation() {
            LoadState::Ready(snapshot) => {
                // 跳过计数（DeltaOutcome）供调用方审计；视图按水合结果如实展示。
                let _ = apply_delta(&mut state, &snapshot);
                LoadState::Ready(())
            }
            LoadState::Loading => LoadState::Loading,
            LoadState::Failed(reason) => {
                state = crate::state::session_demo::demo_session();
                LoadState::Failed(reason)
            }
        };
        Self {
            state,
            ui: SessionUiState::default(),
            source: src.provenance(),
            load,
        }
    }

    /// 演示用视图：[`Self::from_source`] 的薄封装（演示态数据源 ⇒ 保持演示会话 + 如实标注）。
    #[must_use]
    pub fn demo() -> Self {
        Self::from_source(&DemoSource)
    }

    /// 数据来源标注（契约未就绪 ⇒ `Demo`；界面须如实显示）。
    #[must_use]
    pub fn provenance(&self) -> Provenance {
        self.source
    }

    /// C1 会话流读取的三态（`Failed` 的原因供界面显示）。
    #[must_use]
    pub fn load_state(&self) -> &LoadState<()> {
        &self.load
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

    /// 切换某张工具卡片的折叠态（按**消息 id**；只在内存，**不落盘**）。
    pub fn toggle_tool_fold(&mut self, id: &str) {
        self.ui.toggle_tool_fold(id);
    }

    /// 记下审批按钮的本地选择（**不**触发真实决策）。
    pub fn choose_approval(&mut self, id: &str, action: ApprovalAction) {
        self.ui.choose_approval(id, action);
    }

    /// 当前「点文件 → 跳预览」的**跳转意图**（没点过时是 `None`）。
    ///
    /// ⛔ **预览器未实现**（属 P3）：宿主将来读它去开预览器；本卡只产出意图。
    #[must_use]
    pub fn file_intent(&self) -> Option<&FileIntent> {
        self.ui.file_intent()
    }

    /// 记下一次跳转意图（**替换**旧的，不累积；⛔ 不落盘、不读文件）。
    pub fn request_file_preview(&mut self, path: &str, source: FileIntentSource) {
        self.ui.request_file_preview(path, source);
    }

    /// 产出并记下一次审批意图（**不**触发真实决策、不落盘）；返回的意图交宿主 / 运行时消费。
    pub fn record_intent(
        &mut self,
        request: &approval_intent::ApprovalRequest<'_>,
    ) -> approval_intent::ApprovalIntent {
        self.ui.record_intent(request)
    }

    /// 最近一次审批点击的**意图**（只读；宿主据此转交运行时）。
    ///
    /// ⛔ 界面不执行它：不调 `resolve_approval`、不放行、不落盘。
    #[must_use]
    pub fn approval_intent(&self) -> Option<&approval_intent::ApprovalIntent> {
        self.ui.approval_intent()
    }

    /// **取走**最近一次审批意图（一次性投递）。
    pub fn take_approval_intent(&mut self) -> Option<approval_intent::ApprovalIntent> {
        self.ui.take_approval_intent()
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

/// 三态在会话流顶部的提示文案（`Ready` ⇒ 无提示）。
///
/// - `Loading` ⇒ 「加载中…」（界面文案逐字取自文案全表，不新编）；
/// - `Failed` ⇒ 原样给出失败原因（来自数据源；fail-closed：⛔ 不吞错误、
///   ⛔ 不退化成空列表）。
#[must_use]
pub fn load_banner_text(load: &LoadState<()>) -> Option<String> {
    match load {
        LoadState::Loading => Some("加载中…".to_string()),
        LoadState::Failed(reason) => Some(reason.clone()),
        LoadState::Ready(()) => None,
    }
}

/// 三态提示条（样式；颜色一律取自主题 token，不硬编码）。
fn source_banner(text: String, theme: &Theme, failed: bool) -> Div {
    div()
        .w_full()
        .px_3()
        .py_2()
        .text_sm()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.muted)
        .text_color(if failed {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .child(text)
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
        // 工具调用的接线细节（消息 id + 完整参数）：与 `Block::ToolCall` 块一一对应。
        let tools = tool_fold::tool_call_details(self.state.messages());

        let mut root = div()
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .h_full()
            .p_3()
            .bg(theme.background)
            .text_color(theme.foreground);

        // 三态如实表达：Loading / Failed 在流顶部给出可见提示
        //（fail-closed：不吞错误，也不用「暂无消息」冒充加载中 / 失败态）。
        let banner_text = load_banner_text(&self.load);
        if blocks.is_empty() && banner_text.is_none() {
            return root.items_center().justify_center().child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("暂无消息"),
            );
        }
        if let Some(text) = banner_text {
            let failed = self.load.failure().is_some();
            root = root.child(source_banner(text, &theme, failed));
        }

        let mut tool_ordinal = 0usize;
        for (index, block) in blocks.iter().enumerate() {
            let ctx = BlockCtx::interactive(&theme, &self.ui, &host, index)
                .with_approvals(self.state.approvals())
                .with_tools(&tools)
                .with_tool_ordinal(tool_ordinal);
            if matches!(block, Block::ToolCall(_)) {
                tool_ordinal += 1;
            }
            root = root.child(render_block_with(block, &ctx));
        }
        root
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`，与 render_tests 一致）：通配会把 GPUI 的
    // `test` 宏带进作用域，遮蔽标准库 `#[test]`，导致测试无法编译。
    use super::{load_banner_text, SessionStreamGpui};
    use crate::state::datasource::{
        ConversationDeltaOp, ConversationItem, ConversationMessageKind, ConversationSnapshot,
        DemoSource, LoadState, Provenance, TerminalChunk, ToolCardSnapshot, ViewDataSource,
    };
    use serde_json::json;

    /// 只接通 C1 的桩数据源（其余能力如实 Failed，与 DemoSource 同口径）。
    struct StubSource {
        conversation: LoadState<ConversationSnapshot>,
    }

    impl ViewDataSource for StubSource {
        fn provenance(&self) -> Provenance {
            Provenance::Demo
        }

        fn conversation(&self) -> LoadState<ConversationSnapshot> {
            self.conversation.clone()
        }

        fn tool_cards(&self) -> LoadState<Vec<ToolCardSnapshot>> {
            LoadState::Failed("未接通".into())
        }

        fn approvals(&self) -> LoadState<Vec<crate::state::datasource::ApprovalRequest>> {
            LoadState::Failed("未接通".into())
        }

        fn stop_ack(&self) -> LoadState<crate::state::datasource::StopAck> {
            LoadState::Failed("未接通".into())
        }

        fn terminal(&self) -> LoadState<TerminalChunk> {
            LoadState::Failed("未接通".into())
        }
    }

    #[test]
    fn demo_is_thin_wrapper_of_from_source() {
        let demo = SessionStreamGpui::demo();
        let via_source = SessionStreamGpui::from_source(&DemoSource);
        assert_eq!(
            demo.state(),
            via_source.state(),
            "demo() 必须仍是 from_source 的薄封装（演示会话原样保留）"
        );
        assert!(demo.provenance().is_demo(), "契约未就绪 ⇒ 来源仍标 Demo");
        assert!(
            !demo.state().messages().is_empty(),
            "演示视图不得退化成空列表"
        );
    }

    #[test]
    fn failed_source_renders_error_and_keeps_demo() {
        let view = SessionStreamGpui::from_source(&DemoSource);
        let reason = view.load_state().failure().expect("Failed 必须带失败原因");
        let banner =
            load_banner_text(view.load_state()).expect("Failed 必须产出可见错误文案，不 panic");
        assert_eq!(banner, reason, "错误文案须原样携带失败原因");
        assert!(!view.state().messages().is_empty());
    }

    #[test]
    fn ready_source_hydrates_via_append_delta() {
        let src = StubSource {
            conversation: LoadState::Ready(ConversationSnapshot {
                event: "delta".into(),
                op: ConversationDeltaOp::Append,
                batch: vec![
                    ConversationItem {
                        kind: ConversationMessageKind::User,
                        payload: json!({"text": "你好"}),
                    },
                    ConversationItem {
                        kind: ConversationMessageKind::AssistantText,
                        payload: json!({"text": "在的"}),
                    },
                ],
            }),
        };
        let view = SessionStreamGpui::from_source(&src);
        assert_eq!(view.state().messages().len(), 2, "Ready ⇒ 按增量批次水合");
        assert_eq!(view.state().messages()[0].content, "你好", "顺序必须稳定");
        assert!(view.load_state().is_ready());
        assert_eq!(
            load_banner_text(view.load_state()),
            None,
            "Ready ⇒ 无提示条"
        );
    }

    #[test]
    fn loading_source_shows_loading_banner_not_empty_hint() {
        let src = StubSource {
            conversation: LoadState::Loading,
        };
        let view = SessionStreamGpui::from_source(&src);
        assert!(view.state().messages().is_empty());
        assert_eq!(
            load_banner_text(view.load_state()).as_deref(),
            Some("加载中…"),
            "Loading 态显示加载文案，不拿「暂无消息」冒充"
        );
    }
}
