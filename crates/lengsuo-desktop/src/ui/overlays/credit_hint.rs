//! 云服务额度不足提示（波次 7，卡 `ui-overlay-credit-hint`，条目 A55）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `conversation.cloudServiceCreditsExhausted` /
//! `application.cloudServiceTag`。
//! 数据源在运行时侧：本模块当前是**诚实占位**，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "云服务额度";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

// ===================== 纯数据层（无 GPUI 依赖） =====================

/// 分支文案模板：单应用（占位符 `{app1}` 渲染时替换）。
pub const TEXT_ONE: &str = "「{app1}」应用云服务额度不足";
/// 分支文案模板：双应用（占位符 `{app1}`/`{app2}` 渲染时替换）。
pub const TEXT_TWO: &str = "「{app1}」「{app2}」应用云服务额度不足";
/// 分支文案模板：多应用（≥3，取前两个应用名示例）。
pub const TEXT_MANY: &str = "「{app1}」「{app2}」等应用云服务额度不足";
/// 「去升级」入口文案。
pub const TEXT_UPGRADE: &str = "去升级";
/// 辅助说明行（同族额度提示文案，逐字）。
pub const TEXT_BANNER: &str = "云服务额度不足，可升级套餐或删除闲置的云服务应用后重新开启";
/// 加载态文案（逐字取自文案全表）。
pub const TEXT_LOADING: &str = "加载中...";
/// 空态文案（Normal 且无额度不足应用；逐字取自文案全表）。
pub const TEXT_EMPTY: &str = "暂无内容";
/// 状态标签文案（命名空间 `application.cloudServiceTag`，4 键全量）。
pub const TAG_EXPIRING: &str = "额度即将耗尽";
pub const TAG_PROVISIONING: &str = "开通中";
pub const TAG_CREDITS_EXHAUSTED: &str = "暂停";
pub const TAG_DESTROYED: &str = "已销毁";

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CreditHintState {
    /// 已下发且存在额度不足的应用。
    Normal,
    Loading,
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 单个应用的云服务状态标签。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudServiceTag {
    /// 额度即将耗尽。
    Expiring,
    /// 开通中。
    Provisioning,
    /// 额度耗尽已暂停。
    CreditsExhausted,
    /// 已销毁。
    Destroyed,
}

impl CloudServiceTag {
    /// 状态标签文案（逐字）。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Expiring => TAG_EXPIRING,
            Self::Provisioning => TAG_PROVISIONING,
            Self::CreditsExhausted => TAG_CREDITS_EXHAUSTED,
            Self::Destroyed => TAG_DESTROYED,
        }
    }
}

/// 额度不足的应用条目（名称 + 状态标签）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreditApp {
    pub name: String,
    pub tag: CloudServiceTag,
}

/// 浮层数据模型：状态 + 额度不足应用列表。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CreditHint {
    pub state: CreditHintState,
    pub apps: Vec<CreditApp>,
}

impl CreditHint {
    /// fail-closed 构造：数据未下发。
    #[must_use]
    pub fn not_delivered() -> Self {
        Self {
            state: CreditHintState::NotDelivered,
            apps: Vec::new(),
        }
    }

    /// 空输入 ⇒ 空态（Normal 且无应用），⛔ 不是空列表渲染。
    #[must_use]
    pub fn is_empty_state(&self) -> bool {
        self.state == CreditHintState::Normal && self.apps.is_empty()
    }

    /// 按应用数量选分支模板：1 → one，2 → two，≥3 → many；0 → 无（空态）。
    #[must_use]
    pub fn message_template_for(len: usize) -> Option<&'static str> {
        match len {
            1 => Some(TEXT_ONE),
            2 => Some(TEXT_TWO),
            n if n >= 3 => Some(TEXT_MANY),
            _ => None,
        }
    }

    /// 占位符替换：槽位无数据时如实填「未下发」。
    #[must_use]
    pub fn fill_template(template: &str, app1: Option<&str>, app2: Option<&str>) -> String {
        template
            .replace("{app1}", app1.unwrap_or(NOT_DELIVERED))
            .replace("{app2}", app2.unwrap_or(NOT_DELIVERED))
    }

    /// 当前模型的分支文案（已替换占位符；无应用时 fail-closed 显示「未下发」）。
    #[must_use]
    pub fn rendered_message(&self) -> String {
        match Self::message_template_for(self.apps.len()) {
            Some(template) => Self::fill_template(
                template,
                self.apps.first().map(|app| app.name.as_str()),
                self.apps.get(1).map(|app| app.name.as_str()),
            ),
            None => NOT_DELIVERED.to_string(),
        }
    }

    /// 状态标签行数据：按首次出现顺序去重。
    #[must_use]
    pub fn distinct_tags(&self) -> Vec<CloudServiceTag> {
        let mut tags = Vec::new();
        for app in &self.apps {
            if !tags.contains(&app.tag) {
                tags.push(app.tag);
            }
        }
        tags
    }
}
/// 浮层渲染（波次 7：浮层不占主区，返回整层容器由调用方叠加）。
/// 数据源在运行时侧：`main_area` 以 fail-closed 的「未下发」模型渲染。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &CreditHint::not_delivered())
}

/// 按数据模型渲染整层：标题行 + 状态分支主体。
#[must_use]
pub fn render(theme: &Theme, hint: &CreditHint) -> Div {
    let body = match hint.state {
        CreditHintState::Loading => centered_line(theme, TEXT_LOADING),
        CreditHintState::Failed | CreditHintState::NotDelivered => {
            centered_line(theme, NOT_DELIVERED)
        }
        CreditHintState::Normal if hint.is_empty_state() => centered_line(theme, TEXT_EMPTY),
        CreditHintState::Normal => hint_body(theme, hint),
    };
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(body)
}

/// 居中单行（加载 / 空态 / 未下发共用）。
fn centered_line(theme: &Theme, text: &str) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text.to_string())
}

/// 内容主体：分支文案 + 辅助说明 + 「去升级」+ 状态标签行。
fn hint_body(theme: &Theme, hint: &CreditHint) -> Div {
    let mut tags = div().flex().flex_wrap().gap_2();
    for tag in hint.distinct_tags() {
        tags = tags.child(
            div()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(tag.label()),
        );
    }
    div()
        .flex_1()
        .flex()
        .flex_col()
        .gap_3()
        .px_4()
        .py_3()
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(hint.rendered_message()),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(TEXT_BANNER),
        )
        .child(
            div()
                .h_12()
                .min_w(px(112.))
                .self_start()
                .px_6()
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .border_1()
                .border_color(theme.border)
                .text_sm()
                .text_color(theme.accent)
                .child(TEXT_UPGRADE),
        )
        .child(tags)
}
#[cfg(test)]
mod tests {
    use super::{main_area, NOT_DELIVERED, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn title_is_taken_verbatim_and_not_blank() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn not_delivered_text_is_honest_fail_closed() {
        assert_eq!(NOT_DELIVERED, "未下发");
    }
    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }
}
