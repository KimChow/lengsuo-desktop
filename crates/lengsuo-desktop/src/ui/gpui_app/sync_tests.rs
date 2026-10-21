//! 根视图**同步层**的单元测试（无窗口可测）。
//!
//! ⛔ 文件级 `#![cfg(test)]`、不包一层 `mod tests {}`（clippy `module_inception`
//! ⇒ `-D warnings` 整卡失败）；⛔ 不用 `use super::*`（父模块的 glob 会把 GPUI 的
//! `test` 宏带进作用域并遮蔽标准库 `#[test]`，见速查表 §12.3）。
#![cfg(test)]

use crate::state::file_intent::{FileIntent, FileIntentSource};
use crate::state::session_stream::SessionStreamState;
use crate::ui::layout::RIGHT_PANEL_MAX_WIDTH;
use crate::ui::layout::RIGHT_PANEL_MIN_WIDTH;
use crate::ui::layout::{RightPanelLayout, RightPanelTab, WorkbenchLayout};
use crate::ui::right_panel_gpui::RightPanelGpui;

use crate::ui::gpui_app::sync::{
    adopt_right_panel_selection, apply_preview_intent, tab_strip_layout,
};

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

// ---- e25：点文件 → 跳预览的接线 ----

fn panel_with_tab(tab: RightPanelTab) -> RightPanelGpui {
    let mut layout = RightPanelLayout::new();
    layout.select_tab(tab);
    RightPanelGpui::from_state(&layout, &SessionStreamState::new())
}

/// A4：点文件必须让右栏的「文件预览」tab **被激活**（点了没反应 = 意图白产生）。
#[test]
fn a_new_intent_activates_the_preview_tab() {
    let mut panel = panel_with_tab(RightPanelTab::Tasks);
    let intent = FileIntent::new("src/main.rs", FileIntentSource::FileCard);

    assert!(apply_preview_intent(&mut panel, Some(intent.clone())));
    assert_eq!(
        panel.layout().tab(),
        RightPanelTab::Preview,
        "点了文件必须切到「文件预览」tab"
    );
    assert_eq!(panel.preview_intent(), Some(&intent));
}

/// 折叠态下点文件 ⇒ 窄条展开并切到预览 tab（否则用户看不到任何变化）。
#[test]
fn a_new_intent_expands_a_collapsed_panel() {
    let mut layout = RightPanelLayout::new();
    layout.collapse();
    let mut panel = RightPanelGpui::from_state(&layout, &SessionStreamState::new());

    let intent = FileIntent::new("a.rs", FileIntentSource::FileList);
    assert!(apply_preview_intent(&mut panel, Some(intent)));
    assert_eq!(panel.layout().tab(), RightPanelTab::Preview);
    assert!(
        !panel.layout().is_collapsed(),
        "折叠窄条里看不到 tab 内容，必须展开"
    );
}

/// 同一个意图重复下发 ⇒ 不算变化（避免 observe → notify → observe 循环）。
#[test]
fn the_same_intent_is_not_a_change() {
    let mut panel = panel_with_tab(RightPanelTab::Preview);
    let intent = FileIntent::new("src/main.rs", FileIntentSource::FileCard);

    assert!(apply_preview_intent(&mut panel, Some(intent.clone())));
    assert!(
        !apply_preview_intent(&mut panel, Some(intent)),
        "同一意图重复下发不应再报告变化"
    );
}

/// 换了目标文件 ⇒ 算变化（新意图替换旧意图，不累积）。
#[test]
fn a_different_intent_is_a_change() {
    let mut panel = panel_with_tab(RightPanelTab::Preview);
    let first = FileIntent::new("a.rs", FileIntentSource::FileCard);
    let second = FileIntent::new("b.rs", FileIntentSource::FileList);

    assert!(apply_preview_intent(&mut panel, Some(first)));
    assert!(apply_preview_intent(&mut panel, Some(second.clone())));
    assert_eq!(panel.preview_intent(), Some(&second), "只留最近一次意图");
}

/// 清掉意图 ⇒ 算变化，但**不动 tab**（用户可能已经切到别的 tab 了）。
#[test]
fn clearing_the_intent_does_not_move_the_tab() {
    let mut panel = panel_with_tab(RightPanelTab::Approvals);
    let intent = FileIntent::new("a.rs", FileIntentSource::FileCard);
    assert!(apply_preview_intent(&mut panel, Some(intent)));
    assert_eq!(panel.layout().tab(), RightPanelTab::Preview);

    assert!(apply_preview_intent(&mut panel, None));
    assert_eq!(panel.preview_intent(), None);
    assert_eq!(
        panel.layout().tab(),
        RightPanelTab::Preview,
        "清意图不回收 tab 选中态"
    );
}
