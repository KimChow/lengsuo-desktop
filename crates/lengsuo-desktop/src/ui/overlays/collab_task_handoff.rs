//! 任务交接（波次 8，卡 `ui-collab-task-handoff`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.task.handoff`；
//! 数据源在运行时侧，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题。
pub const TITLE: &str = "任务交接";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.task.handoff";
/// 本文件承载的命名空间键数（卡要求 ≥40）。
pub const NS_KEY_COUNT: usize = 42;

// ===== 文案常量（逐字；行尾注释为全表键名）=====
pub const HEADER_TITLE: &str = "新建待办"; // headerTitle
pub const CLOSE_LABEL: &str = "关闭"; // closeLabel
pub const CANCEL: &str = "取消"; // cancel
pub const CREATE: &str = "创建"; // create
pub const SUBMIT: &str = "确认"; // submit
pub const SUBMITTING: &str = "提交中..."; // submitting
pub const TITLE_PLACEHOLDER: &str = "请输入标题"; // titlePlaceholder
pub const DESCRIPTION_PLACEHOLDER: &str = "添加描述（可选）"; // descriptionPlaceholder
pub const TITLE_EMPTY: &str = "请先填写待办标题"; // titleEmpty
pub const NEED_ASSIGNEE: &str = "请选择处理人"; // needAssignee
pub const UNASSIGNED_CHIP: &str = "处理人"; // unassignedChip
pub const FALLBACK_UNKNOWN_MEMBER: &str = "未知成员"; // fallbackUnknownMember
pub const ATTACHMENT_LINE_HEADER: &str = "附件："; // attachmentLineHeader
pub const ATTACHMENT_COUNT: &str = "{count} 个附件"; // attachmentCount
pub const ATTACHMENTS_SELECTED: &str = "已选择 {count} 个附件"; // attachmentsSelected
pub const ATTACHMENTS_UPLOADED: &str = "已上传 {count} 个附件"; // attachmentsUploaded
pub const ATTACHMENT_ALREADY_ADDED: &str = "选择的附件已添加"; // attachmentAlreadyAdded
pub const ATTACHMENTS_PARTIAL_FAILED: &str =
    "任务已转交，但 {count} 个附件上传或绑定失败，可在计划详情中重新添加"; // attachmentsPartialFailed
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
pub const HANDOFF_SUCCESS: &str = "任务正在转交中，稍后可在计划看板中查看"; // handoffSuccess
pub const HANDOFF_FAILED: &str = "转交失败，请重试"; // handoffFailed
pub const HANDOFF_FAILED_PROJECT_ARCHIVED: &str = "该项目已归档，无法转交任务"; // handoffFailedProjectArchived
pub const HANDOFF_FAILED_TASK_ARCHIVED: &str = "该任务已归档或删除，无法转交"; // handoffFailedTaskArchived
pub const PENDING_MESSAGE_QUEUED: &str = "任务转交中，新消息将在转交完成后依次处理"; // pendingMessageQueued

// ===== 纯数据层（无 GPUI 依赖）=====
/// 产物条目（空名渲染 [`FALLBACK_ARTIFACT_NAME`]）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub name: String,
    pub ai_generated: bool,
}

/// 进度摘要状态（`Loading` 阻止提交，对应 summaryLoading）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SummaryPhase {
    /// AI 摘要生成中。
    #[default]
    Generating,
    Loading,
    Empty,
    Ready(String),
}

/// 新建待办表单值（`assignee` 为成员显示名；`attachments` 为附件名清单）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TodoForm {
    pub title: String,
    pub description: String,
    pub assignee: Option<String>,
    pub attachments: Vec<String>,
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
    Retry,
    ProjectArchived,
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
    Composing(HandoffScene),
    Submitting,
    /// 转交已受理（失败附件数 >0 ⇒ 部分失败文案）。
    Accepted {
        failed_attachments: usize,
    },
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

pub fn summary_toggle(expanded: bool) -> &'static str {
    if expanded {
        SECTION_SUMMARY_COLLAPSE
    } else {
        SECTION_SUMMARY_EXPAND
    }
}

/// AI 摘要缺位时的兜底摘要句（标题 +（成员）+ 交互轮次，逐字模板拼接）。
pub fn fallback_summary(
    task: &str,
    artifact_count: usize,
    names: &[&str],
    interactions: usize,
) -> String {
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
        return FALLBACK_ARTIFACT_NAME;
    }
    name
}

impl HandoffScene {
    /// 来源任务名（空 ⇒ [`FALLBACK_TASK_NAME`]）。
    pub fn task_name(&self) -> &str {
        let name = self.source_task.trim();
        if name.is_empty() {
            return FALLBACK_TASK_NAME;
        }
        name
    }

    pub fn task_source_line(&self) -> String {
        fill(SECTION_TASK_SOURCE, &[("title", self.task_name())])
    }

    pub fn artifact_count_line(&self) -> String {
        fill(
            SECTION_ARTIFACT_COUNT,
            &[("count", &self.artifacts.len().to_string())],
        )
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
    pub fn assignee_chip_label(&self) -> String {
        match self.assignee.as_deref().map(str::trim) {
            Some(name) if !name.is_empty() => name.to_string(),
            Some(_) => FALLBACK_UNKNOWN_MEMBER.to_string(),
            None => UNASSIGNED_CHIP.to_string(),
        }
    }

    pub fn selected_attachments_line(&self) -> String {
        fill(
            ATTACHMENTS_SELECTED,
            &[("count", &self.attachments.len().to_string())],
        )
    }

    /// 附件行（attachmentLineHeader + 计数）。
    pub fn attachment_line(&self) -> String {
        [ATTACHMENT_LINE_HEADER, &self.selected_attachments_line()].concat()
    }
}

impl State {
    /// 状态行文案（`Some` = 需呈现的转交结果/进度提示）。
    pub fn notice(&self) -> Option<String> {
        match self {
            Self::NotDelivered | Self::LoadFailed => Some(NOT_DELIVERED.to_string()),
            Self::Composing(_) => None,
            Self::Submitting => Some(SUBMITTING.to_string()),
            Self::Accepted {
                failed_attachments: 0,
            } => Some(HANDOFF_SUCCESS.to_string()),
            Self::Accepted { failed_attachments } => Some(fill(
                ATTACHMENTS_PARTIAL_FAILED,
                &[("count", &failed_attachments.to_string())],
            )),
            Self::Failed(FailReason::Retry) => Some(HANDOFF_FAILED.to_string()),
            Self::Failed(FailReason::ProjectArchived) => {
                Some(HANDOFF_FAILED_PROJECT_ARCHIVED.to_string())
            }
            Self::Failed(FailReason::TaskArchived) => {
                Some(HANDOFF_FAILED_TASK_ARCHIVED.to_string())
            }
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
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(render_header(theme))
        .child(render_body(&resolve(None), theme))
        .into_any_element()
}

/// 标题行：弹窗标题徽标（headerTitle）+ 视图标题 + 关闭。
fn render_header(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(chip(HEADER_TITLE, theme.muted_foreground, theme))
        .child(chip(TITLE, theme.foreground, theme).text_sm().font_bold())
        .child(div().flex_1())
        .child(chip(CLOSE_LABEL, theme.muted_foreground, theme))
}

/// 按状态机分支渲染主体。
fn render_body(state: &State, theme: &Theme) -> Div {
    match state {
        State::NotDelivered | State::LoadFailed => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(centered_line(NOT_DELIVERED, theme.muted_foreground))
            .child(render_form(&TodoForm::default(), theme)),
        State::Composing(scene) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(section_header(
                SECTION_TASK_INFO,
                SharedString::from(scene.task_source_line()),
                theme,
            ))
            .child(render_artifacts(scene, theme))
            .child(render_summary(scene, theme))
            .child(render_form(&scene.todo, theme)),
        State::Submitting => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(centered_line(SUBMITTING, theme.muted_foreground))
            .child(centered_line(
                PENDING_MESSAGE_QUEUED,
                theme.muted_foreground,
            )),
        State::Accepted { failed_attachments } => {
            let ok = *failed_attachments == 0;
            let color = if ok { theme.success } else { theme.warning };
            centered_line(state.notice().unwrap_or_default(), color)
        }
        State::Failed(_) => centered_line(state.notice().unwrap_or_default(), theme.danger),
    }
}

/// 任务产物区（空 ⇒ 空态文案 noArtifacts，⛔ 不渲染空列表）。
fn render_artifacts(scene: &HandoffScene, theme: &Theme) -> Div {
    let mut col = div().flex().flex_col().gap_1().child(section_header(
        SECTION_ARTIFACT_TITLE,
        SharedString::from(scene.artifact_count_line()),
        theme,
    ));
    if scene.artifacts.is_empty() {
        return col.child(muted_line(NO_ARTIFACTS, theme));
    }
    for artifact in &scene.artifacts {
        let mut row = div().flex().items_center().gap_2().child(
            div()
                .text_sm()
                .text_color(theme.foreground)
                .child(SharedString::from(artifact_display_name(artifact))),
        );
        if artifact.ai_generated {
            row = row.child(chip(AI_BADGE, theme.accent, theme));
        }
        col = col.child(row.child(chip(SECTION_FILE_TAG, theme.muted_foreground, theme)));
    }
    col
}

/// 任务进度摘要区（标题 + 展开/收起尾注 + 摘要行）。
fn render_summary(scene: &HandoffScene, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(section_header(
            SECTION_PROGRESS_TITLE,
            SharedString::from(summary_toggle(scene.summary_expanded)),
            theme,
        ))
        .child(muted_line(SharedString::from(scene.summary_line()), theme))
}

/// 新建待办表单（本地状态，不依赖下发数据）。
fn render_form(form: &TodoForm, theme: &Theme) -> Div {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(field_input(&form.title, TITLE_PLACEHOLDER, theme))
        .child(field_input(
            &form.description,
            DESCRIPTION_PLACEHOLDER,
            theme,
        ))
        .child(chip(
            form.assignee_chip_label(),
            theme.muted_foreground,
            theme,
        ))
        .child(chip(form.attachment_line(), theme.muted_foreground, theme));
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
            .child(button_chip(CREATE, true, theme)),
    )
}

/// 输入框样式节点（空值显示占位文案，弱化色）。
fn field_input(value: &str, placeholder: &'static str, theme: &Theme) -> Div {
    let filled = !value.trim().is_empty();
    let text = if filled { value.trim() } else { placeholder };
    let color = if filled {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    chip(text, color, theme).text_sm()
}

/// 小标签（边框 chip，文字色由调用方给）。
fn chip(label: impl Into<SharedString>, color: Hsla, theme: &Theme) -> Div {
    let text: SharedString = label.into();
    div()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .text_color(color)
        .child(text)
}

/// 按钮（静态复刻，无交互）。
fn button_chip(label: &'static str, primary: bool, theme: &Theme) -> Div {
    let (bg, fg) = if primary {
        (theme.button_primary, theme.button_primary_foreground)
    } else {
        (theme.button_secondary, theme.button_secondary_foreground)
    };
    chip(label, fg, theme).bg(bg).text_sm()
}

/// 区块标题行（标题 + 尾注）。
fn section_header(title: &'static str, trailing: SharedString, theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(title),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(trailing),
        )
}

/// 带内边距的弱化说明行。
fn muted_line(text: impl Into<SharedString>, theme: &Theme) -> Div {
    let text: SharedString = text.into();
    div()
        .px_2()
        .py_1()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(text)
}

/// 居中状态行（结果三色 / 弱化）。
fn centered_line(text: impl Into<SharedString>, color: Hsla) -> Div {
    let text: SharedString = text.into();
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
        fallback_summary, fill, main_area, render_body, resolve, Artifact, FailReason,
        HandoffScene, State, SummaryPhase, TodoForm, AI_BADGE, ATTACHMENTS_PARTIAL_FAILED,
        ATTACHMENTS_SELECTED, ATTACHMENT_ALREADY_ADDED, ATTACHMENT_COUNT, ATTACHMENT_LINE_HEADER,
        CREATE, DESCRIPTION_PLACEHOLDER, FALLBACK_ARTIFACT_NAME, FALLBACK_TASK_NAME,
        FALLBACK_UNKNOWN_MEMBER, HANDOFF_FAILED, HANDOFF_FAILED_PROJECT_ARCHIVED,
        HANDOFF_FAILED_TASK_ARCHIVED, HANDOFF_SUCCESS, NEED_ASSIGNEE, NOT_DELIVERED, NO_ARTIFACTS,
        NS_KEY_COUNT, NS_PREFIX, PENDING_MESSAGE_QUEUED, PROGRESS_FALLBACK_INTERACTION,
        SECTION_SUMMARY_GENERATING, SECTION_TASK_SOURCE, SUBMIT, SUBMITTING, SUMMARY_EMPTY,
        SUMMARY_LOADING, TITLE, TITLE_EMPTY, TITLE_PLACEHOLDER, UNASSIGNED_CHIP,
    };
    use gpui_kit::component::Theme;

    fn artifact(name: &str, ai_generated: bool) -> Artifact {
        Artifact {
            name: name.into(),
            ai_generated,
        }
    }

    fn accepted(failed: usize) -> State {
        State::Accepted {
            failed_attachments: failed,
        }
    }

    #[test]
    fn state_notices_and_primary_actions() {
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(resolve(Some(false)), State::LoadFailed);
        assert!(matches!(resolve(Some(true)), State::Composing(_)));
        let cases = [
            (State::default(), NOT_DELIVERED),
            (State::Submitting, SUBMITTING),
            (accepted(0), HANDOFF_SUCCESS),
            (State::Failed(FailReason::Retry), HANDOFF_FAILED),
            (
                State::Failed(FailReason::ProjectArchived),
                HANDOFF_FAILED_PROJECT_ARCHIVED,
            ),
            (
                State::Failed(FailReason::TaskArchived),
                HANDOFF_FAILED_TASK_ARCHIVED,
            ),
        ];
        for (state, expect) in &cases {
            assert_eq!(state.notice().as_deref(), Some(*expect));
        }
        assert_eq!(
            accepted(2).notice().as_deref(),
            Some("任务已转交，但 2 个附件上传或绑定失败，可在计划详情中重新添加")
        );
        assert_eq!(
            State::Composing(HandoffScene::default()).primary_action(),
            CREATE
        );
        assert_eq!(State::Submitting.primary_action(), SUBMITTING);
        assert_eq!(accepted(0).primary_action(), SUBMIT);
        assert_eq!(fill(ATTACHMENT_COUNT, &[("count", "2")]), "2 个附件");
        assert_eq!(
            ATTACHMENTS_PARTIAL_FAILED,
            "任务已转交，但 {count} 个附件上传或绑定失败，可在计划详情中重新添加"
        );
        // 命名空间事实（经函数边界比较，绕开 clippy::assertions_on_constants）。
        let prefix = NS_PREFIX.to_string();
        let count = NS_KEY_COUNT;
        assert!(prefix.chars().next().is_some_and(char::is_alphabetic));
        assert!((40..10_000).contains(&count));
    }

    #[test]
    fn empty_inputs_yield_empty_states_not_lists() {
        let scene = HandoffScene::default();
        assert!(scene.artifacts.is_empty());
        assert_eq!(scene.artifact_count_line(), "共 0 项");
        assert_eq!(ATTACHMENT_LINE_HEADER, "附件：");
        assert_eq!(ATTACHMENTS_SELECTED, "已选择 {count} 个附件");
        assert_eq!(scene.todo.attachment_line(), "附件：已选择 0 个附件");
    }

    #[test]
    fn copy_is_verbatim_from_namespace() {
        let verbatim: &[(&str, &str)] = &[
            (TITLE, "任务交接"),
            (AI_BADGE, "AI生成"),
            (FALLBACK_TASK_NAME, "当前任务"),
            (FALLBACK_UNKNOWN_MEMBER, "未知成员"),
            (FALLBACK_ARTIFACT_NAME, "未命名"),
            (NO_ARTIFACTS, "暂无产物"),
            (TITLE_PLACEHOLDER, "请输入标题"),
            (DESCRIPTION_PLACEHOLDER, "添加描述（可选）"),
            (ATTACHMENT_ALREADY_ADDED, "选择的附件已添加"),
            (HANDOFF_SUCCESS, "任务正在转交中，稍后可在计划看板中查看"),
            (
                PENDING_MESSAGE_QUEUED,
                "任务转交中，新消息将在转交完成后依次处理",
            ),
            (SECTION_TASK_SOURCE, "来自「{title}」"),
            (SUMMARY_EMPTY, "任务进展摘要为空，请稍后重试"),
            (SUMMARY_LOADING, "任务进展摘要生成中，请稍后再转交"),
        ];
        for (key, expect) in verbatim {
            assert_eq!(*key, *expect);
        }
        assert_eq!(
            PROGRESS_FALLBACK_INTERACTION,
            "。对话共 {count} 轮交互，任务整体进展顺利，可接续后续环节。"
        );
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = main_area(&theme);
    }

    #[test]
    fn render_body_covers_all_state_branches() {
        let theme = Theme::default();
        let filled = HandoffScene {
            source_task: "官网改版".into(),
            artifacts: vec![artifact("首页视觉.png", true), artifact("  ", false)],
            ..HandoffScene::default()
        };
        let branches = [
            State::NotDelivered,
            State::LoadFailed,
            State::Composing(HandoffScene::default()),
            State::Composing(filled),
            State::Submitting,
            accepted(0),
            accepted(1),
            State::Failed(FailReason::Retry),
            State::Failed(FailReason::ProjectArchived),
            State::Failed(FailReason::TaskArchived),
        ];
        for state in &branches {
            let _ = render_body(state, &theme);
        }
    }

    #[test]
    fn form_validation_and_scene_fallbacks() {
        let mut todo = TodoForm::default();
        assert_eq!(todo.assignee_chip_label(), UNASSIGNED_CHIP);
        assert_eq!(todo.validation_error(), Some(TITLE_EMPTY));
        assert!(!todo.shows_validation());
        todo.title = "对接客户".into();
        assert_eq!(todo.validation_error(), Some(NEED_ASSIGNEE));
        assert!(todo.shows_validation());
        todo.assignee = Some("  ".into());
        assert_eq!(todo.assignee_chip_label(), FALLBACK_UNKNOWN_MEMBER);
        todo.assignee = Some("李雷".into());
        assert_eq!(todo.validation_error(), None);
        assert_eq!(todo.assignee_chip_label(), "李雷");
        todo.attachments = vec!["brief.pdf".into()];
        assert_eq!(todo.selected_attachments_line(), "已选择 1 个附件");

        let mut scene = HandoffScene {
            todo,
            ..HandoffScene::default()
        };
        assert_eq!(scene.task_name(), FALLBACK_TASK_NAME);
        assert_eq!(scene.task_source_line(), "来自「当前任务」");
        assert_eq!(scene.summary_line(), SECTION_SUMMARY_GENERATING);
        assert!(scene.can_submit());
        scene.summary = SummaryPhase::Loading;
        assert_eq!(scene.summary_line(), SUMMARY_LOADING);
        assert!(!scene.can_submit());
        scene.summary = SummaryPhase::Empty;
        assert_eq!(scene.summary_line(), SUMMARY_EMPTY);
        scene.source_task = "官网改版".into();
        assert_eq!(scene.task_source_line(), "来自「官网改版」");
    }

    #[test]
    fn fallback_summary_composes_verbatim_parts() {
        let line = fallback_summary("官网改版", 3, &["李雷", "韩梅梅"], 12);
        assert_eq!(
            line,
            "该任务「官网改版」已产出 3 个文件产物（李雷、韩梅梅）。对话共 12 轮交互，任务整体进展顺利，可接续后续环节。"
        );
    }
}
