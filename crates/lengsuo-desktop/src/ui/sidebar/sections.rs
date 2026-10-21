//! 边栏的会话分组：置顶 / 全部会话 / 归档。
//!
//! 描述层只负责把 [`SidebarState`] 里的会话条目**按分组**摊平成可渲染的行，
//! 不做过滤之外的判断：哪些分组可见由 [`super::SidebarState`] 决定。

use super::{SessionEntry, SidebarState, UNTITLED_SESSION};

/// 渲染后的一行会话。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    /// 会话 id（用于生成界面 id 与点击回写）。
    pub id: String,
    /// 展示标题。
    pub title: String,
    /// 标题是否为占位文案（原始标题缺失 / 空白）。
    pub title_is_placeholder: bool,
    /// 是否置顶。
    pub pinned: bool,
    /// 是否已归档。
    pub archived: bool,
    /// 是否有未读。
    pub unread: bool,
    /// 是否被选中。
    pub selected: bool,
}

/// 边栏中的一个分组。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarSection {
    /// 分组标题（中文，取自描述层常量）。
    pub label: &'static str,
    /// 组内的会话行。
    pub rows: Vec<SessionRow>,
}

/// 置顶分组的行（其余会话被排除）。
pub(super) fn pinned_rows(state: &SidebarState) -> Vec<SessionRow> {
    state
        .visible_entries()
        .into_iter()
        .filter(|entry| entry.pinned)
        .map(|entry| row(entry, state.selected()))
        .collect()
}

/// 「全部会话」分组的行（置顶会话被排除）。
pub(super) fn regular_rows(state: &SidebarState) -> Vec<SessionRow> {
    state
        .visible_entries()
        .into_iter()
        .filter(|entry| !entry.pinned)
        .map(|entry| row(entry, state.selected()))
        .collect()
}

/// 归档分组的行（只在归档可见时调用）。
pub(super) fn archived_rows(state: &SidebarState) -> Vec<SessionRow> {
    state
        .entries()
        .iter()
        .filter(|entry| entry.archived)
        .map(|entry| row(entry, state.selected()))
        .collect()
}

/// 一条会话 → 一行渲染描述。
pub(super) fn row(entry: &SessionEntry, selected: Option<&str>) -> SessionRow {
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
