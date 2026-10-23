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
                    entries.iter().filter(|e| e.status == status).collect::<Vec<_>>(),
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

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
#[cfg(feature = "ui")]
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
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
        ACTION_PAUSE, ACTION_RESUME, EMPTY, NOT_DELIVERED, STATUS_AVAILABLE, STATUS_IDLE,
        SUBTITLE, TITLE,
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
        let view = View::build(vec![
            AgentEntry::new("主编", "内容结构与质量把控", AgentStatus::Idle).selected(),
        ]);
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
