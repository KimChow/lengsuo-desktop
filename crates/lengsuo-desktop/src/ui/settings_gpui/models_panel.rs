//! 「模型」面板的接线层（`feature = "ui"` 隔离）。
//!
//! 职责 = 把描述层（`view::models`）的状态映射成元素：面板说明、「已保存模型」
//! 三态（fail-closed：未下发 ≠ 空集）、静态目录区块（高级配置 / 能力开关 / 本地模型）。
//! 区块标题与行**只读**（不可点、无增删改、无保存按钮）；颜色一律取自 `theme`
//! （⛔ 不硬编码）。模型配置下发源尚未接线 ⇒ 生产路径固定走「未下发」分支，
//! ⛔ 不编模型名 / 不编数值。
#![cfg(feature = "ui")]
use super::window::SettingsGpui;
use crate::ui::settings_gpui::view::models as panel_view;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 渲染「模型」面板（接线层：把描述层状态映射成元素）。
///
/// 模型配置下发源尚未接线 ⇒ 固定 `None`（未下发 ≠ 空集，fail-closed）；
/// 有内容 / 空态分支由 [`models_panel`] 承接（渲染断言直接驱动它走全部分支）。
pub(super) fn render_models_panel(theme: &Theme, _cx: &mut Context<SettingsGpui>) -> AnyElement {
    models_panel(theme, None).into_any_element()
}

/// 面板体：`saved` = 已下发的模型参数值清单（`None` = 未下发）。
fn models_panel(theme: &Theme, saved: Option<&[String]>) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(title_text(theme, panel_view::TITLE))
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(panel_view::SUBTITLE),
        )
        .child(saved_block(theme, &panel_view::body(saved)))
        .children(panel_view::catalog().iter().map(|s| section(theme, s)))
}

/// 「已保存模型」区块：三态由纯函数 `view::models::body` 判定，接线层只做映射。
fn saved_block(theme: &Theme, state: &panel_view::SavedModels) -> Div {
    let block = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(title_text(theme, panel_view::SAVED_TITLE));
    match state {
        // 未下发：兜底提示（⛔ 不冒充空态、不编条目）。
        panel_view::SavedModels::NotDelivered => {
            block.child(muted_text(theme, panel_view::NOT_DELIVERED))
        }
        // 已下发的空态：标题 + 说明。
        panel_view::SavedModels::Empty => block
            .child(plain_text(theme, panel_view::EMPTY_TITLE))
            .child(muted_text(theme, panel_view::EMPTY_DESCRIPTION)),
        // 已下发的条目：模型参数值原样透传（不加动作、不加工）。
        panel_view::SavedModels::Entries(names) => {
            block.children(names.iter().map(|name| plain_text(theme, name.clone())))
        }
    }
}

/// 标题行（面板标题 / 区块标题共用一套 token）。
fn title_text(theme: &Theme, text: &'static str) -> Div {
    div()
        .text_sm()
        .font_bold()
        .text_color(theme.foreground)
        .child(text)
}

/// 静态目录区块 → 元素：标题（描述层为 `None` 的区块不编标题）+ 说明行 + 只读行。
fn section(theme: &Theme, s: &panel_view::Section) -> Div {
    let block = div().flex().flex_col().gap_2();
    let block = match s.title {
        Some(title) => block.child(title_text(theme, title)),
        None => block,
    };
    let block = if s.intro.is_empty() {
        block
    } else {
        block.child(muted_text(theme, s.intro))
    };
    block.children(s.rows.iter().map(|r| row(theme, r)))
}

/// 只读行：标签 + 说明（⛔ 不可点、无编辑动作）。
fn row(theme: &Theme, r: &panel_view::Row) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(plain_text(theme, r.label))
        .child(muted_text(theme, r.value.clone()))
}

/// 普通只读行（前景色文字）。
fn plain_text(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .text_color(theme.foreground)
        .child(text.into())
}

/// 弱化只读行（说明 / 兜底提示）。
fn muted_text(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.into())
}

#[cfg(test)]
mod tests {
    // 显式导入符号（不用 `use super::*`）：与 render_assert_tests 约定一致，
    // 避免 GPUI 的 test 属性宏遮蔽标准库 #[test]。
    use super::{models_panel, saved_block, section};
    use crate::ui::settings_gpui::view::models as panel_view;
    use gpui_kit::component::Theme;
    use gpui_kit::{
        div, point, px, size, Bounds, Div, InteractiveElement as _, IntoElement,
        ParentElement as _, Pixels, Styled as _, TestAppContext,
    };

    /// 无窗口渲染一帧，返回各选择器的布局 bounds（`None` = 未进渲染帧）。
    ///
    /// 跑完 layout + prepaint + paint 本身就是「不 panic」断言。
    fn draw_locating(root: Div, selectors: &[&'static str]) -> Vec<Option<Bounds<Pixels>>> {
        let mut ctx = TestAppContext::single();
        // 接线层统一用 theme 取色；不预置全局 Theme 会 panic。
        ctx.update(|cx| cx.set_global(Theme::default()));

        let visual = ctx.add_empty_window();
        let (_layout, _prepaint) = visual.draw(
            point(px(0.0), px(0.0)),
            size(px(800.0), px(600.0)),
            |_window, _cx| root.into_any_element(),
        );
        selectors.iter().map(|s| visual.debug_bounds(s)).collect()
    }

    /// 测试侧探针：包住被测元素并打 `debug_selector`（生产侧不掺测试钩子）。
    fn probe(element: impl IntoElement, selector: &str) -> Div {
        let selector = selector.to_owned();
        div().flex().items_start().child(
            div()
                .debug_selector(move || selector.clone())
                .child(element),
        )
    }

    /// 元素可定位 + 布局非零（与 render_assert_tests 的第 2/3 类断言同口径）。
    fn assert_non_zero(bounds: Option<Bounds<Pixels>>, what: &str) {
        let bounds = bounds.unwrap_or_else(|| panic!("{what}：未进入渲染帧"));
        assert!(bounds.size.width > px(0.0), "{what}：布局宽度为 0");
        assert!(bounds.size.height > px(0.0), "{what}：布局高度为 0");
    }

    /// 未下发分支（生产实际路径）：不 panic + 布局非零。
    #[test]
    fn undelivered_branch_draws_without_panic() {
        let root = probe(models_panel(&Theme::default(), None), "models-undelivered");
        let bounds = draw_locating(root, &["models-undelivered"]).remove(0);
        assert_non_zero(bounds, "未下发分支");
    }

    /// 空态分支（已下发但为空 ≠ 未下发）：不 panic + 布局非零。
    #[test]
    fn empty_branch_draws_without_panic() {
        let root = probe(models_panel(&Theme::default(), Some(&[])), "models-empty");
        let bounds = draw_locating(root, &["models-empty"]).remove(0);
        assert_non_zero(bounds, "空态分支");
    }

    /// 有内容分支：不 panic + 元素树能定位到 ≥2 个区块标题（验收 ② 口径）。
    ///
    /// 带标题区块 = 「已保存模型」+ 目录里带标题的区块（高级配置 / 本地模型）；
    /// 用**生产构造器**（`saved_block` / `section`）+ 生产数据（`body` / `catalog`）
    /// 拼装后逐个打探针画进同一帧，非零 bounds 即标题随区块真的上屏。
    #[test]
    fn delivered_branch_locates_two_plus_section_titles() {
        let theme = Theme::default();
        let names = vec!["gpt-4o".to_owned(), "deepseek-chat".to_owned()];

        let root = probe(models_panel(&theme, Some(&names)), "models-delivered");
        let bounds = draw_locating(root, &["models-delivered"]).remove(0);
        assert_non_zero(bounds, "有内容分支整面板");

        let titled: Vec<Div> =
            std::iter::once(saved_block(&theme, &panel_view::body(Some(&names))))
                .chain(
                    panel_view::catalog()
                        .iter()
                        .filter(|s| s.title.is_some())
                        .map(|s| section(&theme, s)),
                )
                .collect();
        assert!(
            titled.len() >= 2,
            "带标题区块必须 ≥2（已保存模型/高级配置/本地模型），实际 {}",
            titled.len()
        );

        // debug_bounds 只认 'static 选择器 ⇒ 测试侧泄漏成 'static（每帧几个短串）。
        let selectors: Vec<&'static str> = (0..titled.len())
            .map(|i| &*format!("models-section-{i}").leak())
            .collect();
        let root = div().flex().flex_col().children(
            titled
                .into_iter()
                .zip(selectors.iter().copied())
                .map(|(block, selector)| probe(block, selector)),
        );
        for (i, bounds) in draw_locating(root, &selectors).iter().enumerate() {
            assert_non_zero(*bounds, &format!("区块标题 #{i}"));
        }
    }
}
