//! 边栏折叠的宿主侧接线：**下发**（权威版式 → 侧栏视图）与**回收**
//! （侧栏视图 → 权威版式）。
//!
//! ⛔ 本文件只做「权威 ↔ 副本」的同步，不实现折叠语义：
//! 折叠态的判定住在 [`crate::ui::layout::SidebarLayout`]（描述层）。
//!
//! **为什么必须成对**（与右栏 `adopt_right_panel_selection` 同一教训，
//! 2026-10-21 Windows 真机实测）：只做下发不做回收时，侧栏按钮改的是它私有的
//! `collapsed` 副本，紧接着下发会用权威版式里**旧**的状态覆盖回去 ⇒
//! 表现为「点了完全没反应」。编译、clippy 与单元测试都发现不了。

use gpui_kit::*;

use crate::ui::layout::WorkbenchLayout;

use super::AppShell;

/// 把侧栏视图刚产生的折叠态**收回**给权威版式。
///
/// 只回写折叠态，**不动宽度**（宽度归分隔条管，与右栏回收同款）。
/// 返回是否真的发生了改变，供调用方决定要不要 `notify()`（避免 observe 死循环）。
pub(super) fn adopt_sidebar_collapse(
    authority: &mut WorkbenchLayout,
    incoming_collapsed: bool,
) -> bool {
    let sidebar = authority.sidebar_mut();
    let changed = sidebar.is_collapsed() != incoming_collapsed;
    if changed {
        if incoming_collapsed {
            sidebar.collapse();
        } else {
            sidebar.expand();
        }
    }
    changed
}

impl AppShell {
    /// 回收：侧栏视图自己改了折叠态 ⇒ 回写给权威版式（左分隔条持有）。
    ///
    /// 与 [`Self::adopt_right_panel`](super::sync) 同型：读取子视图的 **bool**
    /// （不整份克隆视图），再就地改权威版式。
    pub(super) fn adopt_sidebar(&mut self, cx: &mut Context<Self>) {
        let incoming_collapsed = self.sidebar.read(cx).is_collapsed();
        self.left_divider.update(cx, |divider, cx| {
            if adopt_sidebar_collapse(divider.layout_mut(), incoming_collapsed) {
                cx.notify();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 的
    // `test` 属性宏带进作用域并遮蔽标准库的 `#[test]`（速查表 §12.3）。
    use super::adopt_sidebar_collapse;
    use crate::ui::layout::WorkbenchLayout;

    #[test]
    fn adopts_the_collapsed_flag_both_ways_and_reports_changes() {
        let mut authority = WorkbenchLayout::new();
        assert!(
            !adopt_sidebar_collapse(&mut authority, false),
            "没有变化时不得报告改变（避免 observe 死循环）"
        );
        assert!(adopt_sidebar_collapse(&mut authority, true));
        assert!(authority.sidebar().is_collapsed());
        assert!(
            !adopt_sidebar_collapse(&mut authority, true),
            "重复回收同一状态不算变化"
        );
        assert!(adopt_sidebar_collapse(&mut authority, false));
        assert!(!authority.sidebar().is_collapsed());
    }

    #[test]
    fn adopting_collapse_keeps_the_expanded_width() {
        let mut authority = WorkbenchLayout::new();
        authority.set_sidebar_width(300.0);
        assert!(adopt_sidebar_collapse(&mut authority, true));
        assert_eq!(
            authority.sidebar().width(),
            300.0,
            "回收折叠态不得改动展开宽度（宽度归分隔条管）"
        );
        assert_eq!(
            authority.sidebar().effective_width(),
            crate::ui::layout::SIDEBAR_COLLAPSED_WIDTH
        );
    }
}
