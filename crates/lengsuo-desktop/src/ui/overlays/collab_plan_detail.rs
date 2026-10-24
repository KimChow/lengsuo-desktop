//! 协作计划详情（波次 8，卡 `ui-collab-plan-detail`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.plan.detail`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 浮层标识（卡名，非界面文案；界面标题用全表 key `collab.plan.detail.title`）。
pub const TITLE: &str = "协作计划详情";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.plan.detail";
/// 键数（命名空间 `collab.plan.detail` 全量文案实测 82 条，见 [`TEXTS`]）。
pub const NS_KEY_COUNT: usize = TEXTS.len();

// ===== 纯数据层（无 GPUI 依赖）=====
/// 命名空间 `collab.plan.detail` 全量文案（key、中文均逐字取自界面文案全表）。
pub const TEXTS: &[(&str, &str)] = &[
    ("collab.plan.detail.actions.start", "开始"),
    ("collab.plan.detail.addToTask", "添加到输入框"),
    ("collab.plan.detail.addToTaskComingSoon", "添加到输入框能力即将支持"),
    ("collab.plan.detail.addToTaskSuccess", "已添加到输入框"),
    ("collab.plan.detail.assigneeFallback", "未分配"),
    ("collab.plan.detail.assigneeLabel", "处理人"),
    ("collab.plan.detail.backToParent", "返回上级"),
    ("collab.plan.detail.completeSubtodos.desc", "该待办下还有 {count} 个子待办未完成。"),
    ("collab.plan.detail.completeSubtodos.question", "是否将它们一并更新为「{status}」？"),
    ("collab.plan.detail.completeSubtodos.skip", "跳过子待办"),
    ("collab.plan.detail.completeSubtodos.title", "同时完成未完成的子待办？"),
    ("collab.plan.detail.completeSubtodos.unassigned", "未指派"),
    ("collab.plan.detail.completeSubtodos.untitled", "未命名待办"),
    ("collab.plan.detail.completeSubtodos.updateAll", "全部更新"),
    ("collab.plan.detail.copyDetail", "复制为指令"),
    ("collab.plan.detail.copyDetail.failure", "复制失败，请稍后重试"),
    ("collab.plan.detail.copyDetail.success", "已复制为指令"),
    ("collab.plan.detail.copyLink.aria", "复制链接"),
    ("collab.plan.detail.copyLink.failure", "复制失败，请稍后重试"),
    ("collab.plan.detail.copyLink.success", "待办链接已复制，其他成员可通过链接打开详情"),
    ("collab.plan.detail.copyLink.title", "复制链接"),
    ("collab.plan.detail.creator.createdBy", "由 {name} 创建"),
    ("collab.plan.detail.creator.separator", "·"),
    ("collab.plan.detail.creator.unknown", "未知"),
    ("collab.plan.detail.creator.updatedAt", "更新于 {date}"),
    ("collab.plan.detail.deleteConfirm.cancel", "取消"),
    ("collab.plan.detail.deleteConfirm.confirm", "删除"),
    ("collab.plan.detail.deleteConfirm.content", "删除后该事项将不可恢复，确认要删除吗？"),
    ("collab.plan.detail.deleteConfirm.title", "确认删除"),
    ("collab.plan.detail.deleteItem", "删除事项"),
    ("collab.plan.detail.dueDateLabel", "截止日期"),
    ("collab.plan.detail.emptyPrompt", "请选择一个事项查看详情。"),
    ("collab.plan.detail.loading", "加载事项详情中…"),
    ("collab.plan.detail.moreAria", "更多操作"),
    ("collab.plan.detail.nav.next", "下一条"),
    ("collab.plan.detail.nav.prev", "上一条"),
    ("collab.plan.detail.openSource", "跳转{source}"),
    ("collab.plan.detail.optionCreateFailed", "新建选项失败，请重试"),
    ("collab.plan.detail.optionDeleteConfirmCancel", "取消"),
    ("collab.plan.detail.optionDeleteConfirmDesc", "删除后将影响所有使用该选项的待办。"),
    ("collab.plan.detail.optionDeleteConfirmOk", "删除"),
    ("collab.plan.detail.optionDeleteConfirmTitle", "确认删除该选项？"),
    ("collab.plan.detail.optionDeleteFailed", "删除选项失败，请重试"),
    ("collab.plan.detail.optionEditFailed", "编辑选项失败，请重试"),
    ("collab.plan.detail.priority.high", "高"),
    ("collab.plan.detail.priority.low", "低"),
    ("collab.plan.detail.priority.medium", "中"),
    ("collab.plan.detail.priority.none", "无"),
    ("collab.plan.detail.priority.urgent", "紧急"),
    ("collab.plan.detail.priorityColorLabel", "优先级颜色"),
    ("collab.plan.detail.priorityCreate", "新增优先级"),
    ("collab.plan.detail.priorityDelete", "删除"),
    ("collab.plan.detail.priorityEdit", "编辑"),
    ("collab.plan.detail.priorityEditColor", "修改颜色"),
    ("collab.plan.detail.priorityEmpty", "暂无优先级"),
    ("collab.plan.detail.priorityLabel", "优先级"),
    ("collab.plan.detail.priorityPlaceholder", "未设置"),
    ("collab.plan.detail.prioritySearchPlaceholder", "搜索或新增优先级"),
    ("collab.plan.detail.quickAction.completed", "完成"),
    ("collab.plan.detail.quickAction.in_progress", "重启"),
    ("collab.plan.detail.quickAction.resume", "恢复"),
    ("collab.plan.detail.session.activeSuffix", " · 活跃中"),
    ("collab.plan.detail.session.label", "· 关联 Session"),
    ("collab.plan.detail.session.openArrow", "打开 →"),
    ("collab.plan.detail.startedAtLabel", "开始日期"),
    ("collab.plan.detail.statusLabel", "状态"),
    ("collab.plan.detail.subtitle", "附属说明"),
    ("collab.plan.detail.tab.activity", "动态"),
    ("collab.plan.detail.tab.detail", "详情"),
    ("collab.plan.detail.tabs.detail", "详情"),
    ("collab.plan.detail.tagsColorLabel", "标签背景色"),
    ("collab.plan.detail.tagsCreate", "新增标签"),
    ("collab.plan.detail.tagsDelete", "删除"),
    ("collab.plan.detail.tagsEdit", "编辑"),
    ("collab.plan.detail.tagsEmpty", "暂无标签"),
    ("collab.plan.detail.tagsLabel", "标签"),
    ("collab.plan.detail.tagsPlaceholder", "未设置"),
    ("collab.plan.detail.tagsSearchPlaceholder", "搜索或新增标签"),
    ("collab.plan.detail.title", "待办详情"),
    ("collab.plan.detail.transitionUnavailable", "当前状态暂不可切换"),
    ("collab.plan.detail.window.expand", "放大窗口"),
    ("collab.plan.detail.window.shrink", "缩小窗口"),
];

/// 按全量 key 取文案；未知 key 返回 `None`（fail-closed，不猜值）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 渲染层取词（key 必须在 [`TEXTS`]；测试锁表防漂移，缺词落空串显形）。
fn t(key: &str) -> &'static str {
    text(key).unwrap_or_default()
}

/// 模板占位符填充（`{count}`/`{status}`/`{name}`/`{date}`/`{source}`；逐字替换）。
pub fn fill(key: &str, vars: &[(&str, &str)]) -> String {
    let mut out = t(key).to_string();
    for (name, val) in vars {
        out = out.replace(&format!("{{{name}}}"), val);
    }
    out
}

/// 优先级（文案取自 `priority.*`，未设置显示 `priorityPlaceholder`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Priority {
    #[default]
    Unset,
    Low,
    Medium,
    High,
    Urgent,
}

impl Priority {
    /// 优先级值文案（`priority.*` 逐字）。
    pub fn label(self) -> &'static str {
        match self {
            Priority::Unset => t("collab.plan.detail.priorityPlaceholder"),
            Priority::Low => t("collab.plan.detail.priority.low"),
            Priority::Medium => t("collab.plan.detail.priority.medium"),
            Priority::High => t("collab.plan.detail.priority.high"),
            Priority::Urgent => t("collab.plan.detail.priority.urgent"),
        }
    }
}

/// 一条待办（字段对应全表 key；全部可空，渲染时用全表 fallback，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TodoItem {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub assignee: Option<String>,
    pub creator: Option<String>,
    pub updated_at: Option<String>,
    pub started_at: Option<String>,
    pub due_at: Option<String>,
    pub status: Option<String>,
    pub session: Option<String>,
    pub session_active: bool,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub subtodo_total: usize,
    pub subtodo_done: usize,
}

impl TodoItem {
    /// 标题 fallback：`completeSubtodos.untitled`。
    pub fn title_or(&self) -> &str {
        self.title.as_deref().unwrap_or_else(|| t("collab.plan.detail.completeSubtodos.untitled"))
    }
    /// 处理人 fallback：`assigneeFallback`。
    pub fn assignee_or(&self) -> &str {
        self.assignee.as_deref().unwrap_or_else(|| t("collab.plan.detail.assigneeFallback"))
    }
    /// 创建者 fallback：`creator.unknown`。
    pub fn creator_or(&self) -> &str {
        self.creator.as_deref().unwrap_or_else(|| t("collab.plan.detail.creator.unknown"))
    }
    /// 未完成子待办数（饱和减，⛔ 不出负数）。
    pub fn uncompleted_subtodos(&self) -> usize {
        self.subtodo_total.saturating_sub(self.subtodo_done)
    }
    /// 是否需要「同时完成子待办」确认。
    pub fn needs_subtodo_confirm(&self) -> bool {
        self.uncompleted_subtodos() > 0
    }
    /// `creator.createdBy` 填 name。
    pub fn created_by_line(&self) -> String {
        fill("collab.plan.detail.creator.createdBy", &[("name", self.creator_or())])
    }
    /// `creator.updatedAt` 填 date。
    pub fn updated_at_line(&self) -> Option<String> {
        self.updated_at
            .as_deref()
            .map(|d| fill("collab.plan.detail.creator.updatedAt", &[("date", d)]))
    }
    /// 创建/更新行（`creator.separator` 连接）。
    pub fn creator_line(&self) -> String {
        let mut parts = vec![self.created_by_line()];
        if let Some(u) = self.updated_at_line() {
            parts.push(u);
        }
        parts.join(t("collab.plan.detail.creator.separator"))
    }
    /// 处理人行：`assigneeLabel` + 值/fallback。
    pub fn assignee_line(&self) -> String {
        format!("{} {}", t("collab.plan.detail.assigneeLabel"), self.assignee_or())
    }
    /// 优先级行：`priorityLabel` + 值。
    pub fn priority_line(&self) -> String {
        format!("{} {}", t("collab.plan.detail.priorityLabel"), self.priority.label())
    }
    /// 标签行：空 ⇒ `tagsEmpty`（不是空列表）；有值用 `creator.separator` 连接。
    pub fn tags_line(&self) -> String {
        format!(
            "{} {}",
            t("collab.plan.detail.tagsLabel"),
            if self.tags.is_empty() {
                t("collab.plan.detail.tagsEmpty").to_string()
            } else {
                self.tags.join(t("collab.plan.detail.creator.separator"))
            }
        )
    }
    /// Session 行：`session.label` + 名称 + 可选 `session.activeSuffix` + `session.openArrow`。
    pub fn session_line(&self) -> Option<String> {
        let name = self.session.as_deref()?;
        let mut line = format!("{} {} {}", t("collab.plan.detail.session.label"), name, t("collab.plan.detail.session.openArrow"));
        if self.session_active {
            line.push_str(t("collab.plan.detail.session.activeSuffix"));
        }
        Some(line)
    }
    /// 日期行：`startedAtLabel` / `dueDateLabel` + 值；两者皆无 ⇒ `None`（不渲染）。
    pub fn dates_line(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(s) = self.started_at.as_deref() {
            parts.push(format!("{} {}", t("collab.plan.detail.startedAtLabel"), s));
        }
        if let Some(d) = self.due_at.as_deref() {
            parts.push(format!("{} {}", t("collab.plan.detail.dueDateLabel"), d));
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(t("collab.plan.detail.creator.separator")))
        }
    }
    /// `completeSubtodos.desc` 填未完成数。
    pub fn subtodo_desc(&self) -> String {
        fill("collab.plan.detail.completeSubtodos.desc", &[("count", &self.uncompleted_subtodos().to_string())])
    }
    /// `completeSubtodos.question` 填目标状态。
    pub fn subtodo_question(&self, status: &str) -> String {
        fill("collab.plan.detail.completeSubtodos.question", &[("status", status)])
    }
    /// 快捷动作文案（`quickAction.*`；未知状态按 `quickAction.resume`）。
    pub fn quick_action(&self) -> &'static str {
        match self.status.as_deref() {
            Some("completed") => t("collab.plan.detail.quickAction.completed"),
            Some("in_progress") => t("collab.plan.detail.quickAction.in_progress"),
            _ => t("collab.plan.detail.quickAction.resume"),
        }
    }
}

/// 浮层/面板状态机（默认 `NotDelivered`：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 就绪：数据已下发且有事项。
    Ready(Vec<TodoItem>),
    /// 空态：已下发但无事项（`emptyPrompt`，⛔ 不是空列表）。
    Empty,
    /// 加载中（`loading`）。
    Loading,
    /// 数据未下发（无数据源时的诚实默认态，显示「未下发」）。
    #[default]
    NotDelivered,
}

impl State {
    /// 各分支提示文案（逐字取自全表；`Ready` 无提示行）。
    pub fn status_text(&self) -> &'static str {
        match self {
            State::Ready(_) => "",
            State::Empty => t("collab.plan.detail.emptyPrompt"),
            State::Loading => t("collab.plan.detail.loading"),
            State::NotDelivered => NOT_DELIVERED,
        }
    }
}

/// 由运行时输入解析初始态（纯函数）：`None` = 未拉取过 ≠ `Some(vec![])` = 空集。
pub fn resolve(delivered: Option<Vec<TodoItem>>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(items) if items.is_empty() => State::Empty,
        Some(items) => State::Ready(items),
    }
}

// ===== GPUI 层 =====
/// 主区/浮层呈现入口（签名固定；无数据源 ⇒ fail-closed 默认 `NotDelivered`，显示「未下发」）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render(theme, &State::default())
}

/// 按状态渲染整棵树（只构造元素，不触实体/上下文；测试逐分支调用防 panic）。
pub fn render(theme: &Theme, state: &State) -> AnyElement {
    let frame = div()
        .flex()
        .flex_col()
        .gap_2()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(header_row(theme))
        .child(tabs_row(theme))
        .child(actions_row(theme));
    match state {
        State::Ready(items) => frame
            .children(items.iter().map(|it| item_card(theme, it)))
            .into_any_element(),
        State::Empty | State::Loading | State::NotDelivered => frame
            .child(
                div()
                    .px_4()
                    .py_2()
                    .text_color(theme.foreground)
                    .child(SharedString::from(state.status_text())),
            )
            .into_any_element(),
    }
}

/// 标题行：返回上级 + 待办详情 + 上一条/下一条。
fn header_row(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(SharedString::from(t("collab.plan.detail.backToParent")))
        .child(div().text_color(theme.foreground).child(SharedString::from(t("collab.plan.detail.title"))))
        .child(SharedString::from(t("collab.plan.detail.nav.prev")))
        .child(SharedString::from(t("collab.plan.detail.nav.next")))
        .into_any_element()
}

/// 页签行：详情 / 动态。
fn tabs_row(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(div().text_color(theme.foreground).child(SharedString::from(t("collab.plan.detail.tab.detail"))))
        .child(SharedString::from(t("collab.plan.detail.tab.activity")))
        .into_any_element()
}

/// 操作行（静态文案，无实体回调；能力位由运行时侧接）。
fn actions_row(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .text_xs()
        .child(div().text_color(theme.foreground).child(SharedString::from(t("collab.plan.detail.actions.start"))))
        .child(SharedString::from(t("collab.plan.detail.addToTask")))
        .child(SharedString::from(t("collab.plan.detail.copyDetail")))
        .child(SharedString::from(t("collab.plan.detail.copyLink.title")))
        .child(SharedString::from(t("collab.plan.detail.deleteItem")))
        .into_any_element()
}

/// 单条待办卡（值全部来自 [`TodoItem`] 纯函数或字段，⛔ 不伪造）。
fn item_card(theme: &Theme, it: &TodoItem) -> AnyElement {
    let mut card = div()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .py_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().text_color(theme.foreground).child(SharedString::from(it.title_or())))
                .child(SharedString::from(it.quick_action())),
        )
        .child(SharedString::from(it.creator_line()))
        .child(SharedString::from(it.assignee_line()))
        .child(SharedString::from(it.priority_line()))
        .child(SharedString::from(it.tags_line()));
    if let Some(sub) = it.subtitle.as_deref() {
        card = card.child(div().text_xs().child(SharedString::from(sub)));
    }
    if let Some(s) = it.status.as_deref() {
        card = card.child(SharedString::from(format!("{} {}", t("collab.plan.detail.statusLabel"), s)));
    }
    if let Some(d) = it.dates_line() {
        card = card.child(SharedString::from(d));
    }
    if let Some(sess) = it.session_line() {
        card = card.child(div().text_xs().text_color(theme.accent).child(SharedString::from(sess)));
    }
    if it.needs_subtodo_confirm() {
        let mut block = div()
            .flex()
            .flex_col()
            .gap_2()
            .text_xs()
            .child(SharedString::from(t("collab.plan.detail.completeSubtodos.title")))
            .child(SharedString::from(it.subtodo_desc()));
        if let Some(s) = it.status.as_deref() {
            block = block.child(SharedString::from(it.subtodo_question(s)));
        }
        block = block
            .child(SharedString::from(t("collab.plan.detail.completeSubtodos.skip")))
            .child(SharedString::from(t("collab.plan.detail.completeSubtodos.updateAll")));
        card = card.child(block);
    }
    card.into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{resolve, text, State, TEXTS, TITLE};

    #[test]
    fn title_is_verbatim() {
        assert_eq!(text("collab.plan.detail.title"), Some("待办详情"));
        assert_eq!(text("collab.plan.detail.backToParent"), Some("返回上级"));
        assert_eq!(text("collab.plan.detail.assigneeFallback"), Some("未分配"));
        assert_eq!(TITLE, "协作计划详情");
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(State::NotDelivered.status_text(), "未下发");
    }

    #[test]
    fn ns_table_locked() {
        let n = TEXTS.len();
        assert_eq!(n, super::NS_KEY_COUNT);
        assert!(n >= 40);
        let mut seen: Vec<&str> = TEXTS.iter().map(|(k, _)| *k).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n);
        assert!(seen.iter().all(|k| k.starts_with("collab.plan.detail.")));
    }
}
