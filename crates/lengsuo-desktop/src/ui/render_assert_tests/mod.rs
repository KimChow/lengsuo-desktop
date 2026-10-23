//! 接线层的**无窗口渲染断言**：在测试里真的跑一遍 layout + prepaint + paint。
//!
//! 本文件补的是接线层长期缺口「只做编译验证、没有渲染验证」：视图写错了
//! （元素没渲染出来 / 布局塌了 / 条件分支走错）编译期发现不了。这里用
//! `TestAppContext` + `VisualTestContext::draw` 在**无窗口**环境跑完整绘制，
//! 把这类错误变成测试失败。
//!
//! 断言分三类：
//! 1. **不 panic**：`draw` 跑完 layout + prepaint + paint 即证明该分支没崩；
//! 2. **布局非零**：目标元素的实际布局 bounds 宽高都大于 0；
//! 3. **元素可定位**：`debug_selector` 命名的元素确实出现在渲染帧里。
//!
//! ⚠️ 边界（详见 `docs/render-assert-status.md`）：
//! - 无窗口断言 **≠** 真机视觉验证；
//! - `debug_selector` 只在 **debug + test-support** 构建下记录 ⇒ release 下
//!   第 2 / 3 类断言不成立（预期，不是回归）；
//! - 不做像素级断言（本机 headless 无 GPU），也不做交互模拟。
//!
//! ⚠️ 本模块**不写 `use super::*`**：显式列出符号，避免 GPUI 的 `test` 属性宏
//! 遮蔽标准库的 `#[test]`（见 `docs/gpui-wiring-cheatsheet.md` §12.3）。
//!
//! 用例按目标分五个子模块：`blocks`（会话块纯函数）、`env`（环境视图三态）、
//! `input`（输入类视图）、`org_terminal`（组织域 + 终端抽屉）、`views`（视图族）。
#![cfg(all(test, feature = "ui"))]

use gpui_kit::component::Theme;
use gpui_kit::{
    div, point, px, size, AnyElement, Bounds, Div, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Styled as _, TestAppContext,
};

mod blocks;
mod env;
mod input;
mod org_terminal;
mod sp_claw;
mod sp_extensions;
mod sp_laboratory;
mod sp_mcp_consent;
mod sp_memory;
mod sp_models;
mod sp_software;
mod sp_subscription;
mod views;

/// 探针选择器：每帧只放一个探针，名字固定即可（每个用例独立窗口）。
pub(super) const PROBE: &str = "render-assert-probe";

/// 无窗口绘制的可用空间。
pub(super) const CANVAS_W: f32 = 800.0;
pub(super) const CANVAS_H: f32 = 600.0;

/// 测试侧探针容器：包住被测元素，并打上 `debug_selector`。
///
/// ⛔ 生产代码不在本卡可改范围，所以选择器只能加在测试侧。
/// 外层 `items_start` 让探针**不被拉伸** ⇒ 探针尺寸就是被测内容撑出来的尺寸。
pub(super) fn probe(element: impl IntoElement) -> Div {
    div()
        .flex()
        .items_start()
        .child(div().debug_selector(|| PROBE.to_owned()).child(element))
}

/// 无窗口渲染一帧，返回探针容器的实际布局 bounds。
///
/// **第 1 类断言**（不 panic）由本函数完成：`draw` 会跑完整
/// layout + prepaint + paint；能返回就说明该渲染分支没崩。
///
/// 返回 `None` ⇒ 探针没被记录进渲染帧（第 3 类断言会挂）。
pub(super) fn draw_probe(
    build: impl FnOnce(&mut TestAppContext) -> AnyElement,
) -> Option<Bounds<Pixels>> {
    let mut ctx = TestAppContext::single();
    // 接线层统一用 `cx.theme()` 取色；不预置全局 Theme 会 panic。
    ctx.update(|cx| cx.set_global(Theme::default()));

    let element = build(&mut ctx);

    let visual = ctx.add_empty_window();
    let (_layout, _prepaint) = visual.draw(
        point(px(0.0), px(0.0)),
        size(px(CANVAS_W), px(CANVAS_H)),
        |_window, _cx| probe(element).into_any_element(),
    );
    visual.debug_bounds(PROBE)
}

/// 第 2 / 3 类断言：元素可定位 + 布局非零。
pub(super) fn assert_rendered(bounds: Option<Bounds<Pixels>>, what: &str) -> Bounds<Pixels> {
    // ③ 元素可定位：探针必须出现在渲染帧里。
    assert!(
        bounds.is_some(),
        "{what}：探针没进入渲染帧（debug_selector 未记录）"
    );
    let bounds = bounds.expect("上面已断言 Some");

    // ② 布局非零：真的占到了空间。
    assert!(
        bounds.size.width > px(0.0),
        "{what}：布局宽度为 0，元素没真正参与排版"
    );
    assert!(
        bounds.size.height > px(0.0),
        "{what}：布局高度为 0，元素没真正参与排版"
    );
    bounds
}
