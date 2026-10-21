//! 接线层的**纯逻辑**部分：模式项 / 快选命中项的渲染描述。
//!
//! 这部分不碰 GPUI，因此**无窗口可测**；渲染层（[`super::mode_menu`]、
//! [`super::mention`]）只负责把这些描述画成元素。
#![cfg(feature = "ui")]

use crate::ui::input_area::{InputAreaState, InputMode};

/// 模式下拉里的一项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModeOption {
    /// 该项代表的模式。
    pub mode: InputMode,
    /// 展示文案（取自描述层，不另造）。
    pub label: &'static str,
    /// 是否为当前模式（选中者高亮）。
    pub selected: bool,
    /// 稳定的元素 id（GPUI 交互状态用，同屏不得重复）。
    pub id: String,
}

/// 渲染模式条下拉面板里的选项：5 项，当前模式高亮。
pub(super) fn mode_options(current: InputMode) -> Vec<ModeOption> {
    super::mode_buttons(current)
        .into_iter()
        .map(|button| ModeOption {
            mode: button.mode,
            label: button.label,
            selected: button.selected,
            id: format!("input-mode-option-{}", button.mode.id()),
        })
        .collect()
}

/// `@` 快选面板里的一项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MentionHit {
    /// 候选路径。
    pub path: String,
    /// 稳定的元素 id。
    pub id: String,
}

/// 快选面板最多显示的条数（避免长列表把输入区撑爆）。
pub(super) const MAX_MENTION_HITS: usize = 5;

/// 按草稿里的 `@` 查询词过滤候选路径，最多取前 [`MAX_MENTION_HITS`] 条。
///
/// 没有 `@` 标记、查询词为空、或没有命中时都返回空列表（不弹层）。
pub(super) fn mention_hits(state: &InputAreaState) -> Vec<MentionHit> {
    let Some(query) = state.mention_query() else {
        return Vec::new();
    };
    state
        .filter_candidates(&query)
        .into_iter()
        .take(MAX_MENTION_HITS)
        .map(|path| MentionHit {
            id: format!("input-mention-option-{path}"),
            path,
        })
        .collect()
}
