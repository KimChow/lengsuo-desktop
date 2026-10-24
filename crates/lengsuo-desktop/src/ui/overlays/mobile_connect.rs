//! 连接移动端浮层（波次 8，卡 `ui-overlay-mobile-connect`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `mobileConnect`。
#![cfg(feature = "ui")]

// ── 纯数据层（无 GPUI 依赖）────────────────────────────────────────────

/// 命名空间全键（8 键，逐字对应界面文案全表）。
pub const KEYS: [&str; 8] = [
    "mobileConnect.title",
    "mobileConnect.tab.app",
    "mobileConnect.tab.miniprogram",
    "mobileConnect.qr.app.desc",
    "mobileConnect.qr.miniprogram.desc",
    "mobileConnect.switch.allowConnect",
    "mobileConnect.switch.preventSleep",
    "mobileConnect.empty",
];

/// 标题（`mobileConnect.title`）。
pub const TITLE: &str = "连接移动端";
/// Tab 文案（`mobileConnect.tab.app`）。
pub const TAB_APP: &str = "App";
/// Tab 文案（`mobileConnect.tab.miniprogram`）。
pub const TAB_MINIPROGRAM: &str = "小程序";
/// App Tab 二维码说明（`mobileConnect.qr.app.desc`）。
pub const QR_APP_DESC: &str = "使用 App 连接电脑";
/// 小程序 Tab 二维码说明（`mobileConnect.qr.miniprogram.desc`）。
pub const QR_MINIPROGRAM_DESC: &str = "使用小程序连接电脑";
/// 开关文案（`mobileConnect.switch.allowConnect`）。
pub const SWITCH_ALLOW_CONNECT: &str = "允许移动端连接此设备";
/// 开关文案（`mobileConnect.switch.preventSleep`）。
pub const SWITCH_PREVENT_SLEEP: &str = "保持唤醒状态，以便远程访问";
/// 空态文案（`mobileConnect.empty`）：已下发但无可用连接方式（⛔ 不是空列表）。
pub const EMPTY: &str = "暂无可用的移动端连接方式";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 双 Tab：App / 小程序。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectTab {
    #[default]
    App,
    Miniprogram,
}

impl ConnectTab {
    /// Tab 文案（逐字）。
    pub fn label(self) -> &'static str {
        match self {
            Self::App => TAB_APP,
            Self::Miniprogram => TAB_MINIPROGRAM,
        }
    }

    /// 当前 Tab 的二维码说明（逐字）。
    pub fn qr_desc(self) -> &'static str {
        match self {
            Self::App => QR_APP_DESC,
            Self::Miniprogram => QR_MINIPROGRAM_DESC,
        }
    }
}

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用（Tab + 二维码说明 + 开关）。
    Normal,
    Loading,
    Failed,
    /// 已下发但无可用连接方式 ⇒ 空态文案。
    Empty,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 非正常态的正文文案（fail-closed：拿不到数据 ⇒ 如实说明，⛔ 不伪造条目）。
/// `Normal` ⇒ `None`（渲染真实内容）；`Loading`/`Failed` 视同未下发。
pub fn body_text(state: State) -> Option<&'static str> {
    match state {
        State::Normal => None,
        State::Loading | State::Failed | State::NotDelivered => Some(NOT_DELIVERED),
        State::Empty => Some(EMPTY),
    }
}

/// 浮层数据模型（纯数据层；开关值只有已下发才可信）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MobileConnect {
    pub state: State,
    pub tab: ConnectTab,
    /// 「允许移动端连接此设备」开关值；未下发 ⇒ false（fail-closed）。
    pub allow_connect: bool,
    /// 「保持唤醒状态，以便远程访问」开关值；未下发 ⇒ false。
    pub prevent_sleep: bool,
}

impl MobileConnect {
    /// 开关行（文案, 值）×2；仅 `Normal` 态下发，否则 `None`（⛔ 不伪造开关值）。
    pub fn switch_rows(&self) -> Option<[(&'static str, bool); 2]> {
        match self.state {
            State::Normal => Some([
                (SWITCH_ALLOW_CONNECT, self.allow_connect),
                (SWITCH_PREVENT_SLEEP, self.prevent_sleep),
            ]),
            _ => None,
        }
    }
}

// ── GPUI 层 ────────────────────────────────────────────────────────────
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 主区渲染（浮层不占主区，保持签名；默认模型 = 未下发 fail-closed 分支）。
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(TITLE))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}
#[cfg(test)]
mod tests {
    use super::{main_area, State, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
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
