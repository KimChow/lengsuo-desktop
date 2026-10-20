//! 轻量 Markdown 子集解析（纯函数，不碰 GPUI）。
//!
//! **不是完整 CommonMark**：只认下面这些语法，其余一律原样输出为文本。
//!
//! 块级：标题（`#`~`###`）、无序列表（`- ` / `* ` / `+ `）、有序列表（`1. `）、
//! 围栏代码块（```` ``` ```` 与 `~~~`）。多行连续文本合并为段落。
//! 行内：`**粗体**`、`` `code` ``、`*斜体*`。链接 / 图片 / 表格 / 内嵌 HTML /
//! 引用块**不支持**，原样保留。
//!
//! fail-closed：未闭合围栏、超长行 ⇒ 整个输入回退为单个纯文本段落，
//! **不丢内容、不渲染成空白**。

/// 允许解析的最大行数；超出即整体回退纯文本。
pub(super) const MAX_LINES: usize = 20_000;
/// 允许解析的单行最大字符数；超出即整体回退纯文本。
pub(super) const MAX_LINE_CHARS: usize = 4_096;

/// 一个 Markdown 块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MarkdownBlock {
    /// 段落（行内语法已解析）。
    Paragraph(Vec<InlineSpan>),
    /// 标题，`level` 为 1~3。
    Heading { level: u8, spans: Vec<InlineSpan> },
    /// 列表项；`number` 为有序列表序号（无序为 `None`），可含多个行内片段。
    ListItem {
        number: Option<u64>,
        spans: Vec<InlineSpan>,
    },
    /// 围栏代码块（内容原样，不解析行内）。
    CodeBlock(String),
}

/// 行内片段（只承担「长什么样」，颜色与字重由渲染层决定）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InlineSpan {
    Text(String),
    Strong(String),
    Emphasis(String),
    Code(String),
}

impl InlineSpan {
    /// 片段的纯文本（仅供测试断言）。
    #[cfg(test)]
    #[must_use]
    pub(super) fn text(&self) -> &str {
        match self {
            Self::Text(s) | Self::Strong(s) | Self::Emphasis(s) | Self::Code(s) => s,
        }
    }
}

/// 把一段正文切成块。
///
/// 空输入 / 只有换行 ⇒ 空列表。遇到无法安全处理的输入（见模块文档）⇒
/// 回退为 `[Paragraph([Text(原文)])]`。
#[must_use]
pub(super) fn parse(input: &str) -> Vec<MarkdownBlock> {
    if input.trim().is_empty() {
        return Vec::new();
    }
    if input.lines().count() > MAX_LINES
        || input.lines().any(|l| l.chars().count() > MAX_LINE_CHARS)
    {
        return fallback(input);
    }
    let mut blocks = Vec::new();
    let mut lines = input.lines();
    while let Some(line) = lines.next() {
        if let Some((marker, _)) = fence(line) {
            let mut body = String::new();
            let mut closed = false;
            for inner in lines.by_ref() {
                if fence(inner).is_some_and(|(m, _)| m == marker) {
                    closed = true;
                    break;
                }
                body.push_str(inner);
                body.push('\n');
            }
            if !closed {
                // 未闭合围栏：不猜边界，整段回退纯文本。
                return fallback(input);
            }
            blocks.push(MarkdownBlock::CodeBlock(body));
        } else if let Some((level, text)) = heading(line) {
            blocks.push(MarkdownBlock::Heading {
                level,
                spans: parse_inline(text),
            });
        } else if let Some((number, text)) = list_item(line) {
            blocks.push(MarkdownBlock::ListItem {
                number,
                spans: parse_inline(text),
            });
        } else if line.trim().is_empty() {
            // 空行分隔段落。
        } else {
            blocks.push(MarkdownBlock::Paragraph(parse_inline(line)));
        }
    }
    blocks
}

/// 整体回退为纯文本。
fn fallback(input: &str) -> Vec<MarkdownBlock> {
    vec![MarkdownBlock::Paragraph(vec![InlineSpan::Text(
        input.to_owned(),
    )])]
}

/// 识别围栏行，返回 `(围栏字符, 剩余字符数)`。
fn fence(line: &str) -> Option<(char, usize)> {
    let trimmed = line.trim_start();
    for marker in ['`', '~'] {
        let count = trimmed.chars().take_while(|c| *c == marker).count();
        if count >= 3 {
            return Some((marker, count));
        }
    }
    None
}

/// 识别标题行（`#`~`###` 后必须跟空格）。
fn heading(line: &str) -> Option<(u8, &str)> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if !(1..=3).contains(&level) {
        return None;
    }
    let rest = &line[level..];
    rest.strip_prefix(' ')
        .map(|text| (u8::try_from(level).unwrap_or(3), text))
}

/// 识别列表项，返回 `(有序序号, 项文本)`。
fn list_item(line: &str) -> Option<(Option<u64>, &str)> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return None;
    }
    for marker in ["- ", "* ", "+ "] {
        if let Some(text) = trimmed.strip_prefix(marker) {
            return Some((None, text));
        }
    }
    let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = &trimmed[digits..];
    let text = rest.strip_prefix(". ")?;
    Some((trimmed[..digits].parse().ok(), text))
}

/// 解析一行行内语法。
///
/// 某个 `**` / `` ` `` / `*` 找不到配对时，该标记**原样保留**为文本。
#[must_use]
pub(super) fn parse_inline(line: &str) -> Vec<InlineSpan> {
    let mut spans = Vec::new();
    let mut buf = String::new();
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        let width = c.len_utf8();
        let (open, close, kind): (&str, &str, fn(String) -> InlineSpan) = if rest.starts_with("**")
        {
            ("**", "**", InlineSpan::Strong)
        } else if rest.starts_with('`') {
            ("`", "`", InlineSpan::Code)
        } else if rest.starts_with('*') {
            ("*", "*", InlineSpan::Emphasis)
        } else {
            buf.push(c);
            rest = &rest[width..];
            continue;
        };
        let after = &rest[open.len()..];
        let found = after.match_indices(close).next().map(|(i, _)| i);
        match found {
            Some(end) if end > 0 => {
                flush(&mut buf, &mut spans);
                spans.push(kind(after[..end].to_owned()));
                rest = &after[end + close.len()..];
            }
            // 不支持的 / 未配对的标记：原样保留。
            _ => {
                buf.push_str(open);
                rest = after;
            }
        }
    }
    flush(&mut buf, &mut spans);
    spans
}

fn flush(buf: &mut String, spans: &mut Vec<InlineSpan>) {
    if !buf.is_empty() {
        spans.push(InlineSpan::Text(std::mem::take(buf)));
    }
}

#[cfg(test)]
mod tests {
    // 显式导入（不用 `use super::*`），理由见 mod.rs 测试段。
    use super::{parse, parse_inline, InlineSpan, MarkdownBlock, MAX_LINES};

    fn plain(text: &str) -> Vec<InlineSpan> {
        vec![InlineSpan::Text(text.to_owned())]
    }

    #[test]
    fn strong_code_and_emphasis_spans() {
        assert_eq!(
            parse_inline("a **b** c"),
            vec![
                InlineSpan::Text("a ".into()),
                InlineSpan::Strong("b".into()),
                InlineSpan::Text(" c".into()),
            ]
        );
        assert_eq!(
            parse_inline("run `cargo test` now"),
            vec![
                InlineSpan::Text("run ".into()),
                InlineSpan::Code("cargo test".into()),
                InlineSpan::Text(" now".into()),
            ]
        );
        assert_eq!(
            parse_inline("*斜体*"),
            vec![InlineSpan::Emphasis("斜体".into())]
        );
    }

    #[test]
    fn unsupported_link_is_kept_verbatim() {
        let spans = parse_inline("看 [链接](https://x) 吧");
        assert_eq!(spans, plain("看 [链接](https://x) 吧"));
        let sp = parse_inline("**未闭合 与 `code");
        assert_eq!(sp, plain("**未闭合 与 `code"));
    }

    #[test]
    fn headings_and_paragraphs() {
        let blocks = parse("# 标题\n\n正文 **粗**");
        assert_eq!(
            blocks,
            vec![
                MarkdownBlock::Heading {
                    level: 1,
                    spans: plain("标题")
                },
                MarkdownBlock::Paragraph(vec![
                    InlineSpan::Text("正文 ".into()),
                    InlineSpan::Strong("粗".into()),
                ]),
            ]
        );
        assert_eq!(
            parse("### 三"),
            vec![MarkdownBlock::Heading {
                level: 3,
                spans: plain("三")
            }]
        );
        // `#` 后无空格不是标题。
        assert_eq!(
            parse("#不是标题"),
            vec![MarkdownBlock::Paragraph(plain("#不是标题"))]
        );
    }

    #[test]
    fn fenced_code_both_markers() {
        let blocks = parse("```rust\nlet x = **1**;\n```");
        assert_eq!(
            blocks,
            vec![MarkdownBlock::CodeBlock("let x = **1**;\n".into())]
        );
        let blocks = parse("~~~\na\n\nb\n~~~");
        assert_eq!(blocks, vec![MarkdownBlock::CodeBlock("a\n\nb\n".into())]);
    }

    #[test]
    fn unclosed_fence_falls_back_to_plain_text() {
        let input = "前言\n```\n内容不会丢\n";
        let blocks = parse(input);
        assert_eq!(blocks, vec![MarkdownBlock::Paragraph(plain(input))]);
        assert!(blocks[0].clone_text().contains("内容不会丢"));
    }

    #[test]
    fn unordered_and_ordered_lists_keep_numbers() {
        assert_eq!(
            parse("- 甲\n- 乙"),
            vec![
                MarkdownBlock::ListItem {
                    number: None,
                    spans: plain("甲")
                },
                MarkdownBlock::ListItem {
                    number: None,
                    spans: plain("乙")
                },
            ]
        );
        let blocks = parse("1. 第一\n2. 第二");
        assert_eq!(
            blocks,
            vec![
                MarkdownBlock::ListItem {
                    number: Some(1),
                    spans: plain("第一")
                },
                MarkdownBlock::ListItem {
                    number: Some(2),
                    spans: plain("第二")
                },
            ]
        );
    }

    #[test]
    fn empty_and_blank_input_yield_no_blocks() {
        assert!(parse("").is_empty());
        assert!(parse("\n\n\n").is_empty());
        assert!(parse("   \n\t\n").is_empty());
    }

    #[test]
    fn overly_long_input_falls_back_without_truncation() {
        let long = "**x**".repeat(2_000); // 10_000 字符
        let blocks = parse(&long);
        assert_eq!(blocks, vec![MarkdownBlock::Paragraph(plain(&long))]);
        assert_eq!(blocks[0].clone_text(), long);

        let long_ascii = "a".repeat(10_000);
        assert_eq!(parse(&long_ascii)[0].clone_text(), long_ascii);

        let many = "a\n".repeat(MAX_LINES + 1);
        let blocks = parse(&many);
        assert_eq!(blocks[0].clone_text(), many);
    }

    impl MarkdownBlock {
        fn clone_text(&self) -> String {
            match self {
                Self::Paragraph(spans) | Self::ListItem { spans, .. } => {
                    spans.iter().map(|s| s.text()).collect()
                }
                Self::Heading { spans, .. } => spans.iter().map(|s| s.text()).collect(),
                Self::CodeBlock(body) => body.clone(),
            }
        }
    }
}
