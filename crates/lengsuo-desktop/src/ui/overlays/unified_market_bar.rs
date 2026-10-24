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
/// 主区渲染（诚实占位：仅标题 + 未下发说明；本卡负责补全真实视图）。
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(TITLE))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}
#[cfg(test)]
mod tests {
    use super::{main_area, State, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
    }
    #[test]
    fn title_not_empty() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
