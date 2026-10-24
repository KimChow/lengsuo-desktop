//! 安全中心·审计配置（波次 9，卡 `ui-overlay-security-audit-config`）。
//!
//! 纯数据层：`securityCenter.audit.config` 逐字文案表 + 视图态状态机。
//! 硬约束：文案与本卡任务书逐字摘录一致（禁止自编中文，其余键待全表入库后补齐）；
//! 未拉取过 ≠ 空集（默认加载中）；失败 fail-closed 不退化成空列表。

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
