//! 我的文件主区视图（波次 5，卡 `ui-my-files`）。
//!
//! 分层约定（同 `env_gpui`）：
//! - **纯数据层**（本文件上半，无 GPUI 依赖）：可枚举状态（未下发 / 空态 / 条目）
//!   与逐字文案常量；GPUI 渲染是它的纯函数；
//! - **GPUI 层**（[`main_area`]）：把描述接到元素树，颜色一律取主题 token。
//!
//! 硬约束：
//! - **fail-closed**：数据未下发必须如实显示「未下发」，不得退化成空列表；
//! - **只做界面**：不判权限、不持凭据、不直连运行时；
//! - ⛔ **不伪造服务端数据**（界面文案逐字取自界面文案全表命名空间 `myFiles`）。
//!
//! 数据源在运行时侧：`main_area` 当前按 `NotDelivered` 态渲染，等跨线契约就绪后
//! 由 `ui-*-switch-source` 类卡换数据源（**只换数据源、不改视图结构**）。
#![cfg(feature = "ui")]

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ===================== 纯数据层（无 GPUI 依赖） =====================

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "我的文件";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 已下发且为空的空态文案（`myFiles.empty.noFiles`）。
pub const EMPTY: &str = "暂无文件";

/// 表格列头：名称 / 更新人 / 最近编辑（`myFiles.browser.header*`）。
pub const HEADER_NAME: &str = "名称";
pub const HEADER_OWNER: &str = "更新人";
pub const HEADER_TIME: &str = "最近编辑";

/// 工具栏操作行文案（`myFiles.actions.*`；顺序即渲染顺序）。
pub const ACTION_OPEN_FOLDER: &str = "打开文件夹";
pub const ACTION_DOWNLOAD: &str = "下载";
pub const ACTION_RENAME: &str = "重命名";
pub const ACTION_MOVE_TO: &str = "移动到";
pub const ACTION_MORE: &str = "更多操作";
pub const TOOLBAR_ACTIONS: [&str; 5] = [
    ACTION_OPEN_FOLDER,
    ACTION_DOWNLOAD,
    ACTION_RENAME,
    ACTION_MOVE_TO,
    ACTION_MORE,
];

/// 一行文件条目（纯数据；由运行时下发，本模块不伪造）。
pub struct FileEntry {
    pub name: String,
    pub updated_by: String,
    pub updated_time: String,
}

/// 可枚举的视图状态。
pub enum MyFilesState {
    /// 数据未下发（fail-closed 默认态）。
    NotDelivered,
    /// 已下发且为空 ⇒ 空态（不是空列表）。
    Empty,
    /// 已下发且有条目。
    Entries(Vec<FileEntry>),
}

/// 我的文件视图的纯数据描述。
pub struct MyFilesView {
    state: MyFilesState,
}

impl MyFilesView {
    /// fail-closed 构造：无数据源时如实显示「未下发」。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            state: MyFilesState::NotDelivered,
        }
    }

    /// 由下发条目构造：空输入 ⇒ `Empty` 空态，非空 ⇒ `Entries`。
    #[must_use]
    pub fn build(entries: Vec<FileEntry>) -> Self {
        Self {
            state: if entries.is_empty() {
                MyFilesState::Empty
            } else {
                MyFilesState::Entries(entries)
            },
        }
    }

    pub fn state(&self) -> &MyFilesState {
        &self.state
    }

    /// 状态体文案：未下发 / 空态；有条目时为 `None`（由行渲染接管）。
    pub fn body_notice(&self) -> Option<&'static str> {
        match self.state {
            MyFilesState::NotDelivered => Some(NOT_DELIVERED),
            MyFilesState::Empty => Some(EMPTY),
            MyFilesState::Entries(_) => None,
        }
    }

    /// 条目行（名称 / 更新人 / 最近编辑）；仅 `Entries` 态非空。
    pub fn rows(&self) -> Vec<[&str; 3]> {
        let MyFilesState::Entries(entries) = &self.state else {
            return Vec::new();
        };
        entries
            .iter()
            .map(|e| {
                [
                    e.name.as_str(),
                    e.updated_by.as_str(),
                    e.updated_time.as_str(),
                ]
            })
            .collect()
    }
}

// ===================== GPUI 层 =====================

/// 渲染断言的节点标记（只在 debug + test-support 构建记录，生产零开销）。
const TITLE_SELECTOR: &str = "my-files-title";
const TOOLBAR_SELECTOR: &str = "my-files-toolbar";
const HEADER_SELECTOR: &str = "my-files-header";
const BODY_SELECTOR: &str = "my-files-body";

/// 主区渲染。
///
/// 返回**具体 `Div`**：`main_area` 的 match 各分支类型必须一致
/// （不用 `impl IntoElement`，不透明类型无法与 `Div` 统一）。
/// 数据源未接 ⇒ 按 `NotDelivered` 态如实渲染「未下发」。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&MyFilesView::not_delivered(), theme)
}

/// 把纯数据描述接到元素树：标题行 + 工具栏操作行 + 三列表格 + 状态体。
fn render(view: &MyFilesView, theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(title_row(theme))
        .child(toolbar_row(theme))
        .child(header_row(theme))
        .child(body(view, theme))
}

fn title_row(theme: &Theme) -> Div {
    div()
        .debug_selector(|| TITLE_SELECTOR.to_owned())
        .flex()
        .items_center()
        .px_4()
        .py_2()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(TITLE)
}

/// 工具栏操作行：本卡只落结构与文案，不接行为（接行为归后续接线卡）。
fn toolbar_row(theme: &Theme) -> Div {
    div()
        .debug_selector(|| TOOLBAR_SELECTOR.to_owned())
        .flex()
        .items_center()
        .gap_4()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.muted_foreground)
        .children(TOOLBAR_ACTIONS.iter().map(|action| div().child(*action)))
}

/// 三列表格行骨架：名称自适应，更新人 / 最近编辑定宽。
fn table_row(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_4()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .text_sm()
}

fn header_row(theme: &Theme) -> Div {
    table_row(theme)
        .debug_selector(|| HEADER_SELECTOR.to_owned())
        .text_color(theme.muted_foreground)
        .child(div().flex_1().min_w_0().child(HEADER_NAME))
        .child(div().w(px(112.0)).child(HEADER_OWNER))
        .child(div().w(px(144.0)).child(HEADER_TIME))
}

fn entry_row(theme: &Theme, row: [&str; 3]) -> Div {
    let [name, owner, updated_time] = row;
    table_row(theme)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(theme.foreground)
                .child(name.to_owned()),
        )
        .child(
            div()
                .w(px(112.0))
                .truncate()
                .text_color(theme.muted_foreground)
                .child(owner.to_owned()),
        )
        .child(
            div()
                .w(px(144.0))
                .truncate()
                .text_color(theme.muted_foreground)
                .child(updated_time.to_owned()),
        )
}

/// 状态体：`Entries` ⇒ 逐条渲染；空态 / 未下发 ⇒ 居中一行说明。
fn body(view: &MyFilesView, theme: &Theme) -> Div {
    let base = div()
        .debug_selector(|| BODY_SELECTOR.to_owned())
        .flex_1()
        .flex()
        .flex_col();
    match view.state() {
        MyFilesState::Entries(_) => {
            base.children(view.rows().into_iter().map(|row| entry_row(theme, row)))
        }
        MyFilesState::Empty => notice(base, theme, EMPTY),
        MyFilesState::NotDelivered => notice(base, theme, NOT_DELIVERED),
    }
}

fn notice(base: Div, theme: &Theme, text: &'static str) -> Div {
    base.child(
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.muted_foreground)
            .child(text),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        FileEntry, MyFilesState, MyFilesView, EMPTY, HEADER_NAME, HEADER_OWNER, HEADER_TIME,
        NOT_DELIVERED, TITLE, TOOLBAR_ACTIONS,
    };

    fn entry(name: &str) -> FileEntry {
        FileEntry {
            name: name.to_owned(),
            updated_by: format!("{name}-owner"),
            updated_time: format!("{name}-t"),
        }
    }

    /// ② 空输入 ⇒ 空态（不是空列表，也不是未下发）。
    #[test]
    fn empty_input_yields_empty_state_not_entries() {
        let view = MyFilesView::build(vec![]);
        assert!(matches!(view.state(), MyFilesState::Empty));
        assert_eq!(view.body_notice(), Some(EMPTY));
        assert!(view.rows().is_empty());
    }

    /// ① 纯函数层分组 / 计数正确。
    #[test]
    fn entries_state_counts_and_orders_rows() {
        let view = MyFilesView::build(vec![entry("a.md"), entry("b.png"), entry("c.zip")]);
        assert!(matches!(view.state(), MyFilesState::Entries(_)));
        assert_eq!(view.rows().len(), 3);
        assert_eq!(view.rows()[0], ["a.md", "a.md-owner", "a.md-t"]);
        assert_eq!(view.body_notice(), None);
    }

    /// ③ 文案逐字一致（对 ≥3 个 key 断言）。
    #[test]
    fn copy_is_verbatim_from_myfiles_namespace() {
        assert_eq!(
            (HEADER_NAME, HEADER_OWNER, HEADER_TIME),
            ("名称", "更新人", "最近编辑")
        );
        assert_eq!(EMPTY, "暂无文件");
        assert_eq!(NOT_DELIVERED, "未下发");
        assert_eq!(TITLE, "我的文件");
        assert_eq!(
            TOOLBAR_ACTIONS,
            ["打开文件夹", "下载", "重命名", "移动到", "更多操作"]
        );
    }

    /// fail-closed 默认构造：无数据源 ⇒ 如实「未下发」。
    #[test]
    fn not_delivered_is_the_fail_closed_default() {
        let view = MyFilesView::not_delivered();
        assert!(matches!(view.state(), MyFilesState::NotDelivered));
        assert_eq!(view.body_notice(), Some(NOT_DELIVERED));
        assert!(view.rows().is_empty());
    }

    /// 视图描述的节点清单：标题 + ≥3 个分组/清单行来源。
    #[test]
    fn view_describes_title_plus_at_least_three_group_rows() {
        assert_eq!(TOOLBAR_ACTIONS.len(), 5);
        let view = MyFilesView::build(vec![entry("a.md"), entry("b.png"), entry("c.zip")]);
        assert!(view.rows().len() >= 3);
    }

    /// ④ `main_area(&Theme::default())` 不 panic。
    #[test]
    fn main_area_builds_without_panic() {
        let theme = gpui_kit::component::Theme::default();
        let _ = super::main_area(&theme);
    }
}

/// 接线层的无窗口渲染断言：`TestAppContext` + `VisualTestContext::draw`
/// 跑完整 layout + prepaint + paint。
///
/// ⛔ 不写 `use super::*`：避免 GPUI 的 `test` 属性宏遮蔽标准库 `#[test]`。
#[cfg(test)]
mod render_tests {
    use super::{
        main_area, render, FileEntry, MyFilesView, BODY_SELECTOR, HEADER_SELECTOR, TITLE_SELECTOR,
        TOOLBAR_SELECTOR,
    };
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, IntoElement, ParentElement as _, Styled as _, TestAppContext,
    };

    /// 画一帧并断言每个命名节点都真的进了渲染帧且排版非零。
    /// `debug_bounds` 只认 `'static` 选择器 ⇒ 参数收 `'static` 切片（调用点全是字面量）。
    fn assert_nodes_present(view: &MyFilesView, checks: &'static [(&'static str, &'static str)]) {
        let mut ctx = TestAppContext::single();
        ctx.update(|cx| cx.set_global(Theme::default()));

        let theme = Theme::default();
        let element = render(view, &theme);

        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(960.0), px(640.0)),
            |_window, _cx| div().size_full().flex().child(element).into_any_element(),
        );
        for (selector, what) in checks {
            let bounds = visual
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("{what}：节点未进入渲染帧（debug_selector 未记录）"));
            assert!(
                bounds.size.width > px(0.0) && bounds.size.height > px(0.0),
                "{what}：布局宽高为 0，节点没真正参与排版"
            );
        }
    }

    fn sample_entries(n: usize) -> Vec<FileEntry> {
        (0..n)
            .map(|i| FileEntry {
                name: format!("f{i}.md"),
                updated_by: format!("u{i}"),
                updated_time: format!("2026-10-{i:02}"),
            })
            .collect()
    }

    /// 元素树可定位：标题节点 + 工具栏 / 表头 / 状态体三个分组节点（未下发态）。
    #[test]
    fn tree_locates_title_and_three_group_rows_when_not_delivered() {
        assert_nodes_present(
            &MyFilesView::not_delivered(),
            &[
                (TITLE_SELECTOR, "标题行"),
                (TOOLBAR_SELECTOR, "工具栏操作行"),
                (HEADER_SELECTOR, "表头行"),
                (BODY_SELECTOR, "状态体（未下发）"),
            ],
        );
    }

    /// 空态整帧：状态体渲染空态说明，不是空列表。
    #[test]
    fn tree_locates_empty_state_notice() {
        assert_nodes_present(
            &MyFilesView::build(vec![]),
            &[
                (TITLE_SELECTOR, "标题行"),
                (BODY_SELECTOR, "状态体（空态）"),
            ],
        );
    }

    /// 条目态整帧：清单体逐条渲染（行数断言在纯函数层测试）。
    #[test]
    fn tree_locates_rows_when_entries_delivered() {
        assert_nodes_present(
            &MyFilesView::build(sample_entries(3)),
            &[
                (TITLE_SELECTOR, "标题行"),
                (TOOLBAR_SELECTOR, "工具栏操作行"),
                (HEADER_SELECTOR, "表头行"),
                (BODY_SELECTOR, "清单体（条目行）"),
            ],
        );
    }

    /// `main_area`（未下发默认态）完整绘制不 panic。
    #[test]
    fn main_area_draws_full_tree_without_panic() {
        let mut ctx = TestAppContext::single();
        ctx.update(|cx| cx.set_global(Theme::default()));

        let element = main_area(&Theme::default());

        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(960.0), px(640.0)),
            |_window, _cx| div().size_full().flex().child(element).into_any_element(),
        );
        let bounds = visual
            .debug_bounds(TITLE_SELECTOR)
            .expect("main_area：标题行未进入渲染帧");
        assert!(bounds.size.height > px(0.0));
    }
}
