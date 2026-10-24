//! 协作·连接器（波次 9，卡 `ui-overlay-collab-connectors`）。
//!
//! 分层约定（同 `panel_views` / `collab_plan_detail`）：纯数据层 + GPUI 层。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.connectors`。
//! 全表 140 键未随卡下发：[`TEXTS`] 先落任务书逐字摘录 12 条，其余键待全表下发后补（⛔ 不自编中文）。
#![cfg(feature = "ui")]

/// 面板标题（逐字取自文案全表，占位卡沿用）。
pub const PANEL_TITLE: &str = "协作·连接器";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.connectors";
/// 已下发键数（全表 140 键待下发，见 [`TEXTS`] 文档）。
pub const NS_KEY_COUNT: usize = TEXTS.len();

// ===== 纯数据层（无 GPUI 依赖）=====
/// 命名空间 `collab.connectors` 已下发文案（key、中文均逐字，⛔ 不自编）。
/// 每行两条仅为压行数，键序与摘录一致。
#[rustfmt::skip]
pub const TEXTS: &[(&str, &str)] = &[
    ("collab.connectors.action.addTeamCredential", "添加团队凭证"), ("collab.connectors.action.authorizeAll", "授权全员使用"),
    ("collab.connectors.action.connect", "连接"), ("collab.connectors.action.connecting", "连接中…"),
    ("collab.connectors.action.deleteTeamCredential", "删除团队凭证"), ("collab.connectors.action.disconnect", "断开连接"),
    ("collab.connectors.action.goConnect", "去连接"), ("collab.connectors.action.moreLabel", "更多操作"),
    ("collab.connectors.action.remove", "移除"), ("collab.connectors.action.removeFromList", "删除"),
    ("collab.connectors.action.updateCredential", "更新凭证"), ("collab.connectors.addWithPlus", "+ 添加"),
];

/// 按全量 key 取文案；未知 key 返回 `None`（fail-closed，不猜值）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 渲染层取词（key 必须在 [`TEXTS`]；测试锁表防漂移，缺词落空串显形）。
fn t(key: &str) -> &'static str {
    text(key).unwrap_or_default()
}

/// 单条连接器的连接态（主按钮文案映射见 [`Connector::action_label`]）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnState {
    /// 未连接（可发起连接）。
    #[default]
    Disconnected,
    /// 连接进行中（`action.connecting`）。
    Connecting,
    /// 已连接（可断开）。
    Connected,
}

/// 一条连接器（名称必填、其余可空；⛔ 不伪造服务端数据）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Connector {
    /// 连接器名称（数据下发时必有，无逐字 fallback 故不做 Option）。
    pub name: String,
    /// 描述（可无；无值不渲染该行）。
    pub desc: Option<String>,
    /// 连接态。
    pub conn: ConnState,
    /// 是否已配置团队凭证（决定凭证行动作集）。
    pub team_credential: bool,
}

impl Connector {
    /// 主动作文案：已连接「断开连接」/连接中「连接中…」/未连接「连接」；
    /// 未连接且未配凭证须先走授权流程 ⇒「去连接」。
    pub fn action_label(&self) -> &'static str {
        match self.conn {
            ConnState::Connected => t("collab.connectors.action.disconnect"),
            ConnState::Connecting => t("collab.connectors.action.connecting"),
            ConnState::Disconnected if !self.team_credential => {
                t("collab.connectors.action.goConnect")
            }
            ConnState::Disconnected => t("collab.connectors.action.connect"),
        }
    }

    /// 行内次级动作：未连接「移除」；连接中/已连接「删除」。
    pub fn secondary_label(&self) -> &'static str {
        match self.conn {
            ConnState::Disconnected => t("collab.connectors.action.remove"),
            ConnState::Connecting | ConnState::Connected => {
                t("collab.connectors.action.removeFromList")
            }
        }
    }

    /// 凭证行文案：已配 ⇒ 更新/删除/授权全员使用；未配 ⇒ 添加团队凭证。
    pub fn credential_labels(&self) -> Vec<&'static str> {
        if self.team_credential {
            vec![
                t("collab.connectors.action.updateCredential"),
                t("collab.connectors.action.deleteTeamCredential"),
                t("collab.connectors.action.authorizeAll"),
            ]
        } else {
            vec![t("collab.connectors.action.addTeamCredential")]
        }
    }
}

/// 浮层/面板状态机（默认 `NotDelivered`：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 就绪：数据已下发且有连接器。
    Ready(Vec<Connector>),
    /// 空态：已下发但无连接器（以「+ 添加」引导，⛔ 不是空列表）。
    Empty,
    /// 加载中（面板级加载即连接，显示 `action.connecting`）。
    Loading,
    /// 数据未下发（无数据源时的诚实默认态，显示「未下发」）。
    #[default]
    NotDelivered,
}

impl State {
    /// 各分支提示文案（逐字取自 [`TEXTS`]；`Ready` 无提示行）。
    /// 空态专属键未随摘录下发，暂以 `addWithPlus` 引导，待全表下发后替换。
    pub fn status_text(&self) -> &'static str {
        match self {
            State::Ready(_) => "",
            State::Empty => t("collab.connectors.addWithPlus"),
            State::Loading => t("collab.connectors.action.connecting"),
            State::NotDelivered => NOT_DELIVERED,
        }
    }
}

/// 由运行时输入解析初始态（纯函数）：`None` = 未拉取过 ≠ `Some(vec![])` = 空集。
pub fn resolve(delivered: Option<Vec<Connector>>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(items) if items.is_empty() => State::Empty,
        Some(items) => State::Ready(items),
    }
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{resolve, text, State, NOT_DELIVERED, PANEL_TITLE, TEXTS};

    #[test]
    fn copy_is_verbatim() {
        assert_eq!(PANEL_TITLE, "协作·连接器");
        assert_eq!(
            text("collab.connectors.action.addTeamCredential"),
            Some("添加团队凭证")
        );
        assert_eq!(
            text("collab.connectors.action.authorizeAll"),
            Some("授权全员使用")
        );
        assert_eq!(text("collab.connectors.action.connect"), Some("连接"));
        assert_eq!(text("collab.connectors.action.connecting"), Some("连接中…"));
        assert_eq!(
            text("collab.connectors.action.disconnect"),
            Some("断开连接")
        );
        assert_eq!(text("collab.connectors.action.goConnect"), Some("去连接"));
        assert_eq!(text("collab.connectors.action.remove"), Some("移除"));
        assert_eq!(
            text("collab.connectors.action.removeFromList"),
            Some("删除")
        );
        assert_eq!(
            text("collab.connectors.action.updateCredential"),
            Some("更新凭证")
        );
        assert_eq!(text("collab.connectors.addWithPlus"), Some("+ 添加"));
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(State::NotDelivered.status_text(), NOT_DELIVERED);
    }

    #[test]
    fn ns_table_locked() {
        let n = TEXTS.len();
        assert_eq!(n, super::NS_KEY_COUNT);
        let mut seen: Vec<&str> = TEXTS.iter().map(|(k, _)| *k).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n);
        assert!(seen.iter().all(|k| k.starts_with("collab.connectors.")));
    }
}
