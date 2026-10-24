//! 日程创建（波次 8，卡 `ui-home-schedule-create`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `home.schedule.create`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字：`home.schedule.create` → 新建日程）。
pub const TITLE: &str = "新建日程";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀与键数（据文案全表，共 202 键；本卡先落主表单所需子集）。
pub const NS_PREFIX: &str = "home.schedule.create";
pub const NS_KEY_COUNT: usize = 202;

// ===== 纯数据层（无 GPUI 依赖）=====

/// 文案表（key → 中文，逐字取自文案全表命名空间 `home.schedule.create`；⛔ 不自编中文）。
// rustfmt::skip：中文按 2 列计宽，逐键单行才不会被拆成三行。
#[rustfmt::skip]
pub const COPY: &[(&str, &str)] = &[
    ("home.schedule.create", "新建日程"),
    ("home.schedule.create.start", "开始"),
    ("home.schedule.create.end", "结束"),
    ("home.schedule.create.timeTo", "至"),
    ("home.schedule.create.duration", "时长"),
    ("home.schedule.create.duration.hour", "1 小时"),
    ("home.schedule.create.duration.hours", "{count} 小时"),
    ("home.schedule.create.duration.minutes", "{count} 分钟"),
    ("home.schedule.create.quick.title", "快速创建日程"),
    ("home.schedule.create.quick.expand", "更多设置"),
    ("home.schedule.create.kindHint", "当前只开放会议。时间安排、自动化、提醒要等对应契约。"),
    ("home.schedule.create.formHint", "表单与自然语言使用同一份日程字段。"),
    ("home.schedule.create.field.participants", "参与人"),
    ("home.schedule.create.field.recurrence", "重复"),
    ("home.schedule.create.field.reminder", "提醒"),
    ("home.schedule.create.field.onlineMeeting", "会议"),
    ("home.schedule.create.field.meetingMinutes", "纪要"),
    ("home.schedule.create.field.attachment", "附件"),
    ("home.schedule.create.placeholder.title", "添加主题"),
    ("home.schedule.create.placeholder.participants", "添加联系人、邮件地址"),
    ("home.schedule.create.addParticipants", "添加参会人"),
    ("home.schedule.create.batchAdd", "批量添加"),
    ("home.schedule.create.error.participantsRequired", "至少邀请一位参与人"),
    ("home.schedule.create.scope.personal", "个人日历 · {count} 人正式受邀"),
    ("home.schedule.create.scope.channel", "频道可见 · {count} 人正式受邀"),
    ("home.schedule.create.addAttachment", "添加附件"),
    ("home.schedule.create.attachment.tooMany", "最多添加 {count} 个附件"),
    ("home.schedule.create.attachment.unavailable", "当前环境还不能添加附件"),
    ("home.schedule.create.attachment.waitPick", "正在选择附件，请稍后再创建"),
    ("home.schedule.create.attachment.pickFailed", "打开资料库失败，请重试"),
    ("home.schedule.create.busy.date", "{month} 月 {day} 日 · {weekday}"),
    ("home.schedule.create.busy.everyoneFree", "所有人都有空"),
    ("home.schedule.create.busy.me", "我"),
    ("home.schedule.create.busy.membersBusy", "{count} 名成员在忙（{names}）"),
    ("home.schedule.create.busy.membersBusyNameSep", "、"),
    ("home.schedule.create.busy.loading", "正在加载当天日程"),
    ("home.schedule.create.busy.unknown", "暂无忙闲"),
    ("home.schedule.create.error.invalidRange", "结束时间需晚于开始时间"),
    ("home.schedule.create.mode.switchToNl", "切换到自然语言"),
    ("home.schedule.create.mode.formHint", "与 @WorkBuddy 自然语言草稿共用全部字段"),
    ("home.schedule.create.natural.heading", "告诉 WorkBuddy 你的日程安排"),
    ("home.schedule.create.natural.description", "可以描述时间、参与人、主题和会议方式，随后在表单中检查与补充。"),
    ("home.schedule.create.natural.placeholder", "例如：{example}"),
    ("home.schedule.create.natural.example.personal", "明天上午 10 点安排一小时方案评审"),
    ("home.schedule.create.natural.trySay", "试试这样说"),
    ("home.schedule.create.natural.footerHint", "下一步可在表单中确认和调整"),
    ("home.schedule.create.natural.openForm", "直接打开表单"),
    ("home.schedule.create.natural.continue", "继续创建"),
    ("home.schedule.create.natural.switchToForm", "切换到表单创建"),
    ("home.schedule.create.submit", "创建日程"),
];

/// 浮层状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发，正常呈现（模式 + 草稿）。
    Ready { mode: Mode, draft: Draft },
    /// 拉取失败（fail-closed，必须显示错误）。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 创建模式：表单 / 自然语言（`mode.*`、`natural.*`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Form,
    Natural,
}

/// 创建草稿（空输入 ⇒ 空态，⛔ 不是空列表）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    /// 主题；空 ⇒ `placeholder.title` 空态。
    pub title: String,
    /// 参与人名；空 ⇒ `addParticipants` 空态。
    pub participants: Vec<String>,
    /// 已选附件名。
    pub attachments: Vec<String>,
    /// 时长（分钟）；`None` ⇒ 未下发（⛔ 不伪造默认时长）。
    pub duration_minutes: Option<u32>,
    /// 忙闲数据源；`None` ⇒ 未下发。
    pub busy: Option<BusyFeed>,
    /// 附件选择器门禁。
    pub attachment_gate: AttachmentGate,
}

/// 忙闲源（`busy.*`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusyFeed {
    /// `busy.loading`
    Loading,
    /// `busy.unknown`
    Unknown,
    /// `busy.everyoneFree`
    EveryoneFree,
    /// `busy.membersBusy`（{count}/{names}）
    MembersBusy { names: Vec<String> },
}

/// 附件选择器门禁（`attachment.*`）；上限计数由运行时给出，⛔ 不内置数值。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttachmentGate {
    /// `attachment.unavailable`（诚实默认：本复刻未接选择器）
    #[default]
    Unsupported,
    /// `attachment.waitPick`
    Picking,
    /// `attachment.pickFailed`
    Failed,
    /// `attachment.tooMany`（运行时计数）
    AtLimit { count: u32 },
    /// 可添加；未选 ⇒ `addAttachment` 空态。
    Ok,
}

/// 内容行输出：有值 / 空态（⛔ 不是空列表）/ 未下发。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Value(String),
    /// 空态提示（逐字文案）。
    Empty(&'static str),
    /// fail-closed：数据未下发。
    NotDelivered,
}

/// 渲染分区。
pub struct Section {
    /// 分区标题（逐字文案）。
    pub label: &'static str,
    /// 行：(行标签（逐字文案，可为空）, 行内容)。
    pub rows: Vec<(&'static str, Row)>,
}

/// 由 [`State`] 纯函数映射出的渲染模型（GPUI 层唯一输入）。
pub struct ViewModel {
    pub title: &'static str,
    /// 模式切换入口（`mode.switchToNl` / `natural.switchToForm`）。
    pub mode_switch: &'static str,
    pub sections: Vec<Section>,
    /// 脚注（`kindHint` / `formHint` / `mode.formHint`）。
    pub footnotes: Vec<&'static str>,
    /// 提交按钮（`submit` / `natural.continue`）。
    pub submit: &'static str,
}

/// 由运行时输入解析初始态（纯函数）。
pub fn resolve(delivered: Option<bool>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(false) => State::LoadFailed,
        Some(true) => State::Ready {
            mode: Mode::default(),
            draft: Draft::default(),
        },
    }
}

/// 文案查键（纯函数；键须在 [`COPY`] 表内且属本命名空间）。
pub fn copy(key: &str) -> Option<&'static str> {
    let dotted = format!("{NS_PREFIX}.");
    debug_assert!(key == NS_PREFIX || key.starts_with(&dotted));
    debug_assert!(COPY.len() <= NS_KEY_COUNT);
    COPY.iter().find(|(k, _)| *k == key).map(|(_, t)| *t)
}

/// 模板填槽：`{count}`/`{names}`/`{month}`… ← args（纯函数）。
fn fill(tpl: &str, args: &[(&str, &str)]) -> String {
    let mut out = tpl.to_string();
    for (slot, val) in args {
        out = out.replace(&format!("{{{slot}}}"), val);
    }
    out
}

/// 人名并列分隔（`busy.membersBusyNameSep`）。
fn join_names(names: &[String]) -> String {
    names.join(copy("home.schedule.create.busy.membersBusyNameSep").unwrap())
}

/// 时长文案：60 ⇒ `1 小时`；整小时 ⇒ `{count} 小时`；其余 ⇒ 总分钟数 `{count} 分钟`。
pub fn duration_label(minutes: u32) -> String {
    match (minutes / 60, minutes % 60) {
        (1, 0) => copy("home.schedule.create.duration.hour")
            .unwrap()
            .to_string(),
        (h, 0) => fill(
            copy("home.schedule.create.duration.hours").unwrap(),
            &[("count", &h.to_string())],
        ),
        _ => fill(
            copy("home.schedule.create.duration.minutes").unwrap(),
            &[("count", &minutes.to_string())],
        ),
    }
}

/// 可见范围行（`scope.*`）：`{count}` = 正式受邀人数（真实计数，⛔ 不伪造）。
pub fn scope_line(channel: bool, invited: usize) -> String {
    let key = if channel {
        "home.schedule.create.scope.channel"
    } else {
        "home.schedule.create.scope.personal"
    };
    fill(copy(key).unwrap(), &[("count", &invited.to_string())])
}

/// 忙闲日期头（`busy.date`）：`{month} 月 {day} 日 · {weekday}`。
pub fn busy_date(month: u32, day: u32, weekday: &str) -> String {
    fill(
        copy("home.schedule.create.busy.date").unwrap(),
        &[
            ("month", &month.to_string()),
            ("day", &day.to_string()),
            ("weekday", weekday),
        ],
    )
}

/// 参与人区输出：空输入 ⇒ 空态 `addParticipants`（⛔ 不是空列表）。
pub fn participants_row(draft: &Draft) -> Row {
    if draft.participants.is_empty() {
        Row::Empty(copy("home.schedule.create.addParticipants").unwrap())
    } else {
        Row::Value(join_names(&draft.participants))
    }
}

/// 忙闲汇总（`busy.*`）；`None` ⇒ 未下发（fail-closed）。
pub fn busy_row(feed: Option<&BusyFeed>) -> Row {
    let key = match feed {
        None => return Row::NotDelivered,
        Some(BusyFeed::Loading) => "home.schedule.create.busy.loading",
        Some(BusyFeed::Unknown) => "home.schedule.create.busy.unknown",
        Some(BusyFeed::EveryoneFree) => "home.schedule.create.busy.everyoneFree",
        Some(BusyFeed::MembersBusy { names }) => {
            return Row::Value(fill(
                copy("home.schedule.create.busy.membersBusy").unwrap(),
                &[
                    ("count", &names.len().to_string()),
                    ("names", &join_names(names)),
                ],
            ));
        }
    };
    Row::Value(copy(key).unwrap().to_string())
}

/// 附件区输出（`attachment.*`）：门禁优先；`Ok` 且未选 ⇒ `addAttachment` 空态。
pub fn attachment_row(gate: AttachmentGate, names: &[String]) -> Row {
    let key = match gate {
        AttachmentGate::Unsupported => "home.schedule.create.attachment.unavailable",
        AttachmentGate::Picking => "home.schedule.create.attachment.waitPick",
        AttachmentGate::Failed => "home.schedule.create.attachment.pickFailed",
        AttachmentGate::AtLimit { count } => {
            return Row::Value(fill(
                copy("home.schedule.create.attachment.tooMany").unwrap(),
                &[("count", &count.to_string())],
            ));
        }
        AttachmentGate::Ok if names.is_empty() => {
            return Row::Empty(copy("home.schedule.create.addAttachment").unwrap());
        }
        AttachmentGate::Ok => return Row::Value(join_names(names)),
    };
    Row::Value(copy(key).unwrap().to_string())
}

/// 自然语言输入占位：`natural.placeholder` 的 {example} 取 `natural.example.personal`。
pub fn natural_placeholder() -> String {
    fill(
        copy("home.schedule.create.natural.placeholder").unwrap(),
        &[(
            "example",
            copy("home.schedule.create.natural.example.personal").unwrap(),
        )],
    )
}

/// 时间选择骨架（`start`/`timeTo`/`end` 逐字文案组合）。
fn time_range_label() -> String {
    format!(
        "{} {} {}",
        copy("home.schedule.create.start").unwrap(),
        copy("home.schedule.create.timeTo").unwrap(),
        copy("home.schedule.create.end").unwrap()
    )
}

/// 主题行：有值显值，空输入 ⇒ `placeholder.title` 空态（⛔ 不是空字符串）。
fn draft_title_row(draft: &Draft) -> Row {
    if draft.title.is_empty() {
        Row::Empty(copy("home.schedule.create.placeholder.title").unwrap())
    } else {
        Row::Value(draft.title.clone())
    }
}

/// 「更多设置」分区：重复/提醒/会议/纪要——设置数据未接线，如实标「未下发」。
fn settings_section() -> Section {
    let c = |key: &str| copy(key).expect("missing copy key");
    Section {
        label: c("home.schedule.create.quick.expand"),
        rows: vec![
            (
                c("home.schedule.create.field.recurrence"),
                Row::NotDelivered,
            ),
            (c("home.schedule.create.field.reminder"), Row::NotDelivered),
            (
                c("home.schedule.create.field.onlineMeeting"),
                Row::NotDelivered,
            ),
            (
                c("home.schedule.create.field.meetingMinutes"),
                Row::NotDelivered,
            ),
        ],
    }
}

/// 状态 → 渲染模型（纯函数；GPUI 层唯一输入）。
pub fn view_model(state: &State) -> ViewModel {
    let c = |key: &str| copy(key).expect("missing copy key");
    match state {
        // 未下发/拉取失败：fail-closed。诚实呈现 = 空草稿表单骨架，
        // 数据位（时长/忙闲/更多设置）一律标「未下发」，⛔ 不伪造条目/数值。
        State::NotDelivered | State::LoadFailed => view_model(&State::Ready {
            mode: Mode::Form,
            draft: Draft::default(),
        }),
        State::Ready { mode, draft } => match mode {
            Mode::Form => {
                let ph = c("home.schedule.create.placeholder.participants");
                let mut participant_rows = vec![
                    ("", participants_row(draft)),
                    ("", Row::Value(ph.to_string())),
                ];
                if !draft.participants.is_empty() {
                    participant_rows
                        .push(("", Row::Value(scope_line(false, draft.participants.len()))));
                }
                let duration_row = draft
                    .duration_minutes
                    .map(|m| Row::Value(duration_label(m)))
                    .unwrap_or(Row::NotDelivered);
                let attachments = attachment_row(draft.attachment_gate, &draft.attachments);
                ViewModel {
                    title: TITLE,
                    mode_switch: c("home.schedule.create.mode.switchToNl"),
                    sections: vec![
                        Section {
                            label: c("home.schedule.create.quick.title"),
                            rows: vec![
                                ("", draft_title_row(draft)),
                                ("", Row::Value(time_range_label())),
                                (c("home.schedule.create.duration"), duration_row),
                                (
                                    c("home.schedule.create.busy.me"),
                                    busy_row(draft.busy.as_ref()),
                                ),
                            ],
                        },
                        Section {
                            label: c("home.schedule.create.field.participants"),
                            rows: participant_rows,
                        },
                        Section {
                            label: c("home.schedule.create.field.attachment"),
                            rows: vec![("", attachments)],
                        },
                        settings_section(),
                    ],
                    footnotes: vec![
                        c("home.schedule.create.kindHint"),
                        c("home.schedule.create.formHint"),
                    ],
                    submit: c("home.schedule.create.submit"),
                }
            }
            Mode::Natural => {
                let (desc, try_say, footer, open_form) = (
                    c("home.schedule.create.natural.description"),
                    c("home.schedule.create.natural.trySay"),
                    c("home.schedule.create.natural.footerHint"),
                    c("home.schedule.create.natural.openForm"),
                );
                ViewModel {
                    title: TITLE,
                    mode_switch: c("home.schedule.create.natural.switchToForm"),
                    sections: vec![Section {
                        label: c("home.schedule.create.natural.heading"),
                        rows: vec![
                            ("", Row::Value(desc.to_string())),
                            ("", Row::Value(natural_placeholder())),
                            ("", Row::Value(try_say.to_string())),
                            ("", Row::Value(footer.to_string())),
                            ("", Row::Value(open_form.to_string())),
                        ],
                    }],
                    footnotes: vec![c("home.schedule.create.mode.formHint")],
                    submit: c("home.schedule.create.natural.continue"),
                }
            }
        },
    }
}

// ===== GPUI 层 =====

/// 主区/浮层呈现入口。无数据源 ⇒ `resolve(None)` 如实落到未下发（fail-closed，⛔ 不伪造条目）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render(theme, &view_model(&resolve(None)))
}

/// 渲染模型 → 元素树（纯构建，不持实体/凭据；颜色一律主题 token，⛔ 不硬编码色值）。
fn render(theme: &Theme, vm: &ViewModel) -> AnyElement {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        // 标题行：标题 + 模式切换入口（文案即无障碍标签）。
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
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(vm.mode_switch),
                ),
        );
    for section in &vm.sections {
        let mut sec = div().flex().flex_col().gap_1().px_4().child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(section.label),
        );
        for (label, row) in &section.rows {
            sec = sec.child(row_view(theme, label, row));
        }
        col = col.child(sec);
    }
    for note in &vm.footnotes {
        col = col.child(
            div()
                .px_4()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(*note),
        );
    }
    col.child(
        div().flex().flex_row().justify_end().px_4().py_2().child(
            div()
                .px_4()
                .py_2()
                .rounded_md()
                .border_1()
                .border_color(theme.accent)
                .text_sm()
                .text_color(theme.foreground)
                .child(vm.submit),
        ),
    )
    .into_any_element()
}

/// 单行（行标签 + 内容）：`Value` 前景 / `Empty` 弱化占位 / `NotDelivered` 以警示色如实标出。
fn row_view(theme: &Theme, label: &str, row: &Row) -> Div {
    let (text, color) = match row {
        Row::Value(v) => (v.clone(), theme.foreground),
        Row::Empty(hint) => ((*hint).to_string(), theme.muted_foreground),
        Row::NotDelivered => (NOT_DELIVERED.to_string(), theme.danger),
    };
    div()
        .flex()
        .flex_row()
        .justify_between()
        .gap_2()
        .text_sm()
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(label.to_string()),
        )
        .child(div().text_color(color).child(text))
}

/// 逐字断言用例（key → 期望中文，均取自文案全表）。
#[cfg(test)]
#[rustfmt::skip]
const VERBATIM: &[(&str, &str)] = &[
    ("home.schedule.create", "新建日程"),
    ("home.schedule.create.attachment.tooMany", "最多添加 {count} 个附件"),
    ("home.schedule.create.busy.membersBusy", "{count} 名成员在忙（{names}）"),
    ("home.schedule.create.natural.heading", "告诉 WorkBuddy 你的日程安排"),
    ("home.schedule.create.error.invalidRange", "结束时间需晚于开始时间"),
    ("home.schedule.create.error.participantsRequired", "至少邀请一位参与人"),
];

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    // 覆盖卡要求的五类：③ 文案逐字 / ① 纯函数 / ② 空态 / ④ main_area / ⑤ 分支。
    use super::{
        attachment_row, busy_date, busy_row, copy, duration_label, main_area, natural_placeholder,
        participants_row, resolve, scope_line, view_model, AttachmentGate, BusyFeed, Draft, Mode,
        Row, State, ViewModel, COPY, NS_KEY_COUNT, NS_PREFIX, TITLE, VERBATIM,
    };
    use gpui_kit::component::Theme;

    /// 门禁 → 未选附件时的行输出。
    fn gate(g: AttachmentGate) -> Row {
        attachment_row(g, &[])
    }

    /// 渲染模型里是否存在某行内容。
    fn has_row(vm: &ViewModel, want: &Row) -> bool {
        vm.sections
            .iter()
            .any(|s| s.rows.iter().any(|(_, r)| r == want))
    }

    /// ③ 文案逐字（≥3 键）+ 全表键数与命名空间前缀约束。
    #[test]
    fn copy_verbatim() {
        for &(k, want) in VERBATIM {
            assert_eq!(copy(k), Some(want), "文案不符: {k}");
        }
        let n = COPY.len();
        assert!((40..=NS_KEY_COUNT).contains(&n));
        for (k, _) in COPY {
            assert!(*k == NS_PREFIX || k.starts_with("home.schedule.create."));
        }
    }

    /// ① 纯函数层：状态解析、时长、计数、模板填槽。
    #[test]
    fn pure_counts_and_templates() {
        assert_eq!(TITLE, "新建日程");
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(duration_label(60), "1 小时");
        assert_eq!(duration_label(120), "2 小时");
        assert_eq!(duration_label(90), "90 分钟");
        assert_eq!(scope_line(false, 3), "个人日历 · 3 人正式受邀");
        assert_eq!(scope_line(true, 12), "频道可见 · 12 人正式受邀");
        assert_eq!(busy_date(10, 24, "周五"), "10 月 24 日 · 周五");
        assert_eq!(
            natural_placeholder(),
            "例如：明天上午 10 点安排一小时方案评审"
        );
        let names = vec!["李昊".to_string(), "王五".to_string()];
        let members = BusyFeed::MembersBusy { names };
        assert_eq!(
            busy_row(Some(&members)),
            Row::Value("2 名成员在忙（李昊、王五）".to_string())
        );
    }

    /// ② 空输入 ⇒ 空态（⛔ 不是空列表）；无源 ⇒ 未下发。
    #[test]
    fn empty_input_is_empty_state_not_empty_list() {
        let empty = Draft::default();
        assert_eq!(participants_row(&empty), Row::Empty("添加参会人"));
        assert_eq!(gate(AttachmentGate::Ok), Row::Empty("添加附件"));
        assert_eq!(busy_row(None), Row::NotDelivered);
        let vm = view_model(&State::Ready {
            mode: Mode::Form,
            draft: empty,
        });
        assert!(has_row(&vm, &Row::Empty("添加参会人")));
    }

    /// ④ main_area 只构建元素树，不 panic。
    #[test]
    fn main_area_builds_tree_only() {
        let _ = main_area(&Theme::default());
    }

    /// 附件门禁各态文案逐字（上限计数由调用方给出，⛔ 不内置数值）。
    #[test]
    fn attachment_gates_verbatim() {
        let cases = [
            (AttachmentGate::Unsupported, "当前环境还不能添加附件"),
            (AttachmentGate::Picking, "正在选择附件，请稍后再创建"),
            (AttachmentGate::Failed, "打开资料库失败，请重试"),
            (AttachmentGate::AtLimit { count: 9 }, "最多添加 9 个附件"),
        ];
        for (g, want) in cases {
            assert_eq!(gate(g), Row::Value(want.to_string()));
        }
        let picked = vec!["纪要.pdf".to_string()];
        assert_eq!(
            attachment_row(AttachmentGate::Ok, &picked),
            Row::Value("纪要.pdf".to_string())
        );
    }

    /// ⑤-1 NotDelivered（默认态）：数据位如实标未下发。
    #[test]
    fn branch_not_delivered_is_default_and_fail_closed() {
        assert_eq!(State::default(), State::NotDelivered);
        let vm = view_model(&State::NotDelivered);
        assert_eq!(vm.title, "新建日程");
        assert_eq!(vm.submit, "创建日程");
        assert!(has_row(&vm, &Row::NotDelivered));
    }

    /// ⑤-2 LoadFailed（拉取失败同样 fail-closed）。
    #[test]
    fn branch_load_failed_stays_fail_closed() {
        assert_eq!(resolve(Some(false)), State::LoadFailed);
        let vm = view_model(&State::LoadFailed);
        assert_eq!(vm.submit, "创建日程");
        assert!(has_row(&vm, &Row::NotDelivered));
    }

    /// ⑤-3 Ready·表单：草稿字段逐项落到行。
    #[test]
    fn branch_ready_form_fills_draft() {
        let draft = Draft {
            title: "需求评审".to_string(),
            participants: vec!["李昊".to_string()],
            duration_minutes: Some(30),
            busy: Some(BusyFeed::EveryoneFree),
            ..Draft::default()
        };
        let vm = view_model(&State::Ready {
            mode: Mode::Form,
            draft,
        });
        for want in [
            "需求评审",
            "李昊",
            "30 分钟",
            "所有人都有空",
            "个人日历 · 1 人正式受邀",
        ] {
            assert!(has_row(&vm, &Row::Value(want.to_string())));
        }
        assert_eq!(vm.mode_switch, "切换到自然语言");
    }

    /// ⑤-4 Ready·自然语言：整段走 `natural.*` 文案。
    #[test]
    fn branch_ready_natural_uses_natural_copy() {
        let vm = view_model(&State::Ready {
            mode: Mode::Natural,
            draft: Draft::default(),
        });
        assert_eq!(vm.sections[0].label, "告诉 WorkBuddy 你的日程安排");
        assert_eq!(vm.mode_switch, "切换到表单创建");
        assert_eq!(vm.submit, "继续创建");
        let example = Row::Value("例如：明天上午 10 点安排一小时方案评审".to_string());
        assert!(has_row(&vm, &example));
    }
}
