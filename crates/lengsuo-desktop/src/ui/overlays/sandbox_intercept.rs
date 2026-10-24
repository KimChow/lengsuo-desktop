//! 沙箱权限拦截浮层（波次 8，卡 `ui-overlay-sandbox-intercept`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `sandbox`。
//! 数据源在运行时侧：无数据源时如实渲染「未下发」，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 文案命名空间 `sandbox` 全 9 键，逐字取自界面文案全表。
pub mod copy {
    /// `sandbox.intercept.action`
    pub const ACTION: &str = "操作";
    /// `sandbox.intercept.allow`
    pub const ALLOW: &str = "允许";
    /// `sandbox.intercept.allowSession`
    pub const ALLOW_SESSION: &str = "允许，不再询问";
    /// `sandbox.intercept.deny`
    pub const DENY: &str = "拒绝";
    /// `sandbox.intercept.description`
    pub const DESCRIPTION: &str = "AI 想要访问当前沙箱范围之外的资源。";
    /// `sandbox.intercept.fileAccess`
    pub const FILE_ACCESS: &str = "文件访问";
    /// `sandbox.intercept.networkTitle`
    pub const NETWORK_TITLE: &str = "网络访问请求";
    /// `sandbox.intercept.permissionTitle`
    pub const PERMISSION_TITLE: &str = "权限请求";
    /// `sandbox.intercept.scope`
    pub const SCOPE: &str = "范围";
}

/// 视图标题（= `sandbox.intercept.permissionTitle`，逐字）。
pub const TITLE: &str = copy::PERMISSION_TITLE;
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    Loading,
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 越界访问的分类（对应文案键 `networkTitle` / `fileAccess`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    /// 网络访问请求。
    Network,
    /// 文件访问。
    File,
}

impl AccessKind {
    /// 分类标题文案（逐字）。
    pub fn title(self) -> &'static str {
        match self {
            AccessKind::Network => copy::NETWORK_TITLE,
            AccessKind::File => copy::FILE_ACCESS,
        }
    }
}

/// 单条越界访问请求：分类 + 范围目标（域名/路径；仅展示，不判权限、不持凭据）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessRequest {
    pub kind: AccessKind,
    pub scope: String,
}

/// 用户可选的处置动作（仅驱动按钮文案；浮层本身不执行任何动作）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Allow,
    AllowSession,
    Deny,
}

impl Action {
    /// 按钮文案（逐字）。
    pub fn label(self) -> &'static str {
        match self {
            Action::Allow => copy::ALLOW,
            Action::AllowSession => copy::ALLOW_SESSION,
            Action::Deny => copy::DENY,
        }
    }
}

/// 纯数据模型（无 GPUI 依赖；视图是其纯函数）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SandboxIntercept {
    pub state: State,
    pub requests: Vec<AccessRequest>,
}

impl SandboxIntercept {
    /// 诚实构造：数据未下发（fail-closed 默认态）。
    pub fn not_delivered() -> Self {
        Self {
            state: State::NotDelivered,
            requests: Vec::new(),
        }
    }

    /// 由已下发请求构造；空列表仍落 `NotDelivered`（空输入 ⇒ 空态，不是空列表）。
    pub fn from_requests(requests: Vec<AccessRequest>) -> Self {
        if requests.is_empty() {
            Self::not_delivered()
        } else {
            Self {
                state: State::Normal,
                requests,
            }
        }
    }

    /// 是否处于「未下发」空态。
    pub fn is_not_delivered(&self) -> bool {
        self.state == State::NotDelivered
    }

    /// 展示行（分类标题, 范围目标）；仅聚合已下发数据，不产生新条目。
    pub fn rows(&self) -> Vec<(&'static str, &str)> {
        self.requests
            .iter()
            .map(|r| (r.kind.title(), r.scope.as_str()))
            .collect()
    }

    /// 「范围」载荷行：非 `Normal` 一律 fail-closed 显示「未下发」。
    pub fn scope_lines(&self) -> Vec<&str> {
        if self.state == State::Normal {
            self.requests.iter().map(|r| r.scope.as_str()).collect()
        } else {
            vec![NOT_DELIVERED]
        }
    }

    /// 三个动作按钮文案，顺序固定：允许 / 允许，不再询问 / 拒绝。
    pub fn actions(&self) -> [&'static str; 3] {
        [
            Action::Allow.label(),
            Action::AllowSession.label(),
            Action::Deny.label(),
        ]
    }
}

/// 主区渲染：标题行 + 说明行 + 「范围」载荷区 + 操作行（颜色全取主题 token）。
pub fn main_area(theme: &Theme) -> Div {
    let model = SandboxIntercept::not_delivered();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(TITLE))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(copy::DESCRIPTION),
        )
        .child(scope_section(&model, theme))
        .child(action_row(theme))
}

/// 「范围」载荷区：未下发 ⇒ 如实一行「未下发」（fail-closed，⛔ 不伪造条目）。
fn scope_section(model: &SandboxIntercept, theme: &Theme) -> Div {
    let mut section = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_color(theme.muted_foreground).child(copy::SCOPE));
    if model.is_not_delivered() {
        section = section.child(div().text_color(theme.foreground).child(NOT_DELIVERED));
    } else {
        for (kind, scope) in model.rows() {
            section = section.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().text_color(theme.muted_foreground).child(kind))
                    .child(div().text_color(theme.foreground).child(scope.to_string())),
            );
        }
    }
    section
}

/// 操作行：「操作」标签 + 允许 / 允许，不再询问 / 拒绝（纯展示骨架，不接线）。
fn action_row(theme: &Theme) -> Div {
    let model = SandboxIntercept::not_delivered();
    let mut row = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_color(theme.muted_foreground).child(copy::ACTION));
    for label in model.actions() {
        row = row.child(div().text_color(theme.foreground).child(label));
    }
    row
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
