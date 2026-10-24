//! 安全中心·审计配置（波次 9，卡 `ui-overlay-security-audit-config`）。
//!
//! 纯数据层：`securityCenter.audit.config` 逐字文案表 + 视图态状态机。
//! 硬约束：文案与本卡任务书逐字摘录一致（禁止自编中文，其余键待全表入库后补齐）；
//! 未拉取过 ≠ 空集（默认加载中）；失败 fail-closed 不退化成空列表。
//! GPUI 层只做四分支映射，颜色一律取 theme token。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 面板标题（逐字取自文案全表）。
pub const PANEL_TITLE: &str = "安全中心·审计配置";

/// 命名空间前缀。
pub const NS_PREFIX: &str = "securityCenter.audit.config";

/// 命名空间键数（据 `docs/ui-parity-spec.md` 计数 47；全表未入库）。
pub const NS_KEY_COUNT: usize = 47;

/// 本表条数 = 本卡任务书逐字摘录条数（其余键待补齐，禁止自编）。
pub const NS_QUOTED_COUNT: usize = 12;

/// 命名空间文案（键，中文模板）：本卡任务书逐字摘录 12 条。
pub const TEXTS: &[(&str, &str)] = &[
    (
        "securityCenter.audit.config.allowedDomainsUpdated",
        "网络访问允许域名已更新",
    ),
    (
        "securityCenter.audit.config.auditExported",
        "审计日志已导出",
    ),
    (
        "securityCenter.audit.config.auditRetentionPruned",
        "审计日志清理完成：删除 {count} 个历史分片",
    ),
    (
        "securityCenter.audit.config.commandSafetyAllow.cleared",
        "{listLabel}已清空",
    ),
    (
        "securityCenter.audit.config.commandSafetyAllow.reset",
        "{listLabel}已重置为默认",
    ),
    (
        "securityCenter.audit.config.commandSafetyAllow.updatedNoCount",
        "{listLabel}已更新",
    ),
    (
        "securityCenter.audit.config.commandSafetyAllow.updatedWithCount",
        "{listLabel}已更新：{count} 条规则",
    ),
    (
        "securityCenter.audit.config.commandSafetyAsk.cleared",
        "{listLabel}已清空",
    ),
    (
        "securityCenter.audit.config.commandSafetyAsk.reset",
        "{listLabel}已重置为默认",
    ),
    (
        "securityCenter.audit.config.commandSafetyAsk.updatedNoCount",
        "{listLabel}已更新",
    ),
    (
        "securityCenter.audit.config.commandSafetyAsk.updatedWithCount",
        "{listLabel}已更新：{count} 条规则",
    ),
    (
        "securityCenter.audit.config.deniedDomainsUpdated",
        "网络访问禁止域名已更新",
    ),
];

/// 渲染占位：数据未接入时必须可见（fail-closed），接线层兼容保留。
pub fn render_placeholder() -> String {
    format!("{}：数据未接入（fail-closed）", PANEL_TITLE)
}

/// 正常态数据：审计配置摘要（域名清单 + 命令安全清单 + 历史分片数）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AuditConfigSummary {
    pub allowed_domains: Vec<String>,
    pub denied_domains: Vec<String>,
    pub command_safety_allow: Vec<String>,
    pub command_safety_ask: Vec<String>,
    pub retention_shards: usize,
}

impl AuditConfigSummary {
    /// 已拉取但全部清单为空且无历史分片 → 空态（未拉取过 ≠ 空集，空态单独成支）。
    pub fn is_empty(&self) -> bool {
        self.allowed_domains.is_empty()
            && self.denied_domains.is_empty()
            && self.command_safety_allow.is_empty()
            && self.command_safety_ask.is_empty()
            && self.retention_shards == 0
    }
}

/// 视图态状态机：加载中 / 失败（fail-closed）/ 空态 / 正常。默认必须是加载中。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ViewState {
    #[default]
    Loading,
    Failed {
        message: String,
    },
    Empty,
    Active {
        summary: AuditConfigSummary,
    },
}

/// 由拉取结果解析四态（纯函数）：未拉取 → 加载中；失败 → 可见失败态；
/// 成功但摘要为空 → 空态（不是沉默的零行列表）；否则正常。
pub fn resolve(fetched: Option<Result<AuditConfigSummary, String>>) -> ViewState {
    match fetched {
        None => ViewState::Loading,
        Some(Err(message)) => ViewState::Failed { message },
        Some(Ok(summary)) if summary.is_empty() => ViewState::Empty,
        Some(Ok(summary)) => ViewState::Active { summary },
    }
}

/// 配置面板数据承载（fail-closed：默认未拉取 ⇒ 加载中，不伪造数据）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Feed {
    pub state: ViewState,
}

impl Feed {
    /// 未拉取（生产当前态：数据源在运行时侧，就绪后换源）。
    pub fn undelivered() -> Self {
        Self {
            state: ViewState::Loading,
        }
    }

    /// 拉取失败（fail-closed，携带原始错误信息）。
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            state: ViewState::Failed {
                message: message.into(),
            },
        }
    }

    /// 拉取成功：按摘要是否为空解析空态 / 正常。
    pub fn delivered(summary: AuditConfigSummary) -> Self {
        Self {
            state: resolve(Some(Ok(summary))),
        }
    }
}

/// 文案查找（纯函数；未知键返回 None，不伪造模板）。
pub fn audit_config_copy(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, t)| *t)
}

/// 占位参数：`{count}` / `{listLabel}`。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CopyParams<'a> {
    pub count: Option<usize>,
    pub list_label: Option<&'a str>,
}

/// 占位替换（纯函数；未提供的占位原样保留）。
pub fn format_copy(text: &str, params: CopyParams<'_>) -> String {
    let mut out = text.to_string();
    if let Some(count) = params.count {
        out = out.replace("{count}", &count.to_string());
    }
    if let Some(label) = params.list_label {
        out = out.replace("{listLabel}", label);
    }
    out
}

/// 按键取模板并填占位（纯函数；未知键返回 None，调用方回显键名，不自编文案）。
pub fn action_feedback(key: &str, params: CopyParams<'_>) -> Option<String> {
    audit_config_copy(key).map(|t| format_copy(t, params))
}

// ===== GPUI 层 =====
/// 数据未接入时的可见说明（fail-closed；沿用占位期措辞，⛔ 不退化成空列表）。
pub const NOT_DELIVERED: &str = "数据未接入（fail-closed）";

/// 空态文案：本卡摘录未覆盖空态键，按仓库空态惯例「暂无+面名」
/// （同 `security_cmd_safety::EMPTY_TEXT` 的处理）。
pub const EMPTY_TEXT: &str = "暂无审计配置";

/// 四分支状态渲染（fail-closed：失败可见；空态非沉默；未拉取不加载数据）。
pub fn render_state(theme: &Theme, feed: &Feed) -> AnyElement {
    let col = div().flex().flex_col().gap_2().text_sm();
    let col = match &feed.state {
        ViewState::Loading => col.child(SharedString::from(format!(
            "{NS_PREFIX} · {NS_QUOTED_COUNT}/{NS_KEY_COUNT}"
        ))),
        ViewState::Failed { message } => col
            .child(
                div()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child(SharedString::from(NOT_DELIVERED)),
            )
            .child(SharedString::from(message.clone())),
        ViewState::Empty => col.child(SharedString::from(EMPTY_TEXT)),
        ViewState::Active { summary } => col.child(render_summary(theme, summary)),
    };
    col.into_any_element()
}

/// 正常态：四个清单分区 + 历史分片计数行。
fn render_summary(theme: &Theme, summary: &AuditConfigSummary) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(render_section(
            theme,
            "allowedDomains",
            &summary.allowed_domains,
        ))
        .child(render_section(
            theme,
            "deniedDomains",
            &summary.denied_domains,
        ))
        .child(render_section(
            theme,
            "commandSafetyAllow",
            &summary.command_safety_allow,
        ))
        .child(render_section(
            theme,
            "commandSafetyAsk",
            &summary.command_safety_ask,
        ))
        .child(div().text_sm().child(SharedString::from(format!(
            "retentionShards · {}",
            summary.retention_shards
        ))))
}

/// 摘要分区：ASCII 键段做区头 + 计数 + 规则行（⛔ 不自编中文组名）。
fn render_section(theme: &Theme, header: &str, rules: &[String]) -> Div {
    let section = div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(SharedString::from(format!("{header} · {}", rules.len()))),
        );
    rules.iter().fold(section, |acc, rule| {
        acc.child(div().text_sm().child(SharedString::from(rule.clone())))
    })
}

/// 逐字模板目录（真实参考数据，非伪造条目）：命名空间 + 键 + 中文模板。
pub fn render_catalog(theme: &Theme) -> AnyElement {
    let group = div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(format!(
            "{NS_PREFIX} · {NS_QUOTED_COUNT}/{NS_KEY_COUNT}"
        )));
    TEXTS
        .iter()
        .fold(group, |acc, (k, t)| {
            acc.child(SharedString::from(format!("{k} {t}")))
        })
        .into_any_element()
}

/// 浮层呈现入口：标题行 + 状态分支（无数据源 ⇒ 加载中，不伪造）+ 模板目录。
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
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(SharedString::from(PANEL_TITLE)),
        )
        .child(render_state(theme, &feed))
        .child(render_catalog(theme))
        .into_any_element()
}
