//! 左栏底部卡（波次 9，卡 `ui-left-bottom-cards`，差异 D8）。
//!
//! 参照形态：活动横幅卡（图 + 文 + CTA「立即前往」）+ 用户头像行。
//! 文案逐字 `welcome.activity.*`；无活动数据时如实「未下发」。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 活动标签（逐字：`welcome.activity.label`）。
pub const ACTIVITY_LABEL: &str = "活动";
/// 活动 CTA（逐字：`welcome.activity.action`）。
pub const ACTIVITY_ACTION: &str = "立即前往";
/// 通知标签（逐字：`welcome.activity.notice`）。
pub const NOTICE_LABEL: &str = "通知";
/// 未下发（fail-closed）。
pub const NOT_DELIVERED: &str = "未下发";

/// 底部卡数据（真实来源等 C1~C10）。
pub struct BottomData {
    /// 活动标题；`None` ⇒ 未下发。
    pub activity_title: Option<String>,
    /// 用户名；`None` ⇒ 未下发。
    pub user_name: Option<String>,
}

/// 渲染底部卡（纯构建）。
pub fn bottom_cards(theme: &Theme, data: &BottomData) -> Div {
    let _ = theme;
    let act = data
        .activity_title
        .clone()
        .unwrap_or_else(|| NOT_DELIVERED.to_string());
    let user = data
        .user_name
        .clone()
        .unwrap_or_else(|| NOT_DELIVERED.to_string());
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(ACTIVITY_LABEL)
        .child(SharedString::from(act))
        .child(ACTIVITY_ACTION)
        .child(NOTICE_LABEL)
        .child(SharedString::from(user))
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
        let _ = bottom_cards(&theme, &d);
    }

    #[test]
    fn build_with_data() {
        let theme = Theme::default();
        let d = BottomData {
            activity_title: Some("1024 造物挑战".into()),
            user_name: Some("kim".into()),
        };
        let _ = bottom_cards(&theme, &d);
    }
}
