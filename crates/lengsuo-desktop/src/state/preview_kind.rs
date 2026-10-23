//! 预览类型判定：按扩展名把「待预览文件」分成类型，供右面板分派渲染。
//!
//! 纯函数、无 GPUI 依赖（可在无窗口环境测）。
//! ⛔ 只做**类型判定**，不读文件、不解析内容（解码器属 A14 后续卡，需依赖授权）。

/// 预览类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    /// 纯文本 / Markdown 文档。
    Text,
    /// 源代码。
    Code,
    /// 图片。
    Image,
    /// PDF。
    Pdf,
    /// 办公文档（文档 / 表格 / 演示）。
    Office,
    /// 图表 / 白板等图形描述。
    Diagram,
    /// 其他未知类型。
    Other,
}

impl PreviewKind {
    /// 类型的界面标签。
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Text => "文本",
            Self::Code => "代码",
            Self::Image => "图片",
            Self::Pdf => "PDF",
            Self::Office => "办公文档",
            Self::Diagram => "图表",
            Self::Other => "其他",
        }
    }
}

/// 按扩展名判定预览类型（大小写不敏感）。
///
/// 没有扩展名、或扩展名落在目录分隔符之后（如 `dir.with.dot/file`）时，判为 [`PreviewKind::Other`]。
#[must_use]
pub fn classify(path: &str) -> PreviewKind {
    let Some((stem, ext)) = path.rsplit_once('.') else {
        return PreviewKind::Other;
    };
    if stem.is_empty() || ext.is_empty() || stem.ends_with('/') || stem.ends_with('\\') {
        return PreviewKind::Other;
    }
    match ext.to_ascii_lowercase().as_str() {
        "md" | "markdown" | "txt" | "log" => PreviewKind::Text,
        "rs" | "toml" | "json" | "yaml" | "yml" | "js" | "mjs" | "ts" | "tsx" | "jsx" | "py"
        | "go" | "java" | "c" | "h" | "cpp" | "hpp" | "sh" | "bash" | "zsh" | "ps1" | "sql"
        | "html" | "css" | "xml" | "ini" | "cfg" => PreviewKind::Code,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico" => PreviewKind::Image,
        "pdf" => PreviewKind::Pdf,
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "csv" | "tsv" => PreviewKind::Office,
        "mmd" | "mermaid" | "excalidraw" | "drawio" => PreviewKind::Diagram,
        _ => PreviewKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_covers_every_kind() {
        assert_eq!(classify("notes.md"), PreviewKind::Text);
        assert_eq!(classify("src/main.rs"), PreviewKind::Code);
        assert_eq!(classify("shot.PNG"), PreviewKind::Image);
        assert_eq!(classify("spec.pdf"), PreviewKind::Pdf);
        assert_eq!(classify("book.xlsx"), PreviewKind::Office);
        assert_eq!(classify("flow.mmd"), PreviewKind::Diagram);
        assert_eq!(classify("data.bin"), PreviewKind::Other);
    }

    #[test]
    fn classify_is_path_safe() {
        // 无扩展名 / 空扩展名 / 点号落在目录段 ⇒ 其他，不得误判成代码。
        assert_eq!(classify("Makefile"), PreviewKind::Other);
        assert_eq!(classify("a/b/c"), PreviewKind::Other);
        assert_eq!(classify("dir.with.dot/file"), PreviewKind::Other);
        assert_eq!(classify(".hidden"), PreviewKind::Other);
    }
}
