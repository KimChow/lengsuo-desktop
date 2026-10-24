//! 账户·菜单（波次 9，卡 `ui-overlay-account-menu`）。
//!
//! 分层约定（同 `panel_views` / `account_checkin`）：纯数据层 + GPUI 层。
//! 硬约束：fail-closed（未拉取 ≠ 空集，默认态 [`State::Loading`]，失败如实「未下发」，
//! ⛔ 不得退化成空列表）；⛔ 不伪造服务端数据；文案逐字取自命名空间
//! `account.menu` 摘录全表（⛔ 不得增删改）。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 命名空间前缀。
pub const NS_PREFIX: &str = "account.menu";
/// 键数（据文案全表命名空间 `account.menu` 已下发摘录；其余键未下发，⛔ 不自编）。
pub const NS_KEY_COUNT: usize = 12;
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

// ===== 纯数据层（无 GPUI 依赖）=====
/// 文案表：命名空间 `account.menu` 已下发摘录共 [`NS_KEY_COUNT`] 键（逐字；⛔ 不得增删改）。
pub const COPY: &[(&str, &str)] = &[
    ("account.menu.account", "账户管理"),
    ("account.menu.adminConsole", "企业智能体"),
    ("account.menu.appearance", "外观"),
    ("account.menu.appearanceNew", "外观上新"),
    ("account.menu.archivedTasks", "已归档任务"),
    ("account.menu.checkForUpdates", "检查更新"),
    ("account.menu.clawSettings", "设置"),
    ("account.menu.credits", "积分余额"),
    ("account.menu.dataManagement", "数据管理"),
    ("account.menu.fuelStation", "Buddy加油站"),
    ("account.menu.fuelStation.claimCredits", "签到领积分"),
    ("account.menu.fuelStation.claimed", "今日已签到"),
];

/// 取命名空间内文案（纯函数；未收录 key ⇒ `None`，fail-closed，不猜不编）。
pub fn copy(key: &str) -> Option<&'static str> {
    COPY.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 菜单条目：只承载表内键，构造时校验并冻结文案。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    key: &'static str,
    text: &'static str,
}

impl MenuItem {
    /// 键必须在文案表内，否则 `None`（fail-closed，不猜不编）。
    pub fn new(key: &'static str) -> Option<Self> {
        let text = copy(key)?;
        Some(Self { key, text })
    }

    pub fn key(&self) -> &'static str {
        self.key
    }

    pub fn text(&self) -> &'static str {
        self.text
    }
}

/// 全量顶层条目：文案表内三段键（`account.menu.<id>`），按表序（供宿主/测试）。
pub fn catalog() -> Vec<MenuItem> {
    COPY.iter()
        .filter(|(key, _)| key.split('.').count() == 3)
        .filter_map(|(key, _)| MenuItem::new(key))
        .collect()
}

/// 浮层状态机（默认 `Loading`：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 未拉取过（诚实默认态，骨架呈现）。
    #[default]
    Loading,
    /// 拉取失败（fail-closed：如实显示「未下发」，⛔ 不落空列表）。
    Failed,
    /// 已拉取且下发 0 条（≠ 未拉取）。
    Empty,
    /// 已拉取且有条目。
    Ready(Vec<MenuItem>),
}

/// 由运行时下发解析初始态（纯函数；`None` ⇒ 未拉取 ⇒ `Loading`，⛔ 不猜）。
pub fn resolve(delivered: Option<Vec<MenuItem>>) -> State {
    match delivered {
        None => State::Loading,
        Some(items) => State::loaded(items),
    }
}

impl State {
    /// 拉取完成：0 条 ⇒ `Empty`，非空 ⇒ `Ready`。
    pub fn loaded(items: Vec<MenuItem>) -> Self {
        if items.is_empty() {
            State::Empty
        } else {
            State::Ready(items)
        }
    }

    /// 正常态条目；其余三态一律 `None`（渲染层据此分支）。
    pub fn items(&self) -> Option<&[MenuItem]> {
        match self {
            State::Ready(items) => Some(items),
            _ => None,
        }
    }

    /// fail-closed 态（拉取失败：必须显式呈现「未下发」，⛔ 不落空列表）。
    pub fn is_fail_closed(&self) -> bool {
        matches!(self, State::Failed)
    }
}

// ===== GPUI 层 =====
/// 浮层呈现入口（无数据源 ⇒ 诚实默认态 `Loading` 骨架，⛔ 不伪造条目）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render(&resolve(None), theme).into_any_element()
}

/// 整棵树渲染（视图是状态的纯函数；颜色一律 `theme.*`，⛔ 不硬编码色值）。
pub fn render(state: &State, theme: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .p_1()
        .text_sm()
        .text_color(theme.foreground)
        .border_1()
        .border_color(theme.border)
        .rounded_lg()
        .children(state_rows(state, theme))
}

/// 分支渲染：`Loading` 骨架 / `Failed`「未下发」/ `Empty` 空框 / `Ready` 条目行。
fn state_rows(state: &State, theme: &Theme) -> Vec<AnyElement> {
    match state {
        State::Loading => (0..4)
            .map(|_| skeleton_row(theme).into_any_element())
            .collect(),
        State::Failed => vec![not_delivered_row(theme).into_any_element()],
        State::Empty => Vec::new(),
        State::Ready(items) => items
            .iter()
            .map(|item| menu_row(item).into_any_element())
            .collect(),
    }
}

/// 骨架行（加载中不下发任何条目文案，只给占位块）。
fn skeleton_row(theme: &Theme) -> impl IntoElement {
    div()
        .h_12()
        .w_full()
        .flex()
        .items_center()
        .px_3()
        .rounded_sm()
        .child(div().h_4().w_24().rounded_sm().bg(theme.border))
}

/// 「未下发」行（fail-closed；⛔ 不是空列表）。
fn not_delivered_row(theme: &Theme) -> impl IntoElement {
    div()
        .h_12()
        .w_full()
        .flex()
        .items_center()
        .px_3()
        .rounded_sm()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(NOT_DELIVERED))
}

/// 条目行（`h_12` = 48px 触达目标；文案逐字取自文案表）。
fn menu_row(item: &MenuItem) -> impl IntoElement {
    div()
        .h_12()
        .w_full()
        .flex()
        .items_center()
        .px_3()
        .rounded_sm()
        .child(SharedString::from(item.text()))
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        catalog, copy, main_area, render, resolve, MenuItem, State, COPY, NOT_DELIVERED,
        NS_KEY_COUNT, NS_PREFIX,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn copy_table_verbatim() {
        // 全表 12 键逐字一致；键数与声明一致；键全部落在命名空间内。
        let expect: &[(&str, &str)] = &[
            ("account.menu.account", "账户管理"),
            ("account.menu.adminConsole", "企业智能体"),
            ("account.menu.appearance", "外观"),
            ("account.menu.appearanceNew", "外观上新"),
            ("account.menu.archivedTasks", "已归档任务"),
            ("account.menu.checkForUpdates", "检查更新"),
            ("account.menu.clawSettings", "设置"),
            ("account.menu.credits", "积分余额"),
            ("account.menu.dataManagement", "数据管理"),
            ("account.menu.fuelStation", "Buddy加油站"),
            ("account.menu.fuelStation.claimCredits", "签到领积分"),
            ("account.menu.fuelStation.claimed", "今日已签到"),
        ];
        assert_eq!(COPY, expect);
        assert_eq!(COPY.len(), NS_KEY_COUNT);
        assert!(COPY.iter().all(|(k, _)| k.starts_with(NS_PREFIX)));
    }

    #[test]
    fn copy_lookup_fail_closed() {
        // 命中抽查（含四段子键）+ 未收录键 ⇒ None（不猜不编）+ 兜底文案逐字。
        assert_eq!(copy("account.menu.account"), Some("账户管理"));
        assert_eq!(copy("account.menu.fuelStation"), Some("Buddy加油站"));
        assert_eq!(
            copy("account.menu.fuelStation.claimCredits"),
            Some("签到领积分")
        );
        assert_eq!(copy("account.menu.fuelStation.claimed"), Some("今日已签到"));
        assert_eq!(copy("account.menu.notInTable"), None);
        assert_eq!(MenuItem::new("account.menu.notInTable"), None);
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn catalog_top_level_only() {
        // 三段键入列（12-2=10 条），四段子键不入列；首尾文案逐字；键全部可回查。
        let items = catalog();
        assert_eq!(items.len(), 10);
        assert_eq!(items[0].text(), "账户管理");
        assert_eq!(items[9].text(), "Buddy加油站");
        assert!(items
            .iter()
            .all(|item| copy(item.key()).is_some() && MenuItem::new(item.key()).is_some()));
    }

    #[test]
    fn default_state_is_loading() {
        // 三态契约：未拉取过 ≠ 空集；默认态必须是 Loading。
        assert_eq!(State::default(), State::Loading);
        assert_eq!(resolve(None), State::Loading);
        assert_eq!(State::Loading.items(), None);
        assert!(!State::Loading.is_fail_closed());
    }

    #[test]
    fn delivered_empty_is_empty_not_loading() {
        // 下发空集 ⇒ Empty（≠ 未拉取 Loading）；loaded 与 resolve 等价。
        assert_eq!(resolve(Some(Vec::new())), State::Empty);
        assert_eq!(State::Empty.items(), None);
        assert_eq!(State::loaded(catalog()), resolve(Some(catalog())));
    }

    #[test]
    fn failed_is_fail_closed() {
        // 拉取失败：fail-closed 标记成立，不暴露条目，且与空态可区分。
        assert!(State::Failed.is_fail_closed());
        assert_eq!(State::Failed.items(), None);
        assert_ne!(State::Failed, State::Empty);
    }

    #[test]
    fn ready_items_exposed() {
        let state = State::loaded(catalog());
        let items = state.items().expect("Ready 必须暴露条目");
        assert_eq!(items.len(), 10);
        assert_eq!(items[0].text(), "账户管理");
        assert_eq!(items[9].text(), "Buddy加油站");
    }

    #[test]
    fn render_branches_do_not_panic() {
        // 四分支整树渲染不 panic；无数据源入口（main_area）⇒ Loading 骨架。
        let theme = Theme::default();
        let _ = render(&State::Loading, &theme);
        let _ = render(&State::Failed, &theme);
        let _ = render(&State::Empty, &theme);
        let _ = render(&State::loaded(catalog()), &theme);
        let _ = main_area(&theme);
    }
}
