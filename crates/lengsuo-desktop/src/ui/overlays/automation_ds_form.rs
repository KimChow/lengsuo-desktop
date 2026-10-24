//! 数据源表单（波次 8，卡 `ui-automation-ds-form`）。
//!
//! 分层约定（同 `panel_views` / `delete`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `automation.datasource.form`。
//! 数据源在运行时侧：连接器目录未接入前保持「未下发」的诚实默认态。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ── 纯数据层（无 GPUI 依赖） ─────────────────────────────────────────

/// 界面文案全表：命名空间 `automation.datasource.form` 全 68 键（`key`、中文），逐字取自文案全表。
pub const TEXTS: [(&str, &str); 68] = [
    ("automation.datasource.form.addRule", "+ 添加规则"),
    ("automation.datasource.form.assigneeEmpty", "assignee 为空"),
    ("automation.datasource.form.connector", "连接器"),
    (
        "automation.datasource.form.connectorEmpty",
        "请选择一个已授权的连接器",
    ),
    (
        "automation.datasource.form.connectorHint",
        "选择一个连接器作为数据同步来源。",
    ),
    (
        "automation.datasource.form.connectorUnauthorizedHint",
        "需先在连接器配置中完成授权",
    ),
    ("automation.datasource.form.create", "创建"),
    ("automation.datasource.form.disable", "禁用"),
    ("automation.datasource.form.enable", "启用"),
    (
        "automation.datasource.form.filterEmptyHint",
        "不填则同步全部 Issue",
    ),
    (
        "automation.datasource.form.filterInputLabel",
        "过滤条件（可选）",
    ),
    ("automation.datasource.form.filterText", "同步说明"),
    (
        "automation.datasource.form.filterTextHint",
        "用一句话描述你希望同步哪些内容，系统会据此生成过滤规则。",
    ),
    (
        "automation.datasource.form.filterTextPlaceholder",
        "只同步符合条件的数据 · 不填则默认同步最多500条待办数据",
    ),
    (
        "automation.datasource.form.generateAndPreview",
        "✨ 生成并预览",
    ),
    (
        "automation.datasource.form.generateEmptyHint",
        "无法从描述中提取有效的过滤条件",
    ),
    ("automation.datasource.form.generateRules", "生成规则"),
    (
        "automation.datasource.form.generateRulesWithSparkle",
        "✨ 生成规则",
    ),
    ("automation.datasource.form.generating", "生成中..."),
    ("automation.datasource.form.interval.3h", "3 小时"),
    ("automation.datasource.form.intervalEvery", "每"),
    ("automation.datasource.form.intervalPullOnce", "拉取一次"),
    ("automation.datasource.form.intervalUnitHours", "小时"),
    ("automation.datasource.form.intervalUnitMinutes", "分钟"),
    ("automation.datasource.form.matchAll", "全部满足"),
    ("automation.datasource.form.matchAny", "满足任一"),
    ("automation.datasource.form.matchLabel", "规则关系"),
    ("automation.datasource.form.notSelected", "未选择"),
    ("automation.datasource.form.operatorContains", "包含"),
    ("automation.datasource.form.operatorIs", "为"),
    ("automation.datasource.form.previewing", "预览中..."),
    ("automation.datasource.form.pullInterval", "拉取间隔"),
    ("automation.datasource.form.redescribe", "重新描述"),
    ("automation.datasource.form.regenerate", "重新生成"),
    ("automation.datasource.form.removeRule", "删除规则"),
    ("automation.datasource.form.repo", "选择目标"),
    (
        "automation.datasource.form.repoHint",
        "当前阶段先使用 mock 仓库列表。",
    ),
    (
        "automation.datasource.form.repoPlaceholder",
        "选择要同步的仓库",
    ),
    ("automation.datasource.form.ruleCount", "规则数"),
    ("automation.datasource.form.ruleCountValue", "{count} 条"),
    ("automation.datasource.form.ruleField", "字段"),
    (
        "automation.datasource.form.ruleFieldPlaceholder",
        "如：labels",
    ),
    ("automation.datasource.form.ruleOperator", "条件"),
    (
        "automation.datasource.form.ruleOperatorPlaceholder",
        "如：contains",
    ),
    ("automation.datasource.form.rules", "过滤规则"),
    (
        "automation.datasource.form.rulesEmpty",
        "还没有过滤规则，可先填写同步说明并生成。",
    ),
    ("automation.datasource.form.ruleValue", "值"),
    (
        "automation.datasource.form.ruleValuePlaceholder",
        "如：ai-ready",
    ),
    ("automation.datasource.form.save", "保存"),
    ("automation.datasource.form.saveAndEnable", "保存并启用"),
    ("automation.datasource.form.saving", "保存中..."),
    ("automation.datasource.form.selected", "已选择"),
    ("automation.datasource.form.selectedConnector", "连接器"),
    ("automation.datasource.form.selectedRepo", "目标仓库"),
    ("automation.datasource.form.testButton", "测试同步"),
    (
        "automation.datasource.form.testComingSoon",
        "规则测试功能即将上线",
    ),
    ("automation.datasource.form.testEmpty", "未匹配到任何数据"),
    ("automation.datasource.form.testFailed", "测试失败"),
    (
        "automation.datasource.form.testHint",
        "生成一份 mock 预览，确认同步结果是否符合预期。",
    ),
    ("automation.datasource.form.testing", "测试中..."),
    (
        "automation.datasource.form.testResultSummary",
        "Mock 预览：预计同步 12 条条目，其中 8 条为新条目，4 条为更新。",
    ),
    (
        "automation.datasource.form.testResultTitle",
        "最近一次 mock 测试结果",
    ),
    ("automation.datasource.form.testRules", "▶ 测试"),
    (
        "automation.datasource.form.testRulesResultTitle",
        "此处展示最多 5 条样例数据",
    ),
    ("automation.datasource.form.testRulesRunning", "测试中..."),
    ("automation.datasource.form.testSection", "测试同步"),
    ("automation.datasource.form.titleCreate", "添加外部数据源"),
    ("automation.datasource.form.titleEdit", "编辑外部数据源"),
];

/// 命名空间前缀。
pub const NS_PREFIX: &str = "automation.datasource.form";

/// 按全表 key 取文案；未命中 ⇒ `None`（fail-closed，⛔ 不回退到自编文案）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 按全表 key 取文案的兜底版；未命中 ⇒ 「未下发」（fail-closed）。
fn t(key: &str) -> &'static str {
    text(key).unwrap_or(NOT_DELIVERED)
}

/// 视图标题（浮层名）。
pub const TITLE: &str = "数据源表单";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 表单状态机（每条分支均有全表文案支撑）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 创建分支（`titleCreate` 添加外部数据源）。
    Create,
    /// 编辑分支（`titleEdit` 编辑外部数据源）。
    Edit,
    /// 所选连接器未授权（`connectorUnauthorizedHint`）。
    Unauthorized,
    /// 数据未下发（诚实默认态：未拉取过 ≠ 空集，⛔ 不伪造）。
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

/// 规则操作符（`operatorIs` / `operatorContains`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// 为。
    Is,
    /// 包含。
    Contains,
}

impl Op {
    /// 展示文案键。
    pub const fn label_key(&self) -> &'static str {
        match self {
            Op::Is => "automation.datasource.form.operatorIs",
            Op::Contains => "automation.datasource.form.operatorContains",
        }
    }
}

/// 规则组合关系（`matchAll` / `matchAny`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleMatch {
    /// 全部满足。
    All,
    /// 满足任一。
    Any,
}

impl RuleMatch {
    /// 展示文案键。
    pub const fn label_key(&self) -> &'static str {
        match self {
            RuleMatch::All => "automation.datasource.form.matchAll",
            RuleMatch::Any => "automation.datasource.form.matchAny",
        }
    }
}

/// 拉取间隔单位（`intervalUnitMinutes` / `intervalUnitHours`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalUnit {
    /// 分钟。
    Minutes,
    /// 小时。
    Hours,
}

impl IntervalUnit {
    /// 展示文案键。
    pub const fn label_key(&self) -> &'static str {
        match self {
            IntervalUnit::Minutes => "automation.datasource.form.intervalUnitMinutes",
            IntervalUnit::Hours => "automation.datasource.form.intervalUnitHours",
        }
    }
}

/// 拉取策略：每 N 单位（`intervalEvery`+数值+单位）或仅一次（`intervalPullOnce`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullPolicy {
    /// 每 `value` 个 `unit` 拉取。
    Every { value: u32, unit: IntervalUnit },
    /// 拉取一次。
    Once,
}

impl PullPolicy {
    /// 展示文案（如「每 3 小时」/「拉取一次」）。
    pub fn label(&self) -> String {
        match self {
            PullPolicy::Once => t("automation.datasource.form.intervalPullOnce").to_string(),
            PullPolicy::Every { value, unit } => format!(
                "{} {} {}",
                t("automation.datasource.form.intervalEvery"),
                value,
                t(unit.label_key())
            ),
        }
    }
}

/// 规则生成/预览阶段（生成区文案随阶段切换）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GenStage {
    /// 空闲（无阶段提示）。
    #[default]
    Idle,
    /// 生成中（`generating`）。
    Generating,
    /// 预览中（`previewing`）。
    Previewing,
    /// 描述提不出有效条件（`generateEmptyHint`）。
    NoExtract,
}

impl GenStage {
    /// 阶段提示键；空闲 ⇒ `None`（fail-closed）。
    pub fn stage_key(&self) -> Option<&'static str> {
        match self {
            GenStage::Idle => None,
            GenStage::Generating => Some("automation.datasource.form.generating"),
            GenStage::Previewing => Some("automation.datasource.form.previewing"),
            GenStage::NoExtract => Some("automation.datasource.form.generateEmptyHint"),
        }
    }
}

/// 测试同步阶段（测试区文案随阶段切换）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TestStage {
    /// 空闲（无结果）。
    #[default]
    Idle,
    /// 测试中（`testRulesRunning`）。
    Testing,
    /// 测试失败（`testFailed`）。
    Failed,
    /// 未匹配到任何数据（`testEmpty`）。
    Empty,
    /// 已出结果（`testResultTitle`）。
    Done,
}

impl TestStage {
    /// 阶段提示键；空闲 ⇒ `None`。
    pub fn stage_key(&self) -> Option<&'static str> {
        match self {
            TestStage::Idle => None,
            TestStage::Testing => Some("automation.datasource.form.testRulesRunning"),
            TestStage::Failed => Some("automation.datasource.form.testFailed"),
            TestStage::Empty => Some("automation.datasource.form.testEmpty"),
            TestStage::Done => Some("automation.datasource.form.testResultTitle"),
        }
    }
}

/// 过滤规则行：字段 / 操作符 / 值（空段显示全表 `rule*Placeholder` 占位符）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterRule {
    /// 字段名（如 labels）。
    pub field: String,
    /// 操作符。
    pub op: Op,
    /// 比较值（如 ai-ready）。
    pub value: String,
}

/// 已选连接器；未授权 ⇒ 连接器区转 `Unauthorized` 分支（⛔ 不持凭据，只记名称）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectorPick {
    /// 连接器名（展示用）。
    pub name: String,
    /// 是否已在连接器配置中完成授权。
    pub authorized: bool,
}

/// 连接器区视图分支（fail-closed：未下发 ≠ 未选择 ≠ 已选，三态分明）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectorView {
    /// 连接器目录未下发（如实显示「未下发」，⛔ 不渲染空目录）。
    NotDelivered,
    /// 目录已下发但未选择（`connectorEmpty` 空态）。
    Empty,
    /// 已选但未授权（`connectorUnauthorizedHint`）。
    Unauthorized,
    /// 已选且已授权（`selected` 摘要）。
    Picked,
}

/// 规则区视图分支：无规则 ⇒ 空态文案（⛔ 不是空列表）。
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
    /// 状态机分支。
    pub state: State,
    /// 已选连接器；`None` ⇒ 连接器区空态。
    pub connector: Option<ConnectorPick>,
    /// 同步说明（自然语言描述，驱动规则生成）。
    pub filter_text: String,
    /// 过滤规则集。
    pub rules: Vec<FilterRule>,
    /// 规则组合关系。
    pub match_mode: RuleMatch,
    /// 拉取策略（默认每 3 小时）。
    pub interval: PullPolicy,
    /// 规则生成/预览阶段。
    pub gen: GenStage,
    /// 测试同步阶段。
    pub test: TestStage,
    /// 保存中（底部动作转 `saving`）。
    pub saving: bool,
}

impl Default for DsForm {
    fn default() -> Self {
        Self {
            state: State::NotDelivered,
            connector: None,
            filter_text: String::new(),
            rules: Vec::new(),
            match_mode: RuleMatch::All,
            interval: PullPolicy::Every {
                value: 3,
                unit: IntervalUnit::Hours,
            },
            gen: GenStage::Idle,
            test: TestStage::Idle,
            saving: false,
        }
    }
}

impl DsForm {
    /// 未下发的诚实构造（fail-closed 默认；⛔ 不伪造目录/条目/数值）。
    pub fn not_delivered() -> Self {
        Self::default()
    }

    /// 页面标题（创建/编辑分支）；未下发/未授权 ⇒ `None`（fail-closed）。
    pub fn title(&self) -> Option<&'static str> {
        match self.state {
            State::Create => text("automation.datasource.form.titleCreate"),
            State::Edit => text("automation.datasource.form.titleEdit"),
            _ => None,
        }
    }

    /// 连接器区视图分支（纯函数）。
    pub fn connector_view(&self) -> ConnectorView {
        match self.state {
            State::NotDelivered => ConnectorView::NotDelivered,
            State::Unauthorized => ConnectorView::Unauthorized,
            _ => match self.connector {
                Some(_) => ConnectorView::Picked,
                None => ConnectorView::Empty,
            },
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
        (!self.rules.is_empty()).then(|| {
            t("automation.datasource.form.ruleCountValue").replacen(
                "{count}",
                &self.rules.len().to_string(),
                1,
            )
        })
    }

    /// 拉取间隔展示文案。
    pub fn interval_text(&self) -> String {
        self.interval.label()
    }

    /// 生成区动作键序列：无描述（生成/生成并预览）⇄ 有描述（重新描述/重新生成）。
    pub fn generate_keys(&self) -> &'static [&'static str] {
        if self.filter_text.is_empty() {
            &[
                "automation.datasource.form.generateRulesWithSparkle",
                "automation.datasource.form.generateAndPreview",
                "automation.datasource.form.generateRules",
            ]
        } else {
            &[
                "automation.datasource.form.generateRulesWithSparkle",
                "automation.datasource.form.redescribe",
                "automation.datasource.form.regenerate",
            ]
        }
    }

    /// 测试区按钮文案键：测试中显示进行中文案，其余回默认动作。
    pub fn test_action_key(&self) -> &'static str {
        match self.test {
            TestStage::Testing => "automation.datasource.form.testing",
            _ => "automation.datasource.form.testRules",
        }
    }

    /// 最近一次测试结果文案键；仅 `Done` 分支提供（⛔ 未出结果不展示数值）。
    pub fn test_result_keys(&self) -> &'static [&'static str] {
        match self.test {
            TestStage::Done => &[
                "automation.datasource.form.testResultSummary",
                "automation.datasource.form.testRulesResultTitle",
            ],
            _ => &[],
        }
    }

    /// 底部动作键序列：保存中 ⇒ `saving`；创建态（保存并启用/保存/创建）；
    /// 编辑态（保存 + 启用/禁用切换对）；未下发 ⇒ 空（fail-closed）。
    pub fn footer_keys(&self) -> &'static [&'static str] {
        if self.saving {
            return &["automation.datasource.form.saving"];
        }
        match self.state {
            State::Create => &[
                "automation.datasource.form.saveAndEnable",
                "automation.datasource.form.save",
                "automation.datasource.form.create",
            ],
            State::Edit => &[
                "automation.datasource.form.save",
                "automation.datasource.form.enable",
                "automation.datasource.form.disable",
            ],
            _ => &[],
        }
    }
}

// ── GPUI 层 ─────────────────────────────────────────────────────────

/// 动作 chip（颜色一律 theme token，⛔ 不硬编码色值；触摸目标 ≥48dp 由 px/py 保证）。
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

/// 键序列 → chip 行。
fn chips(theme: &Theme, keys: &[&str]) -> Div {
    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .children(keys.iter().map(|k| chip(theme, t(k))))
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

/// 静态说明行（muted 文案）。
fn hint(theme: &Theme, line: impl Into<SharedString>) -> Div {
    div().text_color(theme.muted_foreground).child(line.into())
}

/// 连接器区：标题 `connector`，按 [`ConnectorView`] 四分支渲染（含未下发 fail-closed）。
fn connector_section(theme: &Theme, form: &DsForm) -> Div {
    let picked = form
        .connector
        .as_ref()
        .map(|c| c.name.clone())
        .unwrap_or_else(|| NOT_DELIVERED.to_string());
    let body = match form.connector_view() {
        ConnectorView::NotDelivered => hint(theme, NOT_DELIVERED),
        ConnectorView::Empty => div()
            .flex()
            .flex_col()
            .gap_1()
            .child(hint(theme, t("automation.datasource.form.connectorEmpty")))
            .child(hint(theme, t("automation.datasource.form.connectorHint"))),
        ConnectorView::Unauthorized => hint(
            theme,
            t("automation.datasource.form.connectorUnauthorizedHint"),
        ),
        ConnectorView::Picked => hint(
            theme,
            SharedString::from(format!(
                "{} {}：{}",
                t("automation.datasource.form.selected"),
                t("automation.datasource.form.selectedConnector"),
                picked
            )),
        ),
    };
    section(theme, t("automation.datasource.form.connector"), body)
}

/// 选择目标区：目标仓库未选择时如实显示 `notSelected`（⛔ 不伪造仓库列表）。
fn repo_section(theme: &Theme) -> Div {
    let body = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(hint(
            theme,
            SharedString::from(format!(
                "{}：{}",
                t("automation.datasource.form.selectedRepo"),
                t("automation.datasource.form.notSelected")
            )),
        ))
        .child(hint(theme, t("automation.datasource.form.repoPlaceholder")))
        .child(hint(theme, t("automation.datasource.form.repoHint")));
    section(theme, t("automation.datasource.form.repo"), body)
}

/// 同步说明区：说明输入（`filterText`）+ 过滤条件标签 + 空/占位提示。
fn filter_section(theme: &Theme, form: &DsForm) -> Div {
    let body = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(hint(theme, t("automation.datasource.form.filterTextHint")))
        .child(hint(
            theme,
            SharedString::from(format!(
                "{}：{}",
                t("automation.datasource.form.filterInputLabel"),
                if form.filter_text.is_empty() {
                    t("automation.datasource.form.filterTextPlaceholder")
                } else {
                    form.filter_text.as_str()
                }
            )),
        ))
        .child(hint(theme, t("automation.datasource.form.filterEmptyHint")))
        .child(chips(theme, form.generate_keys()))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(form.gen.stage_key().map(|k| chip(theme, t(k)))),
        );
    section(theme, t("automation.datasource.form.filterText"), body)
}

/// 单条规则行：字段/条件/值三段（空段显示全表占位符）+ 删除动作。
fn rule_row(theme: &Theme, rule: &FilterRule) -> Div {
    let seg = |label: &'static str, value: &str, placeholder: &'static str| {
        SharedString::from(format!(
            "{} {}",
            t(label),
            if value.is_empty() { placeholder } else { value }
        ))
    };
    div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .child(seg(
            "automation.datasource.form.ruleField",
            &rule.field,
            t("automation.datasource.form.ruleFieldPlaceholder"),
        ))
        .child(seg(
            "automation.datasource.form.ruleOperator",
            t(rule.op.label_key()),
            t("automation.datasource.form.ruleOperatorPlaceholder"),
        ))
        .child(seg(
            "automation.datasource.form.ruleValue",
            &rule.value,
            t("automation.datasource.form.ruleValuePlaceholder"),
        ))
        .child(chip(theme, t("automation.datasource.form.removeRule")))
}

/// 过滤规则区：空输入 ⇒ `rulesEmpty` 空态（⛔ 不是空列表）；有规则 ⇒ 计数 + 行明细。
fn rules_section(theme: &Theme, form: &DsForm) -> Div {
    let heading = format!(
        "{}（{}：{} / {}：{}）",
        t("automation.datasource.form.rules"),
        t("automation.datasource.form.matchLabel"),
        t(form.match_mode.label_key()),
        t("automation.datasource.form.ruleCount"),
        form.rule_count_text()
            .as_deref()
            .unwrap_or(t("automation.datasource.form.notSelected"))
    );
    let mut body = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(chips(
            theme,
            &[
                "automation.datasource.form.matchAll",
                "automation.datasource.form.matchAny",
            ],
        ))
        .child(chips(
            theme,
            &[
                "automation.datasource.form.addRule",
                "automation.datasource.form.assigneeEmpty",
            ],
        ));
    body = match form.rules_view() {
        RulesView::Empty => body.child(hint(theme, t("automation.datasource.form.rulesEmpty"))),
        RulesView::Table => body.children(form.rules.iter().map(|r| rule_row(theme, r))),
    };
    section(theme, heading, body)
}

/// 拉取间隔区：当前策略文案 + 全表现成预设（`interval.3h` / `intervalPullOnce`）。
fn interval_section(theme: &Theme, form: &DsForm) -> Div {
    let body = div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_2()
        .child(chip(theme, SharedString::from(form.interval_text())))
        .child(chip(theme, t("automation.datasource.form.interval.3h")))
        .child(chip(
            theme,
            t("automation.datasource.form.intervalPullOnce"),
        ));
    section(theme, t("automation.datasource.form.pullInterval"), body)
}

/// 测试同步区：提示 + 动作 + 即将上线说明 + 阶段/结果行（未出结果不展示数值）。
fn test_section(theme: &Theme, form: &DsForm) -> Div {
    let mut body = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(hint(theme, t("automation.datasource.form.testHint")))
        .child(hint(theme, t("automation.datasource.form.testComingSoon")))
        .child(chips(
            theme,
            &[
                form.test_action_key(),
                "automation.datasource.form.testButton",
            ],
        ));
    if let Some(k) = form.test.stage_key() {
        body = body.child(chip(theme, t(k)));
    }
    body = body.children(form.test_result_keys().iter().map(|k| hint(theme, t(k))));
    section(theme, t("automation.datasource.form.testSection"), body)
}

/// 底部动作行（未下发 ⇒ 空行，fail-closed）。
fn action_row(theme: &Theme, form: &DsForm) -> Div {
    chips(theme, form.footer_keys())
}

/// 主区渲染：标题行 + 六个内容节点 + 空态 + 「未下发」fail-closed 分支。
pub fn main_area(theme: &Theme) -> AnyElement {
    let form = DsForm::not_delivered();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(format!(
                    "{}（{}）",
                    TITLE,
                    form.title().unwrap_or(NOT_DELIVERED)
                ))),
        )
        .child(connector_section(theme, &form))
        .child(repo_section(theme))
        .child(filter_section(theme, &form))
        .child(rules_section(theme, &form))
        .child(interval_section(theme, &form))
        .child(test_section(theme, &form))
        .child(action_row(theme, &form))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{resolve, text, State, NOT_DELIVERED, NS_PREFIX, TEXTS, TITLE};

    /// ① 纯函数层状态/计数正确：全表 68 键、全部在命名空间内且不重复。
    #[test]
    fn table_has_68_unique_keys_in_namespace() {
        assert_eq!(TEXTS.len(), 68);
        assert!(TEXTS.iter().all(|(k, _)| k.starts_with(NS_PREFIX)));
        let mut keys: Vec<&str> = TEXTS.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), TEXTS.len());
    }

    /// ③ 文案逐字一致（对 ≥3 个 key 断言，含占位符与特殊符号）。
    #[test]
    fn copy_is_verbatim_from_table() {
        assert_eq!(text("automation.datasource.form.connector"), Some("连接器"));
        assert_eq!(
            text("automation.datasource.form.titleCreate"),
            Some("添加外部数据源")
        );
        assert_eq!(
            text("automation.datasource.form.assigneeEmpty"),
            Some("assignee 为空")
        );
        assert_eq!(
            text("automation.datasource.form.ruleCountValue"),
            Some("{count} 条")
        );
    }

    /// ② 空输入 ⇒ `NotDelivered` 空态（fail-closed，不是空列表）。
    #[test]
    fn empty_input_yields_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None, None), State::NotDelivered);
        assert_eq!(resolve(Some(false), Some(true)), State::NotDelivered);
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    /// ⑤ 状态分支覆盖：每条分支一条断言。
    #[test]
    fn resolve_covers_every_branch() {
        assert_eq!(resolve(Some(true), None), State::Create);
        assert_eq!(resolve(Some(true), Some(true)), State::Create);
        assert_eq!(resolve(Some(true), Some(false)), State::Unauthorized);
        assert_eq!(resolve(None, Some(true)), State::NotDelivered);
    }

    /// 浮层名非空（house style：TITLE 为本卡视图名）。
    #[test]
    fn title_is_nonempty() {
        assert!(!TITLE.is_empty());
    }
}
