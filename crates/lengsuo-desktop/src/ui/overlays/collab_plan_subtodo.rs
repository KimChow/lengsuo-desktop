//! 协作计划·子任务（波次 9，卡 `ui-overlay-collab-plan-subtodo`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（无 GPUI 依赖）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.plan.subtodo`
//! （摘录 12 键见 [`TEXTS`]；全表其余 13 键原文到位前不落键、不自编中文充数，
//! 视图态占位文案在 GPUI 层单独登记）。带 `{占位符}` 的键由纯函数替换占位符，不改字。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

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

// ===== GPUI 层 =====
/// 加载中占位说明（全表 load* 键原文到位前临时使用；到位后随键替换）。
const LOADING_SCAFFOLD: &str = "加载中...";
/// 加载失败占位说明（承自脚手架先例；全表键到位后随键替换）。
const LOAD_FAILED_SCAFFOLD: &str = "加载失败";

/// 主区/浮层呈现入口。数据源未接线 ⇒ 保持 Loading 如实呈现（fail-closed）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render_state(&SubtodoState::default(), &CreateForm::default(), theme)
}

/// 按状态渲染：标题行 + 分支主体；主体直属内容节点数与 [`content_node_count`]
/// 一一对应（Loading/Failed 各 1 节点，⛔ 不是空列表）。颜色一律取 `theme.*`。
fn render_state(state: &SubtodoState, form: &CreateForm, theme: &Theme) -> AnyElement {
    let header = div()
        .flex()
        .items_center()
        .gap_2()
        .child(div().text_sm().text_color(theme.foreground).child(TITLE))
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.subtodo.config.fields")),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.subtodo.config.sort")),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.subtodo.configAria")),
        );
    let body = match state {
        SubtodoState::Loading => caption(LOADING_SCAFFOLD, theme),
        SubtodoState::Failed => caption(LOAD_FAILED_SCAFFOLD, theme),
        SubtodoState::Empty => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(add_entry_row(theme))
            .child(create_form_node(form, theme))
            .into_any_element(),
        SubtodoState::Ready(items) => {
            let mut col = div().flex().flex_col().gap_2();
            for item in items {
                col = col.child(item_row(item, theme));
            }
            col.child(add_entry_row(theme))
                .child(create_form_node(form, theme))
                .into_any_element()
        }
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(header)
        .child(body)
        .into_any_element()
}

/// 说明行（弱化色；加载中/失败必须可见，fail-closed）。
fn caption(body: &str, theme: &Theme) -> AnyElement {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(body.to_string()))
        .into_any_element()
}

/// 添加入口行（`addEntry`，逐字；触达目标 ≥48dp）。
fn add_entry_row(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .h_12()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(zh("collab.plan.subtodo.addEntry")),
        )
        .into_any_element()
}

/// 单条子待办行：层级缩进（层级 1 起，每深一级缩进一格）；完成态弱化色。
fn item_row(item: &SubtodoItem, theme: &Theme) -> AnyElement {
    let title_color = if item.done {
        theme.muted_foreground
    } else {
        theme.foreground
    };
    let row = match item.depth.min(3) {
        1 => div().flex().items_center().h_12().gap_2(),
        2 => div().flex().items_center().h_12().gap_2().pl_4(),
        _ => div().flex().items_center().h_12().gap_2().pl_8(),
    };
    row.child(
        div()
            .text_sm()
            .text_color(title_color)
            .child(SharedString::from(item.title.clone())),
    )
    .into_any_element()
}

/// 创建表单骨架（文案逐字取自 `create.*`；⛔ 不携带任何伪造值）。
fn create_form_node(form: &CreateForm, theme: &Theme) -> AnyElement {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_1()
        .border_1()
        .border_color(theme.border)
        .p_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.subtodo.create.titlePlaceholder")),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.subtodo.create.descPlaceholder")),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(zh("collab.plan.subtodo.create.dateMenuAria")),
        );
    if form.shows_attachments_exist() {
        col = col.child(
            div()
                .text_xs()
                .text_color(theme.foreground)
                .child(zh("collab.plan.subtodo.create.attachmentsExist")),
        );
    }
    if let Some(text) = form.depth_limit_caption() {
        col = col.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(text)),
        );
    }
    col.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_12()
                    .px_3()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(submit_label(form.submitting)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_12()
                    .px_3()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(zh("collab.plan.subtodo.create.cancel")),
            ),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        main_area, render_state, zh, CreateForm, SubtodoItem, SubtodoState, NS_PREFIX, TEXTS, TITLE,
    };
    use gpui_kit::component::Theme;

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

    /// 渲染冒烟：四分支只构造元素树、不 panic；默认入口 fail-closed 呈现 Loading。
    #[test]
    fn render_branches_smoke() {
        let theme = Theme::default();
        let _ = main_area(&theme);
        let items = vec![SubtodoItem {
            title: zh("collab.plan.subtodo.addEntry").to_string(),
            done: true,
            depth: 2,
        }];
        let ready = SubtodoState::from_fetch(Some(Ok(items)));
        let _ = render_state(&ready, &CreateForm::default(), &theme);
        let _ = render_state(&SubtodoState::Loading, &CreateForm::default(), &theme);
        let _ = render_state(&SubtodoState::Failed, &CreateForm::default(), &theme);
        let _ = render_state(&SubtodoState::Empty, &CreateForm::default(), &theme);
    }
}
