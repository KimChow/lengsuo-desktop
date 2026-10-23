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

/// 渲染办公文档预览（占位，待接线层填充）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(format!("{path}：办公文档预览尚未实现"))
        .into_any_element()
}

#[cfg(all(test, feature = "ui"))]
mod tests {
    use super::{kind_label, office_kind, toolbar_items, OfficeKind};

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
