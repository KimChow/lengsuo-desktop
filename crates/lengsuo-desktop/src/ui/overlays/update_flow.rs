//! 更新提示浮层（波次 6，卡 `ui-overlay-update-flow`，对齐 ui-parity-spec §8 A41）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `update`。
//! 数据源在运行时侧：[`UpdateFlow::initial`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 层标题（逐字取自界面文案全表）。
pub const TITLE: &str = "更新提示";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 强制升级标题。
pub const FORCE_TITLE: &str = "当前版本已停止支持";
/// 强制升级 · 外部渠道副标题模板（占位符 `{version}`）。
pub const FORCE_EXTERNAL_SUBTITLE_TMPL: &str = "请升级至 V{version} 后继续使用";
/// 强制升级 · 受管设备副标题。
pub const FORCE_CONTACT_ADMIN_SUBTITLE: &str = "请联系企业管理员升级至最新版本";
/// 外部商店打开失败提示。
pub const STORE_OPEN_FAILED_HINT: &str = "请手动前往应用商店更新";
/// 普通升级就绪徽标。
pub const NEW_VERSION_READY: &str = "新版本就绪";
/// 普通升级就绪副标题模板（占位符 `{version}`）。
pub const AVAILABLE_TMPL: &str = "发现新版本 V{version}，请前往官网下载并安装";
/// 清单行：更新日志入口标签。
pub const CHANGELOG: &str = "更新日志";
/// 动作：打开软件商店。
pub const OPEN_SOFTWARE_STORE: &str = "前往软件商店更新";
/// 动作：打开应用商店。
pub const OPEN_APP_STORE: &str = "前往应用商店更新";
/// 动作：前往官网下载。
pub const OPEN_WEBSITE: &str = "前往官网下载更新";
/// 动作：稍后（仅非强制升级可推迟）。
pub const LATER: &str = "稍后";
/// 动作：重启升级。
pub const RESTART_TO_UPDATE: &str = "重启升级";
/// 动作：受管设备强制升级的确认。
pub const DISMISS: &str = "知道了";
/// 清单行：软件与系统架构不匹配提示。
pub const ARCH_AVAILABLE: &str = "当前软件版本和系统架构不匹配，可能导致卡顿";
/// 清单行：正确版本下载进度模板（占位符 `{percent}`）。
pub const ARCH_DOWNLOADING_TMPL: &str = "正在下载正确版本 {percent}%";
/// 清单行：正确版本下载完成。
pub const ARCH_READY: &str = "正确版本已下载完成";
/// 清单行：正确版本下载失败。
pub const ARCH_FAILED: &str = "正确版本下载失败，请稍后再试";

/// 浮层可枚举状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateFlowState {
    /// 数据未下发：唯一诚实的初始态（⛔ 不许伪装成空集）。
    NotDelivered,
    /// 强制升级 · 外部渠道：停止支持标题 + 版本副标题 + 三个外部入口。
    ForceExternal { version: String },
    /// 强制升级 · 受管设备：联系企业管理员。
    ForceContactAdmin,
    /// 外部商店打开失败：提示手动前往。
    StoreOpenFailed,
    /// 普通升级就绪：徽标 + 稍后/重启升级。
    Ready { version: String },
}

/// 架构不匹配提示（清单行来源，逐字取自命名空间 `update`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArchNotice {
    /// 版本与架构不匹配，可能卡顿。
    Available,
    /// 正在下载正确版本（进度百分比）。
    Downloading { percent: u8 },
    /// 正确版本已下载完成。
    Ready,
    /// 正确版本下载失败。
    Failed,
}

/// 浮层数据快照（纯数据，Ⓐ 不持凭据、不判权限）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateFlow {
    /// 升级流程状态。
    pub state: UpdateFlowState,
    /// 附带的架构不匹配提示（可选清单行）。
    pub arch: Option<ArchNotice>,
}

fn fill_version(tmpl: &str, version: &str) -> String {
    tmpl.replace("{version}", version)
}
fn fill_percent(tmpl: &str, percent: u8) -> String {
    tmpl.replace("{percent}", &percent.to_string())
}

impl ArchNotice {
    /// 清单行文案。
    pub fn line(&self) -> String {
        match self {
            Self::Available => ARCH_AVAILABLE.to_string(),
            Self::Downloading { percent } => fill_percent(ARCH_DOWNLOADING_TMPL, *percent),
            Self::Ready => ARCH_READY.to_string(),
            Self::Failed => ARCH_FAILED.to_string(),
        }
    }
}

impl UpdateFlow {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self {
            state: UpdateFlowState::NotDelivered,
            arch: None,
        }
    }

    /// 数据是否已下发（未下发 ⇒ 空态，不是空列表）。
    pub fn is_delivered(&self) -> bool {
        !matches!(self.state, UpdateFlowState::NotDelivered)
    }

    /// 徽标文案：已下发才有（⛔ 未下发不伪造徽标）。
    pub fn badge_text(&self) -> Option<&'static str> {
        match &self.state {
            UpdateFlowState::NotDelivered => None,
            UpdateFlowState::Ready { .. } => Some(NEW_VERSION_READY),
            _ => Some(FORCE_TITLE),
        }
    }

    /// 副标题（含模板填充）。
    pub fn subtitle(&self) -> Option<String> {
        match &self.state {
            UpdateFlowState::NotDelivered => None,
            UpdateFlowState::ForceExternal { version } => {
                Some(fill_version(FORCE_EXTERNAL_SUBTITLE_TMPL, version))
            }
            UpdateFlowState::ForceContactAdmin => Some(FORCE_CONTACT_ADMIN_SUBTITLE.to_string()),
            UpdateFlowState::StoreOpenFailed => Some(STORE_OPEN_FAILED_HINT.to_string()),
            UpdateFlowState::Ready { version } => Some(fill_version(AVAILABLE_TMPL, version)),
        }
    }

    /// 清单行：更新日志 + 架构提示（⛔ 未下发返回空，不伪造条目）。
    pub fn rows(&self) -> Vec<String> {
        if !self.is_delivered() {
            return Vec::new();
        }
        let mut rows = vec![CHANGELOG.to_string()];
        if let Some(arch) = &self.arch {
            rows.push(arch.line());
        }
        rows
    }

    /// 底部操作条文案（⛔ 未下发为空，不伪造按钮）。
    pub fn actions(&self) -> &'static [&'static str] {
        match &self.state {
            UpdateFlowState::NotDelivered => &[],
            UpdateFlowState::ForceExternal { .. } | UpdateFlowState::StoreOpenFailed => {
                &[OPEN_SOFTWARE_STORE, OPEN_APP_STORE, OPEN_WEBSITE]
            }
            UpdateFlowState::ForceContactAdmin => &[DISMISS],
            UpdateFlowState::Ready { .. } => &[LATER, RESTART_TO_UPDATE],
        }
    }

    /// 主操作下标（渲染为主按钮）：外部强升=软件商店、打开失败=官网、受管=确认、就绪=重启升级。
    pub fn primary_action(&self) -> usize {
        match &self.state {
            UpdateFlowState::StoreOpenFailed => 2,
            UpdateFlowState::Ready { .. } => 1,
            _ => 0,
        }
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 6：浮层不占主区，恒以 [`UpdateFlow::initial`] 渲染 —— fail-closed）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    render(theme, &UpdateFlow::initial())
}

/// 按状态渲染整层：标题行 + 徽标/副标题/清单行 + 底部操作条。
fn render(theme: &Theme, flow: &UpdateFlow) -> Div {
    let mut col = div()
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
        );
    if !flow.is_delivered() {
        return col.child(center_not_delivered(theme));
    }
    if let Some(text) = flow.badge_text() {
        col = col.child(badge(theme, text));
    }
    if let Some(text) = flow.subtitle() {
        col = col.child(subtitle_row(theme, text));
    }
    for row in flow.rows() {
        col = col.child(detail_row(theme, row));
    }
    col.child(div().flex_1()).child(action_bar(theme, flow))
}
fn subtitle_row(theme: &Theme, text: String) -> Div {
    div()
        .px_4()
        .pt_2()
        .text_sm()
        .text_color(theme.foreground)
        .child(text)
}
fn detail_row(theme: &Theme, text: String) -> Div {
    div()
        .px_4()
        .py_1()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}
/// 未下发空态：居中一行「未下发」（⛔ 不是空列表）。
fn center_not_delivered(theme: &Theme) -> Div {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(NOT_DELIVERED)
}
/// 徽标（已下发态的状态标题）。
fn badge(theme: &Theme, text: &'static str) -> Div {
    div().px_4().pt_2().child(
        div()
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme.accent)
            .text_xs()
            .text_color(theme.foreground)
            .child(text),
    )
}
/// 底部操作条（⛔ 未下发不进这里，不伪造按钮）。
fn action_bar(theme: &Theme, flow: &UpdateFlow) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .px_4()
        .py_2()
        .border_t_1()
        .border_color(theme.border);
    for (i, label) in flow.actions().iter().enumerate() {
        row = row.child(action_button(theme, label, i == flow.primary_action()));
    }
    row
}
/// 单个动作按钮：主操作用强调边框，其余弱样式。
fn action_button(theme: &Theme, label: &'static str, primary: bool) -> Div {
    div()
        .px_4()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(if primary { theme.accent } else { theme.border })
        .text_sm()
        .text_color(if primary {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .child(label)
}
#[cfg(test)]
mod tests {
    use super::{
        main_area, ArchNotice, UpdateFlow, UpdateFlowState, ARCH_AVAILABLE, ARCH_DOWNLOADING_TMPL,
        ARCH_FAILED, ARCH_READY, AVAILABLE_TMPL, CHANGELOG, DISMISS, FORCE_CONTACT_ADMIN_SUBTITLE,
        FORCE_EXTERNAL_SUBTITLE_TMPL, FORCE_TITLE, LATER, NEW_VERSION_READY, NOT_DELIVERED,
        OPEN_APP_STORE, OPEN_SOFTWARE_STORE, OPEN_WEBSITE, RESTART_TO_UPDATE,
        STORE_OPEN_FAILED_HINT, TITLE,
    };
    use gpui_kit::component::Theme;

    fn flow(state: UpdateFlowState, arch: Option<ArchNotice>) -> UpdateFlow {
        UpdateFlow { state, arch }
    }

    #[test]
    fn state_counts_primary_and_templates() {
        let force = flow(
            UpdateFlowState::ForceExternal {
                version: "2.3.1".to_string(),
            },
            None,
        );
        assert_eq!(force.actions().len(), 3);
        assert_eq!(force.actions()[0], OPEN_SOFTWARE_STORE);
        assert_eq!(force.primary_action(), 0);
        assert_eq!(force.rows().len(), 1);
        assert_eq!(
            force.subtitle(),
            Some("请升级至 V2.3.1 后继续使用".to_string())
        );
        let with_arch = flow(
            force.state.clone(),
            Some(ArchNotice::Downloading { percent: 45 }),
        );
        assert_eq!(with_arch.rows().len(), 2);
        assert_eq!(with_arch.rows()[1], "正在下载正确版本 45%");
        let ready = flow(
            UpdateFlowState::Ready {
                version: "1.0.0".to_string(),
            },
            None,
        );
        assert_eq!(ready.actions().len(), 2);
        assert_eq!(ready.primary_action(), 1);
        assert_eq!(
            ready.subtitle(),
            Some("发现新版本 V1.0.0，请前往官网下载并安装".to_string())
        );
        let failed = flow(UpdateFlowState::StoreOpenFailed, None);
        assert_eq!(failed.subtitle(), Some(STORE_OPEN_FAILED_HINT.to_string()));
        assert_eq!(failed.primary_action(), 2);
    }

    #[test]
    fn empty_input_is_not_delivered_state_not_blank() {
        let empty = UpdateFlow::initial();
        assert!(!empty.is_delivered());
        assert!(empty.rows().is_empty());
        assert!(empty.actions().is_empty());
        assert_eq!(empty.badge_text(), None);
        assert_eq!(empty.subtitle(), None);
        // 空态是「未下发」提示，不是空白或空列表。
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn copy_is_verbatim() {
        assert_eq!(TITLE, "更新提示");
        assert_eq!(FORCE_TITLE, "当前版本已停止支持");
        assert_eq!(
            FORCE_EXTERNAL_SUBTITLE_TMPL,
            "请升级至 V{version} 后继续使用"
        );
        assert_eq!(
            FORCE_CONTACT_ADMIN_SUBTITLE,
            "请联系企业管理员升级至最新版本"
        );
        assert_eq!(STORE_OPEN_FAILED_HINT, "请手动前往应用商店更新");
        assert_eq!(NEW_VERSION_READY, "新版本就绪");
        assert_eq!(
            AVAILABLE_TMPL,
            "发现新版本 V{version}，请前往官网下载并安装"
        );
        assert_eq!(CHANGELOG, "更新日志");
        assert_eq!(OPEN_SOFTWARE_STORE, "前往软件商店更新");
        assert_eq!(OPEN_APP_STORE, "前往应用商店更新");
        assert_eq!(OPEN_WEBSITE, "前往官网下载更新");
        assert_eq!(LATER, "稍后");
        assert_eq!(RESTART_TO_UPDATE, "重启升级");
        assert_eq!(DISMISS, "知道了");
        assert_eq!(ARCH_AVAILABLE, "当前软件版本和系统架构不匹配，可能导致卡顿");
        assert_eq!(ARCH_DOWNLOADING_TMPL, "正在下载正确版本 {percent}%");
        assert_eq!(ARCH_READY, "正确版本已下载完成");
        assert_eq!(ARCH_FAILED, "正确版本下载失败，请稍后再试");
    }

    #[test]
    fn badge_and_subtitle_per_state() {
        assert_eq!(UpdateFlow::initial().badge_text(), None);
        let managed = flow(UpdateFlowState::ForceContactAdmin, None);
        assert_eq!(managed.badge_text(), Some(FORCE_TITLE));
        assert_eq!(
            managed.subtitle(),
            Some(FORCE_CONTACT_ADMIN_SUBTITLE.to_string())
        );
        let ready = flow(
            UpdateFlowState::Ready {
                version: "1.0.0".to_string(),
            },
            None,
        );
        assert_eq!(ready.badge_text(), Some(NEW_VERSION_READY));
        assert_eq!(ArchNotice::Available.line(), ARCH_AVAILABLE);
        assert_eq!(
            ArchNotice::Downloading { percent: 100 }.line(),
            "正在下载正确版本 100%"
        );
        assert_eq!(ArchNotice::Ready.line(), ARCH_READY);
        assert_eq!(ArchNotice::Failed.line(), ARCH_FAILED);
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    #[test]
    fn render_all_states_without_panic() {
        let theme = Theme::default();
        let flows = [
            UpdateFlow::initial(),
            flow(
                UpdateFlowState::ForceExternal {
                    version: "2.3.1".to_string(),
                },
                Some(ArchNotice::Available),
            ),
            flow(UpdateFlowState::ForceContactAdmin, None),
            flow(UpdateFlowState::StoreOpenFailed, Some(ArchNotice::Failed)),
            flow(
                UpdateFlowState::Ready {
                    version: "1.0.0".to_string(),
                },
                Some(ArchNotice::Ready),
            ),
        ];
        for f in &flows {
            let _ = super::render(&theme, f);
        }
    }
}
