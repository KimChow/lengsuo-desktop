//! 灵感社区主区视图（波次 5，卡 `ui-discover-panel`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：条目 / 分类筛选 / 空态 / 未下发；
//! - **GPUI 层**（[`main_area`]）：把描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自界面文案全表命名空间 `discover`）。
//!
//! 数据源在运行时侧：[`main_area`] 当前固定渲染「未下发」分支，等跨线契约就绪后
//! 由 `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------------- 纯数据层（无 GPUI 依赖） ----------------

/// 视图标题（逐字：`discover.title`）。
pub const TITLE: &str = "灵感社区";
/// 副标题（逐字：`discover.subtitle`）。
pub const SUBTITLE: &str = "常见工作流沉淀成可复用的任务起点";
/// 分类「全部」标签（逐字：`discover.allCategory`）。
pub const CATEGORY_ALL: &str = "全部";
/// 分类「精选」标签（逐字：`discover.featured`）。
pub const CATEGORY_FEATURED: &str = "精选";
/// 分类「我的收藏」标签（逐字：`discover.myFavorites`）。
pub const CATEGORY_FAVORITES: &str = "我的收藏";
/// 收藏标记·未收藏（逐字：`discover.favorite`）。
pub const FAVORITE: &str = "收藏";
/// 收藏标记·已收藏（逐字：`discover.favorited`）。
pub const FAVORITED: &str = "已收藏";
/// 卡片主操作（逐字：`discover.addToTaskFromCommunity`）。
pub const ADD_TO_TASK: &str = "添加到任务（来自社区）";
/// 空态主文案（逐字：`discover.noCategoryResults`）。
pub const EMPTY_CATEGORY: &str = "当前分类下暂无内容";
/// 空态提示（逐字：`discover.noCategoryResultsHint`）。
pub const EMPTY_CATEGORY_HINT: &str = "切换其他分类看看";
/// 收藏分类空态主文案（逐字：`discover.noFavorites`）。
pub const EMPTY_FAVORITES: &str = "暂无收藏";
/// 收藏分类空态提示（逐字：`discover.noFavoritesHint`）。
pub const EMPTY_FAVORITES_HINT: &str = "点击案例卡片上的收藏图标添加收藏";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 分类筛选（面板顶部的固定三个 tab）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// 全部条目。
    All,
    /// 仅精选条目。
    Featured,
    /// 仅收藏条目。
    Favorites,
}

impl Category {
    /// 渲染顺序固定的全部分类。
    pub const ALL: [Category; 3] = [Category::All, Category::Featured, Category::Favorites];

    /// 分类标签（逐字文案）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Category::All => CATEGORY_ALL,
            Category::Featured => CATEGORY_FEATURED,
            Category::Favorites => CATEGORY_FAVORITES,
        }
    }
}

/// 一条社区灵感（卡片流的纯数据描述）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoverEntry {
    /// 案例标题。
    pub title: String,
    /// 精选标记（「精选」分类的过滤依据）。
    pub featured: bool,
    /// 收藏标记（「我的收藏」分类的过滤依据）。
    pub favorite: bool,
}

impl DiscoverEntry {
    /// 收藏标记文案（随收藏状态切换，逐字取自文案全表）。
    #[must_use]
    pub fn favorite_label(&self) -> &'static str {
        if self.favorite { FAVORITED } else { FAVORITE }
    }
}

/// 可枚举的视图状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiscoverState {
    /// 数据未下发（fail-closed，如实显示「未下发」）。
    NotDelivered,
    /// 数据已下发（空清单 ⇒ 空态，**不是**「未下发」）。
    Delivered(Vec<DiscoverEntry>),
}

/// 灵感社区视图的纯数据层。
#[derive(Clone, Debug, PartialEq)]
pub struct DiscoverView {
    state: DiscoverState,
    category: Category,
}

/// 从已下发的条目构造视图（空输入 ⇒ 空态，仍属于 `Delivered`）。
#[must_use]
pub fn build(entries: Vec<DiscoverEntry>) -> DiscoverView {
    DiscoverView { state: DiscoverState::Delivered(entries), category: Category::All }
}

/// 数据未下发时的 fail-closed 构造。
#[must_use]
pub fn not_delivered() -> DiscoverView {
    DiscoverView { state: DiscoverState::NotDelivered, category: Category::All }
}

impl DiscoverView {
    /// 当前状态。
    #[must_use]
    pub fn state(&self) -> &DiscoverState {
        &self.state
    }

    /// 是否「未下发」。
    #[must_use]
    pub fn is_not_delivered(&self) -> bool {
        matches!(self.state, DiscoverState::NotDelivered)
    }

    /// 当前分类。
    #[must_use]
    pub fn category(&self) -> Category {
        self.category
    }

    /// 切换分类（纯函数式：借用旧视图，返回新视图）。
    #[must_use]
    pub fn with_category(&self, category: Category) -> Self {
        let mut next = self.clone();
        next.category = category;
        next
    }

    /// 当前分类下可见的清单行（「未下发」时为空，用 [`Self::is_not_delivered`] 区分二者）。
    #[must_use]
    pub fn rows(&self) -> Vec<&DiscoverEntry> {
        let DiscoverState::Delivered(entries) = &self.state else {
            return Vec::new();
        };
        entries
            .iter()
            .filter(|e| match self.category {
                Category::All => true,
                Category::Featured => e.featured,
                Category::Favorites => e.favorite,
            })
            .collect()
    }

    /// 空态文案（主行 + 提示行）；「未下发」或清单非空时返回 `None`。
    #[must_use]
    pub fn empty_lines(&self) -> Option<(&'static str, &'static str)> {
        if self.is_not_delivered() || !self.rows().is_empty() {
            return None;
        }
        Some(match self.category {
            Category::Favorites => (EMPTY_FAVORITES, EMPTY_FAVORITES_HINT),
            _ => (EMPTY_CATEGORY, EMPTY_CATEGORY_HINT),
        })
    }
}

// ---------------- GPUI 层 ----------------

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
    use super::{build, not_delivered, Category, DiscoverEntry};
    use gpui_kit::component::Theme;

    fn entry(title: &str, featured: bool, favorite: bool) -> DiscoverEntry {
        DiscoverEntry { title: title.to_owned(), featured, favorite }
    }

    #[test]
    fn grouping_counts_per_category() {
        let entries = vec![
            entry("a", true, false),
            entry("b", true, true),
            entry("c", false, true),
            entry("d", false, false),
        ];
        let view = build(entries);
        assert_eq!(view.with_category(Category::All).rows().len(), 4);
        assert_eq!(view.with_category(Category::Featured).rows().len(), 2);
        assert_eq!(view.with_category(Category::Favorites).rows().len(), 2);
    }

    #[test]
    fn empty_input_yields_empty_state_not_blank_list() {
        let view = build(vec![]);
        assert!(!view.is_not_delivered());
        assert!(view.rows().is_empty());
        let (line, hint) = view.empty_lines().expect("空输入必须给出空态文案");
        assert_eq!((line, hint), (super::EMPTY_CATEGORY, super::EMPTY_CATEGORY_HINT));

        let favorites = view.with_category(Category::Favorites);
        assert_eq!(
            favorites.empty_lines(),
            Some((super::EMPTY_FAVORITES, super::EMPTY_FAVORITES_HINT))
        );
    }

    #[test]
    fn copy_is_verbatim_from_discover_namespace() {
        assert_eq!(super::TITLE, "灵感社区");
        assert_eq!(super::SUBTITLE, "常见工作流沉淀成可复用的任务起点");
        assert_eq!(super::CATEGORY_ALL, "全部");
        assert_eq!(super::EMPTY_CATEGORY_HINT, "切换其他分类看看");
        assert_eq!(super::ADD_TO_TASK, "添加到任务（来自社区）");
        assert_eq!((super::FAVORITE, super::FAVORITED), ("收藏", "已收藏"));
    }

    #[test]
    fn not_delivered_is_fail_closed() {
        let view = not_delivered();
        assert!(view.is_not_delivered());
        assert!(view.rows().is_empty());
        assert_eq!(view.empty_lines(), None);
        assert_eq!(super::NOT_DELIVERED, "未下发");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = super::main_area(&theme);
    }
}
