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
fn render_page_area_note(
    path: &str,
    theme: &gpui_kit::component::Theme,
    note: &str,
) -> AnyElement {
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
        (&bytes[..bytes.len().min(MAX_BYTES)], bytes.len() > MAX_BYTES)
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
        let mut shows = doc
            .get_page_content(page_id)
            .map(|content| extract_shows(&content))
            .unwrap_or_default();
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
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16(&units).unwrap_or_default()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// 从一页 content stream 抽出文本「show」序列：只认 `Tj` / `TJ`，字符串操作数按
/// 出现顺序拼接（尽力抽取，⛔ 不伪造 —— 非文本指令 / 跳变一律忽略）。
fn extract_shows(content: &[u8]) -> Vec<String> {
    let b = content;
    let mut shows = Vec::new();
    let mut pending: Vec<String> = Vec::new(); // 上一个操作符之前累计的字符串操作数
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'(' => {
                let (text, next) = parse_literal_string(b, i);
                pending.push(text);
                i = next;
            }
            b'<' => {
                if b.get(i + 1) == Some(&b'<') {
                    i = skip_inline_dict(b, i); // 如 BDC 的属性字典：整段跳过
                } else {
                    let (text, next) = parse_hex_string(b, i);
                    pending.push(text);
                    i = next;
                }
            }
            b'%' => {
                while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                    i += 1;
                }
            }
            b'/' => {
                while i < b.len() && !is_delim(b[i]) && !b[i].is_ascii_whitespace() {
                    i += 1;
                }
            }
            c if c.is_ascii_whitespace() || matches!(c, b'[' | b']' | b'{' | b'}' | b')' | b'>') => {
                i += 1;
            }
            _ => {
                let start = i;
                while i < b.len() && !is_delim(b[i]) && !b[i].is_ascii_whitespace() {
                    i += 1;
                }
                match &b[start..i] {
                    b"Tj" => {
                        if let Some(text) = pending.pop() {
                            shows.push(text);
                        }
                    }
                    b"TJ" => {
                        if !pending.is_empty() {
                            shows.push(pending.concat());
                        }
                    }
                    _ => {}
                }
                pending.clear();
            }
        }
    }
    shows
}

/// PDF 分隔符（含注释符 `%` 与名字前导 `/`）。
fn is_delim(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// 解析 `(` 字面字符串：嵌套括号 + 反斜杠转义（`\n` 等 / 1~3 位八进制）；返回（文本, 结束下标）。
fn parse_literal_string(b: &[u8], mut i: usize) -> (String, usize) {
    i += 1;
    let mut depth = 1usize;
    let mut out: Vec<u8> = Vec::new();
    while i < b.len() && depth > 0 {
        match b[i] {
            b'\\' => {
                i += 1;
                if i >= b.len() {
                    break;
                }
                match b[i] {
                    b'n' => out.push(b'\n'),
                    b'r' => out.push(b'\r'),
                    b't' => out.push(b'\t'),
                    b'b' => out.push(0x08),
                    b'f' => out.push(0x0C),
                    b'0'..=b'7' => {
                        let mut v = 0u32;
                        let mut n = 0;
                        while i < b.len() && n < 3 && (b'0'..=b'7').contains(&b[i]) {
                            v = v * 8 + u32::from(b[i] - b'0');
                            i += 1;
                            n += 1;
                        }
                        out.push((v & 0xFF) as u8);
                        continue;
                    }
                    other => out.push(other), // \( \) \\ 及其他字面
                }
                i += 1;
            }
            b'(' => {
                depth += 1;
                out.push(b'(');
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth > 0 {
                    out.push(b')');
                }
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    (decode_pdf_text(&out), i)
}

/// 解析 `<` 十六进制字符串（忽略空白，奇数位补 0）；返回（文本, 结束下标）。
fn parse_hex_string(b: &[u8], mut i: usize) -> (String, usize) {
    i += 1;
    let mut nibbles: Vec<u8> = Vec::new();
    while i < b.len() && b[i] != b'>' {
        if let Some(v) = (b[i] as char).to_digit(16) {
            nibbles.push(v as u8);
        }
        i += 1;
    }
    if i < b.len() {
        i += 1; // 收掉 '>'
    }
    if nibbles.len() % 2 == 1 {
        nibbles.push(0);
    }
    let bytes = nibbles
        .chunks(2)
        .map(|pair| (pair[0] << 4) | pair[1])
        .collect::<Vec<_>>();
    (decode_pdf_text(&bytes), i)
}

/// 跳过 `<< … >>` 内联字典（按 `<<`/`>>` 配对计数）。
fn skip_inline_dict(b: &[u8], mut i: usize) -> usize {
    let mut depth = 0usize;
    while i < b.len() {
        if b[i] == b'<' && b.get(i + 1) == Some(&b'<') {
            depth += 1;
            i += 2;
        } else if b[i] == b'>' && b.get(i + 1) == Some(&b'>') {
            depth -= 1;
            i += 2;
            if depth == 0 {
                break;
            }
        } else {
            i += 1;
        }
    }
    i
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
        col = col.child(div().text_xs().child(format!("……仅显示前 {EXCERPT_PAGES} 页摘录")));
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
    use super::{page_label, page_step, zoom_label, zoom_step, PdfPanelState};

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
}
