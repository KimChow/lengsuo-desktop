//! 智能体管理主区视图（波次 5，卡 `ui-agents-manager`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（上半，无 GPUI 依赖）：条目 / 分组 / 空态 / 诚实占位文案；
//! - **GPUI 层**（[`main_area`]）：把描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自文案全表命名空间 `colleagues`）。
//!
//! 数据源在运行时侧：当前无接线 ⇒ 默认态是「未下发」；等跨线契约就绪后由
//! `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。

// ============================================================ 纯数据层（无 GPUI 依赖）

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "智能体管理";
/// 副标题（`colleagues.title`，逐字）。
pub const SUBTITLE: &str = "助理协作空间";
/// 空态说明（`colleagues.empty`，逐字）。
pub const EMPTY: &str = "暂无可用助理，创建或获取授权后会显示在这里。";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 状态标签：工作中（`colleagues.statusAvailable`，逐字）。
pub const STATUS_AVAILABLE: &str = "工作中";
/// 状态标签：已停用（`colleagues.statusIdle`，逐字）。
pub const STATUS_IDLE: &str = "已停用";
/// 行内动作：暂停助理（`colleagues.action.pauseColleague`，逐字）。
pub const ACTION_PAUSE: &str = "暂停助理";
/// 行内动作：恢复助理（`colleagues.action.resumeColleague`，逐字）。
pub const ACTION_RESUME: &str = "恢复助理";

/// 助理状态：工作中 / 已停用。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentStatus {
    /// 工作中（可派单）。
    Available,
    /// 已停用。
    Idle,
}

impl AgentStatus {
    /// 状态标签文案（逐字取自文案全表）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => STATUS_AVAILABLE,
            Self::Idle => STATUS_IDLE,
        }
    }

    /// 行内动作文案：工作中的助理给「暂停」，已停用的给「恢复」。
    #[must_use]
    pub fn action_label(self) -> &'static str {
        match self {
            Self::Available => ACTION_PAUSE,
            Self::Idle => ACTION_RESUME,
        }
    }

    /// 调试选择器用的 ASCII 键（内部定位用，不是界面文案）。
    #[must_use]
    pub fn debug_key(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Idle => "idle",
        }
    }
}

/// 一条助理清单条目：名称 + 角色标签 + 状态 + 选中态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentEntry {
    /// 助理名称。
    pub name: String,
    /// 角色标签。
    pub role: String,
    /// 状态（工作中 / 已停用）。
    pub status: AgentStatus,
    /// 是否选中。
    pub selected: bool,
}

impl AgentEntry {
    /// 新建条目（默认未选中）。
    #[must_use]
    pub fn new(name: impl Into<String>, role: impl Into<String>, status: AgentStatus) -> Self {
        Self {
            name: name.into(),
            role: role.into(),
            status,
            selected: false,
        }
    }

    /// 构建器：标记为选中。
    #[must_use]
    pub fn selected(mut self) -> Self {
        self.selected = true;
        self
    }
}

/// 视图可枚举的三态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentsState {
    /// 数据未下发（fail-closed，⛔ 不得伪装成空列表）。
    NotDelivered,
    /// 数据已下发且为空。
    Empty,
    /// 数据已下发：清单条目。
    Entries(Vec<AgentEntry>),
}

/// 智能体管理视图的纯数据模型：视图是它的纯函数。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentsManagerView {
    state: AgentsState,
}

impl AgentsManagerView {
    /// 无数据源时的诚实占位（fail-closed）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            state: AgentsState::NotDelivered,
        }
    }

    /// 由已下发的条目构建视图：**空输入落 [`AgentsState::Empty`]**，不是空 `Entries`。
    #[must_use]
    pub fn build(entries: Vec<AgentEntry>) -> Self {
        Self {
            state: if entries.is_empty() {
                AgentsState::Empty
            } else {
                AgentsState::Entries(entries)
            },
        }
    }

    /// 当前态。
    #[must_use]
    pub fn state(&self) -> &AgentsState {
        &self.state
    }

    /// 条目总数（未下发 / 空态均为 0）。
    #[must_use]
    pub fn total(&self) -> usize {
        match &self.state {
            AgentsState::Entries(entries) => entries.len(),
            AgentsState::Empty | AgentsState::NotDelivered => 0,
        }
    }

    /// 按状态分组（固定序：先「工作中」后「已停用」），**只保留非空组**。
    #[must_use]
    pub fn groups(&self) -> Vec<(AgentStatus, Vec<&AgentEntry>)> {
        let entries = match &self.state {
            AgentsState::Entries(entries) => entries,
            AgentsState::Empty | AgentsState::NotDelivered => return Vec::new(),
        };
        [AgentStatus::Available, AgentStatus::Idle]
            .into_iter()
            .map(|status| {
                (
                    status,
                    entries
                        .iter()
                        .filter(|e| e.status == status)
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, group)| !group.is_empty())
            .collect()
    }
}

// ============================================================ GPUI 层

#[cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
#[cfg(feature = "ui")]
use gpui_kit::component::Theme;
#[cfg(feature = "ui")]
use gpui_kit::*;

/// 主区渲染入口（签名固定：返回**具体 `Div`**，`main_area` 各分支类型必须一致）。
///
/// 当前无数据源接线 ⇒ 默认渲染 fail-closed 的「未下发」态；有数据时由
/// [`render`] 走 `Entries` 分支（**换数据源不改视图结构**）。
#[cfg(feature = "ui")]
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &AgentsManagerView::not_delivered())
}

/// 把纯数据模型接到元素树：标题行 + 三态清单区。
#[cfg(feature = "ui")]
fn render(theme: &Theme, view: &AgentsManagerView) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title_row(theme))
        .child(body_area(theme, view))
}

/// 标题行：标题 + 副标题。
#[cfg(feature = "ui")]
fn title_row(theme: &Theme) -> Div {
    div()
        .debug_selector(|| "agents-manager-title".to_owned())
        .px_4()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_base()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SUBTITLE),
        )
}

/// 清单区：三态分发（「未下发」fail-closed 分支不得删）。
#[cfg(feature = "ui")]
fn body_area(theme: &Theme, view: &AgentsManagerView) -> Div {
    match view.state() {
        AgentsState::NotDelivered => centered_note(theme, NOT_DELIVERED),
        AgentsState::Empty => centered_note(theme, EMPTY),
        AgentsState::Entries(_) => div().flex_1().flex().flex_col().children(
            view.groups()
                .into_iter()
                .enumerate()
                .map(|(index, (status, group))| group_section(theme, index, status, &group)),
        ),
    }
}

/// 居中说明（空态 / 未下发共用布局，文案各走各的常量）。
#[cfg(feature = "ui")]
fn centered_note(theme: &Theme, text: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(text))
}

/// 分组段：组头（状态标签 + 计数）+ 条目行。
#[cfg(feature = "ui")]
fn group_section(theme: &Theme, index: usize, status: AgentStatus, group: &[&AgentEntry]) -> Div {
    div()
        .debug_selector(move || format!("agents-manager-group-{index}"))
        .flex()
        .flex_col()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .px_4()
                .py_2()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(theme.muted_foreground)
                        .child(status.label()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(SharedString::from(format!("· {}", group.len()))),
                ),
        )
        .children(
            group
                .iter()
                .enumerate()
                .map(|(row_index, entry)| entry_row(theme, status, row_index, entry)),
        )
}

/// 条目行：名称 + 角色标签 + 状态点/标签 + 行内动作；选中态用主题 accent 标出。
#[cfg(feature = "ui")]
fn entry_row(theme: &Theme, status: AgentStatus, index: usize, entry: &AgentEntry) -> Div {
    let mut name = div().text_sm().text_color(theme.foreground);
    if entry.selected {
        name = name.font_bold();
    }
    div()
        .debug_selector(move || format!("agents-manager-row-{}-{index}", status.debug_key()))
        .min_h(px(48.0)) // 触摸目标 ≥48dp
        .px_4()
        .py_2()
        .flex()
        .items_center()
        .gap_3()
        .border_l_2()
        .border_color(if entry.selected {
            theme.accent
        } else {
            theme.border
        })
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(name.child(SharedString::from(entry.name.clone())))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(SharedString::from(entry.role.clone())),
                ),
        )
        .child(
            div()
                .ml_auto()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().w_2().h_2().rounded_full().bg(
                            if status == AgentStatus::Available {
                                theme.accent
                            } else {
                                theme.muted_foreground
                            },
                        ))
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(status.label()),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(status.action_label()),
                ),
        )
}

/// 测试夹具：三条样例条目（名称 / 角色逐字取自文案全表 `colleagues.cards.*`）。
#[cfg(test)]
fn sample_entries() -> Vec<AgentEntry> {
    vec![
        AgentEntry::new("小研", "需求分析与拆解专家", AgentStatus::Available).selected(),
        AgentEntry::new("数据分析师", "指标口径与漏斗分析", AgentStatus::Available),
        AgentEntry::new("主编", "内容结构与质量把控", AgentStatus::Idle),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        sample_entries, AgentEntry, AgentStatus, AgentsManagerView as View, AgentsState,
        ACTION_PAUSE, ACTION_RESUME, EMPTY, NOT_DELIVERED, STATUS_AVAILABLE, STATUS_IDLE, SUBTITLE,
        TITLE,
    };

    #[test]
    fn groups_split_by_status_in_fixed_order() {
        let view = View::build(sample_entries());
        assert_eq!(view.total(), 3);
        let groups = view.groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].0, AgentStatus::Available);
        assert_eq!(
            groups[0]
                .1
                .iter()
                .map(|e| e.name.as_str())
                .collect::<Vec<_>>(),
            vec!["小研", "数据分析师"]
        );
        assert_eq!(groups[1].0, AgentStatus::Idle);
        assert_eq!(groups[1].1.len(), 1);
    }

    #[test]
    fn groups_skip_empty_and_keep_selected_flag() {
        let view = View::build(vec![AgentEntry::new(
            "主编",
            "内容结构与质量把控",
            AgentStatus::Idle,
        )
        .selected()]);
        let groups = view.groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, AgentStatus::Idle);
        assert!(groups[0].1[0].selected);
    }

    #[test]
    fn empty_input_yields_empty_state_not_empty_list() {
        assert_eq!(View::build(Vec::new()).state(), &AgentsState::Empty);
    }

    #[test]
    fn not_delivered_is_fail_closed() {
        let view = View::not_delivered();
        assert_eq!(view.state(), &AgentsState::NotDelivered);
        assert_eq!(view.total(), 0);
        assert!(view.groups().is_empty());
    }

    #[test]
    fn copy_is_verbatim_from_namespace() {
        assert_eq!(TITLE, "智能体管理");
        assert_eq!(SUBTITLE, "助理协作空间");
        assert_eq!(EMPTY, "暂无可用助理，创建或获取授权后会显示在这里。");
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(STATUS_AVAILABLE, "工作中");
        assert_eq!(STATUS_IDLE, "已停用");
        assert_eq!(ACTION_PAUSE, "暂停助理");
        assert_eq!(ACTION_RESUME, "恢复助理");
    }

    #[test]
    fn status_labels_and_actions_pair_up() {
        assert_eq!(AgentStatus::Available.label(), STATUS_AVAILABLE);
        assert_eq!(AgentStatus::Idle.label(), STATUS_IDLE);
        assert_eq!(AgentStatus::Available.action_label(), ACTION_PAUSE);
        assert_eq!(AgentStatus::Idle.action_label(), ACTION_RESUME);
    }
}

/// 无窗口渲染断言：真跑一遍 layout + prepaint + paint（同 `render_assert_tests` 范式）。
///
/// ⛔ 显式导入、不 `use super::*`：避免 GPUI 的 `test` 属性宏遮蔽标准库 `#[test]`。
#[cfg(all(test, feature = "ui"))]
mod render_tests {
    use super::{main_area, render, sample_entries, AgentsManagerView as View};
    use gpui_kit::component::Theme;
    use gpui_kit::{point, px, size, IntoElement as _, TestAppContext};

    /// 无窗口画布大小（够放下三行清单）。
    const CANVAS_W: f32 = 800.0;
    const CANVAS_H: f32 = 600.0;

    /// 画一帧并断言：`located` 里的选择器都进渲染帧且布局非零；`absent` 里的不出现。
    fn assert_located(
        view: &View,
        located: &'static [&'static str],
        absent: &'static [&'static str],
    ) {
        let mut ctx = TestAppContext::single();
        // 与接线层一致：预置全局 Theme，避免绘制路径读全局主题时 panic。
        ctx.update(|cx| cx.set_global(Theme::default()));

        let element = render(&Theme::default(), view).into_any_element();
        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(CANVAS_W), px(CANVAS_H)),
            move |_window, _cx| element,
        );

        for sel in located {
            let bounds = visual
                .debug_bounds(sel)
                .unwrap_or_else(|| panic!("选择器 {sel} 没进入渲染帧"));
            assert!(
                bounds.size.width > px(0.0) && bounds.size.height > px(0.0),
                "选择器 {sel} 布局为零，元素没真正参与排版"
            );
        }
        for sel in absent {
            assert!(
                visual.debug_bounds(sel).is_none(),
                "选择器 {sel} 不应出现在该态的渲染帧里"
            );
        }
    }

    #[test]
    fn main_area_builds_without_panic() {
        let _ = main_area(&Theme::default());
    }

    /// 验收②：元素树能定位到 标题节点 + ≥3 个清单/分组节点（2 组 + 3 行 = 5 个）。
    #[test]
    fn entries_tree_locates_title_groups_and_rows() {
        let view = View::build(sample_entries());
        assert_located(
            &view,
            &[
                "agents-manager-title",
                "agents-manager-group-0",
                "agents-manager-group-1",
                "agents-manager-row-available-0",
                "agents-manager-row-available-1",
                "agents-manager-row-idle-0",
            ],
            &[],
        );
    }

    #[test]
    fn empty_state_tree_has_title_but_no_list_nodes() {
        let view = View::build(Vec::new());
        assert_located(
            &view,
            &["agents-manager-title"],
            &["agents-manager-group-0", "agents-manager-row-available-0"],
        );
    }

    #[test]
    fn not_delivered_tree_is_fail_closed_no_list_nodes() {
        let view = View::not_delivered();
        assert_located(
            &view,
            &["agents-manager-title"],
            &["agents-manager-group-0", "agents-manager-row-idle-0"],
        );
    }
}
