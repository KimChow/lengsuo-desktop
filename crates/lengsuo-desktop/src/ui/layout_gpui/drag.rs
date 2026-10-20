//! 分隔条拖拽状态机（纯逻辑，无 GPUI 依赖，随 `cargo test` 一起跑）。
//!
//! 拖拽有三份状态：是否在拖、按下时的鼠标 x、按下时的面板宽度。
//! 这些状态**只属于界面本地**（本卡放在 [`DividerDrag`] 里），**不进**
//! [`WorkbenchLayout`] —— 描述层只保存宽度结果，不保存交互过程。
//!
//! 宽度换算：`目标宽度 = 起始宽度 + 鼠标位移 × 方向`。目标宽度交给描述层的
//! `resize_sidebar_by` / `right_panel_mut().resize_by` 做夹取（夹取边界只在
//! `layout.rs` 一处实现，本文件不重复算）。每次实际下发的是「目标 − 当前」，
//! 因此拖到边界后再往回拖能立刻响应，不会出现「死区」。

use crate::ui::layout::WorkbenchLayout;

/// 被拖拽的分隔条属于哪一侧。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerSide {
    /// 边栏与会话流之间：向右拖变宽。
    Left,
    /// 会话流与右侧面板之间：向右拖变窄（面板在右边）。
    Right,
}

impl DividerSide {
    /// 稳定 id 片段（元素 id 用 `divider-{key}`，由数据派生、同屏唯一）。
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// 光标水平位移到宽度增量的符号：右侧分隔条向右拖是缩小面板。
    #[must_use]
    pub const fn direction(self) -> f32 {
        match self {
            Self::Left => 1.0,
            Self::Right => -1.0,
        }
    }

    /// 该侧分隔条能否在当前版式下拖拽：右侧面板折叠（窄条）时不可拖。
    #[must_use]
    pub fn is_draggable(self, layout: &WorkbenchLayout) -> bool {
        match self {
            Self::Left => true,
            Self::Right => !layout.right_panel().is_collapsed(),
        }
    }
}

/// 一次拖拽的起始快照。
#[derive(Debug, Clone, Copy, PartialEq)]
struct DragSession {
    side: DividerSide,
    start_x: f32,
    start_width: f32,
}

/// 分隔条拖拽状态（界面本地态）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DividerDrag {
    session: Option<DragSession>,
}

impl DividerDrag {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否正在拖拽。
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.session.is_some()
    }

    /// 正在拖拽哪一侧；未拖拽时 `None`。
    #[must_use]
    pub fn side(&self) -> Option<DividerSide> {
        self.session.map(|session| session.side)
    }

    /// 按下：记录起始鼠标 x 与该侧当前宽度。折叠态右侧分隔条不可拖拽。
    ///
    /// 返回是否真的进入了拖拽态（已折叠的右侧面板返回 `false`）。
    pub fn begin(&mut self, side: DividerSide, x: f32, layout: &WorkbenchLayout) -> bool {
        if !x.is_finite() || !side.is_draggable(layout) {
            self.session = None;
            return false;
        }
        let start_width = match side {
            DividerSide::Left => layout.sidebar_width(),
            DividerSide::Right => layout.right_panel().width(),
        };
        self.session = Some(DragSession {
            side,
            start_x: x,
            start_width,
        });
        true
    }

    /// 移动：把鼠标位移换算成目标宽度，经描述层夹取后写回。
    ///
    /// 返回本次调动后该侧宽度是否与鼠标位移方向一致（即没有被夹取挡死）。
    /// 未在拖拽 / 非有限坐标时返回 `false` 且不改动宽度。
    pub fn drag_to(&mut self, x: f32, layout: &mut WorkbenchLayout) -> bool {
        let Some(session) = self.session else {
            return false;
        };
        if !x.is_finite() {
            return false;
        }
        let current = width_of(session.side, layout);
        let target =
            session.start_width + (x - session.start_x) * session.side.direction() - current;
        let wanted = target != 0.0;
        if wanted {
            match session.side {
                DividerSide::Left => layout.resize_sidebar_by(target),
                DividerSide::Right => layout.right_panel_mut().resize_by(target),
            }
        }
        width_of(session.side, layout) != current
    }

    /// 松开：结束拖拽，状态清零。此后 [`Self::drag_to`] 不再改动宽度。
    ///
    /// 返回此前是否处于拖拽态。
    pub fn end(&mut self) -> bool {
        self.session.take().is_some()
    }
}

/// 某一侧当前的宽度：右侧折叠时取窄条宽度（fail-closed，绝不返回 0）。
fn width_of(side: DividerSide, layout: &WorkbenchLayout) -> f32 {
    match side {
        DividerSide::Left => layout.sidebar_width(),
        DividerSide::Right => layout.right_panel().effective_width(),
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 GPUI 导入会带进 GPUI 自己的
    // `test` 属性宏并遮蔽标准库的 `#[test]`（速查表 §12.3）。
    use super::{DividerDrag, DividerSide};
    use crate::ui::layout::{
        WorkbenchLayout, RIGHT_PANEL_COLLAPSED_WIDTH, RIGHT_PANEL_MAX_WIDTH, RIGHT_PANEL_MIN_WIDTH,
        SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
    };

    #[test]
    fn sidebar_drag_moves_width_by_pointer_delta_with_sign() {
        let mut layout = WorkbenchLayout::new();
        let start = layout.sidebar_width();
        let mut drag = DividerDrag::new();

        assert!(drag.begin(DividerSide::Left, 100.0, &layout));
        assert!(drag.is_dragging());
        assert_eq!(drag.side(), Some(DividerSide::Left));

        assert!(drag.drag_to(140.0, &mut layout), "向右拖 40px 必须变宽");
        assert_eq!(layout.sidebar_width(), start + 40.0);

        assert!(drag.drag_to(120.0, &mut layout), "往回拖 20px 必须变窄");
        assert_eq!(layout.sidebar_width(), start + 20.0);

        // 重复上报同一位置不改宽度。
        assert!(!drag.drag_to(120.0, &mut layout));
        assert_eq!(layout.sidebar_width(), start + 20.0);
    }

    #[test]
    fn sidebar_drag_is_clamped_by_layout_bounds() {
        let mut layout = WorkbenchLayout::new();
        let mut drag = DividerDrag::new();
        assert!(drag.begin(DividerSide::Left, 0.0, &layout));

        assert!(drag.drag_to(-10_000.0, &mut layout));
        assert_eq!(
            layout.sidebar_width(),
            SIDEBAR_MIN_WIDTH,
            "越界被夹到最小值"
        );
        assert!(
            !drag.drag_to(-20_000.0, &mut layout),
            "已顶到边界，不再变化"
        );
        assert_eq!(layout.sidebar_width(), SIDEBAR_MIN_WIDTH);

        assert!(drag.drag_to(10_000.0, &mut layout));
        assert_eq!(
            layout.sidebar_width(),
            SIDEBAR_MAX_WIDTH,
            "越界被夹到最大值"
        );

        // 顶到上限后往回拖能立刻响应（没有「死区」）。
        let back_off = SIDEBAR_MAX_WIDTH - SIDEBAR_DEFAULT_WIDTH - 50.0;
        assert!(drag.drag_to(back_off, &mut layout));
        assert_eq!(layout.sidebar_width(), SIDEBAR_MAX_WIDTH - 50.0);
    }

    #[test]
    fn right_drag_shrinks_panel_when_moving_right() {
        let mut layout = WorkbenchLayout::new();
        let start = layout.right_panel().width();
        let mut drag = DividerDrag::new();

        assert!(drag.begin(DividerSide::Right, 500.0, &layout));
        assert!(
            drag.drag_to(540.0, &mut layout),
            "右侧分隔条向右拖 = 面板变窄"
        );
        assert_eq!(layout.right_panel().width(), start - 40.0);
        assert!(drag.drag_to(460.0, &mut layout), "向左拖 = 面板变宽");
        assert_eq!(layout.right_panel().width(), start + 40.0);

        assert!(drag.drag_to(0.0, &mut layout));
        assert_eq!(layout.right_panel().width(), RIGHT_PANEL_MAX_WIDTH);
        assert!(drag.drag_to(100_000.0, &mut layout));
        assert_eq!(layout.right_panel().width(), RIGHT_PANEL_MIN_WIDTH);
    }

    #[test]
    fn right_divider_does_not_drag_or_swallow_a_collapsed_panel() {
        let mut layout = WorkbenchLayout::new();
        layout.right_panel_mut().collapse();
        let mut drag = DividerDrag::new();

        assert!(
            !drag.begin(DividerSide::Right, 500.0, &layout),
            "折叠态不可拖"
        );
        assert!(!drag.is_dragging());
        assert!(!drag.drag_to(900.0, &mut layout));
        assert_eq!(layout.right_panel().width(), 320.0, "展开宽度不被改动");
        assert_eq!(
            layout.regions(1200.0).right_panel,
            RIGHT_PANEL_COLLAPSED_WIDTH,
            "fail-closed：折叠面板仍有窄条宽度，不会被吞成 0"
        );
    }

    #[test]
    fn releasing_clears_drag_state_so_later_moves_are_ignored() {
        let mut layout = WorkbenchLayout::new();
        let mut drag = DividerDrag::new();
        assert!(drag.begin(DividerSide::Left, 10.0, &layout));
        assert!(drag.drag_to(30.0, &mut layout));
        let width = layout.sidebar_width();

        assert!(drag.end(), "松开时应处于拖拽态");
        assert!(!drag.is_dragging());
        assert_eq!(drag.side(), None);
        assert!(!drag.end(), "重复松开无副作用");

        assert!(
            !drag.drag_to(500.0, &mut layout),
            "松开后移动鼠标不得改宽度"
        );
        assert_eq!(layout.sidebar_width(), width);
    }

    #[test]
    fn non_finite_pointer_positions_are_ignored() {
        let mut layout = WorkbenchLayout::new();
        let mut drag = DividerDrag::new();
        assert!(!drag.begin(DividerSide::Left, f32::NAN, &layout));
        assert!(!drag.is_dragging());

        assert!(drag.begin(DividerSide::Left, 10.0, &layout));
        let width = layout.sidebar_width();
        assert!(!drag.drag_to(f32::INFINITY, &mut layout));
        assert_eq!(layout.sidebar_width(), width);
    }
}
