//! 左栏主入口图标行（波次 9，卡 `ui-rail-icons`，差异 D1）。
//!
//! 参照形态（桌面 2 实拍）：6 项主入口各一枚 16~18px 单色图标
//! （新建任务=加号 / 助理=机器人 / 专家·技能·连接器=组合件 /
//! 定时任务=时钟 / 资料库=书 / 更多=省略号），水平 icon+text 行，
//! 选中态高亮条。
//!
//! 分层约定（同 [`crate::ui::sidebar_gpui`]）：
//! - **数据层**（不门控）：意图枚举 + 逐字文案 + fail-closed 条目模型，
//!   纯数据、随 `cargo test` 跑。
//! - **渲染层**（文件内 `feature = "ui"` 门控）：图标几何 + 行 / 列，
//!   颜色一律 `theme.*`，布局用 gpui flex。
//!
//! 硬约束：
//! - **文案逐字**：六个入口文案全部取自文案全表 `conversation.*` 命名空间，
//!   ⛔ 本文件不自造中文。
//! - **fail-closed**：上级未下发文案 ⇒ 显示「未下发」，不伪造。
//! - **点击只上报意图**：`on_click` 只调 `on_intent(RailIntent)`，
//!   ⛔ 不实现任何入口行为（开面板 / 建任务等都归后续卡）。
//! - 图标不用字体字形 / 位图资产，全部用 div 几何画（单色 = 主题前景色），
//!   避免字体缺字渲染成豆腐块。

/// 未下发（fail-closed）。
pub const NOT_DELIVERED: &str = "未下发";

/// 主入口意图：点击只上报这个枚举，不触发任何行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailIntent {
    /// 新建任务（图标：加号）。
    NewTask,
    /// 助理（图标：机器人）。
    Assistant,
    /// 专家·技能·连接器（图标：组合件）。
    Experts,
    /// 定时任务（图标：时钟）。
    Automation,
    /// 资料库（图标：书）。
    Library,
    /// 更多（图标：省略号）。
    More,
}

impl RailIntent {
    /// 六个主入口（顺序 = 参照形态从上到下）。
    pub const ALL: [RailIntent; 6] = [
        RailIntent::NewTask,
        RailIntent::Assistant,
        RailIntent::Experts,
        RailIntent::Automation,
        RailIntent::Library,
        RailIntent::More,
    ];

    /// 入口文案（逐字取自文案全表 `conversation.*` 命名空间）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            // `conversation.newTask`
            RailIntent::NewTask => "新建任务",
            // `conversation.colleagues`
            RailIntent::Assistant => "助理",
            // `conversation.experts`
            RailIntent::Experts => "专家·技能·连接器",
            // `conversation.automation`
            RailIntent::Automation => "定时任务",
            // `conversation.myFilesLibrary`
            RailIntent::Library => "资料库",
            // `conversation.more`（摘录 `home.practiceCases.more` 同为「更多」）
            RailIntent::More => "更多",
        }
    }

    /// 稳定 id（GPUI 元素 id / 测试定位）。
    #[must_use]
    pub fn as_id(self) -> &'static str {
        match self {
            RailIntent::NewTask => "rail-new-task",
            RailIntent::Assistant => "rail-assistant",
            RailIntent::Experts => "rail-experts",
            RailIntent::Automation => "rail-automation",
            RailIntent::Library => "rail-library",
            RailIntent::More => "rail-more",
        }
    }
}

/// 一行主入口的数据（纯数据，可在无窗口环境测）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailItem {
    /// 入口意图（决定图标与文案）。
    pub intent: RailIntent,
    /// 是否选中（选中态画高亮条）。
    pub selected: bool,
    /// 上级下发的文案；`None` ⇒ 未下发（fail-closed，不伪造）。
    pub label: Option<&'static str>,
}

impl RailItem {
    /// 标准入口：文案取逐字常量，未选中。
    #[must_use]
    pub fn new(intent: RailIntent) -> Self {
        Self {
            intent,
            selected: false,
            label: Some(intent.label()),
        }
    }

    /// 选中态构建器。
    #[must_use]
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// 上级未下发文案的条目（显示「未下发」，不伪造）。
    #[must_use]
    pub fn undelivered(intent: RailIntent) -> Self {
        Self {
            intent,
            selected: false,
            label: None,
        }
    }

    /// 实际显示的文案（缺数据 ⇒ 「未下发」）。
    #[must_use]
    pub fn display_label(&self) -> &'static str {
        self.label.unwrap_or(NOT_DELIVERED)
    }
}

// === 渲染层：GPUI 元素树（feature = "ui"） ===

#[cfg(feature = "ui")]
use gpui_kit::component::Theme;
#[cfg(feature = "ui")]
use gpui_kit::*;

/// 图标边长（参照形态：16~18px 单色，取下限 16）。
#[cfg(feature = "ui")]
const ICON_SIZE: f32 = 16.;
/// 行高：触摸目标 ≥48dp（设计方法论固定段）。
#[cfg(feature = "ui")]
const ROW_HEIGHT: f32 = 48.;

/// 绝对定位的几何单元（图标的每一笔）。
#[cfg(feature = "ui")]
fn cell(left: f32, top: f32, w: f32, h: f32) -> Div {
    div()
        .absolute()
        .left(px(left))
        .top(px(top))
        .w(px(w))
        .h(px(h))
}

/// 一枚 16px 单色图标：全部用 div 几何画，墨色 = 主题前景色。
#[cfg(feature = "ui")]
fn rail_icon(intent: RailIntent, theme: &Theme) -> Div {
    let ink = theme.sidebar_foreground;
    let icon = div().relative().w(px(ICON_SIZE)).h(px(ICON_SIZE));
    match intent {
        // 加号：横竖两根 2px 短棒。
        RailIntent::NewTask => icon
            .child(cell(0., 7., 16., 2.).bg(ink))
            .child(cell(7., 0., 2., 16.).bg(ink)),
        // 机器人：圆角方头 + 双眼 + 天线。
        RailIntent::Assistant => icon
            .child(
                cell(2., 4., 12., 10.)
                    .border_2()
                    .border_color(ink)
                    .rounded(px(3.)),
            )
            .child(cell(5., 8., 2., 2.).bg(ink))
            .child(cell(9., 8., 2., 2.).bg(ink))
            .child(cell(7., 1., 2., 2.).bg(ink).rounded_full()),
        // 组合件：两只错位描边小方（拼合感）。
        RailIntent::Experts => icon
            .child(cell(4., 2., 10., 10.).border_2().border_color(ink))
            .child(cell(2., 4., 10., 10.).border_2().border_color(ink)),
        // 时钟：描边圆环 + 12 点 / 3 点两根指针。
        RailIntent::Automation => icon
            .child(
                cell(1., 1., 14., 14.)
                    .border_2()
                    .border_color(ink)
                    .rounded_full(),
            )
            .child(cell(7., 3., 2., 6.).bg(ink))
            .child(cell(8., 7., 5., 2.).bg(ink)),
        // 书：描边封面 + 一根书脊。
        RailIntent::Library => icon
            .child(
                cell(2., 2., 12., 12.)
                    .border_2()
                    .border_color(ink)
                    .rounded(px(2.)),
            )
            .child(cell(5., 2., 2., 12.).bg(ink)),
        // 省略号：三颗 2px 圆点。
        RailIntent::More => icon
            .child(cell(2., 7., 2., 2.).bg(ink).rounded_full())
            .child(cell(7., 7., 2., 2.).bg(ink).rounded_full())
            .child(cell(12., 7., 2., 2.).bg(ink).rounded_full()),
    }
}

/// 一行主入口：水平 icon+text，选中态画左缘高亮条。
///
/// 点击只上报 `on_intent(意图)`，⛔ 不实现行为。
#[cfg(feature = "ui")]
pub fn rail_row<F>(theme: &Theme, item: &RailItem, on_intent: F) -> impl IntoElement
where
    F: Fn(RailIntent) + 'static,
{
    let mut row = div()
        .id(item.intent.as_id())
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(ROW_HEIGHT))
        .px_2()
        .text_color(theme.sidebar_foreground)
        .on_click(move |_| on_intent(item.intent));
    if item.selected {
        row = row
            .bg(theme.sidebar_border)
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .w(px(3.))
                    .bg(theme.sidebar_foreground),
            );
    }
    row.child(rail_icon(item.intent, theme))
        .child(div().text_sm().child(item.display_label()))
}

/// 主入口列：六行垂直排布（纯构建）。
#[cfg(feature = "ui")]
pub fn rail<F>(theme: &Theme, items: &[RailItem], on_intent: F) -> Div
where
    F: Fn(RailIntent) + Clone + 'static,
{
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_1()
        .children(items.iter().map(|item| rail_row(theme, item, on_intent.clone())))
}
