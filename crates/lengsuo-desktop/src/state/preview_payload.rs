//! 预览内容载荷（A14）：**解码结果的数据形状**。
//!
//! 纯数据、无 GPUI 依赖 ⇒ 无窗口也能测。分工：
//! - **解码器**（各 `ui/right_panel_gpui/preview_<kind>.rs` 的 `decode`）产出本模块的类型；
//! - **接线层**（同文件的 `render_with_payload`）把载荷画出来。
//!
//! 硬约束：
//! - **只消费内容**：字节可由运行时下发（`bytes`）；未下发时解码器**自行读本地文件**。
//!   两条路径都 ⛔ **不伪造**：拿不到就返回 [`PreviewPayload::NotDelivered`] / [`PreviewPayload::Failed`]。
//! - **有界**：单次读取 ≤ [`MAX_BYTES`]，文本/代码行数 ≤ [`MAX_LINES`]，超出**截断并如实标注**。
//! - ⛔ 不做权限判定、不持凭据、不联网。

/// 单次预览允许读取的字节上限（8 MiB）。超过即截断并标注，⛔ 不整包进内存。
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// 文本 / 代码预览最多保留的行数。
pub const MAX_LINES: usize = 2_000;

/// 解码失败的**诚实原因**（界面上照实显示，⛔ 不吞掉、⛔ 不伪装成空内容）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// 读不到内容（路径不存在 / 无权限 / io 错误）。
    Io(String),
    /// 超过 [`MAX_BYTES`]（已截断到上限）。
    TooLarge {
        /// 允许的上限字节数。
        limit: usize,
        /// 实际字节数。
        actual: usize,
    },
    /// 解析失败（结构不合法 / 该结构尚不支持）。
    Parse(String),
    /// 该格式还没有解码器。
    Unsupported(String),
}

impl DecodeError {
    /// 界面上显示的一行说明（中文，供接线层直接使用）。
    #[must_use]
    pub fn line(&self) -> String {
        match self {
            Self::Io(msg) => format!("读取失败：{msg}"),
            Self::TooLarge { limit, actual } => {
                format!("文件过大（{actual} 字节 > 上限 {limit} 字节），已截断")
            }
            Self::Parse(msg) => format!("解析失败：{msg}"),
            Self::Unsupported(msg) => format!("暂不支持：{msg}"),
        }
    }
}

/// 图片预览载荷。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImagePayload {
    /// 原文件字节（已按 [`MAX_BYTES`] 截断）；接线层交给 GPUI 图像栈渲染。
    pub bytes: Vec<u8>,
    /// 格式名（`png` / `jpeg` / `gif` / `webp` / `bmp` / `svg`…）。
    pub format: String,
    /// 像素宽（0 = 未解出）。
    pub width: u32,
    /// 像素高（0 = 未解出）。
    pub height: u32,
    /// 原文件总字节数（用于「已截断」标注）。
    pub source_len: usize,
}

/// 纯文本预览载荷。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextPayload {
    /// 行（已按行切分，便于按可视窗口懒渲染）。
    pub lines: Vec<String>,
    /// 是否因超上限被截断。
    pub truncated: bool,
}

/// 代码预览载荷。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CodePayload {
    /// 语言判定结果（按扩展名，如 `rust` / `ts` / `py`；未知为 `text`）。
    pub language: String,
    /// 行。
    pub lines: Vec<String>,
    /// 是否因超上限被截断。
    pub truncated: bool,
}

/// 办公文档种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeKind {
    /// Word 文档（`.docx`）。
    Docx,
    /// 演示文稿（`.pptx`）。
    Pptx,
    /// 表格（`.xlsx`）。
    Xlsx,
    /// 其他 / 未知。
    Unknown,
}

/// 办公文档里一个内容块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeBlock {
    /// 块的种类（决定界面上的排版层级）。
    pub kind: OfficeBlockKind,
    /// 块内文本（已抽取，⛔ 保留原文不加工）。
    pub text: String,
}

/// 办公文档块的种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeBlockKind {
    /// 标题。
    Heading,
    /// 普通段落。
    Paragraph,
    /// 列表项。
    Bullet,
    /// 表格单元格。
    Cell,
    /// 幻灯片 / 工作表标题。
    Slide,
}

/// 办公文档预览载荷。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficePayload {
    /// 文档种类。
    pub kind: OfficeKind,
    /// 抽取出的块序列。
    pub blocks: Vec<OfficeBlock>,
    /// 是否因超上限被截断。
    pub truncated: bool,
}

/// PDF 预览载荷。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PdfPayload {
    /// 页数（从文档结构解出）。
    pub pages: u32,
    /// 文档标题（`/Title`，没有则 `None`）。
    pub title: Option<String>,
    /// 每页抽取的文本（可能为空字符串：纯图页）。
    pub page_text: Vec<String>,
    /// 是否因超上限被截断。
    pub truncated: bool,
}

/// 预览载荷：解码器产出的内容形状。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewPayload {
    /// 内容未下发 / 解码器尚未接入 —— **fail-closed 的诚实态**。
    NotDelivered {
        /// 界面上照实显示的原因。
        reason: &'static str,
    },
    /// 图片。
    Image(ImagePayload),
    /// 纯文本。
    Text(TextPayload),
    /// 代码。
    Code(CodePayload),
    /// 办公文档（docx / pptx / xlsx）。
    Office(OfficePayload),
    /// PDF。
    Pdf(PdfPayload),
    /// 解码失败。
    Failed(DecodeError),
}

/// A14 解码器尚未接入时的统一说明文案。
pub const DECODE_NOT_DELIVERED: &str = "内容解码尚未接入（A14）";

#[cfg(test)]
mod tests {
    use super::{DecodeError, PreviewPayload, DECODE_NOT_DELIVERED};

    #[test]
    fn decode_error_lines_are_honest_not_empty() {
        let cases = [
            DecodeError::Io("missing".to_owned()),
            DecodeError::TooLarge {
                limit: 10,
                actual: 99,
            },
            DecodeError::Parse("bad".to_owned()),
            DecodeError::Unsupported("pdf".to_owned()),
        ];
        for c in cases {
            assert!(!c.line().trim().is_empty(), "{c:?} 必须给出可读说明");
        }
    }

    #[test]
    fn not_delivered_reason_is_not_blank() {
        let p = PreviewPayload::NotDelivered {
            reason: DECODE_NOT_DELIVERED,
        };
        assert!(matches!(p, PreviewPayload::NotDelivered { reason } if !reason.trim().is_empty()));
    }
}
