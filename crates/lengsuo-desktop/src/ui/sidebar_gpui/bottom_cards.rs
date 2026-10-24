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
                    .on_click(move |_event, _window, _app| {
                        cta(BottomCardIntent::OpenActivity)
                    }),
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
                .on_click(move |_event, _window, _app| {
                    notice(BottomCardIntent::OpenNotice)
                }),
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
                .on_click(move |_event, _window, _app| {
                    settings(BottomCardIntent::OpenSettings)
                }),
        )
}

#[cfg(test)]
mod tests {
    use super::{bottom_cards, BottomData, ACTIVITY_ACTION, ACTIVITY_LABEL};
    use gpui_kit::component::Theme;

    #[test]
    fn labels_verbatim() {
        assert_eq!(ACTIVITY_LABEL, "活动");
        assert_eq!(ACTIVITY_ACTION, "立即前往");
    }

    #[test]
    fn fail_closed_not_empty() {
        let theme = Theme::default();
        let d = BottomData {
            activity_title: None,
            user_name: None,
        };
        let _ = bottom_cards(&theme, &d, |_| ());
    }

    #[test]
    fn build_with_data() {
        let theme = Theme::default();
        let d = BottomData {
            activity_title: Some("1024 造物挑战".into()),
            user_name: Some("kim".into()),
        };
        let _ = bottom_cards(&theme, &d, |_| ());
    }
}
