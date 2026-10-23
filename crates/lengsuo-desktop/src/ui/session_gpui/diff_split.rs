//! diff **左右分栏**的纯逻辑（描述层，不碰 GPUI）。
//!
//! 对齐基准：`docs/ui-parity-spec.md` 的 **A3**（Edit diff 红绿正确）。
//! 参照平台：react-diff-viewer 的**左右分栏**（旧文件在左、新文件在右）。
//!
//! 分层：
//! - 本文件（描述层）：把 diff 原文算成「两栏对照」或「单列」；
//! - [`super::diff_split_view`]（接线层）：把对照画成 GPUI 元素，
//!   **不重新实现**任何判断。
//!
//! 硬约束：
//! - ⛔ **不引 diff 库**（`similar` / `diffy` 一律不许）：行号与配对**自己算**。
//! - **fail-closed**：解析不出成对行 ⇒ 退回**单列**（宁可保守，不显示错位对照）。
//! - **红绿成对**：同屏既有删除又有新增时才画红绿底；只有单侧有行 ⇒ 退回单列。
//! - 只读复用 [`super::diff_lines`]（`classify` / `split`）——⛔ 不改那个文件。

use super::diff_lines::{classify, split, DiffLine, DiffLineKind};

/// 单元格的红绿着色语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CellTone {
    /// 普通（上下文行）——⛔ 不画红绿底。
    Plain,
    /// 删除行（左栏**红底**）。
    Removed,
    /// 新增行（右栏**绿底**）。
    Added,
}

/// 单栏里的一个单元格（一行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiffCell {
    /// 行号：左栏 = **旧文件**行号，右栏 = **新文件**行号。
    pub(super) no: usize,
    /// 原文（保留 diff 前缀）。
    pub(super) text: String,
    /// 行类别（复用 [`super::diff_lines::classify`] 的判定）。
    pub(super) kind: DiffLineKind,
    /// 红绿对照的着色语义。
    pub(super) tone: CellTone,
}

impl DiffCell {
    /// 造一个普通单元格。
    fn plain(no: usize, text: &str, kind: DiffLineKind) -> Self {
        Self {
            no,
            text: text.to_owned(),
            kind,
            tone: CellTone::Plain,
        }
    }
}

/// 一行对照：左栏（旧文件）+ 右栏（新文件）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiffRow {
    /// 左栏单元格（旧文件行号）；该行在旧文件里没有对应行时为 `None`。
    pub(super) left: Option<DiffCell>,
    /// 右栏单元格（新文件行号）；该行在新文件里没有对应行时为 `None`。
    pub(super) right: Option<DiffCell>,
}

/// 分栏里的一段：**跨栏标题** 或 **一行对照**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DiffBlock {
    /// 跨两栏的标题行（文件头 `--- a` / `+++ b`、段头 `@@ … @@`、其它杂项行）。
    Header(String),
    /// 一行左右对照。
    Row(DiffRow),
}

/// diff 的整体形态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum DiffLayout {
    /// **左右分栏**：删除行在左、新增行在右、上下文行两侧都有。
    Split {
        /// 分栏内容（按出现顺序）。
        blocks: Vec<DiffBlock>,
    },
    /// **单列**：fail-closed 退回形态（逐行，行号从 1 起连续）。
    Single {
        /// 单列内容（就是 [`super::diff_lines::split`] 的产物，一行不丢）。
        lines: Vec<DiffLine>,
    },
}

impl DiffLayout {
    /// 是否是左右分栏形态。
    #[cfg(test)]
    pub(super) fn is_split(&self) -> bool {
        matches!(self, DiffLayout::Split { .. })
    }

    /// 本次布局**是否允许**画红绿底。
    ///
    /// ⛔ 红绿必须**成对**：同屏既有删除又有新增才画；只有单侧有行 ⇒ 不画。
    pub(super) fn paints_pair_colors(&self) -> bool {
        match self {
            DiffLayout::Split { blocks } => {
                let mut added = false;
                let mut removed = false;
                for block in blocks {
                    if let DiffBlock::Row(row) = block {
                        if let Some(cell) = &row.left {
                            removed |= cell.kind == DiffLineKind::Removed;
                        }
                        if let Some(cell) = &row.right {
                            added |= cell.kind == DiffLineKind::Added;
                        }
                    }
                }
                added && removed
            }
            DiffLayout::Single { .. } => false,
        }
    }

    /// **左栏**（旧文件）自上而下的单元格。
    #[cfg(test)]
    pub(super) fn left_column(&self) -> Vec<&DiffCell> {
        match self {
            DiffLayout::Split { blocks } => blocks
                .iter()
                .filter_map(|block| match block {
                    DiffBlock::Row(row) => row.left.as_ref(),
                    DiffBlock::Header(_) => None,
                })
                .collect(),
            DiffLayout::Single { .. } => Vec::new(),
        }
    }

    /// **右栏**（新文件）自上而下的单元格。
    #[cfg(test)]
    pub(super) fn right_column(&self) -> Vec<&DiffCell> {
        match self {
            DiffLayout::Split { blocks } => blocks
                .iter()
                .filter_map(|block| match block {
                    DiffBlock::Row(row) => row.right.as_ref(),
                    DiffBlock::Header(_) => None,
                })
                .collect(),
            DiffLayout::Single { .. } => Vec::new(),
        }
    }
}

/// 把 diff 原文算成渲染描述。
///
/// - 解析得出**成对行** ⇒ [`DiffLayout::Split`]（左右分栏）；
/// - 解析失败 / 只有单侧有行 ⇒ [`DiffLayout::Single`]（fail-closed 退回单列）。
pub(super) fn parse(text: &str) -> DiffLayout {
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks: Vec<DiffBlock> = Vec::new();
    let mut removed: Vec<DiffCell> = Vec::new();
    let mut added: Vec<DiffCell> = Vec::new();
    let mut old_no = 0usize;
    let mut new_no = 0usize;
    let mut in_hunk = false;
    let mut saw_hunk = false;
    let mut index = 0usize;

    while index < lines.len() {
        let line = lines[index];

        // 新文件段：`diff --git …` / `index …` ⇒ 后面紧跟文件头。
        if line.starts_with("diff ") || line.starts_with("index ") {
            flush_changes(&mut removed, &mut added, &mut blocks);
            in_hunk = false;
            blocks.push(DiffBlock::Header(line.to_owned()));
            index += 1;
            continue;
        }

        // 段头 `@@ -a,b +c,d @@` ⇒ 重置两侧行号。
        if line.starts_with("@@") {
            let Some((old_start, new_start)) = parse_hunk_header(line) else {
                // 段头解析不出来 ⇒ 格式不可信 ⇒ fail-closed 退回单列。
                return single(text);
            };
            flush_changes(&mut removed, &mut added, &mut blocks);
            old_no = old_start;
            new_no = new_start;
            in_hunk = true;
            saw_hunk = true;
            blocks.push(DiffBlock::Header(line.to_owned()));
            index += 1;
            continue;
        }

        if !in_hunk {
            // 段外的一律当跨栏标题：文件头 `--- a` / `+++ b`、模式行、`Binary files …` 等。
            flush_changes(&mut removed, &mut added, &mut blocks);
            blocks.push(DiffBlock::Header(line.to_owned()));
            index += 1;
            continue;
        }

        // `\ No newline at end of file` 是**标记行**，不是内容行：
        // 不占行号、不进对照行，跨栏显示。
        if line.starts_with('\\') {
            flush_changes(&mut removed, &mut added, &mut blocks);
            blocks.push(DiffBlock::Header(line.to_owned()));
            index += 1;
            continue;
        }

        // 段内正文行：按 unified diff 的**首字符语义**判定。
        match classify(line) {
            DiffLineKind::Header => {
                // 段内的 `--- x` / `+++ x` 其实是「内容以 `--` / `++` 开头的增删行」。
                if line.starts_with('-') {
                    removed.push(DiffCell::plain(old_no, line, DiffLineKind::Removed));
                    old_no += 1;
                } else if line.starts_with('+') {
                    added.push(DiffCell::plain(new_no, line, DiffLineKind::Added));
                    new_no += 1;
                } else {
                    blocks.push(DiffBlock::Header(line.to_owned()));
                }
                index += 1;
            }
            DiffLineKind::Added => {
                added.push(DiffCell::plain(new_no, line, DiffLineKind::Added));
                new_no += 1;
                index += 1;
            }
            DiffLineKind::Removed => {
                removed.push(DiffCell::plain(old_no, line, DiffLineKind::Removed));
                old_no += 1;
                index += 1;
            }
            DiffLineKind::Context => {
                flush_changes(&mut removed, &mut added, &mut blocks);
                blocks.push(DiffBlock::Row(DiffRow {
                    left: Some(DiffCell::plain(old_no, line, DiffLineKind::Context)),
                    right: Some(DiffCell::plain(new_no, line, DiffLineKind::Context)),
                }));
                old_no += 1;
                new_no += 1;
                index += 1;
            }
            // 段内出现没有前缀的普通行 ⇒ 格式不可信 ⇒ fail-closed 退回单列。
            DiffLineKind::Plain | DiffLineKind::Hunk => return single(text),
        }
    }
    flush_changes(&mut removed, &mut added, &mut blocks);

    // 没有段头 ⇒ 不知道两侧行号 ⇒ 退回单列。
    if !saw_hunk {
        return single(text);
    }

    // 红绿必须成对：只有单侧有行 ⇒ 不画红绿底 ⇒ 退回单列（fail-closed）。
    let has_added = count_kind(&blocks, Side::Right, DiffLineKind::Added) > 0;
    let has_removed = count_kind(&blocks, Side::Left, DiffLineKind::Removed) > 0;
    if has_added != has_removed {
        return single(text);
    }

    // 成对 ⇒ 落红绿语义（颜色由渲染层从主题取，这里只标语义）。
    for block in &mut blocks {
        if let DiffBlock::Row(row) = block {
            if let Some(cell) = row.left.as_mut() {
                if cell.kind == DiffLineKind::Removed {
                    cell.tone = CellTone::Removed;
                }
            }
            if let Some(cell) = row.right.as_mut() {
                if cell.kind == DiffLineKind::Added {
                    cell.tone = CellTone::Added;
                }
            }
        }
    }

    DiffLayout::Split { blocks }
}

/// 单列形态（fail-closed 退回）：行号从 1 起连续，**一行不丢**。
fn single(text: &str) -> DiffLayout {
    DiffLayout::Single { lines: split(text) }
}

/// 哪一侧。
#[derive(Debug, Clone, Copy)]
enum Side {
    /// 左栏（旧文件）。
    Left,
    /// 右栏（新文件）。
    Right,
}

/// 数某一侧某类别的行数。
fn count_kind(blocks: &[DiffBlock], side: Side, kind: DiffLineKind) -> usize {
    blocks
        .iter()
        .filter_map(|block| match block {
            DiffBlock::Row(row) => match side {
                Side::Left => row.left.as_ref(),
                Side::Right => row.right.as_ref(),
            },
            DiffBlock::Header(_) => None,
        })
        .filter(|cell| cell.kind == kind)
        .count()
}

/// 把一段连续的删除行 + 一段连续的新增行配成左右对照行。
fn flush_changes(
    removed: &mut Vec<DiffCell>,
    added: &mut Vec<DiffCell>,
    blocks: &mut Vec<DiffBlock>,
) {
    let count = removed.len().max(added.len());
    for i in 0..count {
        blocks.push(DiffBlock::Row(DiffRow {
            left: removed.get(i).cloned(),
            right: added.get(i).cloned(),
        }));
    }
    removed.clear();
    added.clear();
}

/// 解析段头 `@@ -old_start[,old_count] +new_start[,new_count] @@`。
///
/// ⛔ 解析不出来就返回 `None`（调用方据此 fail-closed）。
fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    let rest = line.strip_prefix("@@")?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('-')?;
    let (old_start, rest) = parse_range(rest)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('+')?;
    let (new_start, _) = parse_range(rest)?;
    Some((old_start, new_start))
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`），理由见 mod.rs 测试段。
    use super::{parse, CellTone, DiffBlock, DiffLayout, DiffRow};

    /// A3 ①③④：**单行**对照里行号出现两次（左旧右新）、红绿**同一行**成对、上下文双侧都有。
    #[test]
    fn each_row_carries_both_numbers_and_paired_tones() {
        let layout = parse("--- a\n+++ b\n@@ -1,2 +1,2 @@\n-old\n+new\n ctx\n");
        let rows: Vec<&DiffRow> = match &layout {
            DiffLayout::Split { blocks } => blocks
                .iter()
                .filter_map(|block| match block {
                    DiffBlock::Row(row) => Some(row),
                    DiffBlock::Header(_) => None,
                })
                .collect(),
            DiffLayout::Single { .. } => panic!("样例必须是分栏形态"),
        };
        assert_eq!(rows.len(), 2, "替换行 + 上下文行");
        // 替换行：同一行左右**两个行号**、红绿成对。
        assert_eq!(rows[0].left.as_ref().map(|c| c.no), Some(1), "左栏旧文件行号");
        assert_eq!(rows[0].right.as_ref().map(|c| c.no), Some(1), "右栏新文件行号");
        assert_eq!(
            rows[0].left.as_ref().map(|c| c.tone),
            Some(CellTone::Removed),
            "同一行左删 ⇒ 红"
        );
        assert_eq!(
            rows[0].right.as_ref().map(|c| c.tone),
            Some(CellTone::Added),
            "同一行右增 ⇒ 绿"
        );
        // 上下文行：双侧都有、着色 Plain。
        assert!(rows[1].left.is_some() && rows[1].right.is_some(), "上下文双侧都有");
        assert_eq!(rows[1].left.as_ref().map(|c| c.no), Some(2));
        assert_eq!(rows[1].right.as_ref().map(|c| c.no), Some(2));
    }
}

/// 解析 `1` 或 `1,2` 这样的范围，返回起始值与剩余文本。
fn parse_range(text: &str) -> Option<(usize, &str)> {
    let digits_end = text
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(text.len());
    if digits_end == 0 {
        return None;
    }
    let start: usize = text[..digits_end].parse().ok()?;
    let rest = &text[digits_end..];
    match rest.strip_prefix(',') {
        Some(after_comma) => {
            let count_end = after_comma
                .find(|ch: char| !ch.is_ascii_digit())
                .unwrap_or(after_comma.len());
            if count_end == 0 {
                return None;
            }
            // 计数必须能解析成数字（解析不了说明格式不可信）。
            after_comma[..count_end].parse::<usize>().ok()?;
            Some((start, &after_comma[count_end..]))
        }
        None => Some((start, rest)),
    }
}
