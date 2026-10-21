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

/// 把 tab 条 / 右侧面板刚产生的选中态与折叠态**收回**给权威版式。
///
/// 与 [`tab_strip_layout`] 方向相反，两者必须成对使用：
/// - `tab_strip_layout` 是**下发**（权威 → 子视图）；
/// - 本函数是**回收**（子视图 → 权威），在子视图自己 `notify()` 之后、下发之前调用。
///
/// 2026-10-21 Windows 真机实测教训：只做下发不做回收时，tab 条的点击会改它私有的
/// `RightPanelLayout`，紧接着 `sync_layout` 用权威版式里**旧的**选中态覆盖回去，
/// 表现为「点 tab 完全没反应」。当时编译、clippy、289 个单元测试全绿 ——
/// 因为纯函数 `tab_strip_layout` 本身没错，错的是**调用顺序**缺了一环。
///
/// 只回写选中 tab 与折叠态，**不动宽度**（宽度归分隔条管）。
/// 返回是否真的发生了改变，供调用方决定要不要 `notify()`（避免 observe 死循环）。
pub(super) fn adopt_right_panel_selection(
    authority: &mut WorkbenchLayout,
    incoming: &RightPanelLayout,
) -> bool {
    let panel = authority.right_panel_mut();
    let changed = panel.tab() != incoming.tab() || panel.is_collapsed() != incoming.is_collapsed();
    if changed {
        panel.select_tab(incoming.tab());
        if incoming.is_collapsed() {
            panel.collapse();
        } else {
            panel.expand();
        }
    }
    changed
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

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 glob 会把 GPUI 的 `test` 属性宏
    // 带进作用域并遮蔽标准 `#[test]`（速查表 §12.3）。
    use super::{adopt_right_panel_selection, tab_strip_layout};
    use crate::ui::layout::{RightPanelLayout, RightPanelTab, WorkbenchLayout};

    /// 回归测试：**只下发不回收** ⇒ 点 tab 的选中态会被下一次下发覆盖回去。
    ///
    /// 这正是 2026-10-21 Windows 真机实测发现的缺陷（编译 + 289 测试全绿也测不到，
    /// 因为错的不是纯函数，而是根视图少了一次「回收」）。
    #[test]
    fn without_adopt_a_tab_click_is_lost() {
        let mut authority = WorkbenchLayout::new();
        // tab 条被点击：它自己的版式变成 Preview
        let mut strip = RightPanelLayout::new();
        strip.select_tab(RightPanelTab::Preview);
        // 缺了「回收」这一步，直接下发：权威版式里仍是旧 tab
        let pushed = tab_strip_layout(&authority, &strip);
        assert_eq!(
            pushed.tab(),
            RightPanelTab::Tasks,
            "没有回收时，下发应把点击覆盖回旧 tab（这就是 bug 的表现）"
        );
        // 补上回收后，权威版式才跟上
        let changed = adopt_right_panel_selection(&mut authority, &strip);
        assert!(changed, "回收必须报告发生了改变");
        assert_eq!(authority.right_panel().tab(), RightPanelTab::Preview);
        assert_eq!(
            tab_strip_layout(&authority, &strip).tab(),
            RightPanelTab::Preview
        );
    }

    /// 回收必须**幂等**：同一状态再收一次返回 false（否则 observe 会死循环）。
    #[test]
    fn adopt_is_idempotent() {
        let mut authority = WorkbenchLayout::new();
        let mut strip = RightPanelLayout::new();
        strip.select_tab(RightPanelTab::Approvals);
        assert!(adopt_right_panel_selection(&mut authority, &strip));
        assert!(
            !adopt_right_panel_selection(&mut authority, &strip),
            "第二次回收不应再报告改变"
        );
    }

    /// 回收**不许动宽度**：宽度归分隔条管，tab 条无权改。
    #[test]
    fn adopt_does_not_touch_width() {
        let mut authority = WorkbenchLayout::new();
        let before = authority.right_panel().width();
        let mut strip = RightPanelLayout::new();
        strip.select_tab(RightPanelTab::Preview);
        adopt_right_panel_selection(&mut authority, &strip);
        assert_eq!(
            authority.right_panel().width(),
            before,
            "回收只改选中/折叠，不应改宽度"
        );
    }

    /// 折叠态也要能被回收（否则折叠后点 tab 恢复不了展开）。
    #[test]
    fn adopt_carries_collapsed_state() {
        let mut authority = WorkbenchLayout::new();
        let mut strip = RightPanelLayout::new();
        strip.collapse();
        assert!(adopt_right_panel_selection(&mut authority, &strip));
        assert!(authority.right_panel().is_collapsed());
        assert!(authority.right_panel().is_collapsed());
    }
}

#[cfg(test)]
mod width_sync_tests {
    // 显式导入（不用 `use super::*`），理由见速查表 §12.3。
    use crate::ui::layout::{WorkbenchLayout, RIGHT_PANEL_MAX_WIDTH, RIGHT_PANEL_MIN_WIDTH};

    /// 回归测试：右分隔条拖出来的宽度必须能**回收**给权威版式。
    ///
    /// 2026-10-21 Windows 真机实测：只做「权威 → 右分隔条」下发、不做回收时，
    /// 右分隔条拖完立刻被旧宽度覆盖 ⇒ 表现成「右栏被锁死、拖不动」。
    #[test]
    fn right_divider_width_round_trips_through_authority() {
        let mut authority = WorkbenchLayout::new();
        let mut right_divider = WorkbenchLayout::new();

        // 模拟右分隔条被拖动：它自己的宽度变小
        let target = authority.right_panel().width() - 60.0;
        right_divider.right_panel_mut().set_width(target);
        assert_eq!(right_divider.right_panel().width(), target);

        // 回收：把它写回权威
        let incoming = right_divider.right_panel().width();
        let panel = authority.right_panel_mut();
        if panel.width() != incoming {
            panel.set_width(incoming);
        }

        // 下发：权威 → 右分隔条应保持 target，而不是退回默认值
        assert_eq!(
            authority.right_panel().width(),
            target,
            "回收后权威必须是拖动结果"
        );
        assert_eq!(
            authority.right_panel().width(),
            right_divider.right_panel().width(),
            "两侧必须一致，否则下一次下发会把拖动冲掉"
        );
    }

    /// 宽度夹取仍由描述层独占：回收越界值不会突破 min/max。
    #[test]
    fn adopted_width_is_still_clamped() {
        let mut authority = WorkbenchLayout::new();
        authority.right_panel_mut().set_width(99_999.0);
        assert_eq!(authority.right_panel().width(), RIGHT_PANEL_MAX_WIDTH);

        authority.right_panel_mut().set_width(-500.0);
        assert_eq!(authority.right_panel().width(), RIGHT_PANEL_MIN_WIDTH);
    }
}
