//! 三区布局的**渲染描述类型**（纯数据、无 GPUI 依赖，可在无窗口环境测）。
//!
//! 由 `ui/layout.rs` 拆出（`e33-layout-split`）：只放「状态 → 描述」的形状，
//! 不含版式状态本身。

use super::{RegionWidths, WorkbenchLayout, INPUT_PLACEHOLDER, STREAM_EMPTY_TEXT};
use crate::state::session_stream::SessionStreamState;
use crate::ui::input_area::{InputAreaState, InputMode, SendButton};
use crate::ui::right_panel::{self, RightPanelView};
use crate::ui::session_render::{self, Block};
use crate::ui::sidebar::{self, SidebarState, SidebarView};

/// 会话流区域视图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamView {
    /// 消息 / 工具卡片 / 审批 / 断档提示（顺序即渲染顺序）。
    pub blocks: Vec<Block>,
    /// 是否处于事件流断档态（与 `blocks` 首块的断档提示一致）。
    pub stale: bool,
    /// 没有任何可渲染内容时的占位文案。
    pub empty_text: Option<&'static str>,
}

/// 输入区模式按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeButton {
    /// 稳定 id（`quick` / `ask` / ...）。
    pub id: &'static str,
    pub label: &'static str,
    pub selected: bool,
}

/// 输入区视图：多行草稿 + 5 模式 + 发送按钮。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputView {
    pub draft: String,
    pub line_count: usize,
    pub placeholder: &'static str,
    pub modes: Vec<ModeButton>,
    pub send: SendButton,
}

/// 完整工作台的渲染描述。
#[derive(Debug, Clone, PartialEq)]
pub struct WorkbenchView {
    pub widths: RegionWidths,
    pub sidebar: SidebarView,
    pub stream: StreamView,
    pub right_panel: RightPanelView,
    pub input: InputView,
}

impl WorkbenchLayout {
    /// 生成完整工作台的渲染描述。
    ///
    /// 会话流块由 [`session_render::render`] 生成：断档提示、四类消息、
    /// 待审批卡片全部只读消费 [`SessionStreamState`]。
    #[must_use]
    pub fn render(
        &self,
        session: &SessionStreamState,
        sidebar_state: &SidebarState,
        input: &InputAreaState,
        available_width: f32,
    ) -> WorkbenchView {
        let widths = self.regions(available_width);
        let blocks = session_render::render(session);
        WorkbenchView {
            widths,
            sidebar: sidebar::render(sidebar_state, widths.sidebar),
            stream: StreamView {
                stale: session.is_stale(),
                empty_text: blocks.is_empty().then_some(STREAM_EMPTY_TEXT),
                blocks,
            },
            right_panel: right_panel::render(&self.right_panel, session),
            input: input_view(input),
        }
    }
}

fn input_view(input: &InputAreaState) -> InputView {
    let mode = input.mode();
    InputView {
        draft: input.draft().to_owned(),
        line_count: input.line_count(),
        placeholder: INPUT_PLACEHOLDER,
        modes: InputMode::ALL
            .iter()
            .map(|&candidate| ModeButton {
                id: candidate.id(),
                label: candidate.label(),
                selected: candidate == mode,
            })
            .collect(),
        send: input.send_button(),
    }
}
