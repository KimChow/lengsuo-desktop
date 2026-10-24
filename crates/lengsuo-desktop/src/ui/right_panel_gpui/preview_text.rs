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

/// 是否等宽展示：仅 md/markdown 走正文字体，其余（含未识别扩展名）等宽。
fn use_mono(path: &str) -> bool {
    !(is_document(path) && is_prose(path))
}

/// 路径头（`child()` 文案参数要求 `'static`：借用 path 需先转 owned）。
fn path_row(path: &str, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(path.to_owned())
}

/// 未下发 / 失败态：路径头 + 一行照实说明（⛔ 不伪造正文）。
fn render_notice(path: &str, notice: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(path_row(path, theme))
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(notice.to_owned()),
        )
        .into_any_element()
}

/// 按下发内容渲染文档区；`content` 为 `None` 即未下发态（⛔ 不伪造正文）。
fn render_body(
    path: &str,
    content: Option<&str>,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let Some(content) = content else {
        return render_notice(path, "内容未下发（等运行时契约）", theme);
    };

    let total = content.chars().count();
    let visible: String = content.chars().take(MAX_CHARS).collect();
    let mut body = div().text_sm().text_color(theme.foreground);
    if use_mono(path) {
        body = body.font_family(theme.mono_font_family.clone());
    }

    let mut column = div().flex().flex_col().gap_2().child(path_row(path, theme));
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

/// A14：带载荷渲染。`Text` ⇒ 等宽行列表 + 截断提示行；未下发 / 失败 ⇒
/// 照实显示原因（⛔ 不伪造正文）；其他种类载荷与本预览器不符 ⇒ 占位外壳。
pub(super) fn render_with_payload(
    path: &str,
    payload: &PreviewPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    match payload {
        PreviewPayload::Text(text) => render_payload_lines(path, text, theme),
        PreviewPayload::NotDelivered { reason } => render_notice(path, reason, theme),
        PreviewPayload::Failed(err) => render_notice(path, &err.line(), theme),
        _ => render(path, theme),
    }
}

/// 行列表：沿用既有外壳（路径头 + 可滚正文 + 截断提示）。
fn render_payload_lines(
    path: &str,
    payload: &TextPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let mut body = div().text_sm().text_color(theme.foreground);
    if use_mono(path) {
        body = body.font_family(theme.mono_font_family.clone());
    }
    let mut column = div().flex().flex_col().gap_2().child(path_row(path, theme));
    // 正文区：行列表纵向可滚（`overflow_y_scroll` 需带 id 的元素）。
    column = column.child(
        div()
            .id("preview-text-lines")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(body.child(payload.lines.join("\n"))),
    );
    if payload.truncated {
        column = column.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child("内容超过预览上限，已截断显示。"),
        );
    }
    column.into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不写 `use super::*`：显式列符号，避免 GPUI 的 test 属性宏遮蔽标准库 #[test]。
    use super::{
        decode, extension, is_document, is_prose, is_truncated, render_body, render_with_payload,
        truncation_note, MAX_CHARS,
    };
    use crate::state::preview_payload::{DecodeError, PreviewPayload, MAX_BYTES, MAX_LINES};
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, AnyElement, AppContext as _, Bounds, Context,
        InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render, Styled as _,
        TestAppContext, Window,
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

    /// ① 正常字节 ⇒ `Text` 载荷字段正确（行切分 / CRLF / 非 UTF-8 lossy 照实解出）。
    #[test]
    fn decode_normal_bytes_to_lines() {
        let PreviewPayload::Text(p) = decode("a.txt", Some("l1\nl2\nl3".as_bytes())) else {
            panic!("正常文本必须解出 Text 载荷");
        };
        assert_eq!(p.lines, ["l1", "l2", "l3"]);
        assert!(!p.truncated);

        let PreviewPayload::Text(p) = decode("b.log", Some(b"crlf\r\nline2")) else {
            panic!("CRLF 也应解出 Text 载荷");
        };
        assert_eq!(p.lines, ["crlf", "line2"]);

        // 无 NUL 的非 UTF-8 字节：lossy 解出替换字符（照实显示，不判失败）。
        let PreviewPayload::Text(p) = decode("c.txt", Some(&[0x63, 0x61, 0x66, 0xE9])) else {
            panic!("lossy 路径也应解出 Text 载荷");
        };
        assert_eq!(p.lines, ["caf\u{FFFD}"]);
    }

    /// ② 垃圾字节（含 NUL）⇒ `Failed(Parse)`，fail-closed 不 panic、不伪装成文本。
    #[test]
    fn decode_binary_bytes_fail_closed() {
        match decode("x.bin", Some(&[0xFF, 0x00, 0xFE, 0x00])) {
            PreviewPayload::Failed(DecodeError::Parse(_)) => {}
            other => panic!("含 NUL 垃圾字节应 fail-closed，实际 {other:?}"),
        }
    }

    /// `bytes = None` 且读不到文件 ⇒ `Failed(Io)`（诚实失败，不伪装成空内容）。
    #[test]
    fn decode_missing_file_fails_io() {
        match decode("/__lengsuo_preview_no_such__/missing.txt", None) {
            PreviewPayload::Failed(DecodeError::Io(_)) => {}
            other => panic!("读不到文件应 Failed(Io)，实际 {other:?}"),
        }
    }

    /// ③ 超 `MAX_LINES` / `MAX_BYTES` ⇒ 截断 + `truncated == true`。
    #[test]
    fn decode_truncates_over_limits() {
        let many = "a\n".repeat(MAX_LINES + 1);
        let PreviewPayload::Text(p) = decode("big.log", Some(many.as_bytes())) else {
            panic!("超行数也应解出载荷");
        };
        assert_eq!(p.lines.len(), MAX_LINES);
        assert!(p.truncated);

        let huge = vec![b'a'; MAX_BYTES + 1];
        let PreviewPayload::Text(p) = decode("huge.txt", Some(&huge)) else {
            panic!("超字节上限也应解出载荷");
        };
        assert_eq!(p.lines.len(), 1);
        assert!(p.truncated);
    }

    /// 空字节 ⇒ 空 `Text` 载荷（零行、未截断），不是失败。
    #[test]
    fn decode_empty_bytes_is_empty_text() {
        let PreviewPayload::Text(p) = decode("empty.txt", Some(&[])) else {
            panic!("空内容应解出空 Text 载荷");
        };
        assert!(p.lines.is_empty());
        assert!(!p.truncated);
    }

    /// 渲染元素树非空：无窗口 layout + prepaint + paint 一帧，
    /// 探针必须进入渲染帧且占非零空间。
    ///
    /// 滚动监听要求在视图实体内绘制（裸元素 draw 会在 `current_view` 处 panic），
    /// 所以用测试宿主视图承载。
    const PROBE: &str = "preview-text-probe";

    struct PreviewHost {
        make: Box<dyn Fn(&Theme) -> AnyElement>,
    }

    impl Render for PreviewHost {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().flex().items_start().child(
                div()
                    .debug_selector(|| PROBE.to_owned())
                    .child((self.make)(&Theme::default())),
            )
        }
    }

    fn assert_renders_non_empty(make: impl Fn(&Theme) -> AnyElement + 'static) {
        let mut ctx = TestAppContext::single();
        ctx.update(|cx| cx.set_global(Theme::default()));
        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(800.0), px(600.0)),
            |_window, cx| {
                cx.new(|_| PreviewHost {
                    make: Box::new(make),
                })
                .into_any_element()
            },
        );
        let bounds: Bounds<Pixels> = visual
            .debug_bounds(PROBE)
            .expect("探针未进入渲染帧（debug_selector 未记录）");
        assert!(
            bounds.size.width > px(0.0) && bounds.size.height > px(0.0),
            "布局为零：元素没有真正参与排版"
        );
    }

    #[test]
    fn preview_text_renders_non_empty_tree() {
        assert_renders_non_empty(|theme| render_body("notes.md", None, theme));
        assert_renders_non_empty(|theme| {
            render_body("app.log", Some("first line\nsecond line"), theme)
        });
    }

    /// ④ `render_with_payload` 覆盖全部诚实态（Text / 截断 / 未下发 / 失败）：
    /// 不 panic 且布局占非零空间。
    #[test]
    fn render_with_payload_covers_all_honest_states() {
        let text = decode("app.log", Some("first\nsecond".as_bytes()));
        assert_renders_non_empty(move |theme| render_with_payload("app.log", &text, theme));

        let many = "a\n".repeat(MAX_LINES + 1);
        let truncated = decode("big.log", Some(many.as_bytes()));
        assert_renders_non_empty(move |theme| render_with_payload("big.log", &truncated, theme));

        let not_delivered = PreviewPayload::NotDelivered {
            reason: crate::state::preview_payload::DECODE_NOT_DELIVERED,
        };
        assert_renders_non_empty(move |theme| render_with_payload("a.txt", &not_delivered, theme));

        let failed = PreviewPayload::Failed(DecodeError::Io("missing".to_owned()));
        assert_renders_non_empty(move |theme| render_with_payload("a.txt", &failed, theme));
    }
}
