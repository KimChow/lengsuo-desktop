//! 分支切换浮层（波次 6，卡 `ui-overlay-worktree-branch`，对齐 ui-parity-spec §8 A37）。
//!
//! 分层约定（同 `panel_views`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `worktree`。
//! 数据源在运行时侧：[`BranchSwitchState::initial`] 恒为未下发，等跨线契约就绪后换源。
#![cfg(feature = "ui")]
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::Theme;
use gpui_kit::*;

// ---------- 纯数据层（无 GPUI 依赖） ----------

/// 层标题 / 无障碍标签（worktree.branchSwitch.title）。
pub const TITLE: &str = "切换分支";
/// 数据未下发时的说明（fail-closed；⛔ 不得替换为空列表）。
pub const NOT_DELIVERED: &str = "未下发";
/// 空态：已下发但零分支（worktree.noBranches）。
pub const NO_BRANCHES: &str = "未找到分支";
/// 搜索行占位（worktree.searchBranch）。
pub const SEARCH_PLACEHOLDER: &str = "搜索分支";
/// 本地分组徽标（worktree.localBranches）。
pub const LOCAL_BRANCHES: &str = "本地分支";
/// 远程分组徽标（worktree.remoteBranches）。
pub const REMOTE_BRANCHES: &str = "远程分支";
/// 行内复制动作文案（worktree.branchLabel.copy）。
pub const COPY_BRANCH_NAME: &str = "复制分支名";
/// 渲染上限提示模板（worktree.branchRenderLimitHint，占位符 `{count}`）。
pub const RENDER_LIMIT_HINT: &str = "仅展示前 {count} 个分支，可搜索缩小范围";
/// 确认按钮（worktree.branchSwitch.confirm）。
pub const CONFIRM: &str = "确认";
/// 取消按钮（worktree.branchSwitch.cancel）。
pub const CANCEL: &str = "取消";
/// 切换失败说明（worktree.branchSwitch.failed）。
pub const FAILED: &str = "切换分支失败，请重试。";
/// 脏工作区被拒切换说明（worktree.branchSwitch.dirtyBlocked）。
pub const DIRTY_BLOCKED: &str = "当前工作空间存在未提交改动，无法切换分支，请先处理后重试。";
/// 说明句前缀（worktree.branchSwitch.prefix）。
pub const NOTICE_PREFIX: &str = "当前 ";
/// 说明句中缀（worktree.branchSwitch.currentMiddle）。
pub const NOTICE_CURRENT_MIDDLE: &str = " 空间内容归属 ";
/// 说明句中缀（worktree.branchSwitch.selectedMiddle）。
pub const NOTICE_SELECTED_MIDDLE: &str = " 分支。本次任务已选择 ";
/// 说明句后缀（worktree.branchSwitch.suffix）。
pub const NOTICE_SUFFIX: &str = " 分支。确认后，将切换分支并创建任务。";
/// 脏工作区说明句前缀（worktree.branchSwitch.dirty.prefix）。
pub const DIRTY_PREFIX: &str = "当前 ";
/// 脏工作区说明句中缀（worktree.branchSwitch.dirty.currentMiddle）。
pub const DIRTY_CURRENT_MIDDLE: &str = " 空间内容归属 ";
/// 脏工作区说明句中缀（worktree.branchSwitch.dirty.selectedMiddle）。
pub const DIRTY_SELECTED_MIDDLE: &str =
    " 分支，且存在未提交的变更，请选择处理方式，完成后将自动切换至 ";
/// 脏工作区说明句后缀（worktree.branchSwitch.dirty.suffix）。
pub const DIRTY_SUFFIX: &str = " 分支并创建任务。";
/// 脏工作区「提交」选项前缀（worktree.branchSwitch.dirty.commitPrefix）。
pub const DIRTY_COMMIT_PREFIX: &str = "提交至 ";
/// 脏工作区「提交」选项后缀（worktree.branchSwitch.dirty.commitSuffix）。
pub const DIRTY_COMMIT_SUFFIX: &str = " 分支";
/// 脏工作区「暂存」选项（worktree.branchSwitch.dirty.stash）。
pub const DIRTY_STASH: &str = "暂存，包含未跟踪文件。";
/// 脏工作区「丢弃」选项（worktree.branchSwitch.dirty.discard）。
pub const DIRTY_DISCARD: &str = "丢弃，此操作无法撤销。";
/// 推荐徽标（worktree.branchSwitch.dirty.recommend）。
pub const DIRTY_RECOMMEND: &str = "推荐";
/// 渲染上限：超出部分截断并提示（渲染护栏，非服务端数据）。
pub const RENDER_LIMIT: usize = 100;

/// 单条分支条目（纯数据，Ⓐ 不持凭据、不判权限）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchItem {
    /// 分支名。
    pub name: String,
    /// 是否当前分支。
    pub is_current: bool,
    /// 是否远程分支（驱动「本地分支/远程分支」分组徽标）。
    pub is_remote: bool,
}

impl BranchItem {
    /// 本地分支条目。
    pub fn local(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_current: false,
            is_remote: false,
        }
    }

    /// 远程分支条目。
    pub fn remote(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            is_current: false,
            is_remote: true,
        }
    }

    /// 分组徽标文案（本地分支 / 远程分支）。
    pub fn group_label(&self) -> &'static str {
        if self.is_remote {
            REMOTE_BRANCHES
        } else {
            LOCAL_BRANCHES
        }
    }
}

/// 浮层可枚举状态（fail-closed：唯一诚实初始态是未下发）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum BranchSwitchState {
    /// 数据未下发（⛔ 不许伪装成空集）。
    #[default]
    NotDelivered,
    /// 分支清单已下发（`dirty` 表示目标空间有未提交变更）。
    Delivered {
        items: Vec<BranchItem>,
        /// 本次任务已选择的目标分支名。
        selected: Option<String>,
        dirty: bool,
    },
    /// 当前工作空间有未提交改动，被拒绝切换。
    DirtyBlocked,
    /// 切换失败。
    Failed,
}

/// 把模板里的 `{count}` 占位符替换成数值。
fn fill_count(tmpl: &str, count: usize) -> String {
    tmpl.replace("{count}", &count.to_string())
}

impl BranchSwitchState {
    /// 数据源未接线时的诚实初始态。
    pub fn initial() -> Self {
        Self::NotDelivered
    }

    /// 是否处于未下发态。
    pub fn is_not_delivered(&self) -> bool {
        matches!(self, Self::NotDelivered)
    }

    /// 全量条目数（仅已下发态非零）。
    pub fn total_count(&self) -> usize {
        match self {
            Self::Delivered { items, .. } => items.len(),
            _ => 0,
        }
    }

    /// 实际渲染条目：截断到 [`RENDER_LIMIT`]。
    pub fn visible_items(&self) -> &[BranchItem] {
        match self {
            Self::Delivered { items, .. } => &items[..items.len().min(RENDER_LIMIT)],
            _ => &[],
        }
    }

    /// 是否触发渲染上限提示（超限才提示）。
    pub fn render_limited(&self) -> bool {
        self.total_count() > RENDER_LIMIT
    }

    /// 渲染上限提示文案；未超限 ⇒ None。
    pub fn limit_hint(&self) -> Option<String> {
        self.render_limited()
            .then(|| fill_count(RENDER_LIMIT_HINT, RENDER_LIMIT))
    }

    /// 已下发但零条目 ⇒ 空态（≠ 未下发）。
    pub fn is_empty_delivered(&self) -> bool {
        matches!(self, Self::Delivered { items, .. } if items.is_empty())
    }

    /// 当前分支名（取自清单里的 is_current 条目）。
    pub fn current_branch(&self) -> Option<&str> {
        match self {
            Self::Delivered { items, .. } => items
                .iter()
                .find(|item| item.is_current)
                .map(|item| item.name.as_str()),
            _ => None,
        }
    }

    /// 已选目标分支名。
    pub fn selected_branch(&self) -> Option<&str> {
        match self {
            Self::Delivered { selected, .. } => selected.as_deref(),
            _ => None,
        }
    }

    /// 顶部说明句（clean / dirty 两套逐字模板拼接）；缺当前或已选分支 ⇒ None。
    pub fn notice(&self) -> Option<String> {
        if self.is_empty_delivered() {
            return None;
        }
        let current = self.current_branch()?;
        let selected = self.selected_branch()?;
        let dirty = matches!(self, Self::Delivered { dirty: true, .. });
        Some(if dirty {
            format!(
                "{DIRTY_PREFIX}{current}{DIRTY_CURRENT_MIDDLE}{selected}{DIRTY_SELECTED_MIDDLE}{selected}{DIRTY_SUFFIX}"
            )
        } else {
            format!(
                "{NOTICE_PREFIX}{current}{NOTICE_CURRENT_MIDDLE}{selected}{NOTICE_SELECTED_MIDDLE}{selected}{NOTICE_SUFFIX}"
            )
        })
    }

    /// 脏工作区处理选项（文案，是否推荐）：提交至目标分支（推荐）/ 暂存 / 丢弃。
    pub fn dirty_options(&self) -> Vec<(String, bool)> {
        if let Self::Delivered {
            items,
            selected: Some(selected),
            dirty: true,
        } = self
        {
            if !items.is_empty() {
                return vec![
                    (
                        format!("{DIRTY_COMMIT_PREFIX}{selected}{DIRTY_COMMIT_SUFFIX}"),
                        true,
                    ),
                    (DIRTY_STASH.to_string(), false),
                    (DIRTY_DISCARD.to_string(), false),
                ];
            }
        }
        Vec::new()
    }
}

// ---------- GPUI 层 ----------

/// 浮层渲染（波次 6：浮层不占主区，返回整层容器由调用方叠加）。
#[must_use]
pub fn main_area(theme: &Theme) -> Div {
    div()
        .flex_1()
        .min_w(px(crate::ui::layout::STREAM_MIN_WIDTH))
        .h_full()
        .flex()
        .flex_col()
        .child(
            div()
                .px_4()
                .py_2()
                .text_sm()
                .font_bold()
                .text_color(theme.foreground)
                .child(TITLE),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(NOT_DELIVERED),
        )
}
#[cfg(test)]
mod tests {
    use super::{
        fill_count, BranchItem, BranchSwitchState, CONFIRM, COPY_BRANCH_NAME, NOT_DELIVERED,
        NO_BRANCHES, RENDER_LIMIT, RENDER_LIMIT_HINT, TITLE,
    };
    use gpui_kit::component::Theme;

    fn delivered(
        items: Vec<BranchItem>,
        selected: Option<&str>,
        dirty: bool,
    ) -> BranchSwitchState {
        BranchSwitchState::Delivered {
            items,
            selected: selected.map(str::to_string),
            dirty,
        }
    }

    #[test]
    fn notice_composes_clean_segments_verbatim() {
        let mut current = BranchItem::local("main");
        current.is_current = true;
        let state = delivered(vec![current, BranchItem::local("dev")], Some("dev"), false);
        assert_eq!(
            state.notice(),
            Some("当前 main 空间内容归属 dev 分支。本次任务已选择 dev 分支。确认后，将切换分支并创建任务。".to_string())
        );
    }

    #[test]
    fn notice_composes_dirty_segments_verbatim() {
        let mut current = BranchItem::local("main");
        current.is_current = true;
        let state = delivered(vec![current], Some("feat/x"), true);
        assert_eq!(
            state.notice(),
            Some("当前 main 空间内容归属 feat/x 分支，且存在未提交的变更，请选择处理方式，完成后将自动切换至 feat/x 分支并创建任务。".to_string())
        );
    }

    #[test]
    fn dirty_options_commit_label_is_recommended() {
        let state = delivered(vec![BranchItem::local("main")], Some("feat/x"), true);
        let opts = state.dirty_options();
        assert_eq!(opts.len(), 3);
        assert_eq!(opts[0], ("提交至 feat/x 分支".to_string(), true));
        assert_eq!(opts[1], ("暂存，包含未跟踪文件。".to_string(), false));
        assert_eq!(opts[2], ("丢弃，此操作无法撤销。".to_string(), false));
        // clean 态没有处理选项
        assert!(delivered(vec![BranchItem::local("main")], Some("x"), false)
            .dirty_options()
            .is_empty());
    }

    #[test]
    fn render_limit_caps_visible_items_and_fills_hint() {
        let items: Vec<BranchItem> = (0..RENDER_LIMIT + 5)
            .map(|i| BranchItem::local(format!("b{i}")))
            .collect();
        let state = delivered(items, None, false);
        assert_eq!(state.total_count(), RENDER_LIMIT + 5);
        assert_eq!(state.visible_items().len(), RENDER_LIMIT);
        assert!(state.render_limited());
        assert_eq!(
            state.limit_hint(),
            Some(fill_count(RENDER_LIMIT_HINT, RENDER_LIMIT))
        );
        // 未超限不提示
        let small = delivered(vec![BranchItem::local("main")], None, false);
        assert!(!small.render_limited());
        assert_eq!(small.limit_hint(), None);
    }

    #[test]
    fn empty_delivered_is_empty_state_not_not_delivered() {
        let state = delivered(vec![], None, false);
        assert!(state.is_empty_delivered());
        assert!(!state.is_not_delivered());
        // 空态有专门文案，不是未下发、也不是空列表占位
        assert_ne!(NO_BRANCHES, NOT_DELIVERED);
        assert!(!NO_BRANCHES.is_empty());
        // 空输入不出说明句 / 选项
        assert_eq!(state.notice(), None);
        assert!(state.dirty_options().is_empty());
    }

    #[test]
    fn initial_state_is_fail_closed_not_delivered() {
        assert!(BranchSwitchState::initial().is_not_delivered());
        assert_eq!(BranchSwitchState::default(), BranchSwitchState::NotDelivered);
        assert_eq!(NOT_DELIVERED, "未下发");
    }

    #[test]
    fn current_and_selected_come_from_data_only() {
        let mut current = BranchItem::local("main");
        current.is_current = true;
        let state = delivered(
            vec![current, BranchItem::remote("origin/dev")],
            Some("origin/dev"),
            false,
        );
        assert_eq!(state.current_branch(), Some("main"));
        assert_eq!(state.selected_branch(), Some("origin/dev"));
        // 远程条目分组徽标正确
        assert_eq!(BranchItem::local("a").group_label(), "本地分支");
        assert_eq!(BranchItem::remote("b").group_label(), "远程分支");
        // 未下发态不产出任何分支名（⛔ 不伪造）
        assert_eq!(BranchSwitchState::initial().current_branch(), None);
        assert_eq!(BranchSwitchState::initial().selected_branch(), None);
    }

    #[test]
    fn main_area_builds_without_panic() {
        let theme = Theme::default();
        let _ = super::main_area(&theme);
    }

    #[test]
    fn copy_is_verbatim() {
        assert_eq!(TITLE, "切换分支");
        assert_eq!(COPY_BRANCH_NAME, "复制分支名");
        assert_eq!(CONFIRM, "确认");
        assert_eq!(RENDER_LIMIT_HINT, "仅展示前 {count} 个分支，可搜索缩小范围");
    }
}
