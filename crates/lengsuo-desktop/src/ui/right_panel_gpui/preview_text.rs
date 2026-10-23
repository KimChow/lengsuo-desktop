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
    render_body(path, None, theme)
}

/// 按下发内容渲染文档区；`content` 为 `None` 即未下发态（⛔ 不伪造正文）。
fn render_body(
    path: &str,
    content: Option<&str>,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let path_row = div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(path);

    let Some(content) = content else {
        return div()
            .flex()
            .flex_col()
            .gap_2()
            .child(path_row)
            .child(
                div()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child("内容未下发（等运行时契约）"),
            )
            .into_any_element();
    };

    let total = content.chars().count();
    let visible: String = content.chars().take(MAX_CHARS).collect();
    // 字体按类型选择：文档形态里 md/markdown 走正文字体，其余（含未识别扩展名）等宽。
    let mono = if is_document(path) {
        !is_prose(path)
    } else {
        true
    };
    let mut body = div().text_sm().text_color(theme.foreground);
    if mono {
        body = body.font_family(theme.mono_font_family.clone());
    }

    let mut column = div().flex().flex_col().gap_2().child(path_row);
    // 正文区：截断兜底后的内容纵向可滚（`overflow_y_scroll` 需带 id 的元素）。
    column = column.child(
        div()
            .id("preview-text-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(body.child(visible)),
    );
    if let Some(note) = truncation_note(total) {
        column = column.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(note),
        );
    }
    column.into_any_element()
}
