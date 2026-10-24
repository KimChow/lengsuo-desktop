//! 协作计划·新建（波次 9，卡 `ui-overlay-collab-plan-create`）。
//!
//! 分层约定（同 `panel_views` / `home_schedule_create`）：纯数据层（文案表 + 状态机 +
//! 纯函数）+ GPUI 层（`main_area` 只做映射）。硬约束：fail-closed；未拉取过 ≠ 空集
//! （默认态 [`State::Loading`]）；⛔ 不伪造服务端数据；文案逐字取自命名空间
//! `collab.plan.create`（全表 34 有效键；本卡落逐字摘录的 12 键，其余键待全表下发后补）。
#![cfg(feature = "ui")]

/// 面板标题（逐字取自文案全表）。
pub const PANEL_TITLE: &str = "协作计划·新建";
/// 数据未下发时的如实标注（fail-closed；⛔ 不得替换为空列表/伪造计数）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀与全表键数（据文案全表，共 34 有效键；本卡先落逐字摘录子集）。
pub const NS_PREFIX: &str = "collab.plan.create";
pub const NS_KEY_COUNT: usize = 34;

// ===== 纯数据层（无 GPUI 依赖）=====

/// 文案表（key → 中文，逐字取自文案全表命名空间 `collab.plan.create`；⛔ 不自编中文）。
// rustfmt::skip：中文按 2 列计宽，逐键单行才不会被拆成三行。
#[rustfmt::skip]
pub const COPY: &[(&str, &str)] = &[
    ("collab.plan.create.addAttachmentAria", "添加附件"),
    ("collab.plan.create.addLibraryAria", "添加资料"),
    ("collab.plan.create.aria.todoAttributes", "待办属性"),
    ("collab.plan.create.assigneeFallback", "处理人"),
    ("collab.plan.create.assigneeMulti", "{first} 等 {count} 人"),
    ("collab.plan.create.attachmentFallbackName", "附件"),
    ("collab.plan.create.attachmentPendingUpload", "{name}（待上传）"),
    ("collab.plan.create.attachmentRemoveAria", "移除附件"),
    ("collab.plan.create.attachmentSectionTitle", "附件 {count}"),
    ("collab.plan.create.attachmentSource.local", "本地文件"),
    ("collab.plan.create.attachmentSource.netdisk", "项目资产"),
    ("collab.plan.create.collapseAria", "恢复默认高度"),
];

/// 视图态状态机（默认 [`State::Loading`]：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 加载中：数据位一律「未下发」，⛔ 不显示伪造的 0 计数。
    #[default]
    Loading,
    /// 拉取失败（fail-closed，必须如实标错，不退化成空列表）。
    LoadFailed,
    /// 数据已下发但无草稿内容：空态（逐字空态提示，不是空白）。
    Empty,
    /// 正常态：草稿数据逐项呈现。
    Normal(PlanDraft),
}

/// 新建草稿（纯数据）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanDraft {
    /// 处理人名；空 ⇒ `assigneeFallback` 空态。
    pub assignees: Vec<String>,
    /// 附件条目；`{count}` 取真实长度（⛔ 不伪造计数）。
    pub attachments: Vec<Attachment>,
}

/// 附件条目（纯数据）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    /// 展示名；`None` ⇒ `attachmentFallbackName`。
    pub name: Option<String>,
    /// 来源（`attachmentSource.*`）。
    pub source: AttachmentSource,
    /// 待上传 ⇒ `attachmentPendingUpload` 模板。
    pub pending_upload: bool,
}

/// 附件来源（`attachmentSource.local` / `attachmentSource.netdisk`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttachmentSource {
    /// `attachmentSource.local`
    #[default]
    Local,
    /// `attachmentSource.netdisk`
    Netdisk,
}

/// 行输出：有值 / 空态（⛔ 不是空列表）/ 未下发。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Value(String),
    /// 空态提示（逐字文案）。
    Empty(&'static str),
    /// fail-closed：数据未下发。
    NotDelivered,
}

/// 由运行时加载结果解析视图态（纯函数）：
/// `None` = 加载中；`Err` = 失败；`Ok(None)` = 空态；`Ok(Some)` = 正常。
pub fn resolve(load: Option<Result<Option<PlanDraft>, ()>>) -> State {
    match load {
        None => State::Loading,
        Some(Err(())) => State::LoadFailed,
        Some(Ok(None)) => State::Empty,
        Some(Ok(Some(draft))) => State::Normal(draft),
    }
}

/// 文案查键（纯函数；键须在 [`COPY`] 表内且属本命名空间）。
pub fn copy(key: &str) -> Option<&'static str> {
    let dotted = format!("{NS_PREFIX}.");
    debug_assert!(key == NS_PREFIX || key.starts_with(&dotted));
    debug_assert!(COPY.len() <= NS_KEY_COUNT);
    COPY.iter().find(|(k, _)| *k == key).map(|(_, t)| *t)
}

/// 模板填槽：`{first}`/`{count}`/`{name}` ← args（纯函数）。
fn fill(tpl: &str, args: &[(&str, &str)]) -> String {
    let mut out = tpl.to_string();
    for (slot, val) in args {
        out = out.replace(&format!("{{{slot}}}"), val);
    }
    out
}

impl AttachmentSource {
    /// 来源文案（逐字）。
    pub fn label(self) -> &'static str {
        let key = match self {
            AttachmentSource::Local => "collab.plan.create.attachmentSource.local",
            AttachmentSource::Netdisk => "collab.plan.create.attachmentSource.netdisk",
        };
        copy(key).expect("missing copy key")
    }
}

/// 处理人行（纯函数）：0 人 ⇒ `assigneeFallback` 空态；1 人 ⇒ 该成员；多人 ⇒ `assigneeMulti`。
pub fn assignee_row(assignees: &[String]) -> Row {
    match assignees {
        [] => Row::Empty(copy("collab.plan.create.assigneeFallback").expect("missing copy key")),
        [only] => Row::Value(only.clone()),
        [first, ..] => Row::Value(fill(
            copy("collab.plan.create.assigneeMulti").expect("missing copy key"),
            &[
                ("first", first.as_str()),
                ("count", &assignees.len().to_string()),
            ],
        )),
    }
}

/// 附件展示名（纯函数）：无名 ⇒ `attachmentFallbackName`；待上传 ⇒ `attachmentPendingUpload`。
pub fn attachment_display(attachment: &Attachment) -> String {
    let fallback = copy("collab.plan.create.attachmentFallbackName").expect("missing copy key");
    let name = attachment.name.as_deref().unwrap_or(fallback);
    if attachment.pending_upload {
        fill(
            copy("collab.plan.create.attachmentPendingUpload").expect("missing copy key"),
            &[("name", name)],
        )
    } else {
        name.to_string()
    }
}

/// 附件区标题（纯函数）：`attachmentSectionTitle`，`{count}` = 真实计数（⛔ 不伪造）。
pub fn attachment_section_title(count: usize) -> String {
    fill(
        copy("collab.plan.create.attachmentSectionTitle").expect("missing copy key"),
        &[("count", &count.to_string())],
    )
}
