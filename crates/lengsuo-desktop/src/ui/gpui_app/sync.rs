//! 根视图的纯函数：版式派生、发送回显、等待态解除判据。
//!
//! 这里**不依赖窗口**，全部可在单元测试里直接调用。

use crate::state::session_stream::{MessageRole, SessionMessage, SessionStreamState};
use crate::ui::input_area::OutgoingInput;
use crate::ui::layout::{RegionWidths, RightPanelLayout, WorkbenchLayout};

/// 由版式状态算三区宽度（渲染读宽度的**唯一入口**）。
///
/// 版式来自左分隔条持有的 [`WorkbenchLayout`]，本函数不持有任何状态，
/// 因此「渲染读到的宽度」与「分隔条持有的宽度」天然是同一份，不会有第二份副本。
///
/// fail-closed：窗口宽度取不到（NaN / 负值）时按 0 算 —— `regions` 会把会话流
/// 夹到 [`crate::ui::layout::STREAM_MIN_WIDTH`]，边栏 / 右栏取各自默认或当前宽度，三者都**不为 0**。
#[must_use]
pub(super) fn regions_for(layout: &WorkbenchLayout, viewport_width: f32) -> RegionWidths {
    let available = if viewport_width.is_finite() {
        viewport_width.max(0.0)
    } else {
        0.0
    };
    layout.regions(available)
}

/// 渲染右侧面板 / tab 条时应使用的版式：以左分隔条持有的版式为准。
///
/// 回写到左侧时保留 `current` 的宽度（宽度归分隔条管，tab 条只管选中 / 折叠），
/// 回写右侧时整份以左侧为准（选中 tab 随即生效）。
#[must_use]
pub(super) fn tab_strip_layout(
    authority: &WorkbenchLayout,
    current: &RightPanelLayout,
) -> RightPanelLayout {
    let mut layout = current.clone();
    layout.select_tab(authority.right_panel().tab());
    if authority.right_panel().is_collapsed() {
        layout.collapse();
    } else {
        layout.expand();
    }
    layout
}

/// 把一条已发出的输入变成会话流里的**本地回显**消息。
///
/// - 角色固定为 [`MessageRole::User`]，`tool` 为 `None`（用户消息不关联工具）。
/// - `ts_ms` 不取系统时钟（会让测试不确定）：取「新序号减一」，只保证单调，
///   **不是真实时间**。真实时间戳由运行时下发的事件携带。
/// - `seq` 由调用方用 [`next_echo_seq`] 从会话流序号派生，保证严格递增。
#[must_use]
pub(super) fn echo_message(next_seq: u64, out: &OutgoingInput) -> SessionMessage {
    SessionMessage {
        role: MessageRole::User,
        content: out.text.clone(),
        tool: None,
        ts_ms: next_seq.saturating_sub(1) as i64,
    }
}

/// 回显消息应使用的序号：会话流当前最大序号 + 1。
#[must_use]
pub(super) fn next_echo_seq(state: &SessionStreamState) -> u64 {
    state.last_seq() + 1
}

/// 「等待运行时确认」是否可以解除：存在已发出但未确认的消息，且会话流最新一条
/// 不是 `User`（即已有回应抵达）。
///
/// 会话流为空时**不解除**（没有回应）。回显本身是 `User` 消息，因此回显不会
/// 误触发解除 —— 必须等到非用户消息推进到末尾。
#[must_use]
pub(super) fn should_finish_send(pending: Option<u64>, state: &SessionStreamState) -> bool {
    if pending.is_none() {
        return false;
    }
    match state.messages().last() {
        Some(message) => message.role != MessageRole::User,
        None => false,
    }
}

/// 审批列表是否已与来源一致（只做读取侧同步的比较）。
///
/// 缺口 7 的收敛：根视图是唯一写入者，右栏只是读取侧副本；每次会话流变化时
/// 用本判据决定要不要把审批列表整体拷给右栏，避免 observe → notify → observe 循环。
#[must_use]
pub(super) fn approvals_in_sync(authority: &SessionStreamState, copy: &SessionStreamState) -> bool {
    authority.approvals() == copy.approvals()
}
