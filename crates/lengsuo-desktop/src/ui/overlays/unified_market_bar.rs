//! 统一市场导航条（波次 8，卡 `ui-overlay-unified-market-bar`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间。
//! 数据源在运行时侧：默认态如实显示「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "统一市场导航条";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
// ---- 纯数据层：文案命名空间 `unifiedMarket`，共 14 键，逐字取自全表 ----
/// `unifiedMarket.addSkill`
pub const ADD_SKILL: &str = "添加技能";
/// `unifiedMarket.backToAll`
pub const BACK_TO_ALL: &str = "全部专家";
/// `unifiedMarket.backToAllSkills`
pub const BACK_TO_ALL_SKILLS: &str = "全部技能";
/// `unifiedMarket.customConnector`
pub const CUSTOM_CONNECTOR: &str = "自定义连接器";
/// `unifiedMarket.myExperts`
pub const MY_EXPERTS: &str = "我的专家";
/// `unifiedMarket.myInstalled`
pub const MY_INSTALLED: &str = "已安装";
/// `unifiedMarket.search.experts`
pub const SEARCH_EXPERTS: &str = "搜索专家职称或描述";
/// `unifiedMarket.search.skills`
pub const SEARCH_SKILLS: &str = "搜索技能";
/// `unifiedMarket.tab.connectors`
pub const TAB_CONNECTORS: &str = "连接器";
/// `unifiedMarket.tab.connectors.tooltip`
pub const TAB_CONNECTORS_TOOLTIP: &str = "帮你连接外部应用数据";
/// `unifiedMarket.tab.experts`
pub const TAB_EXPERTS: &str = "专家";
/// `unifiedMarket.tab.experts.tooltip`
pub const TAB_EXPERTS_TOOLTIP: &str = "以专家视角完成你的任务";
/// `unifiedMarket.tab.skills`
pub const TAB_SKILLS: &str = "技能";
/// `unifiedMarket.tab.skills.tooltip`
pub const TAB_SKILLS_TOOLTIP: &str = "适合高效完成特定的任务";
/// 命名空间 `unifiedMarket` 的文案键总数。
pub const KEY_COUNT: usize = 14;
/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    /// 拉取中。
    Loading,
    /// 拉取失败。
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}
/// 市场标签页（专家 / 技能 / 连接器）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    /// `unifiedMarket.tab.experts`
    #[default]
    Experts,
    /// `unifiedMarket.tab.skills`
    Skills,
    /// `unifiedMarket.tab.connectors`
    Connectors,
}
/// 列表范围：全部（backToAll 系列）或我的（myExperts / myInstalled）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    /// 全部专家 / 全部技能。
    #[default]
    All,
    /// 我的专家 / 已安装。
    My,
}
/// 标签页条目（名称 + 悬停提示，均为全表文案）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabItem {
    pub tab: Tab,
    pub label: &'static str,
    pub tooltip: &'static str,
}
/// 浮层纯数据（无 GPUI 依赖；视图 = [`render`] 的纯函数）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnifiedMarketBar {
    pub state: State,
    pub tab: Tab,
    pub scope: Scope,
}
impl UnifiedMarketBar {
    /// 空态文案：仅 `Normal` 有内容可画；其余状态 fail-closed 显示「未下发」。
    pub fn empty_state(&self) -> Option<&'static str> {
        match self.state {
            State::Normal => None,
            State::Loading | State::Failed | State::NotDelivered => Some(NOT_DELIVERED),
        }
    }
}
/// 三个标签页条目（固定顺序：专家 / 技能 / 连接器）。
pub fn tab_items() -> [TabItem; 3] {
    [
        TabItem {
            tab: Tab::Experts,
            label: TAB_EXPERTS,
            tooltip: TAB_EXPERTS_TOOLTIP,
        },
        TabItem {
            tab: Tab::Skills,
            label: TAB_SKILLS,
            tooltip: TAB_SKILLS_TOOLTIP,
        },
        TabItem {
            tab: Tab::Connectors,
            label: TAB_CONNECTORS,
            tooltip: TAB_CONNECTORS_TOOLTIP,
        },
    ]
}
/// 搜索占位文案；连接器页全表未下发对应键 ⇒ `None`（⛔ 不自编）。
pub fn search_placeholder(tab: Tab) -> Option<&'static str> {
    match tab {
        Tab::Experts => Some(SEARCH_EXPERTS),
        Tab::Skills => Some(SEARCH_SKILLS),
        Tab::Connectors => None,
    }
}
/// 列表标题（backToAll / my 系列）；连接器页未下发 ⇒ `None`。
pub fn list_heading(tab: Tab, scope: Scope) -> Option<&'static str> {
    match (tab, scope) {
        (Tab::Experts, Scope::All) => Some(BACK_TO_ALL),
        (Tab::Experts, Scope::My) => Some(MY_EXPERTS),
        (Tab::Skills, Scope::All) => Some(BACK_TO_ALL_SKILLS),
        (Tab::Skills, Scope::My) => Some(MY_INSTALLED),
        (Tab::Connectors, _) => None,
    }
}
/// 当前页主操作文案；专家页无对应键 ⇒ `None`。
pub fn action_label(tab: Tab) -> Option<&'static str> {
    match tab {
        Tab::Skills => Some(ADD_SKILL),
        Tab::Connectors => Some(CUSTOM_CONNECTOR),
        Tab::Experts => None,
    }
}
/// 主区渲染（默认态 fail-closed 显示「未下发」；签名稳定，供浮层挂载点调用）。
pub fn main_area(theme: &Theme) -> Div {
    render(&UnifiedMarketBar::default(), theme)
}
/// 浮层渲染：标题行 + 空态分支 / 内容节点（标签页 + 提示 + 搜索 + 列表标题 + 主操作）。
/// 颜色一律取 `theme.*` token，⛔ 不硬编码色值。
pub fn render(data: &UnifiedMarketBar, theme: &Theme) -> Div {
    let foreground = theme.foreground;
    let muted = theme.muted_foreground;
    let column = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(foreground).child(TITLE));
    if let Some(empty) = data.empty_state() {
        return column.child(div().text_color(muted).child(empty));
    }
    let items = tab_items();
    let active = items
        .iter()
        .find(|item| item.tab == data.tab)
        .expect("tab_items 覆盖所有 Tab 分支");
    column
        .child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .children(items.iter().map(|item| {
                    let color = if item.tab == data.tab {
                        foreground
                    } else {
                        muted
                    };
                    div().text_color(color).child(item.label)
                })),
        )
        .child(div().text_color(muted).child(active.tooltip))
        .children(
            search_placeholder(data.tab)
                .map(|placeholder| div().text_color(muted).child(placeholder)),
        )
        .children(
            list_heading(data.tab, data.scope)
                .map(|heading| div().text_color(foreground).child(heading)),
        )
        .children(action_label(data.tab).map(|action| div().text_color(foreground).child(action)))
}
#[cfg(test)]
mod tests {
    use super::{
        action_label, list_heading, main_area, render, search_placeholder, tab_items, Scope, State,
        Tab, UnifiedMarketBar, ADD_SKILL, BACK_TO_ALL, BACK_TO_ALL_SKILLS, CUSTOM_CONNECTOR,
        KEY_COUNT, MY_EXPERTS, MY_INSTALLED, NOT_DELIVERED, SEARCH_EXPERTS, SEARCH_SKILLS,
        TAB_CONNECTORS, TAB_CONNECTORS_TOOLTIP, TAB_EXPERTS, TAB_EXPERTS_TOOLTIP, TAB_SKILLS,
        TAB_SKILLS_TOOLTIP, TITLE,
    };
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(
            UnifiedMarketBar::default(),
            UnifiedMarketBar {
                state: State::NotDelivered,
                tab: Tab::Experts,
                scope: Scope::All,
            }
        );
    }
    #[test]
    fn pure_layer_counts() {
        assert_eq!(tab_items().len(), 3);
        assert_eq!(KEY_COUNT, 14);
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn empty_input_is_empty_state_not_empty_list() {
        assert_eq!(
            UnifiedMarketBar::default().empty_state(),
            Some(NOT_DELIVERED)
        );
    }
    #[test]
    fn copy_is_verbatim_from_table() {
        let table = [
            (ADD_SKILL, "添加技能"),
            (BACK_TO_ALL, "全部专家"),
            (BACK_TO_ALL_SKILLS, "全部技能"),
            (CUSTOM_CONNECTOR, "自定义连接器"),
            (MY_EXPERTS, "我的专家"),
            (MY_INSTALLED, "已安装"),
            (SEARCH_EXPERTS, "搜索专家职称或描述"),
            (SEARCH_SKILLS, "搜索技能"),
            (TAB_CONNECTORS, "连接器"),
            (TAB_CONNECTORS_TOOLTIP, "帮你连接外部应用数据"),
            (TAB_EXPERTS, "专家"),
            (TAB_EXPERTS_TOOLTIP, "以专家视角完成你的任务"),
            (TAB_SKILLS, "技能"),
            (TAB_SKILLS_TOOLTIP, "适合高效完成特定的任务"),
        ];
        assert_eq!(table.len(), KEY_COUNT);
        for (got, want) in table {
            assert_eq!(got, want);
        }
    }
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
    #[test]
    fn render_covers_every_state_branch() {
        for state in [
            State::Normal,
            State::Loading,
            State::Failed,
            State::NotDelivered,
        ] {
            let bar = UnifiedMarketBar {
                state,
                ..UnifiedMarketBar::default()
            };
            let expected = match state {
                State::Normal => None,
                _ => Some(NOT_DELIVERED),
            };
            assert_eq!(bar.empty_state(), expected, "state={state:?}");
            let _ = render(&bar, &Theme::default());
        }
    }
    #[test]
    fn mappings_are_fail_closed_where_key_missing() {
        assert_eq!(search_placeholder(Tab::Experts), Some(SEARCH_EXPERTS));
        assert_eq!(search_placeholder(Tab::Skills), Some(SEARCH_SKILLS));
        assert_eq!(search_placeholder(Tab::Connectors), None);
        assert_eq!(list_heading(Tab::Experts, Scope::All), Some(BACK_TO_ALL));
        assert_eq!(list_heading(Tab::Experts, Scope::My), Some(MY_EXPERTS));
        assert_eq!(
            list_heading(Tab::Skills, Scope::All),
            Some(BACK_TO_ALL_SKILLS)
        );
        assert_eq!(list_heading(Tab::Skills, Scope::My), Some(MY_INSTALLED));
        assert_eq!(list_heading(Tab::Connectors, Scope::All), None);
        assert_eq!(action_label(Tab::Skills), Some(ADD_SKILL));
        assert_eq!(action_label(Tab::Connectors), Some(CUSTOM_CONNECTOR));
        assert_eq!(action_label(Tab::Experts), None);
    }
}
