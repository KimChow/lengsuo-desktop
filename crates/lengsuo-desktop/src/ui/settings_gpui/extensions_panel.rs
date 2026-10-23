//! 「扩展」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 把描述层（`view::extensions`）的两分支映射成元素：
//! 未下发 ⇒ 标题 + 「未下发」提示（fail-closed）；已下发 ⇒ 标题 + 条目行清单。
//! 行**只读**：状态标签与动作文案是静态文本，⛔ 不渲染成可点按钮、不给增删改。
//! 颜色一律取自 `theme`（⛔ 不硬编码）、⛔ 界面不判定权限、⛔ 界面不持有凭据。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::extensions as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 渲染「扩展」面板：未下发显示提示，已下发显示只读行清单。
///
/// 服务端尚未接入下发通道 ⇒ 当前恒走未下发分支（⛔ 不伪造服务端数据）。
pub(super) fn render_extensions_panel(
    theme: &Theme,
    _cx: &mut Context<SettingsGpui>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(panel_view::TITLE),
        )
        .child(match panel_view::body(None) {
            panel_view::Body::NotDelivered => div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::NOT_DELIVERED)
                .into_any_element(),
            panel_view::Body::Delivered(rows) => div()
                .flex()
                .flex_col()
                .gap_2()
                // 分区标题（`settings.extensions.title`）：条目清单的类别名。
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(panel_view::ITEM_TITLE),
                )
                .children(rows.iter().map(|row| render_row(theme, row)))
                .into_any_element(),
        })
        .into_any_element()
}

/// 一条只读行：条目名 + 状态标签 + 当前可用动作文案。
///
/// ⛔ 纯展示：不给交互（行不可点、无增删改按钮）；参考的动作文案按状态展示为静态文本。
fn render_row(theme: &Theme, row: &panel_view::Row) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(row.name.clone()),
        )
        .children(row.tag().map(|tag| {
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(tag)
        }))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .ml_auto()
                .child(row.action()),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不写 `use super::*`：GPUI 的 `test` 属性宏会随 glob 遮蔽标准库的
    // `#[test]`（口径同 `render_assert_tests/mod.rs`），必须显式列出符号。
    use super::render_row;
    use crate::ui::settings_gpui::view::extensions as panel_view;
    use crate::ui::settings_gpui::view::PanelId;
    use crate::ui::settings_gpui::SettingsGpui;
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, AnyElement, AppContext as _, IntoElement as _, ParentElement as _,
        Styled as _, TestAppContext,
    };

    /// 无窗口绘制一帧：跑完整 layout + prepaint + paint（不 panic 即该分支没崩）。
    ///
    /// 取 builder 而非成品元素：含 Entity 的元素必须与绘制共用同一个 TestAppContext。
    fn draw(build: impl FnOnce(&mut TestAppContext) -> AnyElement) {
        let mut ctx = TestAppContext::single();
        // 接线层经 `cx.theme()` 取色；不预置全局 Theme 会 panic（同断言套件口径）。
        ctx.update(|cx| cx.set_global(Theme::default()));
        let element = build(&mut ctx);
        let visual = ctx.add_empty_window();
        visual.draw(
            point(px(0.0), px(0.0)),
            size(px(800.0), px(600.0)),
            |_window, _cx| div().child(element).into_any_element(),
        );
    }

    /// 未下发分支（当前恒走）经真实分发路径渲染不 panic。
    #[test]
    fn not_delivered_branch_renders_without_panic() {
        draw(|ctx| {
            ctx.new(|cx| SettingsGpui::with_panel(cx, PanelId::Extensions))
                .into_any_element()
        });
    }

    /// 内容分支（有行清单）渲染不 panic：开启 / 禁用中两种状态各画一行。
    #[test]
    fn delivered_branch_rows_render_without_panic() {
        let theme = Theme::default();
        let rows = [
            panel_view::Row {
                name: "agent-01".to_owned(),
                disabled: false,
            },
            panel_view::Row {
                name: "agent-02".to_owned(),
                disabled: true,
            },
        ];
        draw(|_| {
            div()
                .flex()
                .flex_col()
                .children(rows.iter().map(|row| render_row(&theme, row)))
                .into_any_element()
        });
    }
}
