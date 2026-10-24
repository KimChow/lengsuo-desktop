//! 命令安全审计（波次 8，卡 `ui-security-cmd-safety`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `securityCenter.audit.commandSafety`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "命令安全审计";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空态文案：本命名空间全表未覆盖空态键，按仓库空态惯例「暂无+面名」
/// （同 `right_panel::APPROVALS_EMPTY_TEXT`）。
pub const EMPTY_TEXT: &str = "暂无命令安全事件";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "securityCenter.audit.commandSafety";
/// 命名空间键数（据文案全表，严格前缀 `securityCenter.audit.commandSafety.`）。
pub const NS_KEY_COUNT: usize = 33;
/// 兜底事件键：事件类别无法识别时按全表 fallback 模板呈现（不伪造类别）。
pub const FALLBACK_KEY: &str = "securityCenter.audit.commandSafety.fallback";

/// 命名空间全量文案（key，中文模板），逐字取自界面文案全表，共 [`NS_KEY_COUNT`] 条。
pub const TEXTS: &[(&str, &str)] = &[
    (
        "securityCenter.audit.commandSafety.allowMatched",
        "命令命中允许规则：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.allowMatchedWithRule",
        "命令命中允许规则：{command}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.allowOverriddenByCredential",
        "放行命令仍需审批：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.allowOverriddenByCredentialWithRule",
        "放行命令仍需审批：{command}（敏感凭据：{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.allowOverriddenByOpaqueHost",
        "放行命令仍需审批：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.allowOverriddenByOpaqueHostWithRule",
        "放行命令仍需审批：{command}（脚本宿主:{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.approved",
        "用户已批准执行命令：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.askMatched",
        "命令命中询问规则：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.askMatchedWithRule",
        "命令命中询问规则：{command}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.autoApproved",
        "沙箱自动放行命令：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.bypassLegacySilent",
        "模型请求绕过沙箱（legacy），已直接本地执行：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.credentialOrOpaqueBlocked",
        "命令触发敏感保护：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.credentialOrOpaqueBlockedWithRule",
        "命令触发敏感保护：{command}（命中：{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.denyMatched",
        "命中命令安全禁止规则，命令已拦截：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.denyMatchedWithRule",
        "命中命令安全禁止规则，命令已拦截：{command}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.excludedCommand",
        "命令不进入沙箱，已直接本地执行：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.execPolicyAllow",
        "命中执行白名单，已直接本地执行：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.execPolicyAllowWithRule",
        "命中执行白名单，已直接本地执行：{command}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.execPolicyForbidden",
        "命中执行禁止规则，命令已拦截：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.execPolicyForbiddenWithRule",
        "命中执行禁止规则，命令已拦截：{command}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.commandSafety.fallback",
        "命令安全事件：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.networkStrictBlocked",
        "网络严格模式已拦截命令（未经代理的直连被系统层拒绝）：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.preExecBypassApproved",
        "用户已批准绕过沙箱执行：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.preExecBypassRejected",
        "用户已拒绝绕过沙箱执行：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.preExecBypassRequested",
        "模型请求绕过沙箱，等待用户确认：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.programBlacklistBlocked",
        "程序黑名单已拦截命令：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.programBlacklistBlockedWithPrograms",
        "程序黑名单已拦截命令：{command}（程序：{programs}）",
    ),
    (
        "securityCenter.audit.commandSafety.rejected",
        "用户已拒绝执行命令：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.safetyFilterAllowOverride",
        "命中允许规则的命令已自动通过普通文件检查：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.safetyFilterAskOverride",
        "用户已确认继续执行命中询问规则的命令：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.sandboxExecuted",
        "沙箱内执行命令：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.sensitiveSessionCacheHit",
        "敏感命令命中会话放行缓存，已直接本地执行：{command}",
    ),
    (
        "securityCenter.audit.commandSafety.sessionCacheHit",
        "命中会话放行缓存，已直接本地执行：{command}",
    ),
];

/// 一条命令安全审计事件（纯数据；`key` 必须落在 [`TEXTS`]，未知键走 [`FALLBACK_KEY`]）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditEvent {
    pub key: &'static str,
    pub command: String,
    /// 模板占位 `{matchedRule}` 的实参（无则模板不带该占位）。
    pub matched_rule: Option<String>,
    /// 模板占位 `{programs}` 的实参（仅程序黑名单事件使用）。
    pub programs: Option<String>,
}

impl AuditEvent {
    pub fn new(key: &'static str, command: impl Into<String>) -> Self {
        Self {
            key,
            command: command.into(),
            matched_rule: None,
            programs: None,
        }
    }
}

/// 浮层三态（未拉取过 ≠ 空集；默认 `NotDelivered`，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发且有事件。
    Active,
    /// 数据已下发但零事件（空态，不是空列表）。
    Empty,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 由「是否已下发 + 事件条数」解析三态（纯函数）。
pub fn resolve(delivered: bool, event_count: usize) -> State {
    if !delivered {
        State::NotDelivered
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
    pub events: Vec<AuditEvent>,
}

impl Feed {
    /// fail-closed 默认：未下发 ⇒ 事件必须为空。
    pub fn undelivered() -> Self {
        Self {
            state: State::NotDelivered,
            events: Vec::new(),
        }
    }

    /// 已下发构造：按事件条数解析 `Empty` / `Active`。
    pub fn delivered(events: Vec<AuditEvent>) -> Self {
        Self {
            state: resolve(true, events.len()),
            events,
        }
    }
}

/// 事件判定分类（审计目录的三组）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// 自动放行 / 沙箱内或直接本地执行。
    AutoAllow,
    /// 需用户确认或用户已确认。
    NeedsApproval,
    /// 已拦截 / 已拒绝。
    Blocked,
}

const BLOCKED_MARKERS: &[&str] = &[
    "Blocked",
    "denyMatched",
    "Forbidden",
    "rejected",
    "Rejected",
];
const APPROVAL_MARKERS: &[&str] = &[
    "Overridden",
    "askMatched",
    "AskOverride",
    "Requested",
    "BypassApproved",
    "approved",
];

/// 按事件键判定分类（纯函数；`autoApproved` 是沙箱自动放行，先于审批标记特判）。
pub fn verdict_of_key(key: &str) -> Verdict {
    if BLOCKED_MARKERS.iter().any(|m| key.contains(m)) {
        Verdict::Blocked
    } else if key.ends_with(".autoApproved") {
        Verdict::AutoAllow
    } else if APPROVAL_MARKERS.iter().any(|m| key.contains(m)) {
        Verdict::NeedsApproval
    } else {
        Verdict::AutoAllow
    }
}

/// 三组分类计数（纯函数，用于状态栏汇总）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub auto_allow: usize,
    pub needs_approval: usize,
    pub blocked: usize,
}

/// 统计事件流的分类计数（纯函数）。
pub fn counts(events: &[AuditEvent]) -> Counts {
    let mut c = Counts::default();
    for e in events {
        match verdict_of_key(e.key) {
            Verdict::AutoAllow => c.auto_allow += 1,
            Verdict::NeedsApproval => c.needs_approval += 1,
            Verdict::Blocked => c.blocked += 1,
        }
    }
    c
}

/// 按模板占位符渲染一条事件文案（纯函数；未知键 fail-closed 走 fallback 模板，不伪造类别）。
pub fn render_event(event: &AuditEvent) -> String {
    let template = TEXTS
        .iter()
        .find(|(k, _)| *k == event.key)
        .map(|(_, t)| *t)
        .unwrap_or_else(|| {
            TEXTS
                .iter()
                .find(|(k, _)| *k == FALLBACK_KEY)
                .expect("fallback 键必须在 TEXTS")
                .1
        });
    template
        .replace("{command}", &event.command)
        .replace("{matchedRule}", event.matched_rule.as_deref().unwrap_or(""))
        .replace("{programs}", event.programs.as_deref().unwrap_or(""))
}

// ===== GPUI 层 =====
/// 单条事件行：判定分类标识 + 按模板渲染的逐字文案。
fn render_event_row(theme: &Theme, event: &AuditEvent) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(format!(
            "[{:?}] {}",
            verdict_of_key(event.key),
            render_event(event)
        )))
}

/// 事件流三分支渲染（fail-closed：未下发 / 空态都不伪造条目）。
pub fn render_feed(theme: &Theme, feed: &Feed) -> AnyElement {
    let list = div().flex().flex_col().gap_2();
    let list = match feed.state {
        State::NotDelivered => list.child(SharedString::from(NOT_DELIVERED)),
        State::Empty => list.child(SharedString::from(EMPTY_TEXT)),
        State::Active => feed
            .events
            .iter()
            .fold(list, |acc, e| acc.child(render_event_row(theme, e))),
    };
    list.into_any_element()
}

/// 审计事件目录：按三组判定分类列出命名空间全量模板（真实参考数据，非伪造条目）。
/// 组标签用枚举标识符 + 计数（⛔ 不自编中文组名）。
pub fn render_catalog(theme: &Theme) -> AnyElement {
    let section = div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground);
    let section = section.child(SharedString::from(NS_PREFIX));
    [Verdict::AutoAllow, Verdict::NeedsApproval, Verdict::Blocked]
        .into_iter()
        .fold(section, |acc, verdict| {
            let rows: Vec<_> = TEXTS
                .iter()
                .filter(|(k, _)| verdict_of_key(k) == verdict)
                .map(|(k, t)| format!("{k} {t}"))
                .collect();
            let group = div()
                .flex()
                .flex_col()
                .gap_1()
                .child(SharedString::from(format!("{verdict:?} · {}", rows.len())));
            let group = rows
                .into_iter()
                .fold(group, |a, line| a.child(SharedString::from(line)));
            acc.child(group)
        })
        .into_any_element()
}

/// 主区/浮层呈现入口：标题行 + 事件流（无数据源 ⇒ 未下发）+ 事件目录。
/// 签名固定 `theme: &Theme`；数据源就绪后由运行时侧喂 [`Feed`]。
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
        .child(render_catalog(theme))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        counts, main_area, render_catalog, render_event, render_feed, resolve, verdict_of_key,
        AuditEvent, Counts, Feed, State, Verdict, EMPTY_TEXT, FALLBACK_KEY, NOT_DELIVERED,
        NS_KEY_COUNT, NS_PREFIX, TEXTS, TITLE,
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
    fn texts_cover_namespace_verbatim() {
        assert_eq!(TEXTS.len(), NS_KEY_COUNT);
        assert!(TEXTS.iter().all(|(k, _)| k.starts_with(NS_PREFIX)));
        // 键唯一且模板非空。
        let mut keys: Vec<_> = TEXTS.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        let unique = keys.clone();
        keys.dedup();
        assert_eq!(keys.len(), unique.len());
        assert!(TEXTS.iter().all(|(_, t)| !t.is_empty()));
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(Feed::default(), Feed::undelivered());
        assert_eq!(Feed::default().events.len(), 0);
    }

    #[test]
    fn counts_classify_all_events() {
        let events: Vec<_> = TEXTS
            .iter()
            .map(|(k, _)| AuditEvent::new(k, "ls"))
            .collect();
        let c = counts(&events);
        assert_eq!(
            c,
            Counts {
                auto_allow: 12,
                needs_approval: 10,
                blocked: 11
            }
        );
        assert_eq!(c.auto_allow + c.needs_approval + c.blocked, NS_KEY_COUNT);
        // 特判键不落入审批组：沙箱自动放行是自动放行。
        assert_eq!(
            verdict_of_key("securityCenter.audit.commandSafety.autoApproved"),
            Verdict::AutoAllow
        );
    }

    #[test]
    fn resolve_three_states() {
        assert_eq!(resolve(false, 3), State::NotDelivered);
        assert_eq!(resolve(true, 0), State::Empty);
        assert_eq!(resolve(true, 2), State::Active);
    }

    #[test]
    fn empty_input_is_empty_state_not_empty_list() {
        let feed = Feed::delivered(Vec::new());
        assert_eq!(feed.state, State::Empty);
        // 空态 ≠ 未下发（未拉取过 ≠ 空集），且不是零行列表的沉默渲染。
        assert_ne!(feed.state, State::NotDelivered);
        assert_ne!(feed.state, State::Active);
        assert_eq!(feed.events.len(), 0);
        assert_ne!(EMPTY_TEXT, NOT_DELIVERED);
    }

    #[test]
    fn templates_verbatim_from_table() {
        // 逐字断言 ≥3 键：含半角冒号的「脚本宿主:」与 {programs} 占位两种特殊形态。
        assert_eq!(
            text_of("securityCenter.audit.commandSafety.allowMatched"),
            "命令命中允许规则：{command}"
        );
        assert_eq!(
            text_of("securityCenter.audit.commandSafety.allowOverriddenByOpaqueHostWithRule"),
            "放行命令仍需审批：{command}（脚本宿主:{matchedRule}）"
        );
        assert_eq!(
            text_of("securityCenter.audit.commandSafety.programBlacklistBlockedWithPrograms"),
            "程序黑名单已拦截命令：{command}（程序：{programs}）"
        );
        assert_eq!(
            text_of("securityCenter.audit.commandSafety.fallback"),
            "命令安全事件：{command}"
        );
        assert_eq!(
            text_of("securityCenter.audit.commandSafety.networkStrictBlocked"),
            "网络严格模式已拦截命令（未经代理的直连被系统层拒绝）：{command}"
        );
    }

    #[test]
    fn render_event_fills_placeholders_and_falls_back() {
        let mut e = AuditEvent::new(
            "securityCenter.audit.commandSafety.denyMatchedWithRule",
            "cargo build",
        );
        e.matched_rule = Some("exec-policy-3".into());
        assert_eq!(
            render_event(&e),
            "命中命令安全禁止规则，命令已拦截：cargo build（规则：exec-policy-3）"
        );

        let mut p = AuditEvent::new(
            "securityCenter.audit.commandSafety.programBlacklistBlockedWithPrograms",
            "bash -c",
        );
        p.programs = Some("curl,wget".into());
        assert_eq!(
            render_event(&p),
            "程序黑名单已拦截命令：bash -c（程序：curl,wget）"
        );

        // 未知键 fail-closed 走 fallback 模板，不伪造类别。
        let unknown = AuditEvent::new("securityCenter.audit.commandSafety.noSuchKey", "ls -l");
        assert_eq!(render_event(&unknown), "命令安全事件：ls -l");
        assert_eq!(FALLBACK_KEY, "securityCenter.audit.commandSafety.fallback");
    }

    #[test]
    fn main_area_constructs_element_tree() {
        // 只构造元素树，不 panic；无数据源 ⇒ 未下发分支。
        let _ = main_area(&Theme::default());
        assert_eq!(TITLE, "命令安全审计");
    }

    #[test]
    fn render_branch_not_delivered() {
        let feed = Feed::undelivered();
        assert_eq!(feed.state, State::NotDelivered);
        let _ = render_feed(&Theme::default(), &feed);
    }

    #[test]
    fn render_branch_empty() {
        let feed = Feed::delivered(Vec::new());
        assert_eq!(feed.state, State::Empty);
        let _ = render_feed(&Theme::default(), &feed);
    }

    #[test]
    fn render_branch_active() {
        let events = vec![
            AuditEvent::new(
                "securityCenter.audit.commandSafety.sandboxExecuted",
                "cargo test",
            ),
            AuditEvent::new("securityCenter.audit.commandSafety.rejected", "rm -rf /"),
        ];
        let feed = Feed::delivered(events);
        assert_eq!(feed.state, State::Active);
        assert_eq!(
            counts(&feed.events),
            Counts {
                auto_allow: 1,
                needs_approval: 0,
                blocked: 1
            }
        );
        let _ = render_feed(&Theme::default(), &feed);
        let _ = render_catalog(&Theme::default());
    }
}
