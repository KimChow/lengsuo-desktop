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
