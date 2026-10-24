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

use super::{
    preview_code, preview_diagram, preview_image, preview_office, preview_pdf, preview_text,
};

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
    let base = match kind {
        PreviewKind::Code => theme.accent,
        PreviewKind::Text => theme.muted,
        PreviewKind::Image => theme.foreground,
        PreviewKind::Pdf => theme.accent,
        PreviewKind::Office => theme.muted,
        PreviewKind::Diagram | PreviewKind::Other => theme.border,
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
        // A14：先 decode（字节由运行时下发时传 Some，当前未接入 ⇒ None）再渲染载荷。
        // ⛔ 解码器拿不到内容时返回 NotDelivered/Failed，界面照实显示，不伪造。
        PreviewKind::Code => {
            let p = preview_code::decode(path, None);
            preview_code::render_with_payload(path, &p, theme)
        }
        PreviewKind::Text => {
            let p = preview_text::decode(path, None);
            preview_text::render_with_payload(path, &p, theme)
        }
        PreviewKind::Image => {
            let p = preview_image::decode(path, None);
            preview_image::render_with_payload(path, &p, theme)
        }
        PreviewKind::Pdf => {
            let p = preview_pdf::decode(path, None);
            preview_pdf::render_with_payload(path, &p, theme)
        }
        PreviewKind::Office => {
            let p = preview_office::decode(path, None);
            preview_office::render_with_payload(path, &p, theme)
        }
        PreviewKind::Diagram => {
            let p = preview_diagram::decode(path, None);
            preview_diagram::render_with_payload(path, &p, theme)
        }
        PreviewKind::Other => unsupported(theme),
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
        .children((!dir.is_empty()).then(|| {
            div()
                .w_full()
                .truncate()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(dir)
        }))
        // fail-closed：内容未下发时必须明示，不得退化成看似空白的「正常」状态。
        .children((!content_dispatched(kind)).then(|| dispatch_notice(theme)))
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
    use super::{badge_bg, header_text, BADGE_WASH_ALPHA};
    use crate::state::preview_kind::PreviewKind;
    use gpui_kit::component::{Theme, ThemeColor};
    use gpui_kit::Hsla;

    #[test]
    fn header_text_splits_at_last_separator() {
        assert_eq!(
            header_text("src/ui/right_panel_gpui/preview.rs"),
            (
                "preview.rs".to_owned(),
                "src/ui/right_panel_gpui".to_owned()
            )
        );
        assert_eq!(
            header_text("docs/a.b/notes.md"),
            ("notes.md".to_owned(), "docs/a.b".to_owned())
        );
    }

    #[test]
    fn header_text_without_separator_has_empty_dir() {
        assert_eq!(
            header_text("README.md"),
            ("README.md".to_owned(), String::new())
        );
        assert_eq!(
            header_text("a\\b\\c.rs"),
            ("a\\b\\c.rs".to_owned(), String::new())
        );
    }

    /// 默认主题的色 token 全是占位黑（h/s/l 皆为 0），
    /// 断言「不同类型不同色」前先铺一组**有区分度**的 token。
    fn theme_with_distinct_tokens() -> Theme {
        let colors = ThemeColor {
            accent: Hsla {
                h: 0.58,
                s: 0.8,
                l: 0.5,
                a: 1.0,
            },
            muted: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.9,
                a: 1.0,
            },
            foreground: Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.1,
                a: 1.0,
            },
            border: Hsla {
                h: 0.33,
                s: 0.3,
                l: 0.7,
                a: 1.0,
            },
            ..ThemeColor::default()
        };
        Theme::from(&colors)
    }

    #[test]
    fn badge_bg_differs_across_kinds() {
        let theme = theme_with_distinct_tokens();
        let code = badge_bg(PreviewKind::Code, &theme);
        let text = badge_bg(PreviewKind::Text, &theme);
        let image = badge_bg(PreviewKind::Image, &theme);
        assert_ne!(code, text, "代码与文本的徽标底色必须可辨差异");
        assert_ne!(code, image, "代码与图片的徽标底色必须可辨差异");
        assert_ne!(text, image, "文本与图片的徽标底色必须可辨差异");
    }

    #[test]
    fn badge_bg_takes_colors_from_theme_tokens_only() {
        // 颜色本体必须逐字来自主题 token，仅透明度是徽标自己的样式参数。
        let theme = theme_with_distinct_tokens();
        let washed = |c: Hsla| Hsla {
            a: BADGE_WASH_ALPHA,
            ..c
        };
        assert_eq!(badge_bg(PreviewKind::Code, &theme), washed(theme.accent));
        assert_eq!(badge_bg(PreviewKind::Text, &theme), washed(theme.muted));
        assert_eq!(
            badge_bg(PreviewKind::Image, &theme),
            washed(theme.foreground)
        );
        assert_eq!(badge_bg(PreviewKind::Pdf, &theme), washed(theme.accent));
        assert_eq!(badge_bg(PreviewKind::Office, &theme), washed(theme.muted));
        assert_eq!(badge_bg(PreviewKind::Diagram, &theme), washed(theme.border));
        assert_eq!(badge_bg(PreviewKind::Other, &theme), washed(theme.border));
    }
}

/// 无窗口渲染断言：真的跑一遍 layout + prepaint + paint，
/// 按 `debug_selector` 定位徽标 / 提示行节点（只在 debug + test-support 构建记录）。
/// ⛔ 不用 `use super::*`（GPUI 的 `test` 宏会遮蔽标准库 `#[test]`）。
#[cfg(all(test, feature = "ui"))]
mod render_tests {
    use gpui_kit::component::Theme;
    use gpui_kit::{
        point, px, size, AppContext as _, Bounds, Context, Entity, IntoElement, Pixels, Render,
        TestAppContext, Window,
    };

    use super::{render_preview, BADGE_SELECTOR, DISPATCH_NOTICE_SELECTOR};
    use crate::ui::right_panel::PreviewContent;

    fn target(path: &str) -> PreviewContent {
        PreviewContent::Target {
            path: path.to_owned(),
        }
    }

    /// 测试探针视图：把被测内容包进一个**真实 view**。
    ///
    /// ⚠️ 必须经 `Entity` 绘制：GPUI 的文本绘制会调 `Window::current_view()`，而
    /// `rendered_entity_stack` 只在「在某个 view 内渲染」时才非空 ⇒ 直接画**裸元素**
    /// （非 `Entity`）会在 paint 期 panic（gpui `window.rs` 的 `last().copied().unwrap()`）。
    struct PreviewProbe {
        content: PreviewContent,
    }

    impl Render for PreviewProbe {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::default();
            render_preview(&self.content, &theme)
        }
    }

    /// 无窗口渲染一帧，返回 `selector` 命名节点的实际布局 bounds。
    fn draw_and_find(content: PreviewContent, selector: &'static str) -> Option<Bounds<Pixels>> {
        let mut ctx = TestAppContext::single();
        // 接线层统一用主题 token 取色；不预置全局 Theme 会 panic。
        ctx.update(|cx| cx.set_global(Theme::default()));

        let entity: Entity<PreviewProbe> = ctx.new(|_cx| PreviewProbe { content });
        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(800.0), px(600.0)),
            |_window, _cx| entity.into_any_element(),
        );
        visual.debug_bounds(selector)
    }

    #[test]
    fn badge_node_is_locatable_and_laid_out() {
        for path in ["src/main.rs", "docs/spec.pdf", "notes.md"] {
            let bounds = draw_and_find(target(path), BADGE_SELECTOR);
            let bounds =
                bounds.unwrap_or_else(|| panic!("{path}：徽标节点没进入渲染帧（未记录选择器）"));
            assert!(
                bounds.size.width > px(0.0) && bounds.size.height > px(0.0),
                "{path}：徽标宽高必须非零，真的参与排版"
            );
        }
    }

    #[test]
    fn dispatch_notice_is_locatable_while_undispatched() {
        let bounds = draw_and_find(target("notes.md"), DISPATCH_NOTICE_SELECTOR);
        assert!(
            bounds.is_some(),
            "内容未下发时必须显示统一提示行（fail-closed）"
        );
    }
}
