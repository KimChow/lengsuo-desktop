//! 协作计划·子任务（波次 9，卡 `ui-overlay-collab-plan-subtodo`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（无 GPUI 依赖）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.plan.subtodo`
//! （摘录 12 键见 [`TEXTS`]；全表其余 13 键原文到位前不落键、不自编中文充数，
//! 视图态占位文案在 GPUI 层单独登记）。带 `{占位符}` 的键由纯函数替换占位符，不改字。
#![cfg(feature = "ui")]

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "协作计划·子任务";
/// 命名空间前缀（文案键值表见 [`TEXTS`]）。
pub const NS_PREFIX: &str = "collab.plan.subtodo";

/// 命名空间文案表（key → 中文，逐字，摘录 12 键）。
pub const TEXTS: &[(&str, &str)] = &[
    ("collab.plan.subtodo.addEntry", "添加子待办"),
    ("collab.plan.subtodo.config.fields", "展示字段"),
    ("collab.plan.subtodo.config.sort", "排序"),
    ("collab.plan.subtodo.configAria", "配置子待办展示"),
    (
        "collab.plan.subtodo.create.attachmentsExist",
        "所选附件已存在",
    ),
    ("collab.plan.subtodo.create.cancel", "取消"),
    (
        "collab.plan.subtodo.create.dateMenuAria",
        "设置开始/截止日期",
    ),
    (
        "collab.plan.subtodo.create.depthLimit",
        "子待办最多支持 {max} 层",
    ),
    ("collab.plan.subtodo.create.descPlaceholder", "添加描述"),
    ("collab.plan.subtodo.create.submit", "提交"),
    ("collab.plan.subtodo.create.submitting", "提交中"),
    ("collab.plan.subtodo.create.titlePlaceholder", "输入标题"),
];

/// 按键取文案（缺键 = 程序缺陷，fail-fast）。
pub fn zh(key: &str) -> &'static str {
    TEXTS
        .iter()
        .find(|(k, _)| *k == key)
        .expect("collab.plan.subtodo 文案键缺失")
        .1
}

// ===== 纯数据层（无 GPUI 依赖）=====

/// 单条子待办（仅由调用方/测试构造；⛔ 不伪造服务端条目）。
#[derive(Clone, Debug, PartialEq)]
pub struct SubtodoItem {
    pub title: String,
    pub done: bool,
    /// 嵌套层级，1 起。
    pub depth: u32,
}

/// 视图态状态机：加载中 / 失败（fail-closed）/ 空态 / 正常。
///
/// 三态契约：未拉取过 ≠ 空集 —— 默认 [`SubtodoState::Loading`]，
/// 只有确认拉取成功且结果为空才进 [`SubtodoState::Empty`]，
/// 拉取失败必须以 [`SubtodoState::Failed`] 可见呈现，不退化成空列表。
#[derive(Clone, Debug, Default, PartialEq)]
pub enum SubtodoState {
    /// 未拉取过（诚实默认态，≠ 空态）。
    #[default]
    Loading,
    /// 拉取失败（fail-closed）。
    Failed,
    /// 已拉取且确认为空（≠ 未拉取过）。
    Empty,
    /// 已拉取且有条目。
    Ready(Vec<SubtodoItem>),
}

impl SubtodoState {
    /// 拉取结果的纯函数映射：`None` = 未拉取过（⇒ Loading，⛔ 不是空态）。
    pub fn from_fetch(result: Option<Result<Vec<SubtodoItem>, ()>>) -> Self {
        match result {
            None => Self::Loading,
            Some(Err(())) => Self::Failed,
            Some(Ok(items)) if items.is_empty() => Self::Empty,
            Some(Ok(items)) => Self::Ready(items),
        }
    }
}

/// 创建表单态（值由调用方/测试构造；⛔ 不伪造提交结果/层级上限）。
#[derive(Clone, Debug, PartialEq)]
pub struct CreateForm {
    pub title: String,
    pub desc: String,
    pub submitting: bool,
    /// 所选附件已存在（`create.attachmentsExist` 的呈现条件）。
    pub attachments_exist: bool,
    /// 待创建条目的层级（父条目 depth + 1）。
    pub next_depth: u32,
    /// 层级上限；`0` = 未下发 ⇒ 不呈现上限说明。
    pub max_depth: u32,
}

impl Default for CreateForm {
    fn default() -> Self {
        Self {
            title: String::new(),
            desc: String::new(),
            submitting: false,
            attachments_exist: false,
            next_depth: 1,
            max_depth: 0,
        }
    }
}

impl CreateForm {
    /// `create.depthLimit` 模板「子待办最多支持 {max} 层」，逐字替换占位符。
    pub fn depth_limit_caption(&self) -> Option<String> {
        (self.max_depth > 0 && self.next_depth > self.max_depth)
            .then(|| depth_limit_text(self.max_depth))
    }

    /// `create.attachmentsExist` 警示行的呈现条件。
    pub fn shows_attachments_exist(&self) -> bool {
        self.attachments_exist
    }
}

/// `create.depthLimit` 的 `{max}` 占位替换（不改字）。
pub fn depth_limit_text(max: u32) -> String {
    zh("collab.plan.subtodo.create.depthLimit").replace("{max}", &max.to_string())
}

/// 提交按钮文案（`create.submit` / `create.submitting`，逐字）。
pub fn submit_label(submitting: bool) -> &'static str {
    if submitting {
        zh("collab.plan.subtodo.create.submitting")
    } else {
        zh("collab.plan.subtodo.create.submit")
    }
}

/// 各状态主体内容节点数（与 GPUI 层 `render_state` 的主体直属内容节点一一对应；
/// Loading/Failed 各计 1 个说明节点，⛔ 不是 0 个节点的空列表）。
pub fn content_node_count(state: &SubtodoState) -> usize {
    match state {
        SubtodoState::Loading | SubtodoState::Failed => 1,
        SubtodoState::Empty => 2,
        SubtodoState::Ready(items) => items.len() + 2,
    }
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{zh, SubtodoState, NS_PREFIX, TEXTS, TITLE};

    #[test]
    fn title_and_ns_verbatim() {
        assert_eq!(TITLE, "协作计划·子任务");
        assert_eq!(NS_PREFIX, "collab.plan.subtodo");
    }

    #[test]
    fn texts_cover_namespace() {
        assert_eq!(TEXTS.len(), 12, "摘录键数");
        for (k, v) in TEXTS {
            assert!(k.starts_with(NS_PREFIX), "越界键 {k}");
            assert!(!v.is_empty(), "空文案 {k}");
        }
    }

    #[test]
    fn missing_key_fails_fast() {
        let gone = std::panic::catch_unwind(|| zh("collab.plan.subtodo.notInTable"));
        assert!(gone.is_err(), "缺键必须 fail-fast");
    }

    #[test]
    fn state_defaults_to_loading() {
        assert_eq!(SubtodoState::default(), SubtodoState::Loading);
    }

    #[test]
    fn unfetched_is_not_empty() {
        assert_eq!(SubtodoState::from_fetch(None), SubtodoState::Loading);
        assert_eq!(
            SubtodoState::from_fetch(Some(Ok(Vec::new()))),
            SubtodoState::Empty
        );
    }
}
