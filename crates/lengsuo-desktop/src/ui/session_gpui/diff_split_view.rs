//! diff **左右分栏**的 GPUI 渲染分支（接线层）。
//!
//! 对齐基准：`docs/ui-parity-spec.md` 的 **A3**（Edit diff 红绿正确）。
//! 参照平台：react-diff-viewer 的**左右分栏**（旧文件在左、新文件在右）。
//!
//! 硬约束：
//! - 判断全在描述层 [`super::diff_split`]；本文件**只做「描述 → 元素」的映射**。
//! - 颜色一律取自 `cx.theme()`，⛔ 不硬编码色值。
//! - **红绿成对**：只有成对时才画红绿底（[`super::diff_split::DiffLayout::paints_pair_colors`]）。
//! - **fail-closed**：解析不出成对行 ⇒ 退回单列渲染（不显示错位对照）。
//! - 分栏宽度**对半**（两栏各 `flex_1`），**等宽字体**。

use gpui_kit::component::Theme;
use gpui_kit::*;

use super::diff_lines::DiffLineKind;
use super::diff_split::{CellTone, DiffBlock, DiffCell, DiffLayout};

/// 行号栏的宽度（等宽数字，避免行号抖动）。
const NO_W: f32 = 44.0;

/// 红绿底的**不透明度**（底色只做淡染，文字仍是前景色）。
///
/// 亮暗两套主题下都成立：颜色本身取自主题（红 = `danger`、绿 = `success`），
/// 这里只控制**淡染强度**，⛔ 不硬编码色值。
const TINT_ALPHA: f32 = 0.16;

/// 渲染 diff：**左右分栏**（删除在左、新增在右、上下文两侧都有）。
///
/// 解析不出成对行 ⇒ 退回**单列**（fail-closed）。
pub(super) fn render_diff_split(text: &str, theme: &Theme) -> AnyElement {
    let layout = super::diff_split::parse(text);
    match &layout {
        DiffLayout::Split { blocks } => render_split(blocks, &layout, theme),
        DiffLayout::Single { lines } => render_single(lines, theme),
    }
}

/// 左右分栏：两栏各占一半宽，逐行对照。
fn render_split(blocks: &[DiffBlock], layout: &DiffLayout, theme: &Theme) -> AnyElement {
    let pair_colors = layout.paints_pair_colors();
    let mut body = div()
        .flex()
        .flex_col()
        .w_full()
        .font_family(theme.mono_font_family.clone());

    for block in blocks {
        match block {
            DiffBlock::Header(text) => {
                body = body.child(render_header(text, theme));
            }
            DiffBlock::Row(row) => {
                body = body.child(
                    div()
                        .flex()
                        .flex_row()
                        .w_full()
                        .child(render_cell(row.left.as_ref(), true, pair_colors, theme))
                        .child(render_cell(row.right.as_ref(), false, pair_colors, theme)),
                );
            }
        }
    }
    body.into_any_element()
}

/// 跨两栏的标题行（文件头 / 段头 / 杂项行）。
///
/// 段头 `@@ … @@` 用强调色（accent），其余标题行用弱化色（muted_foreground）。
fn render_header(text: &str, theme: &Theme) -> AnyElement {
    let color = if text.starts_with("@@") {
        theme.accent
    } else {
        theme.muted_foreground
    };
    div()
        .flex()
        .flex_row()
        .w_full()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(color)
        .child(text.to_owned())
        .into_any_element()
}

/// 一个单元格（一行）：行号 + 原文；缺失的一侧留**空位**（保持左右对齐）。
///
/// `is_left`：是否左栏（左栏右侧有一条分隔线，把两栏分开）。
fn render_cell(
    cell: Option<&DiffCell>,
    is_left: bool,
    pair_colors: bool,
    theme: &Theme,
) -> AnyElement {
    let Some(cell) = cell else {
        // 空位：宽度与有内容的单元格一致 ⇒ 另一侧不塌陷；
        // 左栏空位仍画分隔线，两栏的列界不因单侧缺失断口。
        let mut hole = div().flex().flex_1().min_w_0().h(px(LINE_H));
        if is_left {
            hole = hole.border_r_1().border_color(theme.border);
        }
        return hole.into_any_element();
    };

    let mut row = div()
        .flex()
        .flex_1()
        .min_w_0()
        .flex_row()
        .items_center()
        .h(px(LINE_H));
    if is_left {
        row = row.border_r_1().border_color(theme.border);
    }

    // 行号（各自连续：左栏旧文件行号、右栏新文件行号）。
    row = row.child(
        div()
            .w(px(NO_W))
            .flex_none()
            .text_xs()
            .text_color(theme.muted_foreground)
            .text_right()
            .pr_1()
            .child(format!("{}", cell.no)),
    );

    // 原文：等宽字体；红绿底**只**在成对时画。
    let mut content = div()
        .flex_1()
        .min_w_0()
        .text_xs()
        .whitespace_nowrap()
        .overflow_hidden()
        .text_color(text_color(cell.kind, theme))
        .child(cell.text.clone());

    if pair_colors {
        match cell.tone {
            CellTone::Removed => {
                content = content.bg(theme.danger.alpha(TINT_ALPHA));
            }
            CellTone::Added => {
                content = content.bg(theme.success.alpha(TINT_ALPHA));
            }
            CellTone::Plain => {}
        }
    }
    row.child(content).into_any_element()
}

/// 单列（fail-closed 退回）：逐行渲染，行号从 1 起连续。
fn render_single(lines: &[super::diff_lines::DiffLine], theme: &Theme) -> AnyElement {
    let mut body = div()
        .flex()
        .flex_col()
        .w_full()
        .font_family(theme.mono_font_family.clone());
    for line in lines {
        body = body.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .h(px(LINE_H))
                .text_xs()
                .child(
                    div()
                        .w(px(NO_W))
                        .flex_none()
                        .text_color(theme.muted_foreground)
                        .text_right()
                        .pr_1()
                        .child(format!("{}", line.no)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_color(text_color(line.kind, theme))
                        .child(line.text.clone()),
                ),
        );
    }
    body.into_any_element()
}

/// 行文字颜色（全部取自主题，⛔ 不硬编码）。
fn text_color(kind: DiffLineKind, theme: &Theme) -> Hsla {
    match kind {
        DiffLineKind::Added => theme.success,
        DiffLineKind::Removed => theme.danger,
        DiffLineKind::Hunk => theme.accent,
        DiffLineKind::Header | DiffLineKind::Plain => theme.foreground,
        DiffLineKind::Context => theme.muted_foreground,
    }
}

/// 一行的行高（等宽字体下的固定行高，保证左右两栏行对齐）。
const LINE_H: f32 = 20.0;

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
    // `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。
    use gpui_kit::component::theme::ThemeColor;
    use gpui_kit::Hsla;

    /// 亮色主题下：红底偏红、绿底偏绿，且两者**不等**（可区分）。
    #[test]
    fn the_light_theme_danger_is_red_and_success_is_green() {
        let colors = ThemeColor::light();
        assert_red_green(colors.danger, colors.success, "亮色");
    }

    /// 暗色主题下：红底偏红、绿底偏绿，且两者**不等**（可区分）。
    #[test]
    fn the_dark_theme_danger_is_red_and_success_is_green() {
        let colors = ThemeColor::dark();
        assert_red_green(colors.danger, colors.success, "暗色");
    }

    /// 红绿两色在**亮暗两套主题**下都成立：偏红 / 偏绿 / 互不相同。
    fn assert_red_green(danger: Hsla, success: Hsla, label: &str) {
        let danger = danger.to_rgb();
        let success = success.to_rgb();

        assert!(
            danger.r > danger.g && danger.r > danger.b,
            "{label}主题的 danger 必须偏红（实际 {danger:?}）"
        );
        assert!(
            success.g > success.r && success.g > success.b,
            "{label}主题的 success 必须偏绿（实际 {success:?}）"
        );
        assert_ne!(
            (danger.r, danger.g, danger.b),
            (success.r, success.g, success.b),
            "{label}主题下红绿必须**可区分**（不能是同一个颜色）"
        );
    }
}

/// 无窗口渲染断言（`draw` 跑完整 layout + prepaint + paint）：
/// 分栏画得出、fail-closed 的两种单列形态**元素树非空**且不 panic。
#[cfg(all(test, feature = "ui"))]
mod render_asserts {
    // 显式导入（不用 `use super::*`）：父模块的 `use gpui_kit::*` 会把 GPUI 自己的
    // `test` 属性宏带进作用域，遮蔽标准库的 `#[test]`，导致测试无法编译。
    use gpui_kit::component::Theme;

    use super::super::diff_split::{parse, DiffLayout};
    use super::render_diff_split;
    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};

    /// 任务书样例：删一行 + 加一行 + 一行上下文 ⇒ 分栏形态。
    const SAMPLE: &str = "--- a\n+++ b\n@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n";
    /// 只有删除行 ⇒ 红绿不成对 ⇒ fail-closed 退单列。
    const ONE_SIDED: &str = "--- a\n+++ b\n@@ -1,1 +1,1 @@\n-old\n";

    /// ① 双列分栏真的画得出来（布局宽高非零，两栏分支没被跳过）。
    #[test]
    fn the_split_form_paints_a_non_empty_tree() {
        let bounds = draw_probe(|_| render_diff_split(SAMPLE, &Theme::default()));
        assert_rendered(bounds, "分栏形态的 diff");
    }

    /// ② 单侧缺失 ⇒ 退单列，元素树仍非空（不 panic、不空白）。
    #[test]
    fn a_one_sided_diff_falls_back_to_a_painted_single_column() {
        assert!(
            matches!(parse(ONE_SIDED), DiffLayout::Single { .. }),
            "描述层必须先退单列"
        );
        let bounds = draw_probe(|_| render_diff_split(ONE_SIDED, &Theme::default()));
        assert_rendered(bounds, "单侧缺失退回的单列");
    }

    /// ③ 解析失败 ⇒ 不 panic 且内容仍在（单列一行不丢 + 元素树非空）。
    #[test]
    fn an_unparsable_diff_paints_without_panicking_or_losing_lines() {
        let garbage = "这根本不是 diff\n只是普通文本\n";
        match parse(garbage) {
            DiffLayout::Single { lines } => assert_eq!(lines.len(), 2, "一行都不能丢"),
            DiffLayout::Split { .. } => panic!("解析失败必须是单列形态"),
        }
        let bounds = draw_probe(|_| render_diff_split(garbage, &Theme::default()));
        assert_rendered(bounds, "解析失败退回的单列");
    }
}
