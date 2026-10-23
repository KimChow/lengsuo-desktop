//! 办公文档预览（`PreviewKind::Office`）的内容渲染。
//!
//! 分两层落：纯函数层（子类型分派/标签/工具条项，无 GPUI 依赖）→
//! 接线层（工具条 + 页面/网格占位区）。⛔ 不读文件、不解析文档内容、
//! 不加解析依赖——预览内容需运行时契约下发，未下发前只诚实显示占位。
//! 颜色一律取自 `theme`，⛔ 不硬编码。
use gpui_kit::*;

/// 办公文档预览的三个子类型（由扩展名分派）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum OfficeKind {
    /// Word 类（doc/docx；未知扩展名兜底到这里）。
    Word,
    /// 表格类（xls/xlsx/csv/tsv）。
    Sheet,
    /// 演示类（ppt/pptx）。
    Slide,
}

/// 按扩展名分派子类型：`doc/docx`→Word、`xls/xlsx/csv/tsv`→Sheet、
/// `ppt/pptx`→Slide；未知/无扩展名兜底 Word。
fn office_kind(path: &str) -> OfficeKind {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "xls" | "xlsx" | "csv" | "tsv" => OfficeKind::Sheet,
        "ppt" | "pptx" => OfficeKind::Slide,
        _ => OfficeKind::Word,
    }
}

/// 子类型在界面上显示的标签。
fn kind_label(kind: OfficeKind) -> &'static str {
    match kind {
        OfficeKind::Word => "文档",
        OfficeKind::Sheet => "表格",
        OfficeKind::Slide => "演示",
    }
}

/// 各子类型的工具条项：Word/Sheet 只有缩放；Slide 是 上一页/页码/下一页。
fn toolbar_items(kind: OfficeKind) -> Vec<&'static str> {
    match kind {
        OfficeKind::Word | OfficeKind::Sheet => vec!["缩放"],
        OfficeKind::Slide => vec!["上一页", "页码", "下一页"],
    }
}

/// 未下发态的诚实提示：内容需运行时契约下发，本视图不伪造任何文档内容。
const NOT_DELIVERED: &str = "文档未下发（等运行时契约）";

/// 渲染办公文档预览：子类型标签 + 对应工具条 + 页面/网格占位区（未下发态）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    let kind = office_kind(path);
    div()
        .flex()
        .flex_col()
        .child(header_row(path, kind, theme))
        .child(toolbar_row(kind, theme))
        .child(placeholder_area(kind, theme))
        .into_any_element()
}

/// 头部：子类型标签 + 文件路径（等宽字体）。
fn header_row(path: &str, kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .bg(theme.muted)
                .text_xs()
                .text_color(theme.foreground)
                .child(kind_label(kind)),
        )
        .child(
            div()
                .flex_1()
                .truncate()
                .text_xs()
                .text_color(theme.muted_foreground)
                .font_family(theme.mono_font_family.clone())
                .child(path.to_owned()),
        )
}

/// 工具条：按子类型显示对应操作项。未下发态下为静态展示，交互待运行时契约。
fn toolbar_row(kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border);
    for item in toolbar_items(kind) {
        row = row.child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .text_xs()
                .text_color(theme.foreground)
                .child(item),
        );
    }
    row
}

/// 占位区：Word/Slide 为页面、Sheet 为网格，中央叠未下发提示。
fn placeholder_area(kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    div().flex().items_center().justify_center().p_6().child(
        div()
            .relative()
            .child(placeholder_frame(kind, theme))
            .child(
                div()
                    .absolute()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(NOT_DELIVERED),
                    ),
            ),
    )
}

/// 占位框架：Word 竖版页、Slide 横版页、Sheet 网格。均为空画布，不伪造内容。
fn placeholder_frame(kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    let frame = div()
        .bg(theme.background)
        .border_1()
        .border_color(theme.border)
        .rounded_sm();
    match kind {
        OfficeKind::Word => frame.w_64().h_96(),
        OfficeKind::Slide => frame.w_96().h_64(),
        OfficeKind::Sheet => sheet_grid(theme),
    }
}

/// 表格空画布：6×4 网格线（末行/末列不加线，避免与容器边框重叠）。
fn sheet_grid(theme: &gpui_kit::component::Theme) -> Div {
    let mut grid = div()
        .flex()
        .flex_col()
        .w_96()
        .h_64()
        .overflow_hidden()
        .rounded_sm()
        .bg(theme.background)
        .border_1()
        .border_color(theme.border);
    for row in 0..6 {
        let mut line = div().flex().flex_1();
        if row + 1 < 6 {
            line = line.border_b_1().border_color(theme.border);
        }
        for col in 0..4 {
            let mut cell = div().flex_1();
            if col + 1 < 4 {
                cell = cell.border_r_1().border_color(theme.border);
            }
            line = line.child(cell);
        }
        grid = grid.child(line);
    }
    grid
}

#[cfg(all(test, feature = "ui"))]
mod tests {
    use super::{kind_label, office_kind, render, toolbar_items, OfficeKind};
    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use gpui_kit::component::Theme;
    use gpui_kit::px;

    /// 断言③：三类子类型的渲染元素树非空（无窗口真实绘制一帧）。
    #[test]
    fn render_office_tree_is_nonempty() {
        let cases = [
            ("计划.docx", "Word", OfficeKind::Word),
            ("预算.xlsx", "Sheet", OfficeKind::Sheet),
            ("路演.pptx", "Slide", OfficeKind::Slide),
        ];
        for (path, what, kind) in cases {
            assert_eq!(office_kind(path), kind, "{what}：分派不符");
            let bounds = draw_probe(|_| render(path, &Theme::default()));
            let bounds = assert_rendered(bounds, what);
            assert!(
                bounds.size.width > px(240.0),
                "{what}：渲染树宽度异常 {:?}",
                bounds.size
            );
        }
    }

    /// 断言①：office_kind 三类各一例 + 未知扩展名。
    #[test]
    fn office_kind_dispatches_by_extension() {
        assert_eq!(office_kind("计划.doc"), OfficeKind::Word);
        assert_eq!(office_kind("报告.docx"), OfficeKind::Word);
        assert_eq!(office_kind("预算.xls"), OfficeKind::Sheet);
        assert_eq!(office_kind("预算.xlsx"), OfficeKind::Sheet);
        assert_eq!(office_kind("导出.csv"), OfficeKind::Sheet);
        assert_eq!(office_kind("导出.tsv"), OfficeKind::Sheet);
        assert_eq!(office_kind("路演.ppt"), OfficeKind::Slide);
        assert_eq!(office_kind("/tmp/路演.pptx"), OfficeKind::Slide);
        // 未知/无扩展名：兜底 Word。
        assert_eq!(office_kind("blob.bin"), OfficeKind::Word);
        assert_eq!(office_kind("noext"), OfficeKind::Word);
        // 扩展名大小写不敏感。
        assert_eq!(office_kind("UP.DOCX"), OfficeKind::Word);
    }

    /// 断言②：Slide 工具条含翻页项；Word/Sheet 只有缩放。
    #[test]
    fn toolbar_items_match_kind() {
        let slide = toolbar_items(OfficeKind::Slide);
        assert!(slide.contains(&"上一页"), "Slide 工具条缺上一页：{slide:?}");
        assert!(slide.contains(&"下一页"), "Slide 工具条缺下一页：{slide:?}");
        assert!(slide.contains(&"页码"), "Slide 工具条缺页码：{slide:?}");
        assert_eq!(toolbar_items(OfficeKind::Word), vec!["缩放"]);
        assert_eq!(toolbar_items(OfficeKind::Sheet), vec!["缩放"]);
    }

    /// 子类型标签三类互异。
    #[test]
    fn kind_labels_are_distinct() {
        let labels = [
            kind_label(OfficeKind::Word),
            kind_label(OfficeKind::Sheet),
            kind_label(OfficeKind::Slide),
        ];
        assert_eq!(labels, ["文档", "表格", "演示"]);
    }
}
