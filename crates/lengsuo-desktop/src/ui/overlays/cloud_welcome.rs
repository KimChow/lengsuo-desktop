//! 云端欢迎浮层（波次 7，卡 `ui-overlay-cloud-welcome`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 对齐 docs/ui-parity-spec.md §9 A51：登录/导航链接 + 活动入口。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `cloudWelcome` /
//! `welcome`（仅无品牌词键）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ============ 纯数据层（无 GPUI 依赖） ============

/// 视图标题（浮层通名）。
pub const TITLE: &str = "云端欢迎";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 登录入口文案（`cloudWelcome.login`）。
pub const LOGIN_LABEL: &str = "登录";
/// 成长计划入口文案（`welcome.growthPlan.entry`）。
pub const GROWTH_PLAN_ENTRY: &str = "做任务赢积分好礼";
/// 活动区标题文案（`welcome.activity.label`）。
pub const ACTIVITY_LABEL: &str = "活动";
/// 活动区动作按钮文案（`welcome.activity.action`）。
pub const ACTIVITY_ACTION: &str = "立即前往";
/// 活动通知入口文案（`welcome.activity.notice`）。
pub const ACTIVITY_NOTICE: &str = "通知";
/// 已切换活动模型提示（`welcome.activity.modelSwitched`）。
pub const ACTIVITY_MODEL_SWITCHED: &str = "已为你切换到活动指定模型";

/// 浮层数据下发状态（可枚举全生命周期）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryState {
    /// 导航与活动入口均已下发。
    Normal,
    /// 拉取中（数据未到手，按未下发渲染）。
    Loading,
    /// 拉取失败（数据未到手，按未下发渲染）。
    Failed,
    /// 未下发（无数据源时的诚实默认）。
    NotDelivered,
}

/// 顶部导航链接（静态界面文案，逐字取自 `cloudWelcome.nav.*`）。
pub struct NavLink {
    /// 文案表键名。
    pub key: &'static str,
    /// 逐字文案。
    pub label: &'static str,
}

/// 顶部导航链接全表（顺序即渲染顺序，同文案表）。
pub const NAV_LINKS: &[NavLink] = &[
    NavLink {
        key: "cloudWelcome.nav.apiDocs",
        label: "API文档",
    },
    NavLink {
        key: "cloudWelcome.nav.blog",
        label: "博客",
    },
    NavLink {
        key: "cloudWelcome.nav.cli",
        label: "CLI",
    },
    NavLink {
        key: "cloudWelcome.nav.docs",
        label: "文档",
    },
    NavLink {
        key: "cloudWelcome.nav.ide",
        label: "IDE",
    },
    NavLink {
        key: "cloudWelcome.nav.plugins",
        label: "插件",
    },
    NavLink {
        key: "cloudWelcome.nav.pricing",
        label: "定价",
    },
    NavLink {
        key: "cloudWelcome.nav.promotion",
        label: "活动",
    },
];

/// 活动入口（服务端下发后才存在；⛔ 无源时不得伪造）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Activity {
    /// 是否有活动通知。
    pub has_notice: bool,
    /// 是否已切换到活动指定模型。
    pub model_switched: bool,
}

/// 云端欢迎浮层模型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloudWelcome {
    /// 下发状态。
    pub state: DeliveryState,
    /// 活动入口数据（未下发时为 `None`）。
    pub activity: Option<Activity>,
}

impl CloudWelcome {
    /// 无数据源的诚实默认（fail-closed）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            state: DeliveryState::NotDelivered,
            activity: None,
        }
    }

    /// 由下发结果构造：空输入 ⇒ `NotDelivered` 空态（⛔ 不是空清单）。
    #[must_use]
    pub fn from_delivery(activity: Option<Activity>) -> Self {
        match activity {
            Some(activity) => Self {
                state: DeliveryState::Normal,
                activity: Some(activity),
            },
            None => Self::not_delivered(),
        }
    }

    /// 是否处于未下发空态。
    #[must_use]
    pub fn is_not_delivered(&self) -> bool {
        self.state == DeliveryState::NotDelivered
    }

    /// 活动入口文案（未下发 ⇒ 无入口，渲染层显示「未下发」）。
    #[must_use]
    pub fn activity_entry_label(&self) -> Option<&'static str> {
        self.activity.map(|_| GROWTH_PLAN_ENTRY)
    }

    /// 活动通知文案（无通知或未下发 ⇒ `None`）。
    #[must_use]
    pub fn activity_notice_label(&self) -> Option<&'static str> {
        self.activity
            .filter(|activity| activity.has_notice)
            .map(|_| ACTIVITY_NOTICE)
    }

    /// 已切换模型提示（未切换或未下发 ⇒ `None`）。
    #[must_use]
    pub fn model_switched_hint(&self) -> Option<&'static str> {
        self.activity
            .filter(|activity| activity.model_switched)
            .map(|_| ACTIVITY_MODEL_SWITCHED)
    }
}

// ============ GPUI 渲染层 ============

/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
///
/// 当前无运行时数据源 ⇒ 恒为 [`CloudWelcome::not_delivered`] 的诚实渲染。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&CloudWelcome::not_delivered(), theme)
}

/// 按纯数据层渲染整层：标题行 + 导航行 + 活动区。
fn render(model: &CloudWelcome, theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(render_header(theme))
        .child(render_nav(theme))
        .child(render_activity(model, theme))
}

/// 标题行 + 登录入口。
fn render_header(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .px_3()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(theme.accent)
                .text_sm()
                .text_color(theme.accent)
                .child(LOGIN_LABEL),
        )
}

/// 导航链接行（静态界面文案，非服务端数据）。
fn render_nav(theme: &Theme) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap_4()
        .px_4()
        .py_2();
    for link in NAV_LINKS {
        row = row.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(link.label),
        );
    }
    row
}

/// 活动区（未下发 ⇒ 空态「未下发」，⛔ 不伪造条目/数值）。
fn render_activity(model: &CloudWelcome, theme: &Theme) -> Div {
    let mut section = div()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .py_3()
        .border_t_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(ACTIVITY_LABEL),
        );
    if model.is_not_delivered() {
        // fail-closed：数据未到手时如实说明，不渲染任何活动条目。
        return section.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .py_4()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        );
    }
    if let Some(entry) = model.activity_entry_label() {
        section = section.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(div().text_sm().text_color(theme.accent).child(entry))
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.accent)
                        .text_sm()
                        .text_color(theme.accent)
                        .child(ACTIVITY_ACTION),
                ),
        );
    }
    if let Some(notice) = model.activity_notice_label() {
        section = section.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(notice),
        );
    }
    if let Some(hint) = model.model_switched_hint() {
        section = section.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(hint),
        );
    }
    section
}
#[cfg(test)]
mod tests {
    use super::{
        main_area, Activity, CloudWelcome, DeliveryState, ACTIVITY_ACTION, ACTIVITY_LABEL,
        ACTIVITY_MODEL_SWITCHED, ACTIVITY_NOTICE, GROWTH_PLAN_ENTRY, LOGIN_LABEL, NAV_LINKS,
        NOT_DELIVERED, TITLE,
    };
    use gpui_kit::component::Theme;
    use std::collections::HashSet;

    #[test]
    fn states_are_enumerable_and_distinct() {
        let states = [
            DeliveryState::Normal,
            DeliveryState::Loading,
            DeliveryState::Failed,
            DeliveryState::NotDelivered,
        ];
        assert_eq!(states.len(), 4);
        let distinct: HashSet<String> = states.iter().map(|state| format!("{state:?}")).collect();
        assert_eq!(distinct.len(), states.len());
    }

    #[test]
    fn nav_links_count_and_keys_are_unique() {
        assert_eq!(NAV_LINKS.len(), 8);
        let keys: HashSet<&str> = NAV_LINKS.iter().map(|link| link.key).collect();
        assert_eq!(keys.len(), NAV_LINKS.len());
    }

    #[test]
    fn copy_is_verbatim_from_key_table() {
        assert_eq!(LOGIN_LABEL, "登录");
        assert_eq!(GROWTH_PLAN_ENTRY, "做任务赢积分好礼");
        assert_eq!(ACTIVITY_LABEL, "活动");
        assert_eq!(ACTIVITY_ACTION, "立即前往");
        assert_eq!(ACTIVITY_NOTICE, "通知");
        assert_eq!(ACTIVITY_MODEL_SWITCHED, "已为你切换到活动指定模型");
        assert_eq!(NOT_DELIVERED, "未下发");
        let labels: Vec<&str> = NAV_LINKS.iter().map(|link| link.label).collect();
        for expected in [
            "API文档",
            "博客",
            "CLI",
            "文档",
            "IDE",
            "插件",
            "定价",
            "活动",
        ] {
            assert!(labels.contains(&expected), "缺导航文案: {expected}");
        }
    }

    #[test]
    fn delivered_activity_surfaces_entry_and_notice() {
        let model = CloudWelcome::from_delivery(Some(Activity {
            has_notice: true,
            model_switched: true,
        }));
        assert_eq!(model.state, DeliveryState::Normal);
        assert!(!model.is_not_delivered());
        assert_eq!(model.activity_entry_label(), Some(GROWTH_PLAN_ENTRY));
        assert_eq!(model.activity_notice_label(), Some(ACTIVITY_NOTICE));
        assert_eq!(model.model_switched_hint(), Some(ACTIVITY_MODEL_SWITCHED));

        let quiet = CloudWelcome::from_delivery(Some(Activity {
            has_notice: false,
            model_switched: false,
        }));
        assert_eq!(quiet.state, DeliveryState::Normal);
        assert_eq!(quiet.activity_notice_label(), None);
        assert_eq!(quiet.model_switched_hint(), None);
    }

    #[test]
    fn empty_delivery_yields_not_delivered_empty_state() {
        let model = CloudWelcome::from_delivery(None);
        assert_eq!(model.state, DeliveryState::NotDelivered);
        assert!(model.is_not_delivered());
        assert_eq!(model.activity_entry_label(), None);
        assert_eq!(model.activity_notice_label(), None);
        assert_eq!(model.model_switched_hint(), None);
    }

    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert!(!TITLE.is_empty());
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
