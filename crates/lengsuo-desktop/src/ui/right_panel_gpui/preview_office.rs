//! 办公文档预览（`PreviewKind::Office`）：A14 真解码 + 载荷渲染。
//!
//! 分两层落：纯函数层（子类型分派/标签/工具条/zip+XML 事件流解码，无 GPUI 依赖）→
//! 接线层（外壳 + 载荷渲染）。解码 ⛔ 绝不伪造内容：拿不到 ⇒ `NotDelivered` / `Failed`。
//! 颜色一律取自 `theme`，⛔ 不硬编码。
use gpui_kit::*;

use std::io::{BufReader, Read, Seek};

use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::Reader;

use crate::state::preview_payload::{
    self as payload, DecodeError, OfficeBlock, OfficeBlockKind, OfficePayload, PreviewPayload,
    MAX_BYTES, MAX_LINES,
};

/// 办公文档预览的三个子类型（由扩展名分派）。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum OfficeKind {
    /// Word 类（doc/docx；未知扩展名兜底到这里）。
    Word,
    /// 表格类（xls/xlsx/csv/tsv）。
    Sheet,
    /// 演示类（ppt/pptx）。
    Slide,
}

/// 按扩展名分派子类型：`doc/docx`→Word、`xls/xlsx/csv/tsv`→Sheet、
/// `ppt/pptx`→Slide；未知/无扩展名兜底 Word。
fn office_kind(path: &str) -> OfficeKind {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
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

/// 渲染办公文档预览：子类型标签 + 对应工具条 + 页面/网格占位区（未下发态）。
pub(super) fn render(path: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    let kind = office_kind(path);
    div()
        .flex()
        .flex_col()
        .child(header_row(path, kind, theme))
        .child(toolbar_row(kind, theme))
        .child(placeholder_area(kind, theme))
        .into_any_element()
}

/// 头部：子类型标签 + 文件路径（等宽字体）。
fn header_row(path: &str, kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .bg(theme.muted)
                .text_xs()
                .text_color(theme.foreground)
                .child(kind_label(kind)),
        )
        .child(
            div()
                .flex_1()
                .truncate()
                .text_xs()
                .text_color(theme.muted_foreground)
                .font_family(theme.mono_font_family.clone())
                .child(path.to_owned()),
        )
}

/// 工具条：按子类型显示对应操作项。未下发态下为静态展示，交互待运行时契约。
fn toolbar_row(kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme.border);
    for item in toolbar_items(kind) {
        row = row.child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .text_xs()
                .text_color(theme.foreground)
                .child(item),
        );
    }
    row
}

/// 占位区：Word/Slide 为页面、Sheet 为网格，中央叠未下发提示。
fn placeholder_area(kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    div().flex().items_center().justify_center().p_6().child(
        div()
            .relative()
            .child(placeholder_frame(kind, theme))
            .child(
                div()
                    .absolute()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .rounded_sm()
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(NOT_DELIVERED),
                    ),
            ),
    )
}

/// 占位框架：Word 竖版页、Slide 横版页、Sheet 网格。均为空画布，不伪造内容。
fn placeholder_frame(kind: OfficeKind, theme: &gpui_kit::component::Theme) -> Div {
    let frame = div()
        .bg(theme.background)
        .border_1()
        .border_color(theme.border)
        .rounded_sm();
    match kind {
        OfficeKind::Word => frame.w_64().h_96(),
        OfficeKind::Slide => frame.w_96().h_64(),
        OfficeKind::Sheet => sheet_grid(theme),
    }
}

/// 表格空画布：6×4 网格线（末行/末列不加线，避免与容器边框重叠）。
fn sheet_grid(theme: &gpui_kit::component::Theme) -> Div {
    let mut grid = div()
        .flex()
        .flex_col()
        .w_96()
        .h_64()
        .overflow_hidden()
        .rounded_sm()
        .bg(theme.background)
        .border_1()
        .border_color(theme.border);
    for row in 0..6 {
        let mut line = div().flex().flex_1();
        if row + 1 < 6 {
            line = line.border_b_1().border_color(theme.border);
        }
        for col in 0..4 {
            let mut cell = div().flex_1();
            if col + 1 < 4 {
                cell = cell.border_r_1().border_color(theme.border);
            }
            line = line.child(cell);
        }
        grid = grid.child(line);
    }
    grid
}

// ---------------------------------------------------------------------------
// A14 解码：zip 容器 + quick-xml 事件流（纯逻辑，无 GPUI 依赖，可单测）
// ---------------------------------------------------------------------------

/// OOXML 容器种类（按扩展名）；非 docx/pptx/xlsx ⇒ [`None`]（fail-closed，不猜）。
fn ooxml_kind(path: &str) -> Option<payload::OfficeKind> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "docx" => Some(payload::OfficeKind::Docx),
        "pptx" => Some(payload::OfficeKind::Pptx),
        "xlsx" => Some(payload::OfficeKind::Xlsx),
        _ => None,
    }
}

/// 解码预算：累计**抽取**的字节 ≤ [`MAX_BYTES`]（容器本体不整包进内存），超了截断并标注。
struct Budget {
    left: usize,
    truncated: bool,
}

impl Budget {
    fn new(source_len: usize) -> Self {
        Self {
            left: MAX_BYTES,
            truncated: source_len > MAX_BYTES,
        }
    }
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

/// 压入块：超过 [`MAX_LINES`] 停止累计并如实标注截断。
fn push_block(blocks: &mut Vec<OfficeBlock>, budget: &mut Budget, block: OfficeBlock) {
    if blocks.len() >= MAX_LINES {
        budget.truncated = true;
    } else {
        blocks.push(block);
    }
}

/// 读单个 zip 条目，累计抽取字节 ≤ [`MAX_BYTES`]（超预算就地截断并标注）。
/// 条目不存在 ⇒ `Ok(None)`；容器损坏 ⇒ `Err`。
fn read_entry<R: Read + Seek>(
    arch: &mut zip::ZipArchive<R>,
    name: &str,
    budget: &mut Budget,
) -> Result<Option<Vec<u8>>, String> {
    let mut entry = match arch.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(format!("zip 条目 {name}：{e}")),
    };
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = entry
            .read(&mut buf)
            .map_err(|e| format!("zip 条目 {name}：{e}"))?;
        if n == 0 {
            return Ok(Some(out));
        }
        let take = n.min(budget.left);
        out.extend_from_slice(&buf[..take]);
        budget.left -= take;
        if take < n {
            budget.truncated = true;
            return Ok(Some(out));
        }
    }
}

/// docx 段落累积器：`<w:p>` 一段，`w:val` 含 heading ⇒ 标题，`<w:numPr>` ⇒ 列表项。
struct DocxPara {
    text: String,
    heading: bool,
    bullet: bool,
}

/// `w:val` 样式名含 `heading`（忽略大小写）即视为标题（只判样式名，不猜层级）。
fn style_heading(e: &BytesStart<'_>, para: &mut Option<DocxPara>) {
    if let (Some(p), Some(val)) = (para.as_mut(), attr_value(e, "val")) {
        if val.to_ascii_lowercase().contains("heading") {
            p.heading = true;
        }
    }
}

/// docx：`word/document.xml` → 块序列（标题/段落/列表项）。
fn parse_docx_xml(xml: &str, budget: &mut Budget) -> Result<Vec<OfficeBlock>, String> {
    let mut reader = Reader::from_str(xml);
    let mut blocks = Vec::new();
    let mut para: Option<DocxPara> = None;
    let mut in_text = false;
    loop {
        let ev = match reader.read_event() {
            Ok(ev) => ev,
            // 自己截断的字节流允许中途 EOF；未截断还报错 ⇒ 结构不合法。
            Err(_) if budget.truncated => break,
            Err(e) => return Err(format!("word/document.xml：{e}")),
        };
        match ev {
            Event::Start(e) => match local_tag(e.name().as_ref()) {
                "p" => para = Some(DocxPara {
                    text: String::new(),
                    heading: false,
                    bullet: false,
                }),
                "pStyle" => style_heading(&e, &mut para),
                "numPr" => {
                    if let Some(p) = para.as_mut() {
                        p.bullet = true;
                    }
                }
                "t" => in_text = true,
                _ => {}
            },
            Event::Empty(e) => match local_tag(e.name().as_ref()) {
                "pStyle" => style_heading(&e, &mut para),
                "numPr" => {
                    if let Some(p) = para.as_mut() {
                        p.bullet = true;
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                if in_text {
                    if let Some(p) = para.as_mut() {
                        p.text.push_str(&text_content(&t));
                    }
                }
            }
            Event::End(e) => match local_tag(e.name().as_ref()) {
                "t" => in_text = false,
                "p" => {
                    if let Some(p) = para.take() {
                        if !p.text.trim().is_empty() {
                            let kind = if p.heading {
                                OfficeBlockKind::Heading
                            } else if p.bullet {
                                OfficeBlockKind::Bullet
                            } else {
                                OfficeBlockKind::Paragraph
                            };
                            push_block(
                                &mut blocks,
                                budget,
                                OfficeBlock {
                                    kind,
                                    text: p.text,
                                },
                            );
                        }
                    }
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(blocks)
}

/// 抽取所有 `<…:tag>` 元素的文本，片段间以空格连接（pptx 页文本用）。
fn extract_tagged_text(xml: &str, tag: &str, budget: &Budget) -> Result<String, String> {
    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut in_tag = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => in_tag = local_tag(e.name().as_ref()) == tag,
            Ok(Event::End(e)) => {
                if local_tag(e.name().as_ref()) == tag {
                    in_tag = false;
                }
            }
            Ok(Event::Text(t)) => {
                if in_tag {
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(&text_content(&t));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) if budget.truncated => break,
            Err(e) => return Err(format!("XML 事件流：{e}")),
            Ok(_) => {}
        }
    }
    Ok(out)
}

/// pptx：`ppt/slides/slideN.xml` 按 N 排序，每页一块 [`OfficeBlockKind::Slide`]。
fn parse_pptx_container<R: Read + Seek>(
    arch: &mut zip::ZipArchive<R>,
    budget: &mut Budget,
) -> Result<Vec<OfficeBlock>, String> {
    let mut slides: Vec<(u32, String)> = arch
        .file_names()
        .flatten()
        .filter_map(|n| {
            let num = n
                .strip_prefix("ppt/slides/slide")?
                .strip_suffix(".xml")?
                .parse::<u32>()
                .ok()?;
            Some((num, n.into_owned()))
        })
        .collect();
    slides.sort_by_key(|(num, _)| *num);
    let mut blocks = Vec::new();
    for (_, name) in slides {
        if budget.left == 0 {
            break; // 预算耗尽：余下页不再读，截断标志已置。
        }
        let Some(xml) = read_entry(arch, &name, budget)? else {
            continue;
        };
        let text = extract_tagged_text(&String::from_utf8_lossy(&xml), "t", budget)?;
        push_block(
            &mut blocks,
            budget,
            OfficeBlock {
                kind: OfficeBlockKind::Slide,
                text,
            },
        );
    }
    if blocks.is_empty() {
        return Err("pptx 容器没有幻灯片（ppt/slides/slideN.xml）".to_owned());
    }
    Ok(blocks)
}

/// xlsx：`xl/sharedStrings.xml` → 共享字符串表（`<si>` 一条；富文本 run 直接连写）。
fn parse_shared_strings(xml: &str, budget: &Budget) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_str(xml);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_si = false;
    let mut in_t = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match local_tag(e.name().as_ref()) {
                "si" => {
                    in_si = true;
                    cur.clear();
                }
                "t" => in_t = true,
                _ => {}
            },
            Ok(Event::End(e)) => match local_tag(e.name().as_ref()) {
                "si" => {
                    in_si = false;
                    out.push(std::mem::take(&mut cur));
                }
                "t" => in_t = false,
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if in_si && in_t {
                    cur.push_str(&text_content(&t));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) if budget.truncated => break,
            Err(e) => return Err(format!("sharedStrings.xml：{e}")),
            Ok(_) => {}
        }
    }
    Ok(out)
}

/// xlsx 单元格累积器：`t="s"` 查共享表，`t="inlineStr"` 取 `<is><t>`，其余取 `<v>` 原文。
struct SheetCell {
    ty: Option<String>,
    val: String,
}

/// xlsx：共享表 + 第一个工作表 → 每个非空单元格一块 [`OfficeBlockKind::Cell`]。
fn parse_xlsx_container<R: Read + Seek>(
    arch: &mut zip::ZipArchive<R>,
    budget: &mut Budget,
) -> Result<Vec<OfficeBlock>, String> {
    let shared = match read_entry(arch, "xl/sharedStrings.xml", budget)? {
        Some(xml) => parse_shared_strings(&String::from_utf8_lossy(&xml), budget)?,
        None => Vec::new(), // sharedStrings 可缺省（纯数字表）
    };
    let has_sheet1 = arch
        .file_names()
        .flatten()
        .any(|n| n == "xl/worksheets/sheet1.xml");
    let sheet_name = if has_sheet1 {
        "xl/worksheets/sheet1.xml".to_owned()
    } else {
        arch.file_names()
            .flatten()
            .find(|n| n.starts_with("xl/worksheets/") && n.ends_with(".xml"))
            .map(|n| n.into_owned())
            .ok_or("xlsx 容器缺少工作表（xl/worksheets/sheetN.xml）")?
    };
    let xml = read_entry(arch, &sheet_name, budget)?.ok_or("xlsx 工作表读取失败")?;
    parse_sheet_xml(&String::from_utf8_lossy(&xml), &shared, budget)
}

/// 工作表事件流 → 单元格块（`<c>` 一个、`<v>`/`<is><t>` 取值）。
fn parse_sheet_xml(
    xml: &str,
    shared: &[String],
    budget: &mut Budget,
) -> Result<Vec<OfficeBlock>, String> {
    let mut reader = Reader::from_str(xml);
    let mut blocks = Vec::new();
    let mut cell: Option<SheetCell> = None;
    let mut in_val = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => match local_tag(e.name().as_ref()) {
                "c" => {
                    cell = Some(SheetCell {
                        ty: attr_value(&e, "t"),
                        val: String::new(),
                    })
                }
                "v" | "t" => in_val = true,
                _ => {}
            },
            Ok(Event::Text(t)) => {
                if in_val {
                    if let Some(c) = cell.as_mut() {
                        c.val.push_str(&text_content(&t));
                    }
                }
            }
            Ok(Event::End(e)) => match local_tag(e.name().as_ref()) {
                "v" | "t" => in_val = false,
                "c" => {
                    if let Some(c) = cell.take() {
                        let text = if c.ty.as_deref() == Some("s") {
                            c.val
                                .trim()
                                .parse::<usize>()
                                .ok()
                                .and_then(|i| shared.get(i).cloned())
                        } else {
                            (!c.val.trim().is_empty()).then(|| c.val.trim().to_owned())
                        };
                        if let Some(text) = text {
                            push_block(
                                &mut blocks,
                                budget,
                                OfficeBlock {
                                    kind: OfficeBlockKind::Cell,
                                    text,
                                },
                            );
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) if budget.truncated => break,
            Err(e) => return Err(format!("worksheet：{e}")),
            Ok(_) => {}
        }
    }
    Ok(blocks)
}

/// 容器种类分派到对应解析器；解析失败 ⇒ `Err`（界面照实显示）。
fn decode_with<R: Read + Seek>(
    mut arch: zip::ZipArchive<R>,
    fmt: payload::OfficeKind,
    budget: &mut Budget,
) -> PreviewPayload {
    let parsed = match fmt {
        payload::OfficeKind::Docx => match read_entry(&mut arch, "word/document.xml", budget) {
            Ok(Some(xml)) => parse_docx_xml(&String::from_utf8_lossy(&xml), budget),
            Ok(None) => Err("docx 容器缺少 word/document.xml".to_owned()),
            Err(e) => Err(e),
        },
        payload::OfficeKind::Pptx => parse_pptx_container(&mut arch, budget),
        payload::OfficeKind::Xlsx => parse_xlsx_container(&mut arch, budget),
        payload::OfficeKind::Unknown => Err("未知办公文档格式".to_owned()),
    };
    match parsed {
        Ok(blocks) => PreviewPayload::Office(OfficePayload {
            kind: fmt,
            blocks,
            truncated: budget.truncated,
        }),
        Err(msg) => PreviewPayload::Failed(DecodeError::Parse(msg)),
    }
}

/// A14：解码待预览文件为 [`PreviewPayload::Office`]。
/// `bytes = Some(..)` 时**只用传入字节**（不碰文件系统）；`None` 时读 `path`
/// （读不到 ⇒ `Failed(Io)`）。非 OOXML / 解不开 ⇒ `Failed`，⛔ 绝不伪造内容。
pub(super) fn decode(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let Some(fmt) = ooxml_kind(path) else {
        return PreviewPayload::Failed(DecodeError::Unsupported(format!(
            "仅支持 docx/pptx/xlsx OOXML 容器：{path}"
        )));
    };
    match bytes {
        Some(b) => match zip::ZipArchive::new(std::io::Cursor::new(b)) {
            Ok(arch) => decode_with(arch, fmt, &mut Budget::new(b.len())),
            Err(e) => PreviewPayload::Failed(DecodeError::Parse(format!("不是合法的 zip 容器：{e}"))),
        },
        None => match std::fs::File::open(path) {
            Ok(file) => {
                let mut budget = Budget::new(file.metadata().map(|m| m.len() as usize).unwrap_or(0));
                match zip::ZipArchive::new(BufReader::new(file)) {
                    Ok(arch) => decode_with(arch, fmt, &mut budget),
                    Err(e) => {
                        PreviewPayload::Failed(DecodeError::Parse(format!("不是合法的 zip 容器：{e}")))
                    }
                }
            }
            Err(e) => PreviewPayload::Failed(DecodeError::Io(e.to_string())),
        },
    }
}

/// A14：带载荷渲染。`Office` 画真内容、`Failed` 照实显示原因；其余退回占位外壳。
pub(super) fn render_with_payload(
    path: &str,
    payload: &PreviewPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    match payload {
        PreviewPayload::Office(office) => office_content(path, office, theme).into_any_element(),
        PreviewPayload::Failed(err) => failed_content(path, err, theme).into_any_element(),
        _ => render(path, theme),
    }
}

/// 载荷种类 → 外壳子类型（沿用既有头/工具条）。
fn local_kind(kind: payload::OfficeKind) -> OfficeKind {
    match kind {
        payload::OfficeKind::Docx | payload::OfficeKind::Unknown => OfficeKind::Word,
        payload::OfficeKind::Xlsx => OfficeKind::Sheet,
        payload::OfficeKind::Pptx => OfficeKind::Slide,
    }
}

/// 真内容区：既有外壳（头 + 工具条）+ 块列表；空态/截断如实标注。
fn office_content(path: &str, office: &OfficePayload, theme: &gpui_kit::component::Theme) -> Div {
    let kind = local_kind(office.kind);
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
    div()
        .flex()
        .flex_col()
        .flex_1()
        .child(header_row(path, kind, theme))
        .child(toolbar_row(kind, theme))
        .child(list)
}

/// 单块排版：标题带分隔线、列表项带圆点、单元格等宽、幻灯片带页码徽标。
fn block_line(idx: usize, block: &OfficeBlock, theme: &gpui_kit::component::Theme) -> Div {
    match block.kind {
        OfficeBlockKind::Heading => div()
            .px_2()
            .py_2()
            .text_sm()
            .text_color(theme.foreground)
            .border_b_1()
            .border_color(theme.border)
            .child(block.text.clone()),
        OfficeBlockKind::Paragraph => div()
            .px_2()
            .py_1()
            .text_sm()
            .text_color(theme.foreground)
            .child(block.text.clone()),
        OfficeBlockKind::Bullet => div()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .child(div().size_1().rounded_full().bg(theme.muted_foreground))
            .child(
                div()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(block.text.clone()),
            ),
        OfficeBlockKind::Cell => div()
            .px_2()
            .py_1()
            .text_xs()
            .text_color(theme.foreground)
            .font_family(theme.mono_font_family.clone())
            .child(block.text.clone()),
        OfficeBlockKind::Slide => div()
            .flex()
            .items_center()
            .gap_2()
            .p_2()
            .border_1()
            .border_color(theme.border)
            .rounded_sm()
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(theme.muted)
                    .text_xs()
                    .text_color(theme.foreground)
                    .child(format!("第 {} 页", idx + 1)),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(block.text.clone()),
            ),
    }
}

/// 次要说明行（空态/截断标注）。
fn muted_line(text: &str, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
}

/// 失败态：既有外壳 + 居中一行诚实原因（`DecodeError::line`）。
fn failed_content(path: &str, err: &DecodeError, theme: &gpui_kit::component::Theme) -> Div {
    let kind = office_kind(path);
    div()
        .flex()
        .flex_col()
        .flex_1()
        .child(header_row(path, kind, theme))
        .child(toolbar_row(kind, theme))
        .child(
            div().flex().flex_1().items_center().justify_center().p_6().child(
                div()
                    .px_3()
                    .py_2()
                    .rounded_sm()
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(err.line()),
            ),
        )
}

#[cfg(all(test, feature = "ui"))]
mod tests {
    use super::{kind_label, office_kind, render, toolbar_items, OfficeKind};
    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use gpui_kit::component::Theme;
    use gpui_kit::px;

    /// 断言③：三类子类型的渲染元素树非空（无窗口真实绘制一帧）。
    #[test]
    fn render_office_tree_is_nonempty() {
        let cases = [
            ("计划.docx", "Word", OfficeKind::Word),
            ("预算.xlsx", "Sheet", OfficeKind::Sheet),
            ("路演.pptx", "Slide", OfficeKind::Slide),
        ];
        for (path, what, kind) in cases {
            assert_eq!(office_kind(path), kind, "{what}：分派不符");
            let bounds = draw_probe(|_| render(path, &Theme::default()));
            let bounds = assert_rendered(bounds, what);
            assert!(
                bounds.size.width > px(240.0),
                "{what}：渲染树宽度异常 {:?}",
                bounds.size
            );
        }
    }

    /// 断言①：office_kind 三类各一例 + 未知扩展名。
    #[test]
    fn office_kind_dispatches_by_extension() {
        assert_eq!(office_kind("计划.doc"), OfficeKind::Word);
        assert_eq!(office_kind("报告.docx"), OfficeKind::Word);
        assert_eq!(office_kind("预算.xls"), OfficeKind::Sheet);
        assert_eq!(office_kind("预算.xlsx"), OfficeKind::Sheet);
        assert_eq!(office_kind("导出.csv"), OfficeKind::Sheet);
        assert_eq!(office_kind("导出.tsv"), OfficeKind::Sheet);
        assert_eq!(office_kind("路演.ppt"), OfficeKind::Slide);
        assert_eq!(office_kind("/tmp/路演.pptx"), OfficeKind::Slide);
        // 未知/无扩展名：兜底 Word。
        assert_eq!(office_kind("blob.bin"), OfficeKind::Word);
        assert_eq!(office_kind("noext"), OfficeKind::Word);
        // 扩展名大小写不敏感。
        assert_eq!(office_kind("UP.DOCX"), OfficeKind::Word);
    }

    /// 断言②：Slide 工具条含翻页项；Word/Sheet 只有缩放。
    #[test]
    fn toolbar_items_match_kind() {
        let slide = toolbar_items(OfficeKind::Slide);
        assert!(slide.contains(&"上一页"), "Slide 工具条缺上一页：{slide:?}");
        assert!(slide.contains(&"下一页"), "Slide 工具条缺下一页：{slide:?}");
        assert!(slide.contains(&"页码"), "Slide 工具条缺页码：{slide:?}");
        assert_eq!(toolbar_items(OfficeKind::Word), vec!["缩放"]);
        assert_eq!(toolbar_items(OfficeKind::Sheet), vec!["缩放"]);
    }

    /// 子类型标签三类互异。
    #[test]
    fn kind_labels_are_distinct() {
        let labels = [
            kind_label(OfficeKind::Word),
            kind_label(OfficeKind::Sheet),
            kind_label(OfficeKind::Slide),
        ];
        assert_eq!(labels, ["文档", "表格", "演示"]);
    }
}
