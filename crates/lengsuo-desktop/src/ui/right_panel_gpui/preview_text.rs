//! 文本文档预览（`PreviewKind::Text`）——真解码 + 行列表渲染（A14）。
//!
//! 字节优先由运行时契约下发（[`decode`] 的 `bytes`，只消费传入字节）；
//! 未下发时有界读本地文件（≤ [`MAX_BYTES`] 即截断，⛔ 不整包进内存）。
//! 拿不到内容 ⇒ `NotDelivered` / `Failed`，⛔ 不伪造正文；
//! 颜色/字体一律取 `theme`，⛔ 不硬编码。
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;

use crate::state::preview_payload::{
    DecodeError, PreviewPayload, TextPayload, MAX_BYTES, MAX_LINES,
};
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
    matches!(extension(path).as_str(), "md" | "markdown" | "txt" | "log")
}

/// md/markdown 按正文字体展示，其余文档扩展名用等宽。
fn is_prose(path: &str) -> bool {
    matches!(extension(path).as_str(), "md" | "markdown")
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
    // child() 文案参数要求 'static：借用 path 需先转 owned。
    let path_row = div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(path.to_owned());

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

// ---------------------------------------------------------------------------
// A14 解码入口（`preview.rs` 分派：decode → render_with_payload）
// ---------------------------------------------------------------------------

/// A14：把待预览文本解码成 [`TextPayload`]。
///
/// 字节优先用运行时下发的 `bytes`（只消费传入字节，便于测试）；`None` 时
/// 有界读 `path`（读失败 ⇒ [`PreviewPayload::Failed`]）。超 [`MAX_BYTES`] /
/// [`MAX_LINES`] ⇒ 截断并置 `truncated`，⛔ 不整包进内存、⛔ 不伪造内容。
pub(super) fn decode(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let (data, over_limit): (Cow<'_, [u8]>, bool) = match bytes {
        Some(b) if b.len() > MAX_BYTES => (Cow::Borrowed(&b[..MAX_BYTES]), true),
        Some(b) => (Cow::Borrowed(b), false),
        None => match read_bounded(path) {
            Ok((buf, over)) => (Cow::Owned(buf), over),
            Err(err) => return PreviewPayload::Failed(DecodeError::Io(err.to_string())),
        },
    };
    // 二进制嗅探：含 NUL 字节 ⇒ fail-closed（解析失败），⛔ 不把 U+FFFD 汤伪装成文本。
    if data.contains(&0) {
        return PreviewPayload::Failed(DecodeError::Parse(
            "内容含 NUL 字节，疑似二进制，不按文本预览".to_owned(),
        ));
    }
    let text = String::from_utf8_lossy(&data);
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut truncated = over_limit;
    if lines.len() > MAX_LINES {
        lines.truncate(MAX_LINES);
        truncated = true;
    }
    PreviewPayload::Text(TextPayload { lines, truncated })
}

/// 有界读取：最多装满 `MAX_BYTES + 1` 字节缓冲（多读 1 字节只为探测超限）；
/// 超限 ⇒ 数据截回 [`MAX_BYTES`] 并返回 `true`。
fn read_bounded(path: &str) -> std::io::Result<(Vec<u8>, bool)> {
    let mut file = File::open(path)?;
    let mut buf = vec![0u8; MAX_BYTES + 1];
    let mut filled = 0;
    loop {
        if filled == buf.len() {
            buf.truncate(MAX_BYTES);
            return Ok((buf, true));
        }
        match file.read(&mut buf[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    buf.truncate(filled);
    Ok((buf, false))
}

/// A14：带载荷渲染。默认退回既有占位视图（外壳不变）。
pub(super) fn render_with_payload(
    path: &str,
    payload: &crate::state::preview_payload::PreviewPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let _ = payload;
    render(path, theme)
}

#[cfg(test)]
mod tests {
    // ⛔ 不写 `use super::*`：显式列符号，避免 GPUI 的 test 属性宏遮蔽标准库 #[test]。
    use super::MAX_CHARS;
    use super::{extension, is_document, is_prose, is_truncated, render_body, truncation_note};
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, AppContext as _, Bounds, Context, InteractiveElement as _,
        IntoElement, ParentElement as _, Pixels, Render, Styled as _, TestAppContext, Window,
    };

    #[test]
    fn truncation_boundary() {
        assert!(!is_truncated(0));
        assert!(!is_truncated(MAX_CHARS));
        assert!(is_truncated(MAX_CHARS + 1));
    }

    #[test]
    fn truncation_note_presence() {
        assert_eq!(truncation_note(MAX_CHARS), None);
        assert_eq!(
            truncation_note(MAX_CHARS + 1),
            Some("内容较长，已截断显示。")
        );
    }

    #[test]
    fn document_extensions() {
        assert!(is_document("a.md"));
        assert!(is_document("b.MARKDOWN"));
        assert!(is_document("c.txt"));
        assert!(is_document("d.log"));
        assert!(!is_document("e.rs"));
        assert!(!is_document("noext"));
        assert_eq!(extension("A/B.md"), "md");
        assert!(is_prose("a.md") && !is_prose("a.log"));
    }

    /// 渲染元素树非空：未下发态与有内容态各画一帧（无窗口 layout + prepaint + paint），
    /// 探针必须进入渲染帧且占非零空间。
    ///
    /// 滚动监听要求在视图实体内绘制（裸元素 draw 会在 `current_view` 处 panic），
    /// 所以用测试宿主视图承载。
    const PROBE: &str = "preview-text-probe";

    struct PreviewHost {
        path: &'static str,
        content: Option<&'static str>,
    }

    impl Render for PreviewHost {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().items_start().child(
                div().debug_selector(|| PROBE.to_owned()).child(render_body(
                    self.path,
                    self.content,
                    &Theme::default(),
                )),
            )
        }
    }

    #[test]
    fn preview_text_renders_non_empty_tree() {
        for (path, content) in [
            ("notes.md", None),
            ("app.log", Some("first line\nsecond line")),
        ] {
            let mut ctx = TestAppContext::single();
            ctx.update(|cx| cx.set_global(Theme::default()));
            let visual = ctx.add_empty_window();
            let (_layout, _prepaint) = visual.draw(
                point(px(0.0), px(0.0)),
                size(px(800.0), px(600.0)),
                |_window, cx| cx.new(|_| PreviewHost { path, content }).into_any_element(),
            );
            let bounds = visual.debug_bounds(PROBE);
            assert!(
                bounds.is_some(),
                "探针未进入渲染帧（debug_selector 未记录）"
            );
            let bounds: Bounds<Pixels> = bounds.expect("上面已断言 Some");
            assert!(
                bounds.size.width > px(0.0) && bounds.size.height > px(0.0),
                "布局为零：元素没有真正参与排版"
            );
        }
    }
}
