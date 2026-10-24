//! 代码预览（`PreviewKind::Code`）：等宽代码区 + 行号列 + 截断提示 + 未下发态。
//!
//! 硬约束：
//! - 内容优先用运行时下发的字节；未下发时 [`decode`] 读本地文件。
//!   拿不到 / 解不出 ⇒ `NotDelivered` / `Failed`，界面照实显示，⛔ 不伪造内容。
//! - 颜色一律取自 `theme`，⛔ 不硬编码。
use std::io::Read as _;

use crate::state::preview_payload::{CodePayload, DecodeError, PreviewPayload, MAX_BYTES};

use gpui_kit::*;

/// 截断阈值：超过则只渲染前 `MAX_LINES` 行（§7：10MB 级文件不卡渲染进程）。
pub(super) const MAX_LINES: usize = 2000;

/// 是否截断（`total > MAX_LINES`）。
pub(super) fn is_truncated(total: usize) -> bool {
    total > MAX_LINES
}

/// 可见行范围 `[start, end)`：截断时只取前 `MAX_LINES` 行，否则全量。
pub(super) fn visible_range(total: usize) -> (usize, usize) {
    if is_truncated(total) {
        (0, MAX_LINES)
    } else {
        (0, total)
    }
}

/// 行号列宽（字符数）：最大**可见**行号的十进制位数，最小 3。
pub(super) fn line_no_width(total: usize) -> usize {
    visible_range(total).1.max(1).to_string().len().max(3)
}

/// 等宽容器标记：以 `debug_selector` 挂在容器上，渲染断言据此定位等宽字体区。
pub(super) const MONO_CONTAINER_ID: &str = "preview-code-mono";

/// 等宽外壳：字体 / 底色 / 圆角 / 内边距统一取主题 token，`debug_selector` 供渲染断言定位。
fn mono_shell(theme: &gpui_kit::component::Theme) -> Div {
    div()
        .debug_selector(|| MONO_CONTAINER_ID.to_owned())
        .font_family(theme.mono_font_family.clone())
        .bg(theme.muted)
        .rounded_md()
        .p_2()
        .text_xs()
}

/// 未下发 / 失败等**诚实提示行**：一行说明，照实显示原因。
fn render_notice(text: &str, theme: &gpui_kit::component::Theme) -> AnyElement {
    mono_shell(theme)
        .text_color(theme.muted_foreground)
        .child(text.to_owned())
        .into_any_element()
}

/// 画下发的代码行：滚动容器 + 行号列 + 截断提示。
fn render_lines(lines: &[&str], theme: &gpui_kit::component::Theme) -> AnyElement {
    mono_shell(theme)
        .child(
            // ponytail: 行数 ≤ MAX_LINES 已封顶，overflow 滚动够用；虚拟化等契约下发卡改签名时再上
            div()
                .id("preview-code-scroll")
                .overflow_y_scroll()
                .max_h(px(480.0))
                .child(code_rows(lines, theme)),
        )
        .into_any_element()
}

/// 逐行渲染：行号列（右对齐、等宽补位）+ 内容；截断时追加提示行。
fn code_rows(lines: &[&str], theme: &gpui_kit::component::Theme) -> Div {
    let total = lines.len();
    let (start, end) = visible_range(total);
    let width = line_no_width(total);
    let mut rows = div().flex().flex_col().gap_0p5();
    for (idx, line) in lines.iter().enumerate().take(end).skip(start) {
        rows = rows.child(line_row(idx + 1, line, width, theme));
    }
    if is_truncated(total) {
        rows = rows.child(
            div()
                .mt_2()
                .text_color(theme.muted_foreground)
                .child(format!("已截断（仅显示前 {MAX_LINES} 行）")),
        );
    }
    rows
}

/// 单行：行号右对齐（按 `width` 补位，等宽字体下自然对齐）+ 代码文本。
fn line_row(no: usize, text: &str, width: usize, theme: &gpui_kit::component::Theme) -> Div {
    div()
        .flex()
        .gap_2()
        .child(
            div()
                .flex_shrink_0()
                .text_color(theme.muted_foreground)
                .child(format!("{no:>width$}", width = width)),
        )
        .child(div().min_w_0().child(text.to_string()))
}

#[cfg(all(test, feature = "ui"))]
mod tests {
    use super::code_rows;
    use super::{
        decode, is_truncated, line_no_width, render_with_payload, visible_range, MAX_LINES,
        MONO_CONTAINER_ID,
    };
    use crate::state::preview_payload::{DecodeError, PreviewPayload, MAX_BYTES};
    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, AnyElement, AppContext as _, Bounds, IntoElement as _,
        ParentElement as _, Pixels, TestAppContext,
    };

    /// 行号列宽：最小 3 位、按位数扩展、截断后以 2000 计。
    #[test]
    fn line_no_width_min_three_and_digit_count() {
        assert_eq!(line_no_width(9), 3);
        assert_eq!(line_no_width(1234), 4);
        assert_eq!(line_no_width(0), 3);
        assert_eq!(line_no_width(2001), 4);
    }

    /// 截断边界：1999 不截、2001 截，可见范围随之收缩到前 2000 行。
    #[test]
    fn truncation_boundary() {
        assert!(!is_truncated(1999));
        assert!(is_truncated(2001));
        assert_eq!(visible_range(1999), (0, 1999));
        assert_eq!(visible_range(2001), (0, MAX_LINES));
    }

    /// 无窗口画一帧，取等宽容器的实际 bounds（标记挂生产侧，测试直查）。
    fn draw_mono_bounds(element: AnyElement) -> Option<Bounds<Pixels>> {
        let mut ctx = TestAppContext::single();
        ctx.update(|cx| cx.set_global(Theme::default()));
        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(800.0), px(600.0)),
            |_window, _cx| element,
        );
        visual.debug_bounds(MONO_CONTAINER_ID)
    }

    /// 未下发态：等宽容器可定位 + 布局非零 + 不 panic（原因由载荷如实带出）。
    #[test]
    fn undelivered_state_locates_mono_container() {
        let payload = PreviewPayload::NotDelivered {
            reason: crate::state::preview_payload::DECODE_NOT_DELIVERED,
        };
        let bounds = draw_mono_bounds(render_with_payload("main.rs", &payload, &Theme::default()));
        assert_rendered(bounds, "代码预览未下发态等宽容器");
    }

    /// 有码截断态：整棵行树（含截断提示行）能完整画过且探针非零。
    #[test]
    fn truncated_rows_tree_renders_non_empty() {
        let bounds = draw_probe(|_cx| {
            let lines = vec!["fn main() {}"; 2501];
            code_rows(&lines, &Theme::default()).into_any_element()
        });
        assert_rendered(bounds, "代码预览截断态行树");
    }

    /// ① 正常字节（`bytes = Some`）：载荷字段正确（语言表含别名与未知回退）。
    #[test]
    fn decode_normal_bytes_fields() {
        let PreviewPayload::Code(p) = decode("main.rs", Some(b"fn main() {}\nlet x = 1;\n")) else {
            panic!("正常字节应解出代码载荷");
        };
        assert_eq!(p.language, "rust");
        assert_eq!(p.lines, ["fn main() {}", "let x = 1;"]);
        assert!(!p.truncated);
        for (path, lang) in [
            ("a.ts", "ts"),
            ("b.TSX", "tsx"),
            ("c.py", "py"),
            ("d.yml", "yaml"),
            ("e.toml", "toml"),
            ("f.md", "md"),
            ("g.unknown", "text"),
            ("noext", "text"),
        ] {
            let PreviewPayload::Code(q) = decode(path, Some(b"x")) else {
                panic!("{path} 应解出代码载荷");
            };
            assert_eq!(q.language, lang, "{path}");
        }
    }

    /// ② 垃圾字节：fail-closed 返回 `Failed(Parse)`，⛔ 不 panic、不伪装成空内容。
    #[test]
    fn decode_garbage_bytes_fails_closed() {
        match decode("x.rs", Some(&[0xff, 0xfe, 0x00, 0x41])) {
            PreviewPayload::Failed(DecodeError::Parse(_)) => {}
            other => panic!("垃圾字节应 fail-closed，实测 {other:?}"),
        }
        // 末尾不完整的多字节序列（未截断时不得静默吞掉）也判失败。
        assert!(matches!(
            decode("y.rs", Some(b"let s = \"\xe4\xb8")),
            PreviewPayload::Failed(_)
        ));
    }

    /// ③ 超上限：行数超 `MAX_LINES` / 字节超 `MAX_BYTES` ⇒ `truncated == true` 且如实截断。
    #[test]
    fn decode_over_limits_truncates() {
        let over_lines = "a\n".repeat(MAX_LINES + 1);
        let PreviewPayload::Code(p) = decode("m.rs", Some(over_lines.as_bytes())) else {
            panic!("超行数字节应解出代码载荷");
        };
        assert!(p.truncated);
        assert_eq!(p.lines.len(), MAX_LINES);

        let over_bytes = vec![b'a'; MAX_BYTES + 1];
        let PreviewPayload::Code(p) = decode("m.rs", Some(&over_bytes)) else {
            panic!("超字节数字节应解出代码载荷");
        };
        assert!(p.truncated);
        assert_eq!(p.lines.len(), 1, "无换行 ⇒ 单行");

        // 字节截断劈开多字节字符：退到最长合法 UTF-8 前缀（丢被劈开的尾字符）。
        let mut split = vec![b'a'; MAX_BYTES + 1];
        split[MAX_BYTES - 2] = 0xe4;
        split[MAX_BYTES - 1] = 0xb8;
        split[MAX_BYTES] = 0x80;
        let PreviewPayload::Code(p) = decode("m.rs", Some(&split)) else {
            panic!("劈开多字节字符应仍解出代码载荷");
        };
        assert!(p.truncated);
        assert_eq!(p.lines, ["a".repeat(MAX_BYTES - 2)]);
    }

    /// ④ 的宿主视图：代码态含滚动容器，裸元素 draw 会在 gpui `current_view` 处 panic，
    /// 须在视图实体内绘制（同 preview_text 测试宿主的做法）。
    struct CodeHost {
        payload: PreviewPayload,
    }

    impl gpui_kit::Render for CodeHost {
        fn render(
            &mut self,
            _window: &mut gpui_kit::Window,
            _cx: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            div().child(render_with_payload(
                "main.rs",
                &self.payload,
                &Theme::default(),
            ))
        }
    }

    /// ④ 三种载荷（代码 / 失败 / 未下发）画过 `render_with_payload` 不 panic 且容器可定位。
    #[test]
    fn render_with_payload_all_states_no_panic() {
        for payload in [
            decode("main.rs", Some(b"fn main() {}\nlet y = 2;\n")),
            PreviewPayload::Failed(DecodeError::Io("nope".into())),
            PreviewPayload::NotDelivered {
                reason: crate::state::preview_payload::DECODE_NOT_DELIVERED,
            },
        ] {
            let mut ctx = TestAppContext::single();
            ctx.update(|cx| cx.set_global(Theme::default()));
            let visual = ctx.add_empty_window();
            let (_layout, _prepaint) = visual.draw(
                point(px(0.0), px(0.0)),
                size(px(800.0), px(600.0)),
                |_window, cx| cx.new(|_| CodeHost { payload }).into_any_element(),
            );
            let bounds = visual.debug_bounds(MONO_CONTAINER_ID);
            assert_rendered(bounds, "代码预览载荷态等宽容器");
        }
    }
}

// ---------------------------------------------------------------------------
// A14 解码入口（卡 `ui-a14-decode-code`）
// ---------------------------------------------------------------------------
//
//   · `decode`              —— 字节下发 ⇒ 只用传入字节；未下发 ⇒ 读本地文件。
//                             拿不到 ⇒ `NotDelivered` / `Failed`，⛔ 绝不伪造内容、⛔ 不吞错。
//   · `render_with_payload` —— 把载荷画出来（复用上方行号 / 等宽外壳）。

/// A14：把待预览文件解码成代码载荷。
///
/// `bytes = Some(..)` ⇒ **只用传入字节**（不碰文件系统，便于测试）；
/// `bytes = None` ⇒ 自行读 `path`（读失败 ⇒ `Failed(Io)`）。
/// 字节超 [`MAX_BYTES`] 或行数超 [`MAX_LINES`] ⇒ 截断并置 `truncated`；
/// 拿不到 / 解不出 ⇒ `Failed`，⛔ 绝不伪造内容。
pub(super) fn decode(path: &str, bytes: Option<&[u8]>) -> PreviewPayload {
    let owned;
    let buf: &[u8] = match bytes {
        Some(b) => b,
        None => {
            owned = match read_capped(path) {
                Ok(v) => v,
                Err(e) => {
                    return PreviewPayload::Failed(DecodeError::Io(format!("{path}: {e}")));
                }
            };
            &owned
        }
    };
    // read_capped 已封顶 ⇒ 超限只可能来自调用方直接下发的字节。
    let over = bytes.is_some_and(|b| b.len() > MAX_BYTES);
    let buf = if over { &buf[..MAX_BYTES] } else { buf };
    let text = match to_text(buf, over) {
        Ok(t) => t,
        Err(e) => return PreviewPayload::Failed(e),
    };
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut truncated = over;
    if lines.len() > MAX_LINES {
        lines.truncate(MAX_LINES);
        truncated = true;
    }
    PreviewPayload::Code(CodePayload {
        language: language_of(path).to_owned(),
        lines,
        truncated,
    })
}

/// 读本地文件，最多 `MAX_BYTES`+1 字节即停（探测超限，⛔ 不整包进内存）。
fn read_capped(path: &str) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut buf)?;
    buf.truncate(MAX_BYTES);
    Ok(buf)
}

/// 字节 → UTF-8 文本。垃圾字节 ⇒ `Err(Parse)`（fail-closed，⛔ 不用 lossy 伪装）；
/// 仅当 `MAX_BYTES` 截断劈开了多字节字符（错误在末尾且不完整）时，退到最长合法前缀。
fn to_text(buf: &[u8], over: bool) -> Result<String, DecodeError> {
    match std::str::from_utf8(buf) {
        Ok(s) => Ok(s.to_owned()),
        Err(e) => {
            let split_tail = over && e.error_len().is_none();
            let prefix = &buf[..e.valid_up_to()];
            match (split_tail, std::str::from_utf8(prefix)) {
                (true, Ok(s)) => Ok(s.to_owned()),
                _ => Err(DecodeError::Parse("不是合法的 UTF-8 文本".to_owned())),
            }
        }
    }
}

/// 按扩展名判语言（未知 ⇒ `text`）。
fn language_of(path: &str) -> &'static str {
    let Some((_, ext)) = path.rsplit_once('.') else {
        return "text";
    };
    match ext.to_ascii_lowercase().as_str() {
        "rs" => "rust",
        "ts" => "ts",
        "tsx" => "tsx",
        "js" => "js",
        "py" => "py",
        "go" => "go",
        "java" => "java",
        "c" => "c",
        "cpp" => "cpp",
        "sh" => "sh",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "md" => "md",
        _ => "text",
    }
}

/// A14：把载荷画出来。代码载荷走行号 + 等宽行；
/// 未下发 / 失败 / 载荷类型不符 ⇒ 诚实提示行（⛔ 不伪造内容）。
pub(super) fn render_with_payload(
    path: &str,
    payload: &PreviewPayload,
    theme: &gpui_kit::component::Theme,
) -> AnyElement {
    // 文件名头 / 元信息行由 preview.rs 外壳负责，这里只画主体。
    let _ = path;
    match payload {
        PreviewPayload::Code(p) => {
            let lines: Vec<&str> = p.lines.iter().map(String::as_str).collect();
            render_lines(&lines, theme)
        }
        PreviewPayload::NotDelivered { reason } => render_notice(reason, theme),
        PreviewPayload::Failed(err) => render_notice(&err.line(), theme),
        _ => render_notice("代码预览收到非代码载荷（不伪造内容）", theme),
    }
}
