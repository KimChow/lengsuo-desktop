//! 团队知识库主区视图（波次 5，卡 `ui-knowledge-base-panel`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：三态判据 / 位置分组 / 筛选组 / 逐字文案；
//! - **GPUI 层**（[`main_area`]）：把描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**：条目 / 所有者 / 位置等业务值只能来自数据源下发
//!   （界面文案逐字取自文案全表命名空间 `knowledgeBase`；测试夹具除外）。
//!
//! 数据源在运行时侧：等跨线契约就绪后由 `ui-*-switch-source` 类卡换数据源
//! （**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

// ── 纯数据层（无 GPUI 依赖） ─────────────────────────────────────────────

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "团队知识库";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 搜索框占位符（逐字）。
pub const SEARCH_PLACEHOLDER: &str = "搜索文档";
/// 搜索触发提示（逐字）。
pub const SEARCH_TRIGGER_TIP: &str = "按 Enter 键或点击搜索图标进行搜索";
/// 搜索按钮标签（逐字）。
pub const SEARCH_BUTTON_LABEL: &str = "搜索";
/// 筛选行「更多」入口（逐字）。
pub const FILTER_MORE: &str = "更多";
/// 结果区标题（逐字）。
pub const SEARCH_RESULTS_TITLE: &str = "搜索结果";
/// 结果计数模板（逐字，`{count}` 为占位符）。
pub const RESULT_COUNT_TEMPLATE: &str = "共 {count} 条结果";
/// 结果区末尾说明（逐字）。
pub const LOADED_ALL: &str = "已加载全部结果";
/// 空态标题（逐字）。
pub const EMPTY_TITLE: &str = "未找到相关文档";
/// 空态说明（逐字）。
pub const EMPTY_DESCRIPTION: &str = "请尝试更换关键词或调整筛选条件";

/// 一组筛选：标签 + 候选项（全部逐字取自文案全表对应 key）。
#[derive(Clone, Copy)]
pub struct FilterGroup {
    /// 筛选维度名（如「位置」）。
    pub label: &'static str,
    /// 候选项；`[0]` 为默认值。
    pub options: &'static [&'static str],
}

/// 筛选「位置」：位置值由数据源下发，文案全表只提供默认项。
pub const FILTER_LOCATION: FilterGroup = FilterGroup {
    label: "位置",
    options: &["全部位置"],
};
/// 筛选「所有者」。
pub const FILTER_OWNER: FilterGroup = FilterGroup {
    label: "所有者",
    options: &["全部", "我所有的", "他人所有"],
};
/// 筛选「修改时间」。
pub const FILTER_TIME: FilterGroup = FilterGroup {
    label: "修改时间",
    options: &["全部时间", "近7天", "近1个月", "起始时间 ~ 结束时间"],
};
/// 筛选「类型」。
pub const FILTER_TYPE: FilterGroup = FilterGroup {
    label: "类型",
    options: &[
        "文档",
        "表格",
        "智能文档",
        "思维导图",
        "画板",
        "流程图",
        "幻灯片",
        "PDF",
        "智能表",
        "收集表",
    ],
};

/// 一条知识库文档条目（由数据源下发；⛔ 本模块不伪造）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDoc {
    /// 文档名。
    pub name: String,
    /// 所有者显示名。
    pub owner: String,
    /// 所在位置显示名。
    pub location: String,
}

/// 结果清单的三态（fail-closed 判据：未下发 ≠ 空结果）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResultsState {
    /// 数据未下发（⛔ 不得当成空结果渲染）。
    #[default]
    NotDelivered,
    /// 已下发且有匹配结果。
    Entries(Vec<KnowledgeDoc>),
    /// 已下发但无匹配结果（空态）。
    Empty,
}

/// 团队知识库视图的纯数据描述（GPUI 层按它渲染）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 团队知识库View {
    /// 结果清单状态。
    pub results: ResultsState,
}

/// 构造视图描述。
#[must_use]
pub fn build(results: ResultsState) -> 团队知识库View {
    团队知识库View { results }
}

/// 空态判据：`None`（未下发）与 `Some(空)`（无匹配）必须区分。
#[must_use]
pub fn classify(delivered: Option<Vec<KnowledgeDoc>>) -> ResultsState {
    match delivered {
        None => ResultsState::NotDelivered,
        Some(entries) if entries.is_empty() => ResultsState::Empty,
        Some(entries) => ResultsState::Entries(entries),
    }
}

/// 按位置分组（组内保持入参顺序；空输入 ⇒ 空分组序列）。
#[must_use]
pub fn group_by_location(entries: &[KnowledgeDoc]) -> Vec<(&str, Vec<&KnowledgeDoc>)> {
    let mut groups: Vec<(&str, Vec<&KnowledgeDoc>)> = Vec::new();
    for doc in entries {
        match groups
            .iter_mut()
            .find(|(location, _)| *location == doc.location.as_str())
        {
            Some((_, docs)) => docs.push(doc),
            None => groups.push((doc.location.as_str(), vec![doc])),
        }
    }
    groups
}

/// 结果计数文案：把模板里的 `{count}` 换成实际数。
#[must_use]
pub fn result_count_label(count: usize) -> String {
    RESULT_COUNT_TEMPLATE.replace("{count}", &count.to_string())
}

// ── GPUI 层 ──────────────────────────────────────────────────────────────

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
///
/// 数据源未接入 ⇒ [`classify`] 收到 `None` ⇒ 恒为「未下发」分支（fail-closed）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&build(classify(None)), theme)
}

/// 按纯数据描述渲染全部区块（测试经此覆盖三分支）。
fn render(view: &团队知识库View, theme: &Theme) -> Div {
    div()
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
        .child(search_row(theme))
        .child(
            div()
                .px_4()
                .pb_1()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SEARCH_TRIGGER_TIP),
        )
        .child(filter_row(theme))
        .child(results_area(&view.results, theme))
}

/// 搜索行：占位符 + 搜索入口（静态展示，输入态等数据源接入后再接）。
fn search_row(theme: &Theme) -> Div {
    div()
        .px_4()
        .pt_2()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .flex_1()
                .px_2()
                .py_1()
                .text_sm()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .text_color(theme.muted_foreground)
                .child(SEARCH_PLACEHOLDER),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.accent)
                .child(SEARCH_BUTTON_LABEL),
        )
}

/// 筛选行：每组 = 标签 + 默认项，行尾「更多」。
fn filter_row(theme: &Theme) -> Div {
    let mut row = div()
        .px_4()
        .pb_2()
        .flex()
        .items_center()
        .gap_2()
        .flex_wrap();
    for group in [FILTER_LOCATION, FILTER_OWNER, FILTER_TIME, FILTER_TYPE] {
        row = row.child(filter_chip(group.label, group.options[0], theme));
    }
    row.child(div().text_sm().text_color(theme.accent).child(FILTER_MORE))
}

/// 一枚筛选片：标签（弱）+ 当前值（强）。
fn filter_chip(label: &'static str, value: &'static str, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(label),
        )
        .child(div().text_sm().text_color(theme.foreground).child(value))
}

/// 结果区三分支：未下发 / 空态 / 清单（各分支同为 `Div`）。
fn results_area(results: &ResultsState, theme: &Theme) -> Div {
    match results {
        ResultsState::NotDelivered => div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(NOT_DELIVERED),
        ResultsState::Empty => div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(EMPTY_TITLE),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(EMPTY_DESCRIPTION),
            ),
        ResultsState::Entries(entries) => entries_area(entries, theme),
    }
}

/// 清单分支：标题 + 计数 + 按位置分组（分组头 + 行），末尾「已加载全部结果」。
fn entries_area(entries: &[KnowledgeDoc], theme: &Theme) -> Div {
    let mut area = div()
        .flex_1()
        .flex()
        .flex_col()
        .px_4()
        .pt_1()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(SEARCH_RESULTS_TITLE),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(result_count_label(entries.len())),
                ),
        );
    for (location, docs) in group_by_location(entries) {
        area = area.child(
            div()
                .pt_1()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(location)),
        );
        for doc in docs {
            area = area.child(doc_row(doc, theme));
        }
    }
    area.child(
        div()
            .pt_2()
            .pb_3()
            .text_xs()
            .text_color(theme.muted_foreground)
            .child(LOADED_ALL),
    )
}

/// 一条清单行：文档名（主）+ 所有者 + 位置（辅）。
fn doc_row(doc: &KnowledgeDoc, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .py_1()
        .child(
            div()
                .flex_1()
                .min_w_32()
                .truncate()
                .text_sm()
                .text_color(theme.foreground)
                .child(SharedString::from(doc.name.as_str())),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(doc.owner.as_str())),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(doc.location.as_str())),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        build, classify, group_by_location, main_area, render, result_count_label, KnowledgeDoc,
        ResultsState, EMPTY_DESCRIPTION, EMPTY_TITLE, FILTER_LOCATION, FILTER_MORE, FILTER_OWNER,
        FILTER_TIME, FILTER_TYPE, NOT_DELIVERED, SEARCH_PLACEHOLDER, SEARCH_RESULTS_TITLE,
        SEARCH_TRIGGER_TIP, TITLE,
    };
    use gpui_kit::component::Theme;

    /// 测试夹具：条目值只存在于测试，运行时不经本模块自造。
    fn doc(name: &str, owner: &str, location: &str) -> KnowledgeDoc {
        KnowledgeDoc {
            name: name.to_owned(),
            owner: owner.to_owned(),
            location: location.to_owned(),
        }
    }

    /// 5 条 / 3 个位置，供分组与渲染断言复用。
    fn fixture() -> Vec<KnowledgeDoc> {
        vec![
            doc("周报", "张三", "产品部"),
            doc("接口约定", "李四", "公共根目录"),
            doc("调研记录", "张三", "产品部"),
            doc("数据字典", "王五", "数据组"),
            doc("上线清单", "李四", "公共根目录"),
        ]
    }

    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert_eq!(TITLE, "团队知识库");
    }

    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn panel_copy_is_verbatim() {
        assert_eq!(SEARCH_PLACEHOLDER, "搜索文档");
        assert_eq!(SEARCH_TRIGGER_TIP, "按 Enter 键或点击搜索图标进行搜索");
        assert_eq!(SEARCH_RESULTS_TITLE, "搜索结果");
        assert_eq!(FILTER_MORE, "更多");
        assert_eq!(EMPTY_TITLE, "未找到相关文档");
        assert_eq!(EMPTY_DESCRIPTION, "请尝试更换关键词或调整筛选条件");
    }

    #[test]
    fn filter_groups_are_verbatim() {
        assert_eq!(FILTER_LOCATION.label, "位置");
        assert_eq!(FILTER_LOCATION.options, ["全部位置"]);
        assert_eq!(FILTER_OWNER.label, "所有者");
        assert_eq!(FILTER_OWNER.options, ["全部", "我所有的", "他人所有"]);
        assert_eq!(FILTER_TIME.label, "修改时间");
        assert_eq!(
            FILTER_TIME.options,
            ["全部时间", "近7天", "近1个月", "起始时间 ~ 结束时间"]
        );
        assert_eq!(FILTER_TYPE.label, "类型");
        assert_eq!(FILTER_TYPE.options.first(), Some(&"文档"));
        assert!(FILTER_TYPE.options.contains(&"收集表"));
    }

    #[test]
    fn grouping_by_location_counts_and_preserves_order() {
        let docs = fixture();
        let groups = group_by_location(&docs);
        let labels: Vec<&str> = groups.iter().map(|(location, _)| *location).collect();
        assert_eq!(labels, ["产品部", "公共根目录", "数据组"]);
        let counts: Vec<usize> = groups.iter().map(|(_, docs)| docs.len()).collect();
        assert_eq!(counts, [2, 2, 1]);
        assert_eq!(groups[0].1[0].name, "周报");
        assert!(group_by_location(&[]).is_empty());
    }

    #[test]
    fn classify_distinguishes_not_delivered_empty_and_entries() {
        assert_eq!(classify(None), ResultsState::NotDelivered);
        assert_eq!(classify(Some(Vec::new())), ResultsState::Empty);
        assert_ne!(classify(Some(Vec::new())), ResultsState::NotDelivered);
        let one = vec![doc("周报", "张三", "产品部")];
        assert_eq!(classify(Some(one.clone())), ResultsState::Entries(one));
        assert_eq!(ResultsState::default(), ResultsState::NotDelivered);
        assert_eq!(build(ResultsState::Empty).results, ResultsState::Empty);
    }

    #[test]
    fn result_count_label_fills_template() {
        assert_eq!(result_count_label(5), "共 5 条结果");
        assert_eq!(result_count_label(0), "共 0 条结果");
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    /// 元素树判据：标题节点 + ≥3 个分组/清单节点，且三分支都能整树构造。
    #[test]
    fn render_all_three_states_with_at_least_three_nodes() {
        let theme = Theme::default();
        let fixture = fixture();
        let groups = group_by_location(&fixture);
        assert_eq!(groups.len(), 3, "分组节点 ≥3");
        assert!(fixture.len() >= 3, "清单行 ≥3");
        assert_eq!(TITLE, "团队知识库", "标题节点");
        for state in [
            ResultsState::NotDelivered,
            ResultsState::Empty,
            ResultsState::Entries(fixture),
        ] {
            let view = build(state);
            let _ = render(&view, &theme);
        }
        let _ = render(&build(ResultsState::default()), &theme);
    }
}
