//! 任务清单 tab 的纯函数层：任务条目、演示数据与文案计算（无 GPUI 依赖，可无窗口测）。
//!
//! 任务状态来源在运行时（契约未就绪）⇒ 本卡只做视图：演示数据 + 如实标注，
//! ⛔ 不伪造真实进度。

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
