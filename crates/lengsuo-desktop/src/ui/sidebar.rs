//! 左边栏：会话列表（置顶 / 归档 / 未读）。
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
    pub id: String,
    pub title: String,
    pub pinned: bool,
    pub archived: bool,
    pub unread: bool,
}

impl SessionEntry {
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

    #[must_use]
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    #[must_use]
    pub fn with_archived(mut self, archived: bool) -> Self {
        self.archived = archived;
        self
    }

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
}

impl SidebarState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

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
}

/// 渲染后的一行会话。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub id: String,
    pub title: String,
    /// 标题是否为占位文案（原始标题缺失 / 空白）。
    pub title_is_placeholder: bool,
    pub pinned: bool,
    pub archived: bool,
    pub unread: bool,
    pub selected: bool,
}

/// 边栏中的一个分组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarSection {
    pub label: &'static str,
    pub rows: Vec<SessionRow>,
}

/// 边栏渲染描述。
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarView {
    /// 边栏实际宽度（逻辑像素）。
    pub width: f32,
    pub heading: &'static str,
    pub sections: Vec<SidebarSection>,
    pub selected: Option<String>,
    /// 未读角标数量（不含归档）。
    pub unread_count: usize,
    /// 归档折叠时被隐藏的会话数。
    pub hidden_archived: usize,
    /// 归档展开 / 收起按钮文案。
    pub archived_toggle_label: &'static str,
    /// 没有任何可见会话时的占位文案。
    pub empty_text: Option<&'static str>,
}

/// 生成边栏渲染描述。
#[must_use]
pub fn render(state: &SidebarState, width: f32) -> SidebarView {
    let mut sections = Vec::new();

    let pinned: Vec<SessionRow> = state
        .visible_entries()
        .into_iter()
        .filter(|entry| entry.pinned)
        .map(|entry| row(entry, state.selected()))
        .collect();
    if !pinned.is_empty() {
        sections.push(SidebarSection {
            label: PINNED_SECTION,
            rows: pinned,
        });
    }

    let regular: Vec<SessionRow> = state
        .visible_entries()
        .into_iter()
        .filter(|entry| !entry.pinned)
        .map(|entry| row(entry, state.selected()))
        .collect();
    if !regular.is_empty() {
        sections.push(SidebarSection {
            label: SESSIONS_SECTION,
            rows: regular,
        });
    }

    if state.show_archived() {
        let archived: Vec<SessionRow> = state
            .entries()
            .iter()
            .filter(|entry| entry.archived)
            .map(|entry| row(entry, state.selected()))
            .collect();
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
    }
}

fn row(entry: &SessionEntry, selected: Option<&str>) -> SessionRow {
    let title = entry.title.trim();
    SessionRow {
        id: entry.id.clone(),
        title: if title.is_empty() {
            UNTITLED_SESSION.to_owned()
        } else {
            title.to_owned()
        },
        title_is_placeholder: title.is_empty(),
        pinned: entry.pinned,
        archived: entry.archived,
        unread: entry.unread,
        selected: selected == Some(entry.id.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> SidebarState {
        let mut state = SidebarState::new();
        state.set_entries(vec![
            SessionEntry::new("a", "普通一").with_unread(true),
            SessionEntry::new("b", "置顶一").with_pinned(true),
            SessionEntry::new("c", "归档一").with_archived(true),
            SessionEntry::new("d", "普通二"),
        ]);
        state
    }

    #[test]
    fn entries_keep_injection_order_and_selection_starts_empty() {
        let state = sample();
        assert_eq!(state.entries().len(), 4);
        assert_eq!(state.selected(), None);
        assert_eq!(state.unread_count(), 1);
        assert_eq!(state.archived_count(), 1);
    }

    #[test]
    fn pinned_sessions_are_sorted_first() {
        let state = sample();
        let visible: Vec<&str> = state
            .visible_entries()
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(visible, ["b", "a", "d"], "置顶在前，归档隐藏，其余保持顺序");
    }

    #[test]
    fn archived_are_hidden_by_default_and_toggleable() {
        let mut state = sample();
        let view = render(&state, 260.0);
        assert_eq!(view.hidden_archived, 1);
        assert_eq!(view.archived_toggle_label, ARCHIVED_TOGGLE_SHOW);
        assert!(view
            .sections
            .iter()
            .all(|section| section.label != ARCHIVED_SECTION));

        state.toggle_archived();
        let view = render(&state, 260.0);
        assert_eq!(view.hidden_archived, 0);
        assert_eq!(view.archived_toggle_label, ARCHIVED_TOGGLE_HIDE);
        let archived = view
            .sections
            .iter()
            .find(|section| section.label == ARCHIVED_SECTION)
            .expect("展开后应有归档分组");
        assert_eq!(archived.rows.len(), 1);
        assert_eq!(archived.rows[0].title, "归档一");
    }

    #[test]
    fn sections_show_pinned_then_regular() {
        let view = render(&sample(), 260.0);
        let labels: Vec<&str> = view.sections.iter().map(|s| s.label).collect();
        assert_eq!(labels, [PINNED_SECTION, SESSIONS_SECTION]);
        assert_eq!(view.sections[0].rows.len(), 1);
        assert_eq!(view.sections[1].rows.len(), 2);
        assert_eq!(view.width, 260.0);
        assert_eq!(view.heading, HEADING);
        assert_eq!(view.unread_count, 1);
        assert_eq!(view.empty_text, None);
    }

    #[test]
    fn select_requires_a_visible_entry() {
        let mut state = sample();
        assert!(state.select("a"));
        assert_eq!(state.selected(), Some("a"));
        assert!(!state.select("missing"));
        assert_eq!(state.selected(), Some("a"), "未知 id 不得改选中");
        assert!(!state.select("c"), "归档隐藏时不可选中");
        assert_eq!(state.selected(), Some("a"));
    }

    #[test]
    fn hiding_archived_clears_archived_selection() {
        let mut state = sample();
        state.set_show_archived(true);
        assert!(state.select("c"));
        assert_eq!(state.selected(), Some("c"));
        state.set_show_archived(false);
        assert_eq!(state.selected(), None, "归档隐藏后选中项不得悬空");
    }

    #[test]
    fn replacing_entries_clears_stale_selection() {
        let mut state = sample();
        assert!(state.select("a"));
        state.set_entries(vec![SessionEntry::new("x", "新会话")]);
        assert_eq!(state.selected(), None);
        assert_eq!(state.entries().len(), 1);
    }

    #[test]
    fn empty_state_and_missing_titles_are_explicit() {
        let view = render(&SidebarState::new(), 260.0);
        assert!(view.sections.is_empty());
        assert_eq!(view.empty_text, Some(EMPTY_TEXT));

        let mut only_archived = SidebarState::new();
        only_archived.set_entries(vec![
            SessionEntry::new("archived", "旧会话").with_archived(true)
        ]);
        let view = render(&only_archived, 260.0);
        assert_eq!(view.empty_text, Some(EMPTY_ACTIVE_TEXT));
        assert_eq!(view.hidden_archived, 1);

        let mut state = SidebarState::new();
        state.set_entries(vec![
            SessionEntry::new("blank", "   "),
            SessionEntry::new("archived", "旧会话").with_archived(true),
        ]);
        let view = render(&state, 260.0);
        assert_eq!(view.empty_text, None, "有进行中会话时不显示空态");
        let row = &view.sections[0].rows[0];
        assert_eq!(row.title, UNTITLED_SESSION);
        assert!(row.title_is_placeholder);
        assert!(!row.title.contains("blank"), "不得拿 id 冒充标题");
    }

    #[test]
    fn selected_row_is_marked() {
        let mut state = sample();
        assert!(state.select("b"));
        let view = render(&state, 260.0);
        let selected: Vec<&SessionRow> = view
            .sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .filter(|row| row.selected)
            .collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "b");
        assert_eq!(view.selected.as_deref(), Some("b"));
    }

    #[test]
    fn unread_count_ignores_archived_sessions() {
        let mut state = SidebarState::new();
        state.set_entries(vec![
            SessionEntry::new("a", "进行中").with_unread(true),
            SessionEntry::new("b", "已归档")
                .with_archived(true)
                .with_unread(true),
        ]);
        assert_eq!(state.unread_count(), 1);
    }
}
