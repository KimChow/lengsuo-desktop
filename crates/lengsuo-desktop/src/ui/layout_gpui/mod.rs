//! 三区容器的 GPUI 接线：可拖拽左右分隔条 + 右侧面板 tab 点击。
//!
//! 分层约定（同 [`crate::ui::env_gpui`]）：
//! - **描述层**（[`crate::ui::layout`]）：`WorkbenchLayout` 只保存宽度 / 折叠 /
//!   tab，并负责宽度夹取，已有测试，本模块**不改它、不重复算夹取**。
//! - **接线层**（本目录）：把鼠标事件映射成对描述层的方法调用。
//!
//! 子模块：
//! - [`drag`]：分隔条拖拽状态机（纯逻辑，无 GPUI 依赖）。
//! - [`tab_strip`]：右侧面板 tab 条派生与点击映射（纯逻辑）。
//!
//! 渲染层（`feature = "ui"`，只能编译验证）：
//! - [`DividerGpui`]：一条分隔条。按下开始拖、移动改宽度、松开结束，
//!   每次改动都 `cx.notify()` 重绘。
//! - [`RightPanelTabGpui`]：右侧面板 tab 条。点 tab → `select_tab` → 重新
//!   派生 tab 条状态 → `notify()`。
//!
//! 接入方式：本模块只暴露构造函数，**由 `gpui_app.rs` 在后续卡里调用**；
//! 本卡不改 `gpui_app.rs`。宿主把拖拽结果经 [`DividerGpui::layout`] 读回，
//! 用其宽度渲染三区。
//!
//! 硬约束：
//! - 宽度 / 折叠 / 选中 tab 都是界面本地态，不发请求、不读凭据、不落盘。
//! - 不判定权限：tab 本来就在界面上，可点 ≠ 有权看；可见性由上游状态层决定。
//! - 颜色一律 `cx.theme()`，不硬编码。
//! - fail-closed：拿不到宽度时用 [`WorkbenchLayout::new`] 的默认值（构造即
//!   默认），不渲染 0 宽。

pub mod drag;
pub mod tab_strip;

pub use drag::{DividerDrag, DividerSide};
pub use tab_strip::{click_tab_id, tab_entries, TabEntry};

#[cfg(feature = "ui")]
use gpui_kit::component::ActiveTheme as _;
#[cfg(feature = "ui")]
use gpui_kit::*;

/// 右侧面板 tab 条持有的版式状态（渲染层在 `feature = "ui"` 下使用）。
#[cfg(feature = "ui")]
use crate::ui::layout::RightPanelLayout;
#[cfg(feature = "ui")]
use crate::ui::layout::WorkbenchLayout;

/// 分隔条宽度（逻辑像素），也是拖拽的命中区域。
#[cfg(feature = "ui")]
pub const DIVIDER_WIDTH: f32 = 4.0;

/// 一条可拖拽分隔条。
///
/// 拖拽状态只放在本组件里（[`DividerDrag`]）；宽度结果写进自己持有的
/// [`WorkbenchLayout`] 副本，宿主用 [`Self::layout`] 读回。
///
/// 事件路线（与 gpui-base 的 `ResizeHandle` 同一路线）：
/// - 按下：元素上的 `on_mouse_down` 开始拖拽；
/// - 移动：`Render::paint` 里注册的窗口级 `MouseMoveEvent` 监听，鼠标移出
///   分隔条后仍继续拖动；
/// - 松开：`Render::paint` 里注册的窗口级 `MouseUpEvent` 监听，结束拖拽。
#[cfg(feature = "ui")]
pub struct DividerGpui {
    side: DividerSide,
    layout: WorkbenchLayout,
    drag: DividerDrag,
    hovering: bool,
}

#[cfg(feature = "ui")]
impl DividerGpui {
    /// 用默认版式状态构造分隔条。
    #[must_use]
    pub fn new(side: DividerSide) -> Self {
        Self::with_layout(side, WorkbenchLayout::new())
    }

    /// 用一份界面本地版式状态构造分隔条（内部克隆）。
    #[must_use]
    pub fn with_layout(side: DividerSide, layout: WorkbenchLayout) -> Self {
        Self {
            side,
            layout,
            drag: DividerDrag::new(),
            hovering: false,
        }
    }

    /// 当前版式状态（只读）：宿主据此渲染三区宽度。
    #[must_use]
    pub fn layout(&self) -> &WorkbenchLayout {
        &self.layout
    }

    /// 当前版式状态（可写）：宿主回写折叠 / tab 等版式变化。
    pub fn layout_mut(&mut self) -> &mut WorkbenchLayout {
        &mut self.layout
    }

    /// 被拖拽的一侧。
    #[must_use]
    pub fn side(&self) -> DividerSide {
        self.side
    }

    /// 是否正在拖拽。
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.drag.is_dragging()
    }

    /// 按下与移动的共用处理：未在拖拽时先 `begin`，随后 `drag_to`。
    ///
    /// 返回宽度是否发生变化（调用方据此决定是否 `notify()`）。
    fn pointer_at(&mut self, x: f32) -> bool {
        if !self.drag.is_dragging() {
            self.drag.begin(self.side, x, &self.layout);
        }
        self.drag.drag_to(x, &mut self.layout)
    }

    /// 分隔条颜色：拖拽态 > 悬停态 > 常态，全部取自主题。
    fn color(&self, theme: &gpui_kit::component::Theme) -> gpui_kit::Hsla {
        if self.is_dragging() {
            theme.accent
        } else if self.hovering {
            theme.border
        } else {
            theme.muted_foreground
        }
    }
}

/// 分隔条的 paint 钩子：每次 paint 注册窗口级移动 / 松开监听（与
/// gpui-base `ResizeHandle` 同路线；手写 `Element` 而非闭包，避免在
/// `on_mouse_event` 的 `&mut App` 里再借 `Context`）。
#[cfg(feature = "ui")]
struct SeparatorListeners {
    entity: Entity<DividerGpui>,
}

#[cfg(feature = "ui")]
impl IntoElement for SeparatorListeners {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[cfg(feature = "ui")]
impl Element for SeparatorListeners {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (
            window.request_layout(
                Style {
                    size: size(
                        Length::Definite(px(0.).into()),
                        Length::Definite(px(0.).into()),
                    ),
                    ..Default::default()
                },
                None,
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        __cx: &mut App,
    ) -> Self::PrepaintState {
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        window.on_mouse_event({
            let entity = self.entity.clone();
            move |event: &MouseMoveEvent, phase, _window, cx| {
                if !phase.bubble() {
                    return;
                }
                let x: f32 = event.position.x.into();
                entity.update(cx, |view, cx| {
                    if view.pointer_at(x) {
                        cx.notify();
                    }
                });
            }
        });
        window.on_mouse_event({
            let entity = self.entity.clone();
            move |_: &MouseUpEvent, phase, _window, cx| {
                if !phase.bubble() {
                    return;
                }
                entity.update(cx, |view, cx| {
                    if view.drag.end() {
                        cx.notify();
                    }
                });
            }
        });
    }
}

/// 分隔条渲染：颜色 / 悬停 / 按下全部走元素回调；拖拽中的移动与松开走
/// [`SeparatorListeners`] 注册的窗口级监听。
#[cfg(feature = "ui")]
impl Render for DividerGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let color = self.color(&theme);

        let down = cx.listener(|view: &mut Self, event: &MouseDownEvent, _window, cx| {
            let x: f32 = event.position.x.into();
            if view.pointer_at(x) || view.drag.is_dragging() {
                cx.notify();
            }
        });
        let hover = cx.listener(|view: &mut Self, hovering: &bool, _window, cx| {
            if view.hovering != *hovering {
                view.hovering = *hovering;
                cx.notify();
            }
        });

        div()
            .id(format!("divider-{}", self.side.key()))
            .flex_none()
            .h_full()
            .w(px(DIVIDER_WIDTH))
            .bg(color)
            .cursor_col_resize()
            .on_hover(hover)
            .on_mouse_down(MouseButton::Left, down)
            // 窗口级监听只能在 paint 阶段注册（`on_mouse_event` 内部
            // `debug_assert_paint`）。零尺寸自定义元素承载 paint 钩子，
            // 与 gpui-base `ResizeHandle` 的做法一致。
            .child(
                SeparatorListeners {
                    entity: cx.entity(),
                }
                .into_any_element(),
            )
    }
}

/// 右侧面板 tab 条。
///
/// 持有 [`RightPanelLayout`] 副本：点 tab 只改这份副本的选中态与折叠态。
#[cfg(feature = "ui")]
pub struct RightPanelTabGpui {
    layout: RightPanelLayout,
}

#[cfg(feature = "ui")]
impl RightPanelTabGpui {
    /// 用版式状态构造 tab 条。
    #[must_use]
    pub fn new(layout: RightPanelLayout) -> Self {
        Self { layout }
    }

    /// 从只读引用构造（内部克隆一份）。
    #[must_use]
    pub fn from_layout(layout: &RightPanelLayout) -> Self {
        Self::new(layout.clone())
    }

    /// 当前版式状态（只读）。
    #[must_use]
    pub fn layout(&self) -> &RightPanelLayout {
        &self.layout
    }

    /// 当前版式状态（可写）：宿主回写。
    pub fn layout_mut(&mut self) -> &mut RightPanelLayout {
        &mut self.layout
    }

    /// 重新派生 tab 条（渲染与点击后共用同一份派生逻辑）。
    #[must_use]
    pub fn entries(&self) -> Vec<TabEntry> {
        tab_entries(&self.layout)
    }

    /// 点击 tab：改描述层状态，返回是否处理了这个 id（未知 id 为 `false`）。
    fn click(&mut self, id: &str) -> bool {
        click_tab_id(&mut self.layout, id)
    }
}

/// 一个 tab 入口元素：先 `.id()` 变 `Stateful` 再挂 `on_click`。
#[cfg(feature = "ui")]
fn render_tab(
    entry: &TabEntry,
    collapsed: bool,
    entity: &Entity<RightPanelTabGpui>,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let label = if collapsed {
        entry.compact.clone()
    } else {
        entry.label.to_owned()
    };
    let text_color = if entry.selected {
        theme.accent
    } else {
        theme.muted_foreground
    };

    let entity = entity.clone();
    let id = entry.tab.id().to_owned();
    div()
        .id(format!("layout-tab-{}", entry.tab.id()))
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .py_1()
        .rounded_sm()
        .cursor_pointer()
        .text_sm()
        .text_color(text_color)
        .child(label)
        .on_click(move |_event, _window, app| {
            entity.update(app, |view, cx| {
                if view.click(&id) {
                    cx.notify();
                }
            });
        })
        .into_any_element()
}

#[cfg(feature = "ui")]
impl Render for RightPanelTabGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let collapsed = self.layout.is_collapsed();
        let entity = cx.entity();
        let tabs = self
            .entries()
            .iter()
            .map(|entry| render_tab(entry, collapsed, &entity, &theme))
            .collect::<Vec<_>>();

        div().flex().gap_1().p_1().bg(theme.sidebar).children(tabs)
    }
}
