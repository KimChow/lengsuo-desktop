//! 文本文档预览（`PreviewKind::Text`）——文档区 + 截断 + 未下发态（A14）。
//!
//! 预览内容由运行时契约下发（L2 未就绪）：本模块只做视图与状态，
//! ⛔ 不读文件、不伪造正文；颜色/字体一律取 `theme`，⛔ 不硬编码。
use gpui_kit::*;

/// 截断阈值：内容超过该字符数只展示前缀（10MB 级文件由截断兜底，
/// 分页/懒加载待内容真正下发时再接）。
const MAX_CHARS: usize = 200_000;

/// 内容总长是否超过截断阈值。
fn is_truncated(total: usize) -> bool {
    total > MAX_CHARS
}

/// 截断提示文案；未截断返回 `None`。
fn truncation_note(total: usize) -> Option<&'static str> {
    is_truncated(total).then_some("内容较长，已截断显示。")
}

/// 路径是否按文本文档形态预览（`md`/`markdown`/`txt`/`log`）。
fn is_document(path: &str) -> bool {
    matches!(extension(path), "md" | "markdown" | "txt" | "log")
}

/// md/markdown 按正文字体展示，其余文档扩展名用等宽。
fn is_prose(path: &str) -> bool {
    matches!(extension(path), "md" | "markdown")
}

fn extension(path: &str) -> String {
    path.rsplit_once('.')
        .map_or(String::new(), |(_, ext)| ext.to_ascii_lowercase())
}

/// 渲染文本文档预览：内容未由运行时契约下发 ⇒ 诚实显示未下发态。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(format!("{path}：文本预览尚未实现"))
        .into_any_element()
}
