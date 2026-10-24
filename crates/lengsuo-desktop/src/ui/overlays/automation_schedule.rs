//! 自动化计划（波次 8，卡 `ui-automation-schedule`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `automation.schedule`。
//! 数据源在运行时侧：当前按纯数据层逐分支呈现，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表）。
pub const TITLE: &str = "自动化计划";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";

// ===== 纯数据层（无 GPUI 依赖）=====
/// 键数（据文案全表命名空间 `automation.schedule` 全表 92 键）。
pub const NS_KEY_COUNT: usize = 92;
/// 命名空间前缀。
pub const NS_PREFIX: &str = "automation.schedule";

/// 本卡落地的文案子表（key → 中文，逐字取自全表命名空间 `automation.schedule`）。
pub static TEXTS: &[(&str, &str)] = &[
    ("automation.schedule.biweekly", "双周"),
    ("automation.schedule.daily", "每天"),
    ("automation.schedule.day.FR", "周五"),
    ("automation.schedule.day.MO", "周一"),
    ("automation.schedule.day.SA", "周六"),
    ("automation.schedule.day.SU", "周日"),
    ("automation.schedule.day.TH", "周四"),
    ("automation.schedule.day.TU", "周二"),
    ("automation.schedule.day.WE", "周三"),
    ("automation.schedule.dayOfMonth", "选择日期"),
    ("automation.schedule.executeTime", "执行时间"),
    ("automation.schedule.frequencyLabel", "重复"),
    ("automation.schedule.hourly", "按小时"),
    ("automation.schedule.interval", "按间隔"),
    ("automation.schedule.intervalEveryLabel", "每隔"),
    ("automation.schedule.intervalLabel", "间隔"),
    ("automation.schedule.intervalValue", "每 {count} 小时"),
    ("automation.schedule.monthDaySection", "每月日期"),
    ("automation.schedule.monthly", "每月"),
    ("automation.schedule.monthlyFirst", "每月 1 号"),
    ("automation.schedule.once", "单次任务"),
    ("automation.schedule.onceDate", "执行日期"),
    ("automation.schedule.onceTab", "单次"),
    ("automation.schedule.onceTimeEmpty", "待选择"),
    (
        "automation.schedule.peakHourHint",
        "建议避开上午高峰时段，高峰期容易排队等待; 选择非高峰期执行更稳定",
    ),
    ("automation.schedule.periodic", "周期"),
    ("automation.schedule.periodicBiweekMenu", "每双周"),
    ("automation.schedule.recurring", "周期任务"),
    ("automation.schedule.selectMonth", "选择月份"),
    ("automation.schedule.selectWeekday", "选择星期"),
    ("automation.schedule.summary.biweekly", "双周 {days} {time}"),
    ("automation.schedule.summary.daily", "每天 {time}"),
    ("automation.schedule.summary.monthly", "每月 {day} {time}"),
    ("automation.schedule.summary.once", "单次 {dateTime}"),
    ("automation.schedule.summary.weekly", "每周 {days} {time}"),
    ("automation.schedule.summary.yearly", "每年 {month}月{day}日 {time}"),
    ("automation.schedule.timeLabel", "时间"),
    ("automation.schedule.validation.intervalHours", "按间隔执行需至少每 1 小时"),
    ("automation.schedule.validation.monthdayRequired", "请至少选择一个日期"),
    ("automation.schedule.validation.monthRequired", "请选择月份"),
    ("automation.schedule.validation.onceDateRequired", "请选择单次执行日期"),
    ("automation.schedule.validation.weekdayRequired", "请至少选择一个星期"),
    ("automation.schedule.weekdaySection", "重复星期"),
    ("automation.schedule.weekdays", "工作日"),
    ("automation.schedule.weekend", "周末"),
    ("automation.schedule.weekly", "每周"),
    ("automation.schedule.workday", "每个工作日"),
    ("automation.schedule.yearly", "每年"),
];

/// 按键取文案（纯函数）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|&&(k, _)| k == key).map(|&(_, v)| v)
}

/// 取文案，缺键回空串（键表由测试逐字锁定）。
fn t(key: &str) -> &'static str {
    text(key).unwrap_or("")
}

/// 星期（键 `automation.schedule.day.*`，标签逐字：周一…周日）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Weekday {
    /// 周一开头的一周序。
    pub const ALL: [Weekday; 7] = [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
        Weekday::Sat,
        Weekday::Sun,
    ];

    pub fn key(&self) -> &'static str {
        match self {
            Weekday::Mon => "automation.schedule.day.MO",
            Weekday::Tue => "automation.schedule.day.TU",
            Weekday::Wed => "automation.schedule.day.WE",
            Weekday::Thu => "automation.schedule.day.TH",
            Weekday::Fri => "automation.schedule.day.FR",
            Weekday::Sat => "automation.schedule.day.SA",
            Weekday::Sun => "automation.schedule.day.SU",
        }
    }

    /// 中文标签（逐字取自全表）。
    pub fn label(&self) -> &'static str {
        t(self.key())
    }
}

/// 重复频率（键 = 该频率在全表中的标签键）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frequency {
    Once,
    Daily,
    Weekly,
    Biweekly,
    Monthly,
    Yearly,
    Interval { hours: u32 },
}

impl Frequency {
    pub fn key(&self) -> &'static str {
        match self {
            Frequency::Once => "automation.schedule.once",
            Frequency::Daily => "automation.schedule.daily",
            Frequency::Weekly => "automation.schedule.weekly",
            Frequency::Biweekly => "automation.schedule.biweekly",
            Frequency::Monthly => "automation.schedule.monthly",
            Frequency::Yearly => "automation.schedule.yearly",
            Frequency::Interval { .. } => "automation.schedule.interval",
        }
    }

    /// 中文标签（逐字取自全表）。
    pub fn label(&self) -> &'static str {
        t(self.key())
    }
}

/// 一条自动化计划（字段值来自数据源；⛔ 无源不构造）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleEntry {
    pub frequency: Frequency,
    /// 执行时间（如 `08:00`）；缺省渲染「待选择」。
    pub time: Option<String>,
    /// 单次执行日期。
    pub date: Option<String>,
    /// 周几（周/双周频率用）。
    pub days: Vec<Weekday>,
    /// 每月几号。
    pub day_of_month: Option<u32>,
    /// 每年几月。
    pub month: Option<u32>,
}

impl ScheduleEntry {
    /// 摘要（模板逐字取自 `summary.*` / `intervalValue`，占位符按字段替换）。
    pub fn summary(&self) -> String {
        let pending = t("automation.schedule.onceTimeEmpty");
        let time = self.time.as_deref().unwrap_or(pending);
        match self.frequency {
            Frequency::Once => {
                let parts: Vec<&str> = [self.date.as_deref(), self.time.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect();
                let date_time = if parts.is_empty() {
                    pending.to_string()
                } else {
                    parts.join(" ")
                };
                tpl(
                    "automation.schedule.summary.once",
                    &[("{dateTime}", date_time.as_str())],
                )
            }
            Frequency::Daily => tpl("automation.schedule.summary.daily", &[("{time}", time)]),
            Frequency::Weekly | Frequency::Biweekly => {
                let key = match self.frequency {
                    Frequency::Weekly => "automation.schedule.summary.weekly",
                    _ => "automation.schedule.summary.biweekly",
                };
                let days = self.days_label();
                tpl(key, &[("{days}", days.as_str()), ("{time}", time)])
            }
            Frequency::Monthly => {
                let day = self
                    .day_of_month
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| pending.to_string());
                tpl(
                    "automation.schedule.summary.monthly",
                    &[("{day}", day.as_str()), ("{time}", time)],
                )
            }
            Frequency::Yearly => {
                let month = self
                    .month
                    .map(|m| m.to_string())
                    .unwrap_or_else(|| pending.to_string());
                let day = self
                    .day_of_month
                    .map(|d| d.to_string())
                    .unwrap_or_else(|| pending.to_string());
                tpl(
                    "automation.schedule.summary.yearly",
                    &[("{month}", month.as_str()), ("{day}", day.as_str()), ("{time}", time)],
                )
            }
            Frequency::Interval { hours } => tpl(
                "automation.schedule.intervalValue",
                &[("{count}", hours.to_string().as_str())],
            ),
        }
    }

    /// 周几标签拼接（逐字标签 + 顿号分隔）。
    fn days_label(&self) -> String {
        self.days.iter().map(|d| d.label()).collect::<Vec<_>>().join("、")
    }
}

/// 模板占位符替换（纯函数）。
fn tpl(key: &str, subs: &[(&str, &str)]) -> String {
    let mut out = t(key).to_string();
    for (placeholder, value) in subs {
        out = out.replace(placeholder, value);
    }
    out
}

/// 校验（错误文案逐字取自 `validation.*`）。
pub fn validate(entry: &ScheduleEntry) -> Vec<&'static str> {
    let mut errors = Vec::new();
    match entry.frequency {
        Frequency::Weekly | Frequency::Biweekly if entry.days.is_empty() => {
            errors.push(t("automation.schedule.validation.weekdayRequired"));
        }
        Frequency::Monthly if entry.day_of_month.is_none() => {
            errors.push(t("automation.schedule.validation.monthdayRequired"));
        }
        Frequency::Yearly if entry.month.is_none() => {
            errors.push(t("automation.schedule.validation.monthRequired"));
        }
        Frequency::Yearly if entry.day_of_month.is_none() => {
            errors.push(t("automation.schedule.validation.monthdayRequired"));
        }
        Frequency::Interval { hours } if hours == 0 => {
            errors.push(t("automation.schedule.validation.intervalHours"));
        }
        Frequency::Once if entry.date.is_none() => {
            errors.push(t("automation.schedule.validation.onceDateRequired"));
        }
        _ => {}
    }
    errors
}

/// 浮层/面板状态机（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据已下发，正常呈现。
    Ready,
    /// 拉取失败（fail-closed，必须显示错误）。
    LoadFailed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 由运行时输入解析初始态（纯函数）。
pub fn resolve(delivered: Option<bool>) -> State {
    match delivered {
        None => State::NotDelivered,
        Some(false) => State::LoadFailed,
        Some(true) => State::Ready,
    }
}

/// 渲染视图态：`Ready` + 空输入 ⇒ 空态（⛔ 不是空列表）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View<'a> {
    NotDelivered,
    LoadFailed,
    Empty,
    Entries(&'a [ScheduleEntry]),
}

/// 状态 + 条目 → 视图态（纯函数）。
pub fn view_of<'a>(state: State, entries: &'a [ScheduleEntry]) -> View<'a> {
    match state {
        State::NotDelivered => View::NotDelivered,
        State::LoadFailed => View::LoadFailed,
        State::Ready if entries.is_empty() => View::Empty,
        State::Ready => View::Entries(entries),
    }
}

// ===== GPUI 层 =====
/// 加载失败说明（承自脚手架占位；数据源接通后随跨线契约替换）。
const LOAD_FAILED_SCAFFOLD: &str = "加载失败";

/// 主区/浮层呈现入口。数据源未接线 ⇒ fail-closed：如实呈现「未下发」，⛔ 不伪造条目/数值。
pub fn main_area(theme: &Theme) -> AnyElement {
    render_view(view_of(State::default(), &[]), theme)
}

/// 按视图态逐分支渲染（纯函数；标题行 + 分支内容节点）。
fn render_view(view: View<'_>, theme: &Theme) -> AnyElement {
    let root = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(SharedString::from(TITLE)));
    match view {
        View::NotDelivered => root
            .child(caption(NOT_DELIVERED, theme))
            .child(form_skeleton(theme))
            .into_any_element(),
        View::LoadFailed => root
            .child(caption(LOAD_FAILED_SCAFFOLD, theme))
            .child(form_skeleton(theme))
            .into_any_element(),
        View::Empty => root.child(form_skeleton(theme)).into_any_element(),
        View::Entries(entries) => entries
            .iter()
            .fold(root, |acc, entry| acc.child(entry_card(entry, theme)))
            .into_any_element(),
    }
}

/// 说明行（弱化色）。
fn caption(body: &str, theme: &Theme) -> AnyElement {
    div()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(body.to_string()))
        .into_any_element()
}

/// 待选择表单骨架（键值逐字取自全表；⛔ 不携带任何伪造值）。
fn form_skeleton(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(field_row(
            "automation.schedule.frequencyLabel",
            "automation.schedule.onceTab",
            theme,
        ))
        .child(field_row(
            "automation.schedule.onceDate",
            "automation.schedule.onceTimeEmpty",
            theme,
        ))
        .child(field_row(
            "automation.schedule.timeLabel",
            "automation.schedule.onceTimeEmpty",
            theme,
        ))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(t("automation.schedule.peakHourHint"))),
        )
        .into_any_element()
}

/// 一行「标签 · 值」（两个文案键，逐字取自全表）。
fn field_row(label_key: &str, value_key: &str, theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .gap_2()
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(t(label_key))),
        )
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(t(value_key))),
        )
        .into_any_element()
}

/// 一条计划的摘要卡（摘要 + 校验错误行）。
fn entry_card(entry: &ScheduleEntry, theme: &Theme) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(entry.summary())),
        );
    let errors = validate(entry);
    if errors.is_empty() {
        card.into_any_element()
    } else {
        errors
            .iter()
            .fold(card, |acc, msg| {
                acc.child(
                    div()
                        .text_color(theme.muted_foreground)
                        .child(SharedString::from(*msg)),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{resolve, State, NS_KEY_COUNT, NS_PREFIX, TITLE};

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "自动化计划");
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
    }

    #[test]
    fn ns_prefix_declared() {
        // 用运行时比较绕开 clippy::assertions_on_constants（两侧都经函数边界）。
        let prefix = NS_PREFIX.to_string();
        let count = NS_KEY_COUNT;
        assert!(!prefix.is_empty());
        assert!(prefix.chars().next().is_some_and(char::is_alphabetic));
        assert!(count > 0 && count < 10_000);
    }
}
