//! 右侧面板「文件预览」tab 的内容渲染。
//!
//! 分层：描述层（`crate::ui::right_panel`）已决定「预览哪个文件」；
//! 本文件按**文件类型**分派到同目录的预览子模块，**不重新实现**任何判断。
//! 壳层统一负责：面板头（文件名 + 类型徽标 + 折叠父路径）、
//! 「未下发」提示行、外部编辑器入口占位。
//!
//! 硬约束：
//! - ⛔ **不读文件、不渲染文件内容**（解码器属后续卡；本层只做类型分派 + 诚实占位）。
//! - 颜色一律取自 `cx.theme()`，⛔ 不硬编码。
use gpui_kit::base::StyledExt as _;
use gpui_kit::*;

use crate::state::preview_kind::{classify, PreviewKind};
use crate::ui::right_panel::{PreviewContent, PREVIEW_UNSUPPORTED_TEXT};

use super::{preview_code, preview_image, preview_office, preview_pdf, preview_text};

/// 徽标底色的透明度：颜色本体取自主题 token，仅透明度是徽标的样式参数。
const BADGE_WASH_ALPHA: f32 = 0.15;

/// 「未下发」统一提示行文案。
const DISPATCH_NOTICE_TEXT: &str = "预览内容未下发";
/// 外部编辑器入口占位文案。
const EXTERNAL_EDITOR_TEXT: &str = "跳外部编辑器";

/// 徽标 / 提示行的渲染定位选择器（无窗口渲染断言按此定位节点）。
const BADGE_SELECTOR: &str = "preview-kind-badge";
const DISPATCH_NOTICE_SELECTOR: &str = "preview-dispatch-notice";

/// 纯函数：按预览类型取主题内色作为类型徽标的底色（⛔ 不硬编码色值）。
///
/// 同色系允许复用（主题 token 有限），但不同大类的徽标必须有可辨差异。
#[must_use]
fn badge_bg(kind: PreviewKind, theme: &gpui_kit::component::Theme) -> Hsla {
    let base: Hsla = match kind {
        PreviewKind::Code => theme.accent.into(),
        PreviewKind::Text => theme.muted.into(),
        PreviewKind::Image => theme.foreground.into(),
        PreviewKind::Pdf => theme.accent.into(),
        PreviewKind::Office => theme.muted.into(),
        PreviewKind::Diagram | PreviewKind::Other => theme.border.into(),
    };
    Hsla {
        a: BADGE_WASH_ALPHA,
        ..base
    }
}

/// 纯函数：把路径拆成（文件名, 父目录）。
///
/// 用 `rsplit_once('/')` 取**最后一个**分隔符；无分隔符则父目录为空串
/// （接线层据此不渲染父路径行）。
#[must_use]
fn header_text(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((dir, name)) => (name.to_owned(), dir.to_owned()),
        None => (path.to_owned(), String::new()),
    }
}

/// 运行时是否已下发该文件的内容。
///
/// fail-closed：解码器未接入前一律视为**未下发**，壳层必须明示，
/// 不得让面板退化成看似空白的「正常」状态。
/// ponytail: 解码器卡落地后改为读真实下发状态。
fn content_dispatched(_kind: PreviewKind) -> bool {
    false
}

/// 渲染「待预览文件」的壳层：面板头（文件名 + 类型徽标 + 折叠父路径）、
/// 「未下发」提示行、按类型分派的子模块内容、外部编辑器入口占位。
///
/// 文件名与类型都是真实存在的事实，不是编造的内容。
pub(super) fn render_preview(
    content: &PreviewContent,
    theme: &gpui_kit::component::Theme,
) -> impl IntoElement {
    let PreviewContent::Target { path } = content;
    let kind = classify(path);
    let (name, dir) = header_text(path);
    let body: AnyElement = match kind {
        PreviewKind::Code => preview_code::render(path, theme),
        PreviewKind::Text => preview_text::render(path, theme),
        PreviewKind::Image => preview_image::render(path, theme),
        PreviewKind::Pdf => preview_pdf::render(path, theme),
        PreviewKind::Office => preview_office::render(path, theme),
        PreviewKind::Diagram | PreviewKind::Other => unsupported(theme),
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(
            // 面板头：文件名 + 类型徽标；文件名过长省略，徽标不被挤掉。
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(name),
                )
                .child(badge(kind, theme)),
        )
        // 折叠父路径：无分隔符（父路径为空串）时不渲染这一行。
        .child((!dir.is_empty()).then(|| {
            div()
                .w_full()
                .truncate()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(dir)
        }))
        // fail-closed：内容未下发时必须明示，不得退化成看似空白的「正常」状态。
        .child((!content_dispatched(kind)).then(|| dispatch_notice(theme)))
        .child(body)
        .child(external_editor_entry(theme))
}

/// 类型徽标：圆角 + 底色 + 描边，文案为类型标签（`PreviewKind::label`）。
fn badge(kind: PreviewKind, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .px_2()
        .py_1()
        .rounded_full()
        .border_1()
        .border_color(theme.border)
        .bg(badge_bg(kind, theme))
        .text_xs()
        .text_color(theme.foreground)
        .debug_selector(|| BADGE_SELECTOR.to_owned())
        .child(kind.label())
}

/// 「未下发」统一提示行：子模块只能返回占位态时由壳层统一明示。
fn dispatch_notice(theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .debug_selector(|| DISPATCH_NOTICE_SELECTOR.to_owned())
        .child(DISPATCH_NOTICE_TEXT)
}

/// 外部编辑器入口**占位**：⛔ 本卡不接任何行为（点击意图属后续卡）。
fn external_editor_entry(theme: &gpui_kit::component::Theme) -> impl IntoElement {
    div()
        .flex()
        .justify_end()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(EXTERNAL_EDITOR_TEXT)
}

/// 尚未实现的类型（图表 / 其他）的诚实占位：⛔ 不伪造内容。
fn unsupported(theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(PREVIEW_UNSUPPORTED_TEXT)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{badge_bg, header_text};
    use crate::state::preview_kind::PreviewKind;
    use gpui_kit::component::Theme;

    #[test]
    fn header_text_splits_at_last_separator() {
        assert_eq!(
            header_text("src/ui/right_panel_gpui/preview.rs"),
            ("preview.rs".to_owned(), "src/ui/right_panel_gpui".to_owned())
        );
        assert_eq!(
            header_text("docs/a.b/notes.md"),
            ("notes.md".to_owned(), "docs/a.b".to_owned())
        );
    }

    #[test]
    fn header_text_without_separator_has_empty_dir() {
        assert_eq!(header_text("README.md"), ("README.md".to_owned(), String::new()));
        assert_eq!(header_text("a\\b\\c.rs"), ("a\\b\\c.rs".to_owned(), String::new()));
    }

    #[test]
    fn badge_bg_differs_across_kinds() {
        let theme = Theme::default();
        let code = badge_bg(PreviewKind::Code, &theme);
        let text = badge_bg(PreviewKind::Text, &theme);
        let image = badge_bg(PreviewKind::Image, &theme);
        assert_ne!(code, text, "代码与文本的徽标底色必须可辨差异");
        assert_ne!(code, image, "代码与图片的徽标底色必须可辨差异");
        assert_ne!(text, image, "文本与图片的徽标底色必须可辨差异");
    }
}
