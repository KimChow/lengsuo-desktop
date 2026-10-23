//! 技能主区视图（波次 5，卡 `ui-skills-market`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：条目 / 分类筛选 / 空态 / 文案常量；
//! - **GPUI 层**（[`main_area`]）：把视图模型接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自界面文案全表命名空间 `skills`）。
//!
//! 数据源在运行时侧：`main_area` 当前固定构造 `NotDelivered` 视图（诚实占位），
//! 等数据源接入后**只换数据、不改视图结构**。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------------- 纯数据层（无 GPUI 依赖） ----------------

/// 视图标题（skills.title，逐字）。
pub const TITLE: &str = "技能";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 添加技能按钮（skills.add.button，逐字）。
pub const ADD_BUTTON: &str = "添加技能";

/// 搜索框占位（skills.searchPlaceholder，逐字）。
pub const SEARCH_PLACEHOLDER: &str = "搜索技能";

/// 空态标题（skills.installed.empty.title，逐字）。
pub const EMPTY_TITLE: &str = "还没有安装任何技能";

/// 空态说明（skills.installed.empty.desc，逐字）。
pub const EMPTY_DESC: &str = "上传本地技能文件夹，或通过 AI 对话快速创建一个专属技能";

/// 自动更新设置行标签（skills.autoUpdate.label，逐字）。
pub const AUTO_UPDATE_LABEL: &str = "技能自动更新";

/// 自动更新设置行说明（skills.autoUpdate.description，逐字；`{name}` 为应用名占位）。
pub const AUTO_UPDATE_DESCRIPTION: &str =
    "开启后将自动更新已安装的技能为最新版本，不会更新你在 {name} 中编辑过的技能";

/// 安装次数统计模板（skills.card.installCount，逐字；`{count}` 为次数占位）。
pub const CARD_INSTALL_COUNT: &str = "安装次数 {count}";

/// 自动触发开启时的说明（skills.toggle.tip.enable，逐字）。
pub const TOGGLE_TIP_ENABLE: &str =
    "开启技能自动触发，WorkBuddy 将在对话中自动判断是否使用此技能";

/// 自动触发关闭时的说明（skills.toggle.tip.disable，逐字）。
pub const TOGGLE_TIP_DISABLE: &str =
    "关闭自动触发后，WorkBuddy 将不在对话中自动触发此技能，仅支持手动触发";

/// 分类 Tab：已安装（skills.tab.installed，逐字）。
pub const TAB_INSTALLED: &str = "已安装";
/// 分类 Tab：推荐（skills.tab.recommend，逐字）。
pub const TAB_RECOMMEND: &str = "推荐";
/// 分类 Tab：SkillHub（skills.tab.skillhub，逐字）。
pub const TAB_SKILLHUB: &str = "SkillHub";
/// 分类 Tab：Knot（skills.tab.knot，逐字）。
pub const TAB_KNOT: &str = "Knot";
/// 分类 Tab：套件（skills.tab.plugins，逐字）。
pub const TAB_PLUGINS: &str = "套件";
/// 分类 Tab：企业 Skill（skills.tab.enterprise，逐字）。
pub const TAB_ENTERPRISE: &str = "企业 Skill";

/// 把 `{name}` 占位替换为应用名（模板本体逐字，不做其他改写）。
#[must_use]
pub fn auto_update_description(name: &str) -> String {
    AUTO_UPDATE_DESCRIPTION.replace("{name}", name)
}

/// 把 `{count}` 占位替换为安装次数（模板本体逐字，不做其他改写）。
#[must_use]
pub fn install_count_text(count: u64) -> String {
    CARD_INSTALL_COUNT.replace("{count}", &count.to_string())
}

/// 技能卡片：名称 / 简介 / 安装次数 / 自动触发开关。
#[derive(Clone, Debug, PartialEq)]
pub struct SkillCard {
    pub name: String,
    pub description: String,
    pub install_count: u64,
    pub auto_trigger: bool,
}

impl SkillCard {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        install_count: u64,
        auto_trigger: bool,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            install_count,
            auto_trigger,
        }
    }

    /// 开关行说明文案，随开关状态切换。
    #[must_use]
    pub fn toggle_tip(&self) -> &'static str {
        if self.auto_trigger {
            TOGGLE_TIP_ENABLE
        } else {
            TOGGLE_TIP_DISABLE
        }
    }

    /// 安装次数控件文案（占位已替换）。
    #[must_use]
    pub fn install_count_text(&self) -> String {
        install_count_text(self.install_count)
    }
}

/// 主区状态（三态；无数据源时必须是 [`SkillsMarketState::NotDelivered`]）。
#[derive(Clone, Debug, PartialEq)]
pub enum SkillsMarketState {
    /// 数据未下发：如实显示「未下发」。
    NotDelivered,
    /// 数据源在但为空：显示空态（不是空列表）。
    Empty,
    /// 有条目：渲染分类筛选 + 卡片清单。
    Entries(Vec<SkillCard>),
}

/// 技能主区视图模型：标题 + 分类筛选 + 清单/空态。
#[derive(Clone, Debug, PartialEq)]
pub struct SkillsMarketView {
    pub title: &'static str,
    pub active_tab: usize,
    pub state: SkillsMarketState,
}

impl SkillsMarketView {
    /// 分类 Tab 全集（渲染顺序固定）。
    pub const TABS: [&'static str; 6] = [
        TAB_INSTALLED,
        TAB_RECOMMEND,
        TAB_SKILLHUB,
        TAB_KNOT,
        TAB_PLUGINS,
        TAB_ENTERPRISE,
    ];

    /// 无数据源时的视图（fail-closed）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            title: TITLE,
            active_tab: 0,
            state: SkillsMarketState::NotDelivered,
        }
    }

    /// 数据源在但为空的视图。
    #[must_use]
    pub fn empty() -> Self {
        Self {
            title: TITLE,
            active_tab: 0,
            state: SkillsMarketState::Empty,
        }
    }

    /// 由条目构造视图：空输入归入空态，不产出空清单。
    #[must_use]
    pub fn build(entries: Vec<SkillCard>) -> Self {
        if entries.is_empty() {
            Self::empty()
        } else {
            Self {
                title: TITLE,
                active_tab: 0,
                state: SkillsMarketState::Entries(entries),
            }
        }
    }

    /// 分类 Tab 标签列表。
    #[must_use]
    pub fn tabs() -> Vec<&'static str> {
        Self::TABS.to_vec()
    }

    /// 当前选中 Tab（越界时回落到第一个，不 panic）。
    #[must_use]
    pub fn active_tab_label(&self) -> &'static str {
        Self::TABS
            .get(self.active_tab)
            .copied()
            .unwrap_or(TAB_INSTALLED)
    }

    /// 条目计数（无数据 / 空态均为 0）。
    #[must_use]
    pub fn entry_count(&self) -> usize {
        match &self.state {
            SkillsMarketState::Entries(cards) => cards.len(),
            SkillsMarketState::NotDelivered | SkillsMarketState::Empty => 0,
        }
    }
}

// ---------------- GPUI 层 ----------------

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &SkillsMarketView::not_delivered())
}

fn render(theme: &Theme, view: &SkillsMarketView) -> Div {
    let body = match &view.state {
        SkillsMarketState::NotDelivered => centered_note(theme, NOT_DELIVERED),
        SkillsMarketState::Empty => empty_area(theme),
        SkillsMarketState::Entries(cards) => entries_area(theme, cards),
    };
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(header_row(theme))
        .child(body)
}

/// 标题行。
fn header_row(theme: &Theme) -> Div {
    div()
        .px_4()
        .py_2()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(TITLE)
}

/// 居中单行说明（「未下发」等）。
fn centered_note(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.into())
}

/// 空态：标题 + 说明，居中。
fn empty_area(theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_color(theme.foreground)
                .child(EMPTY_TITLE),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(EMPTY_DESC),
        )
}

/// 条目区（本批先以名称行占位，GPUI 卡片结构下一批落）。
fn entries_area(theme: &Theme, cards: &[SkillCard]) -> Div {
    div()
        .flex_1()
        .flex()
        .flex_col()
        .p_4()
        .gap_2()
        .children(cards.iter().map(|card| {
            div()
                .px_2()
                .py_1()
                .text_sm()
                .text_color(theme.foreground)
                .child(card.name.clone())
        }))
}

#[cfg(test)]
mod tests {
    use super::{
        auto_update_description, install_count_text, main_area, SkillCard, SkillsMarketState,
        SkillsMarketView, ADD_BUTTON, AUTO_UPDATE_DESCRIPTION, AUTO_UPDATE_LABEL, EMPTY_TITLE,
        NOT_DELIVERED, SEARCH_PLACEHOLDER, TAB_ENTERPRISE, TITLE,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn copy_is_verbatim_from_table() {
        assert_eq!(TITLE, "技能");
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(ADD_BUTTON, "添加技能");
        assert_eq!(SEARCH_PLACEHOLDER, "搜索技能");
        assert_eq!(EMPTY_TITLE, "还没有安装任何技能");
        assert_eq!(AUTO_UPDATE_LABEL, "技能自动更新");
        assert_eq!(
            AUTO_UPDATE_DESCRIPTION,
            "开启后将自动更新已安装的技能为最新版本，不会更新你在 {name} 中编辑过的技能"
        );
        assert_eq!(TAB_ENTERPRISE, "企业 Skill");
    }

    #[test]
    fn build_groups_entries_and_counts() {
        let cards = vec![
            SkillCard::new("a", "简介 a", 1, true),
            SkillCard::new("b", "简介 b", 2, false),
            SkillCard::new("c", "简介 c", 3, true),
        ];
        let view = SkillsMarketView::build(cards);
        assert_eq!(view.title, TITLE);
        assert_eq!(view.entry_count(), 3);
        assert!(matches!(view.state, SkillsMarketState::Entries(_)));
        assert!(SkillsMarketView::tabs().len() >= 3);
    }

    #[test]
    fn build_empty_input_yields_empty_state_not_blank_list() {
        let view = SkillsMarketView::build(Vec::new());
        assert_eq!(view.state, SkillsMarketState::Empty);
        assert_eq!(view.entry_count(), 0);
    }

    #[test]
    fn not_delivered_is_the_no_source_state() {
        let view = SkillsMarketView::not_delivered();
        assert_eq!(view.state, SkillsMarketState::NotDelivered);
        assert_eq!(view.entry_count(), 0);
    }

    #[test]
    fn placeholder_templates_substitute_only_the_slot() {
        assert_eq!(install_count_text(128), "安装次数 128");
        assert_eq!(
            auto_update_description("Code"),
            "开启后将自动更新已安装的技能为最新版本，不会更新你在 Code 中编辑过的技能"
        );
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
