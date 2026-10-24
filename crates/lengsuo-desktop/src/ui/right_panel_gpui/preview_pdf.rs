//! PDF 预览（`PreviewKind::Pdf`）：解码（lopdf）+ 视图——页工具栏 / 缩放控件 / 载荷正文（A14）。
//!
//! 解码只做结构级抽取（页数 / `/Title` / 逐页文本），⛔ 不做像素光栅化（需 pdfium/mupdf），
//! 界面如实标注；拿不到内容时诚实显示 `NotDelivered` / `Failed`，⛔ 不伪造。
//! 颜色一律取自 `theme`，⛔ 不硬编码。
use crate::state::preview_payload::{
    DecodeError, PdfPayload, PreviewPayload, MAX_BYTES, MAX_LINES,
};
use gpui_kit::*;

/// PDF 预览面板的纯状态：渲染是它的纯函数（A14 视图层）。
///
/// 页内容与总页数由运行时契约下发；`total == 0` 表示未下发。
#[derive(Clone, Copy, Debug, PartialEq)]
struct PdfPanelState {
    /// 当前页（1 起）。
    page: usize,
    /// 总页数；`0` = 内容未下发。
    total: usize,
    /// 缩放倍率（0.25~4.0）。
    zoom: f32,
}

impl PdfPanelState {
    /// 未下发的初始态：停在第 1 页、总页数未知、100% 缩放。
    fn undelivered() -> Self {
        Self {
            page: 1,
            total: 0,
            zoom: 1.0,
        }
    }
}

/// 页码步进：`cur + delta` 夹紧到 `1..=total`（未下发时停在 1）。
fn page_step(cur: usize, total: usize, delta: i8) -> usize {
    let next = cur as i64 + i64::from(delta);
    let upper = total.max(1) as i64;
    next.clamp(1, upper) as usize
}

/// 页码标签：形如 `3 / 12`；未下发（`total == 0`）时为 `– / –`。
fn page_label(cur: usize, total: usize) -> String {
    if total == 0 {
        "– / –".to_string()
    } else {
        format!("{cur} / {total}")
    }
}

/// 缩放步进：`cur + delta` 夹紧到 0.25~4.0。
fn zoom_step(cur: f32, delta: f32) -> f32 {
    (cur + delta).clamp(0.25, 4.0)
}

/// 缩放标签：形如 `100%`。
fn zoom_label(zoom: f32) -> String {
    format!("{}%", (zoom * 100.0).round() as u32)
}

/// 渲染 PDF 预览面板：页工具栏 + 页面占位区（未下发态）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    render_panel(
        &PdfPanelState::undelivered(),
        theme,
        render_page_area(path, theme),
    )
}

/// 面板主体：工具栏在上，正文区（`body`）撑满余下空间。
fn render_panel(
    state: &PdfPanelState,
    theme: &gpui_kit::component::Theme,
    body: AnyElement,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .child(render_toolbar(state, theme))
        .child(body)
        .into_any_element()
}

/// 页工具栏：左「上一页 / 页码 / 下一页」，右「`-` / 百分比 / `+`」；触摸目标 ≥48dp。
fn render_toolbar(state: &PdfPanelState, theme: &gpui_kit::component::Theme) -> AnyElement {
    let next_page = page_step(state.page, state.total, 1);
    let prev_page = page_step(state.page, state.total, -1);
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(tool_button(
                    "<",
                    format!("上一页 {}→{}", state.page, prev_page),
                    theme,
                ))
                .child(metric_label(page_label(state.page, state.total), theme))
                .child(tool_button(
                    ">",
                    format!("下一页 {}→{}", state.page, next_page),
                    theme,
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(tool_button(
                    "-",
                    format!("缩小 {:.2}→{:.2}", state.zoom, zoom_step(state.zoom, -0.25)),
                    theme,
                ))
                .child(metric_label(zoom_label(state.zoom), theme))
                .child(tool_button(
                    "+",
                    format!("放大 {:.2}→{:.2}", state.zoom, zoom_step(state.zoom, 0.25)),
                    theme,
                )),
        )
        .into_any_element()
}

/// 48×48 的工具按钮；点击只上报意图（不判定、不改宿主状态）。
fn tool_button(
    label: &'static str,
    intent: String,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    div()
        .flex_none()
        .w_12()
        .h_12()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.muted_foreground)
        .cursor_pointer()
        .hover(|style| style.bg(theme.muted))
        .child(label)
        .id(label)
        .on_click(move |_event, _window, _app| {
            eprintln!("[lengsuo-desktop] PDF 预览点击（意图未接线）：{intent}");
        })
        .into_any_element()
}

/// 页码 / 缩放读数：等宽字体 + 48dp 最小宽，读数变化不挤动相邻按钮。
fn metric_label(text: String, theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .flex_none()
        .min_w_12()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .font_family(theme.mono_font_family.clone())
        .text_color(theme.foreground)
        .child(text)
        .into_any_element()
}

/// 页面占位区（未下发）：`theme.muted` 底 + `theme.border` 描边，诚实占位不伪造页内容。
fn render_page_area(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    render_page_area_note(path, theme, "PDF 未下发（等运行时契约）")
}

/// 占位区变体：`note` 为诚实说明（未下发原因 / 解码失败原因）。
fn render_page_area_note(path: &str, theme: &gpui_kit::component::Theme, note: &str) -> AnyElement {
    div()
        .flex_1()
        .m_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(theme.muted)
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap_2()
                .text_color(theme.muted_foreground)
                .child(div().text_sm().child(note.to_string()))
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.mono_font_family.clone())
                        .child(path.to_string()),
                ),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// A14 解码（真解码）：lopdf 结构级抽取 —— 页数 / `/Title` / 逐页文本（`Tj`/`TJ`）。
// ⛔ 不做像素光栅化（需 pdfium/mupdf 原生库），界面如实标注「像素渲染未接入」。
// ---------------------------------------------------------------------------

/// A14：解码待预览文件。`bytes = Some` ⇒ 只用传入字节（不碰文件系统，便于测试）；
/// `None` ⇒ 读 `path`（读失败 ⇒ `Failed(Io)`）。超 [`MAX_BYTES`] 只取前 8 MiB
/// （⛔ 不整包进内存），超 [`MAX_LINES`] 截断文本并置 `truncated`。
/// 拿不到 ⇒ `NotDelivered` / `Failed`，⛔ 不伪造内容、⛔ 不吞错。
pub(super) fn decode(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let owned;
    let (data, oversized) = if let Some(bytes) = bytes {
        (
            &bytes[..bytes.len().min(MAX_BYTES)],
            bytes.len() > MAX_BYTES,
        )
    } else {
        match read_capped(path) {
            Ok((buf, over)) => {
                owned = buf;
                (owned.as_slice(), over)
            }
            Err(msg) => return PreviewPayload::Failed(DecodeError::Io(msg)),
        }
    };
    match decode_pdf(data, oversized) {
        Ok(payload) => PreviewPayload::Pdf(payload),
        Err(err) => PreviewPayload::Failed(err),
    }
}

/// 读 `path` 的前 [`MAX_BYTES`] 字节；返回（字节, 是否超上限截断）。读失败 ⇒ 错误说明。
fn read_capped(path: &str) -> Result<(Vec<u8>, bool), String> {
    use std::io::Read;
    let actual = std::fs::metadata(path)
        .and_then(|meta| std::fs::File::open(path).map(|file| (meta, file)))
        .and_then(|(meta, file)| {
            let mut buf = Vec::with_capacity((meta.len() as usize).min(MAX_BYTES));
            file.take(MAX_BYTES as u64)
                .read_to_end(&mut buf)
                .map(|_| (buf, meta.len() as usize > MAX_BYTES))
        })
        .map_err(|err| err.to_string())?;
    Ok(actual)
}

/// 结构级抽取：页数（`/Pages` 树）、`/Title`、逐页 content stream 的 `Tj`/`TJ` 文本。
/// 能抽则抽、抽不到留空串（纯图页 / 加密流）；解析失败 ⇒ `Parse`（如实带原因）。
fn decode_pdf(data: &[u8], oversized: bool) -> Result<PdfPayload, DecodeError> {
    let doc = lopdf::Document::load_mem(data).map_err(|err| {
        DecodeError::Parse(if oversized {
            format!("字节超过上限 {MAX_BYTES} 已截断，截断后仍无法解析：{err}")
        } else {
            err.to_string()
        })
    })?;
    let pages = doc.get_pages();
    let mut budget = MAX_LINES;
    let mut truncated = oversized;
    let mut page_text = Vec::with_capacity(pages.len());
    for &page_id in pages.values() {
        if budget == 0 {
            // 行预算耗尽：余下页不再抽取（如实标截断，⛔ 不装作抽完了）。
            truncated = true;
            page_text.push(String::new());
            continue;
        }
        let mut shows = page_shows(&doc, page_id);
        if shows.len() > budget {
            shows.truncate(budget);
            truncated = true;
        }
        budget -= shows.len();
        page_text.push(shows.join("\n"));
    }
    Ok(PdfPayload {
        pages: pages.len() as u32,
        title: pdf_title(&doc),
        page_text,
        truncated,
    })
}

/// 读文档 `/Title`（`/Info` 兼容引用与直挂两种挂法）；无 / 空白 ⇒ `None`。
fn pdf_title(doc: &lopdf::Document) -> Option<String> {
    use lopdf::Object;
    let dict = match doc.trailer.get(b"Info").ok()? {
        Object::Dictionary(dict) => Some(dict),
        Object::Reference(id) => match doc.get_object(*id) {
            Ok(Object::Dictionary(dict)) => Some(dict),
            _ => None,
        },
        _ => None,
    }?;
    let title = match dict.get(b"Title").ok()? {
        Object::String(bytes, _) => decode_pdf_text(bytes),
        _ => return None,
    };
    let title = title.trim();
    (!title.is_empty()).then(|| title.to_string())
}

/// PDF 文本字符串解码：UTF-16BE（带 BOM）按 u16 对解码；否则按 UTF-8 容错。
fn decode_pdf_text(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_be_bytes(*pair))
            .collect::<Vec<_>>();
        String::from_utf16(&units).unwrap_or_default()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// 从一页的 content stream（可能多条）抽出文本「show」序列：lopdf 自带解析器解析
/// 操作序列，只认 `Tj` / `TJ` 的字符串操作数（能抽则抽、⛔ 不伪造；其余指令忽略）。
fn page_shows(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Vec<String> {
    use lopdf::Object;
    let mut shows = Vec::new();
    for content_id in doc.get_page_contents(page_id) {
        let Ok(Object::Stream(stream)) = doc.get_object(content_id) else {
            continue;
        };
        let Ok(ops) = stream.decode_content() else {
            continue;
        };
        for op in &ops.operations {
            match op.operator.as_str() {
                "Tj" => {
                    if let Some(Object::String(bytes, _)) = op.operands.first() {
                        shows.push(decode_pdf_text(bytes));
                    }
                }
                "TJ" => {
                    let text: Vec<String> = op
                        .operands
                        .iter()
                        .flat_map(|operand| match operand {
                            Object::String(bytes, _) => vec![decode_pdf_text(bytes)],
                            Object::Array(items) => items
                                .iter()
                                .filter_map(|item| match item {
                                    Object::String(bytes, _) => Some(decode_pdf_text(bytes)),
                                    _ => None,
                                })
                                .collect(),
                            _ => Vec::new(),
                        })
                        .collect();
                    let text = text.concat();
                    if !text.is_empty() {
                        shows.push(text);
                    }
                }
                _ => {}
            }
        }
    }
    shows
}

/// A14：带载荷渲染。`Pdf` ⇒ 工具栏页码接真页数 + 文本摘录正文；`NotDelivered` /
/// `Failed` ⇒ 诚实占位 / 失败说明行；其余载荷形态退回占位视图（外壳行为不变）。
pub(super) fn render_with_payload(
    path: &str,
    payload: &PreviewPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    let state = match payload {
        PreviewPayload::Pdf(p) => PdfPanelState {
            page: 1,
            total: p.pages as usize,
            zoom: 1.0,
        },
        _ => PdfPanelState::undelivered(),
    };
    let body = match payload {
        PreviewPayload::Pdf(p) => render_pdf_body(p, theme),
        PreviewPayload::NotDelivered { reason } => render_page_area_note(path, theme, reason),
        PreviewPayload::Failed(err) => render_page_area_note(path, theme, &err.line()),
        _ => return render(path, theme),
    };
    render_panel(&state, theme, body)
}

/// 摘录正文显示的页数 / 单页行数上限（载荷本身仍受 [`MAX_LINES`] 有界约束）。
const EXCERPT_PAGES: usize = 3;
const EXCERPT_LINES: usize = 60;

/// 载荷正文：`theme.muted` 底卡片内自上而下排元信息 + 前 [`EXCERPT_PAGES`] 页文本摘录。
/// ⛔ 像素渲染未接入（需 pdfium/mupdf），文案如实标注。
fn render_pdf_body(payload: &PdfPayload, theme: &gpui_kit::component::Theme) -> AnyElement {
    let mut meta = format!(
        "{} 页 · 像素渲染未接入（需 pdfium/mupdf），以下为文本抽取摘录",
        payload.pages
    );
    if payload.truncated {
        meta.push_str(&format!(" · 内容超上限，仅保留前 {MAX_LINES} 行"));
    }
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .text_color(theme.muted_foreground)
        .child(div().text_sm().child(meta));
    if let Some(title) = &payload.title {
        col = col.child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(format!("标题：{title}")),
        );
    }
    for (idx, text) in payload.page_text.iter().take(EXCERPT_PAGES).enumerate() {
        col = col.child(render_page_excerpt(idx + 1, text, theme));
    }
    if payload.page_text.len() > EXCERPT_PAGES {
        col = col.child(
            div()
                .text_xs()
                .child(format!("……仅显示前 {EXCERPT_PAGES} 页摘录")),
        );
    }
    div()
        .id("preview-pdf-body")
        .flex_1()
        .m_3()
        .p_3()
        .flex()
        .overflow_y_scroll()
        .rounded_md()
        .bg(theme.muted)
        .border_1()
        .border_color(theme.border)
        .child(col)
        .into_any_element()
}

/// 单页摘录：页号行 + 等宽正文；空页如实标注无可抽取文本，超行数如实标注摘录。
fn render_page_excerpt(no: usize, text: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    let total = text.lines().count();
    let shown: Vec<&str> = text.lines().take(EXCERPT_LINES).collect();
    let label = if total == 0 {
        format!("第 {no} 页（无可抽取文本）")
    } else if total > EXCERPT_LINES {
        format!("第 {no} 页（仅摘录前 {EXCERPT_LINES} 行）")
    } else {
        format!("第 {no} 页")
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .child(div().text_xs().child(label))
        .child(
            div()
                .w_full()
                .text_xs()
                .font_family(theme.mono_font_family.clone())
                .text_color(theme.foreground)
                .child(shown.join("\n")),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        decode, page_label, page_step, render_with_payload, zoom_label, zoom_step, PdfPanelState,
    };
    use crate::state::preview_payload::{DecodeError, PreviewPayload, MAX_BYTES, MAX_LINES};
    use gpui_kit::component::Theme;

    #[test]
    fn page_step_clamps_to_bounds() {
        assert_eq!(page_step(12, 12, 1), 12, "上界夹紧：不越过总页数");
        assert_eq!(page_step(1, 12, -1), 1, "下界夹紧：不小于第 1 页");
        assert_eq!(page_step(3, 12, 1), 4);
        assert_eq!(page_step(3, 12, -1), 2);
        assert_eq!(page_step(1, 0, 1), 1, "未下发（total=0）：停在 1");
    }

    #[test]
    fn page_label_formats() {
        assert_eq!(page_label(3, 12), "3 / 12");
        assert_eq!(page_label(1, 1), "1 / 1");
        assert_eq!(page_label(1, 0), "– / –", "未下发：不伪造页码");
    }

    #[test]
    fn zoom_step_clamps() {
        assert_eq!(zoom_step(4.0, 0.5), 4.0, "上界夹紧：不越过 400%");
        assert_eq!(zoom_step(0.25, -0.5), 0.25, "下界夹紧：不小于 25%");
        assert_eq!(zoom_step(1.0, 0.25), 1.25);
    }

    #[test]
    fn zoom_label_formats() {
        assert_eq!(zoom_label(1.0), "100%");
        assert_eq!(zoom_label(1.25), "125%");
        assert_eq!(zoom_label(0.25), "25%");
    }

    #[test]
    fn undelivered_state_is_honest() {
        let state = PdfPanelState::undelivered();
        assert_eq!(state.total, 0);
        assert_eq!(page_label(state.page, state.total), "– / –");
        assert_eq!(zoom_label(state.zoom), "100%");
    }

    /// 构造最小合法单页 PDF（xref 偏移实时记录，纯内存不落盘）：
    /// `content` 为该页 content stream 原文；`title` 写入 `/Info` 的 `/Title`
    /// （UTF-16BE + BOM，与真实生成器一致，走完整解码路径）。
    fn pdf_bytes(title: Option<&str>, content: &str) -> Vec<u8> {
        let mut objs: Vec<Vec<u8>> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>".to_vec(),
            format!(
                "<< /Length {} >>\nstream\n{content}\nendstream",
                content.len()
            )
            .into_bytes(),
        ];
        if let Some(title) = title {
            let hex = std::iter::once(0xFEFFu16)
                .chain(title.encode_utf16())
                .map(|unit| format!("{unit:04X}"))
                .collect::<String>();
            objs.push(format!("<< /Title <{hex}> >>").into_bytes());
        }
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::with_capacity(objs.len());
        for (idx, body) in objs.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", idx + 1).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref_at = out.len();
        let size = objs.len() + 1;
        out.extend_from_slice(format!("xref\n0 {size}\n").as_bytes());
        out.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets {
            out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        let info_ref = if title.is_some() {
            format!("/Info {} 0 R ", objs.len())
        } else {
            String::new()
        };
        out.extend_from_slice(
            format!(
                "trailer\n<< /Size {size} /Root 1 0 R {info_ref}>>\nstartxref\n{xref_at}\n%%EOF"
            )
            .as_bytes(),
        );
        out
    }

    #[test]
    fn decode_reads_payload_from_bytes_only() {
        let bytes = pdf_bytes(Some("样本标题"), "(Hello) Tj [(W)-120(o)4(rld)] TJ");
        match decode("ignored.pdf", Some(&bytes)) {
            PreviewPayload::Pdf(p) => {
                assert_eq!(p.pages, 1, "页数从 /Pages 树解出");
                assert_eq!(
                    p.title.as_deref(),
                    Some("样本标题"),
                    "/Title 经 UTF-16BE BOM 解码"
                );
                assert_eq!(
                    p.page_text,
                    vec!["Hello\nWorld".to_string()],
                    "Tj 与 TJ 的字符串操作数都要抽到"
                );
                assert!(!p.truncated);
            }
            other => panic!("应为 Pdf 载荷，得到 {other:?}"),
        }
    }

    #[test]
    fn decode_fail_closed_on_garbage() {
        let junks: [&[u8]; 3] = [b"", b"not a pdf at all", b"%PDF-1.4 broken"];
        for junk in junks {
            assert!(
                matches!(
                    decode("g.pdf", Some(junk)),
                    PreviewPayload::Failed(_) | PreviewPayload::NotDelivered { .. }
                ),
                "垃圾输入必须 fail-closed（不 panic、不伪造）：{junk:?}"
            );
        }
    }

    #[test]
    fn decode_truncates_over_line_budget() {
        let content = "(行) Tj ".repeat(MAX_LINES + 25);
        let bytes = pdf_bytes(None, &content);
        match decode("t.pdf", Some(&bytes)) {
            PreviewPayload::Pdf(p) => {
                assert!(p.truncated, "超过 MAX_LINES 必须如实标注截断");
                assert_eq!(
                    p.page_text[0].lines().count(),
                    MAX_LINES,
                    "行数夹紧到 MAX_LINES"
                );
                assert_eq!(p.pages, 1);
            }
            other => panic!("应为 Pdf 载荷，得到 {other:?}"),
        }
    }

    #[test]
    fn decode_fail_closed_over_byte_cap() {
        let oversized = vec![0u8; MAX_BYTES + 1];
        assert!(
            matches!(decode("h.pdf", Some(&oversized)), PreviewPayload::Failed(_)),
            "超 MAX_BYTES 截断后无法解析 ⇒ Failed（不整包进内存、不伪造、不 panic）"
        );
    }

    #[test]
    fn render_with_payload_never_panics_for_all_branches() {
        let theme = Theme::default();
        let payload = decode("a.pdf", Some(&pdf_bytes(None, "(A) Tj")));
        let _ = render_with_payload("a.pdf", &payload, &theme);
        let _ = render_with_payload(
            "a.pdf",
            &PreviewPayload::NotDelivered {
                reason: "内容解码尚未接入（A14）",
            },
            &theme,
        );
        let _ = render_with_payload(
            "a.pdf",
            &PreviewPayload::Failed(DecodeError::Io("file gone".into())),
            &theme,
        );
        let _ = render_with_payload(
            "a.pdf",
            &PreviewPayload::Failed(DecodeError::TooLarge {
                limit: MAX_BYTES,
                actual: MAX_BYTES + 1,
            }),
            &theme,
        );
    }

    #[test]
    fn decode_handles_escapes_hex_and_octal() {
        let bytes = pdf_bytes(None, "(\\(q\\) 100\\% \\101) Tj <48 69 21> Tj /F1 12 Tf");
        match decode("e.pdf", Some(&bytes)) {
            PreviewPayload::Pdf(p) => {
                assert_eq!(
                    p.page_text,
                    vec!["(q) 100% A\nHi!".to_string()],
                    "括号/百分号/八进制转义与十六进制串都要正确解码；非 show 指令不产出"
                );
            }
            other => panic!("应为 Pdf 载荷，得到 {other:?}"),
        }
    }
}
