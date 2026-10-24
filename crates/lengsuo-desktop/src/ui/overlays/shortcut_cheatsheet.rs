//! 快捷键速查浮层（波次 7，卡 `ui-overlay-shortcut-cheatsheet`，条目 A55）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `settings.keyboardShortcuts`。
//! 键位绑定运行时未下发：目录（动作名/分组/列头）为静态逐字数据，绑定列如实空置并显式提示「未下发」。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ==================== 纯数据层（无 GPUI 依赖） ====================

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "快捷键速查";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 搜索框占位符（逐字）。
pub const SEARCH_PLACEHOLDER: &str = "搜索快捷键";
/// 搜索无命中时的空态文案（逐字；⛔ 不得渲染成空列表）。
pub const NO_RESULTS: &str = "没有匹配的快捷键";
/// 列头「命令」（逐字）。
pub const COLUMN_COMMAND: &str = "命令";
/// 列头「按键绑定」（逐字）。
pub const COLUMN_BINDING: &str = "按键绑定";
/// 计数行模板（逐字，`{count}` 为占位符）。
pub const TOTAL_COUNT_TEMPLATE: &str = "共 {count} 条";
/// 冲突提示模板（逐字，`{keys}`/`{label}` 为占位符）。
pub const CONFLICT_TEMPLATE: &str = "“{keys}”已被“{label}”使用";
/// 分组标题（逐字取自 `category.*`）。
pub const CATEGORY_GENERAL: &str = "通用";
/// 分组标题（逐字取自 `category.editing`）。
pub const CATEGORY_EDITING: &str = "聊天";
/// 分组标题（逐字取自 `category.navigation`）。
pub const CATEGORY_NAVIGATION: &str = "任务";
/// 分组标题（逐字取自 `category.window`）。
pub const CATEGORY_WINDOW: &str = "面板·窗口";

/// 单条快捷键：动作名 + 键位组合（未下发时为 `None`，⛔ 不伪造键位）。
pub struct ShortcutEntry {
    /// 动作名（逐字取自 `actions.*`）。
    pub action: &'static str,
    /// 键位组合；绑定源未下发 ⇒ `None`。
    pub combo: Option<&'static str>,
}

/// 分组：分组标题 + 条目清单。
pub struct ShortcutGroup {
    /// 分组标题（逐字取自 `category.*`）。
    pub title: &'static str,
    /// 组内快捷键条目。
    pub entries: &'static [ShortcutEntry],
}

const fn entry(action: &'static str) -> ShortcutEntry {
    ShortcutEntry {
        action,
        combo: None,
    }
}

/// 静态目录：25 个动作按 4 个分组编排，动作名与分组标题全部逐字取自文案表。
pub static CATALOG: &[ShortcutGroup] = &[
    ShortcutGroup {
        title: CATEGORY_GENERAL,
        entries: &[
            entry("新建标签页"),
            entry("关闭标签页"),
            entry("打开设置"),
            entry("打开全局搜索"),
            entry("全局搜索"),
            entry("按住后点击，始终外部打开"),
        ],
    },
    ShortcutGroup {
        title: CATEGORY_EDITING,
        entries: &[
            entry("新建对话"),
            entry("新建快速问答"),
            entry("发送消息"),
            entry("输入时换行"),
            entry("停止生成"),
            entry("对话内搜索"),
            entry("放大内容字号"),
            entry("缩小内容字号"),
            entry("重置内容字号"),
            entry("唤起斜杠命令"),
        ],
    },
    ShortcutGroup {
        title: CATEGORY_NAVIGATION,
        entries: &[
            entry("下一个任务"),
            entry("上一个任务"),
            entry("定位当前任务"),
        ],
    },
    ShortcutGroup {
        title: CATEGORY_WINDOW,
        entries: &[
            entry("切换左侧栏"),
            entry("切换右侧栏"),
            entry("进入/退出全屏"),
            entry("唤起/隐藏主窗口"),
            entry("语音录制开关"),
            entry("唤起选择器"),
        ],
    },
];

/// 键位绑定数据的下发状态（fail-closed：未下发就明说，不拿空列表冒充）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BindingState {
    /// 键位绑定未下发（当前真值：目录里没有任何键位组合）。
    NotDelivered,
    /// 键位绑定已下发（每条动作都带键位组合）。
    Delivered,
}

/// 纯函数：目录条目全部带键位组合 ⇒ `Delivered`；任一缺失或目录为空 ⇒ `NotDelivered`。
pub fn binding_state(catalog: &[ShortcutGroup]) -> BindingState {
    let any_entry = catalog.iter().any(|g| !g.entries.is_empty());
    let all_delivered = catalog
        .iter()
        .flat_map(|g| g.entries.iter())
        .all(|e| e.combo.is_some());
    if any_entry && all_delivered {
        BindingState::Delivered
    } else {
        BindingState::NotDelivered
    }
}

/// 搜索结果：命中分组 + 命中总数 + 是否进入空态。
pub struct SearchOutcome<'a> {
    /// 命中分组（分组引用 + 命中条目；标题经字段取回 `&'static str`）。
    pub groups: Vec<(&'a ShortcutGroup, Vec<&'a ShortcutEntry>)>,
    /// 命中条目总数。
    pub total: usize,
    /// 空态标志：查询非空且零命中时为 `true`（渲染层须显式画空态，⛔ 不是空列表）。
    pub no_match: bool,
}

/// 纯函数搜索过滤：空/纯空白查询返回全量；否则按动作名与键位组合（忽略 ASCII 大小写）过滤。
pub fn search<'a>(catalog: &'a [ShortcutGroup], query: &str) -> SearchOutcome<'a> {
    let q = query.trim().to_lowercase();
    let mut groups = Vec::new();
    let mut total = 0usize;
    for group in catalog {
        let hits: Vec<&ShortcutEntry> = if q.is_empty() {
            group.entries.iter().collect()
        } else {
            group
                .entries
                .iter()
                .filter(|e| {
                    e.action.to_lowercase().contains(&q)
                        || e.combo.is_some_and(|c| c.to_lowercase().contains(&q))
                })
                .collect()
        };
        total += hits.len();
        if !hits.is_empty() {
            groups.push((group, hits));
        }
    }
    let no_match = !q.is_empty() && total == 0;
    SearchOutcome {
        groups,
        total,
        no_match,
    }
}

/// 冲突提示文案：模板逐字，仅替换 `{keys}`/`{label}` 占位符。
pub fn conflict_text(keys: &str, label: &str) -> String {
    CONFLICT_TEMPLATE
        .replace("{keys}", keys)
        .replace("{label}", label)
}

/// 纯函数冲突检测：同一键位组合被多条动作使用时产出冲突提示行（无组合数据 ⇒ 无冲突可报）。
pub fn detect_conflicts(catalog: &[ShortcutGroup]) -> Vec<String> {
    let mut seen: Vec<(&'static str, &'static str)> = Vec::new();
    let mut conflicts = Vec::new();
    for group in catalog {
        for e in group.entries {
            if let Some(combo) = e.combo {
                if let Some(&(_, first)) = seen.iter().find(|(c, _)| *c == combo) {
                    conflicts.push(conflict_text(combo, first));
                } else {
                    seen.push((combo, e.action));
                }
            }
        }
    }
    conflicts
}

/// 计数行文案：模板逐字，仅替换 `{count}` 占位符。
pub fn total_count_line(count: usize) -> String {
    TOTAL_COUNT_TEMPLATE.replace("{count}", &count.to_string())
}

// ==================== GPUI 层 ====================

/// 绑定列最小宽度（px）：键位组合列对齐；无数据时保留空槽位。
const BINDING_MIN_WIDTH: f32 = 96.0;

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
/// 数据驱动：默认空查询 ⇒ 全量目录；键位绑定未下发 ⇒ 计数行下如实提示「未下发」。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    let outcome = search(CATALOG, "");
    let sheet = div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div().px_4().py_2().flex().child(
                div()
                    .flex_1()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.muted)
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(SEARCH_PLACEHOLDER),
            ),
        )
        .child(
            div()
                .px_4()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(total_count_line(outcome.total)),
        )
        .child(render_banner_area(theme));
    sheet.child(render_list(theme, &outcome))
}

/// 提示区（数据驱动分支）：绑定未下发 ⇒ 「未下发」一行；检测到冲突 ⇒ 逐行冲突提示。
fn render_banner_area(theme: &Theme) -> Div {
    let mut area = div().flex().flex_col();
    if binding_state(CATALOG) == BindingState::NotDelivered {
        area = area.child(
            div()
                .px_4()
                .py_1()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        );
    }
    for line in detect_conflicts(CATALOG) {
        area = area.child(
            div()
                .px_4()
                .py_1()
                .text_xs()
                .text_color(theme.accent)
                .child(line),
        );
    }
    area
}

/// 清单区：空态（noResults）或「命令/按键绑定」列头 + 分组标题 + 清单行。
fn render_list(theme: &Theme, outcome: &SearchOutcome<'_>) -> Div {
    let mut list = div().flex_1().flex().flex_col();
    if outcome.no_match {
        return list.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NO_RESULTS),
        );
    }
    list = list.child(
        div()
            .px_4()
            .py_1()
            .flex()
            .gap_3()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(div().flex_1().child(COLUMN_COMMAND))
            .child(div().min_w(px(BINDING_MIN_WIDTH)).child(COLUMN_BINDING)),
    );
    for (group, entries) in &outcome.groups {
        list = list.child(
            div()
                .px_4()
                .py_1()
                .text_xs()
                .font_medium()
                .text_color(theme.muted_foreground)
                .child(group.title),
        );
        for e in entries {
            list = list.child(render_row(theme, e));
        }
    }
    list
}

/// 清单行：动作名（命令列）+ 键位组合槽（未下发 ⇒ 空槽，⛔ 不伪造键位）。
fn render_row(theme: &Theme, e: &ShortcutEntry) -> Div {
    div()
        .px_4()
        .py_1()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(theme.foreground)
                .child(e.action),
        )
        .child(
            div()
                .min_w(px(BINDING_MIN_WIDTH))
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .bg(theme.muted)
                .text_xs()
                .text_color(theme.foreground)
                .child(e.combo.unwrap_or("")),
        )
}
#[cfg(test)]
mod tests {
    use super::{
        binding_state, conflict_text, detect_conflicts, main_area, search, total_count_line,
        BindingState, ShortcutEntry, ShortcutGroup, CATALOG, CONFLICT_TEMPLATE, NOT_DELIVERED,
        NO_RESULTS, SEARCH_PLACEHOLDER, TITLE, TOTAL_COUNT_TEMPLATE,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn catalog_is_four_groups_twenty_five_actions_fail_closed() {
        assert_eq!(CATALOG.len(), 4);
        assert_eq!(CATALOG.iter().map(|g| g.entries.len()).sum::<usize>(), 25);
        assert!(CATALOG.iter().all(|g| !g.entries.is_empty()));
        assert_eq!(binding_state(CATALOG), BindingState::NotDelivered);
        assert!(CATALOG
            .iter()
            .flat_map(|g| g.entries.iter())
            .all(|e| e.combo.is_none()));
    }

    #[test]
    fn binding_state_delivered_only_when_every_entry_has_combo() {
        let delivered = [ShortcutGroup {
            title: "通用",
            entries: &[
                ShortcutEntry {
                    action: "新建标签页",
                    combo: Some("Ctrl+T"),
                },
                ShortcutEntry {
                    action: "关闭标签页",
                    combo: Some("Ctrl+W"),
                },
            ],
        }];
        assert_eq!(binding_state(&delivered), BindingState::Delivered);
        let partial = [ShortcutGroup {
            title: "通用",
            entries: &[
                ShortcutEntry {
                    action: "新建标签页",
                    combo: Some("Ctrl+T"),
                },
                ShortcutEntry {
                    action: "关闭标签页",
                    combo: None,
                },
            ],
        }];
        assert_eq!(binding_state(&partial), BindingState::NotDelivered);
    }

    #[test]
    fn search_empty_query_lists_all_and_no_match_is_explicit_state() {
        let all = search(CATALOG, "   ");
        assert_eq!(all.total, 25);
        assert_eq!(all.groups.len(), 4);
        assert!(!all.no_match);
        let none = search(CATALOG, "绝不存在的命令字");
        assert_eq!(none.total, 0);
        assert!(none.groups.is_empty());
        // 空态是显式标志，由渲染层画「没有匹配的快捷键」，而不是静默空列表
        assert!(none.no_match);
    }

    #[test]
    fn search_filters_by_action_label() {
        let tabs = search(CATALOG, "标签页");
        assert_eq!(tabs.total, 2);
        assert!(tabs
            .groups
            .iter()
            .flat_map(|(_, entries)| entries.iter())
            .all(|e| e.action.contains("标签页")));
        let tasks = search(CATALOG, "任务");
        assert_eq!(tasks.total, 3);
        assert_eq!(tasks.groups.len(), 1);
        assert_eq!(tasks.groups[0].0.title, "任务");
    }

    #[test]
    fn shipped_catalog_has_no_conflict_and_detector_fires_on_duplicate() {
        assert!(detect_conflicts(CATALOG).is_empty());
        let duplicated = [ShortcutGroup {
            title: "通用",
            entries: &[
                ShortcutEntry {
                    action: "新建标签页",
                    combo: Some("Ctrl+T"),
                },
                ShortcutEntry {
                    action: "新建对话",
                    combo: Some("Ctrl+T"),
                },
            ],
        }];
        assert_eq!(
            detect_conflicts(&duplicated),
            vec![conflict_text("Ctrl+T", "新建标签页")]
        );
    }

    #[test]
    fn copy_is_verbatim_from_settings_keyboard_shortcuts() {
        assert!(!TITLE.is_empty());
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(SEARCH_PLACEHOLDER, "搜索快捷键");
        assert_eq!(NO_RESULTS, "没有匹配的快捷键");
        assert_eq!(super::COLUMN_COMMAND, "命令");
        assert_eq!(super::COLUMN_BINDING, "按键绑定");
        assert_eq!(super::CATEGORY_GENERAL, "通用");
        assert_eq!(super::CATEGORY_EDITING, "聊天");
        assert_eq!(super::CATEGORY_NAVIGATION, "任务");
        assert_eq!(super::CATEGORY_WINDOW, "面板·窗口");
        assert_eq!(TOTAL_COUNT_TEMPLATE, "共 {count} 条");
        assert_eq!(CONFLICT_TEMPLATE, "“{keys}”已被“{label}”使用");
        assert_eq!(total_count_line(25), "共 25 条");
        assert_eq!(
            conflict_text("Ctrl+F", "对话内搜索"),
            "“Ctrl+F”已被“对话内搜索”使用"
        );
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
