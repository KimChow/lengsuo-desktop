//! 自动化·模板（波次 9，卡 `ui-overlay-automation-templates`）。
//! 分层约定（同 `automation_ds_form`）：纯数据层（文案表/状态机/校验）先行，GPUI 层只做映射。
//! 硬约束：fail-closed；⛔ 不伪造模板数据；文案逐字取自命名空间 `automation.templates`（40 键全量）。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ── 纯数据层（无 GPUI 依赖） ─────────────────────────────────────────

/// 界面文案全表：命名空间 `automation.templates` 全 40 键（短名、中文），逐字取自文案全表；
/// 完整键 = `NS_PREFIX` + "." + 短名。
pub const TEXTS: [(&str, &str); 40] = [
    ("add", "添加"),
    (
        "bedtimeStory.content",
        "生成 3-5 分钟可读的温和睡前故事，情节完整并附简短寓意。",
    ),
    (
        "bedtimeStory.prompt",
        "写一个适合儿童的睡前故事，语言温和易懂，阅读时长约 3-5 分钟。故事需有完整情节，结尾附上简短寓意。结合 memory.md 里之前讲过的故事，尽量不要重复。",
    ),
    ("bedtimeStory.title", "每日儿童睡前故事"),
    (
        "classicMovie.content",
        "推荐一部高分经典电影，简要介绍剧情梗概、亮点与推荐理由，全程不剧透。",
    ),
    (
        "classicMovie.prompt",
        "给我推荐一部公认的经典电影（评分高、口碑好），简要介绍剧情梗概、亮点所在，以及它为什么值得一看，但是不要剧透。结合 memory.md 里之前推荐过的电影，尽量不要重复。",
    ),
    ("classicMovie.title", "经典电影推荐"),
    (
        "cutePetWallpaper.content",
        "随机从 7 种不同风格中挑选一种，为你生成一张 9:16 竖版高清萌宠手机壁纸。",
    ),
    // 全表源码中该值为 \u 转义（JS 运行时解码），此处按展示文案逐字落表。
    (
        "cutePetWallpaper.prompt",
        "帮我生成一张手机壁纸，以下主题随机选一个就可以。",
    ),
    ("cutePetWallpaper.title", "可爱萌宠手机壁纸"),
    (
        "dailyAiNews.content",
        "关注当天 AI 领域的重要动态，侧重 AI coding 与具身智能进展，筛选 3-5 条值得关注的信息。",
    ),
    (
        "dailyAiNews.prompt",
        "关注当天 AI 领域的重要动态，侧重 AI coding 与具身智能方向。筛选 3-5 条有价值的信息，简要说明事件内容及值得关注的原因。结合 memory.md 里之前推送过的内容，尽量不要重复。",
    ),
    ("dailyAiNews.title", "每日 AI 新闻推送"),
    (
        "dailyWhy.content",
        "每天抛出一个有趣问题，先提问再解答，语气轻松、通俗易懂，答案控制在 200-300 字。",
    ),
    (
        "dailyWhy.prompt",
        "随机挑选一个有趣的冷知识或生活百科问题，并给出详细、有趣、通俗易懂的解答。每次问题尽量不重复，覆盖科学、生活、历史、自然、食物、文化、动物、人体等多个领域；先抛出问题再揭晓答案；语气轻松有趣；答案控制在 200-300 字。",
    ),
    ("dailyWhy.title", "每日一个为什么"),
    (
        "dailyWords.content",
        "每天推荐 5 个高频实用英语单词，包含词义、音标、例句与记忆提示。",
    ),
    (
        "dailyWords.prompt",
        "每天推荐 5 个实用英语单词，优先选取生活和职场中常见词汇。输出词义、音标、例句，以及一条便于记忆的小提示。结合 memory.md 里之前推荐过的单词，尽量不要重复。",
    ),
    ("dailyWords.title", "每日 5 个英语单词"),
    ("emptyStateTitle", "试试这些常用任务"),
    (
        "healthCheckupReminder.content",
        "在 {date} {time} 提醒你确认体检时间、准备证件，并注意空腹与其他事项。",
    ),
    (
        "healthCheckupReminder.prompt",
        "在 {date} {time} 提醒我确认体检时间、准备证件，提前空腹并留意注意事项。",
    ),
    ("healthCheckupReminder.title", "体检预约提醒"),
    (
        "historyToday.content",
        "从科技、电影、音乐等领域挑选一件\"今天发生过\"的有趣事件，200-300 字讲清来龙去脉。",
    ),
    (
        "historyToday.prompt",
        "历史上的今天发生过什么有趣的事？从科技、电影、音乐等领域中挑一个，讲讲它的来龙去脉吧。控制在 200-300 字左右。",
    ),
    ("historyToday.title", "历史上的今天"),
    (
        "interviewPreparationReminder.content",
        "工作日每 2 小时提醒你复习大模型面试内容，并生成 3 个模拟问题。",
    ),
    (
        "interviewPreparationReminder.prompt",
        "每两小时提醒我复习关于大模型的项目亮点、技术难点、常见问答，并生成 3 个模拟面试问题。",
    ),
    (
        "interviewPreparationReminder.title",
        "面试准备提醒",
    ),
    (
        "parentContactReminder.content",
        "每周日 10:00 提醒你给家人打电话或发消息，简单问候近况。",
    ),
    (
        "parentContactReminder.prompt",
        "每周日十点提醒我给家人打电话或发消息，简单问候近况。",
    ),
    ("parentContactReminder.title", "父母联系提醒"),
    (
        "preMeetingPreparation.content",
        "在会议开始前提醒你整理议题、目标、待确认问题和关键结论。",
    ),
    (
        "preMeetingPreparation.prompt",
        "在会议开始前，提醒我整理议题、目标、需要确认的问题，以及要同步的关键结论。",
    ),
    ("preMeetingPreparation.title", "会议前准备"),
    ("sectionTitle", "定时任务模版"),
    ("title", "从模板入手"),
    (
        "weeklyReport.content",
        "每周五汇总仓库 PR 与 Issue 进展，输出关键变更与待关注事项。",
    ),
    (
        "weeklyReport.prompt",
        "梳理本周仓库中的 PR 与 Issue 情况，包括新增、已关闭及重点讨论。输出一份周报，涵盖主要进展、关键变更和待关注事项。",
    ),
    ("weeklyReport.title", "每周工作周报"),
];

/// 命名空间前缀。
pub const NS_PREFIX: &str = "automation.templates";

/// 浮层名（面板注册名；屏上标题用全表 `title` / `sectionTitle`）。
pub const PANEL_TITLE: &str = "自动化·模板";

/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

/// 加载中文案（逐字取自文案全表共享键 `common.loading`；命名空间内无加载键，
/// 按取色协议借用最近语义键，见状态文档）。
pub const LOADING_TEXT: &str = "加载中...";

/// 加载失败文案（逐字取自文案全表同族键 `automation.error.load`；借用同上）。
pub const LOAD_FAILED_TEXT: &str = "加载定时任务数据失败";

/// 按完整键（`前缀.短名`）取文案；未命中 ⇒ `None`（fail-closed，⛔ 不回退到自编文案）。
pub fn text(key: &str) -> Option<&'static str> {
    let name = key.strip_prefix(NS_PREFIX)?.strip_prefix('.')?;
    TEXTS.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

/// 模板条目：`id` 为短名前缀，三段文案按 `<id>.title/content/prompt` 从全表解析。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Template {
    /// 全表短名前缀（如 `bedtimeStory`）。
    pub id: &'static str,
}

impl Template {
    fn field(&self, suffix: &str) -> Option<&'static str> {
        text(&format!("{NS_PREFIX}.{}.{}", self.id, suffix))
    }

    /// 标题（`<id>.title`）；缺键 ⇒ `None`（fail-closed）。
    pub fn title(&self) -> Option<&'static str> {
        self.field("title")
    }

    /// 一句话说明（`<id>.content`）。
    pub fn content(&self) -> Option<&'static str> {
        self.field("content")
    }

    /// 提示词正文（`<id>.prompt`）。
    pub fn prompt(&self) -> Option<&'static str> {
        self.field("prompt")
    }
}

/// 全部模板（静态表，顺序 = 全表键序；共 12 个三段式模板）。
pub const ALL_TEMPLATES: [Template; 12] = [
    Template { id: "bedtimeStory" },
    Template { id: "classicMovie" },
    Template {
        id: "cutePetWallpaper",
    },
    Template { id: "dailyAiNews" },
    Template { id: "dailyWhy" },
    Template { id: "dailyWords" },
    Template {
        id: "healthCheckupReminder",
    },
    Template { id: "historyToday" },
    Template {
        id: "interviewPreparationReminder",
    },
    Template {
        id: "parentContactReminder",
    },
    Template {
        id: "preMeetingPreparation",
    },
    Template { id: "weeklyReport" },
];

/// 视图态状态机：加载中（默认；未拉取 ≠ 空集）/ 失败（fail-closed，可见报错）/
/// 空态（已拉取但为空）/ 正常。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TemplatesView {
    /// 未拉取过（⛔ 不等于空列表）。
    #[default]
    Loading,
    /// 拉取/校验失败：如实显示失败文案，不渲染半张表。
    Failed,
    /// 已拉取且为空。
    Empty,
    /// 已拉取且有模板。
    Ready,
}

/// 数据到达结果（对拉取的纯抽象，静态表亦走此通道）。
pub enum Delivery {
    /// 未拉取过。
    NotPulled,
    /// 拉取失败。
    LoadFailed,
    /// 已拉取（可能为空集）。
    Loaded(&'static [Template]),
}

/// 由数据到达结果解析视图态（纯函数）。
pub fn resolve(delivery: &Delivery) -> TemplatesView {
    match delivery {
        Delivery::NotPulled => TemplatesView::Loading,
        Delivery::LoadFailed => TemplatesView::Failed,
        Delivery::Loaded([]) => TemplatesView::Empty,
        Delivery::Loaded(_) => TemplatesView::Ready,
    }
}

/// 校验全表（fail-closed 闸门）：40 键、键值非空、键不重复、面板键齐、12 模板三段齐。
pub fn validate() -> Result<(), &'static str> {
    if TEXTS.len() != 40 {
        return Err("TEXTS must hold exactly 40 keys");
    }
    for (idx, (key, value)) in TEXTS.iter().enumerate() {
        if key.is_empty() || value.is_empty() {
            return Err("key/value must not be empty");
        }
        if TEXTS
            .iter()
            .enumerate()
            .any(|(j, (k, _))| j != idx && k == key)
        {
            return Err("duplicate key");
        }
    }
    for name in ["add", "title", "sectionTitle", "emptyStateTitle"] {
        if text(&format!("{NS_PREFIX}.{name}")).is_none() {
            return Err("panel key missing");
        }
    }
    for t in ALL_TEMPLATES {
        if t.title().is_none() || t.content().is_none() || t.prompt().is_none() {
            return Err("template field missing");
        }
    }
    Ok(())
}

/// 静态表的到达结果：校验失败 ⇒ 拉取失败（fail-closed，⛔ 不渲染半张表）。
pub fn static_delivery() -> Delivery {
    match validate() {
        Ok(()) => Delivery::Loaded(&ALL_TEMPLATES),
        Err(_) => Delivery::LoadFailed,
    }
}

/// 面板载体：视图态由数据到达结果解析（视图是它的纯函数）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TemplatesPanel {
    /// 当前视图态。
    pub view: TemplatesView,
}

impl TemplatesPanel {
    /// 未拉取的诚实构造（默认加载中，⛔ 不伪造模板）。
    pub fn new() -> Self {
        Self {
            view: TemplatesView::Loading,
        }
    }

    /// 由数据到达结果构造。
    pub fn from(delivery: &Delivery) -> Self {
        Self {
            view: resolve(delivery),
        }
    }

    /// 静态表面板（校验 fail-closed）。
    pub fn from_static() -> Self {
        Self::from(&static_delivery())
    }
}

// ── GPUI 层（颜色一律 theme token，⛔ 不硬编码色值） ─────────────────

/// 按短名取文案；未命中 ⇒ 「未下发」（fail-closed 兜底）。
fn ns(name: &str) -> &'static str {
    text(&format!("{NS_PREFIX}.{name}")).unwrap_or(NOT_DELIVERED)
}

/// 动作 chip（颜色一律 theme token）。
fn chip(theme: &Theme, label: impl Into<SharedString>) -> Div {
    div()
        .px_3()
        .py_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .text_color(theme.muted_foreground)
        .child(label.into())
}

/// 静态说明行（弱化文案）。
fn hint(theme: &Theme, line: impl Into<SharedString>) -> Div {
    div().text_color(theme.muted_foreground).child(line.into())
}

/// 纵向堆叠容器（子节点列表）。
fn stack(children: impl IntoIterator<Item = Div>) -> Div {
    div().flex().flex_col().gap_1().children(children)
}

/// 分区卡：标题行 + 主体（结构对齐 `automation_ds_form` 的 section）。
fn section(theme: &Theme, heading: impl Into<SharedString>, body: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(div().text_color(theme.foreground).child(heading.into()))
        .child(body)
}

/// 模板卡：标题（前景色）+ 一句话说明 + 提示词正文（弱化色）；
/// 缺键 ⇒ 「未下发」（fail-closed，⛔ 不静默缺行）。
fn template_card(theme: &Theme, tpl: &Template) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.border)
        .rounded_sm()
        .child(
            div()
                .text_color(theme.foreground)
                .child(tpl.title().unwrap_or(NOT_DELIVERED)),
        )
        .child(hint(theme, tpl.content().unwrap_or(NOT_DELIVERED)))
        .child(hint(theme, tpl.prompt().unwrap_or(NOT_DELIVERED)))
}

/// 模板列表区：分区标题（`sectionTitle`）+ 12 张模板卡 + 「添加」动作。
fn template_list(theme: &Theme) -> Div {
    let body =
        stack(ALL_TEMPLATES.iter().map(|t| template_card(theme, t))).child(chip(theme, ns("add")));
    section(theme, ns("sectionTitle"), body)
}

/// 分支渲染：加载中 / 失败（fail-closed，可见报错）/ 空态 / 正常（模板列表）。
pub fn render(theme: &Theme, panel: &TemplatesPanel) -> AnyElement {
    let heading = div().text_color(theme.foreground).child(ns("title"));
    let body = match panel.view {
        TemplatesView::Loading => hint(theme, LOADING_TEXT),
        TemplatesView::Failed => hint(theme, LOAD_FAILED_TEXT),
        TemplatesView::Empty => hint(theme, ns("emptyStateTitle")),
        TemplatesView::Ready => template_list(theme),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(heading)
        .child(body)
        .into_any_element()
}

/// 主区渲染（静态表；校验失败自动落入 fail-closed 分支，不渲染半张表）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render(theme, &TemplatesPanel::from_static())
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        main_area, resolve, text, Delivery, TemplatesPanel, TemplatesView, ALL_TEMPLATES,
        NS_PREFIX, TEXTS,
    };
    use gpui_kit::component::Theme;

    /// ① 全表 40 键、不重复、键值非空，短名拼前缀后仍可解析。
    #[test]
    fn table_has_40_unique_nonempty_keys() {
        assert_eq!(TEXTS.len(), 40);
        let mut keys: Vec<&str> = TEXTS.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), TEXTS.len());
        assert!(TEXTS.iter().all(|(k, v)| !k.is_empty() && !v.is_empty()));
        let first = TEXTS[0].0;
        assert_eq!(
            text(&format!("{NS_PREFIX}.{first}")).map(|v| v.to_string()),
            TEXTS
                .iter()
                .find(|(k, _)| *k == first)
                .map(|(_, v)| v.to_string())
        );
    }

    /// ③ 任务书 12 条逐字摘录 + 面板键，全部逐字一致（转义键按展示文案解码）。
    #[test]
    fn copy_is_verbatim_from_excerpts() {
        let t = |n: &str| text(&format!("{NS_PREFIX}.{n}"));
        assert_eq!(t("add"), Some("添加"));
        assert_eq!(
            t("bedtimeStory.content"),
            Some("生成 3-5 分钟可读的温和睡前故事，情节完整并附简短寓意。")
        );
        assert_eq!(
            t("bedtimeStory.prompt"),
            Some("写一个适合儿童的睡前故事，语言温和易懂，阅读时长约 3-5 分钟。故事需有完整情节，结尾附上简短寓意。结合 memory.md 里之前讲过的故事，尽量不要重复。")
        );
        assert_eq!(t("bedtimeStory.title"), Some("每日儿童睡前故事"));
        assert_eq!(
            t("classicMovie.content"),
            Some("推荐一部高分经典电影，简要介绍剧情梗概、亮点与推荐理由，全程不剧透。")
        );
        assert_eq!(
            t("classicMovie.prompt"),
            Some("给我推荐一部公认的经典电影（评分高、口碑好），简要介绍剧情梗概、亮点所在，以及它为什么值得一看，但是不要剧透。结合 memory.md 里之前推荐过的电影，尽量不要重复。")
        );
        assert_eq!(t("classicMovie.title"), Some("经典电影推荐"));
        assert_eq!(
            t("cutePetWallpaper.content"),
            Some("随机从 7 种不同风格中挑选一种，为你生成一张 9:16 竖版高清萌宠手机壁纸。")
        );
        assert_eq!(
            t("cutePetWallpaper.prompt"),
            Some("帮我生成一张手机壁纸，以下主题随机选一个就可以。")
        );
        assert_eq!(t("cutePetWallpaper.title"), Some("可爱萌宠手机壁纸"));
        assert_eq!(
            t("dailyAiNews.content"),
            Some("关注当天 AI 领域的重要动态，侧重 AI coding 与具身智能进展，筛选 3-5 条值得关注的信息。")
        );
        assert_eq!(
            t("dailyAiNews.prompt"),
            Some("关注当天 AI 领域的重要动态，侧重 AI coding 与具身智能方向。筛选 3-5 条有价值的信息，简要说明事件内容及值得关注的原因。结合 memory.md 里之前推送过的内容，尽量不要重复。")
        );
        assert_eq!(t("title"), Some("从模板入手"));
        assert_eq!(t("sectionTitle"), Some("定时任务模版"));
        assert_eq!(t("emptyStateTitle"), Some("试试这些常用任务"));
    }

    /// ② 状态机全分支：默认加载中（未拉取 ≠ 空集）；静态表校验通过 ⇒ 正常。
    #[test]
    fn resolve_covers_every_branch() {
        assert_eq!(TemplatesView::default(), TemplatesView::Loading);
        assert_eq!(resolve(&Delivery::NotPulled), TemplatesView::Loading);
        assert_eq!(resolve(&Delivery::LoadFailed), TemplatesView::Failed);
        assert_eq!(resolve(&Delivery::Loaded(&[])), TemplatesView::Empty);
        assert_eq!(
            resolve(&Delivery::Loaded(&ALL_TEMPLATES)),
            TemplatesView::Ready
        );
        assert_eq!(TemplatesPanel::new().view, TemplatesView::Loading);
        assert_eq!(TemplatesPanel::from_static().view, TemplatesView::Ready);
    }

    /// 校验通过 + 12 模板三段齐（含占位符模板原样保留）。
    #[test]
    fn validate_passes_and_templates_complete() {
        use super::validate;
        validate().expect("static table must validate");
        assert_eq!(ALL_TEMPLATES.len(), 12);
        for t in ALL_TEMPLATES {
            assert!(t.title().is_some(), "title missing: {}", t.id);
            assert!(t.content().is_some(), "content missing: {}", t.id);
            assert!(t.prompt().is_some(), "prompt missing: {}", t.id);
        }
        let h = ALL_TEMPLATES
            .iter()
            .find(|t| t.id == "healthCheckupReminder")
            .unwrap();
        assert_eq!(
            h.content(),
            Some("在 {date} {time} 提醒你确认体检时间、准备证件，并注意空腹与其他事项。")
        );
    }

    /// ③ 缺键 fail-closed：未知键 ⇒ `None`，不回退自编文案。
    #[test]
    fn missing_key_is_none_not_fabricated() {
        use super::{LOADING_TEXT, LOAD_FAILED_TEXT, NOT_DELIVERED};
        assert_eq!(text("automation.templates.nope"), None);
        assert_eq!(text("automation.templates"), None);
        assert_eq!(text("other.ns.add"), None);
        assert_eq!(LOADING_TEXT, "加载中...");
        assert_eq!(LOAD_FAILED_TEXT, "加载定时任务数据失败");
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    /// ④ `main_area(&Theme::default())` 不 panic（静态表 ⇒ 正常分支）。
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
