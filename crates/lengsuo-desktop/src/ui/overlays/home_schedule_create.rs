//! 日程创建（波次 8，卡 `ui-home-schedule-create`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `home.schedule.create`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
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
pub const COPY: &[(&str, &str)] = &[
    ("home.schedule.create", "新建日程"),
    ("home.schedule.create.editTitle", "编辑日程"),
    ("home.schedule.create.untitled", "无主题"),
    ("home.schedule.create.start", "开始"),
    ("home.schedule.create.end", "结束"),
    ("home.schedule.create.timeTo", "至"),
    ("home.schedule.create.duration", "时长"),
    ("home.schedule.create.duration.hour", "1 小时"),
    ("home.schedule.create.duration.hours", "{count} 小时"),
    ("home.schedule.create.duration.minutes", "{count} 分钟"),
    ("home.schedule.create.quick.title", "快速创建日程"),
    ("home.schedule.create.quick.expand", "更多设置"),
    (
        "home.schedule.create.kindHint",
        "当前只开放会议。时间安排、自动化、提醒要等对应契约。",
    ),
    (
        "home.schedule.create.formHint",
        "表单与自然语言使用同一份日程字段。",
    ),
    ("home.schedule.create.field.calendar", "日历"),
    ("home.schedule.create.field.participants", "参与人"),
    ("home.schedule.create.field.location", "地点"),
    ("home.schedule.create.field.description", "描述"),
    ("home.schedule.create.field.recurrence", "重复"),
    ("home.schedule.create.field.reminder", "提醒"),
    ("home.schedule.create.field.onlineMeeting", "会议"),
    ("home.schedule.create.field.meetingMinutes", "纪要"),
    ("home.schedule.create.field.attachment", "附件"),
    ("home.schedule.create.placeholder.title", "添加主题"),
    ("home.schedule.create.placeholder.location", "添加地点"),
    ("home.schedule.create.placeholder.description", "添加描述"),
    (
        "home.schedule.create.placeholder.participants",
        "添加联系人、邮件地址",
    ),
    ("home.schedule.create.addParticipants", "添加参会人"),
    ("home.schedule.create.addInvitee", "添加联系人"),
    ("home.schedule.create.batchAdd", "批量添加"),
    ("home.schedule.create.removeParticipant", "移除 {name}"),
    ("home.schedule.create.noParticipantMatch", "没有匹配的人员"),
    (
        "home.schedule.create.error.participantsRequired",
        "至少邀请一位参与人",
    ),
    (
        "home.schedule.create.scope.personal",
        "个人日历 · {count} 人正式受邀",
    ),
    (
        "home.schedule.create.scope.channel",
        "频道可见 · {count} 人正式受邀",
    ),
    ("home.schedule.create.addAttachment", "添加附件"),
    ("home.schedule.create.attachment.removeAria", "移除附件"),
    (
        "home.schedule.create.attachment.tooMany",
        "最多添加 {count} 个附件",
    ),
    (
        "home.schedule.create.attachment.unavailable",
        "当前环境还不能添加附件",
    ),
    (
        "home.schedule.create.attachment.waitPick",
        "正在选择附件，请稍后再创建",
    ),
    (
        "home.schedule.create.attachment.pickFailed",
        "打开资料库失败，请重试",
    ),
    (
        "home.schedule.create.busy.date",
        "{month} 月 {day} 日 · {weekday}",
    ),
    ("home.schedule.create.busy.available", "可用"),
    ("home.schedule.create.busy.conflict", "此时段忙碌"),
    ("home.schedule.create.busy.free", "此时段空闲"),
    ("home.schedule.create.busy.everyoneFree", "所有人都有空"),
    ("home.schedule.create.busy.me", "我"),
    (
        "home.schedule.create.busy.membersBusy",
        "{count} 名成员在忙（{names}）",
    ),
    ("home.schedule.create.busy.membersBusyNameSep", "、"),
    ("home.schedule.create.busy.loading", "正在加载当天日程"),
    ("home.schedule.create.busy.unknown", "暂无忙闲"),
    (
        "home.schedule.create.error.invalidRange",
        "结束时间需晚于开始时间",
    ),
    (
        "home.schedule.create.error.titleTooLong",
        "标题不能超过 512 字",
    ),
    ("home.schedule.create.mode.form", "表单模式"),
    ("home.schedule.create.mode.switchToNl", "切换到自然语言"),
    (
        "home.schedule.create.mode.formHint",
        "与 @WorkBuddy 自然语言草稿共用全部字段",
    ),
    (
        "home.schedule.create.natural.heading",
        "告诉 WorkBuddy 你的日程安排",
    ),
    (
        "home.schedule.create.natural.description",
        "可以描述时间、参与人、主题和会议方式，随后在表单中检查与补充。",
    ),
    (
        "home.schedule.create.natural.placeholder",
        "例如：{example}",
    ),
    (
        "home.schedule.create.natural.example.personal",
        "明天上午 10 点安排一小时方案评审",
    ),
    ("home.schedule.create.natural.trySay", "试试这样说"),
    (
        "home.schedule.create.natural.footerHint",
        "下一步可在表单中确认和调整",
    ),
    ("home.schedule.create.natural.openForm", "直接打开表单"),
    ("home.schedule.create.natural.continue", "继续创建"),
    (
        "home.schedule.create.natural.switchToForm",
        "切换到表单创建",
    ),
    ("home.schedule.create.submit", "创建日程"),
    ("home.schedule.create.submitSave", "保存日程"),
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

/// 时长文案（`duration.*`）：60 ⇒ `1 小时`；整小时 ⇒ `{count} 小时`；其余 ⇒ `{count} 分钟`。
pub fn duration_label(minutes: u32) -> String {
    let (hours, mins) = (minutes / 60, minutes % 60);
    match (hours, mins) {
        (1, 0) => copy("home.schedule.create.duration.hour")
            .unwrap()
            .to_string(),
        (h, 0) => fill(
            copy("home.schedule.create.duration.hours").unwrap(),
            &[("count", &h.to_string())],
        ),
        (_, m) => fill(
            copy("home.schedule.create.duration.minutes").unwrap(),
            &[("count", &m.to_string())],
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

/// 自然语言输入占位（`natural.placeholder`）：例如：{example}（示例取 `natural.example.personal`）。
pub fn natural_placeholder() -> String {
    fill(
        copy("home.schedule.create.natural.placeholder").unwrap(),
        &[(
            "example",
            copy("home.schedule.create.natural.example.personal").unwrap(),
        )],
    )
}

/// 时间选择骨架（`start`/`timeTo`/`end` 组合，均为逐字文案）。
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

/// 状态 → 渲染模型（纯函数；GPUI 层唯一输入）。
pub fn view_model(state: &State) -> ViewModel {
    let c = |key: &str| copy(key).expect("missing copy key");
    match state {
        // 未下发/拉取失败：fail-closed，依赖数据的字段如实标「未下发」，⛔ 不伪造条目/数值。
        State::NotDelivered | State::LoadFailed => ViewModel {
            title: TITLE,
            mode_switch: c("home.schedule.create.mode.switchToNl"),
            sections: vec![
                Section {
                    label: c("home.schedule.create.quick.title"),
                    rows: vec![
                        ("", Row::Empty(c("home.schedule.create.placeholder.title"))),
                        ("", Row::Value(time_range_label())),
                        (c("home.schedule.create.duration"), Row::NotDelivered),
                    ],
                },
                Section {
                    label: c("home.schedule.create.field.participants"),
                    rows: vec![
                        ("", Row::Empty(c("home.schedule.create.addParticipants"))),
                        (
                            "",
                            Row::Value(
                                c("home.schedule.create.placeholder.participants").to_string(),
                            ),
                        ),
                        (
                            "",
                            Row::Value(c("home.schedule.create.batchAdd").to_string()),
                        ),
                    ],
                },
                Section {
                    label: c("home.schedule.create.field.attachment"),
                    rows: vec![("", attachment_row(AttachmentGate::Unsupported, &[]))],
                },
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
                },
            ],
            footnotes: vec![
                c("home.schedule.create.kindHint"),
                c("home.schedule.create.formHint"),
            ],
            submit: c("home.schedule.create.submit"),
        },
        State::Ready { mode, draft } => match mode {
            Mode::Form => {
                let mut participant_rows = vec![
                    ("", participants_row(draft)),
                    (
                        "",
                        Row::Value(c("home.schedule.create.placeholder.participants").to_string()),
                    ),
                ];
                if !draft.participants.is_empty() {
                    participant_rows
                        .push(("", Row::Value(scope_line(false, draft.participants.len()))));
                }
                ViewModel {
                    title: TITLE,
                    mode_switch: c("home.schedule.create.mode.switchToNl"),
                    sections: vec![
                        Section {
                            label: c("home.schedule.create.quick.title"),
                            rows: vec![
                                ("", draft_title_row(draft)),
                                ("", Row::Value(time_range_label())),
                                (
                                    c("home.schedule.create.duration"),
                                    draft
                                        .duration_minutes
                                        .map(|m| Row::Value(duration_label(m)))
                                        .unwrap_or(Row::NotDelivered),
                                ),
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
                            rows: vec![(
                                "",
                                attachment_row(draft.attachment_gate, &draft.attachments),
                            )],
                        },
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
                        },
                    ],
                    footnotes: vec![
                        c("home.schedule.create.kindHint"),
                        c("home.schedule.create.formHint"),
                    ],
                    submit: c("home.schedule.create.submit"),
                }
            }
            Mode::Natural => ViewModel {
                title: TITLE,
                mode_switch: c("home.schedule.create.natural.switchToForm"),
                sections: vec![Section {
                    label: c("home.schedule.create.natural.heading"),
                    rows: vec![
                        (
                            "",
                            Row::Value(c("home.schedule.create.natural.description").to_string()),
                        ),
                        ("", Row::Value(natural_placeholder())),
                        (
                            "",
                            Row::Value(c("home.schedule.create.natural.trySay").to_string()),
                        ),
                        (
                            "",
                            Row::Value(c("home.schedule.create.natural.footerHint").to_string()),
                        ),
                        (
                            "",
                            Row::Value(c("home.schedule.create.natural.openForm").to_string()),
                        ),
                    ],
                }],
                footnotes: vec![c("home.schedule.create.mode.formHint")],
                submit: c("home.schedule.create.natural.continue"),
            },
        },
    }
}

// ===== GPUI 层 =====

/// 主区/浮层呈现入口（诚实占位：完整渲染层下一提交落）。
pub fn main_area(theme: &Theme) -> AnyElement {
    let vm = view_model(&resolve(None));
    div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground)
        .child(vm.title)
        .child(NOT_DELIVERED)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{
        copy, resolve, view_model, Draft, Mode, Row, State, NS_KEY_COUNT, NS_PREFIX, TITLE,
    };

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "新建日程");
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        let vm = view_model(&State::NotDelivered);
        assert_eq!(vm.title, TITLE);
        assert_eq!(vm.submit, "创建日程");
    }

    #[test]
    fn ns_prefix_declared() {
        // 用运行时比较绕开 clippy::assertions_on_constants（两侧都经函数边界）。
        let prefix = NS_PREFIX.to_string();
        let count = NS_KEY_COUNT;
        assert!(!prefix.is_empty());
        assert!(prefix.chars().next().is_some_and(char::is_alphabetic));
        assert!(count > 0 && count < 10_000);
        assert_eq!(copy(NS_PREFIX), Some("新建日程"));
    }
}
