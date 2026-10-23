//! 专家中心主区视图（波次 5，卡 `ui-expert-center`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：条目 / 空态 / 诚实占位文案；
//! - **GPUI 层**（[`main_area`]）：把描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自文案全表命名空间 `expertCenter`）。
//!
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后由
//! `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "专家中心";

/// 副标题（逐字取自界面文案全表）。
pub const SUBTITLE: &str = "按行业分类浏览专家，召唤他们为你服务";

/// 分类筛选首项（逐字取自界面文案全表）。
pub const ALL_CATEGORY: &str = "全部";

/// 分组头标签（逐字取自界面文案全表）。
pub const CATEGORY_LABEL: &str = "行业分类";

/// 卡片字段标签：擅长领域（逐字取自界面文案全表）。
pub const STRENGTH_LABEL: &str = "擅长领域";

/// 卡片字段标签：能力介绍（逐字取自界面文案全表）。
pub const ABILITY_INTRO_LABEL: &str = "能力介绍";

/// 标题行动作（逐字取自界面文案全表）。
pub const CREATE_EXPERT: &str = "创建专家";

/// 空态文案（逐字取自界面文案全表）。
pub const EMPTY_TEXT: &str = "暂无该分类的专家";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 一条专家条目（纯数据，无 GPUI 依赖）。
pub struct ExpertEntry {
    /// 专家名称。
    pub name: String,
    /// 行业分类（筛选与分组依据）。
    pub category: String,
    /// 擅长领域。
    pub strengths: String,
    /// 能力介绍。
    pub ability_intro: String,
}

/// 可枚举的列表状态：未下发 / 已下发为空 / 已下发有条目。
pub enum ExpertCenterState {
    /// 数据未下发：fail-closed，如实显示「未下发」。
    NotDelivered,
    /// 已下发但为空：显示空态文案，**不是**空列表。
    Empty,
    /// 已下发且有条目。
    Entries(Vec<ExpertEntry>),
}

/// 专家中心视图的纯数据层。
pub struct ExpertCenterView {
    state: ExpertCenterState,
}

impl ExpertCenterView {
    /// 构造：`None` = 未下发；`Some(空)` = 空态；`Some(非空)` = 条目列表。
    #[must_use]
    pub fn build(delivered: Option<Vec<ExpertEntry>>) -> Self {
        let state = match delivered {
            None => ExpertCenterState::NotDelivered,
            Some(entries) if entries.is_empty() => ExpertCenterState::Empty,
            Some(entries) => ExpertCenterState::Entries(entries),
        };
        Self { state }
    }

    /// 当前状态。
    pub fn state(&self) -> &ExpertCenterState {
        &self.state
    }

    /// 去重后的行业分类（按首次出现排序）。
    pub fn categories(&self) -> Vec<&str> {
        let mut seen: Vec<&str> = Vec::new();
        if let ExpertCenterState::Entries(entries) = &self.state {
            for entry in entries {
                if !seen.contains(&entry.category.as_str()) {
                    seen.push(entry.category.as_str());
                }
            }
        }
        seen
    }

    /// 指定分类下的条目（无该分类则为空）。
    fn entries_in(&self, category: &str) -> Vec<&ExpertEntry> {
        match &self.state {
            ExpertCenterState::Entries(entries) => entries
                .iter()
                .filter(|entry| entry.category == category)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 按行业分类分组，供筛选栏与清单行共用（空组不出现）。
    pub fn grouped(&self) -> Vec<(&str, Vec<&ExpertEntry>)> {
        self.categories()
            .into_iter()
            .map(|category| (category, self.entries_in(category)))
            .collect()
    }
}

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
///
/// 当前无数据源：`build(None)` 走「未下发」fail-closed 分支；
/// 空态与条目分支等数据源就绪后由换源卡点亮，**不改视图结构**。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    let view = ExpertCenterView::build(None);
    let page = div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header(theme));
    match view.state() {
        ExpertCenterState::NotDelivered => page.child(centered_note(theme, NOT_DELIVERED)),
        ExpertCenterState::Empty => page
            .child(filter_bar(theme, &view))
            .child(centered_note(theme, EMPTY_TEXT)),
        ExpertCenterState::Entries(_) => page
            .child(filter_bar(theme, &view))
            .child(group_list(theme, &view).flex_1()),
    }
}

/// 标题行：标题 + 副标题，右侧「创建专家」入口（静态，待接线）。
fn header(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
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
                ),
        )
        .child(
            div()
                .px_2()
                .py_1()
                .text_xs()
                .border_1()
                .border_color(theme.border)
                .rounded_sm()
                .text_color(theme.muted_foreground)
                .child(CREATE_EXPERT),
        )
}

/// 居中说明行（未下发 / 空态共用）。
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

/// 行业分类筛选栏：首项「全部」+ 去重分类。
fn filter_bar(theme: &Theme, view: &ExpertCenterView) -> Div {
    let mut bar = div()
        .flex()
        .flex_wrap()
        .gap_2()
        .px_4()
        .py_2()
        .child(category_chip(theme, ALL_CATEGORY, true));
    for category in view.categories() {
        bar = bar.child(category_chip(theme, category, false));
    }
    bar
}

/// 分类 chip：选中项用 accent 边框，未选中用普通边框。
fn category_chip(theme: &Theme, label: impl Into<SharedString>, selected: bool) -> Div {
    let label = label.into();
    let (fg, border) = if selected {
        (theme.foreground, theme.accent)
    } else {
        (theme.muted_foreground, theme.border)
    };
    div()
        .px_2()
        .py_1()
        .text_xs()
        .border_1()
        .border_color(border)
        .rounded_sm()
        .text_color(fg)
        .child(label)
}

/// 分组清单：每个行业分类一行分组头 + 该分类下的专家卡片。
fn group_list(theme: &Theme, view: &ExpertCenterView) -> Div {
    let mut list = div().flex().flex_col().gap_3().px_4().pb_4();
    for (category, entries) in view.grouped() {
        list = list.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(group_header(theme, category))
                .children(entries.into_iter().map(|entry| expert_card(theme, entry))),
        );
    }
    list
}

/// 分组头：「行业分类」标签 + 分类名。
fn group_header(theme: &Theme, category: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(CATEGORY_LABEL),
        )
        .child(
            div()
                .text_xs()
                .font_bold()
                .text_color(theme.foreground)
                .child(SharedString::from(category)),
        )
}

/// 专家卡片：名称 + 擅长领域 + 能力介绍。
fn expert_card(theme: &Theme, entry: &ExpertEntry) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(SharedString::from(entry.name.clone())),
        )
        .child(labeled_line(theme, STRENGTH_LABEL, &entry.strengths))
        .child(labeled_line(theme, ABILITY_INTRO_LABEL, &entry.ability_intro))
}

/// 「标签 + 值」一行。
fn labeled_line(theme: &Theme, label: &'static str, value: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.foreground)
                .child(SharedString::from(value)),
        )
}

#[cfg(test)]
mod tests {
    use super::{main_area, NOT_DELIVERED, TITLE};
    use gpui_kit::component::Theme;

    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert_eq!(TITLE, "专家中心");
    }

    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
