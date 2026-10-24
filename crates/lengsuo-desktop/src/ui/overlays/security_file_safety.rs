//! 文件安全审计（波次 8，卡 `ui-security-file-safety`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；内容文案逐字取自命名空间 `securityCenter.audit.fileSafety`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "文件安全审计";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空态说明（已下发但零条目 ⇒ 空态，⛔ 不是空列表）。状态行标签，同 stub 既有标签先例；
/// 内容文案（68 键）才受逐字红线约束。
pub const EMPTY_HINT: &str = "暂无记录";

// ===== 纯数据层（无 GPUI 依赖）=====
/// 命名空间前缀。
pub const NS_PREFIX: &str = "securityCenter.audit.fileSafety";
/// 键数（= [`TEXTS`] 行数，测试锁定；68 键 = 11 个家族）。
pub const NS_KEY_COUNT: usize = 68;

/// 命名空间全表（key → 中文模板，逐字；占位符 `{paths}` / `{matchedRule}` / `{count}` / `{threshold}` 原样保留）。
pub const TEXTS: &[(&str, &str)] = &[
    (
        "securityCenter.audit.fileSafety.allowListed",
        "文件访问命中文件安全允许规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.allowListed.delete",
        "文件删除命中文件安全允许规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.allowListed.deleteWithRule",
        "文件删除命中文件安全允许规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.allowListed.read",
        "文件读取命中文件安全允许规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.allowListed.readWithRule",
        "文件读取命中文件安全允许规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.allowListed.write",
        "文件写入命中文件安全允许规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.allowListed.writeWithRule",
        "文件写入命中文件安全允许规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.allowListedWithRule",
        "文件访问命中文件安全允许规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.approved",
        "用户已批准文件访问：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.approved.delete",
        "用户已批准删除文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.approved.deleteWithRule",
        "用户已批准删除文件：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.approved.read",
        "用户已批准读取文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.approved.readWithRule",
        "用户已批准读取文件：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.approved.write",
        "用户已批准写入文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.approved.writeWithRule",
        "用户已批准写入文件：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.approvedWithRule",
        "用户已批准文件访问：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.askMatched",
        "文件访问命中询问规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.askMatched.delete",
        "文件删除命中询问规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.askMatched.deleteWithRule",
        "文件删除命中询问规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.askMatched.read",
        "文件读取命中询问规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.askMatched.readWithRule",
        "文件读取命中询问规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.askMatched.write",
        "文件写入命中询问规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.askMatched.writeWithRule",
        "文件写入命中询问规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.askMatchedWithRule",
        "文件访问命中询问规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.autoApproved",
        "沙箱自动放行文件访问：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.autoApproved.delete",
        "沙箱自动放行删除文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.autoApproved.read",
        "沙箱自动放行读取文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.autoApproved.write",
        "沙箱自动放行写入文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.blockListed",
        "文件访问命中文件安全禁止规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.blockListed.delete",
        "文件删除命中文件安全禁止规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.blockListed.deleteWithRule",
        "文件删除命中文件安全禁止规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.blockListed.read",
        "文件读取命中文件安全禁止规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.blockListed.readWithRule",
        "文件读取命中文件安全禁止规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.blockListed.write",
        "文件写入命中文件安全禁止规则：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.blockListed.writeWithRule",
        "文件写入命中文件安全禁止规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.blockListedWithRule",
        "文件访问命中文件安全禁止规则：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.bulkDelete.approved",
        "已允许批量删除：当前请求累计删除 {count} 个条目，已达到或超过阈值 {threshold}",
    ),
    (
        "securityCenter.audit.fileSafety.bulkDelete.needsApproval",
        "批量删除需要确认：当前请求累计删除 {count} 个条目，已达到或超过阈值 {threshold}",
    ),
    (
        "securityCenter.audit.fileSafety.bulkDelete.rejected",
        "已拒绝批量删除：当前请求累计删除 {count} 个条目，已达到或超过阈值 {threshold}",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatched",
        "文件访问命中文件安全禁止规则，已阻止：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatched.delete",
        "文件删除命中文件安全禁止规则，已阻止：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatched.deleteWithRule",
        "文件删除命中文件安全禁止规则，已阻止：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatched.read",
        "文件读取命中文件安全禁止规则，已阻止：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatched.readWithRule",
        "文件读取命中文件安全禁止规则，已阻止：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatched.write",
        "文件写入命中文件安全禁止规则，已阻止：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatched.writeWithRule",
        "文件写入命中文件安全禁止规则，已阻止：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.denyMatchedWithRule",
        "文件访问命中文件安全禁止规则，已阻止：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.fallback",
        "文件安全事件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.needsApproval",
        "文件访问需要审批：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.needsApproval.delete",
        "文件删除需要审批：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.needsApproval.read",
        "文件读取需要审批：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.needsApproval.write",
        "文件写入需要审批：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.rejected",
        "用户已拒绝文件访问：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.rejected.delete",
        "用户已拒绝删除文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.rejected.deleteWithRule",
        "用户已拒绝删除文件：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.rejected.read",
        "用户已拒绝读取文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.rejected.readWithRule",
        "用户已拒绝读取文件：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.rejected.write",
        "用户已拒绝写入文件：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.rejected.writeWithRule",
        "用户已拒绝写入文件：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.rejectedWithRule",
        "用户已拒绝文件访问：{paths}（询问规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHit",
        "文件访问命中询问规则并使用会话放行缓存：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHit.delete",
        "文件删除命中询问规则并使用会话放行缓存：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHit.deleteWithRule",
        "文件删除命中询问规则并使用会话放行缓存：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHit.read",
        "文件读取命中询问规则并使用会话放行缓存：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHit.readWithRule",
        "文件读取命中询问规则并使用会话放行缓存：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHit.write",
        "文件写入命中询问规则并使用会话放行缓存：{paths}",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHit.writeWithRule",
        "文件写入命中询问规则并使用会话放行缓存：{paths}（规则：{matchedRule}）",
    ),
    (
        "securityCenter.audit.fileSafety.sessionCacheHitWithRule",
        "文件访问命中询问规则并使用会话放行缓存：{paths}（规则：{matchedRule}）",
    ),
];

/// 家族序（键按点号边界归组；`*WithRule` 归入去掉该后缀后的家族）。
const FAMILIES: &[&str] = &[
    "allowListed",
    "approved",
    "askMatched",
    "autoApproved",
    "blockListed",
    "bulkDelete",
    "denyMatched",
    "fallback",
    "needsApproval",
    "rejected",
    "sessionCacheHit",
];

/// 目录条目：家族 + i18n 键 + 逐字中文模板。
pub struct Entry {
    pub family: &'static str,
    pub key: &'static str,
    pub text: &'static str,
}

/// 键 → 家族（纯函数；点号边界保证 `approvedWithRule` 不误入 `approved.` 之外的家族）。
pub fn family_of(key: &str) -> &'static str {
    let local = key
        .strip_prefix(NS_PREFIX)
        .unwrap_or("")
        .trim_start_matches('.');
    let local = local.strip_suffix("WithRule").unwrap_or(local);
    FAMILIES
        .iter()
        .find(|f| {
            local == **f
                || local
                    .strip_prefix(**f)
                    .is_some_and(|rest| rest.starts_with('.'))
        })
        .copied()
        .unwrap_or("fallback")
}

/// 全目录（= [`TEXTS`] 的条目视图）。
pub fn entries() -> Vec<Entry> {
    TEXTS
        .iter()
        .map(|(key, text)| Entry {
            family: family_of(key),
            key,
            text,
        })
        .collect()
}

/// 家族分组（表序即分组序，每家族恰一组）。
pub struct Group {
    pub family: &'static str,
    pub entries: Vec<Entry>,
}

pub fn groups() -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for entry in entries() {
        match out.last_mut() {
            Some(g) if g.family == entry.family => g.entries.push(entry),
            _ => out.push(Group {
                family: entry.family,
                entries: vec![entry],
            }),
        }
    }
    out
}

/// 浮层/面板状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发且非空，正常呈现。
    Ready,
    /// 数据已下发但为空（空态，⛔ 不得渲染成空列表）。
    Empty,
    /// 数据未下发（无数据源时的诚实默认态；无独立失败通道，未拉取同样 fail-closed）。
    #[default]
    NotDelivered,
}

/// 运行时源快照（解析 [`State`] 的纯输入）。
pub enum Source<'a> {
    /// 未拉取（≠ 空集）。
    NotFetched,
    /// 已下发条目集（空集 ⇒ [`State::Empty`]）。
    Delivered(&'a [Entry]),
}

/// 由运行时输入解析初始态（纯函数）。
pub fn resolve(source: Source) -> State {
    match source {
        Source::NotFetched => State::NotDelivered,
        Source::Delivered([]) => State::Empty,
        Source::Delivered(_) => State::Ready,
    }
}

/// 状态行标签（卡内自有状态标签；内容文案见 [`TEXTS`]）。
pub fn state_label(state: &State) -> &'static str {
    match state {
        State::Ready => "就绪",
        State::Empty => EMPTY_HINT,
        State::NotDelivered => NOT_DELIVERED,
    }
}

// ===== GPUI 层 =====
/// 家族组节点：组名 + 逐字文案行（颜色一律 theme.*，⛔ 不硬编码色值）。
fn group_node(theme: &Theme, group: &Group) -> AnyElement {
    let mut node = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_color(theme.foreground).child(group.family));
    for entry in &group.entries {
        node = node.child(div().child(entry.text));
    }
    node.into_any_element()
}

/// 空态节点（空输入 ⇒ 单个空态节点，⛔ 不得渲染成空列表）。
fn empty_node(theme: &Theme) -> AnyElement {
    div()
        .text_color(theme.muted_foreground)
        .child(EMPTY_HINT)
        .into_any_element()
}

/// 主区/浮层呈现入口（签名不变）。
///
/// 内容节点 = 规格文案目录（静态、逐字，非运行时事件）；运行时审计事件流未接入 ⇒
/// 标题行如实标注「未下发」（fail-closed，⛔ 不伪造事件条目/数值）。
pub fn main_area(theme: &Theme) -> AnyElement {
    let events_state = resolve(Source::NotFetched);
    let column = div()
        .flex()
        .flex_col()
        .gap_3()
        .text_color(theme.muted_foreground)
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(format!(
                    "{}（{}）",
                    TITLE,
                    state_label(&events_state)
                ))),
        );
    // 目录空集 ⇒ 空态单节点；正常 ⇒ 逐组渲染（目录恒非空，分支由测试锁定）。
    let catalog = entries();
    let column = match resolve(Source::Delivered(&catalog)) {
        State::Empty => column.child(empty_node(theme)),
        _ => groups()
            .into_iter()
            .fold(column, |col, g| col.child(group_node(theme, &g))),
    };
    column.into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        entries, family_of, groups, main_area, resolve, state_label, Source, State, EMPTY_HINT,
        NOT_DELIVERED, NS_KEY_COUNT, NS_PREFIX, TEXTS, TITLE,
    };
    use gpui_kit::component::Theme;
    use std::collections::HashSet;

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "文件安全审计");
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    /// ① 纯数据层计数正确：68 键、唯一、全在命名空间内、模板非空。
    #[test]
    fn catalog_counts_hold() {
        assert_eq!(TEXTS.len(), NS_KEY_COUNT);
        assert_eq!(TEXTS.len(), 68);
        assert_eq!(entries().len(), NS_KEY_COUNT);
        let keys: Vec<&str> = TEXTS.iter().map(|(k, _)| *k).collect();
        let unique: HashSet<&str> = keys.iter().copied().collect();
        assert_eq!(keys.len(), unique.len(), "键必须唯一");
        assert!(keys.iter().all(|k| k.starts_with(NS_PREFIX)));
        assert!(TEXTS.iter().all(|(_, t)| !t.is_empty()));
    }

    /// ① 分组正确：11 家族各一组、表序、`*WithRule` 归家族。
    #[test]
    fn groups_are_wellformed() {
        let gs = groups();
        assert_eq!(gs.len(), 11);
        let families: Vec<&str> = gs.iter().map(|g| g.family).collect();
        let unique: HashSet<&str> = families.iter().copied().collect();
        assert_eq!(families.len(), unique.len(), "每家族恰一组");
        assert_eq!(gs.first().map(|g| g.family), Some("allowListed"));
        assert_eq!(gs.last().map(|g| g.family), Some("sessionCacheHit"));
        assert_eq!(
            family_of("securityCenter.audit.fileSafety.approvedWithRule"),
            "approved"
        );
        assert_eq!(
            family_of("securityCenter.audit.fileSafety.bulkDelete.rejected"),
            "bulkDelete"
        );
        assert_eq!(
            family_of("securityCenter.audit.fileSafety.fallback"),
            "fallback"
        );
        assert_eq!(family_of("whatever.else"), "fallback");
        let total: usize = gs.iter().map(|g| g.entries.len()).sum();
        assert_eq!(total, NS_KEY_COUNT);
    }

    /// ③ 文案逐字一致（抽 5 键断言，覆盖 4 类占位符组合）。
    #[test]
    fn texts_are_verbatim() {
        let get = |needle: &str| {
            TEXTS
                .iter()
                .find(|(key, _)| *key == needle)
                .map(|(_, text)| *text)
        };
        assert_eq!(
            get("securityCenter.audit.fileSafety.allowListed"),
            Some("文件访问命中文件安全允许规则：{paths}")
        );
        assert_eq!(
            get("securityCenter.audit.fileSafety.approved.deleteWithRule"),
            Some("用户已批准删除文件：{paths}（询问规则：{matchedRule}）")
        );
        assert_eq!(
            get("securityCenter.audit.fileSafety.denyMatchedWithRule"),
            Some("文件访问命中文件安全禁止规则，已阻止：{paths}（规则：{matchedRule}）")
        );
        assert_eq!(
            get("securityCenter.audit.fileSafety.bulkDelete.needsApproval"),
            Some("批量删除需要确认：当前请求累计删除 {count} 个条目，已达到或超过阈值 {threshold}")
        );
        assert_eq!(
            get("securityCenter.audit.fileSafety.sessionCacheHit.writeWithRule"),
            Some("文件写入命中询问规则并使用会话放行缓存：{paths}（规则：{matchedRule}）")
        );
    }

    /// ② 空输入 ⇒ 空态（不是就绪、不是空列表、不与未下发混同）。
    #[test]
    fn empty_input_yields_empty_state() {
        assert_eq!(resolve(Source::Delivered(&[])), State::Empty);
        assert_ne!(resolve(Source::Delivered(&[])), State::Ready);
        assert_eq!(state_label(&State::Empty), EMPTY_HINT);
        assert_ne!(EMPTY_HINT, NOT_DELIVERED);
    }

    /// ⑤ 状态分支覆盖：三态各一条 + 默认态 fail-closed。
    #[test]
    fn state_branches_covered() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(Source::NotFetched), State::NotDelivered);
        assert_eq!(state_label(&State::NotDelivered), NOT_DELIVERED);

        let catalog = entries();
        assert_eq!(resolve(Source::Delivered(&catalog)), State::Ready);
        assert_eq!(state_label(&State::Ready), "就绪");

        assert_eq!(resolve(Source::Delivered(&[])), State::Empty);
        assert_eq!(state_label(&State::Empty), EMPTY_HINT);
    }

    /// ④ main_area(&Theme::default()) 只构造元素树，不 panic。
    #[test]
    fn main_area_builds_without_panic() {
        let _ = main_area(&Theme::default());
    }

    /// 附加：条目视图三元组完整（family/key/text 逐字可达）。
    #[test]
    fn entry_fields_complete() {
        let first = &entries()[0];
        assert_eq!(first.family, "allowListed");
        assert_eq!(first.key, "securityCenter.audit.fileSafety.allowListed");
        assert!(first.text.contains("{paths}"));
    }
}
