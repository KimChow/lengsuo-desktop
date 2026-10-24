//! 主区默认页「首页」（波次 9，卡 `ui-home-scene-badges`，差异 D3）。
//!
//! 参照形态（`ui-parity-ref-snapshot.md` §三 D3；据桌面 2 实拍）：
//! 居中标题区——大标题（`home.header.title`）+ 副标题随场景切换（`home.header.subtitle*`），
//! 标题下 3 枚圆角场景徽章（职场/开发/设计），选中徽章浅色底 + 描边；
//! 占位区下方留大输入卡位（输入卡本体属 D4，本卡只留位不实现）；
//! 再下为推荐问句区（`home.practiceCases.*`，数据由 `delivered` 驱动）与
//! 底部 AI 免责声明（`home.aiDisclaimer`）。
//!
//! 硬约束：文案逐字取自命名空间 `home.*`；数据缺失如实「未下发」（fail-closed）；
//! 颜色一律主题 token；点击只上报 [`HomeIntent`] 意图枚举，不实现行为。
#![cfg(feature = "ui")]
use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// —— 文案（逐字：文案全表命名空间 `home.*`）——

/// 标题（逐字：`home.header.title`）。
pub const TITLE: &str = "WorkBuddy, 我帮你";
/// 副标题·职场（逐字：`home.header.subtitle` / `home.header.subtitle.working`）。
pub const SUBTITLE: &str = "你的职场超能力";
/// 副标题·开发（逐字：`home.header.subtitle.coding`）。
pub const SUBTITLE_CODING: &str = "你的开发超能力";
/// 副标题·设计（逐字：`home.header.subtitle.design`）。
pub const SUBTITLE_DESIGN: &str = "你的设计超能力";
/// AI 免责声明（逐字：`home.aiDisclaimer`）。
pub const AI_DISCLAIMER: &str = "内容由 AI 生成，请核实重要信息";
/// 推荐问句区标题（逐字：`home.practiceCases.heading`）。
pub const PRACTICE_CASES_HEADING: &str = "不知道做什么，试试最佳实践案例";
/// 查看更多（逐字：`home.practiceCases.more`）。
pub const PRACTICE_CASES_MORE: &str = "更多";
/// 数据未下发说明（fail-closed，不伪造列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 场景（三枚徽章；标题行随场景切副标题）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scene {
    /// 职场。
    Working,
    /// 开发。
    Coding,
    /// 设计。
    Design,
}

impl Scene {
    /// 副标题（逐字）。
    pub fn subtitle(self) -> &'static str {
        match self {
            Scene::Working => SUBTITLE,
            Scene::Coding => SUBTITLE_CODING,
            Scene::Design => SUBTITLE_DESIGN,
        }
    }

    /// 徽章短标签。全表无独立短标签键，场景词逐字取自
    /// `home.header.subtitle*`（你的**职场**超能力…），与参照实拍徽章一致。
    pub fn badge_label(self) -> &'static str {
        match self {
            Scene::Working => "职场",
            Scene::Coding => "开发",
            Scene::Design => "设计",
        }
    }
}

/// 徽章行固定顺序（参照实拍：职场 → 开发 → 设计）。
pub const SCENES: [Scene; 3] = [Scene::Working, Scene::Coding, Scene::Design];

/// 首页交互意图：点击只上报枚举，行为由接线层实现（本卡不实现、不判权限）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeIntent {
    /// 点场景徽章：请求切换场景。
    SelectScene(Scene),
    /// 点推荐问句：请求以该问句发起会话（索引为 `delivered` 中的位置）。
    AskSuggestion(usize),
    /// 点「更多」：请求打开最佳实践案例列表。
    OpenPracticeCases,
}

/// 纯映射：徽章 → 意图（视图侧唯一「点击语义」）。
pub fn badge_intent(scene: Scene) -> HomeIntent {
    HomeIntent::SelectScene(scene)
}

/// 意图上报 sink（视图纯函数不带实体，回调经 [`Rc`] 分发）。
pub type IntentSink = Rc<dyn Fn(HomeIntent)>;

/// 渲染首页主区（视图是状态的纯函数；`delivered=None` ⇒ 未下发 fail-closed）。
///
/// 接线处（`gpui_app/main_area.rs`）当前用默认空 sink；真实分发由后续接线卡接管。
pub fn main_area(theme: &Theme, scene: Scene, delivered: Option<&[&str]>) -> Div {
    // ponytail: 默认上报落空；接线卡把 sink 换成真实分发即可，视图零改动。
    main_area_with_intents(theme, scene, delivered, Rc::new(|_| {}))
}

/// [`main_area`] 的可上报版本：徽章/问句/「更多」的点击都汇入 `on_intent`。
pub fn main_area_with_intents(
    theme: &Theme,
    scene: Scene,
    delivered: Option<&[&str]>,
    on_intent: IntentSink,
) -> Div {
    let mut area = div().flex().flex_col().items_center().gap_6();
    area = area
        .child(header(theme, scene, on_intent.clone()))
        .child(input_card_slot(theme));
    area = match delivered {
        None => area.child(not_delivered(theme)),
        Some(items) => area.child(practice_cases(theme, items, on_intent)),
    };
    area.child(footer_disclaimer(theme))
}

/// 居中标题区：标题 + 场景副标题 + 3 枚圆角场景徽章。
fn header(theme: &Theme, scene: Scene, on_intent: IntentSink) -> AnyElement {
    let mut badges = div().flex().items_center().gap_2();
    for candidate in SCENES {
        badges = badges.child(badge(
            theme,
            candidate,
            candidate == scene,
            on_intent.clone(),
        ));
    }
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_color(theme.foreground)
                .font_bold()
                .text_2xl()
                .child(TITLE),
        )
        .child(
            div()
                .text_color(theme.muted_foreground)
                .text_base()
                .child(scene.subtitle()),
        )
        .child(badges)
        .into_any_element()
}

/// 场景徽章：选中 ⇒ 浅色底（`muted`）+ 描边；未选中 ⇒ 弱化文字。
/// 触摸目标 ≥48dp（`min_h_12`）；点击只上报意图。
fn badge(theme: &Theme, scene: Scene, selected: bool, on_intent: IntentSink) -> AnyElement {
    let sink = on_intent;
    let mut el = div()
        .id(scene.badge_label())
        .on_click(move |_, _, _| sink(badge_intent(scene)))
        .px_4()
        .min_h_12()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .text_sm();
    if selected {
        el = el
            .bg(theme.muted)
            .border_1()
            .border_color(theme.border)
            .text_color(theme.foreground);
    } else {
        el = el.text_color(theme.muted_foreground);
    }
    el.child(scene.badge_label()).into_any_element()
}

/// 大输入卡位（占位）：输入卡本体属 D4（`ui-input-card`），本卡只留位（宽 ~70%）。
fn input_card_slot(theme: &Theme) -> Div {
    div()
        .w_3_4()
        .min_h_12()
        .h_40()
        .rounded_lg()
        .bg(theme.muted)
        .border_1()
        .border_color(theme.border)
}

/// 推荐问句区：标题 + 问句行 + 「更多」；数据缺失时由 [`not_delivered`] 顶替。
fn practice_cases(theme: &Theme, items: &[&str], on_intent: IntentSink) -> AnyElement {
    let mut list = div().flex().flex_col().items_center().gap_1();
    for (i, question) in items.iter().enumerate() {
        let sink = on_intent.clone();
        let row = div()
            .id(SharedString::from(format!("home-suggestion-{i}")))
            .on_click(move |_, _, _| sink(HomeIntent::AskSuggestion(i)))
            .min_h_12()
            .flex()
            .items_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .cursor_pointer()
            .child(SharedString::from(*question));
        list = list.child(row);
    }
    let sink = on_intent;
    let more = div()
        .id("home-practice-more")
        .on_click(move |_, _, _| sink(HomeIntent::OpenPracticeCases))
        .min_h_12()
        .flex()
        .items_center()
        .text_sm()
        .text_color(theme.accent)
        .cursor_pointer()
        .child(PRACTICE_CASES_MORE);
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_base()
                .font_bold()
                .text_color(theme.foreground)
                .child(PRACTICE_CASES_HEADING),
        )
        .child(list)
        .child(more)
        .into_any_element()
}

/// fail-closed：推荐问句未下发 ⇒ 区块仍在、如实标注，不伪造列表。
fn not_delivered(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_3()
        .child(
            div()
                .text_base()
                .font_bold()
                .text_color(theme.foreground)
                .child(PRACTICE_CASES_HEADING),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
        .into_any_element()
}

/// 底部 AI 免责声明。
fn footer_disclaimer(theme: &Theme) -> Div {
    div()
        .py_4()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(AI_DISCLAIMER)
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块 `use gpui_kit::*` 的 `test` 属性宏会遮蔽 `#[test]`。
    use super::{main_area, Scene, SCENES, SUBTITLE, SUBTITLE_CODING, SUBTITLE_DESIGN, TITLE};

    #[test]
    fn title_verbatim() {
        assert_eq!(TITLE, "WorkBuddy, 我帮你");
        assert_eq!(SUBTITLE, "你的职场超能力");
        assert_eq!(SUBTITLE_CODING, "你的开发超能力");
        assert_eq!(SUBTITLE_DESIGN, "你的设计超能力");
    }

    #[test]
    fn scene_subtitle_mapping() {
        assert_eq!(Scene::Working.subtitle(), SUBTITLE);
        assert_eq!(Scene::Coding.subtitle(), SUBTITLE_CODING);
        assert_eq!(Scene::Design.subtitle(), SUBTITLE_DESIGN);
    }

    #[test]
    fn badge_order_and_labels() {
        assert_eq!(SCENES.len(), 3);
        let labels: Vec<&str> = SCENES.iter().map(|s| s.badge_label()).collect();
        assert_eq!(labels, ["职场", "开发", "设计"]);
    }

    #[test]
    fn not_delivered_is_fail_closed() {
        // None ⇒ 仍渲染标题/徽章（不空白）；构建不 panic 即可。
        let theme = gpui_kit::component::Theme::default();
        let _ = main_area(&theme, Scene::Working, None);
        let _ = main_area(&theme, Scene::Design, None);
    }
}
