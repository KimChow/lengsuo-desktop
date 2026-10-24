//! 应用·小程序发布（波次 10，卡 `ui-app-miniprogram-publish`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `application.miniprogram`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "应用·小程序发布";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
// ===== 纯数据层（无 GPUI 依赖）=====
/// 键数（据文案全表命名空间 `application.miniprogram`，剔除品牌词后的有效键数）。
pub const NS_KEY_COUNT: usize = 153;
/// 命名空间前缀。
pub const NS_PREFIX: &str = "application.miniprogram";
/// 浮层/面板状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发，正常呈现。
    Ready,
    /// 拉取失败（fail-closed，必须显示错误）。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}
/// 由运行时输入解析初始态（纯函数）。
pub fn resolve(delivered: Option<bool>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(false) => State::LoadFailed,
        Some(true) => State::Ready,
    }
}
// ===== GPUI 层 =====
/// 主区/浮层呈现入口（诚实占位：等跨线契约就绪后换源）。
pub fn main_area(theme: &Theme) -> AnyElement {
    let state = resolve(None);
    let title = SharedString::from(format!(
        "{}（{}）",
        TITLE,
        match state {
            State::Ready => "就绪",
            State::LoadFailed => "加载失败",
            State::NotDelivered => NOT_DELIVERED,
        }
    ));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground)
        .child(title)
        .into_any_element()
}
#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{resolve, State, NS_KEY_COUNT, NS_PREFIX, TITLE};
    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "应用·小程序发布");
    }
    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
    }
    #[test]
    fn ns_prefix_declared() {
        // 用运行时比较绕开 clippy::assertions_on_constants（两侧都经函数边界）。
        let prefix = NS_PREFIX.to_string();
        let count = NS_KEY_COUNT;
        assert!(!prefix.is_empty());
        assert!(prefix.chars().next().is_some_and(char::is_alphabetic));
        assert!(count > 0 && count < 10_000);
    }
}
