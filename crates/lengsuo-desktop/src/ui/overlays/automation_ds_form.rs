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
    ("automation.datasource.form.connectorEmpty", "请选择一个已授权的连接器"),
    ("automation.datasource.form.connectorHint", "选择一个连接器作为数据同步来源。"),
    ("automation.datasource.form.connectorUnauthorizedHint", "需先在连接器配置中完成授权"),
    ("automation.datasource.form.create", "创建"),
    ("automation.datasource.form.disable", "禁用"),
    ("automation.datasource.form.enable", "启用"),
    ("automation.datasource.form.filterEmptyHint", "不填则同步全部 Issue"),
    ("automation.datasource.form.filterInputLabel", "过滤条件（可选）"),
    ("automation.datasource.form.filterText", "同步说明"),
    (
        "automation.datasource.form.filterTextHint",
        "用一句话描述你希望同步哪些内容，系统会据此生成过滤规则。",
    ),
    (
        "automation.datasource.form.filterTextPlaceholder",
        "只同步符合条件的数据 · 不填则默认同步最多500条待办数据",
    ),
    ("automation.datasource.form.generateAndPreview", "✨ 生成并预览"),
    ("automation.datasource.form.generateEmptyHint", "无法从描述中提取有效的过滤条件"),
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
    ("automation.datasource.form.repoHint", "当前阶段先使用 mock 仓库列表。"),
    ("automation.datasource.form.repoPlaceholder", "选择要同步的仓库"),
    ("automation.datasource.form.ruleCount", "规则数"),
    ("automation.datasource.form.ruleCountValue", "{count} 条"),
    ("automation.datasource.form.ruleField", "字段"),
    ("automation.datasource.form.ruleFieldPlaceholder", "如：labels"),
    ("automation.datasource.form.ruleOperator", "条件"),
    ("automation.datasource.form.ruleOperatorPlaceholder", "如：contains"),
    ("automation.datasource.form.rules", "过滤规则"),
    (
        "automation.datasource.form.rulesEmpty",
        "还没有过滤规则，可先填写同步说明并生成。",
    ),
    ("automation.datasource.form.ruleValue", "值"),
    ("automation.datasource.form.ruleValuePlaceholder", "如：ai-ready"),
    ("automation.datasource.form.save", "保存"),
    ("automation.datasource.form.saveAndEnable", "保存并启用"),
    ("automation.datasource.form.saving", "保存中..."),
    ("automation.datasource.form.selected", "已选择"),
    ("automation.datasource.form.selectedConnector", "连接器"),
    ("automation.datasource.form.selectedRepo", "目标仓库"),
    ("automation.datasource.form.testButton", "测试同步"),
    ("automation.datasource.form.testComingSoon", "规则测试功能即将上线"),
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
    ("automation.datasource.form.testResultTitle", "最近一次 mock 测试结果"),
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

// ── GPUI 层 ─────────────────────────────────────────────────────────

/// 主区/浮层呈现入口（诚实占位：等跨线契约就绪后换源）。
pub fn main_area(theme: &Theme) -> AnyElement {
    let state = resolve(None, None);
    let title = SharedString::from(format!(
        "{}（{}）",
        TITLE,
        match state {
            State::Create | State::Edit => text("automation.datasource.form.titleCreate"),
            _ => Some(NOT_DELIVERED),
        }
        .unwrap_or(NOT_DELIVERED)
    ));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground)
        .child(title)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{resolve, text, NS_PREFIX, State, TEXTS, TITLE, NOT_DELIVERED};

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
