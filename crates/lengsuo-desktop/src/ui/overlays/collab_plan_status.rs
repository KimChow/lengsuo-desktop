//! 协作计划·状态（波次 9，卡 `ui-overlay-collab-plan-status`）。
//!
//! 分层约定（同 `collab_plan_detail`）：纯数据层 + GPUI 层（[`main_area`]）。
//! 硬约束：fail-closed；⛔ 不伪造服务端数据；文案逐字取自命名空间 `collab.plan.status`。
//! 全表勘明 27 有效键（docs/ui-parity-spec.md），本表已逐字落 12 键；
//! 其余 15 键的全表源不在本 worktree，缺词经 [`text`] 返回 `None` 显形，⛔ 禁止自编。
#![cfg(feature = "ui")]
use gpui_kit::component::Theme;
use gpui_kit::*;

/// 面板标题（逐字取自文案全表，承接占位 stub 契约）。
pub const PANEL_TITLE: &str = "协作计划·状态";

/// 命名空间前缀。
pub const NS_PREFIX: &str = "collab.plan.status";

/// 命名空间 `collab.plan.status` 文案表（key、中文均逐字取自界面文案全表）。
/// 每行两条压行数（≤700 行硬限），键序与全表一致（字母序）。
#[rustfmt::skip]
pub const TEXTS: &[(&str, &str)] = &[
    ("collab.plan.status.desc.completed", "已完成，可回顾产出与变更记录"), ("collab.plan.status.desc.fallback", "按状态维护事项并持续推进项目协作。"),
    ("collab.plan.status.desc.in_progress", "正在推进中，关注进度与截止时间"), ("collab.plan.status.desc.paused", "当前存在阻塞，等待恢复处理"),
    ("collab.plan.status.desc.pending", "已创建，等待成员认领并开始执行"), ("collab.plan.status.label.completed", "完成"),
    ("collab.plan.status.label.in_progress", "进行中"), ("collab.plan.status.label.paused", "暂停"),
    ("collab.plan.status.label.pending", "待开始"), ("collab.plan.status.shortLabel.completed", "已完成"),
    ("collab.plan.status.shortLabel.paused", "已暂停"), ("collab.plan.status.shortLabel.pending", "未启动"),
];

/// 按全量 key 取文案；未知 key 返回 `None`（fail-closed，不猜值）。
pub fn text(key: &str) -> Option<&'static str> {
    TEXTS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// 渲染层取词（key 应在 [`TEXTS`]；缺词落空串显形，全表补齐后一键点亮）。
fn t(key: &str) -> &'static str {
    text(key).unwrap_or_default()
}

/// 事项推进状态（文案取 `label.*` / `shortLabel.*` / `desc.*`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStatus {
    Pending,
    InProgress,
    Paused,
    Completed,
}

impl PlanStatus {
    /// 按生命周期排序的全部状态。
    pub const ALL: [PlanStatus; 4] = [
        PlanStatus::Pending,
        PlanStatus::InProgress,
        PlanStatus::Paused,
        PlanStatus::Completed,
    ];

    /// 由运行时状态串解析；未知值 `None`（fail-closed，不猜状态）。
    pub fn from_slug(slug: &str) -> Option<PlanStatus> {
        match slug {
            "pending" => Some(PlanStatus::Pending),
            "in_progress" => Some(PlanStatus::InProgress),
            "paused" => Some(PlanStatus::Paused),
            "completed" => Some(PlanStatus::Completed),
            _ => None,
        }
    }

    fn slug(self) -> &'static str {
        match self {
            PlanStatus::Pending => "pending",
            PlanStatus::InProgress => "in_progress",
            PlanStatus::Paused => "paused",
            PlanStatus::Completed => "completed",
        }
    }

    /// 键（`collab.plan.status.<kind>.<slug>`）。
    fn key(self, kind: &str) -> String {
        format!("{NS_PREFIX}.{kind}.{}", self.slug())
    }

    /// 徽标文案（`label.*`）。
    pub fn label(self) -> &'static str {
        t(&self.key("label"))
    }

    /// 短文案（`shortLabel.*`；`in_progress` 在全表缺口内，暂空串显形）。
    pub fn short_label(self) -> &'static str {
        t(&self.key("shortLabel"))
    }

    /// 说明文案（`desc.*`）。
    pub fn desc(self) -> &'static str {
        t(&self.key("desc"))
    }
}

/// 一条计划事项（数据源在运行时侧；⛔ 不伪造）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollabPlanStatusItem {
    pub title: String,
    pub status: PlanStatus,
}

/// 视图态状态机：默认 `Loading`（未拉取过 ≠ 空集）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum CollabPlanStatusState {
    /// 正常：数据已下发且有事项。
    Normal(Vec<CollabPlanStatusItem>),
    /// 空态：已下发但无事项（⛔ 不是空列表）。
    Empty,
    /// 加载中（未拉取过的诚实默认态）。
    #[default]
    Loading,
    /// 失败：fail-closed，不回落旧数据（渲染必须可见，不退化成空列表）。
    Failed,
}

impl CollabPlanStatusState {
    /// 各分支提示行（逐字取自全表；`Normal` 无提示行）。
    /// loading/failed 专用键在全表缺口内，非正常分支统一落 `desc.fallback`（唯一合法兜底文案）。
    pub fn status_text(&self) -> &'static str {
        match self {
            CollabPlanStatusState::Normal(_) => "",
            _ => t("collab.plan.status.desc.fallback"),
        }
    }
}

/// 由运行时输入解析视图态（纯函数）：`None` = 未拉取过（Loading）≠ `Some(vec![])` = 空集。
pub fn resolve(delivered: Option<Vec<CollabPlanStatusItem>>) -> CollabPlanStatusState {
    match delivered {
        None => CollabPlanStatusState::Loading,
        Some(items) if items.is_empty() => CollabPlanStatusState::Empty,
        Some(items) => CollabPlanStatusState::Normal(items),
    }
}

// ===== GPUI 层 =====
/// 主区/浮层呈现入口（签名同 `collab_plan_detail::main_area`；无数据源 ⇒ 默认 `Loading`）。
pub fn main_area(theme: &Theme) -> AnyElement {
    render(theme, &CollabPlanStatusState::default())
}

/// 按状态渲染整棵树（只构造元素，不触实体/上下文；测试逐分支调用防 panic）。
pub fn render(theme: &Theme, state: &CollabPlanStatusState) -> AnyElement {
    let frame = div()
        .flex()
        .flex_col()
        .gap_2()
        .text_sm()
        .text_color(theme.muted_foreground)
        .child(header_row(theme));
    match state {
        CollabPlanStatusState::Normal(items) => frame
            .children(items.iter().map(|it| item_card(theme, it)))
            .into_any_element(),
        CollabPlanStatusState::Empty | CollabPlanStatusState::Loading => frame
            .child(hint_row(theme, state.status_text()))
            .into_any_element(),
        CollabPlanStatusState::Failed => frame
            // 失败必须可见：foreground 全对比 + 描边行，不退化成空列表
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_1()
                    .border_color(theme.border)
                    .text_color(theme.foreground)
                    .child(SharedString::from(state.status_text())),
            )
            .into_any_element(),
    }
}

/// 标题行：面板标题 + 全部状态徽标图例（`label.*` 逐字）。
fn header_row(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(
            div()
                .text_color(theme.foreground)
                .child(SharedString::from(PANEL_TITLE)),
        )
        .children(PlanStatus::ALL.iter().map(|s| badge(theme, s.label())))
        .into_any_element()
}

/// 状态徽标（描边小标签，文案逐字来自 `label.*`）。
fn badge(theme: &Theme, label: &str) -> AnyElement {
    div()
        .px_2()
        .border_1()
        .border_color(theme.border)
        .text_xs()
        .child(SharedString::from(label))
        .into_any_element()
}

/// 非正常分支提示行（muted；文案见 [`CollabPlanStatusState::status_text`]）。
fn hint_row(theme: &Theme, hint: &str) -> AnyElement {
    div()
        .px_4()
        .py_2()
        .text_color(theme.muted_foreground)
        .child(SharedString::from(hint))
        .into_any_element()
}

/// 单条事项卡：标题 + 徽标（`label.*`）+ 说明（`desc.*`），全部逐字。
fn item_card(theme: &Theme, it: &CollabPlanStatusItem) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .py_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_color(theme.foreground)
                        .child(SharedString::from(it.title.as_str())),
                )
                .child(badge(theme, it.status.label())),
        )
        .child(SharedString::from(it.status.desc()))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // ⛔ 不用 `use super::*`：父模块的 `use gpui_kit::*` 会把 GPUI 的 `test`
    // 属性宏带进来遮蔽 `#[test]`。显式列符号（同 `collab_plan_detail`）。
    use super::{
        main_area, render, resolve, text, CollabPlanStatusItem, CollabPlanStatusState, PlanStatus,
        PANEL_TITLE, TEXTS,
    };
    use gpui_kit::component::Theme;

    #[test]
    fn copy_is_verbatim() {
        // 任务书逐字摘录 12 条，键值一字不差
        assert_eq!(
            text("collab.plan.status.desc.completed"),
            Some("已完成，可回顾产出与变更记录")
        );
        assert_eq!(
            text("collab.plan.status.desc.fallback"),
            Some("按状态维护事项并持续推进项目协作。")
        );
        assert_eq!(
            text("collab.plan.status.desc.in_progress"),
            Some("正在推进中，关注进度与截止时间")
        );
        assert_eq!(
            text("collab.plan.status.desc.paused"),
            Some("当前存在阻塞，等待恢复处理")
        );
        assert_eq!(
            text("collab.plan.status.desc.pending"),
            Some("已创建，等待成员认领并开始执行")
        );
        assert_eq!(text("collab.plan.status.label.completed"), Some("完成"));
        assert_eq!(text("collab.plan.status.label.in_progress"), Some("进行中"));
        assert_eq!(text("collab.plan.status.label.paused"), Some("暂停"));
        assert_eq!(text("collab.plan.status.label.pending"), Some("待开始"));
        assert_eq!(
            text("collab.plan.status.shortLabel.completed"),
            Some("已完成")
        );
        assert_eq!(text("collab.plan.status.shortLabel.paused"), Some("已暂停"));
        assert_eq!(
            text("collab.plan.status.shortLabel.pending"),
            Some("未启动")
        );
        assert_eq!(PANEL_TITLE, "协作计划·状态");
    }

    #[test]
    fn ns_table_locked() {
        // 已落 12/27 键：其余 15 键全表源不在本 worktree，缺词空串显形（见模块文档）
        assert_eq!(TEXTS.len(), 12);
        let mut seen: Vec<&str> = TEXTS.iter().map(|(k, _)| *k).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), TEXTS.len());
        assert!(seen.iter().all(|k| k.starts_with("collab.plan.status.")));
    }

    #[test]
    fn fail_closed_lookup_and_slug() {
        assert_eq!(text("collab.plan.status.nope"), None);
        assert_eq!(PlanStatus::from_slug("canceled"), None);
        assert_eq!(PlanStatus::from_slug("pending"), Some(PlanStatus::Pending));
        assert_eq!(
            PlanStatus::from_slug("in_progress"),
            Some(PlanStatus::InProgress)
        );
        assert_eq!(PlanStatus::from_slug("paused"), Some(PlanStatus::Paused));
        assert_eq!(
            PlanStatus::from_slug("completed"),
            Some(PlanStatus::Completed)
        );
    }

    #[test]
    fn status_copy_complete_except_known_gap() {
        assert_eq!(PlanStatus::Pending.label(), "待开始");
        assert_eq!(PlanStatus::InProgress.label(), "进行中");
        assert_eq!(PlanStatus::Paused.label(), "暂停");
        assert_eq!(PlanStatus::Completed.label(), "完成");
        assert_eq!(PlanStatus::Pending.desc(), "已创建，等待成员认领并开始执行");
        assert_eq!(
            PlanStatus::InProgress.desc(),
            "正在推进中，关注进度与截止时间"
        );
        assert_eq!(PlanStatus::Paused.desc(), "当前存在阻塞，等待恢复处理");
        assert_eq!(PlanStatus::Completed.desc(), "已完成，可回顾产出与变更记录");
        assert_eq!(PlanStatus::Pending.short_label(), "未启动");
        assert_eq!(PlanStatus::Paused.short_label(), "已暂停");
        assert_eq!(PlanStatus::Completed.short_label(), "已完成");
        // 全表缺口：shortLabel.in_progress 缺词 ⇒ 空串显形，补表后此断言应改为逐字值
        assert_eq!(PlanStatus::InProgress.short_label(), "");
    }

    #[test]
    fn state_machine_contract() {
        // 未拉取过 ≠ 空集：默认态 Loading，None 不落 Empty
        assert_eq!(
            CollabPlanStatusState::default(),
            CollabPlanStatusState::Loading
        );
        assert_eq!(resolve(None), CollabPlanStatusState::Loading);
        assert_eq!(resolve(Some(Vec::new())), CollabPlanStatusState::Empty);
        let item = CollabPlanStatusItem {
            title: "联调事项".into(),
            status: PlanStatus::InProgress,
        };
        assert_eq!(
            resolve(Some(vec![item.clone()])),
            CollabPlanStatusState::Normal(vec![item])
        );
        // 非正常分支提示统一落全表 fallback 文案；Normal 无提示行
        let fb = "按状态维护事项并持续推进项目协作。";
        assert_eq!(CollabPlanStatusState::Loading.status_text(), fb);
        assert_eq!(CollabPlanStatusState::Failed.status_text(), fb);
        assert_eq!(CollabPlanStatusState::Empty.status_text(), fb);
        assert_eq!(CollabPlanStatusState::Normal(vec![]).status_text(), "");
    }

    #[test]
    fn render_branches_construct() {
        let theme = Theme::default();
        let _ = main_area(&theme); // 固定签名入口只构造元素树，不 panic
        let _ = render(&theme, &CollabPlanStatusState::Loading);
        let _ = render(&theme, &CollabPlanStatusState::Failed);
        let _ = render(&theme, &CollabPlanStatusState::Empty);
        let items = vec![
            CollabPlanStatusItem {
                title: "甲".into(),
                status: PlanStatus::Pending,
            },
            CollabPlanStatusItem {
                title: "乙".into(),
                status: PlanStatus::Completed,
            },
        ];
        let _ = render(&theme, &CollabPlanStatusState::Normal(items));
    }
}
