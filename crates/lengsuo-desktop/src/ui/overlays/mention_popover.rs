//! @提及弹层（波次 8，卡 `ui-overlay-mention-popover`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间。
//! 数据源在运行时侧：无数据时如实显示「未下发」。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 视图标题。
pub const TITLE: &str = "@提及弹层";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 助理页签名。
pub const COLLEAGUE_NAME: &str = "助理";
/// 助理页描述。
pub const COLLEAGUE_DESCRIPTION: &str = "选择助理协同推进任务";
/// 助理搜索占位。
pub const COLLEAGUE_PLACEHOLDER: &str = "搜索助理";
/// 助理搜索空态。
pub const COLLEAGUE_SEARCH_EMPTY: &str = "未找到助理";
/// 忙碌状态标签。
pub const STATUS_BUSY: &str = "工作中";
/// 空闲状态标签。
pub const STATUS_IDLE: &str = "空闲";
/// 未就绪状态标签。
pub const STATUS_UNAVAILABLE: &str = "未就绪";
/// 应用页签名。
pub const APPLICATION_NAME: &str = "应用";
/// 文件页描述。
pub const FILE_DESCRIPTION: &str = "选择文件和文件夹";
/// 文件页初始空态。
pub const FILE_EMPTY_INITIAL: &str = "当前对话中暂无文件";
/// 文件搜索占位。
pub const FILE_PLACEHOLDER: &str = "添加文件作为回答背景";
/// 文件搜索空态。
pub const FILE_SEARCH_EMPTY: &str = "无搜索结果";
/// 文件分组标题。
pub const FILE_SESSION_GROUP: &str = "对话中的文件";

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    Loading,
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 助理忙闲状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Busy,
    Idle,
    Unavailable,
}

impl Availability {
    /// 状态标签（逐字取自文案全表）。
    pub fn label(self) -> &'static str {
        match self {
            Availability::Busy => STATUS_BUSY,
            Availability::Idle => STATUS_IDLE,
            Availability::Unavailable => STATUS_UNAVAILABLE,
        }
    }
}

/// @提及弹层的三个页签。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Category {
    #[default]
    Colleague,
    Application,
    File,
}

impl Category {
    /// 全部页签（渲染顺序）。
    pub const ALL: [Category; 3] = [Category::Colleague, Category::Application, Category::File];

    /// 页签名。
    pub fn label(self) -> &'static str {
        match self {
            Category::Colleague => COLLEAGUE_NAME,
            Category::Application => APPLICATION_NAME,
            Category::File => FILE_SESSION_GROUP,
        }
    }

    /// 页签描述（应用页无下发文案，如实为 `None`，⛔ 不自编）。
    pub fn description(self) -> Option<&'static str> {
        match self {
            Category::Colleague => Some(COLLEAGUE_DESCRIPTION),
            Category::Application => None,
            Category::File => Some(FILE_DESCRIPTION),
        }
    }

    /// 搜索占位（应用页无下发文案，如实为 `None`）。
    pub fn placeholder(self) -> Option<&'static str> {
        match self {
            Category::Colleague => Some(COLLEAGUE_PLACEHOLDER),
            Category::Application => None,
            Category::File => Some(FILE_PLACEHOLDER),
        }
    }
}

/// 一条助理条目（运行时由数据源下发；本模块不伪造）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColleagueEntry {
    pub name: String,
    pub availability: Availability,
}

/// 浮层纯数据模型：视图是它的纯函数。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MentionPopover {
    pub state: State,
    pub category: Category,
    pub query: String,
}

impl MentionPopover {
    /// 无数据源时的诚实默认模型（fail-closed）。
    pub fn not_delivered() -> Self {
        Self::default()
    }

    /// 内容区是否应显示「未下发」（一切非 `Normal` 状态都不得渲染条目）。
    pub fn is_not_delivered(&self) -> bool {
        self.state != State::Normal
    }

    /// 按搜索词过滤助理（大小写不敏感；空词匹配全部）。
    pub fn colleague_matches<'a>(&self, entries: &'a [ColleagueEntry]) -> Vec<&'a ColleagueEntry> {
        let needle = self.query.trim().to_lowercase();
        entries
            .iter()
            .filter(|entry| needle.is_empty() || entry.name.to_lowercase().contains(&needle))
            .collect()
    }

    /// 列表为空时要显示的空态文案；`None` = 该页签无下发空态文案。
    pub fn empty_label(&self) -> Option<&'static str> {
        match self.category {
            Category::Colleague => Some(COLLEAGUE_SEARCH_EMPTY),
            Category::Application => None,
            Category::File => {
                if self.query.trim().is_empty() {
                    Some(FILE_EMPTY_INITIAL)
                } else {
                    Some(FILE_SEARCH_EMPTY)
                }
            }
        }
    }
}

// ---------- GPUI 层 ----------

/// 主区渲染：默认模型（数据未下发 ⇒ fail-closed 显示「未下发」）。
pub fn main_area(theme: &Theme) -> Div {
    popover_area(theme, &MentionPopover::default(), &[])
}

/// 浮层整体：标题行 + 页签行 + 页签头（描述/搜索占位）+ 内容区。
fn popover_area(theme: &Theme, model: &MentionPopover, entries: &[ColleagueEntry]) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(title_row(theme))
        .child(category_tabs(theme, model))
        .child(category_header(theme, model))
        .child(content_area(theme, model, entries))
}

/// 标题行。
fn title_row(theme: &Theme) -> Div {
    div().text_color(theme.foreground).child(TITLE)
}

/// 页签行：当前页签高亮，其余弱化；颜色一律取主题 token。
fn category_tabs(theme: &Theme, model: &MentionPopover) -> Div {
    div()
        .flex()
        .flex_row()
        .gap_3()
        .children(Category::ALL.iter().map(|&category| {
            let color = if category == model.category {
                theme.accent
            } else {
                theme.muted_foreground
            };
            div().text_color(color).child(category.label())
        }))
}

/// 页签头：描述一行 + 搜索占位一行（无下发文案的页签如实省略）。
fn category_header(theme: &Theme, model: &MentionPopover) -> Div {
    let mut header = div().flex().flex_col().gap_1();
    if let Some(description) = model.category.description() {
        header = header.child(div().text_color(theme.muted_foreground).child(description));
    }
    if let Some(placeholder) = model.category.placeholder() {
        header = header.child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .text_color(theme.muted_foreground)
                .child(placeholder),
        );
    }
    header
}

/// 内容区：未下发 ⇒ 「未下发」；有匹配 ⇒ 条目行；否则空态文案（不是空列表）。
fn content_area(theme: &Theme, model: &MentionPopover, entries: &[ColleagueEntry]) -> Div {
    if model.is_not_delivered() {
        return not_delivered_line(theme);
    }
    let visible = match model.category {
        Category::Colleague => model.colleague_matches(entries),
        _ => Vec::new(),
    };
    match (visible.is_empty(), model.empty_label()) {
        (true, Some(label)) => div().text_color(theme.muted_foreground).child(label),
        (true, None) => div(),
        (false, _) => div()
            .flex()
            .flex_col()
            .gap_1()
            .children(visible.iter().map(|&entry| colleague_row(theme, entry))),
    }
}

/// 「未下发」一行（fail-closed 分支）。
fn not_delivered_line(theme: &Theme) -> Div {
    div()
        .text_color(theme.muted_foreground)
        .child(NOT_DELIVERED)
}

/// 单条助理行：名称 + 忙闲状态。
fn colleague_row(theme: &Theme, entry: &ColleagueEntry) -> Div {
    div()
        .flex()
        .flex_row()
        .justify_between()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .child(div().text_color(theme.foreground).child(entry.name.clone()))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(entry.availability.label()),
        )
}
#[cfg(test)]
mod tests {
    use super::{
        content_area, main_area, popover_area, Availability, Category, ColleagueEntry,
        MentionPopover, State, APPLICATION_NAME, COLLEAGUE_DESCRIPTION, COLLEAGUE_NAME,
        COLLEAGUE_PLACEHOLDER, COLLEAGUE_SEARCH_EMPTY, FILE_DESCRIPTION, FILE_EMPTY_INITIAL,
        FILE_PLACEHOLDER, FILE_SEARCH_EMPTY, FILE_SESSION_GROUP, STATUS_BUSY, STATUS_IDLE,
        STATUS_UNAVAILABLE, TITLE,
    };
    use gpui_kit::component::Theme;

    fn entry(name: &str, availability: Availability) -> ColleagueEntry {
        ColleagueEntry {
            name: name.to_string(),
            availability,
        }
    }

    fn model(state: State, category: Category, query: &str) -> MentionPopover {
        MentionPopover {
            state,
            category,
            query: query.to_string(),
        }
    }

    /// ① 过滤计数正确：空词全匹配、大小写不敏感、无果为 0。
    #[test]
    fn colleague_matches_filters_and_counts() {
        let entries = vec![
            entry("张三", Availability::Idle),
            entry("李四", Availability::Busy),
            entry("Zoe", Availability::Unavailable),
        ];
        assert_eq!(
            model(State::Normal, Category::Colleague, "")
                .colleague_matches(&entries)
                .len(),
            3
        );
        assert_eq!(
            model(State::Normal, Category::Colleague, "zoe")
                .colleague_matches(&entries)
                .len(),
            1
        );
        assert_eq!(
            model(State::Normal, Category::Colleague, "王五")
                .colleague_matches(&entries)
                .len(),
            0
        );
    }

    /// ① 默认模型 fail-closed。
    #[test]
    fn default_model_is_fail_closed() {
        assert_eq!(State::default(), State::NotDelivered);
        assert!(MentionPopover::default().is_not_delivered());
        assert!(MentionPopover::not_delivered().is_not_delivered());
        assert_eq!(Category::default(), Category::Colleague);
    }

    /// ② 空输入 ⇒ 空态文案（不是空列表）。
    #[test]
    fn empty_input_yields_empty_state_label() {
        let file = model(State::Normal, Category::File, "");
        assert_eq!(file.empty_label(), Some(FILE_EMPTY_INITIAL));
        let colleague = model(State::Normal, Category::Colleague, "不存在的人");
        assert_eq!(colleague.colleague_matches(&[]).len(), 0);
        assert_eq!(colleague.empty_label(), Some(COLLEAGUE_SEARCH_EMPTY));
    }

    /// ② 文件页搜索空态 ≠ 初始空态；应用页无空态文案如实为 None。
    #[test]
    fn empty_label_varies_by_category_and_query() {
        assert_eq!(
            model(State::Normal, Category::File, "x").empty_label(),
            Some(FILE_SEARCH_EMPTY)
        );
        assert_eq!(
            model(State::Normal, Category::Application, "x").empty_label(),
            None
        );
    }

    /// ③ 文案逐字一致（≥3 键）。
    #[test]
    fn copy_is_verbatim_from_table() {
        assert_eq!(TITLE, "@提及弹层");
        assert_eq!(COLLEAGUE_NAME, "助理");
        assert_eq!(COLLEAGUE_PLACEHOLDER, "搜索助理");
        assert_eq!(COLLEAGUE_SEARCH_EMPTY, "未找到助理");
        assert_eq!(APPLICATION_NAME, "应用");
        assert_eq!(FILE_EMPTY_INITIAL, "当前对话中暂无文件");
        assert_eq!(STATUS_BUSY, "工作中");
        assert_eq!(STATUS_IDLE, "空闲");
        assert_eq!(STATUS_UNAVAILABLE, "未就绪");
    }

    /// ③ 命名空间 13 键全部下发、无一为空。
    #[test]
    fn all_thirteen_namespace_keys_delivered() {
        let delivered = [
            COLLEAGUE_NAME,
            COLLEAGUE_DESCRIPTION,
            COLLEAGUE_PLACEHOLDER,
            COLLEAGUE_SEARCH_EMPTY,
            STATUS_BUSY,
            STATUS_IDLE,
            STATUS_UNAVAILABLE,
            APPLICATION_NAME,
            FILE_DESCRIPTION,
            FILE_EMPTY_INITIAL,
            FILE_PLACEHOLDER,
            FILE_SEARCH_EMPTY,
            FILE_SESSION_GROUP,
        ];
        assert_eq!(delivered.len(), 13);
        assert!(delivered.iter().all(|copy| !copy.is_empty()));
    }

    /// ① 三页签齐备且逐字命名。
    #[test]
    fn category_tabs_are_three_and_labeled_verbatim() {
        assert_eq!(Category::ALL.len(), 3);
        let labels: Vec<&str> = Category::ALL.iter().map(|&c| c.label()).collect();
        assert_eq!(
            labels,
            vec![COLLEAGUE_NAME, APPLICATION_NAME, FILE_SESSION_GROUP]
        );
    }

    /// ④ 默认主题下主区渲染不 panic。
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }

    /// ⑤ 状态分支：非 Normal 一律视为未下发，Normal 不算。
    #[test]
    fn state_branches_classify_not_delivered() {
        for state in [State::Loading, State::Failed, State::NotDelivered] {
            assert!(model(state, Category::Colleague, "").is_not_delivered());
        }
        assert!(!model(State::Normal, Category::Colleague, "").is_not_delivered());
    }

    /// ⑤ 渲染分支全覆盖：三个未下发分支、条目分支、三类空态分支、全页签。
    #[test]
    fn render_paths_cover_all_branches() {
        let theme = Theme::default();
        let entries = vec![entry("张三", Availability::Idle)];
        for state in [State::NotDelivered, State::Loading, State::Failed] {
            let _ = content_area(&theme, &model(state, Category::Colleague, ""), &entries);
        }
        let _ = content_area(
            &theme,
            &model(State::Normal, Category::Colleague, ""),
            &entries,
        );
        let _ = content_area(
            &theme,
            &model(State::Normal, Category::Colleague, "无"),
            &entries,
        );
        let _ = content_area(&theme, &model(State::Normal, Category::File, ""), &entries);
        let _ = content_area(
            &theme,
            &model(State::Normal, Category::Application, ""),
            &entries,
        );
        for &category in Category::ALL.iter() {
            let _ = popover_area(&theme, &model(State::Normal, category, ""), &entries);
            let _ = popover_area(&theme, &model(State::NotDelivered, category, ""), &entries);
        }
    }
}
