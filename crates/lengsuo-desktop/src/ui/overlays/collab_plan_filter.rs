//! 计划筛选（波次 8，卡 `ui-collab-plan-filter`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（无 GPUI 依赖）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.plan.filter`
//! （键值全表见 [`TEXTS`]；带 `{占位符}` 的键由纯函数按占位符格式化，不改字）。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "计划筛选";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀（文案键值全表见 [`TEXTS`]）。
pub const NS_PREFIX: &str = "collab.plan.filter";

/// 命名空间文案全表（key → 中文，逐字，共 58 键）。
pub const TEXTS: &[(&str, &str)] = &[
    ("collab.plan.filter.addCondition", "添加筛选条件"),
    ("collab.plan.filter.changeField", "切换字段"),
    ("collab.plan.filter.changeOperator", "切换运算符"),
    ("collab.plan.filter.collapse", "收起"),
    ("collab.plan.filter.conj.all", "全部条件"),
    ("collab.plan.filter.conj.any", "任一条件"),
    ("collab.plan.filter.conj.aria", "条件连接方式"),
    ("collab.plan.filter.conj.prefix", "满足以下"),
    ("collab.plan.filter.dim.owner", "处理人"),
    ("collab.plan.filter.dim.scope", "范围"),
    ("collab.plan.filter.dim.source", "来源"),
    ("collab.plan.filter.dim.status", "状态"),
    ("collab.plan.filter.fallback.unknown", "未知"),
    ("collab.plan.filter.member.empty", "无相关结果"),
    ("collab.plan.filter.member.placeholder", "搜索成员"),
    ("collab.plan.filter.op.contains", "包含"),
    ("collab.plan.filter.op.is", "是"),
    ("collab.plan.filter.op.is_empty", "为空"),
    ("collab.plan.filter.op.is_not", "不是"),
    ("collab.plan.filter.op.is_not_empty", "不为空"),
    ("collab.plan.filter.op.not_contains", "不包含"),
    ("collab.plan.filter.owner.all", "全部处理人"),
    ("collab.plan.filter.pin", "固定到工具栏"),
    ("collab.plan.filter.popover.title", "筛选"),
    ("collab.plan.filter.removeCondition", "删除条件"),
    ("collab.plan.filter.scope.all", "全部归属"),
    ("collab.plan.filter.scope.my_assigned", "指派给我"),
    ("collab.plan.filter.scope.my_created", "我创建的"),
    ("collab.plan.filter.search.aria", "搜索"),
    ("collab.plan.filter.search.empty", "暂无可选项"),
    ("collab.plan.filter.search.placeholder", "搜索..."),
    ("collab.plan.filter.source.all", "全部来源"),
    ("collab.plan.filter.source.cnb", "CNB 导入"),
    (
        "collab.plan.filter.source.cnbEnterprise",
        "CNB（内网版）导入",
    ),
    ("collab.plan.filter.source.fallback", "全部来源"),
    ("collab.plan.filter.source.github", "GitHub 导入"),
    ("collab.plan.filter.source.handoff", "转交"),
    ("collab.plan.filter.source.manual", "手动创建"),
    ("collab.plan.filter.source.tapd", "TAPD 导入"),
    (
        "collab.plan.filter.source.tapdEnterprise",
        "TAPD（内网版）导入",
    ),
    ("collab.plan.filter.status.all", "全部归属"),
    ("collab.plan.filter.status.allDim", "全部状态"),
    ("collab.plan.filter.status.fallback", "全部状态"),
    ("collab.plan.filter.status.my_assigned", "指派给我"),
    ("collab.plan.filter.status.my_created", "我创建的"),
    ("collab.plan.filter.summary.count", "{first} 等 {more}+"),
    (
        "collab.plan.filter.temp.appliedToEveryone",
        "已对所有人生效",
    ),
    ("collab.plan.filter.temp.applyFailed", "操作失败，请重试"),
    ("collab.plan.filter.temp.applyToEveryone", "对所有人生效"),
    ("collab.plan.filter.temp.more", "更多"),
    ("collab.plan.filter.temp.onlyYouSee", "当前修改仅你可见"),
    ("collab.plan.filter.temp.reset", "重置"),
    ("collab.plan.filter.title", "筛选"),
    ("collab.plan.filter.todoCount", "{count} 个待办"),
    ("collab.plan.filter.unpin", "取消固定"),
    ("collab.plan.filter.unset", "未设置"),
    ("collab.plan.filter.value.placeholder", "请选择"),
    ("collab.plan.filter.value.textPlaceholder", "请输入"),
];

/// 按键取文案（缺键 = 程序缺陷，fail-fast）。
pub fn zh(key: &str) -> &'static str {
    TEXTS
        .iter()
        .find(|(k, _)| *k == key)
        .expect("collab.plan.filter 文案键缺失")
        .1
}

// ===== 纯数据层（无 GPUI 依赖）=====

/// 条件字段维度（`dim.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    Owner,
    Scope,
    Source,
    Status,
}

impl Dimension {
    /// 全部维度（渲染顺序）。
    pub const ALL: [Dimension; 4] = [Self::Owner, Self::Scope, Self::Source, Self::Status];

    /// 维度名（逐字：`dim.*`）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Owner => zh("collab.plan.filter.dim.owner"),
            Self::Scope => zh("collab.plan.filter.dim.scope"),
            Self::Source => zh("collab.plan.filter.dim.source"),
            Self::Status => zh("collab.plan.filter.dim.status"),
        }
    }

    /// 「全部X」兜底选项（逐字：`owner.all` / `scope.all` / `source.all` / `status.allDim`）。
    pub fn all_label(self) -> &'static str {
        match self {
            Self::Owner => zh("collab.plan.filter.owner.all"),
            Self::Scope => zh("collab.plan.filter.scope.all"),
            Self::Source => zh("collab.plan.filter.source.all"),
            Self::Status => zh("collab.plan.filter.status.allDim"),
        }
    }

    /// 候选值（逐字；处理人成员名单未下发 ⇒ 仅有兜底项，⛔ 不伪造成员）。
    pub fn value_options(self) -> Vec<&'static str> {
        match self {
            Self::Owner => vec![zh("collab.plan.filter.owner.all")],
            Self::Scope => vec![
                zh("collab.plan.filter.scope.all"),
                zh("collab.plan.filter.scope.my_assigned"),
                zh("collab.plan.filter.scope.my_created"),
            ],
            Self::Source => vec![
                zh("collab.plan.filter.source.all"),
                zh("collab.plan.filter.source.cnb"),
                zh("collab.plan.filter.source.cnbEnterprise"),
                zh("collab.plan.filter.source.github"),
                zh("collab.plan.filter.source.handoff"),
                zh("collab.plan.filter.source.manual"),
                zh("collab.plan.filter.source.tapd"),
                zh("collab.plan.filter.source.tapdEnterprise"),
            ],
            Self::Status => vec![
                zh("collab.plan.filter.status.allDim"),
                zh("collab.plan.filter.status.my_assigned"),
                zh("collab.plan.filter.status.my_created"),
            ],
        }
    }
}

/// 运算符（`op.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    Is,
    IsNot,
    Contains,
    NotContains,
    IsEmpty,
    IsNotEmpty,
}

impl Operator {
    /// 运算符名（逐字：`op.*`）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Is => zh("collab.plan.filter.op.is"),
            Self::IsNot => zh("collab.plan.filter.op.is_not"),
            Self::Contains => zh("collab.plan.filter.op.contains"),
            Self::NotContains => zh("collab.plan.filter.op.not_contains"),
            Self::IsEmpty => zh("collab.plan.filter.op.is_empty"),
            Self::IsNotEmpty => zh("collab.plan.filter.op.is_not_empty"),
        }
    }

    /// 是否需要值（`is_empty` / `is_not_empty` 无值槽）。
    pub fn needs_value(self) -> bool {
        !matches!(self, Self::IsEmpty | Self::IsNotEmpty)
    }

    /// 值槽占位（逐字：文本运算 → `请输入`；集合运算 → `请选择`；无值槽 → `None`）。
    pub fn value_placeholder(self) -> Option<&'static str> {
        match self {
            Self::IsEmpty | Self::IsNotEmpty => None,
            Self::Contains | Self::NotContains => {
                Some(zh("collab.plan.filter.value.textPlaceholder"))
            }
            _ => Some(zh("collab.plan.filter.value.placeholder")),
        }
    }
}

/// 条件连接方式（`conj.*`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Conjunction {
    #[default]
    All,
    Any,
}

impl Conjunction {
    /// 连接方式名（逐字：`conj.all` / `conj.any`）。
    pub fn label(self) -> &'static str {
        match self {
            Self::All => zh("collab.plan.filter.conj.all"),
            Self::Any => zh("collab.plan.filter.conj.any"),
        }
    }

    /// 前缀（逐字：`conj.prefix`）。
    pub fn prefix(self) -> &'static str {
        zh("collab.plan.filter.conj.prefix")
    }

    /// 无障碍描述（逐字：`conj.aria`）。
    pub fn aria(self) -> &'static str {
        zh("collab.plan.filter.conj.aria")
    }
}

/// 单条筛选条件（值未选 ⇒ 显示 `unset`「未设置」，⛔ 不伪造）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Condition {
    pub field: Dimension,
    pub op: Operator,
    pub value: Option<&'static str>,
}

/// 已下发的筛选面板数据（仅由调用方/测试构造；待办数为服务端事实 ⇒ `Option`）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilterModel {
    pub conjunction: Conjunction,
    pub conditions: Vec<Condition>,
    pub todo_count: Option<usize>,
}

impl FilterModel {
    /// 条件集为空（= 空态，⛔ ≠ 未下发）。
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    /// 值未选的条件数。
    pub fn unset_count(&self) -> usize {
        self.conditions.iter().filter(|c| c.value.is_none()).count()
    }
}

/// 面板待办数文案（`todoCount` 模板「{count} 个待办」，逐字替换占位符）。
pub fn todo_count_text(count: usize) -> String {
    zh("collab.plan.filter.todoCount").replace("{count}", &count.to_string())
}

/// 多条件摘要（`summary.count` 模板「{first} 等 {more}+」，逐字替换占位符）。
pub fn summary_count_text(first: &str, more: usize) -> String {
    zh("collab.plan.filter.summary.count")
        .replace("{first}", first)
        .replace("{more}", &more.to_string())
}

/// 浮层状态机（默认 `NotDelivered`：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq)]
pub enum State {
    /// 数据已下发（空条件集 = 空态，≠ 未下发）。
    Ready(FilterModel),
    /// 拉取失败（fail-closed，必须显示失败说明）。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 由运行时输入解析初始态（纯函数）。
pub fn resolve(delivered: Option<bool>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(false) => State::LoadFailed,
        Some(true) => State::Ready(FilterModel::default()),
    }
}

/// 各状态主体内容节点数（与 GPUI 层分支的直属内容节点一一对应；空条件集
/// 计 1 个空态节点而非 0 个，⛔ 不是空列表）。
pub fn content_node_count(state: &State) -> usize {
    match state {
        State::NotDelivered | State::LoadFailed => 1,
        State::Ready(m) => 4 + m.conditions.len().max(1),
    }
}

/// 是否呈空态（已下发且无条件 ⇒ 显式空态节点）。
pub fn shows_empty_hint(state: &State) -> bool {
    matches!(state, State::Ready(m) if m.conditions.is_empty())
}

// ===== GPUI 层 =====
/// 主区/浮层呈现入口（fail-closed：无数据源 ⇒ 未下发；签名不变）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render_state(&resolve(None), theme)
}

/// 按状态渲染：标题行 + 状态分支主体；颜色一律取 `theme.*`（⛔ 不硬编码色值）。
fn render_state(state: &State, theme: &Theme) -> AnyElement {
    let header = div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().text_sm().text_color(theme.foreground).child(TITLE))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.filter.collapse")),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.filter.pin")),
        );
    let body = match state {
        State::NotDelivered => div()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED)
            .into_any_element(),
        State::LoadFailed => div()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(zh("collab.plan.filter.temp.applyFailed"))
            .into_any_element(),
        State::Ready(m) => ready_node(m, theme),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(header)
        .child(body)
        .into_any_element()
}

/// 已下发主体：子标题行 + 连接方式行 + 条件区（空 ⇒ 空态节点）+ 添加行 + 临时态脚注。
/// 直属内容节点数与 [`content_node_count`] 的 `Ready` 分支一一对应（4 + max(n,1)）。
fn ready_node(m: &FilterModel, theme: &Theme) -> AnyElement {
    let mut head = div().flex().items_center().gap_2().child(
        div()
            .text_sm()
            .text_color(theme.foreground)
            .child(zh("collab.plan.filter.popover.title")),
    );
    if m.conditions.len() > 1 {
        let summary = summary_count_text(first_field_label(m), m.conditions.len() - 1);
        head = head.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(summary)),
        );
    }
    if let Some(n) = m.todo_count {
        head = head.child(
            div()
                .text_xs()
                .text_color(theme.accent)
                .child(SharedString::from(todo_count_text(n))),
        );
    }
    let mut col = div().flex().flex_col().gap_2().child(head).child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(m.conjunction.prefix()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(m.conjunction.label()),
            ),
    );
    if m.conditions.is_empty() {
        col = col.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.filter.search.empty")),
        );
    } else {
        for c in &m.conditions {
            col = col.child(condition_row(c, theme));
        }
    }
    col.child(
        div()
            .text_sm()
            .text_color(theme.accent)
            .child(zh("collab.plan.filter.addCondition")),
    )
    .child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(zh("collab.plan.filter.temp.onlyYouSee")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(zh("collab.plan.filter.temp.more")),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme.foreground)
                    .child(zh("collab.plan.filter.temp.reset")),
            ),
    )
    .into_any_element()
}

/// 单条条件行：字段 + 运算符 + 值（未选 ⇒ 「未设置」；无值槽运算不设值槽）+ 删除条件。
fn condition_row(c: &Condition, theme: &Theme) -> AnyElement {
    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(c.field.label()),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(c.op.label()),
        );
    if c.op.needs_value() {
        let value = c.value.unwrap_or_else(|| zh("collab.plan.filter.unset"));
        row = row.child(div().text_sm().text_color(theme.foreground).child(value));
    }
    row.child(
        div()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(zh("collab.plan.filter.removeCondition")),
    )
    .into_any_element()
}

/// 摘要 chip 的首条件字段名（无条件 ⇒ 「未设置」兜底，逐字）。
fn first_field_label(m: &FilterModel) -> &'static str {
    m.conditions
        .first()
        .map(|c| c.field.label())
        .unwrap_or_else(|| zh("collab.plan.filter.unset"))
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        content_node_count, main_area, render_state, resolve, shows_empty_hint, summary_count_text,
        todo_count_text, zh, Condition, Conjunction, Dimension, FilterModel, Operator, State,
        NOT_DELIVERED, NS_PREFIX, TEXTS, TITLE,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "计划筛选");
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        assert!(!shows_empty_hint(&resolve(None)));
        assert_eq!(content_node_count(&resolve(None)), 1);
    }

    #[test]
    fn texts_cover_namespace() {
        let n = TEXTS.len();
        assert!((40..=64).contains(&n), "键数 {n} 应 ≥40");
        for (k, v) in TEXTS {
            assert!(k.starts_with(NS_PREFIX), "越界键 {k}");
            assert!(!v.is_empty(), "空文案 {k}");
        }
    }

    /// 文案逐字一致（对 ≥3 个 key 断言；含全角括号/省略号/占位符原样保留）。
    #[test]
    fn texts_verbatim_spot_checks() {
        assert_eq!(zh("collab.plan.filter.addCondition"), "添加筛选条件");
        assert_eq!(zh("collab.plan.filter.conj.any"), "任一条件");
        assert_eq!(zh("collab.plan.filter.op.not_contains"), "不包含");
        assert_eq!(
            zh("collab.plan.filter.source.cnbEnterprise"),
            "CNB（内网版）导入"
        );
        assert_eq!(zh("collab.plan.filter.search.placeholder"), "搜索...");
        assert_eq!(zh("collab.plan.filter.todoCount"), "{count} 个待办");
    }

    #[test]
    fn labels_verbatim() {
        for (d, name) in Dimension::ALL
            .iter()
            .zip(["处理人", "范围", "来源", "状态"])
        {
            assert_eq!(d.label(), name);
        }
        assert_eq!(Dimension::Owner.all_label(), "全部处理人");
        assert_eq!(Dimension::Scope.all_label(), "全部归属");
        assert_eq!(Dimension::Source.all_label(), "全部来源");
        assert_eq!(Dimension::Status.all_label(), "全部状态");
        assert_eq!(Operator::Is.label(), "是");
        assert_eq!(Operator::IsNotEmpty.label(), "不为空");
        assert_eq!(Conjunction::All.label(), "全部条件");
        assert_eq!(Conjunction::Any.label(), "任一条件");
        assert_eq!(Conjunction::Any.prefix(), "满足以下");
        assert_eq!(Conjunction::All.aria(), "条件连接方式");
    }

    #[test]
    fn templates_and_value_slots() {
        assert_eq!(todo_count_text(3), "3 个待办");
        assert_eq!(summary_count_text("处理人", 2), "处理人 等 2+");
        assert_eq!(Operator::Contains.value_placeholder(), Some("请输入"));
        assert_eq!(Operator::Is.value_placeholder(), Some("请选择"));
        assert_eq!(Operator::IsEmpty.value_placeholder(), None);
        assert!(!Operator::IsEmpty.needs_value());
        assert_eq!(Dimension::Source.value_options().len(), 8);
    }

    /// 空输入（已下发、零条件）⇒ 显式空态节点，⛔ 不是空列表。
    #[test]
    fn ready_empty_is_empty_state_not_blank() {
        let st = resolve(Some(true));
        assert_eq!(st, State::Ready(FilterModel::default()));
        assert!(shows_empty_hint(&st));
        assert_eq!(content_node_count(&st), 5); // 4 + 1 个空态节点
        let State::Ready(m) = &st else { unreachable!() };
        assert!(m.is_empty());
        assert_eq!(m.unset_count(), 0);
    }

    /// 状态分支全覆盖（每分支一条）+ 计数正确。
    #[test]
    fn state_branch_counts() {
        let two = FilterModel {
            conjunction: Conjunction::Any,
            conditions: vec![
                Condition {
                    field: Dimension::Owner,
                    op: Operator::Is,
                    value: None,
                },
                Condition {
                    field: Dimension::Source,
                    op: Operator::NotContains,
                    value: Some("转交"),
                },
            ],
            todo_count: Some(7),
        };
        assert_eq!(content_node_count(&State::NotDelivered), 1);
        assert_eq!(content_node_count(&State::LoadFailed), 1);
        assert_eq!(content_node_count(&State::Ready(FilterModel::default())), 5);
        assert_eq!(content_node_count(&State::Ready(two.clone())), 6);
        assert_eq!(two.unset_count(), 1);
        assert_eq!(todo_count_text(two.todo_count.unwrap()), "7 个待办");
    }

    /// main_area 只构造元素树、不 panic；Ready/LoadFailed 分支整树同样构造。
    #[test]
    fn main_area_smoke() {
        let _ = main_area(&Theme::default());
        let model = FilterModel {
            conditions: vec![Condition {
                field: Dimension::Status,
                op: Operator::IsNotEmpty,
                value: None,
            }],
            ..FilterModel::default()
        };
        let _ = render_state(&State::Ready(model), &Theme::default());
        let _ = render_state(&State::LoadFailed, &Theme::default());
    }
}
