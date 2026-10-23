//! 助理协作空间主区视图（波次 5，卡 `ui-colleagues-panel`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：条目 / 置顶分组 / 三态（未下发 / 空态 / 清单）；
//! - **GPUI 层**（[`main_area`]）：把视图模型接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自界面文案全表命名空间 `colleagues`）。
//!
//! 数据源在运行时侧：本模块当前如实渲染「未下发」态，等跨线契约就绪后由
//! `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------------- 纯数据层（无 GPUI 依赖） ----------------

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "助理协作空间";
/// 副标题（colleagues.subtitle）。
pub const SUBTITLE: &str = "选择一位助理加入任务，让专业角色帮你完成研究、分析和内容创作。";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 已下发但零条目时的空态说明（colleagues.empty）。
pub const EMPTY_HINT: &str = "暂无可用助理，创建或获取授权后会显示在这里。";
/// 计数文案模板（colleagues.countLabel），`{count}` 为占位符。
pub const COUNT_LABEL_TEMPLATE: &str = "共 {count} 位助理";
/// 状态标签：工作中（colleagues.statusAvailable）。
pub const STATUS_AVAILABLE: &str = "工作中";
/// 状态标签：已停用（colleagues.statusIdle）。
pub const STATUS_IDLE: &str = "已停用";
/// 操作：置顶（colleagues.action.pin）。
pub const ACTION_PIN: &str = "置顶";
/// 操作：编辑档案（colleagues.action.editProfile）。
pub const ACTION_EDIT_PROFILE: &str = "编辑档案";
/// 操作：暂停助理（colleagues.action.pauseColleague）。
pub const ACTION_PAUSE: &str = "暂停助理";

/// 助理状态（标签见 [`STATUS_AVAILABLE`] / [`STATUS_IDLE`]）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColleagueStatus {
    /// 工作中。
    Available,
    /// 已停用。
    Idle,
}

impl ColleagueStatus {
    /// 状态标签（逐字取自文案全表）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Available => STATUS_AVAILABLE,
            Self::Idle => STATUS_IDLE,
        }
    }
}

/// 一条助理条目：纯界面描述（⛔ 不含凭据 / 权限语义）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColleagueEntry {
    /// 名称（如「小研」）。
    pub name: &'static str,
    /// 角色头衔（如「需求分析与拆解专家」）。
    pub title: &'static str,
    /// 状态。
    pub status: ColleagueStatus,
    /// 是否置顶（置顶条目排在清单前面）。
    pub pinned: bool,
    /// 能力标签（逐字取自 colleagues.tag.*）。
    pub capabilities: &'static [&'static str],
}

/// 视图三态：**未下发 ≠ 空集**。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ColleaguesState {
    /// 数据未下发（fail-closed：如实显示「未下发」）。
    NotDelivered,
    /// 已下发但零条目（空态文案）。
    Empty,
    /// 已下发且有条目。
    Entries(Vec<ColleagueEntry>),
}

/// 助理协作空间视图模型（纯数据层，无 GPUI 依赖）。
#[derive(Clone, Debug)]
pub struct ColleaguesView {
    /// 三态。
    pub state: ColleaguesState,
}

impl ColleaguesView {
    /// 构造：`None` = 数据未下发（**不是**空集）。
    #[must_use]
    pub fn build(delivered: Option<Vec<ColleagueEntry>>) -> Self {
        let state = match delivered {
            None => ColleaguesState::NotDelivered,
            Some(entries) if entries.is_empty() => ColleaguesState::Empty,
            Some(entries) => ColleaguesState::Entries(entries),
        };
        Self { state }
    }

    /// 清单行序：置顶在前、组内保持传入顺序（稳定）。
    ///
    /// 非清单态返回空表（元素树据此退到空态 / 未下发分支）。
    #[must_use]
    pub fn ordered_entries(&self) -> Vec<&ColleagueEntry> {
        let ColleaguesState::Entries(entries) = &self.state else {
            return Vec::new();
        };
        let mut pinned: Vec<_> = entries.iter().filter(|e| e.pinned).collect();
        let mut rest: Vec<_> = entries.iter().filter(|e| !e.pinned).collect();
        pinned.append(&mut rest);
        pinned
    }

    /// 计数文案（colleagues.countLabel，`{count}` 已替换为条数）。
    #[must_use]
    pub fn count_label(&self) -> String {
        let ColleaguesState::Entries(entries) = &self.state else {
            return String::new();
        };
        COUNT_LABEL_TEMPLATE.replace("{count}", &entries.len().to_string())
    }
}

// ---------------- GPUI 层 ----------------

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
///
/// 数据源尚未接入：如实渲染「未下发」态。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &ColleaguesView::build(None))
}

/// 按视图三态渲染：标题行 + （未下发 / 空态 / 清单）。
fn render(theme: &Theme, view: &ColleaguesView) -> Div {
    let panel = div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header(theme, view));
    match &view.state {
        ColleaguesState::NotDelivered => panel.child(centered_note(theme, NOT_DELIVERED)),
        ColleaguesState::Empty => panel.child(centered_note(theme, EMPTY_HINT)),
        ColleaguesState::Entries(_) => panel.child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap_2()
                .p_4()
                .children(view.ordered_entries().into_iter().map(|e| entry_row(theme, e))),
        ),
    }
}

/// 标题行：标题 + 副标题；清单态追加计数。
fn header(theme: &Theme, view: &ColleaguesView) -> Div {
    let mut head = div()
        .px_4()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .border_b()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SUBTITLE),
        );
    if let ColleaguesState::Entries(_) = view.state {
        head = head.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(view.count_label())),
        );
    }
    head
}

/// 居中说明（未下发 / 空态共用）。
fn centered_note(theme: &Theme, text: &'static str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 单条清单行。
fn entry_row(theme: &Theme, entry: &ColleagueEntry) -> Div {
    div()
        .px_3()
        .py_2()
        .text_sm()
        .text_color(theme.foreground)
        .child(entry.name)
}

#[cfg(test)]
mod tests {
    use super::{
        main_area, ColleagueEntry, ColleagueStatus, ColleaguesState, ColleaguesView, ACTION_PIN,
        EMPTY_HINT, STATUS_AVAILABLE, STATUS_IDLE, SUBTITLE, TITLE,
    };
    use gpui_kit::component::Theme;

    /// 样例条目（名称 / 头衔逐字取自文案全表 colleagues.cards.*）。
    fn sample_entries() -> Vec<ColleagueEntry> {
        vec![
            ColleagueEntry {
                name: "小研",
                title: "需求分析与拆解专家",
                status: ColleagueStatus::Idle,
                pinned: false,
                capabilities: &["深度研究", "文档撰写"],
            },
            ColleagueEntry {
                name: "主编",
                title: "内容结构与质量把控",
                status: ColleagueStatus::Available,
                pinned: true,
                capabilities: &["文案策划", "内容润色"],
            },
            ColleagueEntry {
                name: "数据分析师",
                title: "指标口径与漏斗分析",
                status: ColleagueStatus::Available,
                pinned: false,
                capabilities: &["数据分析", "报告生成"],
            },
        ]
    }

    #[test]
    fn none_input_is_not_delivered_and_empty_vec_is_empty_state() {
        assert_eq!(
            ColleaguesView::build(None).state,
            ColleaguesState::NotDelivered
        );
        assert_eq!(
            ColleaguesView::build(Some(Vec::new())).state,
            ColleaguesState::Empty
        );
    }

    #[test]
    fn grouping_puts_pinned_first_and_counts_entries() {
        let view = ColleaguesView::build(Some(sample_entries()));
        let ordered = view.ordered_entries();
        assert_eq!(ordered.len(), 3);
        assert!(ordered[0].pinned, "置顶条目必须排第一");
        assert_eq!(ordered[0].name, "主编");
        assert_eq!(view.count_label(), "共 3 位助理");
        // 非清单态：行序为空、计数为空（不伪造）
        let not_delivered = ColleaguesView::build(None);
        assert!(not_delivered.ordered_entries().is_empty());
        assert_eq!(not_delivered.count_label(), "");
    }

    #[test]
    fn copy_is_verbatim_from_the_namespace() {
        assert_eq!(TITLE, "助理协作空间");
        assert_eq!(
            SUBTITLE,
            "选择一位助理加入任务，让专业角色帮你完成研究、分析和内容创作。"
        );
        assert_eq!(EMPTY_HINT, "暂无可用助理，创建或获取授权后会显示在这里。");
        assert_eq!(STATUS_AVAILABLE, "工作中");
        assert_eq!(STATUS_IDLE, "已停用");
        assert_eq!(ACTION_PIN, "置顶");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
