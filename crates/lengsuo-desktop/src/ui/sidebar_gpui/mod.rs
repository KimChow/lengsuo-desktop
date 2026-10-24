//! 边栏的 GPUI 接线层：把 [`crate::ui::sidebar`] 的渲染描述接到元素树。
//!
//! 分层约定（同 [`crate::ui::env_gpui`]）：
//! - **描述层**（`sidebar`）：纯数据、无 GPUI 依赖，可在无窗口环境测。
//! - **接线层**（本模块）：把描述变成 GPUI 元素，**不重新实现**任何业务判断。
//!
//! 本模块内部再分两半：
//! - **映射层**（无 `feature = "ui"` 门控）：描述字段 → 界面标记 / 交互契约，
//!   纯数据，随 `cargo test` 一起跑。
//! - **渲染层**（`feature = "ui"` 门控）：GPUI 元素树，只能编译验证。
//!
//! 硬约束：
//! - **文案只有一个来源**：分组名、标题、空态、归档按钮文案、频道 / 智能体入口
//!   文案全部取自描述层（[`crate::ui::sidebar`]），本文件不另造一套。
//! - **归档默认隐藏**：哪些分组可见由描述层决定，本文件不自行过滤条目。
//! - 颜色一律取自 `cx.theme()`，不硬编码。
//!
//! **接线层不实现业务语义**：置顶 / 归档 / 未读 / 选中 / 空态的判定都在
//! 描述层，本文件只做「描述字段 → 样式与标记」的映射。
//!
//! 目录式子模块（Rust 2018 起 `ui/mod.rs` 里的 `pub mod sidebar_gpui;` 自动指向本目录）：
//! - [`render`]：会话部分的渲染分支 + 共享小件（徽标 / 演示来源横幅）。
//! - [`nav`]：频道入口 + 智能体管理入口的渲染（A15）。
//! - [`channels`] / [`agents`]：两个入口点进去的视图体（A15 界面部分 / C9 只做视图）。

use crate::ui::sidebar::{SessionRow, SidebarState};

// 渲染层按选中入口分流 body 时需要 `NavKind`（只在 feature = "ui" 下使用）。
#[cfg(feature = "ui")]
use crate::ui::sidebar::nav::NavKind;

// 频道 / 智能体视图体：纯函数层不门控（随 `cargo test` 跑），渲染层在文件内门控。
pub mod agents;
pub mod channels;
#[cfg(feature = "ui")]
mod collapse;
#[cfg(feature = "ui")]
mod nav;
#[cfg(feature = "ui")]
mod render;

#[cfg(test)]
mod tests;

// GPUI 相关导入：只有渲染层需要。
// 三个 import 缺一不可：`StyledExt`（`font_bold` / `truncate` 等扩展）、
// `ActiveTheme`（`cx.theme()`）、`gpui_kit::*`（`div` / `px` / `Entity` / `Render`）。
#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::ActiveTheme as _;
#[cfg(feature = "ui")]
use gpui_kit::*;

/// 置顶标记文案。
pub const PINNED_MARK: &str = "置顶";
/// 归档标记文案。
pub const ARCHIVED_MARK: &str = "归档";
/// 未读标记文案。
pub const UNREAD_MARK: &str = "未读";

/// 演示来源横幅文案：频道 / 智能体视图体顶部的固定标注（C9 未就绪 ⇒ 如实标注）。
pub const DEMO_SOURCE_BANNER: &str = "演示数据（运行时契约未就绪）";

/// 行标记的种类（描述层布尔字段 → 界面标记）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    /// 置顶会话。
    Pinned,
    /// 已归档会话。
    Archived,
    /// 有未读消息。
    Unread,
}

/// 行标记的展示信息（纯数据，可在无窗口环境测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowMark {
    /// 标记种类（决定颜色）。
    pub kind: MarkKind,
    /// 标记文案（中文）。
    pub label: &'static str,
}

/// 描述层的一行 → 界面上的标记集合。
///
/// 这是「描述 → 元素」映射里唯一带判断的一步：描述层只给布尔字段，
/// 标记的**种类与文案**在这里定型，方便无窗口环境测。
#[must_use]
pub fn row_marks(row: &SessionRow) -> Vec<RowMark> {
    let mut marks = Vec::new();
    if row.pinned {
        marks.push(RowMark {
            kind: MarkKind::Pinned,
            label: PINNED_MARK,
        });
    }
    if row.archived {
        marks.push(RowMark {
            kind: MarkKind::Archived,
            label: ARCHIVED_MARK,
        });
    }
    if row.unread {
        marks.push(RowMark {
            kind: MarkKind::Unread,
            label: UNREAD_MARK,
        });
    }
    marks
}

/// 点击一行：写入选中态。
///
/// 接线层的点击处理只调这个入口，不自己判断「这条能不能选」——
/// 能不能选由描述层的 [`SidebarState::select`] 说了算（归档隐藏时不可选）。
pub fn select_row(state: &mut SidebarState, id: &str) -> bool {
    state.select(id)
}

/// 点击「显示 / 隐藏归档」：翻转归档可见性。
pub fn toggle_archived(state: &mut SidebarState) {
    state.toggle_archived();
}

/// 点击一个导航入口（频道 / 智能体管理）：写入选中态。
///
/// 只改**选中态**，⛔ 不触发任何运行时行为（不建频道、不建智能体）。
pub fn select_nav(state: &mut SidebarState, kind: crate::ui::sidebar::NavKind) -> bool {
    state.select_nav(kind)
}

/// 演示用状态：手工构造若干会话条目，**不接真实数据源**。
#[must_use]
pub fn demo_state() -> SidebarState {
    let mut state = SidebarState::new();
    state.set_entries(vec![
        crate::ui::sidebar::SessionEntry::new("demo-pinned", "接口联调")
            .with_pinned(true)
            .with_unread(true),
        crate::ui::sidebar::SessionEntry::new("demo-plain", "打包脚本排查"),
        crate::ui::sidebar::SessionEntry::new("demo-archived", "旧版设计稿").with_archived(true),
    ]);
    // 演示计数：频道 / 智能体入口的角标（A15 的界面部分）。
    // ⛔ 来自 demo 数据，不来自服务端（A1 数据源未接）。
    state.set_channel_count(2);
    state.set_agent_count(3);
    state
}

// === 渲染层：GPUI 元素树（feature = "ui"） ===

/// 边栏的 GPUI 视图。
///
/// 持有**会话列表状态**（[`SidebarState`]）的副本：点击行 / 归档切换按钮
/// 只改这份副本，宿主可用 [`Self::state_mut`] 回写共享状态。
#[cfg(feature = "ui")]
pub struct SidebarViewGpui {
    state: SidebarState,
    /// 侧栏实际宽度（px）。
    ///
    /// **必须由宿主下发**：2026-10-21 Windows 真机实测发现，此前渲染时写死
    /// `SIDEBAR_DEFAULT_WIDTH`（260），导致拖动左分隔条后侧栏内容仍按 260 画 ——
    /// 表现为「左边拖不动 / 不联动」，而右栏（用自身 `layout`）正常。
    width: f32,
    /// 边栏是否折叠（窄条态）。
    ///
    /// 与右栏同款：这是子视图的**私有副本**，宿主在 `observe` 里把它**回收**给
    /// 权威版式，否则下一次下发会把折叠态覆盖回展开（点收起会「弹回来」）。
    collapsed: bool,
}

#[cfg(feature = "ui")]
impl SidebarViewGpui {
    /// 用一份会话列表状态构造视图（宽度取默认值）。
    #[must_use]
    pub fn new(state: SidebarState) -> Self {
        Self {
            state,
            width: crate::ui::layout::SIDEBAR_DEFAULT_WIDTH,
            collapsed: false,
        }
    }

    /// 从只读引用构造视图（内部克隆一份状态）。
    #[must_use]
    pub fn from_state(state: &SidebarState) -> Self {
        Self::new(state.clone())
    }

    /// 宿主下发侧栏实际宽度（拖动分隔条后由根视图调用）。
    ///
    /// 非有限值（NaN / ∞）被忽略，保持原宽度（fail-closed）。
    pub fn set_width(&mut self, width: f32) {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
    }

    /// 当前侧栏宽度。
    #[must_use]
    pub fn width(&self) -> f32 {
        self.width
    }

    /// 宿主下发边栏折叠态（启动水合 / 权威版式变化后由根视图调用）。
    pub fn set_collapsed(&mut self, collapsed: bool) {
        self.collapsed = collapsed;
    }

    /// 折叠 / 展开边栏。
    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
    }

    /// 当前是否折叠。
    #[must_use]
    pub fn is_collapsed(&self) -> bool {
        self.collapsed
    }

    /// 演示用视图：手工构造的会话条目，**不接真实数据源**。
    #[must_use]
    pub fn demo() -> Self {
        Self::new(demo_state())
    }

    /// 当前会话列表状态（只读）。
    #[must_use]
    pub fn state(&self) -> &SidebarState {
        &self.state
    }

    /// 当前会话列表状态（可写）：宿主据此回写共享状态。
    pub fn state_mut(&mut self) -> &mut SidebarState {
        &mut self.state
    }
}

#[cfg(feature = "ui")]
impl Render for SidebarViewGpui {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = cx.entity();
        // 描述层产出渲染描述；本文件只做「描述 → 元素」的映射。
        // 宽度用**宿主下发的实际宽度**（`self.width`），不再写死默认值 ——
        // 写死会让拖动分隔条后侧栏内容仍按默认宽渲染（2026-10-21 真机实测）。
        let view = crate::ui::sidebar::render(&self.state, self.width);

        // 折叠态：只渲染窄条与展开入口，**不渲染会话列表**。
        // 外框与展开态共用（宽度由宿主下发，折叠时是窄条宽度）。
        if self.collapsed {
            return div()
                .flex()
                .flex_col()
                .h_full()
                .w(px(self.width))
                .bg(theme.sidebar)
                .text_color(theme.sidebar_foreground)
                .border_r_1()
                .border_color(theme.sidebar_border)
                .child(collapse::render_collapsed_strip(&entity, theme));
        }

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .py_2()
            .border_b_1()
            .border_color(theme.sidebar_border)
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.sidebar_foreground)
                    .child(view.heading),
            );
        let header = if view.unread_count > 0 {
            header.child(render::render_count_badge(view.unread_count, theme))
        } else {
            header
        };
        // 展开态的折叠入口放在标题行末尾。
        let header = header.child(collapse::render_toggle(false, &entity, theme));

        // 顶部两个导航入口：频道 + 智能体管理（A15）。
        let nav = nav::render_nav(&view.nav, &entity, theme);

        // A15：选中频道 / 智能体入口时，body 换成对应视图体（演示数据 + 如实标注）；
        // 其他入口 / 未选中时保持会话列表（本卡只补这两个视图体）。
        let selected_nav = view
            .nav
            .iter()
            .find(|entry| entry.selected)
            .map(|entry| (entry.kind, entry.title));
        let body = match selected_nav {
            Some((NavKind::Channels, title)) => channels::render(
                title,
                &channels::demo_messages(),
                channels::provenance(),
                theme,
            ),
            Some((NavKind::Agents, title)) => {
                agents::render(title, &agents::demo_agents(), agents::provenance(), theme)
            }
            _ => {
                if let Some(text) = view.empty_text {
                    render::render_empty(text, theme).into_any_element()
                } else {
                    div()
                        .id("sidebar-body")
                        .flex()
                        .flex_col()
                        .flex_1()
                        .overflow_y_scroll()
                        .children(
                            view.sections
                                .iter()
                                .map(|section| render::render_section(section, &entity, theme)),
                        )
                        .into_any_element()
                }
            }
        };

        div()
            .flex()
            .flex_col()
            .h_full()
            .w(px(view.width))
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .child(header)
            .child(nav)
            .child(body)
            .child(render::render_archived_toggle(&view, &entity, theme))
    }
}
pub mod bottom_cards;
