//! 办公文档预览（`PreviewKind::Office`）：A14 真解码 + 载荷渲染。
//! 纯函数层（扩展名分派 / zip 容器 + quick-xml 事件流解码，无 GPUI 依赖）→
//! 接线层（外壳 + 载荷渲染）。解码 ⛔ 绝不伪造内容：拿不到 ⇒ `NotDelivered` / `Failed`。
//! 颜色一律取自 `theme`，⛔ 不硬编码。
use crate::state::preview_payload::OfficeBlockKind::{Bullet, Cell, Heading, Paragraph, Slide};
use crate::state::preview_payload::{
    self as payload, DecodeError, OfficeBlock, OfficeBlockKind, OfficePayload, PreviewPayload,
    MAX_BYTES, MAX_LINES,
};
use gpui_kit::component::Theme;
use gpui_kit::*;
use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::Reader;
use std::io::{BufReader, Read, Seek};

type Blocks = Vec<OfficeBlock>;

/// 办公文档预览的三个子类型（由扩展名分派）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum OfficeKind {
    Word,  // doc/docx；未知扩展名兜底
    Sheet, // xls/xlsx/csv/tsv
    Slide, // ppt/pptx
}

/// 小写扩展名（无扩展名 ⇒ 空串）。
fn ext_of(path: &str) -> String {
    let p = std::path::Path::new(path);
    let e = p.extension().and_then(|e| e.to_str());
    e.map(str::to_ascii_lowercase).unwrap_or_default()
}

/// 按扩展名分派子类型：`doc/docx`→Word、`xls/xlsx/csv/tsv`→Sheet、`ppt/pptx`→Slide；其余兜底 Word。
fn office_kind(path: &str) -> OfficeKind {
    match ext_of(path).as_str() {
        "xls" | "xlsx" | "csv" | "tsv" => OfficeKind::Sheet,
        "ppt" | "pptx" => OfficeKind::Slide,
        _ => OfficeKind::Word,
    }
}

/// 子类型在界面上显示的标签。
fn kind_label(kind: OfficeKind) -> &'static str {
    match kind {
        OfficeKind::Word => "文档",
        OfficeKind::Sheet => "表格",
        OfficeKind::Slide => "演示",
    }
}

/// 各子类型的工具条项：Word/Sheet 只有缩放；Slide 是 上一页/页码/下一页。
fn toolbar_items(kind: OfficeKind) -> Vec<&'static str> {
    match kind {
        OfficeKind::Word | OfficeKind::Sheet => vec!["缩放"],
        OfficeKind::Slide => vec!["上一页", "页码", "下一页"],
    }
}

/// 未下发态的诚实提示：内容需运行时契约下发，本视图不伪造任何文档内容。
const NOT_DELIVERED: &str = "文档未下发（等运行时契约）";

/// 头部：子类型标签 + 文件路径（等宽字体）。
fn header_row(path: &str, kind: OfficeKind, theme: &Theme) -> Div {
    let badge = div().px_2().py_1().rounded_sm().bg(theme.muted);
    let badge = badge.text_xs().text_color(theme.foreground);
    let badge = badge.child(kind_label(kind));
    let file = div().flex_1().truncate().text_xs();
    let file = file.text_color(theme.muted_foreground);
    let file = file.font_family(theme.mono_font_family.clone());
    let file = file.child(path.to_owned());
    let row = div().flex().items_center().gap_2().px_3().py_2();
    let row = row.border_b_1().border_color(theme.border);
    row.child(badge).child(file)
}

/// 工具条：按子类型显示对应操作项；交互待运行时契约。
fn toolbar_row(kind: OfficeKind, theme: &Theme) -> Div {
    let row = div().flex().items_center().gap_2().px_3().py_2();
    let mut row = row.border_b_1().border_color(theme.border);
    for item in toolbar_items(kind) {
        let chip = div().px_2().py_1().rounded_sm().border_1();
        let chip = chip.border_color(theme.border);
        let chip = chip.text_xs().text_color(theme.foreground);
        row = row.child(chip.child(item));
    }
    row
}

/// 预览外壳：头 + 工具条 + 内容区。
fn shell_with(path: &str, kind: OfficeKind, content: Div, theme: &Theme) -> Div {
    let shell = div().flex().flex_col().flex_1().w_full();
    shell
        .child(header_row(path, kind, theme))
        .child(toolbar_row(kind, theme))
        .child(content)
}

// A14 解码：zip 容器 + quick-xml 事件流（纯逻辑，无 GPUI 依赖，可单测）

/// 解码预算：累计**抽取**的字节 ≤ [`MAX_BYTES`]（容器本体不整包进内存），超了截断并标注。
struct Budget {
    left: usize,
    truncated: bool,
}

/// 解码器状态：zip 容器 + 预算。
struct Zip<R: Read + Seek> {
    arch: zip::ZipArchive<R>,
    budget: Budget,
}

/// 元素/属性的本地名（去掉 `w:` 等命名空间前缀）；非 UTF-8 按空处理。
fn local_tag(name: &[u8]) -> &str {
    let s = std::str::from_utf8(name).unwrap_or("");
    s.rsplit(':').next().unwrap_or(s)
}

/// 取 start 事件的指定本地名属性值（如 `w:val` 按 `val` 匹配）。
fn attr_value(e: &BytesStart<'_>, want: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| local_tag(a.key.as_ref()) == want)
        .map(|a| String::from_utf8_lossy(a.value.as_ref()).into_owned())
}

/// 文本事件内容：正常反转义；非法转义时按原始字节无损降级（不静默丢字）。
fn text_content(t: &BytesText<'_>) -> String {
    match t.xml_content(quick_xml::XmlVersion::Implicit1_0) {
        Ok(s) => s.into_owned(),
        Err(_) => String::from_utf8_lossy(t.as_ref()).into_owned(),
    }
}

/// 推进一个事件：EOF 或自截断的字节流 ⇒ `Ok(None)`（正常收尾）；结构错误 ⇒ `Err`。
fn next_event<'a>(
    reader: &mut Reader<&'a [u8]>,
    budget: &Budget,
    ctx: &str,
) -> Result<Option<Event<'a>>, String> {
    match reader.read_event() {
        Ok(Event::Eof) => Ok(None),
        Err(_) if budget.truncated => Ok(None),
        Err(e) => Err(format!("{ctx}：{e}")),
        Ok(ev) => Ok(Some(ev)),
    }
}

/// 打开 zip 容器；失败 ⇒ 一行说明。
fn open_zip<R: Read + Seek>(reader: R) -> Result<zip::ZipArchive<R>, String> {
    zip::ZipArchive::new(reader).map_err(|e| format!("不是合法的 zip 容器：{e}"))
}

/// 压入块：超过 [`MAX_LINES`] 停止累计并如实标注截断。
fn push_block(
    blocks: &mut Vec<OfficeBlock>,
    budget: &mut Budget,
    kind: OfficeBlockKind,
    text: String,
) {
    if blocks.len() >= MAX_LINES {
        budget.truncated = true;
    } else {
        blocks.push(OfficeBlock { kind, text });
    }
}

impl<R: Read + Seek> Zip<R> {
    fn new(arch: zip::ZipArchive<R>, source_len: usize) -> Self {
        Self {
            arch,
            budget: Budget {
                left: MAX_BYTES,
                truncated: source_len > MAX_BYTES,
            },
        }
    }

    /// 读单个条目（超预算就地截断并标注）；缺条目 ⇒ `Ok(None)`。
    fn read_entry(&mut self, name: &str) -> Result<Option<Vec<u8>>, String> {
        let mut entry = match self.arch.by_name(name) {
            Ok(entry) => entry,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(format!("zip 条目 {name}：{e}")),
        };
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = entry.read(&mut buf);
            let n = n.map_err(|e| format!("zip 条目 {name}：{e}"))?;
            if n == 0 {
                return Ok(Some(out));
            }
            let take = n.min(self.budget.left);
            out.extend_from_slice(&buf[..take]);
            self.budget.left -= take;
            if take < n {
                self.budget.truncated = true;
                return Ok(Some(out));
            }
        }
    }
}

/// docx 段落累积器：`<w:p>` 一段，`w:val` 含 heading ⇒ 标题，`<w:numPr>` ⇒ 列表项。
#[derive(Default)]
struct DocxPara {
    text: String,
    heading: bool,
    bullet: bool,
}

/// 段内属性（Start/Empty 两种事件等价）：pStyle 的 `w:val` 含 heading ⇒ 标题；numPr ⇒ 列表项。
fn note_para_props(e: &BytesStart<'_>, para: &mut Option<DocxPara>) {
    let Some(p) = para.as_mut() else { return };
    match local_tag(e.name().as_ref()) {
        "pStyle" => {
            if attr_value(e, "val").is_some_and(|v| v.to_ascii_lowercase().contains("heading")) {
                p.heading = true;
            }
        }
        "numPr" => p.bullet = true,
        _ => {}
    }
}

/// 段落 → 块种类：标题 > 列表项 > 段落。
fn docx_kind(p: &DocxPara) -> OfficeBlockKind {
    match (p.heading, p.bullet) {
        (true, _) => Heading,
        (_, true) => Bullet,
        _ => Paragraph,
    }
}

/// docx：`word/document.xml` → 块序列（`w:p` 一段一块）。
fn parse_docx_xml(xml: &str, budget: &mut Budget) -> Result<Blocks, String> {
    let mut reader = Reader::from_str(xml);
    let mut blocks = Vec::new();
    let mut para: Option<DocxPara> = None;
    let mut in_text = false;
    while let Some(ev) = next_event(&mut reader, budget, "document.xml")? {
        match ev {
            Event::Start(e) => match local_tag(e.name().as_ref()) {
                "p" => para = Some(DocxPara::default()),
                "pStyle" | "numPr" => note_para_props(&e, &mut para),
                "t" => in_text = true,
                _ => {}
            },
            Event::Empty(e) => match local_tag(e.name().as_ref()) {
                "pStyle" | "numPr" => note_para_props(&e, &mut para),
                _ => {}
            },
            Event::Text(t) => {
                if let (true, Some(p)) = (in_text, para.as_mut()) {
                    p.text.push_str(&text_content(&t));
                }
            }
            Event::End(e) => match local_tag(e.name().as_ref()) {
                "t" => in_text = false,
                "p" => {
                    if let Some(p) = para.take().filter(|p| !p.text.trim().is_empty()) {
                        push_block(&mut blocks, budget, docx_kind(&p), p.text);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(blocks)
}

/// 按 `group` 标签分组收集 `item` 标签文本（`group = None` ⇒ 整篇一组），组内以 `sep` 连接。
fn collect_texts(
    xml: &str,
    group: Option<&str>,
    item: &str,
    sep: &str,
    budget: &Budget,
    ctx: &str,
) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_str(xml);
    let mut out = Vec::new();
    let mut parts: Vec<String> = Vec::new();
    let mut in_item = false;
    while let Some(ev) = next_event(&mut reader, budget, ctx)? {
        match ev {
            Event::Start(e) => match local_tag(e.name().as_ref()) {
                tag if Some(tag) == group => parts.clear(),
                tag if tag == item => in_item = true,
                _ => {}
            },
            Event::End(e) => match local_tag(e.name().as_ref()) {
                tag if Some(tag) == group => out.push(parts.join(sep)),
                tag if tag == item => in_item = false,
                _ => {}
            },
            Event::Text(t) if in_item => parts.push(text_content(&t)),
            _ => {}
        }
    }
    if group.is_none() && !parts.is_empty() {
        out.push(parts.join(sep));
    }
    Ok(out)
}

/// 幻灯片条目名 → 页号（`ppt/slides/slide12.xml` ⇒ 12）。
fn slide_no(name: &str) -> Option<u32> {
    name.strip_prefix("ppt/slides/slide")?
        .strip_suffix(".xml")?
        .parse()
        .ok()
}

/// pptx：`ppt/slides/slideN.xml` 按页号数值排序，每页一块 Slide（页文本 = `<…:t>` 空格连写）。
fn parse_pptx_container<R: Read + Seek>(zip: &mut Zip<R>) -> Result<Blocks, String> {
    let mut slides: Vec<(u32, String)> = Vec::new();
    for n in zip.arch.file_names().flatten() {
        if let Some(num) = slide_no(&n) {
            slides.push((num, n.into_owned()));
        }
    }
    slides.sort_by_key(|(num, _)| *num);
    let mut blocks = Vec::new();
    for (_, name) in slides {
        if zip.budget.left == 0 {
            break; // 预算耗尽：余下页不读，截断标志已置。
        }
        if let Some(xml) = zip.read_entry(&name)? {
            let s = String::from_utf8_lossy(&xml);
            let mut texts = collect_texts(&s, None, "t", " ", &zip.budget, "slide.xml")?;
            let text = texts.pop().unwrap_or_default();
            push_block(&mut blocks, &mut zip.budget, Slide, text);
        }
    }
    if blocks.is_empty() {
        return Err("pptx 没有幻灯片（ppt/slides/slideN.xml）".into());
    }
    Ok(blocks)
}

/// xlsx 单元格累积器：`t="s"` 查共享表、`t="inlineStr"` 取 `<is><t>`、其余取 `<v>` 原文。
struct SheetCell {
    ty: Option<String>,
    val: String,
}

/// 单元格取值：共享索引查表（越界 ⇒ None）；非共享取已累积文本（空白 ⇒ None）。
fn cell_text(c: Option<SheetCell>, shared: &[String]) -> Option<String> {
    let c = c?;
    if c.ty.as_deref() == Some("s") {
        let i = c.val.trim().parse::<usize>().ok()?;
        return shared.get(i).cloned();
    }
    let val = c.val.trim();
    (!val.is_empty()).then(|| val.to_owned())
}

/// xlsx：共享表（`<si>` 一条，`<…:t>` 连写）+ `xl/worksheets/sheet1.xml` → 非空单元格一块 Cell。
fn parse_xlsx_container<R: Read + Seek>(zip: &mut Zip<R>) -> Result<Blocks, String> {
    let shared = match zip.read_entry("xl/sharedStrings.xml")? {
        Some(xml) => {
            let s = String::from_utf8_lossy(&xml);
            collect_texts(&s, Some("si"), "t", "", &zip.budget, "sharedStrings.xml")?
        }
        None => Vec::new(), // sharedStrings 可缺省（纯数字表）
    };
    let Some(xml) = zip.read_entry("xl/worksheets/sheet1.xml")? else {
        return Err("xlsx 缺 xl/worksheets/sheet1.xml".into());
    };
    parse_sheet_xml(&String::from_utf8_lossy(&xml), &shared, &mut zip.budget)
}

/// 工作表事件流 → 单元格块（`<c>` 一个、`<v>`/`<is><t>` 取值）。
fn parse_sheet_xml(xml: &str, shared: &[String], budget: &mut Budget) -> Result<Blocks, String> {
    let mut reader = Reader::from_str(xml);
    let mut blocks = Vec::new();
    let mut cell: Option<SheetCell> = None;
    let mut in_val = false;
    while let Some(ev) = next_event(&mut reader, budget, "worksheet")? {
        match ev {
            Event::Start(e) => match local_tag(e.name().as_ref()) {
                "c" => {
                    cell = Some(SheetCell {
                        ty: attr_value(&e, "t"),
                        val: String::new(),
                    })
                }
                "v" | "t" => in_val = true,
                _ => {}
            },
            Event::Text(t) if in_val => {
                if let Some(c) = cell.as_mut() {
                    c.val.push_str(&text_content(&t));
                }
            }
            Event::End(e) => match local_tag(e.name().as_ref()) {
                "v" | "t" => in_val = false,
                "c" => {
                    if let Some(text) = cell_text(cell.take(), shared) {
                        push_block(&mut blocks, budget, Cell, text);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(blocks)
}

/// 容器种类分派到对应解析器。
fn decode_with<R: Read + Seek>(mut zip: Zip<R>, fmt: payload::OfficeKind) -> PreviewPayload {
    let parsed = match fmt {
        payload::OfficeKind::Docx => match zip.read_entry("word/document.xml") {
            Ok(Some(xml)) => parse_docx_xml(&String::from_utf8_lossy(&xml), &mut zip.budget),
            Ok(None) => Err("docx 缺 word/document.xml".into()),
            Err(e) => Err(e),
        },
        payload::OfficeKind::Pptx => parse_pptx_container(&mut zip),
        payload::OfficeKind::Xlsx => parse_xlsx_container(&mut zip),
        payload::OfficeKind::Unknown => Err("未知办公文档格式".into()),
    };
    match parsed {
        Ok(blocks) => PreviewPayload::Office(OfficePayload {
            kind: fmt,
            blocks,
            truncated: zip.budget.truncated,
        }),
        Err(msg) => PreviewPayload::Failed(DecodeError::Parse(msg)),
    }
}

/// A14：解码待预览文件。`bytes = Some(..)` 时**只用传入字节**（不碰文件系统）；
/// `None` 时读 `path`（读不到 ⇒ `Failed(Io)`）。非 OOXML / 解不开 ⇒ `Failed`，⛔ 绝不伪造内容。
pub(super) fn decode(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let fmt = match ext_of(path).as_str() {
        "docx" => payload::OfficeKind::Docx,
        "pptx" => payload::OfficeKind::Pptx,
        "xlsx" => payload::OfficeKind::Xlsx,
        _ => return PreviewPayload::Failed(DecodeError::Unsupported(format!("非 OOXML：{path}"))),
    };
    match bytes {
        Some(b) => match open_zip(std::io::Cursor::new(b)) {
            Ok(arch) => decode_with(Zip::new(arch, b.len()), fmt),
            Err(msg) => PreviewPayload::Failed(DecodeError::Parse(msg)),
        },
        None => match std::fs::File::open(path) {
            Ok(file) => {
                let len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
                match open_zip(BufReader::new(file)) {
                    Ok(arch) => decode_with(Zip::new(arch, len), fmt),
                    Err(msg) => PreviewPayload::Failed(DecodeError::Parse(msg)),
                }
            }
            Err(e) => PreviewPayload::Failed(DecodeError::Io(e.to_string())),
        },
    }
}

// A14 接线层：外壳（头 + 工具条）+ 载荷渲染

/// A14：带载荷渲染。`Office` 画真内容、`Failed`/未下发照实显示原因。
pub(super) fn render_with_payload(
    path: &str,
    payload: &PreviewPayload,
    theme: &Theme,
) -> AnyElement {
    match payload {
        PreviewPayload::Office(office) => office_content(path, office, theme).into_any_element(),
        PreviewPayload::Failed(err) => note_content(path, err.line(), theme).into_any_element(),
        _ => note_content(path, NOT_DELIVERED.to_owned(), theme).into_any_element(),
    }
}

/// 真内容区：既有外壳 + 块列表；空态/截断如实标注。
fn office_content(path: &str, office: &OfficePayload, theme: &Theme) -> Div {
    let kind = match office.kind {
        payload::OfficeKind::Xlsx => OfficeKind::Sheet,
        payload::OfficeKind::Pptx => OfficeKind::Slide,
        _ => OfficeKind::Word,
    };
    let mut list = div().flex().flex_col().gap_1().p_3();
    if office.blocks.is_empty() {
        list = list.child(muted_line("未解出文本内容（可能全是图形/图片）", theme));
    }
    for (idx, block) in office.blocks.iter().enumerate() {
        list = list.child(block_line(idx, block, theme));
    }
    if office.truncated {
        list = list.child(muted_line("已截断：超过预览上限（行数/字节）", theme));
    }
    shell_with(path, kind, list, theme)
}

/// 单块排版：标题带分隔线、列表项带圆点、单元格等宽、幻灯片带页码徽标。
fn block_line(idx: usize, block: &OfficeBlock, theme: &Theme) -> Div {
    let text = block.text.clone();
    let fg = theme.foreground;
    let para = div().px_2().py_1().text_sm().text_color(fg);
    let mono = div().px_2().py_1().text_xs().text_color(fg);
    match block.kind {
        Heading => {
            let h = para.py_2().border_b_1().border_color(theme.border);
            h.child(text)
        }
        Paragraph => para.child(text),
        Bullet => {
            let dot = div().size_1().rounded_full().bg(theme.muted_foreground);
            let row = div().flex().items_center().gap_2().px_2().py_1();
            let row = row.text_color(fg);
            row.child(dot).child(para.child(text))
        }
        Cell => {
            let c = mono.font_family(theme.mono_font_family.clone());
            c.child(text)
        }
        Slide => {
            let frame = div().flex().items_center().gap_2().p_2().border_1();
            let frame = frame.border_color(theme.border).rounded_sm();
            let badge = div().px_2().py_1().rounded_sm().bg(theme.muted);
            let badge = badge.text_xs().text_color(fg);
            let badge = badge.child(format!("第 {} 页", idx + 1));
            let body = div().flex_1().text_sm().text_color(fg);
            frame.child(badge).child(body.child(text))
        }
    }
}

/// 次要说明行（空态/截断标注）。
fn muted_line(text: &str, theme: &Theme) -> Div {
    let line = div().px_2().py_1().text_xs();
    let line = line.text_color(theme.muted_foreground);
    line.child(text.to_owned())
}

/// 失败/未下发共用外壳：居中一行诚实说明。
fn note_content(path: &str, note: String, theme: &Theme) -> Div {
    let kind = office_kind(path);
    let card = div().px_3().py_2().rounded_sm().bg(theme.background);
    let card = card.border_1().border_color(theme.border);
    let card = card.text_sm().text_color(theme.foreground);
    let card = card.child(note);
    let center = div().flex().flex_1().items_center().justify_center();
    let center = center.p_6().child(card);
    shell_with(path, kind, center, theme)
}

#[cfg(all(test, feature = "ui"))]
mod tests {
    use super::OfficeBlockKind::{Bullet, Cell, Heading, Paragraph, Slide};
    use super::{decode, kind_label, office_kind, render_with_payload, toolbar_items, OfficeKind};
    use crate::state::preview_payload::{
        DecodeError, OfficeBlockKind, OfficeKind as PayloadKind, OfficePayload, PreviewPayload,
        MAX_LINES,
    };
    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use gpui_kit::component::Theme;
    use std::io::Write;

    /// 测试用最小 zip 容器（存储不压缩）；解码断言全部走 `bytes = Some` 路径，不碰磁盘。
    fn zip_bytes(files: &[(&str, &str)]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in files {
            let opts = zip::write::SimpleFileOptions::default();
            writer.start_file(*name, opts).unwrap();
            writer.write_all(data.as_bytes()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    /// 断言辅助：解出 Office 载荷 / 取块种类、块文本序列。
    fn office_of(path: &str, bytes: &[u8]) -> OfficePayload {
        match decode(path, Some(bytes)) {
            PreviewPayload::Office(p) => p,
            other => panic!("{path} 应解出 Office 载荷：{other:?}"),
        }
    }

    fn kinds_of(p: &OfficePayload) -> Vec<OfficeBlockKind> {
        p.blocks.iter().map(|b| b.kind).collect()
    }

    fn texts_of(p: &OfficePayload) -> Vec<&str> {
        p.blocks.iter().map(|b| b.text.as_str()).collect()
    }

    /// 断言①：docx/pptx/xlsx 三类真解码，字段逐项正确（块数/种类/文本/页序）。
    #[test]
    fn decode_extracts_blocks_for_all_kinds() {
        let docx = zip_bytes(&[(
            "word/document.xml",
            r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body><w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>标题</w:t></w:r></w:p><w:p><w:r><w:t>普通段落</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/></w:numPr></w:pPr><w:r><w:t>列表项</w:t></w:r></w:p></w:body></w:document>"#,
        )]);
        let p = office_of("计划.docx", &docx);
        assert_eq!(p.kind, PayloadKind::Docx);
        assert_eq!(kinds_of(&p), [Heading, Paragraph, Bullet]);
        assert_eq!(texts_of(&p), ["标题", "普通段落", "列表项"]);
        let pptx = zip_bytes(&[
            ("ppt/slides/slide10.xml", r#"<p:sld><a:t>十</a:t></p:sld>"#),
            ("ppt/slides/slide2.xml", r#"<p:sld><a:t>二</a:t></p:sld>"#),
            ("ppt/slides/slide1.xml", r#"<p:sld><a:t>一</a:t></p:sld>"#),
        ]);
        let p = office_of("路演.pptx", &pptx);
        assert_eq!(p.kind, PayloadKind::Pptx);
        assert_eq!(kinds_of(&p), [Slide, Slide, Slide]);
        assert_eq!(texts_of(&p), ["一", "二", "十"], "页序应为 1、2、10");
        let xlsx = zip_bytes(&[
            ("xl/sharedStrings.xml", r#"<sst><si><t>你好</t></si></sst>"#),
            (
                "xl/worksheets/sheet1.xml",
                r#"<worksheet><sheetData><row><c r="A1" t="s"><v>0</v></c><c r="B1"><v>42</v></c><c r="C1" t="inlineStr"><is><t>内联</t></is></c></row></sheetData></worksheet>"#,
            ),
        ]);
        let p = office_of("预算.xlsx", &xlsx);
        assert_eq!(p.kind, PayloadKind::Xlsx);
        assert_eq!(kinds_of(&p), [Cell, Cell, Cell]);
        assert_eq!(texts_of(&p), ["你好", "42", "内联"]);
    }

    /// 断言②：垃圾/缺条目/非 OOXML/路径读不到 ⇒ Failed（fail-closed，不 panic）。
    #[test]
    fn decode_garbage_fails_closed() {
        let p = decode("a.docx", Some(b"not a zip at all"));
        assert!(matches!(p, PreviewPayload::Failed(_)));
        let empty = zip_bytes(&[("unrelated.txt", "hi")]);
        let p = decode("a.docx", Some(&empty));
        assert!(matches!(p, PreviewPayload::Failed(_)), "缺条目应失败");
        assert!(matches!(
            decode("legacy.doc", Some(b"whatever")),
            PreviewPayload::Failed(DecodeError::Unsupported(_))
        ));
        assert!(matches!(
            decode("/nonexistent/不存在.docx", None),
            PreviewPayload::Failed(DecodeError::Io(_))
        ));
    }

    /// 断言③：块数超 [`MAX_LINES`] ⇒ 截断标注 + 恰好保留上限行。
    #[test]
    fn decode_truncates_over_max_lines() {
        let mut xml = String::from(r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body>"#);
        for i in 0..(MAX_LINES + 5) {
            xml.push_str(&format!("<w:p><w:r><w:t>第{i}行</w:t></w:r></w:p>"));
        }
        xml.push_str("</w:body></w:document>");
        let zip = zip_bytes(&[("word/document.xml", xml.as_str())]);
        let p = office_of("大文档.docx", &zip);
        assert!(p.truncated, "超行数必须如实标注截断");
        assert_eq!(p.blocks.len(), MAX_LINES);
    }

    /// 断言④：真载荷 / 失败态 / 未下发态渲染均不 panic 且元素树非空。
    #[test]
    fn render_with_payload_paints_all_states() {
        let not_delivered = PreviewPayload::NotDelivered {
            reason: "未下发"
        };
        let bounds =
            draw_probe(|_| render_with_payload("计划.docx", &not_delivered, &Theme::default()));
        assert_rendered(bounds, "未下发态");
        let docx = zip_bytes(&[(
            "word/document.xml",
            r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body><w:p><w:r><w:t>标题甲</w:t></w:r></w:p></w:body></w:document>"#,
        )]);
        let payload = decode("计划.docx", Some(&docx));
        let bounds = draw_probe(|_| render_with_payload("计划.docx", &payload, &Theme::default()));
        assert_rendered(bounds, "Office 载荷");
        let failed = PreviewPayload::Failed(DecodeError::Parse("测试".into()));
        let bounds = draw_probe(|_| render_with_payload("计划.docx", &failed, &Theme::default()));
        assert_rendered(bounds, "失败态");
    }

    /// 纯函数层：三类分派 + 未知/大小写兜底；工具条项与标签三类互异。
    #[test]
    fn pure_layer_dispatch_toolbar_labels() {
        let cases = [
            ("计划.doc", OfficeKind::Word),
            ("预算.xlsx", OfficeKind::Sheet),
            ("路演.ppt", OfficeKind::Slide),
            ("blob.bin", OfficeKind::Word),
            ("UP.DOCX", OfficeKind::Word),
        ];
        for (path, kind) in cases {
            assert_eq!(office_kind(path), kind, "{path}");
        }
        assert_eq!(
            toolbar_items(OfficeKind::Slide),
            vec!["上一页", "页码", "下一页"]
        );
        assert_eq!(toolbar_items(OfficeKind::Word), vec!["缩放"]);
        assert_eq!(kind_label(OfficeKind::Word), "文档");
        assert_eq!(kind_label(OfficeKind::Sheet), "表格");
        assert_eq!(kind_label(OfficeKind::Slide), "演示");
    }
}
