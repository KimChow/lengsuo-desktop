//! 助理团队升级横幅（波次 8，卡 `ui-overlay-cloud-upgrade`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 对齐 docs/ui-parity-spec.md §11：限时体验/进入 + 开启失败/加载态。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；界面文案逐字取自命名空间
//! `cloudAssistantUpgrade`（14 键全量）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ============ 纯数据层（无 GPUI 依赖） ============

/// 视图标题（浮层通名）。
pub const TITLE: &str = "助理团队升级横幅";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

// —— `cloudAssistantUpgrade.banner.*`（常驻横幅面，4 键）——
/// 横幅主标题（`banner.title`）。
pub const BANNER_TITLE: &str = "组建你的专属助理团队";
/// 横幅副标题（`banner.subtitle`）。
pub const BANNER_SUBTITLE: &str = "7 x 24小时云端待命 · 专属记忆 · 一句话分派任务";
/// 限时体验入口（`banner.action`）。
pub const BANNER_ACTION_TRIAL: &str = "限时体验";
/// 进入入口（`banner.enter`）。
pub const BANNER_ACTION_ENTER: &str = "进入";

// —— `cloudAssistantUpgrade.openTeam*`（开启动作状态行，3 键）——
/// 正在开启（`openTeamLoading`）。
pub const OPEN_TEAM_LOADING: &str = "正在为你开启助理团队…";
/// 开启失败（`openTeamError`）。
pub const OPEN_TEAM_ERROR: &str = "开启助理团队失败，请稍后重试。";
/// 页面跳转失败（`openTeamNavigationError`）。
pub const OPEN_TEAM_NAVIGATION_ERROR: &str = "页面跳转失败，请刷新后重试。";

// —— `cloudAssistantUpgrade.trial.*`（限时体验面板面，7 键）——
/// 体验面板标题前段（`trial.titleLead`）。
pub const TRIAL_TITLE_LEAD: &str = "把需求交给";
/// 体验面板标题强调段（`trial.titleHighlight`）。
pub const TRIAL_TITLE_HIGHLIGHT: &str = "助理";
/// 体验面板标题后段（`trial.titleTail`）。
pub const TRIAL_TITLE_TAIL: &str = "团队，即刻开始协作";
/// 体验面板副标题（`trial.subtitle`）。
pub const TRIAL_SUBTITLE: &str = "7 × 24 小时在线干活，多任务并行处理，和助理一起高效协作";

/// 体验面板特性项（文案表键名 + 逐字文案）。
pub struct TrialFeature {
    /// 文案表键名。
    pub key: &'static str,
    /// 逐字文案。
    pub label: &'static str,
}

/// 特性全表（顺序即渲染顺序，同文案表）。
pub const TRIAL_FEATURES: &[TrialFeature] = &[
    TrialFeature {
        key: "cloudAssistantUpgrade.trial.feature.standby",
        label: "7 x 24 小时待命",
    },
    TrialFeature {
        key: "cloudAssistantUpgrade.trial.feature.memory",
        label: "专属记忆",
    },
    TrialFeature {
        key: "cloudAssistantUpgrade.trial.feature.delegate",
        label: "一句话分派",
    },
];

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    /// 拉取中（数据未到手，按未下发渲染）。
    Loading,
    /// 拉取失败（数据未到手，按未下发渲染）。
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 开启助理团队动作的异步阶段（状态行文案来源，`openTeam*` 3 键）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenTeamPhase {
    /// 正在开启（`openTeamLoading`）。
    Loading,
    /// 开启失败（`openTeamError`）。
    Failed,
    /// 页面跳转失败（`openTeamNavigationError`）。
    NavigationFailed,
}

/// 服务端下发的横幅数据（⛔ 无数据源时不得伪造）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delivery {
    /// 展示限时体验面板（`trial.*`）；否则常驻横幅（`banner.*`）。
    pub trial: bool,
}

/// 渲染面（纯函数输出；空态优先，fail-closed）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    /// 常驻横幅（`banner.*`）。
    Banner,
    /// 限时体验面板（`trial.*`）。
    Trial,
    /// 数据未下发 ⇒ 空态「未下发」。
    NotDelivered,
}

/// 助理团队升级横幅模型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloudUpgradeBanner {
    /// 下发状态。
    pub state: State,
    /// 横幅数据（未下发时为 `None`）。
    pub delivery: Option<Delivery>,
    /// 开启动作阶段（未触发时为 `None`）。
    pub open_team: Option<OpenTeamPhase>,
}

impl CloudUpgradeBanner {
    /// 无数据源的诚实默认（fail-closed）。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            state: State::NotDelivered,
            delivery: None,
            open_team: None,
        }
    }

    /// 由下发结果构造：空输入 ⇒ `NotDelivered` 空态（⛔ 不是空清单）。
    #[must_use]
    pub fn from_delivery(delivery: Option<Delivery>) -> Self {
        match delivery {
            Some(delivery) => Self {
                state: State::Normal,
                delivery: Some(delivery),
                open_team: None,
            },
            None => Self::not_delivered(),
        }
    }

    /// 是否处于未下发空态（拉取中/失败 ⇒ 数据未到手，同样按未下发渲染）。
    #[must_use]
    pub fn is_not_delivered(&self) -> bool {
        self.delivery.is_none() || self.state != State::Normal
    }

    /// 当前应渲染的面（空态优先）。
    #[must_use]
    pub fn surface_kind(&self) -> SurfaceKind {
        if self.is_not_delivered() {
            SurfaceKind::NotDelivered
        } else if matches!(self.delivery, Some(delivery) if delivery.trial) {
            SurfaceKind::Trial
        } else {
            SurfaceKind::Banner
        }
    }

    /// 开启动作状态行文案（未触发 ⇒ `None`）。
    #[must_use]
    pub fn open_team_hint(&self) -> Option<&'static str> {
        match self.open_team {
            Some(OpenTeamPhase::Loading) => Some(OPEN_TEAM_LOADING),
            Some(OpenTeamPhase::Failed) => Some(OPEN_TEAM_ERROR),
            Some(OpenTeamPhase::NavigationFailed) => Some(OPEN_TEAM_NAVIGATION_ERROR),
            None => None,
        }
    }
}

// ============ GPUI 渲染层 ============

/// 浮层渲染（浮层不占主区，返回整层容器由调用方叠加）。
///
/// 当前无运行时数据源 ⇒ 恒为 [`CloudUpgradeBanner::not_delivered`] 的诚实渲染。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(&CloudUpgradeBanner::not_delivered(), theme)
}

/// 按纯数据层渲染整层：标题行（含开启状态行）+ 面内容。
fn render(model: &CloudUpgradeBanner, theme: &Theme) -> Div {
    div()
        .flex_1()
        .h_full()
        .flex()
        .flex_col()
        .child(render_header(model, theme))
        .child(render_surface(model, theme))
}

/// 标题行 + 开启动作状态行。
fn render_header(model: &CloudUpgradeBanner, theme: &Theme) -> Div {
    let mut header = div()
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
        );
    if let Some(hint) = model.open_team_hint() {
        header = header.child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(hint),
        );
    }
    header
}

/// 面内容分发：横幅 / 体验面板 / 空态「未下发」（fail-closed）。
fn render_surface(model: &CloudUpgradeBanner, theme: &Theme) -> Div {
    let surface = div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .py_3();
    match model.surface_kind() {
        SurfaceKind::Banner => render_banner(surface, theme),
        SurfaceKind::Trial => render_trial(surface, theme),
        // fail-closed：数据未到手时如实说明，不渲染任何条目。
        SurfaceKind::NotDelivered => surface.child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .py_4()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        ),
    }
}

/// 常驻横幅面：主标题 + 副标题 + 双入口（限时体验/进入）。
fn render_banner(surface: Div, theme: &Theme) -> Div {
    surface
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(BANNER_TITLE),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(BANNER_SUBTITLE),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_md()
                        .border_1()
                        .border_color(theme.accent)
                        .text_sm()
                        .text_color(theme.accent)
                        .child(BANNER_ACTION_TRIAL),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .text_sm()
                        .text_color(theme.accent)
                        .child(BANNER_ACTION_ENTER),
                ),
        )
}

/// 限时体验面板面：三段标题 + 副标题 + 特性行。
fn render_trial(surface: Div, theme: &Theme) -> Div {
    let panel = surface
        .child(
            div()
                .flex()
                .items_center()
                .flex_wrap()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(TRIAL_TITLE_LEAD),
                )
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.accent)
                        .child(TRIAL_TITLE_HIGHLIGHT),
                )
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(TRIAL_TITLE_TAIL),
                ),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(TRIAL_SUBTITLE),
        );
    let mut features = div().flex().items_center().flex_wrap().gap_4();
    for feature in TRIAL_FEATURES {
        features = features.child(
            div()
                .px_3()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(feature.label),
        );
    }
    panel.child(features)
}
#[cfg(test)]
mod tests {
    use super::{main_area, CloudUpgradeBanner, State, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(CloudUpgradeBanner::not_delivered().state, State::NotDelivered);
    }
    #[test]
    fn title_not_empty() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
