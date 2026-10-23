//! 输入类视图的渲染断言：`InputAreaGpui`（默认态 / plan 模式）与 `SkeletonInput`。
//!
//! 为什么这两组断言要各自建一个「探针实体」当渲染根：被测元素内部挂了
//! `Textarea` 与 `on_click` 监听，paint 阶段需要 `window.current_view()`；
//! 把裸元素当渲染根时视图栈为空 ⇒ `unwrap()` panic（e56 实测，见
//! `docs/gpui-wiring-cheatsheet.md` §12 与 `env_gpui/filter_render_tests.rs`）。
//!
//! 为什么不用 `render_assert_tests::draw_probe`：它的闭包签名拿不到
//! `&mut Window`，而 `InputAreaGpui::new` / `SkeletonInput::new` 都要 `Window`。
//! 把 `Window` 写进 helper 签名会引入可见性风险，三条用例各自内联十几行更划算。
#![cfg(all(test, feature = "ui"))]
use super::{assert_rendered, CANVAS_H, CANVAS_W};
use crate::ui::gpui_input::{SkeletonInput, PLACEHOLDER};
use crate::ui::input_area::InputMode;
use crate::ui::input_gpui::InputAreaGpui;
use gpui_kit::component::Theme;
use gpui_kit::AppContext as _;
use gpui_kit::{
    div, point, px, size, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, TestAppContext, Window,
};
const INPUT_SELECTOR: &str = "e57-input-area-probe";
const SKELETON_SELECTOR: &str = "e57-skeleton-probe";
fn new_ctx() -> TestAppContext {
    let ctx = TestAppContext::single();
    ctx.update(|cx| cx.set_global(Theme::default()));
    ctx
}
/// 输入区探针：把 `InputAreaGpui` 实体当子元素画出来。
struct InputProbe {
    entity: Entity<InputAreaGpui>,
}
impl Render for InputProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().items_start().child(
            div()
                .debug_selector(|| INPUT_SELECTOR.to_owned())
                .child(self.entity.clone()),
        )
    }
}
/// 骨架输入框探针。
struct SkeletonProbe {
    entity: Entity<SkeletonInput>,
}
impl Render for SkeletonProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().items_start().child(
            div()
                .debug_selector(|| SKELETON_SELECTOR.to_owned())
                .child(self.entity.clone()),
        )
    }
}
/// (a) 输入区默认态：画得出来 + 布局非零。
#[test]
fn input_area_renders() {
    let mut ctx = new_ctx();
    let visual = ctx.add_empty_window();
    let (_layout, _prepaint) = visual.draw(
        point(px(0.0), px(0.0)),
        size(px(CANVAS_W), px(CANVAS_H)),
        |window, cx| {
            // `InputAreaGpui::new` 需要 `&mut Window` ⇒ 只能在 draw 闭包里造。
            let entity = cx.new(|cx| InputAreaGpui::new(window, cx));
            cx.new(|_cx| InputProbe { entity }).into_any_element()
        },
    );
    assert_rendered(visual.debug_bounds(INPUT_SELECTOR), "输入区（默认态）");
}
/// (b) 输入区切到 plan 模式后：仍画得出来（高亮分支换了也要能画）。
#[test]
fn input_area_renders_in_plan_mode() {
    let mut ctx = new_ctx();
    let visual = ctx.add_empty_window();
    let (_layout, _prepaint) = visual.draw(
        point(px(0.0), px(0.0)),
        size(px(CANVAS_W), px(CANVAS_H)),
        |window, cx| {
            let entity = cx.new(|cx| InputAreaGpui::new(window, cx));
            entity.update(cx, |view, cx| {
                assert!(
                    view.select_mode(InputMode::Plan, cx),
                    "从默认模式切到 plan 必须报告发生了变化"
                );
            });
            assert_eq!(
                entity.read(cx).mode(),
                InputMode::Plan,
                "切换后描述层模式必须是 plan"
            );
            cx.new(|_cx| InputProbe { entity }).into_any_element()
        },
    );
    assert_rendered(visual.debug_bounds(INPUT_SELECTOR), "输入区（plan 模式）");
}
/// (c) 骨架输入框：画得出来 + 布局非零。
#[test]
fn skeleton_input_renders() {
    let mut ctx = new_ctx();
    let visual = ctx.add_empty_window();
    let (_layout, _prepaint) = visual.draw(
        point(px(0.0), px(0.0)),
        size(px(CANVAS_W), px(CANVAS_H)),
        |window, cx| {
            let entity = cx.new(|cx| SkeletonInput::new(window, cx));
            cx.new(|_cx| SkeletonProbe { entity }).into_any_element()
        },
    );
    assert_rendered(visual.debug_bounds(SKELETON_SELECTOR), "骨架输入框");
}
/// (d) 占位符没被清空：非空且含汉字（中文占位是复现 IME 问题的前提）。
#[test]
fn skeleton_placeholder_is_kept() {
    assert!(
        !PLACEHOLDER.trim().is_empty(),
        "骨架占位符被清空了：中文占位是复现输入法问题的前提"
    );
    assert!(
        PLACEHOLDER.contains('中'),
        "骨架占位符必须含汉字，实测 = {PLACEHOLDER:?}"
    );
}
