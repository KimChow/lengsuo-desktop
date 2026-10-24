//! @提及弹层（波次 8，卡 `ui-overlay-mention-popover`）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间。
//! 数据源在运行时侧：无数据时如实显示「未下发」。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 视图标题。
pub const TITLE: &str = "@提及弹层";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 助理页签名。
pub const COLLEAGUE_NAME: &str = "助理";
/// 助理页描述。
pub const COLLEAGUE_DESCRIPTION: &str = "选择助理协同推进任务";
/// 助理搜索占位。
pub const COLLEAGUE_PLACEHOLDER: &str = "搜索助理";
/// 助理搜索空态。
pub const COLLEAGUE_SEARCH_EMPTY: &str = "未找到助理";
/// 忙碌状态标签。
pub const STATUS_BUSY: &str = "工作中";
/// 空闲状态标签。
pub const STATUS_IDLE: &str = "空闲";
/// 未就绪状态标签。
pub const STATUS_UNAVAILABLE: &str = "未就绪";
/// 应用页签名。
pub const APPLICATION_NAME: &str = "应用";
/// 文件页描述。
pub const FILE_DESCRIPTION: &str = "选择文件和文件夹";
/// 文件页初始空态。
pub const FILE_EMPTY_INITIAL: &str = "当前对话中暂无文件";
/// 文件搜索占位。
pub const FILE_PLACEHOLDER: &str = "添加文件作为回答背景";
/// 文件搜索空态。
pub const FILE_SEARCH_EMPTY: &str = "无搜索结果";
/// 文件分组标题。
pub const FILE_SESSION_GROUP: &str = "对话中的文件";

/// 浮层状态（默认 `NotDelivered`：数据未拉取过 ≠ 空集，⛔ 不伪造）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// 已下发且可用。
    Normal,
    Loading,
    Failed,
    /// 数据未下发（无数据源时的诚实默认态）。
    #[default]
    NotDelivered,
}

/// 助理忙闲状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    Busy,
    Idle,
    Unavailable,
}

impl Availability {
    /// 状态标签（逐字取自文案全表）。
    pub fn label(self) -> &'static str {
        match self {
            Availability::Busy => STATUS_BUSY,
            Availability::Idle => STATUS_IDLE,
            Availability::Unavailable => STATUS_UNAVAILABLE,
        }
    }
}

/// @提及弹层的三个页签。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Category {
    #[default]
    Colleague,
    Application,
    File,
}

impl Category {
    /// 全部页签（渲染顺序）。
    pub const ALL: [Category; 3] = [Category::Colleague, Category::Application, Category::File];

    /// 页签名。
    pub fn label(self) -> &'static str {
        match self {
            Category::Colleague => COLLEAGUE_NAME,
            Category::Application => APPLICATION_NAME,
            Category::File => FILE_SESSION_GROUP,
        }
    }

    /// 页签描述（应用页无下发文案，如实为 `None`，⛔ 不自编）。
    pub fn description(self) -> Option<&'static str> {
        match self {
            Category::Colleague => Some(COLLEAGUE_DESCRIPTION),
            Category::Application => None,
            Category::File => Some(FILE_DESCRIPTION),
        }
    }

    /// 搜索占位（应用页无下发文案，如实为 `None`）。
    pub fn placeholder(self) -> Option<&'static str> {
        match self {
            Category::Colleague => Some(COLLEAGUE_PLACEHOLDER),
            Category::Application => None,
            Category::File => Some(FILE_PLACEHOLDER),
        }
    }
}

/// 一条助理条目（运行时由数据源下发；本模块不伪造）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColleagueEntry {
    pub name: String,
    pub availability: Availability,
}

/// 浮层纯数据模型：视图是它的纯函数。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MentionPopover {
    pub state: State,
    pub category: Category,
    pub query: String,
}

impl MentionPopover {
    /// 无数据源时的诚实默认模型（fail-closed）。
    pub fn not_delivered() -> Self {
        Self::default()
    }

    /// 内容区是否应显示「未下发」（一切非 `Normal` 状态都不得渲染条目）。
    pub fn is_not_delivered(&self) -> bool {
        self.state != State::Normal
    }

    /// 按搜索词过滤助理（大小写不敏感；空词匹配全部）。
    pub fn colleague_matches<'a>(&self, entries: &'a [ColleagueEntry]) -> Vec<&'a ColleagueEntry> {
        let needle = self.query.trim().to_lowercase();
        entries
            .iter()
            .filter(|entry| needle.is_empty() || entry.name.to_lowercase().contains(&needle))
            .collect()
    }

    /// 列表为空时要显示的空态文案；`None` = 该页签无下发空态文案。
    pub fn empty_label(&self) -> Option<&'static str> {
        match self.category {
            Category::Colleague => Some(COLLEAGUE_SEARCH_EMPTY),
            Category::Application => None,
            Category::File => {
                if self.query.trim().is_empty() {
                    Some(FILE_EMPTY_INITIAL)
                } else {
                    Some(FILE_SEARCH_EMPTY)
                }
            }
        }
    }
}

// ---------- GPUI 层 ----------

/// 主区渲染（诚实占位：仅标题 + 未下发说明；本卡负责补全真实视图）。
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(div().text_color(theme.foreground).child(TITLE))
        .child(
            div()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}
#[cfg(test)]
mod tests {
    use super::{main_area, State, TITLE};
    use gpui_kit::component::Theme;
    #[test]
    fn default_state_is_not_delivered() {
        assert_eq!(State::default(), State::NotDelivered);
    }
    #[test]
    fn title_not_empty() {
        assert!(!TITLE.is_empty());
    }
    #[test]
    fn main_area_renders_without_panic() {
        let _ = main_area(&Theme::default());
    }
}
