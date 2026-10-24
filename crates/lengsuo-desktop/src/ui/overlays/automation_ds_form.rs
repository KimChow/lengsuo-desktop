//! 数据源表单（波次 8，卡 `ui-automation-ds-form`）。
//! 分层约定（同 `panel_views` / `delete`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `automation.datasource.form`。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ── 纯数据层（无 GPUI 依赖） ─────────────────────────────────────────

/// 界面文案全表：命名空间 `automation.datasource.form` 全 68 键（短名、中文），
/// 逐字取自文案全表；完整键 = `NS_PREFIX` + "." + 短名。
pub const TEXTS: [(&str, &str); 68] = [
    ("addRule", "+ 添加规则"),
    ("assigneeEmpty", "assignee 为空"),
    ("connector", "连接器"),
    ("connectorEmpty", "请选择一个已授权的连接器"),
    ("connectorHint", "选择一个连接器作为数据同步来源。"),
    ("connectorUnauthorizedHint", "需先在连接器配置中完成授权"),
    ("create", "创建"),
    ("disable", "禁用"),
    ("enable", "启用"),
    ("filterEmptyHint", "不填则同步全部 Issue"),
    ("filterInputLabel", "过滤条件（可选）"),
    ("filterText", "同步说明"),
    (
        "filterTextHint",
        "用一句话描述你希望同步哪些内容，系统会据此生成过滤规则。",
    ),
    (
        "filterTextPlaceholder",
        "只同步符合条件的数据 · 不填则默认同步最多500条待办数据",
    ),
    ("generateAndPreview", "✨ 生成并预览"),
    ("generateEmptyHint", "无法从描述中提取有效的过滤条件"),
    ("generateRules", "生成规则"),
    ("generateRulesWithSparkle", "✨ 生成规则"),
    ("generating", "生成中..."),
    ("interval.3h", "3 小时"),
    ("intervalEvery", "每"),
    ("intervalPullOnce", "拉取一次"),
    ("intervalUnitHours", "小时"),
    ("intervalUnitMinutes", "分钟"),
    ("matchAll", "全部满足"),
    ("matchAny", "满足任一"),
    ("matchLabel", "规则关系"),
    ("notSelected", "未选择"),
    ("operatorContains", "包含"),
    ("operatorIs", "为"),
    ("previewing", "预览中..."),
    ("pullInterval", "拉取间隔"),
    ("redescribe", "重新描述"),
    ("regenerate", "重新生成"),
    ("removeRule", "删除规则"),
    ("repo", "选择目标"),
    ("repoHint", "当前阶段先使用 mock 仓库列表。"),
    ("repoPlaceholder", "选择要同步的仓库"),
    ("ruleCount", "规则数"),
    ("ruleCountValue", "{count} 条"),
    ("ruleField", "字段"),
    ("ruleFieldPlaceholder", "如：labels"),
    ("ruleOperator", "条件"),
    ("ruleOperatorPlaceholder", "如：contains"),
    ("rules", "过滤规则"),
    ("rulesEmpty", "还没有过滤规则，可先填写同步说明并生成。"),
    ("ruleValue", "值"),
    ("ruleValuePlaceholder", "如：ai-ready"),
    ("save", "保存"),
    ("saveAndEnable", "保存并启用"),
    ("saving", "保存中..."),
    ("selected", "已选择"),
    ("selectedConnector", "连接器"),
    ("selectedRepo", "目标仓库"),
    ("testButton", "测试同步"),
    ("testComingSoon", "规则测试功能即将上线"),
    ("testEmpty", "未匹配到任何数据"),
    ("testFailed", "测试失败"),
    ("testHint", "生成一份 mock 预览，确认同步结果是否符合预期。"),
    ("testing", "测试中..."),
    (
        "testResultSummary",
        "Mock 预览：预计同步 12 条条目，其中 8 条为新条目，4 条为更新。",
    ),
    ("testResultTitle", "最近一次 mock 测试结果"),
    ("testRules", "▶ 测试"),
    ("testRulesResultTitle", "此处展示最多 5 条样例数据"),
    ("testRulesRunning", "测试中..."),
    ("testSection", "测试同步"),
    ("titleCreate", "添加外部数据源"),
    ("titleEdit", "编辑外部数据源"),
];

/// 命名空间前缀。
pub const NS_PREFIX: &str = "automation.datasource.form";

/// 按完整键（`前缀.短名`）取文案；未命中 ⇒ `None`（fail-closed，⛔ 不回退到自编文案）。
pub fn text(key: &str) -> Option<&'static str> {
    let name = key.strip_prefix(NS_PREFIX)?.strip_prefix('.')?;
    TEXTS.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

/// 按短名取文案；未命中 ⇒ 「未下发」（fail-closed 兜底）。
fn ns(name: &str) -> &'static str {
    text(&format!("{NS_PREFIX}.{name}")).unwrap_or(NOT_DELIVERED)
}

/// 视图标题（浮层名）。
pub const TITLE: &str = "数据源表单";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 表单状态机：创建（`titleCreate`）/ 编辑（`titleEdit`）/ 未授权（`connectorUnauthorizedHint`）；
/// 默认 `NotDelivered`（未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    Create,
    Edit,
    Unauthorized,
    #[default]
    NotDelivered,
}

/// 由运行时输入解析初始态（纯函数）：目录未下发 ⇒ `NotDelivered`（fail-closed），
/// 已选连接器未授权 ⇒ `Unauthorized`，否则进入创建表单。
pub fn resolve(catalog_delivered: Option<bool>, picked: Option<bool>) -> State {
    match catalog_delivered {
        None | Some(false) => State::NotDelivered,
        Some(true) => match picked {
            Some(false) => State::Unauthorized,
            _ => State::Create,
        },
    }
}

/// 两分支枚举表：变体 ⇒ 全表短名，`label()` 返回逐字文案（未命中 fail-closed）。
macro_rules! label_enum {
    ($t:ident, $($v:ident => $k:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum $t { $($v),+ }
        impl $t {
            /// 展示文案（全表短名解析）。
            pub fn label(&self) -> &'static str {
                match self { $(Self::$v => ns($k)),+ }
            }
        }
    };
}

// 规则操作符（`operatorIs` / `operatorContains`）。
label_enum!(Op, Is => "operatorIs", Contains => "operatorContains");
// 规则组合关系（`matchAll` / `matchAny`）。
label_enum!(RuleMatch, All => "matchAll", Any => "matchAny");
// 拉取间隔单位（`intervalUnitMinutes` / `intervalUnitHours`）。
label_enum!(IntervalUnit, Minutes => "intervalUnitMinutes", Hours => "intervalUnitHours");

/// 拉取策略：每 N 单位（`intervalEvery`+数值+单位）或仅一次（`intervalPullOnce`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullPolicy {
    Every { value: u32, unit: IntervalUnit },
    Once,
}

impl PullPolicy {
    /// 展示文案（如「每 3 小时」/「拉取一次」）。
    pub fn label(&self) -> String {
        match self {
            PullPolicy::Once => ns("intervalPullOnce").to_string(),
            PullPolicy::Every { value, unit } => {
                format!("{} {} {}", ns("intervalEvery"), value, unit.label())
            }
        }
    }
}

/// 规则生成/预览阶段：`generating` / `previewing` / `generateEmptyHint`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GenStage {
    #[default]
    Idle,
    Generating,
    Previewing,
    NoExtract,
}

impl GenStage {
    /// 阶段提示；空闲 ⇒ `None`（fail-closed）。
    pub fn stage_label(&self) -> Option<&'static str> {
        match self {
            GenStage::Idle => None,
            GenStage::Generating => Some(ns("generating")),
            GenStage::Previewing => Some(ns("previewing")),
            GenStage::NoExtract => Some(ns("generateEmptyHint")),
        }
    }
}

/// 测试同步阶段：`testing` / `testFailed` / `testEmpty` / `testResultTitle`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TestStage {
    #[default]
    Idle,
    Testing,
    Failed,
    Empty,
    Done,
}

impl TestStage {
    /// 测试按钮文案：测试中显示进行中文案，其余回默认动作（`testRules`）。
    pub fn action_label(&self) -> &'static str {
        match self {
            TestStage::Testing => ns("testing"),
            _ => ns("testRules"),
        }
    }

    /// 阶段提示；空闲 ⇒ `None`。
    pub fn stage_label(&self) -> Option<&'static str> {
        match self {
            TestStage::Idle => None,
            TestStage::Testing => Some(ns("testRulesRunning")),
            TestStage::Failed => Some(ns("testFailed")),
            TestStage::Empty => Some(ns("testEmpty")),
            TestStage::Done => Some(ns("testResultTitle")),
        }
    }

    /// 最近一次测试结果文案；仅 `Done` 提供（⛔ 未出结果不展示数值）。
    pub fn result_labels(&self) -> Vec<&'static str> {
        match self {
            TestStage::Done => vec![ns("testResultSummary"), ns("testRulesResultTitle")],
            _ => Vec::new(),
        }
    }
}

/// 过滤规则行：字段 / 操作符 / 值（空段显示全表 `rule*Placeholder` 占位符）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterRule {
    pub field: String,
    pub op: Op,
    pub value: String,
}

/// 已选连接器（只记展示名与授权态；⛔ 不持凭据、不判权限）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectorPick {
    pub name: String,
    pub authorized: bool,
}

/// 连接器区视图分支（fail-closed：未下发 ≠ 未选择 ≠ 已选）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectorView {
    /// 目录未下发（如实显示「未下发」，⛔ 不渲染空目录）。
    NotDelivered,
    /// 目录已下发但未选择（`connectorEmpty` 空态）。
    Empty,
    /// 已选但未授权（`connectorUnauthorizedHint`）。
    Unauthorized,
    /// 已选且已授权（`selected` 摘要）。
    Picked,
}

/// 规则区视图分支：空输入 ⇒ 空态文案（⛔ 不是空列表）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RulesView {
    /// 空态（`rulesEmpty`）。
    Empty,
    /// 规则表（`ruleCountValue` 计数 + 行明细）。
    Table,
}

/// 数据源表单载体（纯数据层；视图是它的纯函数）。
#[derive(Clone, Debug, PartialEq)]
pub struct DsForm {
    pub state: State,
    pub connector: Option<ConnectorPick>,
    pub filter_text: String,
    pub rules: Vec<FilterRule>,
    pub match_mode: RuleMatch,
    pub interval: PullPolicy,
    pub gen: GenStage,
    pub test: TestStage,
    pub saving: bool,
}

impl DsForm {
    /// 未下发的诚实构造（fail-closed 默认；⛔ 不伪造目录/条目/数值）。
    #[rustfmt::skip]
    pub fn not_delivered() -> Self {
        Self {
            state: State::NotDelivered, connector: None, filter_text: String::new(),
            rules: Vec::new(), match_mode: RuleMatch::All,
            interval: PullPolicy::Every { value: 3, unit: IntervalUnit::Hours },
            gen: GenStage::Idle, test: TestStage::Idle, saving: false,
        }
    }

    /// 页面标题（创建/编辑分支）；未下发/未授权 ⇒ `None`（fail-closed）。
    pub fn title(&self) -> Option<&'static str> {
        match self.state {
            State::Create => Some(ns("titleCreate")),
            State::Edit => Some(ns("titleEdit")),
            _ => None,
        }
    }

    /// 连接器区视图分支（纯函数）。
    pub fn connector_view(&self) -> ConnectorView {
        if self.state == State::NotDelivered {
            return ConnectorView::NotDelivered;
        }
        match &self.connector {
            None => ConnectorView::Empty,
            Some(c) if !c.authorized => ConnectorView::Unauthorized,
            Some(_) => ConnectorView::Picked,
        }
    }

    /// 规则区视图分支：空输入 ⇒ 空态，⛔ 不是空列表。
    pub fn rules_view(&self) -> RulesView {
        if self.rules.is_empty() {
            RulesView::Empty
        } else {
            RulesView::Table
        }
    }

    /// 规则计数文案（`{count} 条` 模板替换）；无规则 ⇒ `None`。
    pub fn rule_count_text(&self) -> Option<String> {
        (!self.rules.is_empty())
            .then(|| ns("ruleCountValue").replacen("{count}", &self.rules.len().to_string(), 1))
    }

    /// 拉取间隔展示文案。
    pub fn interval_text(&self) -> String {
        self.interval.label()
    }

    /// 生成区动作文案：无描述（生成/生成并预览）⇄ 有描述（重新描述/重新生成）。
    #[rustfmt::skip]
    pub fn generate_labels(&self) -> Vec<&'static str> {
        if self.filter_text.is_empty() {
            vec![ns("generateRulesWithSparkle"), ns("generateAndPreview"), ns("generateRules")]
        } else {
            vec![ns("generateRulesWithSparkle"), ns("redescribe"), ns("regenerate")]
        }
    }

    /// 底部动作文案：保存中 ⇒ `saving`；创建态（保存并启用/保存/创建）；
    /// 编辑态（保存 + 启用/禁用切换对）；未下发 ⇒ 空（fail-closed）。
    pub fn footer_labels(&self) -> Vec<&'static str> {
        if self.saving {
            return vec![ns("saving")];
        }
        match self.state {
            State::Create => vec![ns("saveAndEnable"), ns("save"), ns("create")],
            State::Edit => vec![ns("save"), ns("enable"), ns("disable")],
            _ => Vec::new(),
        }
    }
}

// ── GPUI 层 ─────────────────────────────────────────────────────────

/// 动作 chip（颜色一律 theme token，⛔ 不硬编码色值）。
fn chip(theme: &Theme, label: impl Into<SharedString>) -> Div {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_color(theme.muted_foreground)
        .child(label.into())
}

/// 文案序列 → chip 行。
fn chips(theme: &Theme, labels: &[&'static str]) -> Div {
    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .children(labels.iter().map(|l| chip(theme, *l)))
}

/// 静态说明行（muted 文案）。
fn hint(theme: &Theme, line: impl Into<SharedString>) -> Div {
    div().text_color(theme.muted_foreground).child(line.into())
}

/// 纵向堆叠容器（子节点列表）。
fn stack(children: impl IntoIterator<Item = Div>) -> Div {
    div().flex().flex_col().gap_1().children(children)
}

/// 表单分区卡：标题行 + 主体（结构对齐 `delete` 卡的 branch_card）。
fn section(theme: &Theme, heading: impl Into<SharedString>, body: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(div().text_color(theme.foreground).child(heading.into()))
        .child(body)
}

/// 连接器区：标题 `connector`；按 [`ConnectorView`] 四分支（含未下发 fail-closed）。
fn connector_section(theme: &Theme, form: &DsForm) -> Div {
    let body = match form.connector_view() {
        ConnectorView::NotDelivered => hint(theme, NOT_DELIVERED),
        ConnectorView::Unauthorized => hint(theme, ns("connectorUnauthorizedHint")),
        ConnectorView::Empty => stack([
            hint(theme, ns("connectorEmpty")),
            hint(theme, ns("connectorHint")),
        ]),
        ConnectorView::Picked => {
            let name = &form.connector.as_ref().unwrap().name;
            let picked = format!("{} {}", ns("selected"), name);
            hint(theme, format!("{}：{}", ns("selectedConnector"), picked))
        }
    };
    section(theme, ns("connector"), body)
}

/// 选择目标区：目标仓库未选择时如实显示 `notSelected`（⛔ 不伪造仓库列表）。
fn repo_section(theme: &Theme) -> Div {
    let target = format!("{}：{}", ns("selectedRepo"), ns("notSelected"));
    let body = stack([
        hint(theme, target),
        hint(theme, ns("repoPlaceholder")),
        hint(theme, ns("repoHint")),
    ]);
    section(theme, ns("repo"), body)
}

/// 同步说明区：说明输入 + 过滤条件标签 + 空/占位提示 + 生成动作行。
fn filter_section(theme: &Theme, form: &DsForm) -> Div {
    let value = if form.filter_text.is_empty() {
        ns("filterTextPlaceholder")
    } else {
        form.filter_text.as_str()
    };
    let body = stack([
        hint(theme, ns("filterTextHint")),
        hint(theme, format!("{}：{}", ns("filterInputLabel"), value)),
        hint(theme, ns("filterEmptyHint")),
        chips(theme, &form.generate_labels()),
    ])
    .children(form.gen.stage_label().map(|l| chip(theme, l)));
    section(theme, ns("filterText"), body)
}

/// 空段显示占位符，否则原值。
fn or_ph<'a>(v: &'a str, ph: &'a str) -> &'a str {
    if v.is_empty() {
        ph
    } else {
        v
    }
}

/// 单条规则行：字段/条件/值三段（空段显示全表占位符）+ 删除动作。
fn rule_row(theme: &Theme, rule: &FilterRule) -> Div {
    let line = format!(
        "{} {} · {} {} · {} {}",
        ns("ruleField"),
        or_ph(&rule.field, ns("ruleFieldPlaceholder")),
        ns("ruleOperator"),
        or_ph(rule.op.label(), ns("ruleOperatorPlaceholder")),
        ns("ruleValue"),
        or_ph(&rule.value, ns("ruleValuePlaceholder"))
    );
    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .child(SharedString::from(line))
        .child(chip(theme, ns("removeRule")))
}

/// 过滤规则区：空输入 ⇒ `rulesEmpty` 空态（⛔ 不是空列表）；有规则 ⇒ 计数 + 行明细。
fn rules_section(theme: &Theme, form: &DsForm) -> Div {
    #[rustfmt::skip]
    let count = form.rule_count_text().unwrap_or_else(|| ns("notSelected").to_string());
    let heading = format!(
        "{}（{}：{} / {}：{}）",
        ns("rules"),
        ns("matchLabel"),
        form.match_mode.label(),
        ns("ruleCount"),
        count
    );
    let body = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(chips(theme, &[ns("matchAll"), ns("matchAny")]))
        .child(chips(theme, &[ns("addRule"), ns("assigneeEmpty")]))
        .child(match form.rules_view() {
            RulesView::Empty => hint(theme, ns("rulesEmpty")),
            RulesView::Table => stack(form.rules.iter().map(|r| rule_row(theme, r))),
        });
    section(theme, heading, body)
}

/// 拉取间隔区：当前策略文案 + 全表现成预设（`interval.3h` / `intervalPullOnce`）。
fn interval_section(theme: &Theme, form: &DsForm) -> Div {
    let body = div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .child(chip(theme, form.interval_text()))
        .child(chip(theme, ns("interval.3h")))
        .child(chip(theme, ns("intervalPullOnce")));
    section(theme, ns("pullInterval"), body)
}

/// 测试同步区：提示 + 动作 + 即将上线说明 + 阶段/结果行（未出结果不展示数值）。
fn test_section(theme: &Theme, form: &DsForm) -> Div {
    let act = form.test.action_label();
    let body = stack([
        hint(theme, ns("testHint")),
        hint(theme, ns("testComingSoon")),
        chips(theme, &[act, ns("testButton")]),
    ])
    .children(form.test.stage_label().map(|l| chip(theme, l)))
    .children(form.test.result_labels().iter().map(|l| hint(theme, *l)));
    section(theme, ns("testSection"), body)
}

/// 主区渲染：标题行 + 六个内容节点 + 空态 + 「未下发」fail-closed 分支。
pub fn main_area(theme: &Theme) -> AnyElement {
    let form = DsForm::not_delivered();
    #[rustfmt::skip]
    let title = SharedString::from(format!("{}（{}）", TITLE, form.title().unwrap_or(NOT_DELIVERED)));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(title))
        .child(connector_section(theme, &form))
        .child(repo_section(theme))
        .child(filter_section(theme, &form))
        .child(rules_section(theme, &form))
        .child(interval_section(theme, &form))
        .child(test_section(theme, &form))
        .child(chips(theme, &form.footer_labels()))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        main_area, resolve, text, ConnectorPick, ConnectorView, DsForm, FilterRule, GenStage,
        IntervalUnit, Op, PullPolicy, RuleMatch, RulesView, State, TestStage, NOT_DELIVERED,
        NS_PREFIX, TEXTS,
    };
    use gpui_kit::component::Theme;

    /// ① 纯函数层计数正确：全表 68 键、不重复，短名拼前缀后仍可解析。
    #[test]
    fn table_has_68_unique_keys_in_namespace() {
        assert_eq!(TEXTS.len(), 68);
        let mut keys: Vec<&str> = TEXTS.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), TEXTS.len());
        assert!(keys.iter().all(|k| !k.is_empty() && !k.starts_with('.')));
    }

    /// ③ 文案逐字一致（≥3 个 key，含占位符模板与特殊符号）。
    #[test]
    fn copy_is_verbatim_from_table() {
        let t = |n: &str| text(&format!("{NS_PREFIX}.{n}"));
        assert_eq!(t("connector"), Some("连接器"));
        assert_eq!(t("titleCreate"), Some("添加外部数据源"));
        assert_eq!(t("assigneeEmpty"), Some("assignee 为空"));
        assert_eq!(t("ruleCountValue"), Some("{count} 条"));
        assert_eq!(t("interval.3h"), Some("3 小时"));
        assert_eq!(t("testRules"), Some("▶ 测试"));
    }

    /// ② 空输入 ⇒ 空态（不是空列表）+ `resolve` 全分支（每分支一条断言）。
    #[test]
    fn empty_input_yields_empty_states_not_empty_lists() {
        let form = DsForm::not_delivered();
        assert_eq!(form.state, State::NotDelivered);
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(form.connector_view(), ConnectorView::NotDelivered);
        assert_eq!(form.rules_view(), RulesView::Empty);
        assert!(form.rule_count_text().is_none());
        assert!(form.footer_labels().is_empty());
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(resolve(None, None), State::NotDelivered);
        assert_eq!(resolve(None, Some(true)), State::NotDelivered);
        assert_eq!(resolve(Some(false), Some(true)), State::NotDelivered);
        assert_eq!(resolve(Some(true), None), State::Create);
        assert_eq!(resolve(Some(true), Some(true)), State::Create);
        assert_eq!(resolve(Some(true), Some(false)), State::Unauthorized);
    }

    /// ⑤ 连接器区分支覆盖 + 页面标题随创建/编辑切换。
    #[test]
    fn connector_view_covers_every_branch() {
        let mut form = DsForm::not_delivered();
        form.state = State::Create;
        assert_eq!(form.connector_view(), ConnectorView::Empty);
        #[rustfmt::skip]
        let demo = |ok: bool| Some(ConnectorPick { name: "demo".into(), authorized: ok });
        form.connector = demo(false);
        assert_eq!(form.connector_view(), ConnectorView::Unauthorized);
        form.connector = demo(true);
        assert_eq!(form.connector_view(), ConnectorView::Picked);
        assert_eq!(form.title(), Some("添加外部数据源"));
        form.state = State::Edit;
        assert_eq!(form.title(), Some("编辑外部数据源"));
    }

    /// ① 计数/间隔模板：`{count} 条` 替换、操作符与关系文案、「每 3 小时」。
    #[test]
    fn count_and_interval_templates() {
        #[rustfmt::skip]
        let rule = |f: &str, op: Op, v: &str| FilterRule { field: f.into(), op, value: v.into() };
        let mut form = DsForm::not_delivered();
        form.state = State::Create;
        form.rules = vec![
            rule("labels", Op::Contains, "ai-ready"),
            rule("assignee", Op::Is, ""),
        ];
        assert_eq!(form.rules_view(), RulesView::Table);
        assert_eq!(form.rule_count_text().as_deref(), Some("2 条"));
        assert_eq!(Op::Is.label(), "为");
        assert_eq!(Op::Contains.label(), "包含");
        assert_eq!(RuleMatch::All.label(), "全部满足");
        form.match_mode = RuleMatch::Any;
        assert_eq!(form.match_mode.label(), "满足任一");
        form.interval = PullPolicy::Once;
        assert_eq!(form.interval_text(), "拉取一次");
        #[rustfmt::skip]
        let every3h = PullPolicy::Every { value: 3, unit: IntervalUnit::Hours };
        form.interval = every3h;
        assert_eq!(form.interval_text(), "每 3 小时");
        assert_eq!(IntervalUnit::Minutes.label(), "分钟");
    }

    /// ⑤ 生成/测试/保存各阶段分支覆盖 + 动作文案随状态切换（每分支一条断言）。
    #[test]
    fn stage_and_action_labels_cover_every_branch() {
        assert_eq!(GenStage::Idle.stage_label(), None);
        assert_eq!(TestStage::Idle.stage_label(), None);
        for (got, want) in [
            (GenStage::Generating.stage_label(), "生成中..."),
            (GenStage::Previewing.stage_label(), "预览中..."),
            (
                GenStage::NoExtract.stage_label(),
                "无法从描述中提取有效的过滤条件",
            ),
            (TestStage::Testing.stage_label(), "测试中..."),
            (TestStage::Failed.stage_label(), "测试失败"),
            (TestStage::Empty.stage_label(), "未匹配到任何数据"),
            (TestStage::Done.stage_label(), "最近一次 mock 测试结果"),
        ] {
            assert_eq!(got, Some(want));
        }
        assert_eq!(TestStage::Idle.action_label(), "▶ 测试");
        assert_eq!(TestStage::Testing.action_label(), "测试中...");
        assert!(TestStage::Idle.result_labels().is_empty());
        assert_eq!(TestStage::Done.result_labels().len(), 2);
        let mut form = DsForm::not_delivered();
        form.state = State::Create;
        assert_eq!(form.generate_labels()[0], "✨ 生成规则");
        assert_eq!(form.generate_labels()[1], "✨ 生成并预览");
        assert_eq!(form.generate_labels()[2], "生成规则");
        assert_eq!(form.footer_labels(), vec!["保存并启用", "保存", "创建"]);
        form.state = State::Edit;
        assert_eq!(form.footer_labels(), vec!["保存", "启用", "禁用"]);
        form.saving = true;
        assert_eq!(form.footer_labels(), vec!["保存中..."]);
        form.filter_text = "只同步 ai-ready".into();
        assert_eq!(form.generate_labels()[0], "✨ 生成规则");
        assert_eq!(form.generate_labels()[1], "重新描述");
        assert_eq!(form.generate_labels()[2], "重新生成");
    }

    /// ④ `main_area(&Theme::default())` 不 panic（只构造元素树）。
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
