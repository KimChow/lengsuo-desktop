//! 右侧面板 tab 条的点击映射（纯逻辑，无 GPUI 依赖）。
//!
//! 点击只做一件事：把 tab 交给 [`RightPanelLayout::select_tab_id`]。
//! 折叠态点 tab 会展开面板 —— 这是 `layout.rs` 定下的契约，本层不另外判断。

use crate::ui::layout::{RightPanelLayout, RightPanelTab};

/// 一个 tab 入口的展示信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabEntry {
    /// tab 身份。
    pub tab: RightPanelTab,
    /// 完整标签（展开态）。
    pub label: &'static str,
    /// 紧凑标签（折叠窄条）。
    pub compact: String,
    /// 是否选中。
    pub selected: bool,
}

/// 由版式状态派生 tab 条：顺序固定为 [`RightPanelTab::ALL`]，三个入口始终存在。
#[must_use]
pub fn tab_entries(layout: &RightPanelLayout) -> Vec<TabEntry> {
    let selected = layout.tab();
    RightPanelTab::ALL
        .into_iter()
        .map(|tab| TabEntry {
            tab,
            label: tab.label(),
            compact: compact_label(tab.label()),
            selected: tab == selected,
        })
        .collect()
}

/// 点击 tab（裸 id 版本，供渲染层回调使用）：未知 id 不改动任何状态。
///
/// 返回是否真的处理了这个点击。
pub fn click_tab_id(layout: &mut RightPanelLayout, id: &str) -> bool {
    layout.select_tab_id(id)
}

/// 折叠窄条里的紧凑标签：取中文标签首字（与 `right_panel_gpui` 的展示口径一致）。
pub(crate) fn compact_label(label: &str) -> String {
    label.chars().take(1).collect()
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`），理由见速查表 §12.3。
    use super::{click_tab_id, tab_entries};
    use crate::ui::layout::{RightPanelLayout, RightPanelTab};

    #[test]
    fn entries_follow_layout_order_and_selection() {
        let layout = RightPanelLayout::new();
        let entries = tab_entries(&layout);
        assert_eq!(entries.len(), 3);
        let ids: Vec<&str> = entries.iter().map(|entry| entry.tab.id()).collect();
        assert_eq!(ids, vec!["tasks", "preview", "approvals"]);
        assert!(entries[0].selected);
        assert!(!entries[1].selected);
        assert_eq!(entries[1].compact, "文", "折叠窄条取首字");
    }

    #[test]
    fn clicking_a_tab_selects_it_and_reclicking_is_a_no_op() {
        let mut layout = RightPanelLayout::new();
        assert!(click_tab_id(&mut layout, "approvals"));
        assert_eq!(layout.tab(), RightPanelTab::Approvals);
        assert!(tab_entries(&layout)[2].selected);

        let before = layout.clone();
        assert!(click_tab_id(&mut layout, "approvals"), "重复点击仍算已处理");
        assert_eq!(layout.tab(), RightPanelTab::Approvals);
        assert_eq!(layout, before, "同 tab 再点不改变任何版式状态");
    }

    #[test]
    fn unknown_tab_id_is_rejected_without_changing_state() {
        let mut layout = RightPanelLayout::new();
        assert!(!click_tab_id(&mut layout, "history"));
        assert_eq!(layout.tab(), RightPanelTab::Tasks);
        assert!(!layout.is_collapsed());
    }

    #[test]
    fn clicking_a_tab_in_a_collapsed_panel_expands_it() {
        // 规则：折叠窄条只作恢复入口，点任意 tab 都展开并选中（含当前 tab）。
        let mut layout = RightPanelLayout::new();
        layout.collapse();
        assert!(layout.is_collapsed());

        assert!(click_tab_id(&mut layout, "tasks"));
        assert!(!layout.is_collapsed(), "点当前选中 tab 也展开");
        assert_eq!(layout.tab(), RightPanelTab::Tasks);

        layout.collapse();
        assert!(click_tab_id(&mut layout, "preview"));
        assert!(!layout.is_collapsed());
        assert_eq!(layout.tab(), RightPanelTab::Preview);
        assert_eq!(layout.width(), 320.0, "展开后恢复原宽度");
    }
}
