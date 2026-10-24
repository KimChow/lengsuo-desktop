//! 闲置能力提示浮层（波次 6，卡 `ui-overlay-idle-capability`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（状态机 + 逐字文案 + 计数模板）+ GPUI 层（[`overlay_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `idleCapability`。
//! 数据源在运行时侧：[`main_area`] 以「未下发」诚实占位，跨线契约就绪后改喂 [`overlay_area`]。
#![cfg(feature = "ui")]
use std::collections::BTreeSet;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ===== 纯数据层（无 GPUI 依赖） =====
// 逐字文案常量：取自界面文案全表命名空间 idleCapability，行尾注释为键名。

/// 数据未下发时的说明（fail-closed 约定；⛔ 不得替换为空清单）。
pub const NOT_DELIVERED: &str = "未下发";
pub const ARIA_LABEL: &str = "闲置能力提示卡片"; // ariaLabel
pub const CLOSE_ARIA_LABEL: &str = "关闭"; // closeAriaLabel
pub const TITLE_TEMPLATE: &str = "{count} 项能力超过 {days} 天未使用，建议关闭"; // title
pub const SUBTITLE: &str =
    "已启用能力的说明会发给模型并占用上下文，关闭可省积分，随时在「管理页」重新开启。"; // subtitle
pub const SUBTITLE_PREFIX: &str = "已启用能力的说明会发给模型并占用上下文，关闭可省积分，随时在 "; // subtitlePrefix
pub const MANAGE_LINK: &str = "管理页"; // manageLink
pub const SUBTITLE_SUFFIX: &str = " 重新开启。"; // subtitleSuffix
pub const SUBTITLE_DETAIL: &str = "禁用后可在相应的设置页面随时重新启用"; // subtitleDetail
pub const SELECT_ALL: &str = "全选"; // selectAll
pub const DESELECT_ALL: &str = "取消选择"; // deselectAll
pub const SELECTED_COUNT_TEMPLATE: &str = "已选中 {count} 个"; // selectedCount
pub const DISABLE_ALL: &str = "一键关闭"; // disableAll
pub const DISABLING: &str = "正在关闭…"; // disabling
pub const DISABLE: &str = "关闭"; // disable
pub const VIEW_DETAIL: &str = "查看详情"; // viewDetail
pub const COLLAPSE_DETAIL: &str = "收起详情"; // collapseDetail
pub const NEVER_REMIND: &str = "不再提醒"; // neverRemind
pub const COLUMN_TYPE_AND_NAME: &str = "类型和名称"; // columnTypeAndName
pub const COLUMN_LAST_USED: &str = "上次使用时间"; // columnLastUsed
pub const NEVER_USED: &str = "从未使用"; // neverUsed
pub const LAST_USED_TEMPLATE: &str = "上次使用：{date}"; // lastUsed
pub const CLOSED_TOAST_PREFIX: &str = "已关闭 {count} 项能力，可随时在 "; // closedToastPrefix
pub const CLOSED_TOAST_SUFFIX: &str = " 重新开启"; // closedToastSuffix
pub const DISABLED_TOAST: &str = "已关闭 {count} 项能力，可随时在 管理页 重新开启"; // disabledToast
pub const TYPE_CONNECTOR: &str = "Connect"; // typeLabel.connector
pub const TYPE_MCP: &str = "MCP"; // typeLabel.mcp
pub const TYPE_SKILL: &str = "Skill"; // typeLabel.skill
pub const TYPE_SUBAGENT: &str = "Agent"; // typeLabel.subagent
pub const TYPE_MIXED: &str = "能力"; // typeLabel.mixed
pub const RELATIVE_TODAY: &str = "今天"; // relative.today
pub const RELATIVE_DAYS_AGO: &str = "{n} 天前"; // relative.daysAgo
pub const RELATIVE_MONTHS_AGO: &str = "{n} 个月前"; // relative.monthsAgo
pub const RELATIVE_YEARS_AGO: &str = "{n} 年前"; // relative.yearsAgo

/// 能力类型；徽标文案逐字取自 `typeLabel.*`。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CapabilityKind {
    Connector,
    Mcp,
    Skill,
    Subagent,
    Mixed,
}

impl CapabilityKind {
    /// 徽标文案。
    #[must_use]
    pub fn badge(self) -> &'static str {
        match self {
            Self::Connector => TYPE_CONNECTOR,
            Self::Mcp => TYPE_MCP,
            Self::Skill => TYPE_SKILL,
            Self::Subagent => TYPE_SUBAGENT,
            Self::Mixed => TYPE_MIXED,
        }
    }
}

/// 上次使用情况（`neverUsed` / `lastUsed` + `relative.*`）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LastUsed {
    Never,     // 从未使用
    Days(u32), // 已闲置天数
}

impl LastUsed {
    /// 清单「上次使用时间」列文案。
    #[must_use]
    pub fn column_text(self) -> String {
        match self {
            Self::Never => NEVER_USED.to_owned(),
            Self::Days(days) => fill(LAST_USED_TEMPLATE, "{date}", &relative_text(days)),
        }
    }
}

/// 单条闲置能力（由数据源下发；本模块 ⛔ 不伪造）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IdleItem {
    pub kind: CapabilityKind, // 类型（徽标）
    pub name: String,         // 名称
    pub last_used: LastUsed,  // 上次使用
}

/// 浮层阶段。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// 数据未下发：fail-closed，如实展示「未下发」，⛔ 不用空清单冒充。
    NotDelivered,
    /// 已下发：按清单渲染（零条目 ⇒ 空态，不是空清单）。
    Delivered,
}

/// 闲置能力提示卡片状态机（纯数据层）。
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IdleCapabilityCard {
    pub phase: Phase,                // 阶段
    pub threshold_days: u32,         // 超期阈值（天），标题模板的 {days}
    pub items: Vec<IdleItem>,        // 闲置条目
    selected: BTreeSet<usize>,       // 选中索引集
    pub detail_expanded: bool,       // 详情展开位（查看详情 / 收起详情）
    pub disabling: bool,             // 一键关闭进行中（按钮文案切换「正在关闭…」）
    pub closed_count: Option<usize>, // 已关闭条数（Some ⇒ 渲染已关闭 toast）
}

impl IdleCapabilityCard {
    /// 未下发（[`main_area`] 的诚实默认）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            phase: Phase::NotDelivered,
            threshold_days: 0,
            items: Vec::new(),
            selected: BTreeSet::new(),
            detail_expanded: false,
            disabling: false,
            closed_count: None,
        }
    }

    /// 已下发：携带阈值与条目进入清单渲染。
    #[must_use]
    pub fn delivered(threshold_days: u32, items: Vec<IdleItem>) -> Self {
        Self {
            phase: Phase::Delivered,
            threshold_days,
            items,
            ..Self::not_delivered()
        }
    }

    /// 是否空态（已下发但零条目 ⇒ 居中说明，不是空清单）。
    #[must_use]
    pub fn is_empty_state(&self) -> bool {
        self.phase == Phase::Delivered && self.items.is_empty()
    }

    /// 标题文案；未下发时无标题（不伪造计数）。
    #[must_use]
    pub fn title_text(&self) -> Option<String> {
        (self.phase == Phase::Delivered).then(|| {
            fill_count(
                &TITLE_TEMPLATE.replace("{days}", &self.threshold_days.to_string()),
                self.items.len(),
            )
        })
    }

    /// 全选。
    pub fn select_all(&mut self) {
        self.selected = (0..self.items.len()).collect();
    }

    /// 取消选择。
    pub fn deselect_all(&mut self) {
        self.selected.clear();
    }

    /// 切换单条选中。
    pub fn toggle_select(&mut self, index: usize) {
        if index < self.items.len() && !self.selected.remove(&index) {
            self.selected.insert(index);
        }
    }

    /// 选中条数。
    #[must_use]
    pub fn selected_count(&self) -> usize {
        self.selected.len()
    }

    /// 选中计数文案（selectedCount 模板）。
    #[must_use]
    pub fn selected_count_text(&self) -> String {
        fill_count(SELECTED_COUNT_TEMPLATE, self.selected_count())
    }

    /// 一键关闭按钮文案（disabling ⇒ 正在关闭…）。
    #[must_use]
    pub fn disable_all_label(&self) -> &'static str {
        if self.disabling {
            DISABLING
        } else {
            DISABLE_ALL
        }
    }

    /// 进入「正在关闭…」。
    pub fn begin_disable(&mut self) {
        self.disabling = self.phase == Phase::Delivered;
    }

    /// 关闭当前选中项：移除条目、落已关闭 toast；无可关项 ⇒ `None`。
    pub fn disable_selected(&mut self) -> Option<String> {
        if self.phase != Phase::Delivered || self.selected.is_empty() {
            return None;
        }
        let count = self.selected.len();
        let kept = self
            .items
            .iter()
            .enumerate()
            .filter(|(idx, _)| !self.selected.contains(idx))
            .map(|(_, item)| item.clone())
            .collect();
        self.items = kept;
        self.selected.clear();
        self.disabling = false;
        self.closed_count = Some(count);
        Some(Self::closed_toast_text(count))
    }

    /// 已关闭 toast 文案（closedToastPrefix + 管理页 + closedToastSuffix）。
    #[must_use]
    pub fn closed_toast_text(count: usize) -> String {
        format!(
            "{}{MANAGE_LINK}{}",
            fill_count(CLOSED_TOAST_PREFIX, count),
            CLOSED_TOAST_SUFFIX
        )
    }
}

/// 模板占位符填充。
fn fill(template: &str, placeholder: &str, value: &str) -> String {
    template.replace(placeholder, value)
}

/// `{count}` 计数模板填充。
fn fill_count(template: &str, count: usize) -> String {
    fill(template, "{count}", &count.to_string())
}

/// 闲置天数的相对时间文案（relative.* 模板）。
#[must_use]
pub fn relative_text(days: u32) -> String {
    match days {
        0 => RELATIVE_TODAY.to_owned(),
        n if n < 30 => fill(RELATIVE_DAYS_AGO, "{n}", &n.to_string()),
        n if n < 365 => fill(RELATIVE_MONTHS_AGO, "{n}", &(n / 30).to_string()),
        n => fill(RELATIVE_YEARS_AGO, "{n}", &(n / 365).to_string()),
    }
}

// ===== GPUI 层 =====

/// sm 级弱化文本单元。
fn muted_text(theme: &Theme, text: &str) -> Div {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}
/// sm 级强调文本单元。
fn accent_text(theme: &Theme, text: &str) -> Div {
    div()
        .text_sm()
        .text_color(theme.accent)
        .child(text.to_owned())
}
/// sm 级前景文本单元。
fn plain_text(theme: &Theme, text: &str) -> Div {
    div()
        .text_sm()
        .text_color(theme.foreground)
        .child(text.to_owned())
}
/// xs 级弱化文本单元。
fn muted_text_xs(theme: &Theme, text: &str) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}
/// xs 级强调文本单元。
fn accent_text_xs(theme: &Theme, text: &str) -> Div {
    div()
        .text_xs()
        .text_color(theme.accent)
        .child(text.to_owned())
}

/// 浮层渲染入口：无数据源时的诚实占位（「未下发」，fail-closed）。
///
/// 波次 6：浮层不占主区，返回整层容器由调用方叠加。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    overlay_area(theme, &IdleCapabilityCard::not_delivered())
}

/// 按纯数据层状态渲染整卡：标题行 / 副标题 / 操作条 / 清单 / 空态 / 未下发分支。
#[must_use]
pub fn overlay_area(theme: &Theme, card: &IdleCapabilityCard) -> Div {
    let area = div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col();
    match card.phase {
        Phase::NotDelivered => area
            .child(header_row(theme, ARIA_LABEL))
            .child(centered(theme, NOT_DELIVERED)),
        Phase::Delivered => delivered_area(theme, card, area),
    }
}

/// 已下发分支：标题 + 副标题 +（空态 / 操作条 + 清单 + 脚注）+ 已关闭 toast。
fn delivered_area(theme: &Theme, card: &IdleCapabilityCard, mut area: Div) -> Div {
    let Some(title) = card.title_text() else {
        return area;
    };
    area = area
        .child(header_row(theme, &title))
        .child(subtitle_row(theme));
    if card.is_empty_state() {
        return area.child(centered(theme, SUBTITLE));
    }
    area = area
        .child(action_bar(theme, card))
        .child(list_header(theme))
        .child(list_rows(theme, card))
        .child(footer_rows(theme, card));
    if let Some(count) = card.closed_count {
        area = area.child(toast_row(theme, count));
    }
    area
}

/// 标题行：卡片名/动态标题 + 关闭（closeAriaLabel）。
fn header_row(theme: &Theme, title: &str) -> Div {
    div()
        .flex()
        .items_center()
        .px_4()
        .py_2()
        .child(plain_text(theme, title).font_bold())
        .child(div().flex_1())
        .child(muted_text(theme, CLOSE_ARIA_LABEL))
}

/// 副标题行：subtitlePrefix + 管理页（强调）+ subtitleSuffix。
fn subtitle_row(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .px_4()
        .child(muted_text_xs(theme, SUBTITLE_PREFIX))
        .child(accent_text_xs(theme, MANAGE_LINK))
        .child(muted_text_xs(theme, SUBTITLE_SUFFIX))
}

/// 居中说明（未下发 / 空态）。
fn centered(theme: &Theme, text: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .child(muted_text(theme, text))
}

/// 操作条：全选 / 取消选择 / 已选中计数 / 一键关闭（正在关闭…）。
fn action_bar(theme: &Theme, card: &IdleCapabilityCard) -> Div {
    let label = card.disable_all_label();
    let disable = if card.disabling {
        muted_text(theme, label)
    } else {
        accent_text(theme, label)
    };
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(accent_text(theme, SELECT_ALL))
        .child(accent_text(theme, DESELECT_ALL))
        .child(muted_text(theme, &card.selected_count_text()))
        .child(div().flex_1())
        .child(disable)
}

/// 清单表头：类型和名称 | 上次使用时间。
fn list_header(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .px_4()
        .py_1()
        .child(muted_text_xs(theme, COLUMN_TYPE_AND_NAME).flex_1())
        .child(muted_text_xs(theme, COLUMN_LAST_USED))
}

/// 清单行：徽标 + 名称 | 上次使用 | 单条关闭。
fn list_rows(theme: &Theme, card: &IdleCapabilityCard) -> Div {
    let mut rows = div().flex().flex_col();
    for item in &card.items {
        let badge = accent_text_xs(theme, item.kind.badge())
            .border_1()
            .border_color(theme.border)
            .rounded_sm()
            .px_2();
        rows = rows.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .px_4()
                .py_1()
                .child(badge)
                .child(plain_text(theme, &item.name).flex_1())
                .child(muted_text(theme, &item.last_used.column_text()))
                .child(muted_text(theme, DISABLE)),
        );
    }
    rows
}

/// 脚注：查看详情/收起详情 + 不再提醒；展开时附详情说明（subtitleDetail）。
fn footer_rows(theme: &Theme, card: &IdleCapabilityCard) -> Div {
    let detail_label = if card.detail_expanded {
        COLLAPSE_DETAIL
    } else {
        VIEW_DETAIL
    };
    let mut column = div().flex().flex_col().child(
        div()
            .flex()
            .items_center()
            .px_4()
            .py_2()
            .child(accent_text(theme, detail_label))
            .child(div().flex_1())
            .child(muted_text(theme, NEVER_REMIND)),
    );
    if card.detail_expanded {
        column = column.child(muted_text_xs(theme, SUBTITLE_DETAIL).px_4().pb_2());
    }
    column
}

/// 已关闭 toast：closedToastPrefix + 管理页（强调）+ closedToastSuffix。
fn toast_row(theme: &Theme, count: usize) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_4()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(plain_text(theme, &fill_count(CLOSED_TOAST_PREFIX, count)))
        .child(accent_text(theme, MANAGE_LINK))
        .child(plain_text(theme, CLOSED_TOAST_SUFFIX))
}
#[cfg(test)]
mod tests {
    use super::{
        main_area, CapabilityKind, IdleCapabilityCard, IdleItem, LastUsed, ARIA_LABEL,
        CLOSED_TOAST_PREFIX, CLOSED_TOAST_SUFFIX, CLOSE_ARIA_LABEL, DISABLED_TOAST, MANAGE_LINK,
        NOT_DELIVERED, SUBTITLE, SUBTITLE_PREFIX, SUBTITLE_SUFFIX, TITLE_TEMPLATE,
    };
    use gpui_kit::component::Theme;

    fn sample_items() -> Vec<IdleItem> {
        vec![
            IdleItem {
                kind: CapabilityKind::Mcp,
                name: "alpha".into(),
                last_used: LastUsed::Never,
            },
            IdleItem {
                kind: CapabilityKind::Skill,
                name: "beta".into(),
                last_used: LastUsed::Days(45),
            },
            IdleItem {
                kind: CapabilityKind::Subagent,
                name: "gamma".into(),
                last_used: LastUsed::Days(400),
            },
        ]
    }

    #[test]
    fn copy_is_verbatim_from_idle_capability_namespace() {
        let verbatim = [
            (ARIA_LABEL, "闲置能力提示卡片"),
            (CLOSE_ARIA_LABEL, "关闭"),
            (
                TITLE_TEMPLATE,
                "{count} 项能力超过 {days} 天未使用，建议关闭",
            ),
            (
                SUBTITLE,
                "已启用能力的说明会发给模型并占用上下文，关闭可省积分，随时在「管理页」重新开启。",
            ),
            (
                SUBTITLE_PREFIX,
                "已启用能力的说明会发给模型并占用上下文，关闭可省积分，随时在 ",
            ),
            (SUBTITLE_SUFFIX, " 重新开启。"),
            (MANAGE_LINK, "管理页"),
            (CLOSED_TOAST_PREFIX, "已关闭 {count} 项能力，可随时在 "),
            (CLOSED_TOAST_SUFFIX, " 重新开启"),
            (NOT_DELIVERED, "未下发"),
        ];
        for (actual, expected) in verbatim {
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn closed_toast_composes_to_disabled_toast() {
        for count in [0, 1, 3] {
            let composed = IdleCapabilityCard::closed_toast_text(count);
            let expected = DISABLED_TOAST.replace("{count}", &count.to_string());
            assert_eq!(composed, expected);
        }
    }

    #[test]
    fn title_and_count_templates_fill() {
        let mut card = IdleCapabilityCard::delivered(30, sample_items());
        assert_eq!(
            card.title_text(),
            Some("3 项能力超过 30 天未使用，建议关闭".to_owned())
        );
        assert_eq!(card.selected_count_text(), "已选中 0 个");
        card.select_all();
        assert_eq!(card.selected_count_text(), "已选中 3 个");
    }

    #[test]
    fn selection_state_machine_roundtrip() {
        let mut card = IdleCapabilityCard::delivered(30, sample_items());
        card.select_all();
        assert_eq!(card.selected_count(), 3);
        card.toggle_select(1);
        assert_eq!(card.selected_count(), 2);
        card.toggle_select(1);
        assert_eq!(card.selected_count(), 3);
        card.toggle_select(9);
        assert_eq!(card.selected_count(), 3);
        card.deselect_all();
        assert_eq!(card.selected_count(), 0);
    }

    #[test]
    fn disable_selected_removes_items_and_yields_toast() {
        let mut card = IdleCapabilityCard::delivered(30, sample_items());
        assert_eq!(card.disable_selected(), None);
        card.toggle_select(0);
        card.toggle_select(2);
        card.begin_disable();
        assert_eq!(card.disable_all_label(), "正在关闭…");
        let toast = card.disable_selected().expect("two selected to disable");
        assert_eq!(toast, "已关闭 2 项能力，可随时在 管理页 重新开启");
        assert_eq!(card.items.len(), 1);
        assert_eq!(card.closed_count, Some(2));
        assert_eq!(card.disable_all_label(), "一键关闭");
        assert_eq!(card.items[0].name, "beta");
    }

    #[test]
    fn empty_input_is_empty_state_not_empty_list() {
        let empty = IdleCapabilityCard::delivered(30, Vec::new());
        assert!(empty.is_empty_state());
        assert_eq!(
            empty.title_text(),
            Some("0 项能力超过 30 天未使用，建议关闭".to_owned())
        );
        let undelivered = IdleCapabilityCard::not_delivered();
        assert!(!undelivered.is_empty_state());
        assert_eq!(undelivered.title_text(), None);
    }

    #[test]
    fn relative_and_last_used_texts() {
        use super::relative_text;
        assert_eq!(relative_text(0), "今天");
        assert_eq!(relative_text(3), "3 天前");
        assert_eq!(relative_text(45), "1 个月前");
        assert_eq!(relative_text(400), "1 年前");
        assert_eq!(LastUsed::Never.column_text(), "从未使用");
        assert_eq!(LastUsed::Days(90).column_text(), "上次使用：3 个月前");
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

    #[test]
    fn overlay_area_renders_all_branches_without_panic() {
        let theme = Theme::default();
        let mut card = IdleCapabilityCard::delivered(30, sample_items());
        let _ = super::overlay_area(&theme, &card);
        card.toggle_select(0);
        card.disable_selected();
        let _ = super::overlay_area(&theme, &card);
        card.detail_expanded = true;
        let _ = super::overlay_area(&theme, &card);
        card.disabling = true;
        let _ = super::overlay_area(&theme, &card);
        let empty = IdleCapabilityCard::delivered(30, Vec::new());
        let _ = super::overlay_area(&theme, &empty);
        let undelivered = IdleCapabilityCard::not_delivered();
        let _ = super::overlay_area(&theme, &undelivered);
    }
}
