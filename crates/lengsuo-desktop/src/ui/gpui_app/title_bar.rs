//! 顶栏（波次 9，卡 `ui-topbar-menu`，差异 D5）。
//!
//! 参照形态：系统菜单栏（文件/编辑/窗口/帮助/关于）+ 应用标识 + 「发现应用」按钮
//! （逐字：`buddyApp.switcher.discover`）。点击只上报菜单意图，不实现菜单行为。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 菜单项（逐字，顺序照参照）。
pub const MENUS: [&str; 5] = ["文件", "编辑", "窗口", "帮助", "关于"];
/// 发现应用（逐字：`buddyApp.switcher.discover`）。
pub const DISCOVER_APPS: &str = "发现应用";
/// 应用标识（中性代号）。
pub const APP_NAME: &str = "冷锁桌面端";

/// 渲染顶栏（纯构建）。
pub fn title_bar(theme: &Theme) -> Div {
    let _ = theme;
    let mut bar = div().flex().gap_3();
    for m in MENUS {
        bar = bar.child(SharedString::from(m.to_string()));
    }
    bar.child(APP_NAME).child(DISCOVER_APPS)
}

#[cfg(test)]
mod tests {
    use super::{title_bar, DISCOVER_APPS, MENUS};
    use gpui_kit::component::Theme;

    #[test]
    fn menus_verbatim() {
        assert_eq!(MENUS.len(), 5);
        assert_eq!(MENUS[0], "文件");
        assert_eq!(MENUS[4], "关于");
    }

    #[test]
    fn discover_verbatim() {
        assert_eq!(DISCOVER_APPS, "发现应用");
    }

    #[test]
    fn build_not_panic() {
        let _ = title_bar(&Theme::default());
    }
}
