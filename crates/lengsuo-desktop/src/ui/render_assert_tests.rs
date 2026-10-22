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
#![cfg(all(test, feature = "ui"))]

use gpui_kit::component::Theme;
use gpui_kit::{
    div, point, px, size, AnyElement, AppContext as _, Bounds, Div, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, Styled as _, TestAppContext,
};

use crate::state::env_group::EnvGroupState;
use crate::ui::env_gpui::EnvViewGpui;
use crate::ui::org_gpui::{OrgSection, OrgViewGpui};
use crate::ui::org_view::OrgViewState;
use crate::ui::session_gpui;
use crate::ui::session_render::{Align, Block, FileItem, STALE_NOTICE};
use crate::ui::terminal_gpui::TerminalDrawerGpui;

/// 探针选择器：每帧只放一个探针，名字固定即可（每个用例独立窗口）。
const PROBE: &str = "render-assert-probe";

/// 无窗口绘制的可用空间。
const CANVAS_W: f32 = 800.0;
const CANVAS_H: f32 = 600.0;

/// 测试侧探针容器：包住被测元素，并打上 `debug_selector`。
///
/// ⛔ 生产代码不在本卡可改范围，所以选择器只能加在测试侧。
/// 外层 `items_start` 让探针**不被拉伸** ⇒ 探针尺寸就是被测内容撑出来的尺寸。
fn probe(element: impl IntoElement) -> Div {
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
fn draw_probe(build: impl FnOnce(&mut TestAppContext) -> AnyElement) -> Option<Bounds<Pixels>> {
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
fn assert_rendered(bounds: Option<Bounds<Pixels>>, what: &str) -> Bounds<Pixels> {
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

// === A. 会话流：纯函数 `render_block`（最好测的形态） ===

/// 四种块分支各画一遍：用户气泡 / 助手正文 / 文件网格 / 断档提示。
#[test]
fn session_blocks_all_render_content() {
    let blocks = [
        Block::UserBubble {
            text: "你好".to_owned(),
            align: Align::Right,
        },
        Block::AssistantText {
            text: "这是助手正文。".to_owned(),
            align: Align::Left,
        },
        Block::FilesGrid {
            files: vec![
                FileItem::new("src/main.rs"),
                FileItem::new("docs/readme.md"),
            ],
        },
        Block::StaleNotice { text: STALE_NOTICE },
    ];

    for block in &blocks {
        let bounds = draw_probe(|_ctx| session_gpui::render_block(block).into_any_element());
        assert_rendered(bounds, "会话块");
    }
}

/// 文本真的被排版了：更长的文本必须撑出更宽的元素。
///
/// 只断言「不 panic」抓不到「文本没被排版」（例如字体度量返回 0）；这条抓得到。
#[test]
fn session_longer_text_lays_out_wider() {
    let short = draw_probe(|_ctx| {
        session_gpui::render_block(&Block::UserBubble {
            text: "短".to_owned(),
            align: Align::Right,
        })
        .into_any_element()
    });
    let long = draw_probe(|_ctx| {
        session_gpui::render_block(&Block::UserBubble {
            text: "这是一条明显更长的用户消息".repeat(30),
            align: Align::Right,
        })
        .into_any_element()
    });

    let short = assert_rendered(short, "短文本气泡");
    let long = assert_rendered(long, "长文本气泡");
    assert!(
        long.size.width > short.size.width,
        "长文本没有撑出更宽的元素 ⇒ 文本没被真正排版（long={:?} short={:?}）",
        long.size.width,
        short.size.width
    );
}

// === B. 环境视图：Entity 形态，三态各一条渲染分支 ===

fn env_state_with_cards() -> EnvGroupState {
    let mut state = EnvGroupState::new();
    state.apply_entries(vec![
        serde_json::json!({"id": "e1", "name": "构建机 A", "group_id": "g1", "group_name": "华东组"}),
        serde_json::json!({"id": "e2", "name": "构建机 B"}),
    ]);
    state
}

#[test]
fn env_view_ready_state_renders() {
    let bounds = draw_probe(|ctx| {
        let state = env_state_with_cards();
        ctx.new(|_cx| EnvViewGpui::new(state)).into_any_element()
    });
    assert_rendered(bounds, "环境视图（就绪）");
}

#[test]
fn env_view_empty_state_renders() {
    let bounds = draw_probe(|ctx| {
        ctx.new(|_cx| EnvViewGpui::new(EnvGroupState::new()))
            .into_any_element()
    });
    assert_rendered(bounds, "环境视图（空态）");
}

#[test]
fn env_view_failed_state_renders_error_banner() {
    let bounds = draw_probe(|ctx| {
        let mut state = EnvGroupState::new();
        state.fail("gateway unreachable");
        ctx.new(|_cx| EnvViewGpui::new(state)).into_any_element()
    });
    assert_rendered(bounds, "环境视图（失败态）");
}

// === C. 组织域视图：三分区各画一遍 ===

fn org_view(section: OrgSection) -> OrgViewGpui {
    let mut state = OrgViewState::default();
    state
        .departments_mut()
        .apply(vec![serde_json::json!({"id": "d1", "name": "研发部"})]);
    state.members_mut().apply(vec![
        serde_json::json!({"id": "u1", "name": "甲", "email": "a@example.com"}),
    ]);
    state
        .roles_mut()
        .apply(vec![serde_json::json!({"id": "r1", "name": "管理员"})]);

    let mut view = OrgViewGpui::new(state);
    view.set_section(section);
    view
}

#[test]
fn org_view_every_section_renders() {
    for section in OrgSection::ALL {
        let bounds = draw_probe(|ctx| ctx.new(|_cx| org_view(section)).into_any_element());
        assert_rendered(bounds, section.label());
    }
}

// === D. 终端抽屉：收起 / 展开 ===

#[test]
fn terminal_drawer_collapsed_renders() {
    let bounds = draw_probe(|ctx| ctx.new(|_cx| TerminalDrawerGpui::new()).into_any_element());
    assert_rendered(bounds, "终端抽屉（收起）");
}

/// 展开态多了 tab 条 + 220px 固定内容区 + 输入行 ⇒ 一定比收起态高。
///
/// 这条能抓到「条件分支走错 / 内容区没渲染」——只断言不 panic 抓不到。
#[test]
fn terminal_drawer_expanded_is_taller_than_collapsed() {
    let collapsed = draw_probe(|ctx| ctx.new(|_cx| TerminalDrawerGpui::new()).into_any_element());
    let expanded = draw_probe(|ctx| {
        ctx.new(|_cx| {
            let mut view = TerminalDrawerGpui::new();
            view.state_mut().toggle_expanded();
            view
        })
        .into_any_element()
    });

    let collapsed = assert_rendered(collapsed, "终端抽屉（收起）");
    let expanded = assert_rendered(expanded, "终端抽屉（展开）");
    assert!(
        expanded.size.height > collapsed.size.height,
        "展开态没有比收起态高 ⇒ 内容区没渲染出来（expanded={:?} collapsed={:?}）",
        expanded.size.height,
        collapsed.size.height
    );
}
