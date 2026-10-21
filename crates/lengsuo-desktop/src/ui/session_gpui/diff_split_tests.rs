//! diff 左右分栏 + 文件点击意图的**纯逻辑测试**（不开窗口）。
//!
//! 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
//! `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。

#[cfg(test)]
mod tests {
    use super::super::diff_split::{parse, CellTone, DiffBlock, DiffLayout};
    use super::super::file_card::{is_clickable, record_file_click};
    use super::super::SessionStreamGpui;
    use crate::state::file_intent::FileIntentSource;
    use crate::state::session_stream::SessionStreamState;
    use crate::ui::session_render::FileItem;

    /// 任务书给的样例：一个「删一行 + 加一行 + 一行上下文」的 hunk。
    const SAMPLE: &str = "--- a\n+++ b\n@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n";

    /// 取出左栏（旧文件）的单元格文本。
    fn left_texts(layout: &DiffLayout) -> Vec<String> {
        layout
            .left_column()
            .into_iter()
            .map(|cell| cell.text.clone())
            .collect()
    }

    /// 取出右栏（新文件）的单元格文本。
    fn right_texts(layout: &DiffLayout) -> Vec<String> {
        layout
            .right_column()
            .into_iter()
            .map(|cell| cell.text.clone())
            .collect()
    }

    /// 取出左栏的行号序列。
    fn left_nos(layout: &DiffLayout) -> Vec<usize> {
        layout
            .left_column()
            .into_iter()
            .map(|cell| cell.no)
            .collect()
    }

    /// 取出右栏的行号序列。
    fn right_nos(layout: &DiffLayout) -> Vec<usize> {
        layout
            .right_column()
            .into_iter()
            .map(|cell| cell.no)
            .collect()
    }

    #[test]
    fn a_paired_hunk_produces_two_columns() {
        let layout = parse(SAMPLE);
        assert!(layout.is_split(), "能配对 ⇒ 必须是左右分栏");

        assert_eq!(
            left_texts(&layout),
            vec!["-old".to_owned(), " ctx".to_owned()],
            "左栏（旧文件）应含删除行 `-old` 与上下文行"
        );
        assert_eq!(
            right_texts(&layout),
            vec!["+new".to_owned(), " ctx".to_owned()],
            "右栏（新文件）应含新增行 `+new` 与上下文行"
        );
    }

    #[test]
    fn the_context_line_appears_on_both_sides() {
        let layout = parse(SAMPLE);
        let left = left_texts(&layout);
        let right = right_texts(&layout);
        assert!(left.iter().any(|t| t.trim() == "ctx"), "上下文行必须在左侧");
        assert!(
            right.iter().any(|t| t.trim() == "ctx"),
            "上下文行必须在右侧"
        );
    }

    #[test]
    fn each_column_has_its_own_continuous_line_numbers() {
        let layout = parse(SAMPLE);
        assert_eq!(
            left_nos(&layout),
            vec![1, 2],
            "左栏行号来自旧文件，各自从 1 起连续"
        );
        assert_eq!(
            right_nos(&layout),
            vec![1, 2],
            "右栏行号来自新文件，各自从 1 起连续"
        );
    }

    #[test]
    fn a_later_hunk_keeps_the_real_file_line_numbers() {
        // 第二段从第 10 行开始 ⇒ 行号必须**跟着文件走**，不能重新从 1 数。
        let text = "--- a\n+++ b\n@@ -10,1 +10,1 @@\n-old10\n+new10\n";
        let layout = parse(text);
        assert!(layout.is_split());
        assert_eq!(left_nos(&layout), vec![10]);
        assert_eq!(right_nos(&layout), vec![10]);
    }

    #[test]
    fn paired_changes_paint_red_and_green() {
        let layout = parse(SAMPLE);
        assert!(
            layout.paints_pair_colors(),
            "既有删除又有新增 ⇒ 允许画红绿底"
        );

        let blocks = match &layout {
            DiffLayout::Split { blocks } => blocks.clone(),
            DiffLayout::Single { .. } => panic!("必须是分栏形态"),
        };
        let row = blocks
            .iter()
            .find_map(|block| match block {
                DiffBlock::Row(row) => Some(row.clone()),
                DiffBlock::Header(_) => None,
            })
            .expect("样例里至少有一行对照");
        assert_eq!(
            row.left.as_ref().map(|c| c.tone),
            Some(CellTone::Removed),
            "左栏删除行标红语义"
        );
        assert_eq!(
            row.right.as_ref().map(|c| c.tone),
            Some(CellTone::Added),
            "右栏新增行标绿语义"
        );
    }

    #[test]
    fn a_one_sided_diff_falls_back_to_a_single_column_without_pair_colors() {
        // 只有删除行、没有新增行 ⇒ 红绿不成对 ⇒ 退回单列（fail-closed）。
        let layout = parse("--- a\n+++ b\n@@ -1,1 +1,1 @@\n-old\n");
        assert!(!layout.is_split(), "只有单侧有行 ⇒ 退回单列");
        assert!(!layout.paints_pair_colors(), "不成对时不画红绿底");
        assert!(layout.left_column().is_empty(), "单列形态没有分栏内容");
    }

    #[test]
    fn an_unparsable_diff_falls_back_to_a_single_column_without_losing_lines() {
        // 不是 diff：没有段头 ⇒ 不知道两侧行号 ⇒ 退回单列。
        let layout = parse("这根本不是 diff\n只是普通文本\n");
        assert!(!layout.is_split(), "解析不出成对行 ⇒ 退回单列");
        match &layout {
            DiffLayout::Single { lines } => {
                assert_eq!(lines.len(), 2, "一行都不能丢");
                assert_eq!(lines[0].no, 1, "单列行号从 1 起连续");
                assert_eq!(lines[1].no, 2);
            }
            DiffLayout::Split { .. } => panic!("必须是单列形态"),
        }
    }

    #[test]
    fn a_broken_hunk_header_falls_back_to_a_single_column() {
        // 段头解析不出两侧行号 ⇒ fail-closed。
        let text = "--- a\n+++ b\n@@ 乱码 @@\n-old\n+new\n";
        let layout = parse(text);
        assert!(!layout.is_split(), "段头解析失败 ⇒ 退回单列");
        match &layout {
            DiffLayout::Single { lines } => {
                assert_eq!(lines.len(), 5, "一行都不能丢");
                assert_eq!(lines[0].no, 1, "单列行号从 1 起连续");
            }
            DiffLayout::Split { .. } => panic!("必须是单列形态"),
        }
    }

    #[test]
    fn an_empty_diff_yields_an_empty_single_column() {
        let layout = parse("");
        assert!(!layout.is_split());
        match &layout {
            DiffLayout::Single { lines } => assert!(lines.is_empty()),
            DiffLayout::Split { .. } => panic!("必须是单列形态"),
        }
    }

    #[test]
    fn the_no_newline_marker_is_not_treated_as_content() {
        // `\ No newline at end of file` 是标记行：不占行号、不进对照行。
        let text = "--- a\n+++ b\n@@ -1,1 +1,1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n";
        let layout = parse(text);
        assert!(layout.is_split());
        assert_eq!(left_nos(&layout), vec![1], "标记行不占旧文件行号");
        assert_eq!(right_nos(&layout), vec![1], "标记行不占新文件行号");
        assert_eq!(left_texts(&layout), vec!["-old".to_owned()]);
        assert_eq!(right_texts(&layout), vec!["+new".to_owned()]);
    }

    #[test]
    fn several_hunks_keep_both_sides_numbered_per_file() {
        // 两段 hunk ⇒ 行号**按文件**累计，两栏各自连续。
        let text = "--- a\n+++ b\n@@ -1,2 +1,2 @@\n ctx1\n-old\n+new\n@@ -10,2 +10,2 @@\n ctx2\n-old2\n+new2\n";
        let layout = parse(text);
        assert!(layout.is_split());
        assert_eq!(left_nos(&layout), vec![1, 2, 10, 11]);
        assert_eq!(right_nos(&layout), vec![1, 2, 10, 11]);
    }

    #[test]
    fn more_removals_than_additions_stay_paired_row_by_row() {
        // 删两行、加一行 ⇒ 两行对照（第二行左侧有、右侧空）。
        let text = "--- a\n+++ b\n@@ -1,2 +1,1 @@\n-old1\n-old2\n+new1\n";
        let layout = parse(text);
        assert!(layout.is_split());
        assert_eq!(left_nos(&layout), vec![1, 2]);
        assert_eq!(right_nos(&layout), vec![1]);
    }

    // === 文件点击意图（A4） ===

    #[test]
    fn a_file_card_click_produces_a_preview_intent() {
        let mut view = SessionStreamGpui::new(SessionStreamState::new());
        assert!(
            view.file_intent().is_none(),
            "默认态必须干净（没点过就没有跳转意图）"
        );

        record_file_click(&mut view, "src/main.rs", FileIntentSource::FileCard);

        let intent = view.file_intent().expect("点文件卡片 ⇒ 产生跳转意图");
        assert_eq!(intent.path, "src/main.rs");
        assert_eq!(intent.source, FileIntentSource::FileCard);
    }

    #[test]
    fn clicking_another_file_updates_the_intent_instead_of_accumulating() {
        let mut view = SessionStreamGpui::new(SessionStreamState::new());
        record_file_click(&mut view, "a.rs", FileIntentSource::FileCard);
        record_file_click(&mut view, "b.rs", FileIntentSource::FileList);

        let intent = view.file_intent().expect("第二次点击后仍有意图");
        assert_eq!(intent.path, "b.rs", "意图更新为新文件");
        assert_eq!(intent.source, FileIntentSource::FileList);
    }

    #[test]
    fn clicking_a_file_list_row_also_produces_an_intent() {
        let mut view = SessionStreamGpui::new(SessionStreamState::new());
        record_file_click(&mut view, "docs/readme.md", FileIntentSource::FileList);

        let intent = view.file_intent().expect("点文件列表行 ⇒ 产生跳转意图");
        assert_eq!(intent.path, "docs/readme.md");
        assert_eq!(intent.source, FileIntentSource::FileList);
    }

    #[test]
    fn a_file_item_reports_whether_it_is_clickable() {
        let mut item = FileItem::new("src/main.rs");
        assert!(is_clickable(&item), "描述层默认的文件条目可点");
        item.clickable = false;
        assert!(!is_clickable(&item), "描述层说不可点就不可点");
    }

    // === 渲染分支冒烟（只保证「不 panic」） ===

    #[test]
    fn the_diff_branch_renders_both_forms_without_panicking() {
        let theme = gpui_kit::component::Theme::default();
        // 分栏形态。
        let _ = super::super::diff_split_view::render_diff_split(SAMPLE, &theme);
        // fail-closed 的单列形态。
        let _ = super::super::diff_split_view::render_diff_split("不是 diff\n", &theme);
        // 空输入。
        let _ = super::super::diff_split_view::render_diff_split("", &theme);
    }
}
