//! 顶栏（波次 9，卡 `ui-topbar-menu-actions`，差异 D5）。
//!
//! 参照形态（桌面2实拍）：一行 —— 菜单词横排（间距 12~16px）+ 应用名 + 右侧
//! 「发现应用」描边圆角按钮。点击只上报 [`MenuIntent`] 意图枚举；
//! ⛔ **不实现任何菜单行为**（打开菜单 / 切换 Buddy 应用 / 跳转都属运行时侧，本仓没有）。
//!
//! fail-closed：应用名来自运行时数据，未下发就显示「未下发」，不伪造。
#![cfg(feature = "ui")]
use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// —— 逐字文案（文案全表命名空间 `buddyApp`；⛔ 不许自编中文）——

/// 菜单词（逐字，顺序照参照）。
pub const MENUS: [&str; 5] = ["文件", "编辑", "窗口", "帮助", "关于"];
/// 菜单词与意图的固定对齐（`MENUS[i] ⇔ MENU_INTENTS[i]`，测试守这条约）。
pub const MENU_INTENTS: [MenuIntent; 5] = [
    MenuIntent::File,
    MenuIntent::Edit,
    MenuIntent::Window,
    MenuIntent::Help,
    MenuIntent::About,
];
/// 发现应用（逐字：`buddyApp.switcher.discover`）。
pub const DISCOVER_APPS: &str = "发现应用";
/// 应用名未下发时的占位（fail-closed，不伪造）。
pub const NOT_DELIVERED: &str = "未下发";

// —— 切换 / 退出 / 预览调试的逐字文案（供后续行为卡取用，本卡不渲染）——

/// `buddyApp.deeplink.versionRequired`
pub const DEEPLINK_VERSION_REQUIRED: &str = "Buddy 应用仅 5.5.0 以上版本开放，请升级到最新版本";
/// `buddyApp.marketSwitch.dialog.content`（`{name}` 为应用名占位）
pub const SWITCH_DIALOG_CONTENT: &str = "将切换到「{name}」，确认是否前往？";
/// `buddyApp.marketSwitch.dialog.contentUnknown`
pub const SWITCH_DIALOG_CONTENT_UNKNOWN: &str = "将切换到会话所属的 Buddy 应用，确认是否前往？";
/// `buddyApp.marketSwitch.dialog.title`
pub const SWITCH_DIALOG_TITLE: &str = "确认切换 Buddy 应用";
/// `buddyApp.marketSwitch.exit.content`（`{name}` 为应用名占位）
pub const SWITCH_EXIT_CONTENT: &str = "将退出「{name}」并前往 WorkBuddy，确认继续？";
/// `buddyApp.marketSwitch.exit.contentUnknown`
pub const SWITCH_EXIT_CONTENT_UNKNOWN: &str = "将退出当前 Buddy 应用并前往 WorkBuddy，确认继续？";
/// `buddyApp.marketSwitch.exit.title`
pub const SWITCH_EXIT_TITLE: &str = "确认退出当前 Buddy 应用";
/// `buddyApp.marketSwitch.failed`
pub const SWITCH_FAILED: &str = "切换 Buddy 应用失败，请稍后重试";
/// `buddyApp.marketSwitch.generalName`
pub const SWITCH_GENERAL_NAME: &str = "通用版";
/// `buddyApp.marketSwitch.unknownName`
pub const SWITCH_UNKNOWN_NAME: &str = "该对话所属的 Buddy 应用";
/// `buddyApp.preview.banner`
pub const PREVIEW_BANNER: &str = "正在预览调试未发布配置";
/// `buddyApp.preview.bannerHint`
pub const PREVIEW_BANNER_HINT: &str = "点右侧退出，或重启客户端";
/// `buddyApp.preview.exit`
pub const PREVIEW_EXIT: &str = "退出预览调试";
/// `buddyApp.preview.openFailed`
pub const PREVIEW_OPEN_FAILED: &str = "预览调试打开失败，请稍后重试";

/// 顶栏点击意图（只上报枚举，⛔ 不含行为实现）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuIntent {
    /// 「文件」
    File,
    /// 「编辑」
    Edit,
    /// 「窗口」
    Window,
    /// 「帮助」
    Help,
    /// 「关于」
    About,
    /// 「发现应用」按钮（逐字：`buddyApp.switcher.discover`）。
    DiscoverApps,
}

impl MenuIntent {
    /// 意图的可读标签（日志记账用，⛔ 不参与业务判断）。
    pub fn label(self) -> &'static str {
        match self {
            MenuIntent::File => MENUS[0],
            MenuIntent::Edit => MENUS[1],
            MenuIntent::Window => MENUS[2],
            MenuIntent::Help => MENUS[3],
            MenuIntent::About => MENUS[4],
            MenuIntent::DiscoverApps => DISCOVER_APPS,
        }
    }
}

/// 菜单词 → 意图（fail-closed：不认识的词返回 `None`，不上报伪造意图）。
pub fn intent_from_word(word: &str) -> Option<MenuIntent> {
    MENUS
        .iter()
        .position(|m| *m == word)
        .map(|i| MENU_INTENTS[i])
}

/// 应用名（fail-closed：运行时未下发 ⇒ 「未下发」，不伪造）。
pub fn app_name(delivered: Option<&str>) -> &str {
    delivered.unwrap_or(NOT_DELIVERED)
}

/// 渲染顶栏（无接线记账版：意图只进日志，与 `stop_intent.rs` 同款记账）。
pub fn title_bar(theme: &Theme) -> Div {
    title_bar_with(theme, None, |intent| {
        eprintln!(
            "[lengsuo-desktop] 顶栏意图（未执行，本仓无运行时）：{}",
            intent.label()
        );
    })
}

/// 渲染顶栏：`app` 未下发显示「未下发」；点击经 `on_intent` 只上报意图枚举。
///
/// 颜色一律 `theme.*` token（⛔ 硬编码色值）；布局 gpui flex。
pub fn title_bar_with(
    theme: &Theme,
    app: Option<&str>,
    on_intent: impl Fn(MenuIntent) + 'static,
) -> Div {
    let on_intent = Rc::new(on_intent) as Rc<dyn Fn(MenuIntent)>;
    let mut menus = div().flex().items_center().gap_3();
    for idx in 0..MENUS.len() {
        menus = menus.child(menu_word(theme, idx, Rc::clone(&on_intent)));
    }
    div()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .py_2()
        .bg(theme.title_bar)
        .border_b_1()
        .border_color(theme.title_bar_border)
        .child(
            div().flex().items_center().gap_4().child(menus).child(
                div()
                    .text_sm()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(SharedString::from(app_name(app).to_string())),
            ),
        )
        .child(discover_button(theme, on_intent))
}

/// 单个菜单词：可点，点击上报对应意图（`MENUS[idx] ⇔ MENU_INTENTS[idx]`）。
fn menu_word(theme: &Theme, idx: usize, on_intent: Rc<dyn Fn(MenuIntent)>) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("topbar-menu-{idx}")))
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(MENUS[idx]))
        .on_click(move |_, _, _| on_intent(MENU_INTENTS[idx]))
}

/// 「发现应用」：描边圆角按钮（逐字：`buddyApp.switcher.discover`）。
fn discover_button(theme: &Theme, on_intent: Rc<dyn Fn(MenuIntent)>) -> impl IntoElement {
    div()
        .id("topbar-discover")
        .flex()
        .items_center()
        .h_7()
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.foreground)
        .child(SharedString::from(DISCOVER_APPS.to_string()))
        .on_click(move |_, _, _| on_intent(MenuIntent::DiscoverApps))
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
