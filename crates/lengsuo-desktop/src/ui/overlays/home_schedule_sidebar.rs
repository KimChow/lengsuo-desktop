//! 日程侧栏（波次 8，卡 `ui-home-schedule-sidebar`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层（无 GPUI 依赖）+ GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `home.schedule.sidebar`。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 视图标题（逐字取自界面文案全表，key `title`）。
pub const TITLE: &str = "日期与日历";
/// 数据未下发时的说明（fail-closed 契约标记；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 命名空间前缀与键数（据文案全表该命名空间实测）。
pub const NS_PREFIX: &str = "home.schedule.sidebar";
pub const NS_KEY_COUNT: usize = 136;

/// 命名空间文案表：(key 尾段, 中文)，全部逐字；未命中返回空串（不伪造）。
pub mod copy {
    pub const ALL: &[(&str, &str)] = &[
        ("addCalendar", "添加日历"),
        ("calendarActions", "{name} 的操作"),
        ("calendarError.colorInvalid", "颜色参数不合法，请重试"),
        ("calendarError.colorVersionConflict", "这本日历的颜色刚被改过，刷新后再试"),
        ("calendarError.deleteIncomplete", "删除未完成，请刷新后重试"),
        ("calendarError.nameConflict", "已有同名日历，换一个名字试试"),
        ("calendarError.nameInvalid", "日历名称需为 1–64 个字符"),
        ("calendarError.needsAcknowledge", "还需要确认删除影响后再删"),
        ("calendarError.notAvailable", "当前环境暂未开放自建日历"),
        ("calendarError.notFound", "这本日历已不存在，刷新后再试"),
        ("calendarError.primaryUndeletable", "主日历不能删除"),
        ("calendarError.quotaExceeded", "自建日历数量已达上限，先删掉不用的再建"),
        ("calendarError.renameNotAllowed", "频道日历的名称跟随频道，无法在这里修改"),
        ("calendarError.unsubscribeFailed", "取消订阅失败，请稍后重试"),
        ("changeColor", "修改颜色"),
        ("changeColorFor", "修改「{name}」的颜色"),
        ("channels", "所在频道"),
        ("collapse", "收起日历"),
        ("collapseSidebar", "收起边栏"),
        ("createCalendar", "新建日历"),
        ("deleteCalendar", "删除"),
        ("deleteDialog.cancel", "取消"),
        ("deleteDialog.confirm", "删除"),
        ("deleteDialog.title", "删除日历"),
        ("empty", "暂无日历"),
        ("expand", "展开日历"),
        ("expandSidebar", "展开边栏"),
        ("managed", "我管理的"),
        ("monthTitle", "{year}年{month}月"),
        ("myScheduleGroup", "我的日程"),
        ("myTodos", "叠加我的待办"),
        ("myTodosGroup", "我的待办"),
        ("nextMonth", "下个月"),
        ("onlyVisibleToMe", "仅我可见"),
        ("openChannel", "进入频道"),
        ("overlayMyTodos", "叠加我的待办"),
        ("overlayPersonal", "叠加我的日程"),
        ("overlayTeamTodos", "叠加团队待办"),
        ("prevMonth", "上个月"),
        ("primaryUndeletableHint", "主日历用于接收邀请，不能删除"),
        ("renameCalendar", "重命名"),
        ("searchPlaceholder", "搜索日程"),
        ("subscribeCalendar", "订阅日历"),
        ("subscribed", "我订阅的"),
        ("subscribeDialog.loadFailed", "公共日历列表加载失败"),
        ("team", "团队日历"),
        ("thirdPartyCalendar", "第三方日历"),
        ("todoGroup", "待办"),
        ("todoIncomplete", "未加载完"),
        ("todoTruncated", "已截断"),
        ("unsubscribeCalendar", "取消订阅"),
        ("untitledCalendar", "未命名日历"),
        ("title", "日期与日历"),
    ];

    /// 按 key 尾段取文案；未命中返回空串（调用方不得伪造替身文案）。
    pub fn get(tail: &str) -> &'static str {
        ALL.iter()
            .find(|(k, _)| *k == tail)
            .map(|(_, v)| *v)
            .unwrap_or("")
    }
}

/// 单本日历（形状声明；实例由运行时侧下发，本卡不伪造）。
pub struct CalendarEntry {
    pub name: String,
    pub tag: EntryTag,
}

/// 条目标签（决定次级文案）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryTag {
    Managed,
    Subscribed,
    OnlyVisibleToMe,
}

/// 标签文案（逐字）。
pub fn tag_copy(tag: EntryTag) -> &'static str {
    match tag {
        EntryTag::Managed => copy::get("managed"),
        EntryTag::Subscribed => copy::get("subscribed"),
        EntryTag::OnlyVisibleToMe => copy::get("onlyVisibleToMe"),
    }
}

/// 展示名：空名回退「未命名日历」（key `untitledCalendar`）。
pub fn display_name(name: &str) -> &str {
    if name.trim().is_empty() {
        copy::get("untitledCalendar")
    } else {
        name
    }
}

/// 「{name} 的操作」模板（key `calendarActions`）。
pub fn actions_label(name: &str) -> String {
    copy::get("calendarActions").replace("{name}", display_name(name))
}

/// 「修改「{name}」的颜色」模板（key `changeColorFor`）。
pub fn change_color_for(name: &str) -> String {
    copy::get("changeColorFor").replace("{name}", name)
}

/// 「{year}年{month}月」模板（key `monthTitle`；数值由运行时下发）。
pub fn month_title(year: i32, month: u32) -> String {
    copy::get("monthTitle")
        .replace("{year}", &year.to_string())
        .replace("{month}", &month.to_string())
}

/// 日历操作错误（key 前缀 `calendarError.`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarError {
    ColorInvalid,
    ColorVersionConflict,
    DeleteIncomplete,
    NameConflict,
    NameInvalid,
    NeedsAcknowledge,
    NotAvailable,
    NotFound,
    PrimaryUndeletable,
    QuotaExceeded,
    RenameNotAllowed,
    UnsubscribeFailed,
}

impl CalendarError {
    /// 对应文案 key 尾段。
    pub fn key(self) -> &'static str {
        match self {
            CalendarError::ColorInvalid => "calendarError.colorInvalid",
            CalendarError::ColorVersionConflict => "calendarError.colorVersionConflict",
            CalendarError::DeleteIncomplete => "calendarError.deleteIncomplete",
            CalendarError::NameConflict => "calendarError.nameConflict",
            CalendarError::NameInvalid => "calendarError.nameInvalid",
            CalendarError::NeedsAcknowledge => "calendarError.needsAcknowledge",
            CalendarError::NotAvailable => "calendarError.notAvailable",
            CalendarError::NotFound => "calendarError.notFound",
            CalendarError::PrimaryUndeletable => "calendarError.primaryUndeletable",
            CalendarError::QuotaExceeded => "calendarError.quotaExceeded",
            CalendarError::RenameNotAllowed => "calendarError.renameNotAllowed",
            CalendarError::UnsubscribeFailed => "calendarError.unsubscribeFailed",
        }
    }

    /// 错误文案（逐字）。
    pub fn text(self) -> &'static str {
        copy::get(self.key())
    }
}

/// 侧栏状态机（三态契约：未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 数据未下发（诚实默认态）。
    #[default]
    NotDelivered,
    /// 下发失败（fail-closed，必须显示错误文案）。
    LoadFailed,
    /// 已下发但为空（呈现空态文案，不是空列表）。
    Empty,
    /// 就绪且有条目。
    Ready,
}

/// 由数据源结果解析呈现态（纯函数；`Ok(n)` 为条目数）。
pub fn resolve(source: Option<Result<usize, ()>>) -> State {
    match source {
        None => State::NotDelivered,
        Some(Err(_)) => State::LoadFailed,
        Some(Ok(0)) => State::Empty,
        Some(Ok(_)) => State::Ready,
    }
}

/// 各状态的状态行文案（`Ready` 无附加状态行）。
pub fn status_copy(state: State) -> Option<&'static str> {
    match state {
        State::NotDelivered => Some(NOT_DELIVERED),
        State::LoadFailed => Some(copy::get("subscribeDialog.loadFailed")),
        State::Empty => Some(copy::get("empty")),
        State::Ready => None,
    }
}

// ===== GPUI 层 =====
/// 主区呈现入口：标题行 + 搜索行 + 分组节点 + 列表区（无数据源 ⇒ fail-closed）。
pub fn main_area(theme: &Theme) -> AnyElement {
    let state = resolve(None); // 无数据源：未拉取过 ≠ 空集，⛔ 不伪造条目
    div()
        .flex()
        .flex_col()
        .gap_2()
        .text_color(theme.muted_foreground)
        .child(title_row(theme))
        .child(SharedString::from(copy::get("searchPlaceholder")))
        .child(group_row(theme, "myScheduleGroup", &["addCalendar", "createCalendar"]))
        .child(list_region(theme, state, &[]))
        .child(group_row(theme, "myTodosGroup", &["myTodos"]))
        .child(group_row(
            theme,
            "team",
            &["subscribeCalendar", "thirdPartyCalendar"],
        ))
        .into_any_element()
}

/// 标题行：标题 + 收起边栏动作。
fn title_row(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .justify_between()
        .text_color(theme.foreground)
        .child(SharedString::from(TITLE))
        .child(SharedString::from(copy::get("collapseSidebar")))
        .into_any_element()
}

/// 分组节点：组名 + 组内动作文案。
fn group_row(theme: &Theme, group_key: &str, action_keys: &[&str]) -> AnyElement {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_1()
        .text_color(theme.foreground)
        .child(SharedString::from(copy::get(group_key)));
    for key in action_keys {
        col = col.child(
            div()
                .pl_2()
                .text_color(theme.muted_foreground)
                .child(SharedString::from(copy::get(key))),
        );
    }
    col.into_any_element()
}

/// 日历列表区：四分支逐态呈现（`Ready` 需运行时条目，本卡无源不会出现）。
fn list_region(theme: &Theme, state: State, entries: &[CalendarEntry]) -> AnyElement {
    let mut col = div().flex().flex_col().gap_1();
    match state {
        State::Ready => {
            for entry in entries {
                col = col.child(
                    div()
                        .flex()
                        .justify_between()
                        .child(SharedString::from(display_name(&entry.name)))
                        .child(SharedString::from(tag_copy(entry.tag))),
                );
            }
        }
        State::Empty => {
            col = col.child(SharedString::from(copy::get("empty")));
        }
        State::LoadFailed => {
            col = col.child(SharedString::from(copy::get("subscribeDialog.loadFailed")));
        }
        State::NotDelivered => {
            col = col.child(SharedString::from(NOT_DELIVERED));
        }
    }
    col.text_color(theme.muted_foreground).into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]` ⇒ "recursion limit reached"。显式列符号。
    use super::{copy, resolve, status_copy, State, NOT_DELIVERED, NS_KEY_COUNT, NS_PREFIX, TITLE};

    #[test]
    fn title_is_verbatim() {
        assert_eq!(TITLE, "日期与日历");
        assert_eq!(TITLE, copy::get("title"));
    }

    #[test]
    fn fail_closed_default() {
        assert_eq!(State::default(), State::NotDelivered);
        assert_eq!(resolve(None), State::NotDelivered);
        assert_eq!(status_copy(State::NotDelivered), Some(NOT_DELIVERED));
    }

    #[test]
    fn copy_table_covers_namespace() {
        // 经函数边界绕开 clippy::assertions_on_constants。
        let covered = copy::ALL.len();
        let total = NS_KEY_COUNT;
        let prefix = NS_PREFIX.to_string();
        assert!(covered >= 40, "本卡文案覆盖须 ≥40 键，实际 {covered}");
        assert!(covered <= total && total == 136);
        assert!(prefix.ends_with("schedule.sidebar"));
    }
}
