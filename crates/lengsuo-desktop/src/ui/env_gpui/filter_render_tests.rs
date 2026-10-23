//! 环境筛选栏的专属断言：搜索框 / 「清除筛选」按钮 / 清筛选后输入框被清空。
//!
//! 为什么单独成文件：`filter_bar` 的两个渲染函数是 `pub(super)`，只有
//! `env_gpui` 内部能调到；放进 `render_assert_tests/` 会破坏可见性约束。
//!
//! 为什么前两条用例经一个测试侧 `Render` 视图渲染：搜索框（`Textarea`）与
//! 清除按钮（`on_click`）都会在 paint 阶段注册鼠标监听，而 gpui 的
//! `Interactivity::paint_mouse_listeners` 需要 `window.current_view()` ——
//! 直接画裸元素时渲染视图栈为空会 unwrap panic。走实体根视图（与
//! `render_assert_tests` 的既有做法一致）才有当前视图。
#![cfg(all(test, feature = "ui"))]

use super::filter_bar::{render_clear_filter_button, render_search_box};
use super::mapping::demo_state;
use super::EnvViewGpui;
use crate::ui::render_assert_tests::assert_rendered;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::Theme;
use gpui_kit::AppContext as _;
use gpui_kit::{
    div, point, px, size, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, TestAppContext, Window,
};

const SEARCH_SELECTOR: &str = "e56-search-probe";
const CLEAR_SELECTOR: &str = "e56-clear-probe";
const CANVAS_W: f32 = 800.0;
const CANVAS_H: f32 = 600.0;

fn new_ctx() -> TestAppContext {
    let ctx = TestAppContext::single();
    ctx.update(|cx| cx.set_global(Theme::default()));
    ctx
}

/// 只渲染搜索框的测试视图（`TextareaState` 需要 `Window`，只能在 render 里建）。
struct SearchProbe;

impl Render for SearchProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let textarea = cx.new(|cx| TextareaState::new(window, cx));
        div().flex().items_start().child(
            div()
                .debug_selector(|| SEARCH_SELECTOR.to_owned())
                .child(render_search_box(&textarea, &Theme::default())),
        )
    }
}

/// 只渲染「清除筛选」按钮的测试视图。
struct ClearProbe {
    entity: Entity<EnvViewGpui>,
}

impl Render for ClearProbe {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().items_start().child(
            div()
                .debug_selector(|| CLEAR_SELECTOR.to_owned())
                .child(render_clear_filter_button(&self.entity, &Theme::default())),
        )
    }
}

#[test]
fn search_box_renders_with_a_live_textarea() {
    let mut ctx = new_ctx();
    let visual = ctx.add_empty_window();

    let (_layout, _prepaint) = visual.draw(
        point(px(0.0), px(0.0)),
        size(px(CANVAS_W), px(CANVAS_H)),
        |_window, cx| cx.new(|_cx| SearchProbe).into_any_element(),
    );

    assert_rendered(visual.debug_bounds(SEARCH_SELECTOR), "环境视图搜索框");
}

#[test]
fn clear_filter_button_renders() {
    let mut ctx = new_ctx();
    let visual = ctx.add_empty_window();

    let (_layout, _prepaint) = visual.draw(
        point(px(0.0), px(0.0)),
        size(px(CANVAS_W), px(CANVAS_H)),
        |_window, cx| {
            let entity = cx.new(|_cx| EnvViewGpui::new(demo_state()));
            cx.new(|_cx| ClearProbe { entity }).into_any_element()
        },
    );

    assert_rendered(visual.debug_bounds(CLEAR_SELECTOR), "筛选空态的清除按钮");
}

#[test]
fn sync_after_clear_empties_the_search_box() {
    let mut ctx = new_ctx();
    let visual = ctx.add_empty_window();

    let (_layout, _prepaint) = visual.draw(
        point(px(0.0), px(0.0)),
        size(px(CANVAS_W), px(CANVAS_H)),
        |window, cx| {
            let mut state = demo_state();
            state.set_keyword("构建");
            let entity = cx.new(|_cx| EnvViewGpui::new(state));

            entity.update(cx, |view, cx| {
                view.filter.ensure(window, cx);
                let textarea = view
                    .filter
                    .textarea()
                    .cloned()
                    .expect("ensure 之后必须已有输入框");

                // 幂等：重复 ensure 不得把编辑器丢掉。
                view.filter.ensure(window, cx);
                assert!(
                    view.filter.textarea().is_some(),
                    "重复 ensure 后输入框不得消失"
                );

                textarea.update(cx, |t, tcx| {
                    t.set_value("构建".to_owned(), window, tcx);
                });
                assert_eq!(
                    textarea.read(cx).value().to_string(),
                    "构建",
                    "写入必须同步生效"
                );

                view.filter.sync_after_clear(window, cx);
                assert_eq!(
                    textarea.read(cx).value().to_string(),
                    "",
                    "清筛选后输入框必须被清空，否则界面与状态对不上"
                );
            });

            div().into_any_element()
        },
    );
}
