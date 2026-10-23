//! 助理协作空间主区视图（波次 5，卡 `ui-colleagues-panel`）。
//!
//! 分层（同 `env_gpui`）：上半**纯数据层**（无 GPUI 依赖：条目 / 置顶分组 / 三态），
//! 下半 **GPUI 层**（[`main_area`] 接元素树，颜色一律取主题 token）。
//!
//! 硬约束：**fail-closed**——数据未下发必须如实显示「未下发」，不得退化成空列表；
//! 只做界面（不判权限 / 不持凭据 / 不直连运行时）；⛔ 不伪造服务端数据，
//! 界面文案逐字取自文案全表命名空间 `colleagues`。数据源就绪后由
//! `ui-*-switch-source` 类卡接入（只换数据源、不改视图结构）。
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
/// 状态标签（colleagues.statusAvailable / statusIdle）。
pub const STATUS_AVAILABLE: &str = "工作中";
pub const STATUS_IDLE: &str = "已停用";
/// 操作行文案（colleagues.action.pin / editProfile / pauseColleague）。
pub const ACTION_PIN: &str = "置顶";
pub const ACTION_EDIT_PROFILE: &str = "编辑档案";
pub const ACTION_PAUSE: &str = "暂停助理";

/// 助理状态（标签见 [`STATUS_AVAILABLE`] / [`STATUS_IDLE`]）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColleagueStatus {
    Available,
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

    /// 清单行序：置顶在前、组内保持传入顺序（稳定）；非清单态返回空表。
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

/// 主区渲染（返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致）。
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
            div().flex_1().flex().flex_col().gap_2().p_4().children(
                view.ordered_entries()
                    .into_iter()
                    .map(|e| entry_row(theme, e)),
            ),
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
        .border_b_1()
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
        let count = SharedString::from(view.count_label());
        head = head.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(count),
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

/// 单条清单行：名称 / 头衔 / 状态 / 置顶徽标 + 操作行 + 能力标签行。
fn entry_row(theme: &Theme, entry: &ColleagueEntry) -> Div {
    let status_color = match entry.status {
        ColleagueStatus::Available => theme.foreground,
        ColleagueStatus::Idle => theme.muted_foreground,
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_md()
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(entry.name),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(entry.title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(status_color)
                        .child(entry.status.label()),
                )
                .children(entry.pinned.then(|| chip(theme, ACTION_PIN, theme.accent))),
        )
        .child(
            div().flex().flex_wrap().gap_2().children(
                [ACTION_PIN, ACTION_EDIT_PROFILE, ACTION_PAUSE]
                    .map(|label| chip(theme, label, theme.muted_foreground)),
            ),
        )
        .child(
            div().flex().flex_wrap().gap_2().children(
                entry
                    .capabilities
                    .iter()
                    .copied()
                    .map(|tag| chip(theme, tag, theme.muted_foreground)),
            ),
        )
}

/// 小标签 chip（操作 / 能力 / 置顶徽标共用形状，颜色由调用方传主题 token）。
fn chip(theme: &Theme, text: &'static str, color: Hsla) -> Div {
    div()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(color)
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(text)
}

#[cfg(test)]
mod tests {
    //! 显式 import（不写 `use super::*`，避免 GPUI 的 test 属性宏遮蔽标准库 `#[test]`）。
    use super::{
        entry_row, header, main_area, render, ColleagueEntry, ColleagueStatus, ColleaguesState,
        ColleaguesView, ACTION_EDIT_PROFILE, ACTION_PAUSE, ACTION_PIN, COUNT_LABEL_TEMPLATE,
        EMPTY_HINT, NOT_DELIVERED, STATUS_AVAILABLE, STATUS_IDLE, SUBTITLE, TITLE,
    };
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, Bounds, Div, InteractiveElement as _, IntoElement,
        ParentElement as _, Pixels, Styled as _, TestAppContext,
    };

    /// 探针选择器（每个用例独立窗口，名字固定即可）。
    const PROBE: &str = "colleagues-panel-probe";

    /// 样例条目（名称 / 头衔 / 标签逐字取自文案全表 colleagues.cards.* / colleagues.tag.*）。
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

    /// 无窗口绘制一帧，返回探针实际布局 bounds（None = 没进渲染帧）。
    fn draw_probe(element: Div) -> Option<Bounds<Pixels>> {
        let mut ctx = TestAppContext::single();
        ctx.update(|cx| cx.set_global(Theme::default()));
        let probe = div()
            .flex()
            .items_start()
            .child(div().debug_selector(|| PROBE.to_owned()).child(element));
        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(800.0), px(600.0)),
            move |_window, _cx| probe.into_any_element(),
        );
        visual.debug_bounds(PROBE)
    }

    /// bounds 存在且宽高非零（真的参与排版，不是只构造出来了）。
    fn assert_rendered(bounds: Option<Bounds<Pixels>>, what: &str) {
        let bounds = bounds.unwrap_or_else(|| panic!("{what}：没进入渲染帧"));
        assert!(bounds.size.width > px(0.0), "{what}：布局宽度为 0");
        assert!(bounds.size.height > px(0.0), "{what}：布局高度为 0");
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
        assert_eq!(COUNT_LABEL_TEMPLATE, "共 {count} 位助理");
        assert_eq!(STATUS_AVAILABLE, "工作中");
        assert_eq!(STATUS_IDLE, "已停用");
        assert_eq!(ACTION_PIN, "置顶");
        assert_eq!(ACTION_EDIT_PROFILE, "编辑档案");
        assert_eq!(ACTION_PAUSE, "暂停助理");
        assert_eq!(NOT_DELIVERED, "未下发");
        // 样例条目的名称 / 头衔 / 标签同样逐字
        assert_eq!(sample_entries()[0].name, "小研");
        assert_eq!(sample_entries()[0].title, "需求分析与拆解专家");
        assert_eq!(sample_entries()[0].capabilities, &["深度研究", "文档撰写"]);
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    /// 元素树定位：标题节点 + ≥3 个清单节点逐一真实绘制且布局非零。
    #[test]
    fn element_tree_locates_title_and_list_rows() {
        let theme = Theme::default();
        let view = ColleaguesView::build(Some(sample_entries()));
        assert_rendered(draw_probe(header(&theme, &view)), "标题节点");
        let ordered = view.ordered_entries();
        assert!(ordered.len() >= 3, "样例至少 3 行，元素树断言才有意义");
        for entry in ordered {
            assert_rendered(draw_probe(entry_row(&theme, entry)), "清单行");
        }
    }

    /// 三态整帧绘制均不 panic 且非零（含 fail-closed 的「未下发」）。
    #[test]
    fn all_states_render_nonzero() {
        let theme = Theme::default();
        let views = [
            ColleaguesView::build(None),
            ColleaguesView::build(Some(Vec::new())),
            ColleaguesView::build(Some(sample_entries())),
        ];
        for view in &views {
            assert_rendered(draw_probe(render(&theme, view)), "三态整帧");
        }
    }
}
