//! 安全中心·网络审计（`securityCenter.audit.network`）。
//!
//! 分层约定（同 `security_cmd_safety`）：纯数据层（文案表 + 事件模型 + 状态机）加 GPUI 层（只做映射）。
//! 硬约束：fail-closed（加载失败可见，⛔ 不退化成空列表）；文案逐字，禁止自编中文。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "安全中心·网络审计";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "securityCenter.audit.network";
/// 已落表键数。全表 19 有效键；任务书逐字摘录 12 条，其余 7 键正文未随卡下发
/// 且全表不可寻（`docs/ui-parity-spec.md` 仅记键数）——⛔ 禁止自编，待全表补齐。
pub const NS_KEY_COUNT: usize = 12;
/// 加载中文案（状态机分支提示，非命名空间键）。
pub const LOADING_TEXT: &str = "加载中";
/// 数据未下发/加载失败文案（fail-closed；同仓库惯例「未下发」，⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空态文案：本命名空间全表未覆盖空态键，按仓库空态惯例「暂无+面名」。
pub const EMPTY_TEXT: &str = "暂无网络审计事件";

/// 命名空间文案表（key，中文模板），逐字摘自任务书，共 [`NS_KEY_COUNT`] 条。
pub const TEXTS: &[(&str, &str)] = &[
    (
        "securityCenter.audit.network.allowMatched",
        "网络访问命中允许规则：{target}",
    ),
    (
        "securityCenter.audit.network.allowMatchedWithRule",
        "网络访问命中允许规则：{target}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.network.approved",
        "用户已批准访问网络：{target}",
    ),
    (
        "securityCenter.audit.network.approvedWithRule",
        "用户已批准访问网络：{target}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.network.askMatched",
        "网络访问命中询问规则：{target}",
    ),
    (
        "securityCenter.audit.network.askMatchedWithRule",
        "网络访问命中询问规则：{target}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.network.blocked",
        "网络访问被网络安全策略阻止：{target}",
    ),
    (
        "securityCenter.audit.network.blockedWithRule",
        "网络访问命中禁止规则，已阻止：{target}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.network.failed",
        "网络访问失败：{target}",
    ),
    (
        "securityCenter.audit.network.failedWithReason",
        "网络访问失败：{target}（原因：{reason}）",
    ),
    (
        "securityCenter.audit.network.maliciousBlocked",
        "已拦截恶意网站访问：{target}",
    ),
    (
        "securityCenter.audit.network.maliciousBlockedWithRiskType",
        "已拦截恶意网站访问：{target}（风险类型：{riskType}）",
    ),
];

/// 按键名取逐字模板；未命中返回 `None`（fail-closed：未知键不伪造文案）。
pub fn template_of(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, text)| *text)
}

/// 网络审计事件类别（六族：允许 / 已批准 / 询问 / 阻止 / 失败 / 恶意拦截）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkAuditKind {
    AllowMatched,
    Approved,
    AskMatched,
    Blocked,
    Failed,
    MaliciousBlocked,
}

/// 一条网络审计事件（纯数据）：模板参数随类别取用，
/// 扩展字段为 `Some` 时取 With* 键，全 `None` 时取基础键。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkAuditEvent {
    pub kind: NetworkAuditKind,
    pub target: String,
    /// 模板占位 `{matchedRule}` 的实参（允许/已批准/询问/阻止四族使用）。
    pub matched_rule: Option<String>,
    /// 模板占位 `{reason}` 的实参（失败族使用）。
    pub reason: Option<String>,
    /// 模板占位 `{riskType}` 的实参（恶意拦截族使用）。
    pub risk_type: Option<String>,
}

impl NetworkAuditEvent {
    pub fn new(kind: NetworkAuditKind, target: impl Into<String>) -> Self {
        Self {
            kind,
            target: target.into(),
            matched_rule: None,
            reason: None,
            risk_type: None,
        }
    }

    /// 事件对应的文案键（纯函数；键恒落在 [`TEXTS`]，不存在未知键路径）。
    pub fn copy_key(&self) -> &'static str {
        match self.kind {
            NetworkAuditKind::AllowMatched => {
                if self.matched_rule.is_some() {
                    "securityCenter.audit.network.allowMatchedWithRule"
                } else {
                    "securityCenter.audit.network.allowMatched"
                }
            }
            NetworkAuditKind::Approved => {
                if self.matched_rule.is_some() {
                    "securityCenter.audit.network.approvedWithRule"
                } else {
                    "securityCenter.audit.network.approved"
                }
            }
            NetworkAuditKind::AskMatched => {
                if self.matched_rule.is_some() {
                    "securityCenter.audit.network.askMatchedWithRule"
                } else {
                    "securityCenter.audit.network.askMatched"
                }
            }
            NetworkAuditKind::Blocked => {
                if self.matched_rule.is_some() {
                    "securityCenter.audit.network.blockedWithRule"
                } else {
                    "securityCenter.audit.network.blocked"
                }
            }
            NetworkAuditKind::Failed => {
                if self.reason.is_some() {
                    "securityCenter.audit.network.failedWithReason"
                } else {
                    "securityCenter.audit.network.failed"
                }
            }
            NetworkAuditKind::MaliciousBlocked => {
                if self.risk_type.is_some() {
                    "securityCenter.audit.network.maliciousBlockedWithRiskType"
                } else {
                    "securityCenter.audit.network.maliciousBlocked"
                }
            }
        }
    }

    /// 渲染文本（纯函数）：用事件字段替换 `{target}`/`{matchedRule}`/`{reason}`/`{riskType}`。
    pub fn render_text(&self) -> String {
        let template = template_of(self.copy_key()).expect("copy_key 恒落在 TEXTS，模板必须存在");
        template
            .replace("{target}", &self.target)
            .replace("{matchedRule}", self.matched_rule.as_deref().unwrap_or(""))
            .replace("{reason}", self.reason.as_deref().unwrap_or(""))
            .replace("{riskType}", self.risk_type.as_deref().unwrap_or(""))
    }
}

/// 视图态四分支（未拉取过 ≠ 空集；默认 `Loading`，⛔ 不把「还没数据」当「空列表」）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 加载中（初始态）。
    #[default]
    Loading,
    /// 加载失败 / 数据未下发（fail-closed，可见提示）。
    Failed,
    /// 已加载但零事件（空态，不是空列表的沉默渲染）。
    Empty,
    /// 已加载且有事件。
    Active,
}

/// 由「是否成功拿到数据 + 事件条数」解析状态（纯函数；失败优先于空态）。
pub fn resolve(delivered: bool, event_count: usize) -> State {
    if !delivered {
        State::Failed
    } else if event_count == 0 {
        State::Empty
    } else {
        State::Active
    }
}

/// 审计事件流（纯数据承载）。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Feed {
    pub state: State,
    pub events: Vec<NetworkAuditEvent>,
}

impl Feed {
    /// 初始态：加载中（未拉取过 ≠ 空集）。
    pub fn loading() -> Self {
        Self {
            state: State::Loading,
            events: Vec::new(),
        }
    }

    /// fail-closed 默认：数据源未接入 ⇒ 失败可见，不伪造条目。
    pub fn undelivered() -> Self {
        Self {
            state: State::Failed,
            events: Vec::new(),
        }
    }

    /// 已下发构造：按事件条数解析 `Empty` / `Active`。
    pub fn delivered(events: Vec<NetworkAuditEvent>) -> Self {
        Self {
            state: resolve(true, events.len()),
            events,
        }
    }
}

// ===== GPUI 层 =====
/// 单条事件行：类别标识（枚举名，⛔ 不自编中文组名）+ 按模板渲染的逐字文案。
fn render_event_row(theme: &Theme, event: &NetworkAuditEvent) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(format!(
            "[{:?}] {}",
            event.kind,
            event.render_text()
        )))
}

/// 事件流四分支渲染（fail-closed：未下发可见；空态 ≠ 空列表的沉默渲染）。
pub fn render_feed(theme: &Theme, feed: &Feed) -> AnyElement {
    let list = div().flex().flex_col().gap_2();
    let list = match feed.state {
        State::Loading => list.child(SharedString::from(LOADING_TEXT)),
        State::Failed => list.child(SharedString::from(NOT_DELIVERED)),
        State::Empty => list.child(SharedString::from(EMPTY_TEXT)),
        State::Active => feed
            .events
            .iter()
            .fold(list, |acc, e| acc.child(render_event_row(theme, e))),
    };
    list.into_any_element()
}

/// 主区/浮层呈现入口：标题行 + 事件流。
/// 签名固定 `theme: &Theme`；数据源未接入 ⇒ fail-closed 走 [`Feed::undelivered`]，
/// 数据源就绪后由运行时侧喂 [`Feed`]。
pub fn main_area(theme: &Theme) -> AnyElement {
    let feed = Feed::undelivered();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_color(theme.foreground)
                .child(SharedString::from(TITLE)),
        )
        .child(render_feed(theme, &feed))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        main_area, render_feed, resolve, template_of, Feed, NetworkAuditEvent, NetworkAuditKind,
        State, EMPTY_TEXT, LOADING_TEXT, NOT_DELIVERED, NS_KEY_COUNT, NS_PREFIX, TEXTS, TITLE,
    };
    use gpui_kit::component::Theme;

    fn text_of(key: &str) -> &'static str {
        TEXTS
            .iter()
            .find(|(k, _)| *k == key)
            .expect("键必须在 TEXTS")
            .1
    }

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "安全中心·网络审计");
    }

    #[test]
    fn texts_cover_namespace_verbatim() {
        assert_eq!(TEXTS.len(), NS_KEY_COUNT);
        assert!(TEXTS.iter().all(|(k, _)| k.starts_with(NS_PREFIX)));
        // 键唯一且模板非空、可解析。
        let mut keys: Vec<_> = TEXTS.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), NS_KEY_COUNT);
        assert!(TEXTS.iter().all(|(_, t)| !t.is_empty()));
        assert!(TEXTS.iter().all(|(k, _)| template_of(k).is_some()));
    }

    #[test]
    fn templates_verbatim_from_table() {
        // 逐字断言全部 12 键（含三种 With* 括注形态）。
        let expect = [
            (
                "securityCenter.audit.network.allowMatched",
                "网络访问命中允许规则：{target}",
            ),
            (
                "securityCenter.audit.network.allowMatchedWithRule",
                "网络访问命中允许规则：{target}（规则：{matchedRule}）",
            ),
            (
                "securityCenter.audit.network.approved",
                "用户已批准访问网络：{target}",
            ),
            (
                "securityCenter.audit.network.approvedWithRule",
                "用户已批准访问网络：{target}（询问规则：{matchedRule}）",
            ),
            (
                "securityCenter.audit.network.askMatched",
                "网络访问命中询问规则：{target}",
            ),
            (
                "securityCenter.audit.network.askMatchedWithRule",
                "网络访问命中询问规则：{target}（规则：{matchedRule}）",
            ),
            (
                "securityCenter.audit.network.blocked",
                "网络访问被网络安全策略阻止：{target}",
            ),
            (
                "securityCenter.audit.network.blockedWithRule",
                "网络访问命中禁止规则，已阻止：{target}（规则：{matchedRule}）",
            ),
            (
                "securityCenter.audit.network.failed",
                "网络访问失败：{target}",
            ),
            (
                "securityCenter.audit.network.failedWithReason",
                "网络访问失败：{target}（原因：{reason}）",
            ),
            (
                "securityCenter.audit.network.maliciousBlocked",
                "已拦截恶意网站访问：{target}",
            ),
            (
                "securityCenter.audit.network.maliciousBlockedWithRiskType",
                "已拦截恶意网站访问：{target}（风险类型：{riskType}）",
            ),
        ];
        for (key, text) in expect {
            assert_eq!(text_of(key), text, "文案必须逐字一致：{key}");
        }
    }

    #[test]
    fn fail_closed_default() {
        // 初始态 = 加载中（未拉取过 ≠ 空集）；未接入数据源 ⇒ 失败可见，不伪造条目。
        assert_eq!(State::default(), State::Loading);
        assert_eq!(Feed::default(), Feed::loading());
        assert_eq!(Feed::undelivered().state, State::Failed);
        assert_eq!(Feed::undelivered().events.len(), 0);
        // 三个分支提示互不相同（加载中/未下发/空态不可互相顶替）。
        assert_ne!(LOADING_TEXT, NOT_DELIVERED);
        assert_ne!(NOT_DELIVERED, EMPTY_TEXT);
        assert_ne!(LOADING_TEXT, EMPTY_TEXT);
    }

    #[test]
    fn resolve_four_states() {
        assert_eq!(resolve(false, 3), State::Failed);
        assert_eq!(resolve(true, 0), State::Empty);
        assert_eq!(resolve(true, 2), State::Active);
        assert_eq!(Feed::loading().state, State::Loading);
    }

    #[test]
    fn empty_input_is_empty_state_not_silent_list() {
        let feed = Feed::delivered(Vec::new());
        assert_eq!(feed.state, State::Empty);
        // 空态 ≠ 加载中 ≠ 失败（未拉取过 ≠ 空集），且不是零行列表的沉默渲染。
        assert_ne!(feed.state, State::Loading);
        assert_ne!(feed.state, State::Failed);
        assert_ne!(feed.state, State::Active);
        assert_eq!(feed.events.len(), 0);
    }

    #[test]
    fn copy_key_picks_with_variant_per_field() {
        // 六族基础键：扩展字段全 None。
        let base = [
            (
                NetworkAuditKind::AllowMatched,
                "securityCenter.audit.network.allowMatched",
            ),
            (
                NetworkAuditKind::Approved,
                "securityCenter.audit.network.approved",
            ),
            (
                NetworkAuditKind::AskMatched,
                "securityCenter.audit.network.askMatched",
            ),
            (
                NetworkAuditKind::Blocked,
                "securityCenter.audit.network.blocked",
            ),
            (
                NetworkAuditKind::Failed,
                "securityCenter.audit.network.failed",
            ),
            (
                NetworkAuditKind::MaliciousBlocked,
                "securityCenter.audit.network.maliciousBlocked",
            ),
        ];
        for (kind, key) in base {
            assert_eq!(NetworkAuditEvent::new(kind, "example.com").copy_key(), key);
        }
        // With* 键只在对应字段为 Some 时取用；其他字段不参与判定。
        let mut e = NetworkAuditEvent::new(NetworkAuditKind::AllowMatched, "example.com");
        e.matched_rule = Some("allow-cdn".into());
        assert_eq!(
            e.copy_key(),
            "securityCenter.audit.network.allowMatchedWithRule"
        );
        let mut b = NetworkAuditEvent::new(NetworkAuditKind::Blocked, "example.com");
        b.reason = Some("与规则无关".into());
        assert_eq!(b.copy_key(), "securityCenter.audit.network.blocked");
        b.matched_rule = Some("deny-bank".into());
        assert_eq!(b.copy_key(), "securityCenter.audit.network.blockedWithRule");
        let mut f = NetworkAuditEvent::new(NetworkAuditKind::Failed, "example.com");
        f.matched_rule = Some("与原因无关".into());
        assert_eq!(f.copy_key(), "securityCenter.audit.network.failed");
        f.reason = Some("超时".into());
        assert_eq!(
            f.copy_key(),
            "securityCenter.audit.network.failedWithReason"
        );
        let mut m = NetworkAuditEvent::new(NetworkAuditKind::MaliciousBlocked, "example.com");
        m.matched_rule = Some("与风险类型无关".into());
        assert_eq!(
            m.copy_key(),
            "securityCenter.audit.network.maliciousBlocked"
        );
        m.risk_type = Some("钓鱼".into());
        assert_eq!(
            m.copy_key(),
            "securityCenter.audit.network.maliciousBlockedWithRiskType"
        );
    }

    #[test]
    fn render_text_fills_placeholders() {
        let mut b = NetworkAuditEvent::new(NetworkAuditKind::Blocked, "bank.example.com");
        b.matched_rule = Some("deny-bank".into());
        assert_eq!(
            b.render_text(),
            "网络访问命中禁止规则，已阻止：bank.example.com（规则：deny-bank）"
        );
        let mut f = NetworkAuditEvent::new(NetworkAuditKind::Failed, "cdn.example.com");
        f.reason = Some("DNS 解析失败".into());
        assert_eq!(
            f.render_text(),
            "网络访问失败：cdn.example.com（原因：DNS 解析失败）"
        );
        let mut m = NetworkAuditEvent::new(NetworkAuditKind::MaliciousBlocked, "evil.example.net");
        m.risk_type = Some("钓鱼".into());
        assert_eq!(
            m.render_text(),
            "已拦截恶意网站访问：evil.example.net（风险类型：钓鱼）"
        );
        let mut a = NetworkAuditEvent::new(NetworkAuditKind::Approved, "update.example.org");
        a.matched_rule = Some("ask-cdn".into());
        assert_eq!(
            a.render_text(),
            "用户已批准访问网络：update.example.org（询问规则：ask-cdn）"
        );
    }

    #[test]
    fn main_area_constructs_element_tree() {
        // 只构造元素树，不 panic；无数据源 ⇒ fail-closed 走未下发分支。
        let _ = main_area(&Theme::default());
        assert_eq!(TITLE, "安全中心·网络审计");
    }

    #[test]
    fn render_branches_all_states_no_panic() {
        let theme = Theme::default();
        let loading = Feed::loading();
        assert_eq!(loading.state, State::Loading);
        let _ = render_feed(&theme, &loading);

        let failed = Feed::undelivered();
        assert_eq!(failed.state, State::Failed);
        let _ = render_feed(&theme, &failed);

        let empty = Feed::delivered(Vec::new());
        assert_eq!(empty.state, State::Empty);
        let _ = render_feed(&theme, &empty);

        let events = vec![
            NetworkAuditEvent::new(NetworkAuditKind::Blocked, "tracker.example.io"),
            NetworkAuditEvent::new(NetworkAuditKind::AllowMatched, "api.example.com"),
        ];
        let active = Feed::delivered(events);
        assert_eq!(active.state, State::Active);
        let _ = render_feed(&theme, &active);
    }
}
