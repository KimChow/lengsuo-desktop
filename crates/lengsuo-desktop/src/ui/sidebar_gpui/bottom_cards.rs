//! 左栏底部卡（波次 9，卡 `ui-left-bottom-cards-polish`，差异 D8）。
//!
//! 参照形态：活动横幅卡（图占位 + 标题省略 + 右下 CTA「立即前往」）+
//! 用户头像行（圆头像占位 + 名字 + 两个小图标钮）。
//! 文案逐字 `welcome.activity.*`；数据缺失如实「未下发」，不伪造。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 活动标签（逐字：`welcome.activity.label`）。
pub const ACTIVITY_LABEL: &str = "活动";
/// 活动 CTA（逐字：`welcome.activity.action`）。
pub const ACTIVITY_ACTION: &str = "立即前往";
/// 活动模型切换提示（逐字：`welcome.activity.modelSwitched`）。
pub const MODEL_SWITCHED: &str = "已为你切换到活动指定模型";
/// 通知标签（逐字：`welcome.activity.notice`）。
pub const NOTICE_LABEL: &str = "通知";
/// 未下发（fail-closed）。
pub const NOT_DELIVERED: &str = "未下发";

/// 底部卡点击意图：只上报枚举，不实现行为（行为归后续接线卡）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BottomCardIntent {
    /// 「立即前往」CTA。
    OpenActivity,
    /// 「通知」小图标钮。
    OpenNotice,
    /// 第二枚小图标钮（图标资产未下发，仅占位上报）。
    OpenSettings,
}

/// 底部卡数据（真实来源等 C1~C10）。
pub struct BottomData {
    /// 活动标题；`None` ⇒ 未下发。
    pub activity_title: Option<String>,
    /// 用户名；`None` ⇒ 未下发。
    pub user_name: Option<String>,
}

/// 数据缺失 ⇒ 「未下发」，不伪造。
fn or_not_delivered(value: Option<&str>) -> &str {
    value.unwrap_or(NOT_DELIVERED)
}

/// 渲染底部卡：活动横幅 + 用户头像行。
///
/// `on_intent` 只收意图枚举；三枚按钮各克隆一份 ⇒ `Clone`。
pub fn bottom_cards(
    theme: &Theme,
    data: &BottomData,
    on_intent: impl Fn(BottomCardIntent) + Clone + 'static,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(activity_banner(theme, data, &on_intent))
        .child(user_row(theme, data, &on_intent))
}

/// 活动横幅卡：图占位块 + 标题省略 + 模型切换提示 + 右下 CTA。
fn activity_banner(
    theme: &Theme,
    data: &BottomData,
    on_intent: &(impl Fn(BottomCardIntent) + Clone + 'static),
) -> Div {
    let cta = on_intent.clone();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(theme.sidebar_border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                // 图占位块（图片资产未下发）：主题色浅底。
                .child(
                    div()
                        .size_10()
                        .rounded_sm()
                        .bg(theme.sidebar_accent.opacity(0.35)),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .gap_1()
                        .child(
                            div()
                                .px_1()
                                .text_xs()
                                .text_color(theme.sidebar_accent_foreground)
                                .bg(theme.sidebar_accent.opacity(0.35))
                                .rounded_sm()
                                .child(ACTIVITY_LABEL),
                        )
                        // ponytail: 此版 gpui 无 line_clamp ⇒ 标题先单行省略，两行省略待上游
                        .child(
                            div()
                                .text_sm()
                                .font_bold()
                                .text_color(theme.sidebar_foreground)
                                .truncate()
                                .child(SharedString::from(or_not_delivered(
                                    data.activity_title.as_deref(),
                                ))),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child(MODEL_SWITCHED),
                        ),
                ),
        )
        .child(
            div().flex().justify_end().child(
                div()
                    .id("bottom-card-activity-cta")
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(theme.sidebar_accent)
                    .text_xs()
                    .text_color(theme.sidebar_accent_foreground)
                    .cursor_pointer()
                    .child(ACTIVITY_ACTION)
                    .on_click(move |_event, _window, _app| cta(BottomCardIntent::OpenActivity)),
            ),
        )
}

/// 用户头像行：圆头像占位 + 名字 + 两个小图标钮。
fn user_row(
    theme: &Theme,
    data: &BottomData,
    on_intent: &(impl Fn(BottomCardIntent) + Clone + 'static),
) -> Div {
    let notice = on_intent.clone();
    let settings = on_intent.clone();
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        // 圆头像占位（头像资产未下发）：主题色浅底。
        .child(
            div()
                .size_6()
                .rounded_full()
                .bg(theme.sidebar_accent.opacity(0.35)),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .text_color(theme.sidebar_foreground)
                .truncate()
                .child(SharedString::from(or_not_delivered(
                    data.user_name.as_deref(),
                ))),
        )
        // 小图标钮 1：「通知」（文案已下发 ⇒ 带字胶囊）。
        .child(
            div()
                .id("bottom-card-notice")
                .px_1()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(theme.sidebar_border)
                .text_xs()
                .text_color(theme.muted_foreground)
                .cursor_pointer()
                .child(NOTICE_LABEL)
                .on_click(move |_event, _window, _app| notice(BottomCardIntent::OpenNotice)),
        )
        // 小图标钮 2：图标资产未下发 ⇒ 空占位方块，只上报意图。
        .child(
            div()
                .id("bottom-card-settings")
                .size_6()
                .rounded_sm()
                .border_1()
                .border_color(theme.sidebar_border)
                .cursor_pointer()
                .on_click(move |_event, _window, _app| settings(BottomCardIntent::OpenSettings)),
        )
}

#[cfg(test)]
mod tests {
    //! D8 底部卡保真断言：① 文案逐字 ② fail-closed 两态 ③ 意图枚举闭集
    //! ④ 整卡/横幅/用户行渲染非零。
    //!
    //! 显式导入、不写 `use super::*`（父模块 `use gpui_kit::*` 的 `test`
    //! 属性宏会遮蔽 `#[test]` ⇒ recursion limit，同 render_assert_tests 约定）。

    use gpui_kit::component::Theme;
    use gpui_kit::{AppContext as _, IntoElement as _};

    use crate::ui::render_assert_tests::{assert_rendered, draw_probe};
    use crate::ui::sidebar_gpui::render::HostedElement;

    use super::{
        activity_banner, bottom_cards, or_not_delivered, user_row, BottomCardIntent, BottomData,
        ACTIVITY_ACTION, ACTIVITY_LABEL, MODEL_SWITCHED, NOTICE_LABEL, NOT_DELIVERED,
    };

    fn sample_data() -> BottomData {
        BottomData {
            activity_title: Some("1024 造物挑战".into()),
            user_name: Some("kim".into()),
        }
    }

    /// ① 文案逐字（`welcome.activity.*` 命名空间，不增删一字）。
    #[test]
    fn labels_are_verbatim() {
        assert_eq!(ACTIVITY_LABEL, "活动");
        assert_eq!(ACTIVITY_ACTION, "立即前往");
        assert_eq!(MODEL_SWITCHED, "已为你切换到活动指定模型");
        assert_eq!(NOTICE_LABEL, "通知");
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    /// ② fail-closed：标题 / 用户名缺失 ⇒ 如实「未下发」，不伪造内容。
    #[test]
    fn missing_data_falls_back_to_not_delivered() {
        assert_eq!(or_not_delivered(None), "未下发");
        assert_eq!(or_not_delivered(Some("1024 造物挑战")), "1024 造物挑战");
    }

    /// ③ 意图枚举闭集：无通配 match ⇒ 加变体必须连带改这里；三变体互异。
    #[test]
    fn intent_enum_is_closed_over_three_buttons() {
        fn intent_name(intent: BottomCardIntent) -> &'static str {
            match intent {
                BottomCardIntent::OpenActivity => "activity",
                BottomCardIntent::OpenNotice => "notice",
                BottomCardIntent::OpenSettings => "settings",
            }
        }
        assert_eq!(intent_name(BottomCardIntent::OpenActivity), "activity");
        assert_ne!(BottomCardIntent::OpenNotice, BottomCardIntent::OpenSettings);
    }

    /// ④ 整卡两态（有数据 / 全未下发）都渲染非零。
    #[test]
    fn full_card_renders_nonzero_in_both_states() {
        let theme = Theme::default();
        let draw = |data: &BottomData| {
            draw_probe(|ctx| {
                let card = bottom_cards(&theme, data, |_| ()).into_any_element();
                ctx.new(|_| HostedElement(card)).into_any_element()
            })
        };
        assert_rendered(draw(&sample_data()), "底部卡（有数据）");
        assert_rendered(
            draw(&BottomData {
                activity_title: None,
                user_name: None,
            }),
            "底部卡（未下发态）",
        );
    }

    /// ④ 活动横幅卡单独渲染非零（图占位 + 标题 + 提示 + CTA 都在树里）。
    #[test]
    fn activity_banner_renders_nonzero() {
        let theme = Theme::default();
        let data = sample_data();
        let bounds = draw_probe(|ctx| {
            let banner = activity_banner(&theme, &data, &|_| ()).into_any_element();
            ctx.new(|_| HostedElement(banner)).into_any_element()
        });
        assert_rendered(bounds, "活动横幅卡");
    }

    /// ④ 用户头像行单独渲染非零（圆头像占位 + 名字 + 两枚小钮都在树里）。
    #[test]
    fn user_row_renders_nonzero() {
        let theme = Theme::default();
        let data = sample_data();
        let bounds = draw_probe(|ctx| {
            let row = user_row(&theme, &data, &|_| ()).into_any_element();
            ctx.new(|_| HostedElement(row)).into_any_element()
        });
        assert_rendered(bounds, "用户头像行");
    }
}
