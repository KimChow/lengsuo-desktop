//! 灵感社区主区视图（波次 5，卡 `ui-discover-panel`）。
//!
//! 分层约定（同 `env_gpui`）：上半为**纯数据层**（无 GPUI 依赖），
//! 下半 [`main_area`] 把描述接到元素树，颜色一律取主题 token。
//! 硬约束：**fail-closed**（数据未下发如实显示「未下发」，不伪造、不退化成空列表）；
//! 只做界面，不判权限、不持凭据；文案逐字取自文案全表命名空间 `discover`。
//! 数据源在运行时侧：等跨线契约就绪后由 `ui-*-switch-source` 类卡换数据源。
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
    All,
    Featured,
    Favorites,
}

impl Category {
    /// 渲染顺序固定的全部分类。
    pub const ALL: [Category; 3] = [Category::All, Category::Featured, Category::Favorites];

    /// 分类标签（逐字文案）。
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
    pub fn favorite_label(&self) -> &'static str {
        if self.favorite {
            FAVORITED
        } else {
            FAVORITE
        }
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
    DiscoverView {
        state: DiscoverState::Delivered(entries),
        category: Category::All,
    }
}

/// 数据未下发时的 fail-closed 构造。
#[must_use]
pub fn not_delivered() -> DiscoverView {
    DiscoverView {
        state: DiscoverState::NotDelivered,
        category: Category::All,
    }
}

impl DiscoverView {
    /// 是否「未下发」。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self.state, DiscoverState::NotDelivered)
    }

    /// 当前分类。
    pub fn category(&self) -> Category {
        self.category
    }

    /// 切换分类（纯函数式：借用旧视图，返回新视图）。
    pub fn with_category(&self, category: Category) -> Self {
        let mut next = self.clone();
        next.category = category;
        next
    }

    /// 当前分类下可见的清单行（「未下发」时为空，用 [`Self::is_not_delivered`] 区分二者）。
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

/// 主区渲染（当前无数据源 ⇒ 固定渲染「未下发」分支）。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&not_delivered(), theme)
}

/// 把纯数据层接到元素树（各分支同为 `Div`；颜色一律取主题 token）。
fn render(view: &DiscoverView, theme: &Theme) -> Div {
    let body = if view.is_not_delivered() {
        centered_line(NOT_DELIVERED, true, theme)
    } else if let Some((line, hint)) = view.empty_lines() {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_1()
            .child(centered_line(line, false, theme))
            .child(centered_line(hint, true, theme))
    } else {
        div()
            .flex()
            .flex_col()
            .flex_1()
            .gap_2()
            .p_4()
            .children(view.rows().iter().map(|e| entry_row(e, theme)))
    };

    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header(theme))
        .child(category_row(view.category(), theme))
        .child(body)
}

/// 标题行：主标题 + 副标题。
fn header(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
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
        )
}

/// 分组行：三个分类 tab，当前分类用 accent 强调、其余走弱化前景。
fn category_row(current: Category, theme: &Theme) -> Div {
    let muted_fg = theme.muted_foreground;
    div()
        .px_4()
        .pb_2()
        .flex()
        .gap_2()
        .children(Category::ALL.iter().map(|c| {
            let selected = *c == current;
            div()
                .px_2()
                .py_1()
                .text_xs()
                .rounded_md()
                .border_1()
                .border_color(if selected { theme.accent } else { theme.border })
                .text_color(if selected { theme.accent } else { muted_fg })
                .child(c.label())
        }))
}

/// 清单行：一张灵感卡片（标题 + 收藏标记 + 主操作）。
fn entry_row(entry: &DiscoverEntry, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
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
                        .child(SharedString::from(entry.title.as_str())),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(entry.favorite_label()),
                ),
        )
        .child(div().text_xs().text_color(theme.accent).child(ADD_TO_TASK))
}

/// 居中单行文案（「未下发」/ 空态共用；`muted` 选择弱化色）。
fn centered_line(line: &'static str, muted: bool, theme: &Theme) -> Div {
    let muted_fg = theme.muted_foreground;
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(if muted { muted_fg } else { theme.foreground })
        .child(line)
}

#[cfg(test)]
mod tests {
    use super::{
        build, not_delivered, Category, DiscoverEntry, EMPTY_CATEGORY, EMPTY_CATEGORY_HINT,
        EMPTY_FAVORITES, EMPTY_FAVORITES_HINT,
    };
    use gpui_kit::component::Theme;

    fn entry(title: &str, featured: bool, favorite: bool) -> DiscoverEntry {
        DiscoverEntry {
            title: title.to_owned(),
            featured,
            favorite,
        }
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
        assert_eq!((line, hint), (EMPTY_CATEGORY, EMPTY_CATEGORY_HINT));

        let favorites = view.with_category(Category::Favorites);
        assert_eq!(
            favorites.empty_lines(),
            Some((EMPTY_FAVORITES, EMPTY_FAVORITES_HINT))
        );
    }

    #[test]
    fn copy_is_verbatim_from_discover_namespace() {
        assert_eq!(super::TITLE, "灵感社区");
        assert_eq!(super::SUBTITLE, "常见工作流沉淀成可复用的任务起点");
        assert_eq!(super::CATEGORY_ALL, "全部");
        assert_eq!(EMPTY_CATEGORY_HINT, "切换其他分类看看");
        assert_eq!(super::ADD_TO_TASK, "添加到任务（来自社区）");
        assert_eq!((super::FAVORITE, super::FAVORITED), ("收藏", "已收藏"));
        // 收藏标记随条目状态切换。
        assert_eq!(entry("a", false, false).favorite_label(), super::FAVORITE);
        assert_eq!(entry("a", false, true).favorite_label(), super::FAVORITED);
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
    fn title_plus_three_rows_reach_the_tree() {
        // 元素树的标题节点与 ≥3 个清单行由纯数据层驱动，这里锁定其来源。
        let view = build(vec![
            entry("r1", false, false),
            entry("r2", true, false),
            entry("r3", false, true),
        ]);
        assert_eq!(super::TITLE, "灵感社区"); // 标题节点
        let rows = view.rows();
        assert_eq!(rows.len(), 3); // ≥3 个清单行
        let titles: Vec<_> = rows.iter().map(|e| e.title.as_str()).collect();
        assert_eq!(titles, ["r1", "r2", "r3"]);
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        // 当前真实分支：无数据源 ⇒ 「未下发」。
        let _ = super::main_area(&theme);

        // 其余分支（有数据 / 空态 × 全分类）也必须能构建元素树。
        let delivered = build(vec![entry("a", true, false), entry("b", false, true)]);
        for category in Category::ALL {
            let _ = super::render(&delivered.with_category(category), &theme);
        }
        let _ = super::render(&build(vec![]), &theme);
    }
}
