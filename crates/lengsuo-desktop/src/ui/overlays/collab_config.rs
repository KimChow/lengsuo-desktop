//! 协作·配置（波次 9，卡 `ui-overlay-collab-config`）。
//!
//! 分层约定（同 `panel_views` / `automation_schedule`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.config`（全表 93 键）。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ===== 纯数据层（无 GPUI 依赖）=====
/// 视图标题（`collab.config.title`，逐字取自界面文案全表）。
pub const TITLE: &str = "项目配置";
/// 键数（据文案全表命名空间 `collab.config` 全表 93 键）。
pub const NS_KEY_COUNT: usize = 93;
/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.config";

/// 文案表（key → 中文，逐字取自全表命名空间 `collab.config`，93 键全量）。
pub static TEXTS: &[(&str, &str)] = &[
    ("collab.config.automation.addAction", "+ 添加计划"),
    ("collab.config.automation.description", "让 AI 按计划自动执行任务"),
    ("collab.config.automation.manageAria", "管理定时任务"),
    ("collab.config.automation.title", "定时任务"),
    ("collab.config.automation.viewAll", "查看全部({count})"),
    ("collab.config.experts.addAction", "+ 添加"),
    ("collab.config.experts.description", "配置项目专家，为成员提供更专业的服务"),
    ("collab.config.experts.manageAria", "管理专家"),
    ("collab.config.experts.title", "专家"),
    ("collab.config.expertsDrawer.addAction", "添加专家"),
    ("collab.config.expertsDrawer.addActionShort", "添加"),
    ("collab.config.expertsDrawer.close", "关闭"),
    ("collab.config.expertsDrawer.closeAria", "关闭"),
    ("collab.config.expertsDrawer.confirmCancel", "取消"),
    (
        "collab.config.expertsDrawer.confirmContent",
        "确定要移除专家「{name}」吗？移除后项目内任务将不再使用该专家。",
    ),
    ("collab.config.expertsDrawer.confirmOk", "确定"),
    ("collab.config.expertsDrawer.confirmRemove", "移除"),
    ("collab.config.expertsDrawer.confirmTitle", "移除专家"),
    ("collab.config.expertsDrawer.empty", "还没有添加专家"),
    ("collab.config.expertsDrawer.removeAria", "移除"),
    ("collab.config.expertsDrawer.summaryPrefix", "当前项目已添加"),
    ("collab.config.expertsDrawer.summarySuffix", "个专家"),
    ("collab.config.expertsDrawer.title", "专家"),
    ("collab.config.expertsDrawer.toast.added", "已添加 {count} 个专家"),
    (
        "collab.config.expertsDrawer.toast.alreadyAdded",
        "选取的专家已添加到项目中",
    ),
    (
        "collab.config.expertsDrawer.toast.authExpired",
        "登录已过期，请重新登录后重试",
    ),
    (
        "collab.config.expertsDrawer.toast.badRequest",
        "请求参数有误，请刷新页面后重试",
    ),
    (
        "collab.config.expertsDrawer.toast.networkError",
        "网络连接异常，请检查网络后重试",
    ),
    (
        "collab.config.expertsDrawer.toast.notFoundOrUnpublished",
        "专家不可用或已下架",
    ),
    (
        "collab.config.expertsDrawer.toast.partialFailed",
        "以下专家添加失败：{names}",
    ),
    (
        "collab.config.expertsDrawer.toast.partialUploaded",
        "以下专家部分文件被跳过：{names}",
    ),
    (
        "collab.config.expertsDrawer.toast.permissionDenied",
        "抱歉，您当前没有权限进行该操作",
    ),
    (
        "collab.config.expertsDrawer.toast.providerUnavailable",
        "专家市场暂不可用，请稍后重试",
    ),
    (
        "collab.config.expertsDrawer.toast.removed",
        "已移除 {count} 个专家",
    ),
    (
        "collab.config.expertsDrawer.toast.removedNamed",
        "已移除「{name}」",
    ),
    ("collab.config.expertsDrawer.toast.removeFailed", "移除失败"),
    (
        "collab.config.expertsDrawer.toast.serverError",
        "服务器异常，请稍后重试",
    ),
    (
        "collab.config.expertsDrawer.toast.tooManyRequests",
        "操作过于频繁，请稍后重试",
    ),
    ("collab.config.expertsDrawer.toast.updated", "专家列表已更新"),
    ("collab.config.expertsDrawer.toast.updateFailed", "更新专家失败"),
    (
        "collab.config.instruction.description",
        "设定项目背景与规范，让 AI 与你高效协作",
    ),
    ("collab.config.instruction.editAction", "编辑指令"),
    ("collab.config.instruction.editAria", "编辑指令"),
    ("collab.config.instruction.title", "指令"),
    ("collab.config.instructionDrawer.cancel", "取消"),
    ("collab.config.instructionDrawer.closeAria", "关闭"),
    (
        "collab.config.instructionDrawer.description",
        "在此项目下的所有任务都会参考这里的指令。建议提供当前项目的背景信息和规范，让 {productName} 的回复更精准、更符合要求。",
    ),
    (
        "collab.config.instructionDrawer.placeholder",
        "例：本项目用于 Q2 季度产品规划，所有产出应使用结构化 markdown，分章节并包含目标、范围、里程碑、风险、责任人。文档语言为简体中文，语气专业。",
    ),
    (
        "collab.config.instructionDrawer.readOnlyPlaceholder",
        "当前项目暂未配置指令",
    ),
    ("collab.config.instructionDrawer.save", "保存"),
    ("collab.config.instructionDrawer.saveFailed", "保存失败，请重试"),
    ("collab.config.instructionDrawer.saving", "保存中…"),
    ("collab.config.instructionDrawer.title", "项目指令"),
    ("collab.config.skills.addAction", "+ 添加"),
    (
        "collab.config.skills.description",
        "配置项目技能，让 AI 精准执行任务",
    ),
    ("collab.config.skills.manageAria", "管理技能"),
    ("collab.config.skills.title", "技能"),
    ("collab.config.skillsDrawer.addAction", "添加技能"),
    ("collab.config.skillsDrawer.addActionShort", "添加"),
    ("collab.config.skillsDrawer.close", "关闭"),
    ("collab.config.skillsDrawer.closeAria", "关闭"),
    ("collab.config.skillsDrawer.confirmCancel", "取消"),
    (
        "collab.config.skillsDrawer.confirmContent",
        "确定要移除技能「{name}」吗？移除后项目内任务将不再使用该技能。",
    ),
    ("collab.config.skillsDrawer.confirmOk", "确定"),
    ("collab.config.skillsDrawer.confirmRemove", "移除"),
    ("collab.config.skillsDrawer.confirmTitle", "移除技能"),
    ("collab.config.skillsDrawer.empty", "还没有添加技能"),
    ("collab.config.skillsDrawer.importAction", "导入技能"),
    ("collab.config.skillsDrawer.removeAria", "移除"),
    ("collab.config.skillsDrawer.summaryPrefix", "当前项目已添加"),
    ("collab.config.skillsDrawer.summarySuffix", "个技能"),
    ("collab.config.skillsDrawer.title", "技能"),
    ("collab.config.skillsDrawer.toast.added", "已添加 {count} 个技能"),
    (
        "collab.config.skillsDrawer.toast.alreadyAdded",
        "该技能已添加到项目中",
    ),
    (
        "collab.config.skillsDrawer.toast.authExpired",
        "登录已过期，请重新登录后重试",
    ),
    (
        "collab.config.skillsDrawer.toast.badRequest",
        "请求参数有误，请刷新页面后重试",
    ),
    (
        "collab.config.skillsDrawer.toast.duplicateName",
        "同名技能已存在，请先删除后重试",
    ),
    (
        "collab.config.skillsDrawer.toast.exportZipFailed",
        "技能「{name}」打包失败，请检查技能文件是否完整",
    ),
    (
        "collab.config.skillsDrawer.toast.exportZipUnavailable",
        "当前环境不支持打包本地技能",
    ),
    (
        "collab.config.skillsDrawer.toast.networkError",
        "网络连接异常，请检查网络后重试",
    ),
    (
        "collab.config.skillsDrawer.toast.notFoundOrUnpublished",
        "技能不可用或已下架",
    ),
    (
        "collab.config.skillsDrawer.toast.partialUploadFailed",
        "以下技能上传失败：{names}，其余技能已更新",
    ),
    (
        "collab.config.skillsDrawer.toast.permissionDenied",
        "抱歉，您当前没有权限进行该操作",
    ),
    (
        "collab.config.skillsDrawer.toast.providerUnavailable",
        "技能市场暂不可用，请稍后重试",
    ),
    (
        "collab.config.skillsDrawer.toast.removed",
        "已移除 {count} 个技能",
    ),
    (
        "collab.config.skillsDrawer.toast.removedNamed",
        "已移除「{name}」",
    ),
    ("collab.config.skillsDrawer.toast.removeFailed", "移除失败"),
    (
        "collab.config.skillsDrawer.toast.serverError",
        "服务器异常，请稍后重试",
    ),
    (
        "collab.config.skillsDrawer.toast.tooManyRequests",
        "操作过于频繁，请稍后重试",
    ),
    ("collab.config.skillsDrawer.toast.updated", "技能列表已更新"),
    ("collab.config.skillsDrawer.toast.updateFailed", "更新技能失败"),
    (
        "collab.config.skillsDrawer.toast.uploadFailed",
        "技能上传失败：{names}",
    ),
    ("collab.config.title", "项目配置"),
];

/// 按键取文案（纯函数）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|&&(k, _)| k == key).map(|&(_, v)| v)
}

/// 取文案，缺键回空串（键表由测试逐字锁定）。
fn t(key: &str) -> &'static str {
    text(key).unwrap_or("")
}

/// 模板占位符替换（纯函数；占位符形如 `{count}`/`{name}`）。
fn tpl(key: &str, subs: &[(&str, &str)]) -> String {
    let mut out = t(key).to_string();
    for (placeholder, value) in subs {
        out = out.replace(placeholder, value);
    }
    out
}

/// 「查看全部({count})」（定时任务区；count 为 0 不呈现该行）。
pub fn view_all_label(count: usize) -> Option<String> {
    if count == 0 {
        None
    } else {
        Some(tpl(
            "collab.config.automation.viewAll",
            &[("{count}", count.to_string().as_str())],
        ))
    }
}

/// 各配置区已配置数量（字段值来自数据源；⛔ 无源不构造非零值）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConfigCounts {
    /// 定时任务数。
    pub automation: usize,
    /// 专家数。
    pub experts: usize,
    /// 技能数。
    pub skills: usize,
}

impl ConfigCounts {
    /// 可数区全为 0 ⇒ 空态（指令区为文本，不计数）。
    pub fn is_empty(&self) -> bool {
        self.automation == 0 && self.experts == 0 && self.skills == 0
    }
}

/// 浮层/面板状态机（默认 `Loading`：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已就绪，正常呈现。
    Ready,
    /// 拉取失败（fail-closed，必须显示错误）。
    LoadFailed,
    /// 拉取中（默认态）。
    #[default]
    Loading,
}

/// 由运行时输入解析初始态（纯函数）。
pub fn resolve(loaded: Option<bool>) -> State {
    match loaded {
        None => State::Loading,
        Some(false) => State::LoadFailed,
        Some(true) => State::Ready,
    }
}

/// 渲染视图态：`Ready` + 全零计数 ⇒ 空态（⛔ 不是空列表）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View<'a> {
    /// 拉取中。
    Loading,
    /// 拉取失败（fail-closed）。
    LoadFailed,
    /// 已就绪且无任何配置。
    Empty,
    /// 已就绪且有配置。
    Normal(&'a ConfigCounts),
}

/// 状态 + 计数 → 视图态（纯函数）。
pub fn view_of<'a>(state: State, counts: &'a ConfigCounts) -> View<'a> {
    match state {
        State::Loading => View::Loading,
        State::LoadFailed => View::LoadFailed,
        State::Ready if counts.is_empty() => View::Empty,
        State::Ready => View::Normal(counts),
    }
}

// ===== GPUI 层 =====
/// 拉取中说明（全表 `collab.activity.panel.loading`，跨命名空间借用，状态文档已说明）。
const LOADING_TEXT: &str = "加载中…";
/// 拉取失败说明（全表 `collab.activity.panel.loadFailed`，同上；fail-closed 必须可见）。
const LOAD_FAILED_TEXT: &str = "加载失败";

/// 配置区卡片（键 = 该区在全表中的文案键；指令区为文本无计数）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Instruction,
    Automation,
    Skills,
    Experts,
}

impl Section {
    /// 全表键名（标题）。
    fn title_key(&self) -> &'static str {
        match self {
            Section::Instruction => "collab.config.instruction.title",
            Section::Automation => "collab.config.automation.title",
            Section::Skills => "collab.config.skills.title",
            Section::Experts => "collab.config.experts.title",
        }
    }

    /// 全表键名（说明）。
    fn description_key(&self) -> &'static str {
        match self {
            Section::Instruction => "collab.config.instruction.description",
            Section::Automation => "collab.config.automation.description",
            Section::Skills => "collab.config.skills.description",
            Section::Experts => "collab.config.experts.description",
        }
    }

    /// 全表键名（动作文案）。
    fn action_key(&self) -> &'static str {
        match self {
            Section::Instruction => "collab.config.instruction.editAction",
            Section::Automation => "collab.config.automation.addAction",
            Section::Skills => "collab.config.skills.addAction",
            Section::Experts => "collab.config.experts.addAction",
        }
    }

    /// 标题（逐字）。
    fn label(&self) -> &'static str {
        t(self.title_key())
    }

    /// 说明（逐字）。
    fn description(&self) -> &'static str {
        t(self.description_key())
    }

    /// 动作文案（逐字）。
    fn action(&self) -> &'static str {
        t(self.action_key())
    }
}

/// 主区/浮层呈现入口。数据源未接线 ⇒ 默认 `Loading`（fail-closed，⛔ 不伪造计数）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render_view(view_of(State::default(), &ConfigCounts::default()), theme)
}

/// 按视图态逐分支渲染（纯函数；标题行 + 分支内容节点）。
fn render_view(view: View<'_>, theme: &Theme) -> AnyElement {
    let root = div().flex().flex_col().gap_2().child(
        div()
            .text_color(theme.foreground)
            .child(SharedString::from(TITLE)),
    );
    match view {
        View::Loading => root.child(caption(LOADING_TEXT, theme)).into_any_element(),
        View::LoadFailed => root
            .child(caption(LOAD_FAILED_TEXT, theme))
            .into_any_element(),
        View::Empty => root
            .child(section_card(Section::Instruction, 0, theme))
            .child(section_card(Section::Automation, 0, theme))
            .child(section_card(Section::Skills, 0, theme))
            .child(section_card(Section::Experts, 0, theme))
            .into_any_element(),
        View::Normal(counts) => root
            .child(section_card(Section::Instruction, 0, theme))
            .child(section_card(Section::Automation, counts.automation, theme))
            .child(section_card(Section::Skills, counts.skills, theme))
            .child(section_card(Section::Experts, counts.experts, theme))
            .into_any_element(),
    }
}

/// 一张配置区卡（标题 + 动作行 / 说明；定时任务区附「查看全部({count})」行）。
fn section_card(section: Section, count: usize, theme: &Theme) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .border_1()
        .border_color(theme.border)
        .rounded_lg()
        .child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .child(
                    div()
                        .text_color(theme.foreground)
                        .child(SharedString::from(section.label())),
                )
                .child(
                    div()
                        .text_color(theme.muted_foreground)
                        .child(SharedString::from(section.action())),
                ),
        )
        .child(caption(section.description(), theme));
    if let (Section::Automation, Some(label)) = (section, view_all_label(count)) {
        card.child(caption(&label, theme)).into_any_element()
    } else {
        card.into_any_element()
    }
}

/// 说明行（弱化色）。
fn caption(body: &str, theme: &Theme) -> AnyElement {
    div()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(body.to_string()))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        text, view_all_label, view_of, ConfigCounts, State, View, NS_KEY_COUNT, NS_PREFIX, TEXTS,
        TITLE,
    };

    #[test]
    fn texts_verbatim_and_full_coverage() {
        assert_eq!(TITLE, "项目配置");
        // 任务书逐字摘录 12 条。
        assert_eq!(
            text("collab.config.automation.addAction"),
            Some("+ 添加计划")
        );
        assert_eq!(
            text("collab.config.automation.description"),
            Some("让 AI 按计划自动执行任务")
        );
        assert_eq!(
            text("collab.config.automation.manageAria"),
            Some("管理定时任务")
        );
        assert_eq!(text("collab.config.automation.title"), Some("定时任务"));
        assert_eq!(
            text("collab.config.automation.viewAll"),
            Some("查看全部({count})")
        );
        assert_eq!(text("collab.config.experts.addAction"), Some("+ 添加"));
        assert_eq!(
            text("collab.config.experts.description"),
            Some("配置项目专家，为成员提供更专业的服务")
        );
        assert_eq!(text("collab.config.experts.manageAria"), Some("管理专家"));
        assert_eq!(text("collab.config.experts.title"), Some("专家"));
        assert_eq!(
            text("collab.config.expertsDrawer.addAction"),
            Some("添加专家")
        );
        assert_eq!(
            text("collab.config.expertsDrawer.addActionShort"),
            Some("添加")
        );
        assert_eq!(text("collab.config.expertsDrawer.close"), Some("关闭"));
        // 全量覆盖且不越命名空间；用运行时绑定绕开 clippy::assertions_on_constants。
        let covered = TEXTS.len();
        let total = NS_KEY_COUNT;
        assert_eq!(covered, total);
        let prefix = NS_PREFIX.to_string();
        assert!(TEXTS.iter().all(|(k, _)| k.starts_with(&prefix)));
        // 键不重复。
        let mut keys: Vec<_> = TEXTS.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n);
    }

    #[test]
    fn texts_no_brand_words() {
        assert!(
            TEXTS
                .iter()
                .all(|(_, v)| !v.contains("飞书") && !v.contains("Lark")),
            "文案含品牌词"
        );
    }

    #[test]
    fn fail_closed_default_is_loading() {
        assert_eq!(State::default(), State::Loading);
        assert_eq!(super::resolve(None), State::Loading);
        assert_eq!(
            view_of(State::default(), &ConfigCounts::default()),
            View::Loading
        );
    }

    #[test]
    fn loaded_empty_is_empty_state_not_normal() {
        let ready = super::resolve(Some(true));
        let zero = ConfigCounts::default();
        assert_eq!(ready, State::Ready);
        assert_eq!(view_of(ready, &zero), View::Empty);
        assert_ne!(view_of(ready, &zero), View::Normal(&zero));
        assert_eq!(view_all_label(0), None);
        assert_eq!(view_all_label(3).as_deref(), Some("查看全部(3)"));
    }
}
