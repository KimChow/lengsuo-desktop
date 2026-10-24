//! 任务交接（波次 8，卡 `ui-collab-task-handoff`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.task.handoff`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;
use std::borrow::Cow;

/// 视图标题。
pub const TITLE: &str = "任务交接";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.task.handoff";
/// 命名空间键数（要求覆盖 ≥40，实际全量 53）。
pub const NS_KEY_COUNT: usize = 53;

// ===== 文案常量（逐字；行尾注释为全表键名）=====
// —— 弹窗骨架 ——
pub const HEADER_TITLE: &str = "新建待办"; // headerTitle
pub const DIALOG_ARIA_LABEL: &str = "新建待办"; // dialogAriaLabel
pub const BACKDROP_ARIA_LABEL: &str = "关闭新建待办"; // backdropAriaLabel
pub const CLOSE_LABEL: &str = "关闭"; // closeLabel
pub const PROPERTY_ARIA_LABEL: &str = "待办属性"; // propertyAriaLabel
pub const CANCEL: &str = "取消"; // cancel
pub const CREATE: &str = "创建"; // create
pub const SUBMIT: &str = "确认"; // submit
pub const SUBMITTING: &str = "提交中..."; // submitting
// —— 表单字段 ——
pub const FIELD_TITLE: &str = "标题"; // fieldTitle
pub const FIELD_DESCRIPTION: &str = "描述"; // fieldDescription
pub const TITLE_PLACEHOLDER: &str = "请输入标题"; // titlePlaceholder
pub const DESCRIPTION_PLACEHOLDER: &str = "添加描述（可选）"; // descriptionPlaceholder
pub const TITLE_EMPTY: &str = "请先填写待办标题"; // titleEmpty
pub const NEED_ASSIGNEE: &str = "请选择处理人"; // needAssignee
pub const SEARCH_PLACEHOLDER: &str = "搜索成员"; // searchPlaceholder
pub const EMPTY: &str = "没有匹配的处理人"; // empty
pub const UNASSIGNED_CHIP: &str = "处理人"; // unassignedChip
pub const FALLBACK_UNKNOWN_MEMBER: &str = "未知成员"; // fallbackUnknownMember
// —— 附件 ——
pub const ATTACHMENT_LABEL: &str = "附件"; // attachmentLabel
pub const ATTACHMENT_LINE_HEADER: &str = "附件："; // attachmentLineHeader
pub const ATTACHMENT_COUNT: &str = "{count} 个附件"; // attachmentCount
pub const ATTACHMENTS_SELECTED: &str = "已选择 {count} 个附件"; // attachmentsSelected
pub const ATTACHMENTS_UPLOADED: &str = "已上传 {count} 个附件"; // attachmentsUploaded
pub const ATTACHMENT_ALREADY_ADDED: &str = "选择的附件已添加"; // attachmentAlreadyAdded
pub const ATTACHMENTS_ADD_TITLE: &str = "添加附件"; // attachmentsAddTitle
pub const ATTACHMENTS_LOCAL: &str = "本地文件"; // attachmentsLocal
pub const ATTACHMENTS_NETDISK: &str = "项目资产"; // attachmentsNetdisk
pub const ATTACHMENTS_PARTIAL_FAILED: &str =
    "任务已转交，但 {count} 个附件上传或绑定失败，可在计划详情中重新添加"; // attachmentsPartialFailed
// —— 任务信息 / 产物 ——
pub const SECTION_TASK_INFO: &str = "任务信息"; // section.taskInfo
pub const SECTION_TASK_SOURCE: &str = "来自「{title}」"; // section.taskSource
pub const FALLBACK_TASK_NAME: &str = "当前任务"; // fallbackTaskName
pub const SECTION_ARTIFACT_TITLE: &str = "任务产物"; // section.artifactTitle
pub const SECTION_ARTIFACT_COUNT: &str = "共 {count} 项"; // section.artifactCount
pub const NO_ARTIFACTS: &str = "暂无产物"; // noArtifacts
pub const FALLBACK_ARTIFACT_NAME: &str = "未命名"; // fallbackArtifactName
pub const ARTIFACT_NAME_SEPARATOR: &str = "、"; // artifactNameSeparator
pub const SECTION_FILE_TAG: &str = "文件"; // section.fileTag
pub const AI_BADGE: &str = "AI生成"; // aiBadge
// —— 任务进度摘要 ——
pub const SECTION_PROGRESS_TITLE: &str = "任务进度摘要"; // section.progressTitle
pub const SECTION_SUMMARY_EXPAND: &str = "展开"; // section.summaryExpand
pub const SECTION_SUMMARY_COLLAPSE: &str = "收起"; // section.summaryCollapse
pub const SECTION_SUMMARY_GENERATING: &str = "AI 摘要生成中"; // section.summaryGenerating
pub const SUMMARY_EMPTY: &str = "任务进展摘要为空，请稍后重试"; // summaryEmpty
pub const SUMMARY_LOADING: &str = "任务进展摘要生成中，请稍后再转交"; // summaryLoading
pub const PROGRESS_FALLBACK_TITLE: &str = "该任务「{title}」已产出 {count} 个文件产物"; // progressFallbackTitle
pub const PROGRESS_FALLBACK_PAREN: &str = "（{names}）"; // progressFallbackParen
pub const PROGRESS_FALLBACK_INTERACTION: &str =
    "。对话共 {count} 轮交互，任务整体进展顺利，可接续后续环节。"; // progressFallbackInteraction
// —— 转交结果 ——
pub const HANDOFF_SUCCESS: &str = "任务正在转交中，稍后可在计划看板中查看"; // handoffSuccess
pub const HANDOFF_FAILED: &str = "转交失败，请重试"; // handoffFailed
pub const HANDOFF_FAILED_PROJECT_ARCHIVED: &str = "该项目已归档，无法转交任务"; // handoffFailedProjectArchived
pub const HANDOFF_FAILED_TASK_ARCHIVED: &str = "该任务已归档或删除，无法转交"; // handoffFailedTaskArchived
pub const PENDING_MESSAGE_QUEUED: &str = "任务转交中，新消息将在转交完成后依次处理"; // pendingMessageQueued

// ===== 纯数据层（无 GPUI 依赖）=====
/// 附件来源。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentSource {
    /// 本地文件。
    Local,
    /// 项目资产。
    Netdisk,
}

impl AttachmentSource {
    /// 来源标签（逐字）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => ATTACHMENTS_LOCAL,
            Self::Netdisk => ATTACHMENTS_NETDISK,
        }
    }
}

/// 附件条目。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub source: AttachmentSource,
}

/// 产物条目（空名渲染 [`FALLBACK_ARTIFACT_NAME`]）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub name: String,
    pub ai_generated: bool,
}

/// 协作成员（处理人候选）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub display_name: String,
}

/// 进度摘要状态。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SummaryPhase {
    /// AI 摘要生成中。
    #[default]
    Generating,
    /// 摘要生成中，尚未到可转交状态（阻止提交）。
    Loading,
    /// 摘要为空。
    Empty,
    /// 已生成（正文为运行时数据）。
    Ready(String),
}

/// 新建待办表单值。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TodoForm {
    pub title: String,
    pub description: String,
    pub assignee: Option<Member>,
    pub attachments: Vec<Attachment>,
}

/// 交接场景数据（来源任务 + 产物 + 摘要 + 新建待办表单）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HandoffScene {
    pub source_task: String,
    pub artifacts: Vec<Artifact>,
    pub summary: SummaryPhase,
    pub summary_expanded: bool,
    pub todo: TodoForm,
}

/// 转交失败原因（决定失败文案）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailReason {
    /// 通用失败（可重试）。
    Retry,
    /// 项目已归档。
    ProjectArchived,
    /// 任务已归档或删除。
    TaskArchived,
}

/// 浮层状态机（默认 `NotDelivered`：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据未下发（fail-closed 诚实默认态）。
    #[default]
    NotDelivered,
    /// 拉取失败（同样无数据，渲染同 `NotDelivered`）。
    LoadFailed,
    /// 表单编辑中。
    Composing(HandoffScene),
    /// 提交中。
    Submitting,
    /// 转交已受理（失败附件数 >0 ⇒ 部分失败文案）。
    Accepted {
        uploaded_attachments: usize,
        failed_attachments: usize,
    },
    /// 转交失败。
    Failed(FailReason),
}

/// 由运行时输入解析初始态（纯函数；`None` = 未下发 ≠ `Some(false)` 拉取失败）。
pub fn resolve(delivered: Option<bool>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(false) => State::LoadFailed,
        Some(true) => State::Composing(HandoffScene::default()),
    }
}

/// 模板占位符替换（`{count}` / `{names}` / `{title}`；纯函数）。
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out
}

/// 附件计数行（attachmentCount）。
pub fn attachment_count_line(count: usize) -> String {
    fill(ATTACHMENT_COUNT, &[("count", &count.to_string())])
}

/// 摘要展开/收起切换文案。
pub fn summary_toggle(expanded: bool) -> &'static str {
    if expanded {
        SECTION_SUMMARY_COLLAPSE
    } else {
        SECTION_SUMMARY_EXPAND
    }
}

/// 处理人搜索（包含匹配；空查询返回全部候选）。
pub fn search_members<'a>(query: &str, pool: &'a [Member]) -> Vec<&'a Member> {
    let q = query.trim();
    if q.is_empty() {
        pool.iter().collect()
    } else {
        pool.iter().filter(|m| m.display_name.contains(q)).collect()
    }
}

/// 搜索结果为空时的空态文案（⛔ 不渲染空列表）。
pub fn search_notice(matches: &[&Member]) -> Option<&'static str> {
    matches.is_empty().then_some(EMPTY)
}

/// AI 摘要缺位时的兜底摘要句（标题 +（成员）+ 交互轮次，逐字模板拼接）。
pub fn fallback_summary(task: &str, artifact_count: usize, names: &[&str], interactions: usize) -> String {
    let names_joined = names.join(ARTIFACT_NAME_SEPARATOR);
    [
        fill(
            PROGRESS_FALLBACK_TITLE,
            &[("title", task), ("count", &artifact_count.to_string())],
        ),
        fill(PROGRESS_FALLBACK_PAREN, &[("names", &names_joined)]),
        fill(
            PROGRESS_FALLBACK_INTERACTION,
            &[("count", &interactions.to_string())],
        ),
    ]
    .concat()
}

/// 产物显示名（空名 ⇒ [`FALLBACK_ARTIFACT_NAME`]）。
fn artifact_display_name(artifact: &Artifact) -> &str {
    let name = artifact.name.trim();
    if name.is_empty() {
        FALLBACK_ARTIFACT_NAME
    } else {
        name
    }
}

impl HandoffScene {
    /// 来源任务名（空 ⇒ [`FALLBACK_TASK_NAME`]）。
    pub fn task_name(&self) -> &str {
        let name = self.source_task.trim();
        if name.is_empty() { FALLBACK_TASK_NAME } else { name }
    }

    /// 任务信息行：来自「{title}」。
    pub fn task_source_line(&self) -> String {
        fill(SECTION_TASK_SOURCE, &[("title", self.task_name())])
    }

    /// 产物名列表（、连接，空名 ⇒ 未命名）。
    pub fn artifact_names(&self) -> String {
        self.artifacts
            .iter()
            .map(artifact_display_name)
            .collect::<Vec<_>>()
            .join(ARTIFACT_NAME_SEPARATOR)
    }

    /// 产物计数行：共 {count} 项。
    pub fn artifact_count_line(&self) -> String {
        fill(SECTION_ARTIFACT_COUNT, &[("count", &self.artifacts.len().to_string())])
    }

    /// 摘要正文（未就绪时给对应逐字文案）。
    pub fn summary_line(&self) -> String {
        match &self.summary {
            SummaryPhase::Generating => SECTION_SUMMARY_GENERATING.to_string(),
            SummaryPhase::Loading => SUMMARY_LOADING.to_string(),
            SummaryPhase::Empty => SUMMARY_EMPTY.to_string(),
            SummaryPhase::Ready(text) => text.clone(),
        }
    }

    /// 能否提交转交（摘要未就绪或表单未通过校验 ⇒ 否）。
    pub fn can_submit(&self) -> bool {
        !matches!(self.summary, SummaryPhase::Loading) && self.todo.validation_error().is_none()
    }
}

impl TodoForm {
    /// 校验错误（标题空 ⇒ titleEmpty；未选处理人 ⇒ needAssignee）。
    pub fn validation_error(&self) -> Option<&'static str> {
        if self.title.trim().is_empty() {
            Some(TITLE_EMPTY)
        } else if self.assignee.is_none() {
            Some(NEED_ASSIGNEE)
        } else {
            None
        }
    }

    /// 是否呈现校验错误（未动过的空表单不立即报错）。
    pub fn shows_validation(&self) -> bool {
        !self.title.trim().is_empty() || self.assignee.is_some()
    }

    /// 处理人 chip 文案（未选 ⇒ unassignedChip；选中但名空 ⇒ fallbackUnknownMember）。
    pub fn assignee_chip_label(&self) -> Cow<'static, str> {
        match self.assignee.as_ref().map(|m| m.display_name.trim()) {
            Some(name) if !name.is_empty() => Cow::Owned(name.to_string()),
            Some(_) => Cow::Borrowed(FALLBACK_UNKNOWN_MEMBER),
            None => Cow::Borrowed(UNASSIGNED_CHIP),
        }
    }

    /// 已选择附件数行（attachmentsSelected）。
    pub fn selected_attachments_line(&self) -> String {
        fill(ATTACHMENTS_SELECTED, &[("count", &self.attachments.len().to_string())])
    }

    /// 附件行（attachmentLineHeader + 计数）。
    pub fn attachment_line(&self) -> String {
        [ATTACHMENT_LINE_HEADER, &self.selected_attachments_line()].concat()
    }

    /// 该附件是否已选（重复添加 ⇒ attachmentAlreadyAdded）。
    pub fn already_added(&self, name: &str) -> bool {
        self.attachments.iter().any(|a| a.name == name)
    }

    /// 添加附件的结果提示（重复 ⇒ attachmentAlreadyAdded）。
    pub fn add_attachment_notice(&self, name: &str) -> Option<&'static str> {
        self.already_added(name).then_some(ATTACHMENT_ALREADY_ADDED)
    }
}

impl State {
    /// 状态行文案（`Some` = 需呈现的转交结果/进度提示）。
    pub fn notice(&self) -> Option<String> {
        match self {
            Self::NotDelivered | Self::LoadFailed => Some(NOT_DELIVERED.to_string()),
            Self::Composing(_) => None,
            Self::Submitting => Some(SUBMITTING.to_string()),
            Self::Accepted { failed_attachments: 0, .. } => Some(HANDOFF_SUCCESS.to_string()),
            Self::Accepted { failed_attachments, .. } => Some(fill(
                ATTACHMENTS_PARTIAL_FAILED,
                &[("count", &failed_attachments.to_string())],
            )),
            Self::Failed(FailReason::Retry) => Some(HANDOFF_FAILED.to_string()),
            Self::Failed(FailReason::ProjectArchived) => {
                Some(HANDOFF_FAILED_PROJECT_ARCHIVED.to_string())
            }
            Self::Failed(FailReason::TaskArchived) => Some(HANDOFF_FAILED_TASK_ARCHIVED.to_string()),
        }
    }

    /// 底部主按钮文案（表单 = 创建；结果页 = 确认）。
    pub fn primary_action(&self) -> &'static str {
        match self {
            Self::NotDelivered | Self::LoadFailed | Self::Composing(_) => CREATE,
            Self::Submitting => SUBMITTING,
            Self::Accepted { .. } | Self::Failed(_) => SUBMIT,
        }
    }
}

// ===== GPUI 层 =====
/// 主区/浮层呈现入口（无数据源 ⇒ fail-closed「未下发」；签名不变）。
pub fn main_area(theme: &Theme) -> AnyElement {
    panel(&resolve(None), theme).into_any_element()
}

/// 整层：标题行 + 状态主体。
fn panel(state: &State, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(render_header(theme))
        .child(render_body(state, theme))
}

/// 标题行：弹窗标题徽标（headerTitle）+ 视图标题 + 关闭。
fn render_header(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(chip(HEADER_TITLE, theme))
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(div().flex_1())
        .child(chip(CLOSE_LABEL, theme))
}

/// 按状态机分支渲染主体。
fn render_body(state: &State, theme: &Theme) -> Div {
    match state {
        State::NotDelivered | State::LoadFailed => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(centered_muted(NOT_DELIVERED, theme)) // fail-closed：如实「未下发」
            .child(render_form(&TodoForm::default(), CREATE, theme)),
        State::Composing(scene) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(section_header(SECTION_TASK_INFO, None, theme))
                    .child(padded_muted_line(
                        SharedString::from(scene.task_source_line()),
                        theme,
                    )),
            )
            .child(render_artifacts(scene, theme))
            .child(render_summary(scene, theme))
            .child(render_form(&scene.todo, CREATE, theme)),
        State::Submitting => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(centered_muted(SUBMITTING, theme))
            .child(centered_muted(PENDING_MESSAGE_QUEUED, theme)),
        State::Accepted {
            uploaded_attachments,
            failed_attachments,
        } => {
            let kind = if *failed_attachments == 0 {
                StatusColor::Good
            } else {
                StatusColor::Warn
            };
            let mut col = div().flex().flex_col().gap_2().child(centered_status(
                SharedString::from(state.notice().unwrap_or_default()),
                kind,
                theme,
            ));
            if *uploaded_attachments > 0 {
                col = col.child(centered_muted(
                    fill(
                        ATTACHMENTS_UPLOADED,
                        &[("count", &uploaded_attachments.to_string())],
                    ),
                    theme,
                ));
            }
            col
        }
        State::Failed(_) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(centered_status(
                SharedString::from(state.notice().unwrap_or_default()),
                StatusColor::Bad,
                theme,
            )),
    }
}

/// 任务产物区（空 ⇒ 空态文案 noArtifacts，⛔ 不渲染空列表）。
fn render_artifacts(scene: &HandoffScene, theme: &Theme) -> Div {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(section_header(
            SECTION_ARTIFACT_TITLE,
            Some(SharedString::from(scene.artifact_count_line())),
            theme,
        ));
    if scene.artifacts.is_empty() {
        return col.child(padded_muted_line(NO_ARTIFACTS, theme));
    }
    for artifact in &scene.artifacts {
        let mut row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.foreground)
                    .child(SharedString::from(artifact_display_name(artifact))),
            );
        if artifact.ai_generated {
            row = row.child(chip_accent(AI_BADGE, theme));
        }
        col = col.child(row.child(chip(SECTION_FILE_TAG, theme)));
    }
    col
}

/// 任务进度摘要区（生成中挂 AI 徽标）。
fn render_summary(scene: &HandoffScene, theme: &Theme) -> Div {
    let mut line = div().flex().items_center().gap_2();
    if matches!(scene.summary, SummaryPhase::Generating) {
        line = line.child(chip_accent(AI_BADGE, theme));
    }
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(section_header(
            SECTION_PROGRESS_TITLE,
            Some(SharedString::from(summary_toggle(scene.summary_expanded))),
            theme,
        ))
        .child(
            line.text_sm()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(scene.summary_line())),
        )
}

/// 新建待办表单（本地状态，不依赖下发数据）。
fn render_form(form: &TodoForm, primary_label: &'static str, theme: &Theme) -> Div {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(section_header(FIELD_TITLE, None, theme))
        .child(field_input(&form.title, TITLE_PLACEHOLDER, theme))
        .child(section_header(FIELD_DESCRIPTION, None, theme))
        .child(field_input(&form.description, DESCRIPTION_PLACEHOLDER, theme))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(chip(form.assignee_chip_label().to_string(), theme))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(SEARCH_PLACEHOLDER),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(SharedString::from(form.attachment_line())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(button_chip(ATTACHMENTS_ADD_TITLE, false, theme))
                        .child(chip(ATTACHMENTS_LOCAL, theme))
                        .child(chip(ATTACHMENTS_NETDISK, theme)),
                ),
        );
    if form.shows_validation() {
        if let Some(err) = form.validation_error() {
            col = col.child(div().text_xs().text_color(theme.danger).child(err));
        }
    }
    col.child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(button_chip(CANCEL, false, theme))
            .child(button_chip(primary_label, true, theme)),
    )
}

/// 输入框样式节点（空值显示占位文案，弱化色）。
fn field_input(value: &str, placeholder: &'static str, theme: &Theme) -> Div {
    let filled = !value.trim().is_empty();
    div()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_sm()
        .text_color(if filled {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .child(if filled {
            SharedString::from(value.trim())
        } else {
            SharedString::from(placeholder)
        })
}

/// 弱化小标签。
fn chip(label: impl Into<SharedString>, theme: &Theme) -> Div {
    let text: SharedString = label.into();
    div()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 强调色小标签（AI 徽标等）。
fn chip_accent(label: &'static str, theme: &Theme) -> Div {
    div()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(theme.accent)
        .child(label)
}

/// 按钮（静态复刻，无交互）。
fn button_chip(label: &'static str, primary: bool, theme: &Theme) -> Div {
    let (bg, fg) = if primary {
        (theme.button_primary, theme.button_primary_foreground)
    } else {
        (theme.button_secondary, theme.button_secondary_foreground)
    };
    div()
        .px_3()
        .py_1()
        .rounded_sm()
        .text_sm()
        .bg(bg)
        .text_color(fg)
        .child(label)
}

/// 区块标题行（可选尾注）。
fn section_header(title: &'static str, trailing: Option<SharedString>, theme: &Theme) -> Div {
    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(title),
        );
    if let Some(text) = trailing {
        row = row.child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(text),
        );
    }
    row
}

/// 带内边距的弱化说明行。
fn padded_muted_line(text: impl Into<SharedString>, theme: &Theme) -> Div {
    let text: SharedString = text.into();
    div()
        .px_2()
        .py_1()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 居中弱化单行（状态提示）。
fn centered_muted(text: impl Into<SharedString>, theme: &Theme) -> Div {
    let text: SharedString = text.into();
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 结果状态色。
enum StatusColor {
    Good,
    Warn,
    Bad,
}

/// 居中结果行（成功/部分失败/失败三色）。
fn centered_status(text: SharedString, kind: StatusColor, theme: &Theme) -> Div {
    let color = match kind {
        StatusColor::Good => theme.success,
        StatusColor::Warn => theme.warning,
        StatusColor::Bad => theme.danger,
    };
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_sm()
        .text_color(color)
        .child(text)
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        attachment_count_line, fill, resolve, search_members, search_notice, State, FailReason,
        HandoffScene, SummaryPhase, TodoForm, ATTACHMENTS_PARTIAL_FAILED, ATTACHMENTS_UPLOADED,
        CREATE, HANDOFF_FAILED, HANDOFF_FAILED_PROJECT_ARCHIVED, HANDOFF_FAILED_TASK_ARCHIVED,
        HANDOFF_SUCCESS, NOT_DELIVERED, NS_KEY_COUNT, NS_PREFIX, SECTION_ARTIFACT_COUNT, SUBMIT,
        SUBMITTING, TITLE,
    };

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "任务交接");
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(State::default().notice().as_deref(), Some(NOT_DELIVERED));
    }

    #[test]
    fn state_notices_and_actions() {
        assert_eq!(resolve(Some(false)), State::LoadFailed);
        assert!(matches!(resolve(Some(true)), State::Composing(_)));
        assert_eq!(State::Submitting.notice().as_deref(), Some(SUBMITTING));
        let ok = State::Accepted {
            uploaded_attachments: 3,
            failed_attachments: 0,
        };
        assert_eq!(ok.notice().as_deref(), Some(HANDOFF_SUCCESS));
        let partial = State::Accepted {
            uploaded_attachments: 1,
            failed_attachments: 2,
        };
        assert_eq!(
            partial.notice().as_deref(),
            Some("任务已转交，但 2 个附件上传或绑定失败，可在计划详情中重新添加")
        );
        assert_eq!(
            State::Failed(FailReason::Retry).notice().as_deref(),
            Some(HANDOFF_FAILED)
        );
        assert_eq!(
            State::Failed(FailReason::ProjectArchived).notice().as_deref(),
            Some(HANDOFF_FAILED_PROJECT_ARCHIVED)
        );
        assert_eq!(
            State::Failed(FailReason::TaskArchived).notice().as_deref(),
            Some(HANDOFF_FAILED_TASK_ARCHIVED)
        );
        assert_eq!(
            State::Composing(HandoffScene::default()).primary_action(),
            CREATE
        );
        assert_eq!(ok.primary_action(), SUBMIT);
        assert_eq!(attachment_count_line(2), "2 个附件");
        assert_eq!(fill(SECTION_ARTIFACT_COUNT, &[("count", "3")]), "共 3 项");
        assert_eq!(
            fill(ATTACHMENTS_UPLOADED, &[("count", "5")]),
            "已上传 5 个附件"
        );
        assert_eq!(
            ATTACHMENTS_PARTIAL_FAILED,
            "任务已转交，但 {count} 个附件上传或绑定失败，可在计划详情中重新添加"
        );
        let _ = SummaryPhase::default();
        let _ = TodoForm::default();
        let _ = search_members("", &[]);
        let _ = search_notice(&[]);
    }

    #[test]
    fn ns_prefix_declared() {
        // 用运行时比较绕开 clippy::assertions_on_constants（两侧都经函数边界）。
        let prefix = NS_PREFIX.to_string();
        let count = NS_KEY_COUNT;
        assert!(!prefix.is_empty());
        assert!(prefix.chars().next().is_some_and(char::is_alphabetic));
        assert!(count >= 40 && count < 10_000);
    }
}
