//! diff 行级分类（纯函数，不碰 GPUI）。
//!
//! 按行首字符分类，并为每行分配**从 1 起**、连续的行号；
//! 颜色映射在渲染层（`blocks::render_diff`），不在本文件。

/// 单行 diff 的语义类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffLineKind {
    /// `+` 新增行。
    Added,
    /// `-` 删除行。
    Removed,
    /// `@@` 段头。
    Hunk,
    /// `+++` / `---` / `diff ` 文件头。
    Header,
    /// 以空格开头或以 `\` 开头的上下文行。
    Context,
    /// 其它（无 diff 前缀的普通行）。
    Plain,
}

/// 一行 diff：行号（从 1 起）+ 类别 + 原文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DiffLine {
    pub no: usize,
    pub kind: DiffLineKind,
    pub text: String,
}

/// 判定单行的类别；头部判定必须先于 `+` / `-`。
#[must_use]
pub(super) fn classify(line: &str) -> DiffLineKind {
    if line.starts_with("@@") {
        DiffLineKind::Hunk
    } else if line.starts_with("+++") || line.starts_with("---") || line.starts_with("diff ") {
        DiffLineKind::Header
    } else if line.starts_with('+') {
        DiffLineKind::Added
    } else if line.starts_with('-') {
        DiffLineKind::Removed
    } else if line.starts_with(' ') || line.starts_with('\\') {
        DiffLineKind::Context
    } else {
        DiffLineKind::Plain
    }
}

/// 把 diff 原文切成带行号的行；空输入 ⇒ 空列表。
#[must_use]
pub(super) fn split(text: &str) -> Vec<DiffLine> {
    text.lines()
        .enumerate()
        .map(|(i, line)| DiffLine {
            no: i + 1,
            kind: classify(line),
            text: line.to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`），理由见 mod.rs 测试段。
    use super::{classify, split, DiffLineKind};

    #[test]
    fn kinds_follow_first_character() {
        assert_eq!(classify("+added"), DiffLineKind::Added);
        assert_eq!(classify("-removed"), DiffLineKind::Removed);
        assert_eq!(classify(" context"), DiffLineKind::Context);
        assert_eq!(classify("\\ No newline"), DiffLineKind::Context);
        assert_eq!(classify("@@ -1 +1 @@"), DiffLineKind::Hunk);
        assert_eq!(classify("+++ b/x"), DiffLineKind::Header);
        assert_eq!(classify("--- a/x"), DiffLineKind::Header);
        assert_eq!(classify(""), DiffLineKind::Plain);
    }

    #[test]
    fn line_numbers_start_at_one_and_are_continuous() {
        let lines = split("--- a\n+++ b\n@@ -1 +1 @@\n-old\n+new\n same\n");
        let nos: Vec<usize> = lines.iter().map(|l| l.no).collect();
        assert_eq!(nos, vec![1, 2, 3, 4, 5, 6]);
        let kinds: Vec<DiffLineKind> = lines.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            vec![
                DiffLineKind::Header,
                DiffLineKind::Header,
                DiffLineKind::Hunk,
                DiffLineKind::Removed,
                DiffLineKind::Added,
                DiffLineKind::Context,
            ]
        );
        assert_eq!(lines[4].text, "+new");
    }

    #[test]
    fn empty_diff_yields_no_lines() {
        assert!(split("").is_empty());
    }
}
