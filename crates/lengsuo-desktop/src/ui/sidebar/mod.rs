//! 左边栏：会话列表（置顶 / 归档 / 未读）＋ 频道入口 ＋ 智能体管理入口。
//!
//! 本模块维护边栏的**视图状态**（会话条目、选中项、归档是否可见）与渲染
//! 描述，不接真实数据源、不发起请求。条目由调用方注入，与
//! [`crate::ui::layout::SidebarLayout`] 的宽度状态相互独立：
//! - 宽度属于版式，见 `layout`；
//! - 列表数据与选中属于本模块。
//!
//! 展示约定：
//! - 置顶会话单独成组排在最前，其余保持注入顺序；
//! - 归档会话默认隐藏，只显示数量；显式展开后才出现在「归档」组；
//! - 未读以 `unread` 标记交给渲染层画圆点，本层不推断已读。
//!
//! **边栏现在有五块**（对齐 A15 + A17 + A18）：会话列表、频道入口、智能体管理入口、
//! 浏览器环境管理入口、组织与权限可见性入口。后四块只做**界面**（入口、计数、选中态），
//! ⛔ 不实现频道收发、不实现智能体创建、不实现多代理角色模型（那是运行时与业务语义）。
//!
//! 目录式子模块（Rust 2018 起 `ui/mod.rs` 里的 `pub mod sidebar;` 自动指向本目录）：
//! - [`sections`]：会话分组的摊平（置顶 / 全部会话 / 归档）。
//! - [`nav`]：频道入口 + 智能体管理入口 + 环境入口 + 组织入口的描述（纯数据）。
//! - `tests`：描述层契约测试。

pub mod nav;
mod sections;

#[cfg(test)]
mod tests;

pub use nav::{organization_entry, NavEntry, NavKind, ENVIRONMENTS_TITLE, ORGANIZATION_TITLE};
pub use sections::{SessionRow, SidebarSection};

/// 边栏标题。
pub const HEADING: &str = "会话";
/// 置顶分组的标题。
pub const PINNED_SECTION: &str = "置顶";
/// 普通会话分组的标题。
pub const SESSIONS_SECTION: &str = "全部会话";
/// 归档分组的标题。
pub const ARCHIVED_SECTION: &str = "归档";
/// 一条会话都没有时的占位文案。
pub const EMPTY_TEXT: &str = "还没有会话";
/// 可见会话为零、但存在归档会话时的占位文案。
pub const EMPTY_ACTIVE_TEXT: &str = "没有进行中的会话";
/// 标题缺失时的占位文案（不拿 id 冒充标题）。
pub const UNTITLED_SESSION: &str = "未命名会话";
/// 归档折叠时「显示归档」操作文案。
pub const ARCHIVED_TOGGLE_SHOW: &str = "显示归档";
/// 归档展开时「隐藏归档」操作文案。
pub const ARCHIVED_TOGGLE_HIDE: &str = "隐藏归档";

/// 一条会话列表项（界面数据，不是协议模型）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    /// 会话 id。
    pub id: String,
    /// 会话标题。
    pub title: String,
    /// 是否置顶。
    pub pinned: bool,
    /// 是否已归档。
    pub archived: bool,
    /// 是否有未读。
    pub unread: bool,
}

impl SessionEntry {
    /// 构造一条会话（默认不置顶 / 不归档 / 无未读）。
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            pinned: false,
            archived: false,
            unread: false,
        }
    }

    /// 设置置顶状态。
    #[must_use]
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    /// 设置归档状态。
    #[must_use]
    pub fn with_archived(mut self, archived: bool) -> Self {
        self.archived = archived;
        self
    }

    /// 设置未读状态。
    #[must_use]
    pub fn with_unread(mut self, unread: bool) -> Self {
        self.unread = unread;
        self
    }
}

/// 边栏视图状态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SidebarState {
    entries: Vec<SessionEntry>,
    selected: Option<String>,
    show_archived: bool,
    /// 频道数量（用于频道入口角标）。
    channel_count: usize,
    /// 智能体数量（用于智能体管理入口角标）。
    agent_count: usize,
    /// 当前选中的导航入口（`None` = 选的是会话）。
    selected_nav: Option<NavKind>,
}

impl SidebarState {
    /// 空的边栏状态。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 全部会话条目。
    #[must_use]
    pub fn entries(&self) -> &[SessionEntry] {
        &self.entries
    }

    /// 注入会话条目。选中项若已不在列表中则清空，避免引用悬空 id。
    pub fn set_entries(&mut self, entries: Vec<SessionEntry>) {
        self.entries = entries;
        if let Some(selected) = &self.selected {
            if !self.entries.iter().any(|entry| &entry.id == selected) {
                self.selected = None;
            }
        }
    }

    /// 当前选中的会话 id。
    #[must_use]
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    /// 选中一条**当前可见**的会话；不存在或已归档隐藏时返回 `false`
    /// 且不改变选中态。
    pub fn select(&mut self, id: &str) -> bool {
        let selectable = self
            .entries
            .iter()
            .any(|entry| entry.id == id && (!entry.archived || self.show_archived));
        if selectable {
            self.selected = Some(id.to_owned());
        }
        selectable
    }

    /// 归档是否可见。
    #[must_use]
    pub fn show_archived(&self) -> bool {
        self.show_archived
    }

    /// 设置归档可见性。隐藏归档时若选中项是归档会话，一并清空选中。
    pub fn set_show_archived(&mut self, show: bool) {
        self.show_archived = show;
        if !show {
            let selected_is_archived = self.selected.as_deref().is_some_and(|id| {
                self.entries
                    .iter()
                    .any(|entry| entry.id == id && entry.archived)
            });
            if selected_is_archived {
                self.selected = None;
            }
        }
    }

    /// 翻转归档可见性。
    pub fn toggle_archived(&mut self) {
        self.set_show_archived(!self.show_archived);
    }

    /// 未读会话数（不含归档会话）。
    #[must_use]
    pub fn unread_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.unread && !entry.archived)
            .count()
    }

    /// 归档会话总数。
    #[must_use]
    pub fn archived_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.archived).count()
    }

    /// 当前可见的进行中会话：置顶在前，其余保持注入顺序。
    #[must_use]
    pub fn visible_entries(&self) -> Vec<&SessionEntry> {
        let mut pinned = Vec::new();
        let mut regular = Vec::new();
        for entry in &self.entries {
            if entry.archived {
                continue;
            }
            if entry.pinned {
                pinned.push(entry);
            } else {
                regular.push(entry);
            }
        }
        pinned.extend(regular);
        pinned
    }

    /// 频道数量（频道入口角标）。
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channel_count
    }

    /// 智能体数量（智能体管理入口角标）。
    #[must_use]
    pub fn agent_count(&self) -> usize {
        self.agent_count
    }

    /// 环境数量（环境入口角标）。
    ///
    /// ⚠️ **恒为 0**：环境条目来自运行时侧，界面层拿不到真实列表
    /// ⇒ 角标为 0 ⇒ **不显示角标**（与「没有环境」的观感一致），**不伪造数据**。
    #[must_use]
    pub fn environment_count(&self) -> usize {
        0
    }

    /// 组织成员数量（组织入口角标）。
    ///
    /// ⚠️ **恒为 0**：组织条目来自运行时侧，界面层拿不到真实清单
    /// ⇒ 角标为 0 ⇒ **不显示角标**（与「没有成员」的观感一致），**不伪造数据**。
    #[must_use]
    pub fn organization_count(&self) -> usize {
        0
    }

    /// 设置频道数量。
    pub fn set_channel_count(&mut self, count: usize) {
        self.channel_count = count;
    }

    /// 设置智能体数量。
    pub fn set_agent_count(&mut self, count: usize) {
        self.agent_count = count;
    }

    /// 当前选中的导航入口（`None` = 选的是会话）。
    #[must_use]
    pub fn selected_nav(&self) -> Option<NavKind> {
        self.selected_nav
    }

    /// 选中一个导航入口；返回是否发生了变化。
    pub fn select_nav(&mut self, kind: NavKind) -> bool {
        if self.selected_nav == Some(kind) {
            return false;
        }
        self.selected_nav = Some(kind);
        true
    }
}

/// 边栏渲染描述。
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarView {
    /// 边栏实际宽度（逻辑像素）。
    pub width: f32,
    /// 边栏标题。
    pub heading: &'static str,
    /// 会话分组（置顶 / 全部会话 / 归档）。
    pub sections: Vec<SidebarSection>,
    /// 当前选中的会话 id。
    pub selected: Option<String>,
    /// 未读角标数量（不含归档）。
    pub unread_count: usize,
    /// 归档折叠时被隐藏的会话数。
    pub hidden_archived: usize,
    /// 归档展开 / 收起按钮文案。
    pub archived_toggle_label: &'static str,
    /// 没有任何可见会话时的占位文案。
    pub empty_text: Option<&'static str>,
    /// 三个导航入口（频道 + 智能体管理 + 环境）。
    pub nav: Vec<NavEntry>,
}

/// 生成边栏渲染描述。
#[must_use]
pub fn render(state: &SidebarState, width: f32) -> SidebarView {
    let mut sections = Vec::new();

    let pinned = sections::pinned_rows(state);
    if !pinned.is_empty() {
        sections.push(SidebarSection {
            label: PINNED_SECTION,
            rows: pinned,
        });
    }

    let regular = sections::regular_rows(state);
    if !regular.is_empty() {
        sections.push(SidebarSection {
            label: SESSIONS_SECTION,
            rows: regular,
        });
    }

    if state.show_archived() {
        let archived = sections::archived_rows(state);
        if !archived.is_empty() {
            sections.push(SidebarSection {
                label: ARCHIVED_SECTION,
                rows: archived,
            });
        }
    }

    let visible_count: usize = sections.iter().map(|section| section.rows.len()).sum();
    let hidden_archived = if state.show_archived() {
        0
    } else {
        state.archived_count()
    };
    let empty_text = (visible_count == 0).then_some(if state.entries().is_empty() {
        EMPTY_TEXT
    } else {
        EMPTY_ACTIVE_TEXT
    });

    SidebarView {
        width,
        heading: HEADING,
        sections,
        selected: state.selected().map(str::to_owned),
        unread_count: state.unread_count(),
        hidden_archived,
        archived_toggle_label: if state.show_archived() {
            ARCHIVED_TOGGLE_HIDE
        } else {
            ARCHIVED_TOGGLE_SHOW
        },
        empty_text,
        nav: nav::nav_entries(
            state.channel_count(),
            state.agent_count(),
            state.environment_count(),
            state.organization_count(),
            state.selected_nav(),
        ),
    }
}
