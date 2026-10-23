//! 任务清单 tab 的视图体：任务行（状态点 + 标题 + 进度条）与空态。
//!
//! - 纯函数层（[`TaskItem`] / [`demo_tasks`] / [`progress_clamped`] /
//!   [`task_label`] / [`empty_reason`]）无 GPUI 依赖，可无窗口测；
//! - 接线层（[`render`] / [`render_demo`]）把纯数据变成元素树，颜色一律取主题 token。
//!
//! 任务状态来源在运行时（契约未就绪）⇒ 本卡只做视图：演示数据 + 如实标注
//! （[`DEMO_NOTE_TEXT`]），⛔ 不伪造真实进度。

use gpui_kit::*;

/// 视图体顶部的如实标注：当前数据是演示的，运行时契约未就绪。
pub const DEMO_NOTE_TEXT: &str = "演示数据（运行时契约未就绪）";
/// 任务列表为空时的空态文案。
pub const EMPTY_TEXT: &str = "暂无任务";

/// 一条任务（演示数据；运行时契约就绪后由状态层下发同形数据）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskItem {
    /// 任务标题。
    pub title: String,
    /// 是否已完成。
    pub done: bool,
    /// 进度 0~100；渲染前一律经 [`progress_clamped`] 夹取。
    pub progress: u8,
}

/// 演示任务（≥2 条，进度是演示值，不伪造真实进度）。
#[must_use]
pub fn demo_tasks() -> Vec<TaskItem> {
    vec![
        TaskItem {
            title: "演示任务一".into(),
            done: true,
            progress: 100,
        },
        TaskItem {
            title: "演示任务二".into(),
            done: false,
            progress: 40,
        },
        TaskItem {
            title: "演示任务三".into(),
            done: false,
            progress: 15,
        },
    ]
}

/// 进度夹取：>100 一律按 100 渲染（防御运行时下发的越界值）。
#[must_use]
pub fn progress_clamped(p: u8) -> u8 {
    p.min(100)
}

/// 任务行文案：完成态加「已完成 · 」前缀，未完成态原样。
#[must_use]
pub fn task_label(t: &TaskItem) -> String {
    if t.done {
        let title = &t.title;
        format!("已完成 · {title}")
    } else {
        t.title.clone()
    }
}

/// 空列表 ⇒ 空态文案；非空 ⇒ 任务行（渲染据此分支）。
#[must_use]
pub fn empty_reason(items: &[TaskItem]) -> Option<&'static str> {
    items.is_empty().then_some(EMPTY_TEXT)
}

/// 渲染任务清单视图体：非空 ⇒ 任务行；空 ⇒ 居中空态文案。
///
/// 进度条宽度取 [`progress_clamped`] 之后的百分比；颜色一律取主题 token。
pub(super) fn render(items: &[TaskItem], theme: &gpui_kit::component::Theme) -> AnyElement {
    if let Some(text) = empty_reason(items) {
        return div()
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(text.to_owned()),
            )
            .into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap_2()
        .children(items.iter().map(|task| render_task_row(task, theme)))
        .into_any_element()
}

/// 演示入口：顶部如实标注 + 演示任务行（运行时契约就绪后改喂真实数据）。
pub(super) fn render_demo(theme: &gpui_kit::component::Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(DEMO_NOTE_TEXT.to_owned()),
        )
        .child(render(&demo_tasks(), theme))
        .into_any_element()
}

/// 一条任务行：状态点 + 标题 + 进度条。
fn render_task_row(task: &TaskItem, theme: &gpui_kit::component::Theme) -> impl IntoElement {
    let progress = f32::from(progress_clamped(task.progress)) / 100.0;
    let status_dot = if task.done {
        theme.foreground
    } else {
        theme.accent
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w_2().h_2().rounded_full().bg(status_dot))
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(task_label(task)),
                ),
        )
        .child(
            div().h_1().w_full().rounded_full().bg(theme.muted).child(
                div()
                    .h_full()
                    .w(relative(progress))
                    .rounded_full()
                    .bg(theme.accent),
            ),
        )
}

#[cfg(test)]
mod tests {
    use super::{
        demo_tasks, empty_reason, progress_clamped, render, task_label, TaskItem, EMPTY_TEXT,
    };

    #[test]
    fn progress_clamped_keeps_boundaries() {
        assert_eq!(progress_clamped(0), 0);
        assert_eq!(progress_clamped(100), 100);
        assert_eq!(progress_clamped(101), 100, "越界值必须夹到 100");
        assert_eq!(progress_clamped(u8::MAX), 100);
        assert_eq!(progress_clamped(37), 37, "区间内原样保留");
    }

    #[test]
    fn task_label_differs_between_done_and_pending() {
        let pending = TaskItem {
            title: "演示任务二".into(),
            done: false,
            progress: 40,
        };
        let done = TaskItem {
            title: pending.title.clone(),
            done: true,
            progress: 100,
        };
        assert_eq!(task_label(&pending), "演示任务二");
        assert_eq!(task_label(&done), "已完成 · 演示任务二");
        assert_ne!(task_label(&pending), task_label(&done));
    }

    #[test]
    fn empty_list_yields_empty_text() {
        assert_eq!(empty_reason(&[]), Some(EMPTY_TEXT), "空列表 ⇒ 空态文案出现");
        assert_eq!(empty_reason(&demo_tasks()), None, "非空 ⇒ 不出空态");
    }

    #[test]
    fn demo_tasks_are_at_least_two_and_in_range() {
        let tasks = demo_tasks();
        assert!(tasks.len() >= 2, "演示数据至少 2 条");
        for task in &tasks {
            assert_eq!(task.progress, progress_clamped(task.progress));
            assert!(!task.title.is_empty());
        }
    }

    #[test]
    fn render_smoke_covers_demo_and_empty() {
        let theme = gpui_kit::component::Theme::default();
        let _ = render(&demo_tasks(), &theme);
        let _ = render(&[], &theme);
    }
}
