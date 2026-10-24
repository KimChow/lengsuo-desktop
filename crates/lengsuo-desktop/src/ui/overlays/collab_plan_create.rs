//! 协作计划·新建（波次 9，卡 `ui-overlay-collab-plan-create`）。
//!
//! 分层约定（同 `panel_views` / `home_schedule_create`）：纯数据层（文案表 + 状态机 +
//! 纯函数）+ GPUI 层（`main_area` 只做映射）。硬约束：fail-closed；未拉取过 ≠ 空集
//! （默认态 [`State::Loading`]）；⛔ 不伪造服务端数据；文案逐字取自命名空间
//! `collab.plan.create`（全表 34 有效键；本卡落逐字摘录的 12 键，其余键待全表下发后补）。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

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

// ===== GPUI 层 =====

/// 渲染模型里的附件条目行。
#[derive(Debug, PartialEq)]
pub struct AttachmentRow {
    /// 展示名（真名 / `attachmentFallbackName`；待上传 ⇒ `attachmentPendingUpload`）。
    pub name: String,
    /// 来源（`attachmentSource.*`，逐字）。
    pub source: &'static str,
    /// 移除入口（`attachmentRemoveAria`，逐字）。
    pub remove: &'static str,
}

/// 由 [`State`] 纯函数映射出的渲染模型（GPUI 层唯一输入）。
#[derive(Debug, PartialEq)]
pub struct ViewModel {
    pub title: &'static str,
    /// 折叠入口（`collapseAria`，逐字）。
    pub collapse: &'static str,
    /// 待办属性区标签（`aria.todoAttributes`，逐字）。
    pub attributes_label: &'static str,
    /// 处理人行。
    pub assignee: Row,
    /// 附件区标题（`attachmentSectionTitle` 真实计数；加载/失败 ⇒ 未下发）。
    pub attachment_title: Row,
    /// 附件条目行。
    pub attachments: Vec<AttachmentRow>,
    /// 添加附件入口（`addAttachmentAria`，逐字）。
    pub add_attachment: &'static str,
    /// 添加资料入口（`addLibraryAria`，逐字）。
    pub add_library: &'static str,
}

/// 状态 → 渲染模型（纯函数；GPUI 层唯一输入）。
pub fn view_model(state: &State) -> ViewModel {
    let c = |key: &str| copy(key).expect("missing copy key");
    let chrome = ViewModel {
        title: PANEL_TITLE,
        collapse: c("collab.plan.create.collapseAria"),
        attributes_label: c("collab.plan.create.aria.todoAttributes"),
        assignee: Row::NotDelivered,
        attachment_title: Row::NotDelivered,
        attachments: vec![],
        add_attachment: c("collab.plan.create.addAttachmentAria"),
        add_library: c("collab.plan.create.addLibraryAria"),
    };
    match state {
        // 加载中/拉取失败：fail-closed。数据位一律「未下发」，⛔ 不伪造空态/0 计数。
        State::Loading | State::LoadFailed => chrome,
        // 空态：已下发但无草稿——逐字空态提示，不是空白。
        State::Empty => ViewModel {
            assignee: assignee_row(&[]),
            attachment_title: Row::Value(attachment_section_title(0)),
            ..chrome
        },
        // 正常态：草稿逐项落到行，`{count}` 取真实计数。
        State::Normal(draft) => ViewModel {
            assignee: assignee_row(&draft.assignees),
            attachment_title: Row::Value(attachment_section_title(draft.attachments.len())),
            attachments: draft
                .attachments
                .iter()
                .map(|a| AttachmentRow {
                    name: attachment_display(a),
                    source: a.source.label(),
                    remove: c("collab.plan.create.attachmentRemoveAria"),
                })
                .collect(),
            ..chrome
        },
    }
}

/// 主区/浮层呈现入口。无数据源 ⇒ `resolve(None)` 如实落到加载中（fail-closed，⛔ 不伪造条目）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render(theme, &view_model(&resolve(None)))
}

/// 渲染模型 → 元素树（纯构建，不持实体/凭据；颜色一律主题 token，⛔ 不硬编码色值）。
fn render(theme: &Theme, vm: &ViewModel) -> AnyElement {
    // 附件区：标题（真实计数/未下发）→ 条目 → 添加入口。
    let mut attachments = div()
        .flex()
        .flex_col()
        .gap_1()
        .px_4()
        .child(attachment_header_view(theme, &vm.attachment_title));
    for row in &vm.attachments {
        attachments = attachments.child(attachment_view(theme, row));
    }
    let attachments = attachments.child(
        div()
            .flex()
            .flex_row()
            .gap_2()
            .child(action_view(theme, vm.add_attachment))
            .child(action_view(theme, vm.add_library)),
    );
    div()
        .flex()
        .flex_col()
        .gap_2()
        // 头部：标题 + 恢复默认高度入口。
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px_4()
                .py_2()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(theme.foreground)
                        .child(vm.title),
                )
                .child(collapse_view(theme, vm.collapse)),
        )
        // 待办属性区：处理人行。
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .px_4()
                .child(section_label(theme, vm.attributes_label))
                .child(row_view(theme, &vm.assignee)),
        )
        .child(attachments)
        .into_any_element()
}

/// 分区标签（弱化小字）。
fn section_label(theme: &Theme, label: &str) -> Div {
    div()
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(label.to_string())
}

/// 单行内容：`Value` 前景 / `Empty` 弱化占位 / `NotDelivered` 以警示色如实标出。
fn row_view(theme: &Theme, row: &Row) -> Div {
    let (text, color) = match row {
        Row::Value(v) => (v.clone(), theme.foreground),
        Row::Empty(hint) => ((*hint).to_string(), theme.muted_foreground),
        Row::NotDelivered => (NOT_DELIVERED.to_string(), theme.danger),
    };
    div().text_sm().text_color(color).child(text)
}

/// 附件区标题行：正常 ⇒ 弱化标题（真实计数）；未下发 ⇒ 警示色如实标出（⛔ 不伪造 0 计数）。
fn attachment_header_view(theme: &Theme, row: &Row) -> Div {
    let (text, color) = match row {
        Row::Value(v) => (v.clone(), theme.muted_foreground),
        Row::Empty(hint) => ((*hint).to_string(), theme.muted_foreground),
        Row::NotDelivered => (NOT_DELIVERED.to_string(), theme.danger),
    };
    div().text_xs().text_color(color).child(text)
}

/// 附件条目行：展示名 + 来源；移除入口触达目标 ≥48dp。
fn attachment_view(theme: &Theme, row: &AttachmentRow) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_2()
        .text_sm()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(div().text_color(theme.foreground).child(row.name.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(row.source),
                ),
        )
        .child(
            div()
                .h(px(48.))
                .flex()
                .items_center()
                .text_color(theme.muted_foreground)
                .child(row.remove),
        )
}

/// 添加入口（添加附件/添加资料）：触达目标 ≥48dp。
fn action_view(theme: &Theme, label: &str) -> Div {
    div()
        .h(px(48.))
        .flex()
        .items_center()
        .px_4()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(theme.foreground)
        .child(label.to_string())
}

/// 恢复默认高度入口：触达目标 ≥48dp。
fn collapse_view(theme: &Theme, label: &str) -> Div {
    div()
        .h(px(48.))
        .flex()
        .items_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(label.to_string())
}
